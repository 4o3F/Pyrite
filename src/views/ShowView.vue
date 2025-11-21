<script setup lang="ts">
import { ref, nextTick, shallowRef } from "vue";
import { ParsedRoot, Problem, TeamStats } from "../types";
import { open } from "@tauri-apps/plugin-dialog";
import { join } from "@tauri-apps/api/path";
import { convertFileSrc } from "@tauri-apps/api/core";
import { readTextFile, exists } from "@tauri-apps/plugin-fs";
import { onMounted } from "vue";
import { onBeforeUnmount } from "vue";
import { computed } from "vue";
import { useVirtualizer } from "@tanstack/vue-virtual";

const inputPath = ref<string | null>(null);
const message = ref(
    "Select path containing 'teams' dir, 'affiliations' dir and data.json",
);
const convertedPathUrl = ref<string | null>(null);
const data = shallowRef<ParsedRoot | null>(null);
const highlightedTeamIndex = ref<number>(-1);
const scrollContainer = ref<HTMLElement | null>(null);

const pendingReorderTeamId = ref<string | null>(null);
const pendingAward = ref<{
    team: TeamStats;
    awards: string[];
} | null>(null);
const doAward = ref<boolean>(false);

const virtualizerOptions = computed(() => ({
    count: data.value?.scoreboard.length ?? 1,
    getScrollElement: () => scrollContainer.value,
    estimateSize: () => 80,
    overscan: 1,
}));

const virtualizer = useVirtualizer(virtualizerOptions);
const virtualRows = computed(() => virtualizer.value.getVirtualItems());

async function selectInputFile() {
    const selected = await open({
        directory: true,
        multiple: false,
        filters: [],
    });
    if (typeof selected !== "string") return;

    inputPath.value = selected;
    message.value = `Selected: ${inputPath.value}`;
    data.value = null;
    convertedPathUrl.value = null;

    try {
        const teamsPath = await join(selected, "teams");
        const affiliationsPath = await join(selected, "affiliations");
        const dataJsonPath = await join(selected, "data.json");

        const [teamsExists, affiliationsExists, dataJsonExists] =
            await Promise.all([
                exists(teamsPath),
                exists(affiliationsPath),
                exists(dataJsonPath),
            ]);

        if (teamsExists && affiliationsExists && dataJsonExists) {
            convertedPathUrl.value = convertFileSrc(selected);

            try {
                const contents = await readTextFile(dataJsonPath);
                const parsedRoot: ParsedRoot = ParsedRoot.parse(
                    JSON.parse(contents),
                );

                parsedRoot.scoreboard = parsedRoot.scoreboard.filter(
                    (t: TeamStats) => Object.keys(t.problem_stats).length != 0,
                );
                parsedRoot.problems = parsedRoot.problems.sort(
                    (a: Problem, b: Problem) => a.id.localeCompare(b.id),
                );

                data.value = parsedRoot;
                message.value = "Data Loaded";

                highlightedTeamIndex.value = data.value.scoreboard.length - 1;
                pendingReorderTeamId.value = null; // 重置
            } catch (e) {
                console.error("Failed to read or parse ", e);
                message.value = `Can't read or parse file, error ${e}`;
                data.value = null;
                convertedPathUrl.value = null;
            }
        } else {
            const missing: string[] = [];
            if (!teamsExists) missing.push("'teams' dir");
            if (!affiliationsExists) missing.push("'affiliations' dir");
            if (!dataJsonExists) missing.push("'data.json' file");
            message.value = `Missing: ${missing.join(", ")}。`;
            console.warn("Structure validation failed ", message.value);
        }
    } catch (e) {
        console.log("Tauri file operation failed ", e);
        message.value = "Tauri file operation failed " + e;
    }
}

function getProblemStatusClass(team: TeamStats, shortName: string) {
    const s = team.problem_stats[shortName];
    if (!s) return "bg-gray-800";
    if (s.attempted_during_freeze) return "bg-cyan-500";
    return s.solved ? "bg-green-500" : "bg-red-500";
}

