using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Presenters;
using Avalonia.Controls.Primitives;
using Avalonia.Controls.Templates;
using Avalonia.Utilities;
using Pyrite.ViewModels;
using System;
using System.Collections.Generic;
using System.Collections.Specialized;

namespace Pyrite.Views;

/// <summary>
/// Rebinds existing problem-cell visuals when a virtualized scoreboard row is reused.
/// Retains the original per-cell ContentPresenter layout and pixel rounding.
/// </summary>
public sealed class ProblemCellsView : UniformGrid, IWeakEventSubscriber<NotifyCollectionChangedEventArgs>
{
    public static readonly StyledProperty<IReadOnlyList<ProblemStatusCellViewModel>?> CellsProperty =
        AvaloniaProperty.Register<ProblemCellsView, IReadOnlyList<ProblemStatusCellViewModel>?>(nameof(Cells));
    public static readonly StyledProperty<IDataTemplate?> CellTemplateProperty =
        AvaloniaProperty.Register<ProblemCellsView, IDataTemplate?>(nameof(CellTemplate));

    static ProblemCellsView()
    {
        // ItemsControl clips its cells; preserve that behavior at fractional widths.
        ClipToBoundsProperty.OverrideDefaultValue<ProblemCellsView>(true);
    }

    public IReadOnlyList<ProblemStatusCellViewModel>? Cells
    {
        get => GetValue(CellsProperty);
        set => SetValue(CellsProperty, value);
    }

    public IDataTemplate? CellTemplate
    {
        get => GetValue(CellTemplateProperty);
        set => SetValue(CellTemplateProperty, value);
    }

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property == CellsProperty)
        {
            if (change.OldValue is INotifyCollectionChanged oldCells)
                WeakEvents.CollectionChanged.Unsubscribe(oldCells, this);
            if (change.NewValue is INotifyCollectionChanged newCells)
                WeakEvents.CollectionChanged.Subscribe(newCells, this);
            SyncCells();
        }
        else if (change.Property == CellTemplateProperty)
        {
            Children.Clear();
            SyncCells();
        }
    }

    // A completed fly-away ghost must not be kept alive by its row's collection.
    void IWeakEventSubscriber<NotifyCollectionChangedEventArgs>.OnEvent(
        object? sender, WeakEvent ev, NotifyCollectionChangedEventArgs e) => SyncCells();

    private void SyncCells()
    {
        var cells = Cells;
        if (cells is null)
        {
            // A recycled row may temporarily lose its DataContext. Release the old
            // models without destroying the controls needed by the next team.
            foreach (ContentPresenter presenter in Children)
                ((Control)presenter.Content!).DataContext = null;
            return;
        }

        if (CellTemplate is not { } template)
            return;

        for (var i = 0; i < cells.Count; i++)
        {
            if (i < Children.Count)
                ((Control)((ContentPresenter)Children[i]).Content!).DataContext = cells[i];
            else
            {
                var child = template.Build(cells[i]) ??
                    throw new InvalidOperationException("A problem-cell template must produce a control.");
                child.DataContext = cells[i];
                // Keep the same wrapper as the original ItemsControl: removing it
                // subtly changes margin/layout rounding at fractional cell widths.
                Children.Add(new ContentPresenter { Content = child });
            }
        }
        while (Children.Count > cells.Count)
            Children.RemoveAt(Children.Count - 1);
    }
}
