using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Diagnostics;
using System.IO;
using System.Linq;
using Avalonia.Media.Imaging;
using CommunityToolkit.Mvvm.Input;
using Pyrite.Models;

namespace Pyrite.ViewModels;

public sealed class PresentationStageViewModel : ViewModelBase
{
    private int _advanceCount;
    private ContestState? _contestState;
    private string? _dataPath;
    private bool _isInitialized;
    private bool _isStarted;
    private PyriteConfig _loadedConfig = PyriteConfig.Default();
    private double _viewportHeight;
    private double _viewportWidth;

    public PresentationStageViewModel()
    {
        AdvanceCommand = new RelayCommand(Advance, CanAdvance);
        ExitCommand = new RelayCommand(RequestExit);
        RefreshSessionStatus();
    }

    public event Action? ExitRequested;

    public RelayCommand AdvanceCommand { get; }
    public RelayCommand ExitCommand { get; }
    public ObservableCollection<PreFreezeScoreboardRowViewModel> PreFreezeRows { get; } = [];

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

    public int AdvanceCount
    {
        get => _advanceCount;
        private set
        {
            if (SetProperty(ref _advanceCount, value))
            {
                OnPropertyChanged(nameof(SessionStatus));
            }
        }
    }

    public string SessionStatus =>
        $"Initialized={IsInitialized}, Started={IsStarted}, AdvanceCount={AdvanceCount}, " +
        $"Viewport={_viewportWidth:F0}x{_viewportHeight:F0}";

    public void Initialize(ContestState contestState, PyriteConfig config, string? dataPath)
    {
        ArgumentNullException.ThrowIfNull(contestState);
        ArgumentNullException.ThrowIfNull(config);

        _contestState = contestState;
        _loadedConfig = config;
        _dataPath = dataPath;
        AdvanceCount = 0;
        LoadPreFreezeRows(contestState);
        IsInitialized = true;
        AdvanceCommand.NotifyCanExecuteChanged();
        RefreshSessionStatus();
    }

    public void Start()
    {
        if (!IsInitialized)
        {
            return;
        }

        IsStarted = true;
        AdvanceCommand.NotifyCanExecuteChanged();
        RefreshSessionStatus();
    }

    public void Stop()
    {
        IsStarted = false;
        AdvanceCommand.NotifyCanExecuteChanged();
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

    private void Advance()
    {
        if (!CanAdvance())
        {
            return;
        }

        AdvanceCount += 1;
        RefreshSessionStatus();
    }

    private bool CanAdvance()
    {
        return IsInitialized && IsStarted;
    }

    private void RequestExit()
    {
        ExitRequested?.Invoke();
    }

    private void RefreshSessionStatus()
    {
        OnPropertyChanged(nameof(SessionStatus));
    }

    private void LoadPreFreezeRows(ContestState contestState)
    {
        PreFreezeRows.Clear();
        var orderedProblems = contestState.Problems.Values
            .OrderBy(problem => problem.Ordinal)
            .ThenBy(problem => problem.Label, StringComparer.Ordinal)
            .Select(problem => new ProblemDisplayInfo(
                problem.Id,
                string.IsNullOrWhiteSpace(problem.Label) ? problem.ShortName : problem.Label))
            .ToList();

        for (var i = 0; i < contestState.LeaderboardPreFreeze.Count; i++)
        {
            PreFreezeRows.Add(new PreFreezeScoreboardRowViewModel(
                contestState.LeaderboardPreFreeze[i],
                i + 1,
                orderedProblems,
                _dataPath,
                _loadedConfig.Presentation.LogoExtension));
        }
    }
}

public sealed class PreFreezeScoreboardRowViewModel
{
    public PreFreezeScoreboardRowViewModel(
        TeamStatus source,
        int rank,
        IReadOnlyList<ProblemDisplayInfo> orderedProblems,
        string? cdpPath,
        string? logoExtension)
    {
        Source = source;
        Rank = rank;
        TeamLogoPath = BuildTeamLogoPath(cdpPath, source.TeamAffiliation, logoExtension);
        TeamLogoImage = LoadTeamLogoImage(TeamLogoPath);
        ProblemCells = BuildProblemCells(orderedProblems, source.ProblemStats);
    }

    public TeamStatus Source { get; }
    public int Rank { get; }
    public string? TeamLogoPath { get; }
    public Bitmap? TeamLogoImage { get; }

    public string TeamId => Source.TeamId;
    public string TeamName => Source.TeamName;
    public string TeamAffiliation => Source.TeamAffiliation;
    public int Sortorder => Source.Sortorder;
    public int TotalPoints => Source.TotalPoints;
    public long TotalPenalty => Source.TotalPenalty;
    public DateTimeOffset? LastAcTime => Source.LastAcTime;
    public IReadOnlyList<ProblemStatusCellViewModel> ProblemCells { get; }
    public int ProblemCellCount => ProblemCells.Count;

    public Dictionary<string, ProblemStat> ProblemStats
    {
        get => Source.ProblemStats;
        set => Source.ProblemStats = value ?? [];
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
        Trace.WriteLine( candidatePath );
        return File.Exists(candidatePath) ? candidatePath : null;
    }

    private static Bitmap? LoadTeamLogoImage(string? logoPath)
    {
        if (string.IsNullOrWhiteSpace(logoPath))
        {
            return null;
        }

        try
        {
            return new Bitmap(logoPath);
        }
        catch
        {
            return null;
        }
    }

    private static IReadOnlyList<ProblemStatusCellViewModel> BuildProblemCells(
        IReadOnlyList<ProblemDisplayInfo> orderedProblems,
        Dictionary<string, ProblemStat> problemStats)
    {
        var cells = new List<ProblemStatusCellViewModel>(orderedProblems.Count);

        foreach (var problem in orderedProblems)
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

            cells.Add(new ProblemStatusCellViewModel(text, background));
        }

        return cells;
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

public sealed class ProblemStatusCellViewModel
{
    public ProblemStatusCellViewModel(string text, string background)
    {
        Text = text;
        Background = background;
    }

    public string Text { get; }
    public string Background { get; }
}
