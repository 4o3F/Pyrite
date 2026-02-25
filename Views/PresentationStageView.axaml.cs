using Avalonia.Controls;
using Avalonia.Input;
using Pyrite.ViewModels;

namespace Pyrite.Views;

public partial class PresentationStageView : UserControl
{
    public PresentationStageView()
    {
        InitializeComponent();
        KeyDown += OnKeyDown;
        AttachedToVisualTree += (_, _) => Focus();
        PointerPressed += (_, _) => Focus();
    }

    private void OnKeyDown(object? sender, KeyEventArgs e)
    {
        if (e.Key != Key.Space)
        {
            return;
        }

        if (DataContext is not PresentationStageViewModel vm)
        {
            return;
        }

        vm.HandleSpacePressed();
        e.Handled = true;
    }
}
