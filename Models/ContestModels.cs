using System;
using System.Collections.Generic;
using System.Globalization;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Pyrite.Models;

[JsonConverter(typeof(JsonStringEnumConverter<EventType>))]
public enum EventType
{
    [JsonStringEnumMemberName("contest")] Contest,

    [JsonStringEnumMemberName("judgement-types")]
    JudgementTypes,

    [JsonStringEnumMemberName("languages")]
    Languages,
    [JsonStringEnumMemberName("problems")] Problems,
    [JsonStringEnumMemberName("groups")] Groups,

    [JsonStringEnumMemberName("organizations")]
    Organizations,
    [JsonStringEnumMemberName("teams")] Teams,
    [JsonStringEnumMemberName("persons")] Persons,
    [JsonStringEnumMemberName("accounts")] Accounts,
    [JsonStringEnumMemberName("state")] State,

    [JsonStringEnumMemberName("submissions")]
    Submissions,

    [JsonStringEnumMemberName("judgements")]
    Judgements,
    [JsonStringEnumMemberName("runs")] Runs,

    [JsonStringEnumMemberName("clarifications")]
    Clarifications,
    [JsonStringEnumMemberName("awards")] Awards
}

public sealed class Event
{
    public string? Token { get; set; }
    public string? Id { get; set; }

    [JsonPropertyName("type")] public EventType EventType { get; set; }

    public JsonElement? Data { get; set; }
    public string Time { get; set; } = string.Empty;
}

public interface IHasId
{
    string Id { get; }
}

public sealed class JudgementType : IHasId
{
    public string Id { get; set; } = string.Empty;
    public string Name { get; set; } = string.Empty;
    public bool Penalty { get; set; }
    public bool Solved { get; set; }
}

public sealed class Group : IHasId
{
    public string Id { get; set; } = string.Empty;
    public bool Hidden { get; set; }

    [JsonPropertyName("icpc_id")] public string? IcpcId { get; set; }

    public string Name { get; set; } = string.Empty;
    public int Sortorder { get; set; }
    public string? Color { get; set; }

    [JsonPropertyName("allow_self_registration")]
    public bool AllowSelfRegistration { get; set; }
}

public sealed class Organization : IHasId
{
    public string Id { get; set; } = string.Empty;

    [JsonPropertyName("icpc_id")] public string? IcpcId { get; set; }

    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("formal_name")] public string FormalName { get; set; } = string.Empty;

    public string Shortname { get; set; } = string.Empty;
    public string Country { get; set; } = string.Empty;
    public List<OrganizationImage> Logo { get; set; } = [];

    [JsonPropertyName("country_flag")] public List<OrganizationImage> CountryFlags { get; set; } = [];
}

public sealed class OrganizationImage
{
    public string Href { get; set; } = string.Empty;
    public string Mime { get; set; } = string.Empty;
    public string Filename { get; set; } = string.Empty;
    public uint Width { get; set; }
    public uint Height { get; set; }
}

public sealed class Team : IHasId
{
    public Location? Location { get; set; }

    [JsonPropertyName("organization_id")] public string? OrganizationId { get; set; }

    public bool Hidden { get; set; }

    [JsonPropertyName("group_ids")] public List<string> GroupIds { get; set; } = [];

    public string? Affiliation { get; set; }
    public string? Nationality { get; set; }
    public string Id { get; set; } = string.Empty;

    [JsonPropertyName("icpc_id")] public string? IcpcId { get; set; }

    public string? Label { get; set; }
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("display_name")] public string? DisplayName { get; set; }

    [JsonPropertyName("public_description")]
    public string? PublicDescription { get; set; }
}

public sealed class Location
{
    public string Description { get; set; } = string.Empty;
}

public sealed class Account : IHasId
{
    public string Id { get; set; } = string.Empty;
    public string Username { get; set; } = string.Empty;
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("last_login_time")]
    [JsonConverter(typeof(OptionalDateTimeOffsetConverter))]
    public DateTimeOffset? LastLoginTime { get; set; }

    [JsonPropertyName("last_api_login_time")]
    [JsonConverter(typeof(OptionalDateTimeOffsetConverter))]
    public DateTimeOffset? LastApiLoginTime { get; set; }

    [JsonPropertyName("first_login_time")]
    [JsonConverter(typeof(OptionalDateTimeOffsetConverter))]
    public DateTimeOffset? FirstLoginTime { get; set; }

    public string Team { get; set; } = string.Empty;

    [JsonPropertyName("team_id")] public string TeamId { get; set; } = string.Empty;

    public List<string> Roles { get; set; } = [];

    [JsonPropertyName("type")] public string Type { get; set; } = string.Empty;

    public string? Email { get; set; }

    [JsonPropertyName("last_ip")] public string LastIp { get; set; } = string.Empty;

    public string? Ip { get; set; }
    public bool Enabled { get; set; }
}