function sortScoreboard(a: TeamStats, b: TeamStats) {
    if (a.sortorder !== b.sortorder) return a.sortorder - b.sortorder;
    if (a.total_points !== b.total_points)
        return b.total_points - a.total_points;
    if (a.total_penalty !== b.total_penalty)
        return a.total_penalty - b.total_penalty;
    const at = a.last_ac_time?.getTime() ?? Number.POSITIVE_INFINITY;
    const bt = b.last_ac_time?.getTime() ?? Number.POSITIVE_INFINITY;
    if (at !== bt) return at - bt;
    return a.team_id.localeCompare(b.team_id);
}

function reinsertTeam(scoreboard: TeamStats[], teamId: string) {
    const oldIndex = scoreboard.findIndex((t) => t.team_id === teamId);
    if (oldIndex < 0) return null;

    const item = scoreboard[oldIndex];
    scoreboard.splice(oldIndex, 1);

    let lo = 0,
        hi = scoreboard.length;
    while (lo < hi) {
        const mid = (lo + hi) >> 1;
        if (sortScoreboard(item, scoreboard[mid]) < 0) hi = mid;
        else lo = mid + 1;
    }
    scoreboard.splice(lo, 0, item);

    return {
        from: oldIndex,
        to: lo,
        newScoreboard: scoreboard,
    };
}

function revealNextForTeam(team: TeamStats, problems: any[]): TeamStats | null {
    for (const prob of problems) {
        const ps = team.problem_stats[prob.short_name];
        if (!ps || !ps.attempted_during_freeze) continue;
        ps.attempted_during_freeze = false;
        if (ps.solved) {
            team.total_points += 1;
            team.total_penalty += ps.penalty;
            team.last_ac_time = ps.first_ac_time;
        }
        return team;
    }
    return null;
}

function createUnifiedAnimation(sourceRect: DOMRect, sourceHTML: string) {
    const clone = document.createElement("div");
    clone.innerHTML = sourceHTML;
    const clonedElement = clone.firstElementChild as HTMLElement;

    clonedElement.style.cssText = `
        position: fixed;
        left: ${sourceRect.left}px;
        top: ${sourceRect.top}px;
        width: ${sourceRect.width}px;
        height: ${sourceRect.height}px;
        pointer-events: none;
        z-index: 10000;
    `;

    document.body.appendChild(clonedElement);

    const targetY = -(sourceRect.top + sourceRect.height + 50);

    const animation = clonedElement.animate(
        [
            {
                transform: "translateY(0)",
                opacity: 1,
            },
            {
                transform: `translateY(${targetY}px)`,
                opacity: 0.3,
            },
        ],
        {
            duration: 800,
            easing: "ease-in",
            fill: "forwards",
        },
    );

    animation.onfinish = () => clonedElement.remove();
}

const isProcessing = ref(false);
const processQueue = ref<(() => Promise<void>)[]>([]);

async function processNextInQueue() {
    if (isProcessing.value || processQueue.value.length === 0) {
        return;
    }

    isProcessing.value = true;

    const task = processQueue.value.shift();

    if (task) {
        try {
            await task();
        } catch (error) {
            console.error("处理队列任务失败：", error);
        } finally {
            isProcessing.value = false;
            await processNextInQueue();
        }
    } else {
        isProcessing.value = false;
    }
}

