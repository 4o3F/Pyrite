using System;
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
}
