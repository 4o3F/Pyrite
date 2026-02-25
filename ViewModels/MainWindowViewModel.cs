using CommunityToolkit.Mvvm.Input;
using System;
using System.ComponentModel;
using System.Diagnostics;

namespace Pyrite.ViewModels;

public enum AppStage
{
    LoadData = 0,
    SetMedal = 1
}

public class MainWindowViewModel : ViewModelBase
{
    private AppStage _currentStage = AppStage.LoadData;
    private bool _isPresentationActive;

    public MainWindowViewModel()
    {
        LoadDataStage = new LoadDataStageViewModel();
        SetMedalStage = new SetMedalStageViewModel();
        PresentationStage = new PresentationStageViewModel();
        PreviousStageCommand = new RelayCommand(MovePrevious, () => CanMovePrevious);
        NextStageCommand = new RelayCommand(MoveNext, () => CanMoveNext);
        LaunchPresentationCommand = new RelayCommand(LaunchPresentation, () => CanLaunchPresentation);
        PrimaryActionCommand = new RelayCommand(ExecutePrimaryAction, () => CanExecutePrimaryAction);
        PresentationStage.ExitRequested += ExitPresentation;
        LoadDataStage.PropertyChanged += OnLoadDataStagePropertyChanged;
    }

    public RelayCommand PreviousStageCommand { get; }
    public RelayCommand NextStageCommand { get; }
    public RelayCommand LaunchPresentationCommand { get; }
    public RelayCommand PrimaryActionCommand { get; }
    public LoadDataStageViewModel LoadDataStage { get; }
    public SetMedalStageViewModel SetMedalStage { get; }
    public PresentationStageViewModel PresentationStage { get; }

    public AppStage CurrentStage
    {
        get => _currentStage;
        private set
        {
            if (SetProperty(ref _currentStage, value))
            {
                OnPropertyChanged(nameof(CurrentStageKey));
                OnPropertyChanged(nameof(StageTitle));
                OnPropertyChanged(nameof(StageDescription));
                OnPropertyChanged(nameof(IsLoadDataStage));
                OnPropertyChanged(nameof(IsSetMedalStage));
                NotifyWorkflowStateChanged();
            }
        }
    }

    public bool IsPresentationActive
    {
        get => _isPresentationActive;
        private set
        {
            if (SetProperty(ref _isPresentationActive, value))
            {
                OnPropertyChanged(nameof(IsWorkflowVisible));
                NotifyWorkflowStateChanged();
                if (value)
                    PresentationStage.Start();
                else
                    PresentationStage.Stop();
            }
        }
    }

    public bool IsWorkflowVisible => !IsPresentationActive;

    public string CurrentStageKey => CurrentStage switch
    {
        AppStage.LoadData => "load_data",
        AppStage.SetMedal => "set_medal",
        _ => "unknown"
    };

    public string StageTitle => CurrentStage switch
    {
        AppStage.LoadData => "Load Data",
        AppStage.SetMedal => "Set Medal",
        _ => "Unknown Stage"
    };

    public string StageDescription => CurrentStage switch
    {
        AppStage.LoadData => "Validate CDP input, parse event-feed.ndjson, and build standings.",
        AppStage.SetMedal => "Review ranking and assign medal citations, then launch presentation.",
        _ => string.Empty
    };

    public bool IsLoadDataStage => CurrentStage == AppStage.LoadData;
    public bool IsSetMedalStage => CurrentStage == AppStage.SetMedal;

    public bool CanMovePrevious => !IsPresentationActive && CurrentStage > AppStage.LoadData;
    public bool CanMoveNext => !IsPresentationActive && CurrentStage < AppStage.SetMedal && CanAdvanceCurrentStage;
    public bool CanLaunchPresentation => !IsPresentationActive && CurrentStage == AppStage.SetMedal;
    public bool CanExecutePrimaryAction => CanMoveNext || CanLaunchPresentation;
    public string PrimaryActionText => CurrentStage == AppStage.SetMedal ? "Launch" : "Next";

    private bool CanAdvanceCurrentStage => CurrentStage switch
    {
        AppStage.LoadData => LoadDataStage.IsParseSuccessful && !LoadDataStage.IsParsing,
        _ => true
    };

    private void NotifyWorkflowStateChanged()
    {
        OnPropertyChanged(nameof(CanMovePrevious));
        OnPropertyChanged(nameof(CanMoveNext));
        OnPropertyChanged(nameof(CanLaunchPresentation));
        OnPropertyChanged(nameof(CanExecutePrimaryAction));
        OnPropertyChanged(nameof(PrimaryActionText));
        PreviousStageCommand.NotifyCanExecuteChanged();
        NextStageCommand.NotifyCanExecuteChanged();
        LaunchPresentationCommand.NotifyCanExecuteChanged();
        PrimaryActionCommand.NotifyCanExecuteChanged();
    }

    private void OnLoadDataStagePropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        if (e.PropertyName == nameof(LoadDataStageViewModel.LoadedContestState))
        {
            SetMedalStage.SetContestState(LoadDataStage.LoadedContestState);
        }

        if (e.PropertyName == nameof(LoadDataStageViewModel.IsParsing) ||
            e.PropertyName == nameof(LoadDataStageViewModel.IsParseSuccessful))
        {
            NotifyWorkflowStateChanged();
        }
    }

    private void MovePrevious()
    {
        if (!CanMovePrevious) return;

        CurrentStage -= 1;
    }

    private void MoveNext()
    {
        if (!CanMoveNext) return;

        CurrentStage += 1;
    }

    private void LaunchPresentation()
    {
        if (!CanLaunchPresentation) return;

        if (!SetMedalStage.TryPreparePresentation(out _)) return;
        var contestState = LoadDataStage.LoadedContestState;
        if (contestState is null) return;
        Trace.WriteLine(
            $"[MainWindowVM] LaunchPresentation: ts={DateTime.Now:HH:mm:ss.fff}, " +
            $"teams={contestState.Teams.Count}, preFreeze={contestState.LeaderboardPreFreeze.Count}, " +
            $"finalized={contestState.LeaderboardFinalized.Count}, problems={contestState.Problems.Count}");

        PresentationStage.Initialize(contestState, LoadDataStage.LoadedConfig, LoadDataStage.CdpPath);
        IsPresentationActive = true;
    }

    private void ExitPresentation()
    {
        if (!IsPresentationActive)
        {
            return;
        }

        IsPresentationActive = false;
    }

    private void ExecutePrimaryAction()
    {
        if (CanMoveNext)
        {
            MoveNext();
            return;
        }

        if (CanLaunchPresentation) LaunchPresentation();
    }
}