public sealed class Problem : IHasId
{
    public int Ordinal { get; set; }
    public string Id { get; set; } = string.Empty;

    [JsonPropertyName("short_name")] public string ShortName { get; set; } = string.Empty;

    public string Rgb { get; set; } = string.Empty;
    public string Color { get; set; } = string.Empty;
    public string Label { get; set; } = string.Empty;

    [JsonPropertyName("time_limit")] public double TimeLimit { get; set; }

    public List<JsonElement> Statement { get; set; } = [];

    [JsonPropertyName("externalid")] public string? ExternalId { get; set; }

    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("test_data_count")] public int TestDataCount { get; set; }
}

public sealed class Submission : IHasId
{
    [JsonPropertyName("language_id")] public string LanguageId { get; set; } = string.Empty;

    [JsonConverter(typeof(OptionalDateTimeOffsetConverter))]
    public DateTimeOffset? Time { get; set; }

    [JsonPropertyName("contest_time")]
    [JsonConverter(typeof(ContestDurationConverter))]
    public TimeSpan ContestTime { get; set; }

    [JsonPropertyName("team_id")] public string TeamId { get; set; } = string.Empty;

    [JsonPropertyName("problem_id")] public string ProblemId { get; set; } = string.Empty;

    public List<SubmissionFile> Files { get; set; } = [];
    public string Id { get; set; } = string.Empty;

    [JsonPropertyName("external_id")] public string? ExternalId { get; set; }

    [JsonPropertyName("entry_point")] public string? EntryPoint { get; set; }

    [JsonPropertyName("import_error")] public string? ImportError { get; set; }
}

public sealed class SubmissionFile
{
    public string Href { get; set; } = string.Empty;
    public string Mime { get; set; } = string.Empty;
    public string Filename { get; set; } = string.Empty;
}

public sealed class Judgement : IHasId
{
    [JsonPropertyName("max_run_time")] public double? MaxRunTime { get; set; }

    [JsonPropertyName("start_time")]
    [JsonConverter(typeof(OptionalDateTimeOffsetConverter))]
    public DateTimeOffset? StartTime { get; set; }

    [JsonPropertyName("start_contest_time")]
    [JsonConverter(typeof(ContestDurationConverter))]
    public TimeSpan StartContestTime { get; set; }

    [JsonPropertyName("end_time")]
    [JsonConverter(typeof(OptionalDateTimeOffsetConverter))]
    public DateTimeOffset? EndTime { get; set; }

    [JsonPropertyName("end_contest_time")]
    [JsonConverter(typeof(OptionalContestDurationConverter))]
    public TimeSpan? EndContestTime { get; set; }

    [JsonPropertyName("submission_id")] public string SubmissionId { get; set; } = string.Empty;

    public string Id { get; set; } = string.Empty;
    public bool Valid { get; set; }

    [JsonPropertyName("judgement_type_id")]
    public string? JudgementTypeId { get; set; }
}

public sealed class Award : IHasId
{
    public string Id { get; set; } = string.Empty;
    public string Citation { get; set; } = string.Empty;

    [JsonPropertyName("team_ids")] public List<string> TeamIds { get; set; } = [];
}

public sealed class Contest
{
    [JsonPropertyName("formal_name")] public string FormalName { get; set; } = string.Empty;

    [JsonPropertyName("scoreboard_type")] public string ScoreboardType { get; set; } = string.Empty;

    [JsonPropertyName("start_time")]
    [JsonConverter(typeof(OptionalDateTimeOffsetConverter))]
    public DateTimeOffset? StartTime { get; set; }

    [JsonPropertyName("end_time")]
    [JsonConverter(typeof(OptionalDateTimeOffsetConverter))]
    public DateTimeOffset? EndTime { get; set; }

    [JsonPropertyName("scoreboard_thaw_time")]
    [JsonConverter(typeof(OptionalDateTimeOffsetConverter))]
    public DateTimeOffset? ScoreboardThawTime { get; set; }

    [JsonConverter(typeof(ContestDurationConverter))]
    public TimeSpan Duration { get; set; }

    [JsonPropertyName("scoreboard_freeze_duration")]
    [JsonConverter(typeof(ContestDurationConverter))]
    public TimeSpan ScoreboardFreezeDuration { get; set; }

    public string Id { get; set; } = string.Empty;

    [JsonPropertyName("external_id")] public string? ExternalId { get; set; }

    public string Name { get; set; } = string.Empty;
    public string Shortname { get; set; } = string.Empty;

    [JsonPropertyName("allow_submit")] public bool AllowSubmit { get; set; }

    [JsonPropertyName("runtime_as_score_tiebreaker")]
    public bool RuntimeAsScoreTiebreaker { get; set; }

