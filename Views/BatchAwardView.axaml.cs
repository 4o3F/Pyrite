using Avalonia;
using Avalonia.Controls;
using Avalonia.Media;
using Avalonia.Threading;
using Pyrite.ViewModels;
using System;
using System.Diagnostics;

namespace Pyrite.Views;

public partial class BatchAwardView : UserControl
{
    private const double NameScrollStartDelaySeconds = 2;
    private readonly TranslateTransform _nameScrollTransform = new();
    private readonly Stopwatch _nameScrollClock = new();
    private readonly DispatcherTimer _nameScrollTimer;
    private bool _isAttachedToVisualTree;
    private double _nameLoopHeight;

    public BatchAwardView()
    {
        InitializeComponent();
        NamesStrip.RenderTransform = _nameScrollTransform;
        _nameScrollTimer = new DispatcherTimer(DispatcherPriority.Render)
        {
            Interval = TimeSpan.FromMilliseconds(16)
        };
        _nameScrollTimer.Tick += OnNameScrollTick;
        DataContextChanged += (_, _) => ResetNameScrolling();
        // Check measured text after layout, including font fallback, wrapping and batch changes.
        LayoutUpdated += (_, _) => UpdateNameScrolling();
        AttachedToVisualTree += OnAttachedToVisualTree;
        DetachedFromVisualTree += OnDetachedFromVisualTree;
    }

    private void OnAttachedToVisualTree(object? sender, VisualTreeAttachmentEventArgs e)
    {
        _isAttachedToVisualTree = true;
        UpdateNameScrolling();
    }

    private void OnDetachedFromVisualTree(object? sender, VisualTreeAttachmentEventArgs e)
    {
        _isAttachedToVisualTree = false;
        ResetNameScrolling();
    }

    private void UpdateNameScrolling()
    {
        if (!_isAttachedToVisualTree || !IsEffectivelyVisible || DataContext is not BatchAwardViewModel)
        {
            ResetNameScrolling();
            return;
        }

        var contentHeight = BatchNames.Bounds.Height;
        var viewportHeight = NamesViewport.Bounds.Height;
        if (contentHeight <= 0 || viewportHeight <= 0)
        {
            ResetNameScrolling();
            return;
        }

        if (contentHeight <= viewportHeight + 0.5)
        {
            ResetNameScrolling();
            _nameScrollTransform.Y = Math.Max(0, (viewportHeight - contentHeight) / 2);
            return;
        }

        RepeatedBatchNames.IsVisible = true;
        if (!_nameScrollTimer.IsEnabled || Math.Abs(_nameLoopHeight - contentHeight) > 0.5)
        {
            _nameLoopHeight = contentHeight;
            _nameScrollTransform.Y = 0;
            _nameScrollClock.Restart();
            _nameScrollTimer.Start();
        }
    }

    private void OnNameScrollTick(object? sender, EventArgs e)
    {
        if (!_isAttachedToVisualTree || !IsEffectivelyVisible ||
            DataContext is not BatchAwardViewModel batchAward || _nameLoopHeight <= 0)
        {
            ResetNameScrolling();
            return;
        }

        var elapsed = Math.Max(0, _nameScrollClock.Elapsed.TotalSeconds - NameScrollStartDelaySeconds);
        // One measured copy includes wrapped names, dividers and spacing, but not the repeated copy.
        var scrollSpeed = _nameLoopHeight / batchAward.NameScrollCycleSeconds;
        var elapsedInCycle = elapsed % batchAward.NameScrollCycleSeconds;
        _nameScrollTransform.Y = -elapsedInCycle * scrollSpeed;
    }

    private void ResetNameScrolling()
    {
        _nameScrollTimer.Stop();
        _nameScrollClock.Reset();
        _nameLoopHeight = 0;
        _nameScrollTransform.Y = 0;
        RepeatedBatchNames.IsVisible = false;
    }
}
