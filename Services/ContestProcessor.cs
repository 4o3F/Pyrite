using Pyrite.Models;
using System;
using System.Collections.Generic;
using System.Linq;

namespace Pyrite.Services;

public static class ContestProcessor
{
    public static List<string> ValidateAndTransform(ContestState state, PyriteConfig config)
    {
        ApplySubmissionFilters(state, config);
        ApplyTeamGroupRemap(state, config);

        ValidateTeamGroups(state);
        ValidateAllSubmissionsJudged(state);

        var (contestStart, contestFreeze) = GetContestTimes(state);

        var warnings = new List<string>();
        var preFreezeMap = BuildInitialTeamStatusMap(state);
        ApplyJudgementsToStatusMap(state, preFreezeMap, contestStart, contestFreeze, warnings);

        state.LeaderboardPreFreeze = ToSortedLeaderboard(preFreezeMap);
        state.LeaderboardFinalized = ComputeFinalizedLeaderboard(state);
        return warnings;
    }

    private static void ApplySubmissionFilters(ContestState state, PyriteConfig config)
    {
        if (config.FilterTeamSubmissions.Count == 0) return;

        var filterSet = config.FilterTeamSubmissions.ToHashSet(StringComparer.Ordinal);

        var removedSubmissionIds = state.Submissions
            .Where(x => filterSet.Contains(x.Value.TeamId))
            .Select(x => x.Key)
            .ToHashSet(StringComparer.Ordinal);

        if (removedSubmissionIds.Count == 0) return;

        state.Submissions = state.Submissions
            .Where(x => !removedSubmissionIds.Contains(x.Key))
            .ToDictionary(k => k.Key, v => v.Value, StringComparer.Ordinal);

        state.Judgements = state.Judgements
            .Where(x => !removedSubmissionIds.Contains(x.Value.SubmissionId))
            .ToDictionary(k => k.Key, v => v.Value, StringComparer.Ordinal);
    }

    private static void ApplyTeamGroupRemap(ContestState state, PyriteConfig config)
    {
        if (config.TeamGroupMap.Count == 0) return;

        var errors = new List<string>();

        foreach (var (teamId, targetGroupId) in config.TeamGroupMap)
        {
            if (!state.Groups.ContainsKey(targetGroupId))
            {
                errors.Add($"team_group_map target group '{targetGroupId}' for team '{teamId}' does not exist");
                continue;
            }

            if (!state.Teams.TryGetValue(teamId, out var team))
            {
                errors.Add($"team_group_map team '{teamId}' does not exist in event feed");
                continue;
            }

            team.GroupIds = [targetGroupId];
        }

        if (errors.Count > 0)
            throw new InvalidOperationException(
                $"Invalid team_group_map entries ({errors.Count}): {string.Join(" | ", errors)}");
    }

    private static void ValidateAllSubmissionsJudged(ContestState state)
    {
        var judgedSubmissionIds = state.Judgements.Values
            .Select(j => j.SubmissionId)
            .ToHashSet(StringComparer.Ordinal);

        var unjudged = state.Submissions.Keys.FirstOrDefault(id => !judgedSubmissionIds.Contains(id));
        if (unjudged is not null) throw new InvalidOperationException($"Submission {unjudged} not judged.");
    }

    private static void ValidateTeamGroups(ContestState state)
    {
        var issues = new List<string>();

        foreach (var team in state.Teams.Values)
        {
            if (team.GroupIds.Count == 0)
            {
                issues.Add($"{team.Id} ({team.Name}) has no group_ids");
                continue;
            }

            var unknownGroups = team.GroupIds.Where(groupId => !state.Groups.ContainsKey(groupId)).ToList();
            if (unknownGroups.Count > 0)
                issues.Add($"{team.Id} ({team.Name}) has unknown group_ids: {string.Join(", ", unknownGroups)}");
        }

        if (issues.Count > 0)
            throw new InvalidOperationException(
                $"Invalid team group data for {issues.Count} team(s): {string.Join(" | ", issues)}");
    }

    private static Dictionary<string, TeamStatus> BuildInitialTeamStatusMap(ContestState state)
    {
        var teamStatusMap = new Dictionary<string, TeamStatus>(StringComparer.Ordinal);

        foreach (var team in state.Teams.Values)
        {
            var sortorder = team.GroupIds
                .Where(groupId => state.Groups.ContainsKey(groupId))
                .Select(groupId => state.Groups[groupId].Sortorder)
                .DefaultIfEmpty(0)
                .Min();

            var organizationId = team.OrganizationId
                                 ?? throw new InvalidOperationException($"Missing organization_id for team {team.Id}.");

            teamStatusMap[team.Id] = new TeamStatus(team.Id, team.Name, organizationId, sortorder);
        }

        return teamStatusMap;
    }

