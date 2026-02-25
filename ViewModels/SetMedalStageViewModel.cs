using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Pyrite.Models;
using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.ComponentModel;
using System.IO;
using System.Linq;
using System.Text.Json;

namespace Pyrite.ViewModels;

public sealed class SetMedalStageViewModel : ViewModelBase
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNameCaseInsensitive = true,
        WriteIndented = true
    };

    private ContestState? _contestState;
    private int _eligibleTeamCount;
    private string _finalizedCacheKey = string.Empty;
    private List<TeamStatus> _finalizedLeaderboard = [];
    private string _groupKey = string.Empty;
    private string _manualCitation = string.Empty;

    private string _manualMedalId = string.Empty;
    private string _manualTeamIdsCsv = string.Empty;
    private string _medalBronzeCitation = "Bronze Medal";
    private int _medalBronzeCount;
    private string _medalGoldCitation = "Gold Medal";

    private int _medalGoldCount;
    private string _medalSilverCitation = "Silver Medal";
    private int _medalSilverCount;
    private string _statusMessage = string.Empty;

    public SetMedalStageViewModel()
    {
        SelectAllGroupsCommand = new RelayCommand(SelectAllGroups);
        ClearAllGroupsCommand = new RelayCommand(ClearAllGroups);
        ApplyMedalsCommand = new RelayCommand(ApplyMedals);
        AddOrUpdateMedalCommand = new RelayCommand(AddOrUpdateMedal);
        DeleteMedalCommand = new RelayCommand<string>(DeleteMedal);
    }

    public ObservableCollection<GroupSelectionItemViewModel> Groups { get; } = [];
    public ObservableCollection<TeamPreviewItem> GoldPreview { get; } = [];
    public ObservableCollection<TeamPreviewItem> SilverPreview { get; } = [];
    public ObservableCollection<TeamPreviewItem> BronzePreview { get; } = [];
    public ObservableCollection<MedalSummaryItem> Medals { get; } = [];

    public RelayCommand SelectAllGroupsCommand { get; }
    public RelayCommand ClearAllGroupsCommand { get; }
    public RelayCommand ApplyMedalsCommand { get; }
    public RelayCommand AddOrUpdateMedalCommand { get; }
    public RelayCommand<string> DeleteMedalCommand { get; }

    public bool HasContestState => _contestState is not null;

    public string StatusMessage
    {
        get => _statusMessage;
        private set
        {
            if (SetProperty(ref _statusMessage, value)) OnPropertyChanged(nameof(HasStatusMessage));
        }
    }

    public bool HasStatusMessage => !string.IsNullOrWhiteSpace(StatusMessage);

    public int MedalGoldCount
    {
        get => _medalGoldCount;
        set
        {
            if (SetProperty(ref _medalGoldCount, Math.Max(0, value))) RecomputeMedalPreview();
        }
    }

    public int MedalSilverCount
    {
        get => _medalSilverCount;
        set
        {
            if (SetProperty(ref _medalSilverCount, Math.Max(0, value))) RecomputeMedalPreview();
        }
    }

    public int MedalBronzeCount
    {
        get => _medalBronzeCount;
        set
        {
            if (SetProperty(ref _medalBronzeCount, Math.Max(0, value))) RecomputeMedalPreview();
        }
    }

    public string MedalGoldCitation
    {
        get => _medalGoldCitation;
        set => SetProperty(ref _medalGoldCitation, value);
    }

    public string MedalSilverCitation
    {
        get => _medalSilverCitation;
        set => SetProperty(ref _medalSilverCitation, value);
    }

    public string MedalBronzeCitation
    {
        get => _medalBronzeCitation;
        set => SetProperty(ref _medalBronzeCitation, value);
    }

    public string ManualMedalId
    {
        get => _manualMedalId;
        set => SetProperty(ref _manualMedalId, value);
    }

    public string ManualCitation
    {
        get => _manualCitation;
        set => SetProperty(ref _manualCitation, value);
    }

    public string ManualTeamIdsCsv
    {
        get => _manualTeamIdsCsv;
        set => SetProperty(ref _manualTeamIdsCsv, value);
    }

    public int EligibleTeamCount
    {
        get => _eligibleTeamCount;
        private set
        {
            if (SetProperty(ref _eligibleTeamCount, value))
            {
                OnPropertyChanged(nameof(RequestedMedalCount));
                OnPropertyChanged(nameof(RequestedMedalsExceedEligible));
            }
        }
    }

    public int RequestedMedalCount => MedalGoldCount + MedalSilverCount + MedalBronzeCount;
    public bool RequestedMedalsExceedEligible => RequestedMedalCount > EligibleTeamCount;

    public void SetStatusMessage(string message)
    {
        StatusMessage = message;
    }

    public void SetContestState(ContestState? contestState)
    {
        _contestState = contestState;
        _groupKey = string.Empty;
        _finalizedCacheKey = string.Empty;
        _finalizedLeaderboard = [];
        StatusMessage = string.Empty;

        foreach (var group in Groups) group.PropertyChanged -= OnGroupSelectionChanged;

        Groups.Clear();
        Medals.Clear();
        GoldPreview.Clear();
        SilverPreview.Clear();
        BronzePreview.Clear();
        EligibleTeamCount = 0;

        OnPropertyChanged(nameof(HasContestState));

        if (_contestState is null) return;

        SyncGroupsFromContest();
        RefreshMedals();
        RecomputeMedalPreview();
    }

    public void SaveMedalsToFile(string path)
    {
        if (_contestState is null)
        {
            StatusMessage = "No contest state loaded.";
            return;
        }

        var json = JsonSerializer.Serialize(_contestState.Awards, JsonOptions);
        File.WriteAllText(path, json);
        StatusMessage = $"Saved medals to {path}";
    }

    public void LoadMedalsFromFile(string path)
    {
        if (_contestState is null)
        {
            StatusMessage = "No contest state loaded.";
            return;
        }

        var raw = File.ReadAllText(path);
        var parsed = JsonSerializer.Deserialize<Dictionary<string, Award>>(raw, JsonOptions);
        if (parsed is null) throw new InvalidOperationException("Medals JSON was empty.");

        var normalized = new Dictionary<string, Award>(StringComparer.Ordinal);
        foreach (var medal in parsed.Values)
        {
            if (string.IsNullOrWhiteSpace(medal.Id)) continue;

            normalized[medal.Id] = medal;
        }

        _contestState.Awards = normalized;
        RefreshMedals();
        StatusMessage = $"Loaded {_contestState.Awards.Count} medal(s) from {path}";
    }

    public bool TryPreparePresentation(out string errorMessage)
    {
        errorMessage = string.Empty;
        if (_contestState is null)
        {
            errorMessage = "No contest state loaded.";
            return false;
        }

        try
        {
            var dumpMessage = DumpContestStateBeforePresentation(_contestState);
            var filterMessage = ApplyGroupFilterForPresentation(_contestState);
            StatusMessage = $"{dumpMessage}; {filterMessage}";
            _finalizedCacheKey = string.Empty;
            _finalizedLeaderboard = [];
            RefreshMedals();
            RecomputeMedalPreview();
            return true;
        }
        catch (Exception ex)
        {
            errorMessage = ex.Message;
            StatusMessage = $"Presentation preparation failed: {ex.Message}";
            return false;
        }
    }

    private void SyncGroupsFromContest()
    {
        if (_contestState is null) return;

        var sortedGroups = _contestState.Groups.Values
            .OrderBy(x => x.Sortorder)
            .ThenBy(x => x.Name, StringComparer.Ordinal)
            .ThenBy(x => x.Id, StringComparer.Ordinal)
            .ToList();

        var currentKey = string.Join(
            "|",
            sortedGroups.Select(x => $"{x.Sortorder}:{x.Name}:{x.Id}"));

        var currentIds = sortedGroups.Select(x => x.Id).ToHashSet(StringComparer.Ordinal);
        var existingIds = Groups.Select(x => x.Id).ToHashSet(StringComparer.Ordinal);
        if (_groupKey == currentKey && currentIds.SetEquals(existingIds)) return;

        var previousSelections = Groups.ToDictionary(x => x.Id, x => x.IsSelected, StringComparer.Ordinal);
        var resetToAllSelected = _groupKey != currentKey;

        foreach (var group in Groups) group.PropertyChanged -= OnGroupSelectionChanged;

        Groups.Clear();
        foreach (var group in sortedGroups)
        {
            var selected = resetToAllSelected || !previousSelections.TryGetValue(group.Id, out var oldValue)
                ? true
                : oldValue;

            var item = new GroupSelectionItemViewModel(group.Id, group.Name, selected);
            item.PropertyChanged += OnGroupSelectionChanged;
            Groups.Add(item);
        }

        _groupKey = currentKey;
    }

    private void OnGroupSelectionChanged(object? sender, PropertyChangedEventArgs args)
    {
        if (args.PropertyName == nameof(GroupSelectionItemViewModel.IsSelected)) RecomputeMedalPreview();
    }

    private void SelectAllGroups()
    {
        foreach (var group in Groups) group.IsSelected = true;
    }

    private void ClearAllGroups()
    {
        foreach (var group in Groups) group.IsSelected = false;
    }

    private void RecomputeMedalPreview()
    {
        if (_contestState is null)
        {
            GoldPreview.Clear();
            SilverPreview.Clear();
            BronzePreview.Clear();
            EligibleTeamCount = 0;
            return;
        }

        SyncGroupsFromContest();
        try
        {
            EnsureFinalizedLeaderboard();
        }
        catch (Exception ex)
        {
            _finalizedLeaderboard = [];
            _finalizedCacheKey = string.Empty;
            StatusMessage = $"Failed to compute finalized leaderboard: {ex.Message}";
        }

        var selectedGroupIds = Groups
            .Where(x => x.IsSelected)
            .Select(x => x.Id)
            .ToHashSet(StringComparer.Ordinal);

        var eligible = _finalizedLeaderboard
            .Where(teamStatus =>
                _contestState.Teams.TryGetValue(teamStatus.TeamId, out var team) &&
                team.GroupIds.Any(groupId => selectedGroupIds.Contains(groupId)))
            .Select(teamStatus => new TeamPreviewItem(teamStatus.TeamId, teamStatus.TeamName))
            .ToList();

        EligibleTeamCount = eligible.Count;

        var goldEnd = Math.Min(MedalGoldCount, eligible.Count);
        var silverEnd = Math.Min(goldEnd + MedalSilverCount, eligible.Count);
        var bronzeEnd = Math.Min(silverEnd + MedalBronzeCount, eligible.Count);

        ReplacePreviewCollection(GoldPreview, eligible.Take(goldEnd));
        ReplacePreviewCollection(SilverPreview, eligible.Skip(goldEnd).Take(silverEnd - goldEnd));
        ReplacePreviewCollection(BronzePreview, eligible.Skip(silverEnd).Take(bronzeEnd - silverEnd));

        OnPropertyChanged(nameof(RequestedMedalCount));
        OnPropertyChanged(nameof(RequestedMedalsExceedEligible));
    }

    private static void ReplacePreviewCollection(
        ObservableCollection<TeamPreviewItem> target,
        IEnumerable<TeamPreviewItem> source)
    {
        target.Clear();
        foreach (var item in source) target.Add(item);
    }

    private void EnsureFinalizedLeaderboard()
    {
        if (_contestState is null)
        {
            _finalizedLeaderboard = [];
            return;
        }

        var key =
            $"{_contestState.Teams.Count}:{_contestState.Groups.Count}:{_contestState.Submissions.Count}:{_contestState.Judgements.Count}:{_contestState.LeaderboardPreFreeze.Count}";

        if (_finalizedCacheKey == key && _finalizedLeaderboard.Count > 0) return;

        _finalizedLeaderboard = _contestState.LeaderboardFinalized.Count > 0
            ? _contestState.LeaderboardFinalized
            : _contestState.LeaderboardPreFreeze;
        _finalizedCacheKey = key;
    }

    private void ApplyMedals()
    {
        if (_contestState is null)
        {
            StatusMessage = "No contest state loaded.";
            return;
        }

        _contestState.Awards["medal-gold"] = new Award
        {
            Id = "medal-gold",
            Citation = MedalGoldCitation.Trim(),
            TeamIds = GoldPreview.Select(x => x.TeamId).ToList()
        };

        _contestState.Awards["medal-silver"] = new Award
        {
            Id = "medal-silver",
            Citation = MedalSilverCitation.Trim(),
            TeamIds = SilverPreview.Select(x => x.TeamId).ToList()
        };

        _contestState.Awards["medal-bronze"] = new Award
        {
            Id = "medal-bronze",
            Citation = MedalBronzeCitation.Trim(),
            TeamIds = BronzePreview.Select(x => x.TeamId).ToList()
        };

        RefreshMedals();
        StatusMessage = "Medals applied to contest state.";
    }

    private void AddOrUpdateMedal()
    {
        if (_contestState is null)
        {
            StatusMessage = "No contest state loaded.";
            return;
        }

        var medalId = ManualMedalId.Trim();
        var citation = ManualCitation.Trim();
        var teamIds = ManualTeamIdsCsv
            .Split(',', StringSplitOptions.TrimEntries | StringSplitOptions.RemoveEmptyEntries)
            .Distinct(StringComparer.Ordinal)
            .ToList();

        if (string.IsNullOrWhiteSpace(medalId) || string.IsNullOrWhiteSpace(citation) || teamIds.Count == 0)
        {
            StatusMessage = "Medal ID, citation, and at least one team ID are required.";
            return;
        }

        _contestState.Awards[medalId] = new Award
        {
            Id = medalId,
            Citation = citation,
            TeamIds = teamIds
        };

        RefreshMedals();
        StatusMessage = "Medal upserted to contest state.";
    }

    private void DeleteMedal(string? medalId)
    {
        if (_contestState is null || string.IsNullOrWhiteSpace(medalId)) return;

        if (_contestState.Awards.Remove(medalId))
        {
            RefreshMedals();
            StatusMessage = $"Deleted medal {medalId}.";
        }
    }

    private void RefreshMedals()
    {
        Medals.Clear();

        if (_contestState is null) return;

        foreach (var medal in _contestState.Awards.Values.OrderBy(x => x.Id, StringComparer.Ordinal))
        {
            var preview = medal.TeamIds.Count == 0
                ? "None"
                : string.Join(", ", medal.TeamIds.Take(5)) + (medal.TeamIds.Count > 5 ? " ..." : string.Empty);

            Medals.Add(new MedalSummaryItem(medal.Id, medal.Citation, medal.TeamIds.Count, preview));
        }
    }

    private static string DumpContestStateBeforePresentation(ContestState contestState)
    {
        var dumpPath = Path.Combine("logs", "contest_state_before_present.json");
        Directory.CreateDirectory("logs");
        var json = JsonSerializer.Serialize(contestState, JsonOptions);
        File.WriteAllText(dumpPath, json);
        return $"Dumped contest state to {dumpPath}";
    }

    private string ApplyGroupFilterForPresentation(ContestState contestState)
    {
        var selectedGroups = Groups
            .Where(x => x.IsSelected)
            .Select(x => x.Id)
            .ToHashSet(StringComparer.Ordinal);

        var allowedTeamIds = contestState.Teams.Values
            .Where(team => team.GroupIds.Any(groupId => selectedGroups.Contains(groupId)))
            .Select(team => team.Id)
            .ToHashSet(StringComparer.Ordinal);

        var originalTeamCount = contestState.Teams.Count;
        var originalSubmissionCount = contestState.Submissions.Count;
        var originalJudgementCount = contestState.Judgements.Count;

        contestState.Teams = contestState.Teams
            .Where(x => allowedTeamIds.Contains(x.Key))
            .ToDictionary(k => k.Key, v => v.Value, StringComparer.Ordinal);

        contestState.Accounts = contestState.Accounts
            .Where(x => allowedTeamIds.Contains(x.Value.TeamId))
            .ToDictionary(k => k.Key, v => v.Value, StringComparer.Ordinal);

        contestState.Submissions = contestState.Submissions
            .Where(x => allowedTeamIds.Contains(x.Value.TeamId))
            .ToDictionary(k => k.Key, v => v.Value, StringComparer.Ordinal);

        var allowedSubmissionIds = contestState.Submissions.Keys.ToHashSet(StringComparer.Ordinal);

        contestState.Judgements = contestState.Judgements
            .Where(x => allowedSubmissionIds.Contains(x.Value.SubmissionId))
            .ToDictionary(k => k.Key, v => v.Value, StringComparer.Ordinal);

        contestState.LeaderboardPreFreeze = contestState.LeaderboardPreFreeze
            .Where(x => allowedTeamIds.Contains(x.TeamId))
            .ToList();

        foreach (var medal in contestState.Awards.Values)
            medal.TeamIds = medal.TeamIds
                .Where(teamId => allowedTeamIds.Contains(teamId))
                .ToList();

        return
            $"Filtered presentation set: teams {originalTeamCount} -> {contestState.Teams.Count}, submissions {originalSubmissionCount} -> {contestState.Submissions.Count}, judgements {originalJudgementCount} -> {contestState.Judgements.Count}";
    }
}

public sealed class GroupSelectionItemViewModel : ObservableObject
{
    private bool _isSelected;

    public GroupSelectionItemViewModel(string id, string name, bool isSelected)
    {
        Id = id;
        Name = name;
        _isSelected = isSelected;
    }

    public string Id { get; }
    public string Name { get; }

    public bool IsSelected
    {
        get => _isSelected;
        set => SetProperty(ref _isSelected, value);
    }

    public string DisplayLabel => $"{Name} ({Id})";
}

public sealed class TeamPreviewItem
{
    public TeamPreviewItem(string teamId, string teamName)
    {
        TeamId = teamId;
        TeamName = teamName;
    }

    public string TeamId { get; }
    public string TeamName { get; }
    public string DisplayLabel => $"{TeamId} | {TeamName}";
}

public sealed class MedalSummaryItem
{
    public MedalSummaryItem(string id, string citation, int teamCount, string teamPreview)
    {
        Id = id;
        Citation = citation;
        TeamCount = teamCount;
        TeamPreview = teamPreview;
    }

    public string Id { get; }
    public string Citation { get; }
    public int TeamCount { get; }
    public string TeamPreview { get; }
}