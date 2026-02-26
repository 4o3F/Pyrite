using Pyrite.Models;
using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;

namespace Pyrite.Services;

public sealed class ParseProgressUpdate
{
    public required long LinesRead { get; init; }
    public required long TotalLines { get; init; }
}

public sealed class ParseResult
{
    public required ContestState ContestState { get; init; }
    public required long LinesRead { get; init; }
    public required long ErrorCount { get; init; }
    public required List<string> Errors { get; init; }
    public required List<string> Warnings { get; init; }
}

public static class EventFeedParser
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNameCaseInsensitive = true
    };

    public static async Task<ParseResult> ParseAsync(
        string eventFeedPath,
        PyriteConfig config,
        IProgress<ParseProgressUpdate>? progress,
        CancellationToken cancellationToken)
    {
        var totalLines = await CountLinesAsync(eventFeedPath, cancellationToken);
        var state = ContestState.New();
        var errors = new List<string>();
        long linesRead = 0;

        await using var fs = File.OpenRead(eventFeedPath);
        using var reader = new StreamReader(fs);

        while (true)
        {
            cancellationToken.ThrowIfCancellationRequested();

            var line = await reader.ReadLineAsync(cancellationToken);
            if (line is null) break;

            linesRead += 1;

            ParseEventLine(line, linesRead, state, errors);

            if (linesRead % 100 == 0 || linesRead == totalLines)
                progress?.Report(new ParseProgressUpdate
                {
                    LinesRead = linesRead,
                    TotalLines = totalLines
                });
        }

        if (errors.Count > 0)
            return new ParseResult
            {
                ContestState = state,
                LinesRead = linesRead,
                ErrorCount = errors.Count,
                Errors = errors,
                Warnings = []
            };

        var warnings = ContestProcessor.ValidateAndTransform(state, config);

        return new ParseResult
        {
            ContestState = state,
            LinesRead = linesRead,
            ErrorCount = errors.Count,
            Errors = errors,
            Warnings = warnings
        };
    }

    private static async Task<long> CountLinesAsync(string path, CancellationToken cancellationToken)
    {
        long total = 0;
        await using var fs = File.OpenRead(path);
        using var reader = new StreamReader(fs);

        while (true)
        {
            cancellationToken.ThrowIfCancellationRequested();
            var line = await reader.ReadLineAsync(cancellationToken);
            if (line is null) break;

            total += 1;
        }

        return Math.Max(total, 1);
    }

    private static void ParseEventLine(string line, long lineNumber, ContestState state, List<string> errors)
    {
        Event? parsedEvent;
        try
        {
            parsedEvent = JsonSerializer.Deserialize<Event>(line, JsonOptions);
        }
        catch (Exception ex)
        {
            AddLineError(errors, lineNumber, ex.Message);
            return;
        }

        if (parsedEvent is null)
        {
            AddLineError(errors, lineNumber, "Invalid event payload");
            return;
        }

        if (!parsedEvent.Data.HasValue) return;

        var eventData = parsedEvent.Data.Value;
        var contestDefined = state.Contest is not null;

        switch (parsedEvent.EventType)
        {
            case EventType.Contest:
                TryParseContest(eventData, lineNumber, state, errors);
                break;
            case EventType.JudgementTypes:
                HandleEvent(eventData, lineNumber, state.JudgementTypes, contestDefined, errors, "judgement-types");
                break;
            case EventType.Groups:
                HandleEvent(eventData, lineNumber, state.Groups, contestDefined, errors, "groups");
                break;
            case EventType.Organizations:
                HandleEvent(eventData, lineNumber, state.Organizations, contestDefined, errors, "organizations");
                break;
            case EventType.Teams:
                HandleEvent(eventData, lineNumber, state.Teams, contestDefined, errors, "teams");
                break;
            case EventType.Accounts:
                HandleEvent(eventData, lineNumber, state.Accounts, contestDefined, errors, "accounts");
                break;
            case EventType.Problems:
                HandleEvent(eventData, lineNumber, state.Problems, contestDefined, errors, "problems");
                break;
            case EventType.Submissions:
                HandleEvent(eventData, lineNumber, state.Submissions, contestDefined, errors, "submissions");
                break;
            case EventType.Judgements:
                HandleEvent(eventData, lineNumber, state.Judgements, contestDefined, errors, "judgements");
                break;
            case EventType.Awards:
                HandleEvent(eventData, lineNumber, state.Awards, contestDefined, errors, "awards");
                break;
            case EventType.Languages:
            case EventType.Runs:
            case EventType.State:
            case EventType.Clarifications:
            case EventType.Persons:
                break;
            default:
                AddLineError(errors, lineNumber, $"Unsupported event type '{parsedEvent.EventType}'");
                break;
        }
    }

    private static void TryParseContest(JsonElement eventData, long lineNumber, ContestState state, List<string> errors)
    {
        try
        {
            var contest = eventData.Deserialize<Contest>(JsonOptions);
            if (contest is null)
            {
                AddLineError(errors, lineNumber, "Empty contest payload");
                return;
            }

            if (contest.StartTime.HasValue)
                contest.ScoreboardFreezeTime =
                    contest.StartTime.Value + (contest.Duration - contest.ScoreboardFreezeDuration);

            state.Contest = contest;
        }
        catch (Exception ex)
        {
            AddLineError(errors, lineNumber, $"Failed to parse contest payload: {ex.Message}");
        }
    }

    private static void HandleEvent<T>(
        JsonElement eventData,
        long lineNumber,
        Dictionary<string, T> stateMap,
        bool contestDefined,
        List<string> errors,
        string eventName)
        where T : class, IHasId
    {
        if (!contestDefined)
        {
            AddLineError(errors, lineNumber, $"Contest must be defined before {eventName}");
            return;
        }

        try
        {
            var item = eventData.Deserialize<T>(JsonOptions);
            if (item is null)
            {
                AddLineError(errors, lineNumber, $"Empty {eventName} payload");
                return;
            }

            stateMap[item.Id] = item;
        }
        catch (Exception ex)
        {
            AddLineError(errors, lineNumber, $"Failed to parse {eventName} payload: {ex.Message}");
        }
    }

    private static void AddLineError(List<string> errors, long lineNumber, string message)
    {
        errors.Add($"Line {lineNumber}: {message}");
    }
}