    [JsonPropertyName("warning_message")] public string? WarningMessage { get; set; }

    [JsonPropertyName("penalty_time")] public int PenaltyTime { get; set; }

    [JsonIgnore] public DateTimeOffset? ScoreboardFreezeTime { get; set; }
}

public sealed class ContestState
{
    public Contest? Contest { get; set; }

    [JsonPropertyName("judgement_types")] public Dictionary<string, JudgementType> JudgementTypes { get; set; } = [];

    public Dictionary<string, Group> Groups { get; set; } = [];
    public Dictionary<string, Organization> Organizations { get; set; } = [];
    public Dictionary<string, Team> Teams { get; set; } = [];
    public Dictionary<string, Account> Accounts { get; set; } = [];
    public Dictionary<string, Problem> Problems { get; set; } = [];
    public Dictionary<string, Submission> Submissions { get; set; } = [];
    public Dictionary<string, Judgement> Judgements { get; set; } = [];
    public Dictionary<string, Award> Awards { get; set; } = [];

    [JsonPropertyName("leaderboard_pre_freeze")]
    public List<TeamStatus> LeaderboardPreFreeze { get; set; } = [];

    [JsonPropertyName("leaderboard_finalized")]
    public List<TeamStatus> LeaderboardFinalized { get; set; } = [];

    public static ContestState New() => new();
}

public sealed class TeamStatus : IComparable<TeamStatus>
{
    [JsonPropertyName("team_id")] public string TeamId { get; set; } = string.Empty;

    [JsonPropertyName("team_name")] public string TeamName { get; set; } = string.Empty;

    [JsonPropertyName("team_affiliation")] public string TeamAffiliation { get; set; } = string.Empty;

    public int Sortorder { get; set; }

    [JsonPropertyName("total_points")] public int TotalPoints { get; set; }

    [JsonPropertyName("total_penalty")] public long TotalPenalty { get; set; }

    [JsonPropertyName("last_ac_time")]
    [JsonConverter(typeof(OptionalDateTimeOffsetConverter))]
    public DateTimeOffset? LastAcTime { get; set; }

    [JsonPropertyName("problem_stats")] public Dictionary<string, ProblemStat> ProblemStats { get; set; } = [];

    public TeamStatus()
    {
    }

    public TeamStatus(string teamId, string teamName, string teamAffiliation, int sortorder)
    {
        TeamId = teamId;
        TeamName = teamName;
        TeamAffiliation = teamAffiliation;
        Sortorder = sortorder;
    }

    public void AddSubmission(
        string problemId,
        DateTimeOffset submissionTime,
        string? judgementTypeId,
        Dictionary<string, JudgementType> judgementTypes,
        DateTimeOffset? contestStartTime,
        DateTimeOffset? contestFreezeTime)
    {
        if (!ProblemStats.TryGetValue(problemId, out var problemStat))
        {
            problemStat = new ProblemStat();
            ProblemStats[problemId] = problemStat;
        }

        if (problemStat.Solved)
        {
            return;
        }

        if (judgementTypeId is null || !judgementTypes.TryGetValue(judgementTypeId, out var judgementType))
        {
            return;
        }

        if (judgementType.Penalty || judgementType.Solved)
        {
            problemStat.SubmissionsBeforeSolved += 1;

            if (contestFreezeTime is null)
            {
                throw new InvalidOperationException("No contest freeze time specified.");
            }

            problemStat.AttemptedDuringFreeze = submissionTime > contestFreezeTime.Value;

            if (contestStartTime is null)
            {
                throw new InvalidOperationException("No contest start time specified.");
            }

            problemStat.LastSubmissionTime = (long)(submissionTime - contestStartTime.Value).TotalMinutes;
        }

        if (!judgementType.Solved)
        {
            return;
        }

        problemStat.Solved = true;
        problemStat.FirstAcTime = submissionTime;

        if (contestStartTime is null)
        {
            throw new InvalidOperationException("No contest start time specified.");
        }

        var contestTime = submissionTime - contestStartTime.Value;
        var penaltyMinutes = (problemStat.SubmissionsBeforeSolved - 1) * 20;
        var problemPenalty = (long)contestTime.TotalMinutes + penaltyMinutes;
        problemStat.Penalty = problemPenalty;

        if (problemStat.AttemptedDuringFreeze)
        {
            return;
        }

        TotalPoints += 1;
        TotalPenalty += problemPenalty;
        if (LastAcTime is null || submissionTime > LastAcTime.Value)
        {
            LastAcTime = submissionTime;
        }
    }

