using Avalonia.Media.Imaging;
using CommunityToolkit.Mvvm.Input;
using Pyrite.Models;
using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Diagnostics;
using System.IO;
using System.Linq;

namespace Pyrite.ViewModels;

public sealed class PresentationStageViewModel : ViewModelBase
{
    private ContestState? _contestState;
    private string? _dataPath;
    private int _focusedRowIndex = -1;
    private bool _isInitialized;
    private bool _isStarted;
    private PyriteConfig _loadedConfig = PyriteConfig.Default();
    private readonly Dictionary<string, Queue<string>> _pendingRevealsByTeamId = new(StringComparer.Ordinal);
    private readonly List<ProblemDisplayInfo> _orderedProblems = [];
    private string? _pendingResortSolvedTeamId;
    private MoveUpAnimationRequest? _moveUpAnimationRequest;
    private long _moveUpAnimationRequestCounter;
    private PresentationRowState _state = PresentationRowState.RowInProgress;
    private double _viewportHeight;
    private double _viewportWidth;

    public PresentationStageViewModel()
    {
        ExitCommand = new RelayCommand(RequestExit);
        RevealCommand = new RelayCommand(() => RunReveal(), CanReveal);
        MoveUpCommand = new RelayCommand(RunMoveUp, CanMoveUp);
        RefreshSessionStatus();
    }

    public event Action? ExitRequested;

    public RelayCommand ExitCommand { get; }
    public RelayCommand RevealCommand { get; }
    public RelayCommand MoveUpCommand { get; }
    public ObservableCollection<PreFreezeScoreboardRowViewModel> PreFreezeRows { get; } = [];
    public MoveUpAnimationRequest? MoveUpAnimationRequest
    {
        get => _moveUpAnimationRequest;
        private set => SetProperty(ref _moveUpAnimationRequest, value);
    }
    public double RowFlyAnimationSeconds => Math.Max(0.01, _loadedConfig.Presentation.RowFlyAnimationSeconds);
    public double ScrollAnimationSeconds => Math.Max(0.01, _loadedConfig.Presentation.ScrollAnimationSeconds);

    public PresentationRowState State
    {
        get => _state;
        private set
        {
            if (SetProperty(ref _state, value))
            {
                RefreshSessionStatus();
            }
        }
    }

    public int FocusedRowIndex
    {
        get => _focusedRowIndex;
        private set
        {
            if (SetProperty(ref _focusedRowIndex, value))
            {
                RefreshSessionStatus();
            }
        }
    }

    public bool IsInitialized
    {
        get => _isInitialized;
        private set
        {
            if (SetProperty(ref _isInitialized, value))
            {
                OnPropertyChanged(nameof(SessionStatus));
            }
        }
    }

    public bool IsStarted
    {
        get => _isStarted;
        private set
        {
            if (SetProperty(ref _isStarted, value))
            {
                OnPropertyChanged(nameof(SessionStatus));
            }
        }
    }

    public string SessionStatus =>
        $"Initialized={IsInitialized}, Started={IsStarted}, State={State}, FocusIndex={FocusedRowIndex}, " +
        $"Viewport={_viewportWidth:F0}x{_viewportHeight:F0}";

    public void Initialize(ContestState contestState, PyriteConfig config, string? dataPath)
    {
        ArgumentNullException.ThrowIfNull(contestState);
        ArgumentNullException.ThrowIfNull(config);

        _contestState = contestState;
        _loadedConfig = config;
        OnPropertyChanged(nameof(RowFlyAnimationSeconds));
        OnPropertyChanged(nameof(ScrollAnimationSeconds));
        _dataPath = dataPath;
        InitializePresentationRows(contestState);
        FocusedRowIndex = FindInitialFocusedRowIndex();
        State = PresentationRowState.RowInProgress;
        IsInitialized = true;
        RevealCommand.NotifyCanExecuteChanged();
        MoveUpCommand.NotifyCanExecuteChanged();
        RefreshSessionStatus();
    }

    public void Start()
    {
        if (!IsInitialized)
        {
            return;
        }

        IsStarted = true;
        RevealCommand.NotifyCanExecuteChanged();
        MoveUpCommand.NotifyCanExecuteChanged();
        RefreshSessionStatus();
    }

    public void Stop()
    {
        IsStarted = false;
        RevealCommand.NotifyCanExecuteChanged();
        MoveUpCommand.NotifyCanExecuteChanged();
        RefreshSessionStatus();
    }

