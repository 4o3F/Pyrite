<script setup lang="ts">
import { ref } from "vue";
import { Channel, invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { computed } from "vue";

const inputPath = ref<string | null>(null);
const outputPath = ref<string | null>(null);
const loading = ref(false);
const message = ref("");
const percent = ref(0.0);
const percentText = computed(() => `${(percent.value * 100).toFixed(0)}%`);

async function selectInputFile() {
    const selected = await open({
        multiple: false,
        filters: [
            { name: "Event Feed", extensions: ["ndjson", "json"] },
            { name: "All Files", extensions: ["*"] },
        ],
    });

    if (typeof selected === "string") {
        inputPath.value = selected;
        message.value = "";
    }
}

async function selectOutputFile() {
    const selected = await save({
        filters: [
            { name: "Parsed Event Output", extensions: ["json", "bin"] },
            { name: "All Files", extensions: ["*"] },
        ],
    });

    if (typeof selected === "string") {
        outputPath.value = selected;
        message.value = "";
    }
}

async function parseEventFeed() {
    loading.value = true;
    message.value = "";

    const inPath = inputPath.value;
    const outPath = outputPath.value;

    if (!inPath) {
        message.value = "⚠️ Please select input event feed file first.";
        loading.value = false;
        return;
    }
    if (!outPath) {
        message.value = "⚠️ Please select output path for parsed file.";
        loading.value = false;
        return;
    }

    try {
        const onLogEvent = new Channel<number>();
        onLogEvent.onmessage = (message: number) => {
            percent.value = message;
        };
        invoke("parse_event_feed", {
            inputPath: inPath,
            outputPath: outPath,
            logChannel: onLogEvent,
        }).then(() => {
            message.value = "✅ Parsing finished";
        });
        message.value =
            "✅ Parsing started, check Rust console for progress...";
    } catch (err) {
        console.error(err);
        message.value = "❌ Failed to start parsing: " + err;
    } finally {
        loading.value = false;
    }
}
</script>

<template>
    <div
        class="flex flex-col items-center justify-center min-h-screen bg-gray-50"
    >
        <div class="bg-white shadow-lg rounded-lg p-8 w-full max-w-md">
            <h1 class="text-2xl font-bold mb-6 text-center">
                Select Event Feed & Output File
            </h1>

            <div class="flex flex-col items-center gap-6 w-full">
                <div class="w-full">
                    <button
                        @click="selectInputFile"
                        class="px-6 py-2 font-medium text-white bg-blue-500 rounded-lg hover:bg-blue-400 focus:outline-none focus:ring focus:ring-blue-200 w-full"
                    >
                        Select Event Feed File
                    </button>
                    <p class="mt-2 text-gray-600 text-sm break-all text-center">
                        {{ inputPath || "No input file selected" }}
                    </p>
                </div>

                <div class="w-full">
                    <button
                        @click="selectOutputFile"
                        class="px-6 py-2 font-medium text-white bg-purple-500 rounded-lg hover:bg-purple-400 focus:outline-none focus:ring focus:ring-purple-200 w-full"
                    >
                        Select Output Save Path
                    </button>
                    <p class="mt-2 text-gray-600 text-sm break-all text-center">
                        {{ outputPath || "No output file selected" }}
                    </p>
                </div>

                <button
                    @click="parseEventFeed"
                    class="px-6 py-2 font-medium text-white bg-green-500 rounded-lg hover:bg-green-400 focus:outline-none focus:ring focus:ring-green-200 w-full disabled:bg-gray-400"
                    :disabled="!inputPath || !outputPath || loading"
                >
                    {{ loading ? "Parsing..." : "Parse & Save" }}
                </button>

                <div
                    class="flex w-full h-4 bg-gray-200 rounded-full overflow-hidden dark:bg-neutral-700"
                    role="progressbar"
                    :aria-valuenow="percentText.replace('%', '')"
                    aria-valuemin="0"
                    aria-valuemax="100"
                    v-if="percent != 0"
                >
                    <div
                        class="flex flex-col justify-center rounded-full overflow-hidden bg-blue-600 text-xs text-white text-center whitespace-nowrap dark:bg-blue-500 transition duration-500"
                        :style="{ width: percentText }"
                    >
                        {{ percentText }}
                    </div>
                </div>
                <p v-if="message" class="text-center text-sm text-gray-700">
                    {{ message }}
                </p>
            </div>
        </div>
    </div>
</template>