// 修改后的核心逻辑：
// 1. 如果有 pendingReorderTeamId，则本次按空格只做“重排 + 动画”。
// 2. 否则才进行 reveal。
async function processSingleSpaceCore() {
    if (!data.value) {
        return;
    }

    const currentData = data.value;

    // ---------- 第一步：优先处理“等待重排”的队伍 ----------
    if (pendingReorderTeamId.value) {
        const teamId = pendingReorderTeamId.value;
        pendingReorderTeamId.value = null;

        const scoreboardBefore = [...currentData.scoreboard];
        const result = reinsertTeam(scoreboardBefore, teamId);

        if (result && result.from !== result.to) {
            const startInView = virtualizer.value
                .getVirtualIndexes()
                .includes(result.from);

            if (startInView) {
                // 此时界面上已经是“分数已更新但还未重排”的列表
                await nextTick();

                const sourceElement = document.querySelector(
                    `[data-team-id="${teamId}"]`,
                ) as HTMLElement | null;

                if (sourceElement) {
                    const sourceRect = sourceElement.getBoundingClientRect();
                    const sourceHTML =
                        sourceElement.parentElement?.outerHTML || "";

                    createUnifiedAnimation(sourceRect, sourceHTML);

                    await new Promise<void>((resolve) => {
                        setTimeout(() => {
                            data.value = {
                                ...currentData,
                                scoreboard: result.newScoreboard,
                            };
                            resolve();
                        }, 50);
                    });
                } else {
                    data.value = {
                        ...currentData,
                        scoreboard: result.newScoreboard,
                    };
                }
            } else {
                data.value = {
                    ...currentData,
                    scoreboard: result.newScoreboard,
                };
            }

            if (
                highlightedTeamIndex.value >=
                (data.value?.scoreboard.length ?? 0)
            ) {
                highlightedTeamIndex.value =
                    (data.value?.scoreboard.length ?? 1) - 1;
            }
        }

        await nextTick();
        virtualizer.value.scrollToIndex(highlightedTeamIndex.value, {
            align: "center",
            behavior: "smooth",
        });

        // 本次只负责“重排 + 动画”，不再执行下面的 reveal 逻辑
        return;
    }

    // ---------- 第二步：正常 reveal 逻辑 ----------
    if (
        highlightedTeamIndex.value == null ||
        highlightedTeamIndex.value < 0 ||
        !data.value
    ) {
        return;
    }

    const team = currentData.scoreboard[highlightedTeamIndex.value];
    const revealedTeam = revealNextForTeam(team, currentData.problems);

    if (revealedTeam) {
        // 只更新该队伍的分数、罚时等，不改变列表顺序
        const newScoreboard = [...currentData.scoreboard];
        newScoreboard[highlightedTeamIndex.value] = revealedTeam;

        data.value = {
            ...currentData,
            scoreboard: newScoreboard,
        };

        // 预判：如果现在重排，这个队伍是否会改变名次
        const scoreboardCopy = [...newScoreboard];
        const result = reinsertTeam(scoreboardCopy, revealedTeam.team_id);

        if (result && result.from !== result.to) {
            // AC 且名次会变化：本次不重排，只标记下次空格要重排
            pendingReorderTeamId.value = revealedTeam.team_id;
        } else {
            // 没有名次变化，保持原逻辑：不重排，也不移动高亮
            pendingReorderTeamId.value = null;
        }
    } else {
        // The team has no problem to reveal and have a award
        if (!(team.team_id in data.value.awards) && !doAward.value) {
            highlightedTeamIndex.value = Math.max(
                0,
                highlightedTeamIndex.value - 1,
            );
        } else {
            if (doAward.value) {
                doAward.value = false;
                highlightedTeamIndex.value = Math.max(
                    0,
                    highlightedTeamIndex.value - 1,
                );
            } else {
                doAward.value = true;
                pendingAward.value = {
                    team: team,
                    awards: data.value.awards[team.team_id],
                };
            }
        }
    }

    await nextTick();
    virtualizer.value.scrollToIndex(highlightedTeamIndex.value, {
        align: "center",
        behavior: "smooth",
    });
}

function processSingleSpace() {
    processQueue.value.push(processSingleSpaceCore);
    processNextInQueue();
}

function onKeydown(e: KeyboardEvent) {
    if (e.code === "Space") {
        e.preventDefault();
        processSingleSpace();
    }
}

onMounted(() => window.addEventListener("keydown", onKeydown));
onBeforeUnmount(() => window.removeEventListener("keydown", onKeydown));
</script>