    public void UpdateViewport(double width, double height)
    {
        if (width <= 0 || height <= 0)
        {
            return;
        }

        _viewportWidth = width;
        _viewportHeight = height;
        RefreshSessionStatus();
    }

    public void UpdateViewport(double width, double height, double totalHeight)
    {
        UpdateViewport(width, height);
    }

    public void HandleSpacePressed()
    {
        if (!IsInitialized || !IsStarted)
        {
            return;
        }

        // Template for your transition logic:
        // - decide next state
        // - decide whether to call Reveal or MoveUp
        Trace.WriteLine($"Current state {State} focused on {FocusedRowIndex}");
        switch (State)
        {
            case PresentationRowState.RowInProgress:
                if (FocusedRowIndex < 0 || FocusedRowIndex >= PreFreezeRows.Count)
                {
                    Trace.WriteLine($"ERROR: FocusedRowIndex {FocusedRowIndex} PreFreezeRows Count {PreFreezeRows.Count}");
                    State = PresentationRowState.RowInProgress;
                    break;
                }

                var teamId = PreFreezeRows[FocusedRowIndex].TeamId;
                if (HasPendingReveal(teamId))
                {
                    Trace.WriteLine($"Do reveal on FocusedRowIndex {FocusedRowIndex}");
                    var revealOutcome = RunReveal();
                    if (revealOutcome.NeedResort)
                    {
                        _pendingResortSolvedTeamId = revealOutcome.SolvedTeamId;
                        State = PresentationRowState.RowInProgressAwaitResort;
                    }
                    else
                    {
                        _pendingResortSolvedTeamId = null;
                        State = PresentationRowState.RowInProgress;
                    }
                }
                else
                {
                    // Team has no pending reveals, finished
                    if (!HasPendingReveal(teamId))
                    {
                        Trace.WriteLine($"Team rank {FocusedRowIndex} has no more to reveal");
                        if (HasAwards(teamId))
                        {
                            // Team has award, show it
                            State = PresentationRowState.RowCompleteAwardShowing;
                        }
                        else
                        {
                            Trace.WriteLine($"Execute MoveUp");
                            RunMoveUp();
                            State = PresentationRowState.RowInProgress;
                        }
                    }
                }

                break;
            case PresentationRowState.RowInProgressAwaitResort:
                ResortScoreboard(_pendingResortSolvedTeamId);
                _pendingResortSolvedTeamId = null;
                State = PresentationRowState.RowInProgress;
                break;
            case PresentationRowState.RowCompleteAwardShowing:
                // TODO: hide/finish award presentation logic
                State = PresentationRowState.RowCompleteReadyToAdvance;
                break;
            case PresentationRowState.RowCompleteReadyToAdvance:
                RunMoveUp();
                State = PresentationRowState.RowInProgress;
                break;
            default:
                throw new ArgumentOutOfRangeException();
        }
        Trace.WriteLine($"New state {State}");
    }

    private void RequestExit()
    {
        ExitRequested?.Invoke();
    }

    private void RefreshSessionStatus()
    {
        OnPropertyChanged(nameof(SessionStatus));
    }

    private void InitializePresentationRows(ContestState contestState)
    {
        _orderedProblems.Clear();
        _orderedProblems.AddRange(contestState.Problems.Values
            .OrderBy(problem => problem.Ordinal)
            .ThenBy(problem => problem.Label, StringComparer.Ordinal)
            .Select(problem => new ProblemDisplayInfo(
                problem.Id,
                string.IsNullOrWhiteSpace(problem.Label) ? problem.ShortName : problem.Label))
            .ToList());

        PreFreezeRows.Clear();

        _pendingRevealsByTeamId.Clear();
        for (var i = 0; i < contestState.LeaderboardPreFreeze.Count; i++)
        {
            var team = CloneTeamStatus(contestState.LeaderboardPreFreeze[i]);
            var pendingProblemIds = team.ProblemStats
                .Where(kv => kv.Value.AttemptedDuringFreeze)
                .OrderBy(kv => kv.Key)
                .Select(kv => kv.Key);

            _pendingRevealsByTeamId[team.TeamId] = new Queue<string>(pendingProblemIds);

            var rowVm = new PreFreezeScoreboardRowViewModel(
                team,
                i + 1,
                _orderedProblems,
                _dataPath,
                _loadedConfig.Presentation.LogoExtension);
            PreFreezeRows.Add(rowVm);
        }
    }

    private int FindInitialFocusedRowIndex()
    {
        for (var row = PreFreezeRows.Count - 1; row >= 0; row--)
        {
            var teamId = PreFreezeRows[row].TeamId;
            if (HasPendingReveal(teamId))
            {
                return row;
            }
        }

        return -1;
    }

