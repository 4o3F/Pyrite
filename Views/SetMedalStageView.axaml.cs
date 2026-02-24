using System;
using System.Linq;
using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Platform.Storage;
using Pyrite.ViewModels;

namespace Pyrite.Views;

public partial class SetMedalStageView : UserControl
{
    public SetMedalStageView()
    {
        InitializeComponent();
    }

    private async void OnSaveMedalsClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not SetMedalStageViewModel viewModel) return;

        var topLevel = TopLevel.GetTopLevel(this);
        if (topLevel?.StorageProvider is null) return;

        var file = await topLevel.StorageProvider.SaveFilePickerAsync(new FilePickerSaveOptions
        {
            Title = "Save Medals",
            SuggestedFileName = "medals",
            DefaultExtension = "json",
            FileTypeChoices =
            [
                new FilePickerFileType("JSON")
                {
                    Patterns = ["*.json"]
                }
            ]
        });

        var localPath = file?.TryGetLocalPath();
        if (string.IsNullOrWhiteSpace(localPath)) return;

        try
        {
            viewModel.SaveMedalsToFile(localPath);
        }
        catch (Exception ex)
        {
            viewModel.SetStatusMessage($"Failed to save medals file {localPath}: {ex.Message}");
        }
    }

    private async void OnLoadMedalsClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not SetMedalStageViewModel viewModel) return;

        var topLevel = TopLevel.GetTopLevel(this);
        if (topLevel?.StorageProvider is null) return;

        var files = await topLevel.StorageProvider.OpenFilePickerAsync(new FilePickerOpenOptions
        {
            Title = "Load Medals",
            AllowMultiple = false,
            FileTypeFilter =
            [
                new FilePickerFileType("JSON")
                {
                    Patterns = ["*.json"]
                }
            ]
        });

        var localPath = files.FirstOrDefault()?.TryGetLocalPath();
        if (string.IsNullOrWhiteSpace(localPath)) return;

        try
        {
            viewModel.LoadMedalsFromFile(localPath);
        }
        catch (Exception ex)
        {
            viewModel.SetStatusMessage($"Failed to load medals file {localPath}: {ex.Message}");
        }
    }

    private void OnPresentClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not SetMedalStageViewModel) return;

        var topLevel = TopLevel.GetTopLevel(this);
        if (topLevel?.DataContext is not MainWindowViewModel mainWindowViewModel) return;

        mainWindowViewModel.LaunchPresentationCommand.Execute(null);
    }

    private void OnDeleteMedalClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not SetMedalStageViewModel viewModel) return;

        if (sender is not Button { Tag: string medalId } || string.IsNullOrWhiteSpace(medalId)) return;

        viewModel.DeleteMedalCommand.Execute(medalId);
    }
}