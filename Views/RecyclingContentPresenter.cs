using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Presenters;
using Avalonia.Controls.Templates;
using System;

namespace Pyrite.Views;

/// <summary>
/// Retains template-generated content while a virtualized row is rebound, without
/// changing ContentPresenter layout, rendering, or theme-state selectors.
/// </summary>
public sealed class RecyclingContentPresenter : ContentPresenter
{
    public static readonly StyledProperty<object?> ItemProperty =
        AvaloniaProperty.Register<RecyclingContentPresenter, object?>(nameof(Item));
    public static readonly StyledProperty<IDataTemplate?> ItemTemplateProperty =
        AvaloniaProperty.Register<RecyclingContentPresenter, IDataTemplate?>(nameof(ItemTemplate));

    private IDataTemplate? _builtTemplate;
    private Control? _builtContent;

    // Fluent styles target ContentPresenter#PART_ContentPresenter, including its
    // selected/focused/pressed colors. Preserve that style key as well as the name.
    protected override Type StyleKeyOverride => typeof(ContentPresenter);

    public object? Item
    {
        get => GetValue(ItemProperty);
        set => SetValue(ItemProperty, value);
    }

    public IDataTemplate? ItemTemplate
    {
        get => GetValue(ItemTemplateProperty);
        set => SetValue(ItemTemplateProperty, value);
    }

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property != ItemProperty && change.Property != ItemTemplateProperty)
            return;

        // Clearing a recycled container temporarily clears both properties. Retain
        // the controls through that gap, but release their previous data context.
        if (ItemTemplate is { } template && !ReferenceEquals(template, _builtTemplate))
        {
            _builtContent = template.Build(Item);
            _builtTemplate = template;
            Content = _builtContent;
        }
        if (_builtContent is not null)
            _builtContent.DataContext = Item;
    }
}