    private RevealOutcome Reveal()
    {
        if (FocusedRowIndex < 0 || FocusedRowIndex >= PreFreezeRows.Count)
        {
            return RevealOutcome.None;
        }

        var teamRow = PreFreezeRows[FocusedRowIndex];
        var team = teamRow.TeamStatus;
        if (!_pendingRevealsByTeamId.TryGetValue(team.TeamId, out var pending) || pending.Count == 0)
        {
            return RevealOutcome.None;
        }

        var problemId = pending.Dequeue();
        if (!team.ProblemStats.TryGetValue(problemId, out var stat))
        {
            return RevealOutcome.None;
        }

        stat.AttemptedDuringFreeze = false;
        var solved = false;
        if (stat.Solved)
        {
            solved = true;
            team.TotalPoints += 1;
            team.TotalPenalty += stat.Penalty;

            if (stat.FirstAcTime.HasValue && (!team.LastAcTime.HasValue || stat.FirstAcTime > team.LastAcTime))
            {
                team.LastAcTime = stat.FirstAcTime;
            }
        }

        teamRow.RefreshFromSource();
        return new RevealOutcome(true, solved, solved, solved ? team.TeamId : null);
    }

    private bool MoveUp()
    {
        if (FocusedRowIndex <= 0 || FocusedRowIndex >= PreFreezeRows.Count)
        {
            return false;
        }

        FocusedRowIndex -= 1;
        Trace.WriteLine($"Moved up to {FocusedRowIndex}");
        return true;
    }

    private RevealOutcome RunReveal()
    {
        if (!CanReveal())
        {
            return RevealOutcome.None;
        }

        var outcome = Reveal();
        if (!outcome.Applied)
        {
            return RevealOutcome.None;
        }

        RevealCommand.NotifyCanExecuteChanged();
        MoveUpCommand.NotifyCanExecuteChanged();
        RefreshSessionStatus();
        return outcome;
    }

    private void RunMoveUp()
    {
        if (!CanMoveUp())
        {
            return;
        }

        if (!MoveUp())
        {
            return;
        }

        RevealCommand.NotifyCanExecuteChanged();
        MoveUpCommand.NotifyCanExecuteChanged();
        RefreshSessionStatus();
    }

    private bool CanReveal()
    {
        if (FocusedRowIndex < 0 || FocusedRowIndex >= PreFreezeRows.Count)
        {
            return false;
        }

        return HasPendingReveal(PreFreezeRows[FocusedRowIndex].TeamId);
    }

    private bool CanMoveUp()
    {
        return FocusedRowIndex > 0 && FocusedRowIndex < PreFreezeRows.Count && IsInitialized && IsStarted;
    }

    private bool HasPendingReveal(string teamId)
    {
        return _pendingRevealsByTeamId.TryGetValue(teamId, out var queue) && queue.Count > 0;
    }

    private bool HasAwards(string teamId)
    {
        if (string.IsNullOrWhiteSpace(teamId) || _contestState is null)
        {
            return false;
        }

        foreach (var award in _contestState.Awards.Values)
        {
            if (award.TeamIds.Contains(teamId, StringComparer.Ordinal))
            {
                return true;
            }
        }

        return false;
    }

    private void RefreshRanks()
    {
        for (var i = 0; i < PreFreezeRows.Count; i++)
        {
            PreFreezeRows[i].SetRank(i + 1);
        }
    }