    private static List<Judgement> BuildJudgementOrder(ContestState state)
    {
        return state.Judgements.Values
            .OrderBy(j =>
                state.Submissions.TryGetValue(j.SubmissionId, out var sub) ? sub.Time ?? j.StartTime : j.StartTime)
            .ToList();
    }

    private static Submission? TryGetSubmissionForJudgement(ContestState state, Judgement judgement,
        List<string> warnings)
    {
        if (state.Submissions.TryGetValue(judgement.SubmissionId, out var submission)) return submission;

        warnings.Add($"Skipping judgement {judgement.Id} because submission {judgement.SubmissionId} is missing");
        return null;
    }

    private static void ApplyJudgementToStatus(
        ContestState state,
        Dictionary<string, TeamStatus> teamStatusMap,
        Judgement judgement,
        DateTimeOffset contestStart,
        DateTimeOffset contestFreeze)
    {
        if (!state.Submissions.TryGetValue(judgement.SubmissionId, out var submission)) return;

        if (!teamStatusMap.TryGetValue(submission.TeamId, out var teamStatus))
            throw new InvalidOperationException($"Unknown team id {submission.TeamId}.");

        var submissionTime = submission.Time
                             ?? throw new InvalidOperationException(
                                 $"Unknown submission time for submission {submission.Id}.");

        teamStatus.AddSubmission(
            submission.ProblemId,
            submissionTime,
            judgement.JudgementTypeId,
            state.JudgementTypes,
            contestStart,
            contestFreeze);
    }

    private static void RecomputeTeamTotals(Dictionary<string, TeamStatus> teamStatusMap)
    {
        foreach (var team in teamStatusMap.Values)
        {
            team.TotalPoints = 0;
            team.TotalPenalty = 0;
            team.LastAcTime = null;

            foreach (var stat in team.ProblemStats.Values)
            {
                if (!stat.Solved) continue;

                team.TotalPoints += 1;
                team.TotalPenalty += stat.Penalty;

                if (stat.FirstAcTime.HasValue && (!team.LastAcTime.HasValue || stat.FirstAcTime > team.LastAcTime))
                    team.LastAcTime = stat.FirstAcTime;
            }
        }
    }

    private static List<TeamStatus> ComputeFinalizedLeaderboard(ContestState state)
    {
        var (contestStart, contestFreeze) = GetContestTimes(state);

        var finalizedMap = BuildInitialTeamStatusMap(state);
        ApplyJudgementsToStatusMap(state, finalizedMap, contestStart, contestFreeze);

        RecomputeTeamTotals(finalizedMap);
        return ToSortedLeaderboard(finalizedMap);
    }

    private static List<TeamStatus> ToSortedLeaderboard(Dictionary<string, TeamStatus> map)
    {
        var sorted = map.Values.ToList();
        sorted.Sort();
        return sorted;
    }

    private static (DateTimeOffset ContestStart, DateTimeOffset ContestFreeze) GetContestTimes(ContestState state)
    {
        var contest = state.Contest ?? throw new InvalidOperationException("Contest not defined.");
        var contestStart = contest.StartTime ?? throw new InvalidOperationException("Contest start time not defined.");
        var contestFreeze = contest.ScoreboardFreezeTime ??
                            throw new InvalidOperationException("Contest freeze time not defined.");
        return (contestStart, contestFreeze);
    }

    private static void ApplyJudgementsToStatusMap(
        ContestState state,
        Dictionary<string, TeamStatus> teamStatusMap,
        DateTimeOffset contestStart,
        DateTimeOffset contestFreeze,
        List<string>? warnings = null)
    {
        foreach (var judgement in BuildJudgementOrder(state))
        {
            if (warnings is not null)
            {
                var submission = TryGetSubmissionForJudgement(state, judgement, warnings);
                if (submission is null) continue;

                if (submission.Time is null && judgement.StartTime is null)
                    throw new InvalidOperationException($"Unknown submission time for submission {submission.Id}.");
            }

            ApplyJudgementToStatus(state, teamStatusMap, judgement, contestStart, contestFreeze);
        }
    }
}