<template>
    <div class="flex flex-col items-center justify-center h-full bg-gray-100">
        <template v-if="!data">
            <button
                @click="selectInputFile"
                class="px-6 py-2 font-medium text-white bg-blue-500 rounded-lg hover:bg-blue-400 focus:outline-none focus:ring focus:ring-blue-200"
            >
                Select File
            </button>
            <p class="mt-4 text-sm font-medium text-gray-600">{{ message }}</p>
        </template>

        <template v-else>
            <div
                v-if="doAward"
                id="award-overlay"
                class="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-80 bg-cover"
                :style="{
                    'background-image': `url(${
                        convertedPathUrl +
                        '/teams/' +
                        pendingAward?.team.team_id +
                        '.jpg'
                    })`,
                }"
            >
                <div class="h-full w-full grid grid-rows-6">
                    <div class="row-span-4"></div>
                    <div
                        class="row-span-2 bg-gray-700/80 p-6 flex flex-col gap-5"
                    >
                        <p class="font-bold text-4xl text-white">
                            {{ pendingAward?.team.team_name }}
                        </p>
                        <p
                            v-for="award in pendingAward?.awards"
                            class="text-3xl text-white"
                        >
                            {{ award }}
                        </p>
                    </div>
                </div>
            </div>
            <div
                class="w-full h-full relative overflow-y-auto contain-strict"
                ref="scrollContainer"
            >
                <div
                    :style="{
                        height: `${virtualizer.getTotalSize()}px`,
                        width: '100%',
                        position: 'relative',
                    }"
                >
                    <div
                        :style="{
                            position: 'absolute',
                            top: 0,
                            left: 0,
                            width: '100%',
                            transform: `translateY(${virtualRows[0]?.start ?? 0}px)`,
                        }"
                    >
                        <div
                            v-for="virtualRow in virtualRows"
                            :key="virtualRow.key.toString()"
                            :style="{
                                willChange: 'transform',
                            }"
                        >
                            <div
                                class="h-20 list-item relative transition-all"
                                :data-team-id="
                                    data.scoreboard[virtualRow.index].team_id
                                "
                            >
                                <div
                                    class="flex flex-row w-full h-full px-1 gap-1 items-center justify-center text-white border-b border-gray-700 transition-all duration-300"
                                    :class="[
                                        virtualRow.index ===
                                        highlightedTeamIndex
                                            ? 'bg-yellow-600 shadow-lg scale-[1.02] z-10'
                                            : 'bg-gray-500',
                                    ]"
                                >
                                    <p
                                        class="flex text-xl font-bold w-16 justify-center"
                                    >
                                        {{
                                            String(
                                                virtualRow.index + 1,
                                            ).padStart(
                                                Math.max(
                                                    3,
                                                    Math.ceil(
                                                        Math.log10(
                                                            data.scoreboard
                                                                .length + 1,
                                                        ),
                                                    ),
                                                ),
                                                "0",
                                            )
                                        }}
                                    </p>

                                    <img
                                        class="h-full aspect-square rounded-full p-2"
                                        lazy
                                        :src="
                                            convertedPathUrl +
                                            '/affiliations/' +
                                            data.scoreboard[virtualRow.index]
                                                .team_affiliation +
                                            '.jpg'
                                        "
                                        :alt="
                                            data.scoreboard[virtualRow.index]
                                                .team_affiliation
                                        "
                                    />

                                    <div class="flex flex-col grow gap-2">
                                        <p class="font-bold">
                                            {{
                                                data.scoreboard[
                                                    virtualRow.index
                                                ].team_name
                                            }}
                                        </p>
                                        <div class="flex flex-row gap-1">
                                            <p
                                                v-for="problem in data.problems"
                                                :key="problem.short_name"
                                                class="flex rounded-md grow justify-center w-full transition-all duration-300"
                                                :class="
                                                    getProblemStatusClass(
                                                        data.scoreboard[
                                                            virtualRow.index
                                                        ],
                                                        problem.short_name,
                                                    )
                                                "
                                            >
                                                {{ problem.short_name }}
                                            </p>
                                        </div>
                                    </div>

                                    <!-- 显示统计信息 -->
                                    <div
                                        class="flex flex-col items-end pr-4 min-w-[120px]"
                                    >
                                        <p class="text-lg font-bold">
                                            {{
                                                data.scoreboard[
                                                    virtualRow.index
                                                ].total_points
                                            }}
                                            solved
                                        </p>
                                        <p class="text-sm opacity-80">
                                            {{
                                                data.scoreboard[
                                                    virtualRow.index
                                                ].total_penalty
                                            }}
                                            penalty
                                        </p>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </template>
    </div>
</template>