    public int CompareTo(TeamStatus? other)
    {
        if (other is null)
        {
            return 1;
        }

        if (Sortorder != other.Sortorder)
        {
            return Sortorder.CompareTo(other.Sortorder);
        }

        if (TotalPoints != other.TotalPoints)
        {
            return other.TotalPoints.CompareTo(TotalPoints);
        }

        if (TotalPenalty != other.TotalPenalty)
        {
            return TotalPenalty.CompareTo(other.TotalPenalty);
        }

        if (LastAcTime.HasValue && other.LastAcTime.HasValue)
        {
            return DateTimeOffset.Compare(LastAcTime.Value, other.LastAcTime.Value);
        }

        if (!LastAcTime.HasValue && !other.LastAcTime.HasValue)
        {
            return string.Compare(TeamId, other.TeamId, StringComparison.Ordinal);
        }

        throw new InvalidOperationException("Invalid comparison branch for TeamStatus.");
    }

    public override bool Equals(object? obj) => obj is TeamStatus other && TeamId == other.TeamId;

    public override int GetHashCode() => TeamId.GetHashCode(StringComparison.Ordinal);
}

public sealed class ProblemStat
{
    public bool Solved { get; set; }

    [JsonPropertyName("attempted_during_freeze")]
    public bool AttemptedDuringFreeze { get; set; }

    public long Penalty { get; set; }

    [JsonPropertyName("submissions_before_solved")]
    public int SubmissionsBeforeSolved { get; set; }

    [JsonPropertyName("first_ac_time")]
    [JsonConverter(typeof(OptionalDateTimeOffsetConverter))]
    public DateTimeOffset? FirstAcTime { get; set; }

    [JsonPropertyName("last_submission_time")]
    public long LastSubmissionTime { get; set; }
}

public sealed class OptionalDateTimeOffsetConverter : JsonConverter<DateTimeOffset?>
{
    public override DateTimeOffset? Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        if (reader.TokenType == JsonTokenType.Null)
        {
            return null;
        }

        if (reader.TokenType != JsonTokenType.String)
        {
            throw new JsonException("Expected a string or null for DateTimeOffset value.");
        }

        var value = reader.GetString();
        if (string.IsNullOrWhiteSpace(value))
        {
            return null;
        }

        return DateTimeOffset.Parse(value, CultureInfo.InvariantCulture, DateTimeStyles.RoundtripKind);
    }

    public override void Write(Utf8JsonWriter writer, DateTimeOffset? value, JsonSerializerOptions options)
    {
        if (value.HasValue)
        {
            writer.WriteStringValue(value.Value.ToString("O", CultureInfo.InvariantCulture));
            return;
        }

        writer.WriteNullValue();
    }
}

public sealed class ContestDurationConverter : JsonConverter<TimeSpan>
{
    public override TimeSpan Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        if (reader.TokenType != JsonTokenType.String)
        {
            throw new JsonException("Expected a duration string.");
        }

        return ParseContestDuration(reader.GetString());
    }

    public override void Write(Utf8JsonWriter writer, TimeSpan value, JsonSerializerOptions options)
    {
        var sign = value < TimeSpan.Zero ? "-" : string.Empty;
        var abs = value.Duration();
        writer.WriteStringValue($"{sign}{(int)abs.TotalHours:00}:{abs.Minutes:00}:{abs.Seconds:00}");
    }

    internal static TimeSpan ParseContestDuration(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            throw new JsonException("Duration string was null or empty.");
        }

        var isNegative = value.StartsWith("-", StringComparison.Ordinal);
        var trimmed = isNegative ? value[1..] : value;
        var parts = trimmed.Split(':');
        if (parts.Length != 3)
        {
            throw new JsonException($"Invalid duration format: {value}");
        }

        if (!long.TryParse(parts[0], NumberStyles.Integer, CultureInfo.InvariantCulture, out var hours) ||
            !long.TryParse(parts[1], NumberStyles.Integer, CultureInfo.InvariantCulture, out var minutes) ||
            !double.TryParse(parts[2], NumberStyles.Float, CultureInfo.InvariantCulture, out var seconds))
        {
            throw new JsonException($"Invalid duration format: {value}");
        }

        var totalSeconds = (hours * 3600) + (minutes * 60) + (long)seconds;
        var duration = TimeSpan.FromSeconds(totalSeconds);
        return isNegative ? -duration : duration;
    }
}

public sealed class OptionalContestDurationConverter : JsonConverter<TimeSpan?>
{
    public override TimeSpan? Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        if (reader.TokenType == JsonTokenType.Null)
        {
            return null;
        }

        if (reader.TokenType != JsonTokenType.String)
        {
            throw new JsonException("Expected a duration string or null.");
        }

        return ContestDurationConverter.ParseContestDuration(reader.GetString());
    }

    public override void Write(Utf8JsonWriter writer, TimeSpan? value, JsonSerializerOptions options)
    {
        if (!value.HasValue)
        {
            writer.WriteNullValue();
            return;
        }

        new ContestDurationConverter().Write(writer, value.Value, options);
    }
}