    private void ResortScoreboard(string? solvedTeamId)
    {
        if (FocusedRowIndex < 0 || FocusedRowIndex >= PreFreezeRows.Count)
        {
            return;
        }

        var preservedIndex = FocusedRowIndex;
        var oldIndexByTeamId = new Dictionary<string, int>(PreFreezeRows.Count, StringComparer.Ordinal);
        for (var i = 0; i < PreFreezeRows.Count; i++)
        {
            oldIndexByTeamId[PreFreezeRows[i].TeamId] = i;
        }

        var sortedRows = PreFreezeRows.OrderBy(row => row.TeamStatus).ToList();
        for (var targetIndex = 0; targetIndex < sortedRows.Count; targetIndex++)
        {
            var row = sortedRows[targetIndex];
            var currentIndex = PreFreezeRows.IndexOf(row);
            if (currentIndex >= 0 && currentIndex != targetIndex)
            {
                PreFreezeRows.Move(currentIndex, targetIndex);
            }
        }

        // Keep focus at the same index so presentation continues with the next team at this rank.
        if (PreFreezeRows.Count == 0)
        {
            FocusedRowIndex = -1;
        }
        else
        {
            var clampedIndex = Math.Clamp(preservedIndex, 0, PreFreezeRows.Count - 1);
            if (clampedIndex == FocusedRowIndex)
            {
                // After collection moves, ListBox can keep selected *item* while index value is unchanged.
                // Pulse SelectedIndex through -1 to force container :selected state refresh.
                _focusedRowIndex = -1;
                OnPropertyChanged(nameof(FocusedRowIndex));
                _focusedRowIndex = clampedIndex;
                OnPropertyChanged(nameof(FocusedRowIndex));
                RefreshSessionStatus();
            }
            else
            {
                FocusedRowIndex = clampedIndex;
            }
        }

        if (!string.IsNullOrWhiteSpace(solvedTeamId) &&
            oldIndexByTeamId.TryGetValue(solvedTeamId, out var oldIndex))
        {
            var newIndex = -1;
            for (var i = 0; i < PreFreezeRows.Count; i++)
            {
                if (string.Equals(PreFreezeRows[i].TeamId, solvedTeamId, StringComparison.Ordinal))
                {
                    newIndex = i;
                    break;
                }
            }

            if (newIndex >= 0 && newIndex < oldIndex)
            {
                _moveUpAnimationRequestCounter += 1;
                MoveUpAnimationRequest = new MoveUpAnimationRequest(
                    solvedTeamId,
                    oldIndex,
                    newIndex,
                    _moveUpAnimationRequestCounter);
            }
        }

        RefreshRanks();
        RevealCommand.NotifyCanExecuteChanged();
        MoveUpCommand.NotifyCanExecuteChanged();
        RefreshSessionStatus();
    }

    private static TeamStatus CloneTeamStatus(TeamStatus source)
    {
        var clone = new TeamStatus(source.TeamId, source.TeamName, source.TeamAffiliation, source.Sortorder)
        {
            TotalPoints = source.TotalPoints,
            TotalPenalty = source.TotalPenalty,
            LastAcTime = source.LastAcTime
        };

        foreach (var (problemId, stat) in source.ProblemStats)
        {
            clone.ProblemStats[problemId] = CloneProblemStat(stat);
        }

        return clone;
    }

    private static ProblemStat CloneProblemStat(ProblemStat source)
    {
        return new ProblemStat
        {
            Solved = source.Solved,
            AttemptedDuringFreeze = source.AttemptedDuringFreeze,
            Penalty = source.Penalty,
            SubmissionsBeforeSolved = source.SubmissionsBeforeSolved,
            FirstAcTime = source.FirstAcTime,
            LastSubmissionTime = source.LastSubmissionTime
        };
    }
}

public readonly record struct RevealOutcome(bool Applied, bool Solved, bool NeedResort, string? SolvedTeamId)
{
    public static RevealOutcome None => new(false, false, false, null);
}

public sealed record MoveUpAnimationRequest(string TeamId, int FromIndex, int ToIndex, long RequestId);

public enum PresentationRowState
{
    RowInProgress,
    RowInProgressAwaitResort,
    RowCompleteAwardShowing,
    RowCompleteReadyToAdvance
}

public sealed class PreFreezeScoreboardRowViewModel : ViewModelBase
{
    private static readonly Dictionary<string, Bitmap?> TeamLogoCache = new(StringComparer.OrdinalIgnoreCase);
    private readonly IReadOnlyList<ProblemDisplayInfo> _orderedProblems;
    private readonly TeamStatus _source;
    private int _rank;

    public PreFreezeScoreboardRowViewModel(
        TeamStatus source,
        int rank,
        IReadOnlyList<ProblemDisplayInfo> orderedProblems,
        string? cdpPath,
        string? logoExtension)
    {
        _source = source;
        _orderedProblems = orderedProblems;
        _rank = rank;
        var logoPath = BuildTeamLogoPath(cdpPath, source.TeamAffiliation, logoExtension);
        TeamLogoImage = LoadTeamLogoImageCached(logoPath);
        ProblemCells = BuildProblemCells(orderedProblems, source.ProblemStats);
    }

    public int Rank
    {
        get => _rank;
        private set => SetProperty(ref _rank, value);
    }

    public Bitmap? TeamLogoImage { get; }

    internal string TeamId => _source.TeamId;
    internal TeamStatus TeamStatus => _source;
    public string TeamName => _source.TeamName;
    public int TotalPoints => _source.TotalPoints;
    public long TotalPenalty => _source.TotalPenalty;
    public ObservableCollection<ProblemStatusCellViewModel> ProblemCells { get; }
    public int ProblemCellCount => ProblemCells.Count;

