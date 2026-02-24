using System;
using System.Collections.ObjectModel;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using Pyrite.Models;
using Pyrite.Services;

namespace Pyrite.ViewModels;

public sealed class LoadDataStageViewModel : ViewModelBase
{
    private string? _cdpPath;
    private bool _isParseSuccessful;
    private bool _isParsing;
    private PyriteConfig _loadedConfig = PyriteConfig.Default();
    private ContestState? _loadedContestState;
    private CancellationTokenSource? _parseCts;
    private double _parseProgress;
    private string _parseStatus = "Select a CDP folder to begin.";
    private string _validationStatus = string.Empty;

    public LoadDataStageViewModel()
    {
        ParseErrors = [];
        ParseWarnings = [];
    }

    public ObservableCollection<string> ParseErrors { get; }
    public ObservableCollection<string> ParseWarnings { get; }

    public string? CdpPath
    {
        get => _cdpPath;
        private set => SetProperty(ref _cdpPath, value);
    }

    public bool IsParsing
    {
        get => _isParsing;
        private set
        {
            if (SetProperty(ref _isParsing, value))
            {
                OnPropertyChanged(nameof(IsNotParsing));
            }
        }
    }

    public bool IsNotParsing => !IsParsing;

    public bool IsParseSuccessful
    {
        get => _isParseSuccessful;
        private set => SetProperty(ref _isParseSuccessful, value);
    }

    public double ParseProgress
    {
        get => _parseProgress;
        private set => SetProperty(ref _parseProgress, value);
    }

    public string ParseStatus
    {
        get => _parseStatus;
        private set => SetProperty(ref _parseStatus, value);
    }

    public string ValidationStatus
    {
        get => _validationStatus;
        private set => SetProperty(ref _validationStatus, value);
    }

    public bool HasValidationStatus => !string.IsNullOrWhiteSpace(ValidationStatus);
    public bool HasParseErrors => ParseErrors.Count > 0;
    public bool HasParseWarnings => ParseWarnings.Count > 0;

    public ContestState? LoadedContestState
    {
        get => _loadedContestState;
        private set => SetProperty(ref _loadedContestState, value);
    }

    public PyriteConfig LoadedConfig
    {
        get => _loadedConfig;
        private set => SetProperty(ref _loadedConfig, value);
    }

    public async Task SelectCdpFolderAsync(string folderPath)
    {
        CdpPath = folderPath;
        ResetLoadDataState();

        var validationErrors = ValidateCdpFolder(folderPath);
        if (validationErrors.Count > 0)
        {
            foreach (var error in validationErrors) ParseErrors.Add(error);

            ValidationStatus = "CDP folder validation failed.";
            OnPropertyChanged(nameof(HasValidationStatus));
            OnPropertyChanged(nameof(HasParseErrors));
            return;
        }

        ValidationStatus = "CDP folder validated.";
        OnPropertyChanged(nameof(HasValidationStatus));

        try
        {
            LoadedConfig = ConfigLoader.LoadIfExists(folderPath);
        }
        catch (Exception ex)
        {
            ParseErrors.Add(ex.Message);
            ValidationStatus = "CDP folder is valid but config.toml is invalid.";
            OnPropertyChanged(nameof(HasParseErrors));
            return;
        }

        await ParseEventFeedAsync(Path.Combine(folderPath, "event-feed.ndjson"));
    }

    private async Task ParseEventFeedAsync(string eventFeedPath)
    {
        _parseCts?.Cancel();
        _parseCts = new CancellationTokenSource();

        IsParsing = true;
        ParseStatus = "Parsing event-feed.ndjson...";
        ParseProgress = 0;

        var progress = new Progress<ParseProgressUpdate>(update =>
        {
            ParseProgress = update.TotalLines == 0 ? 0 : (double)update.LinesRead / update.TotalLines;
            ParseStatus = $"Parsing event-feed.ndjson... {update.LinesRead}/{update.TotalLines} lines";
        });

        try
        {
            var result = await EventFeedParser.ParseAsync(eventFeedPath, LoadedConfig, progress, _parseCts.Token);

            foreach (var warning in result.Warnings) ParseWarnings.Add(warning);

            foreach (var error in result.Errors) ParseErrors.Add(error);

            OnPropertyChanged(nameof(HasParseWarnings));
            OnPropertyChanged(nameof(HasParseErrors));

            if (result.ErrorCount > 0)
            {
                ParseStatus = $"Parsed {result.LinesRead} lines with {result.ErrorCount} error(s).";
                IsParseSuccessful = false;
                return;
            }

            LoadedContestState = result.ContestState;
            ParseProgress = 1;
            ParseStatus = result.Warnings.Count > 0
                ? $"Parsed successfully with {result.Warnings.Count} warning(s)."
                : "Parsed successfully with no warnings.";
            IsParseSuccessful = true;
        }
        catch (OperationCanceledException)
        {
            ParseStatus = "Parsing canceled.";
            IsParseSuccessful = false;
        }
        catch (Exception ex)
        {
            ParseErrors.Add(ex.Message);
            OnPropertyChanged(nameof(HasParseErrors));
            ParseStatus = "Parsing failed.";
            IsParseSuccessful = false;
        }
        finally
        {
            IsParsing = false;
        }
    }

    private static Collection<string> ValidateCdpFolder(string folderPath)
    {
        var errors = new Collection<string>();

        if (!Directory.Exists(folderPath))
        {
            errors.Add($"Selected folder does not exist: {folderPath}");
            return errors;
        }

        var eventFeedPath = Path.Combine(folderPath, "event-feed.ndjson");
        if (!File.Exists(eventFeedPath)) errors.Add("Missing required file: event-feed.ndjson");

        var teamsPath = Path.Combine(folderPath, "teams");
        if (!Directory.Exists(teamsPath)) errors.Add("Missing required folder: teams");

        var affiliationsPath = Path.Combine(folderPath, "affiliations");
        if (!Directory.Exists(affiliationsPath)) errors.Add("Missing required folder: affiliations");

        return errors;
    }

    private void ResetLoadDataState()
    {
        ParseErrors.Clear();
        ParseWarnings.Clear();
        ParseStatus = "Preparing parse...";
        ValidationStatus = string.Empty;
        ParseProgress = 0;
        IsParseSuccessful = false;
        LoadedContestState = null;

        OnPropertyChanged(nameof(HasValidationStatus));
        OnPropertyChanged(nameof(HasParseErrors));
        OnPropertyChanged(nameof(HasParseWarnings));
    }
}
