using System;
using System.Linq;
using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Platform.Storage;
using Pyrite.ViewModels;

namespace Pyrite.Views;

public partial class LoadDataStageView : UserControl
{
    public LoadDataStageView()
    {
        InitializeComponent();
    }

    private async void OnSelectFolderClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not LoadDataStageViewModel viewModel) return;

        var topLevel = TopLevel.GetTopLevel(this);
        if (topLevel?.StorageProvider is null) return;

        var folders = await topLevel.StorageProvider.OpenFolderPickerAsync(new FolderPickerOpenOptions
        {
            Title = "Select CDP Folder",
            AllowMultiple = false
        });

        var folder = folders.FirstOrDefault();
        if (folder is null) return;

        var localPath = folder.TryGetLocalPath();
        if (string.IsNullOrWhiteSpace(localPath)) return;

        try
        {
            await viewModel.SelectCdpFolderAsync(localPath);
        }
        catch (Exception)
        {
            // Errors are surfaced through view model status collections.
        }
    }
}
