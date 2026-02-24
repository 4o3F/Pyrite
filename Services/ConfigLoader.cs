using System;
using System.IO;
using Pyrite.Models;
using Tomlyn;
using Tomlyn.Model;

namespace Pyrite.Services;

public static class ConfigLoader
{
    public static PyriteConfig LoadIfExists(string cdpPath)
    {
        var configPath = Path.Combine(cdpPath, "config.toml");
        if (!File.Exists(configPath)) return PyriteConfig.Default();

        var raw = File.ReadAllText(configPath);
        if (!Toml.TryToModel<TomlTable>(raw, out var table, out var diagnostics) || table is null)
        {
            var diagnosticMessage = diagnostics is null ? "Unknown parse error" : string.Join(" | ", diagnostics);
            throw new InvalidOperationException($"Invalid config.toml: {diagnosticMessage}");
        }

        var config = PyriteConfig.Default();

        if (table.TryGetValue("filter_team_submissions", out var filterTeams) && filterTeams is TomlArray filterArray)
            foreach (var value in filterArray)
                if (value is string teamId)
                    config.FilterTeamSubmissions.Add(teamId);

        if (table.TryGetValue("team_group_map", out var mapObject) && mapObject is TomlTable mapTable)
            foreach (var kv in mapTable)
                if (kv.Value is string groupId)
                    config.TeamGroupMap[kv.Key] = groupId;

        if (table.TryGetValue("presentation", out var presentationObject) &&
            presentationObject is TomlTable presentationTable)
            config.Presentation = PresentationConfig.FromToml(presentationTable);

        return config;
    }
}