    public void SetRank(int rank)
    {
        Rank = rank;
    }

    public void RefreshFromSource()
    {
        OnPropertyChanged(nameof(TotalPoints));
        OnPropertyChanged(nameof(TotalPenalty));
        UpdateProblemCells();
    }

    private static string? BuildTeamLogoPath(string? cdpPath, string? teamAffiliation, string? logoExtension)
    {
        if (string.IsNullOrWhiteSpace(cdpPath) ||
            string.IsNullOrWhiteSpace(teamAffiliation) ||
            string.IsNullOrWhiteSpace(logoExtension))
        {
            Trace.WriteLine($"CDP Path {cdpPath} Team Affiliation {teamAffiliation} LOGO Extension {logoExtension}");
            return null;
        }

        var extension = logoExtension.Trim().TrimStart('.');
        if (extension.Length == 0)
        {
            return null;
        }

        var candidatePath = Path.Combine(cdpPath, "affiliations", $"{teamAffiliation}.{extension}");
        return File.Exists(candidatePath) ? candidatePath : null;
    }

    private static Bitmap? LoadTeamLogoImageCached(string? logoPath)
    {
        if (string.IsNullOrWhiteSpace(logoPath))
        {
            return null;
        }

        if (TeamLogoCache.TryGetValue(logoPath, out var cached))
        {
            return cached;
        }

        Bitmap? bitmap;
        try
        {
            bitmap = new Bitmap(logoPath);
        }
        catch
        {
            bitmap = null;
        }

        TeamLogoCache[logoPath] = bitmap;
        return bitmap;
    }

    private static ObservableCollection<ProblemStatusCellViewModel> BuildProblemCells(
        IReadOnlyList<ProblemDisplayInfo> orderedProblems,
        Dictionary<string, ProblemStat> problemStats)
    {
        var cells = new ObservableCollection<ProblemStatusCellViewModel>();

        foreach (var problem in orderedProblems)
        {
            cells.Add(CreateProblemCell(problem, problemStats));
        }

        return cells;
    }

    private void UpdateProblemCells()
    {
        for (var i = 0; i < _orderedProblems.Count; i++)
        {
            var problem = _orderedProblems[i];
            var (text, background) = BuildProblemCellValue(problem, _source.ProblemStats);

            if (i >= ProblemCells.Count)
            {
                ProblemCells.Add(new ProblemStatusCellViewModel(text, background));
                continue;
            }

            ProblemCells[i].Update(text, background);
        }

        while (ProblemCells.Count > _orderedProblems.Count)
        {
            ProblemCells.RemoveAt(ProblemCells.Count - 1);
        }

        OnPropertyChanged(nameof(ProblemCellCount));
    }

    private static ProblemStatusCellViewModel CreateProblemCell(
        ProblemDisplayInfo problem,
        Dictionary<string, ProblemStat> problemStats)
    {
        var (text, background) = BuildProblemCellValue(problem, problemStats);
        return new ProblemStatusCellViewModel(text, background);
    }

    private static (string Text, string Background) BuildProblemCellValue(
        ProblemDisplayInfo problem,
        Dictionary<string, ProblemStat> problemStats)
    {
        problemStats.TryGetValue(problem.Id, out var stat);
        var text = stat is { SubmissionsBeforeSolved: > 0 }
            ? $"{stat.SubmissionsBeforeSolved}-{stat.LastSubmissionTime}"
            : problem.Label;

        var background = stat switch
        {
            { AttemptedDuringFreeze: true } => "#2B7FFF",
            { Solved: true } => "#31C950",
            { SubmissionsBeforeSolved: > 0 } => "#FB2C36",
            _ => "#62748E"
        };

        return (text, background);
    }
}

public sealed class ProblemDisplayInfo
{
    public ProblemDisplayInfo(string id, string label)
    {
        Id = id;
        Label = label;
    }

    public string Id { get; }
    public string Label { get; }
}

public sealed class ProblemStatusCellViewModel : ViewModelBase
{
    private string _background;
    private string _text;

    public ProblemStatusCellViewModel(string text, string background)
    {
        _text = text;
        _background = background;
    }

    public string Text
    {
        get => _text;
        private set => SetProperty(ref _text, value);
    }

    public string Background
    {
        get => _background;
        private set => SetProperty(ref _background, value);
    }

    public void Update(string text, string background)
    {
        Text = text;
        Background = background;
    }
}
