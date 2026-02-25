using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Presenters;
using Avalonia.Input;
using Avalonia.Media;
using Avalonia.Media.Imaging;
using Avalonia.Threading;
using Avalonia.VisualTree;
using Pyrite.ViewModels;
using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.Linq;

namespace Pyrite.Views;

public partial class PresentationStageView : UserControl
{
    private const double FocusAnchorRatio = 2.0 / 3.0;
    private const double ScrollEpsilon = 0.5;
    private static readonly TimeSpan FocusScrollDuration = TimeSpan.FromMilliseconds(180);
    private static readonly TimeSpan AwardOverlayFadeDuration = TimeSpan.FromMilliseconds(260);
    private const double DefaultRowFlyAnimationSeconds = 0.6;
    private const double DefaultScrollAnimationSeconds = 0.4;

    private INotifyPropertyChanged? _subscribedViewModel;
    private DispatcherTimer? _scrollAnimationTimer;
    private DispatcherTimer? _moveUpAnimationTimer;
    private DispatcherTimer? _awardOverlayFadeTimer;
    private ScrollViewer? _animatedScrollViewer;
    private long _animationStartTimestamp;
    private long _awardOverlayFadeStartTimestamp;
    private double _animationStartOffsetY;
    private double _animationTargetOffsetY;
    private double _awardOverlayFadeStartOpacity;
    private double _awardOverlayFadeTargetOpacity;
    private bool _anchorRequestQueued;
    private bool _deferredRetryQueued;
    private long _lastHandledMoveUpRequestId;
    private readonly List<ActiveMoveUpAnimation> _activeMoveUpAnimations = [];
    private readonly List<ActiveDownShiftAnimation> _activeDownShiftAnimations = [];

    public PresentationStageView()
    {
        InitializeComponent();
        KeyDown += OnKeyDown;
        DataContextChanged += OnDataContextChanged;
        AttachedToVisualTree += OnAttachedToVisualTree;
        DetachedFromVisualTree += OnDetachedFromVisualTree;
        PointerPressed += (_, _) => Focus();
        ScoreboardList.SizeChanged += (_, _) => RequestFocusedRowAnchor();
    }

    private void OnKeyDown(object? sender, KeyEventArgs e)
    {
        if (e.Key == Key.F12)
        {
            ToggleFullscreen();
            e.Handled = true;
            return;
        }

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

    private void ToggleFullscreen()
    {
        if (TopLevel.GetTopLevel(this) is not Window window)
        {
            return;
        }

        window.WindowState = window.WindowState == WindowState.FullScreen
            ? WindowState.Normal
            : WindowState.FullScreen;
    }

    private void OnAttachedToVisualTree(object? sender, VisualTreeAttachmentEventArgs e)
    {
        Focus();
        RequestFocusedRowAnchor();
    }

    private void OnDetachedFromVisualTree(object? sender, VisualTreeAttachmentEventArgs e)
    {
        StopScrollAnimation();
        StopAllMoveUpAnimations();
        StopAwardOverlayFadeAnimation();
    }

    private void OnDataContextChanged(object? sender, EventArgs e)
    {
        StopAllMoveUpAnimations();
        _lastHandledMoveUpRequestId = 0;

        if (_subscribedViewModel is not null)
        {
            _subscribedViewModel.PropertyChanged -= OnViewModelPropertyChanged;
            _subscribedViewModel = null;
        }

        _subscribedViewModel = DataContext as INotifyPropertyChanged;
        if (_subscribedViewModel is not null)
        {
            _subscribedViewModel.PropertyChanged += OnViewModelPropertyChanged;
        }

        if (DataContext is PresentationStageViewModel vm)
        {
            SetAwardOverlayVisibilityImmediate(vm.IsAwardOverlayVisible);
        }
        else
        {
            SetAwardOverlayVisibilityImmediate(false);
        }

        RequestFocusedRowAnchor();
    }

    private void OnViewModelPropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        if (e.PropertyName == nameof(PresentationStageViewModel.IsAwardOverlayVisible))
        {
            var isVisible = (DataContext as PresentationStageViewModel)?.IsAwardOverlayVisible ?? false;
            AnimateAwardOverlayVisibility(isVisible);
            return;
        }

        if (e.PropertyName == nameof(PresentationStageViewModel.MoveUpAnimationRequest))
        {
            HandleMoveUpAnimationRequest();
            return;
        }

        if (!string.IsNullOrEmpty(e.PropertyName) &&
            e.PropertyName != nameof(PresentationStageViewModel.FocusedRowIndex))
        {
            return;
        }

        var focusedRowIndex = (DataContext as PresentationStageViewModel)?.FocusedRowIndex ?? -1;
        if (focusedRowIndex < 0)
        {
            return;
        }

        RequestFocusedRowAnchor();
    }

    private void HandleMoveUpAnimationRequest()
    {
        Trace.WriteLine("[MoveUpAnim] Property changed for MoveUpAnimationRequest.");
        if (DataContext is not PresentationStageViewModel vm ||
            vm.MoveUpAnimationRequest is null)
        {
            Trace.WriteLine("[MoveUpAnim] Skip request: VM/request is null.");
            return;
        }

        var request = vm.MoveUpAnimationRequest;
        if (request.RequestId <= _lastHandledMoveUpRequestId)
        {
            Trace.WriteLine($"[MoveUpAnim] Skip request: stale id={request.RequestId}, last={_lastHandledMoveUpRequestId}.");
            return;
        }

        _lastHandledMoveUpRequestId = request.RequestId;
        Trace.WriteLine($"[MoveUpAnim] Accept request: id={request.RequestId}, team={request.TeamId}, from={request.FromIndex}, to={request.ToIndex}.");
        Dispatcher.UIThread.Post(
            () => TryStartMoveUpAnimation(request, allowDeferredRetry: true),
            DispatcherPriority.Render);
    }

    private void RequestFocusedRowAnchor()
    {
        if (_anchorRequestQueued)
        {
            return;
        }

        _anchorRequestQueued = true;
        Dispatcher.UIThread.Post(() =>
        {
            _anchorRequestQueued = false;
            TryAnchorFocusedRow(allowDeferredRetry: true);
        }, DispatcherPriority.Render);
    }

    private void TryAnchorFocusedRow(bool allowDeferredRetry)
    {
        if (DataContext is not PresentationStageViewModel vm)
        {
            return;
        }

        var focusedIndex = vm.FocusedRowIndex;
        if (focusedIndex < 0 || focusedIndex >= ScoreboardList.ItemCount)
        {
            return;
        }

        var scrollViewer = ScoreboardList.GetVisualDescendants().OfType<ScrollViewer>().FirstOrDefault();
        if (scrollViewer is null)
        {
            return;
        }

        var viewportHeight = scrollViewer.Viewport.Height;
        var extentHeight = scrollViewer.Extent.Height;
        if (viewportHeight <= 0 || extentHeight <= 0)
        {
            return;
        }

        var itemContainer = ScoreboardList.ContainerFromIndex(focusedIndex) as Control;
        if (itemContainer is null)
        {
            if (allowDeferredRetry && !_deferredRetryQueued)
            {
                _deferredRetryQueued = true;
                Dispatcher.UIThread.Post(() =>
                {
                    _deferredRetryQueued = false;
                    TryAnchorFocusedRow(allowDeferredRetry: false);
                }, DispatcherPriority.Loaded);
            }

            return;
        }

        var itemTop = itemContainer.TranslatePoint(new Point(0, 0), scrollViewer);
        if (!itemTop.HasValue)
        {
            return;
        }

        var targetYInViewport = viewportHeight * FocusAnchorRatio;
        var currentOffsetY = scrollViewer.Offset.Y;
        var delta = itemTop.Value.Y - targetYInViewport;
        var maxOffsetY = Math.Max(0, extentHeight - viewportHeight);
        var targetOffsetY = Math.Clamp(currentOffsetY + delta, 0, maxOffsetY);

        if (Math.Abs(targetOffsetY - currentOffsetY) <= ScrollEpsilon)
        {
            return;
        }

        StartScrollAnimation(scrollViewer, targetOffsetY);
    }

    private void StartScrollAnimation(ScrollViewer scrollViewer, double targetOffsetY)
    {
        var currentOffsetY = scrollViewer.Offset.Y;
        if (Math.Abs(targetOffsetY - currentOffsetY) <= ScrollEpsilon)
        {
            return;
        }

        _animatedScrollViewer = scrollViewer;
        _animationStartOffsetY = currentOffsetY;
        _animationTargetOffsetY = targetOffsetY;
        _animationStartTimestamp = Stopwatch.GetTimestamp();

        if (_scrollAnimationTimer is null)
        {
            _scrollAnimationTimer = new DispatcherTimer(
                TimeSpan.FromMilliseconds(16),
                DispatcherPriority.Render,
                OnScrollAnimationTick);
        }

        _scrollAnimationTimer.Stop();
        _scrollAnimationTimer.Start();
    }

    private void OnScrollAnimationTick(object? sender, EventArgs e)
    {
        if (_animatedScrollViewer is null || _scrollAnimationTimer is null)
        {
            StopScrollAnimation();
            return;
        }

        var elapsedSeconds = (Stopwatch.GetTimestamp() - _animationStartTimestamp) / (double)Stopwatch.Frequency;
        var progress = Math.Clamp(elapsedSeconds / FocusScrollDuration.TotalSeconds, 0, 1);
        var eased = EaseOutCubic(progress);
        var nextOffsetY = _animationStartOffsetY +
                          ((_animationTargetOffsetY - _animationStartOffsetY) * eased);
        _animatedScrollViewer.Offset = new Vector(_animatedScrollViewer.Offset.X, nextOffsetY);

        if (progress >= 1)
        {
            _animatedScrollViewer.Offset = new Vector(_animatedScrollViewer.Offset.X, _animationTargetOffsetY);
            StopScrollAnimation();
        }
    }

    private void StopScrollAnimation()
    {
        if (_scrollAnimationTimer is not null)
        {
            _scrollAnimationTimer.Stop();
        }

        _animatedScrollViewer = null;
    }

    private static double EaseOutCubic(double t)
    {
        var oneMinusT = 1 - t;
        return 1 - (oneMinusT * oneMinusT * oneMinusT);
    }

    private static double EaseInOutCubic(double t)
    {
        return t < 0.5
            ? 4 * t * t * t
            : 1 - Math.Pow(-2 * t + 2, 3) / 2;
    }

    private void TryStartMoveUpAnimation(MoveUpAnimationRequest request, bool allowDeferredRetry)
    {
        Trace.WriteLine($"[MoveUpAnim] TryStart: id={request.RequestId}, team={request.TeamId}, from={request.FromIndex}, to={request.ToIndex}, retry={allowDeferredRetry}.");
        if (DataContext is not PresentationStageViewModel vm)
        {
            Trace.WriteLine("[MoveUpAnim] Abort: DataContext is not PresentationStageViewModel.");
            return;
        }

        var teamIndex = -1;
        for (var i = 0; i < vm.PreFreezeRows.Count; i++)
        {
            if (string.Equals(vm.PreFreezeRows[i].TeamId, request.TeamId, StringComparison.Ordinal))
            {
                teamIndex = i;
                break;
            }
        }

        if (teamIndex < 0 || teamIndex >= ScoreboardList.ItemCount)
        {
            Trace.WriteLine($"[MoveUpAnim] Abort: invalid teamIndex={teamIndex}, itemCount={ScoreboardList.ItemCount}.");
            return;
        }

        var scrollViewer = ScoreboardList.GetVisualDescendants().OfType<ScrollViewer>().FirstOrDefault();
        if (scrollViewer is null)
        {
            Trace.WriteLine("[MoveUpAnim] Abort: ScrollViewer not found.");
            return;
        }

        var viewportTopInOverlay = scrollViewer.TranslatePoint(new Point(0, 0), MoveUpOverlay);
        if (!viewportTopInOverlay.HasValue || scrollViewer.Viewport.Height <= 0)
        {
            Trace.WriteLine($"[MoveUpAnim] Abort: invalid viewport transform/size. hasTop={viewportTopInOverlay.HasValue}, viewportH={scrollViewer.Viewport.Height:F2}.");
            return;
        }

        var rowDelta = request.FromIndex - request.ToIndex;
        if (rowDelta <= 0)
        {
            Trace.WriteLine($"[MoveUpAnim] Abort: non-upward rowDelta={rowDelta}.");
            return;
        }

        var teamContainer = ScoreboardList.ContainerFromIndex(teamIndex) as Control;
        var fromContainer = request.FromIndex >= 0 && request.FromIndex < ScoreboardList.ItemCount
            ? ScoreboardList.ContainerFromIndex(request.FromIndex) as Control
            : null;

        if (teamContainer is null && fromContainer is null && allowDeferredRetry)
        {
            Trace.WriteLine("[MoveUpAnim] Defer once: both destination and source containers unrealized.");
            Dispatcher.UIThread.Post(
                () => TryStartMoveUpAnimation(request, allowDeferredRetry: false),
                DispatcherPriority.Loaded);
            return;
        }

        var metricContainer = fromContainer ?? teamContainer;
        var rowHeight = metricContainer?.Bounds.Height ?? 0;
        var rowWidth = metricContainer?.Bounds.Width ?? 0;
        if (rowHeight <= 0 || rowWidth <= 0)
        {
            Trace.WriteLine($"[MoveUpAnim] Metric fallback: invalid row size w={rowWidth:F2}, h={rowHeight:F2}; using defaults.");
            rowHeight = 62;
            rowWidth = Math.Max(1, scrollViewer.Viewport.Width);
        }

        var destinationVisible = false;
        var destinationY = viewportTopInOverlay.Value.Y;
        var destinationX = viewportTopInOverlay.Value.X;
        if (teamContainer is not null)
        {
            var rowTopInOverlay = teamContainer.TranslatePoint(new Point(0, 0), MoveUpOverlay);
            var rowTopInViewport = teamContainer.TranslatePoint(new Point(0, 0), scrollViewer);
            if (rowTopInOverlay.HasValue && rowTopInViewport.HasValue)
            {
                destinationY = rowTopInOverlay.Value.Y;
                destinationX = rowTopInOverlay.Value.X;
                destinationVisible =
                    rowTopInViewport.Value.Y >= 0 &&
                    rowTopInViewport.Value.Y + rowHeight <= scrollViewer.Viewport.Height;
            }
            else
            {
                Trace.WriteLine("[MoveUpAnim] Destination container exists but transform failed.");
            }
        }
        else
        {
            Trace.WriteLine("[MoveUpAnim] Destination container is null (likely offscreen/unrealized).");
        }

        var startY = destinationY + (rowDelta * rowHeight);
        if (fromContainer is not null)
        {
            var fromTopInOverlay = fromContainer.TranslatePoint(new Point(0, 0), MoveUpOverlay);
            if (fromTopInOverlay.HasValue)
            {
                startY = fromTopInOverlay.Value.Y;
                destinationX = fromTopInOverlay.Value.X;
            }
            else
            {
                Trace.WriteLine("[MoveUpAnim] Source container exists but transform failed.");
            }
        }
        else
        {
            Trace.WriteLine("[MoveUpAnim] Source container is null (fallback startY from delta).");
        }

        var effectiveTargetY = destinationVisible ? destinationY : viewportTopInOverlay.Value.Y;
        var visualTargetY = destinationVisible ? destinationY : (effectiveTargetY - rowHeight);
        var effectiveDistance = Math.Abs(effectiveTargetY - startY);
        var rowFlyDurationSeconds = GetRowFlyAnimationSeconds();
        var durationMs = rowFlyDurationSeconds * 1000.0;
        var derivedSpeed = effectiveDistance / rowFlyDurationSeconds;
        Trace.WriteLine(
            $"[MoveUpAnim] Geometry: startY={startY:F2}, destinationY={destinationY:F2}, edgeY={viewportTopInOverlay.Value.Y:F2}, " +
            $"destinationVisible={destinationVisible}, edgeTargetY={effectiveTargetY:F2}, visualTargetY={visualTargetY:F2}, distance={effectiveDistance:F2}, durationMs={durationMs:F2}, speed={derivedSpeed:F2}px/s, " +
            $"rowW={rowWidth:F2}, rowH={rowHeight:F2}.");

        RenderTargetBitmap? snapshot = null;
        if (teamContainer is not null)
        {
            snapshot = CreateRowSnapshot(teamContainer, rowWidth, rowHeight);
            Trace.WriteLine($"[MoveUpAnim] Snapshot from live container success={snapshot is not null}.");
        }

        Control overlayVisual;
        if (snapshot is not null)
        {
            var overlayImage = new Image
            {
                Source = snapshot,
                Width = rowWidth,
                Height = rowHeight,
                Stretch = Stretch.Fill,
                IsHitTestVisible = false
            };
            overlayVisual = overlayImage;
            Trace.WriteLine("[MoveUpAnim] Overlay visual uses live-container bitmap snapshot.");
        }
        else if (teamIndex >= 0 && teamIndex < vm.PreFreezeRows.Count)
        {
            var presenter = CreateMoveUpOverlayPresenter(vm.PreFreezeRows[teamIndex], rowWidth, rowHeight);
            if (presenter is null)
            {
                Trace.WriteLine("[MoveUpAnim] Abort: failed to create data-template overlay presenter.");
                return;
            }

            overlayVisual = presenter;
            Trace.WriteLine("[MoveUpAnim] Overlay visual uses live data-template presenter fallback.");
        }
        else
        {
            Trace.WriteLine("[MoveUpAnim] Abort: cannot create any overlay visual.");
            return;
        }

        overlayVisual.ZIndex = 2000;
        overlayVisual.IsHitTestVisible = false;
        Canvas.SetLeft(overlayVisual, destinationX);
        Canvas.SetTop(overlayVisual, startY);
        MoveUpOverlay.Children.Add(overlayVisual);

        Control? hiddenRow = null;
        if (teamContainer is not null && destinationVisible)
        {
            teamContainer.Opacity = 0;
            hiddenRow = teamContainer;
        }

        _activeMoveUpAnimations.Add(new ActiveMoveUpAnimation(
            overlayVisual,
            snapshot,
            hiddenRow,
            startY,
            visualTargetY,
            Stopwatch.GetTimestamp(),
            Math.Max(0.001, durationMs / 1000.0)));
        Trace.WriteLine($"[MoveUpAnim] Animation queued: id={request.RequestId}, hiddenRow={hiddenRow is not null}, activeCount={_activeMoveUpAnimations.Count}.");

        QueueDownShiftAnimations(request, rowHeight);
        EnsureMoveUpAnimationTimerRunning();
    }

    private void QueueDownShiftAnimations(MoveUpAnimationRequest request, double rowHeight)
    {
        if (rowHeight <= 0)
        {
            return;
        }

        var startIndex = Math.Max(0, request.ToIndex + 1);
        var endIndex = Math.Min(request.FromIndex, ScoreboardList.ItemCount - 1);
        var timestamp = Stopwatch.GetTimestamp();
        for (var i = startIndex; i <= endIndex; i++)
        {
            var row = ScoreboardList.ContainerFromIndex(i) as Control;
            if (row is null)
            {
                continue;
            }

            var translate = row.RenderTransform as TranslateTransform;
            if (translate is null)
            {
                translate = new TranslateTransform();
                row.RenderTransform = translate;
            }

            RemoveExistingDownShiftForRow(row);
            translate.Y = -rowHeight;
            _activeDownShiftAnimations.Add(new ActiveDownShiftAnimation(
                row,
                translate,
                -rowHeight,
                0,
                timestamp,
                GetScrollAnimationSeconds()));
        }

        if (endIndex >= startIndex)
        {
            Trace.WriteLine($"[MoveUpAnim] Down-shift queued for visible rows in index range [{startIndex}, {endIndex}], active={_activeDownShiftAnimations.Count}.");
        }
    }

    private void EnsureMoveUpAnimationTimerRunning()
    {
        if (_moveUpAnimationTimer is null)
        {
            _moveUpAnimationTimer = new DispatcherTimer(
                TimeSpan.FromMilliseconds(16),
                DispatcherPriority.Render,
                OnMoveUpAnimationTick);
        }

        if (!_moveUpAnimationTimer.IsEnabled)
        {
            _moveUpAnimationTimer.Start();
            Trace.WriteLine("[MoveUpAnim] Timer started.");
        }
    }

    private void OnMoveUpAnimationTick(object? sender, EventArgs e)
    {
        if (_activeMoveUpAnimations.Count == 0 && _activeDownShiftAnimations.Count == 0)
        {
            StopMoveUpAnimationTimer();
            Trace.WriteLine("[MoveUpAnim] Tick with no active animations.");
            return;
        }

        var now = Stopwatch.GetTimestamp();
        for (var i = _activeMoveUpAnimations.Count - 1; i >= 0; i--)
        {
            var animation = _activeMoveUpAnimations[i];
            var elapsedSeconds = (now - animation.StartTimestamp) / (double)Stopwatch.Frequency;
            var progress = Math.Clamp(elapsedSeconds / animation.DurationSeconds, 0, 1);
            var eased = progress;
            var currentY = animation.StartY + ((animation.TargetY - animation.StartY) * eased);
            Canvas.SetTop(animation.OverlayVisual, currentY);

            if (progress >= 1)
            {
                CompleteMoveUpAnimation(animation);
                _activeMoveUpAnimations.RemoveAt(i);
                Trace.WriteLine($"[MoveUpAnim] Animation completed. Remaining={_activeMoveUpAnimations.Count}.");
            }
        }

        for (var i = _activeDownShiftAnimations.Count - 1; i >= 0; i--)
        {
            var animation = _activeDownShiftAnimations[i];
            var elapsedSeconds = (now - animation.StartTimestamp) / (double)Stopwatch.Frequency;
            var progress = Math.Clamp(elapsedSeconds / animation.DurationSeconds, 0, 1);
            var eased = EaseOutCubic(progress);
            animation.Transform.Y = animation.StartY + ((animation.TargetY - animation.StartY) * eased);

            if (progress >= 1)
            {
                animation.Transform.Y = 0;
                _activeDownShiftAnimations.RemoveAt(i);
            }
        }

        if (_activeMoveUpAnimations.Count == 0 && _activeDownShiftAnimations.Count == 0)
        {
            StopMoveUpAnimationTimer();
        }
    }

    private void StopAllMoveUpAnimations()
    {
        Trace.WriteLine($"[MoveUpAnim] StopAllMoveUpAnimations activeMoveUp={_activeMoveUpAnimations.Count}, activeDownShift={_activeDownShiftAnimations.Count}.");
        for (var i = _activeMoveUpAnimations.Count - 1; i >= 0; i--)
        {
            CompleteMoveUpAnimation(_activeMoveUpAnimations[i]);
        }

        _activeMoveUpAnimations.Clear();
        for (var i = _activeDownShiftAnimations.Count - 1; i >= 0; i--)
        {
            _activeDownShiftAnimations[i].Transform.Y = 0;
        }
        _activeDownShiftAnimations.Clear();
        StopMoveUpAnimationTimer();
    }

    private void StopMoveUpAnimationTimer()
    {
        if (_moveUpAnimationTimer is not null)
        {
            _moveUpAnimationTimer.Stop();
            Trace.WriteLine("[MoveUpAnim] Timer stopped.");
        }
    }

    private void AnimateAwardOverlayVisibility(bool visible)
    {
        _awardOverlayFadeStartOpacity = AwardOverlayRoot.Opacity;
        _awardOverlayFadeTargetOpacity = visible ? 1 : 0;
        _awardOverlayFadeStartTimestamp = Stopwatch.GetTimestamp();

        if (visible)
        {
            AwardOverlayRoot.IsVisible = true;
        }

        if (_awardOverlayFadeTimer is null)
        {
            _awardOverlayFadeTimer = new DispatcherTimer(
                TimeSpan.FromMilliseconds(16),
                DispatcherPriority.Render,
                OnAwardOverlayFadeTick);
        }

        _awardOverlayFadeTimer.Stop();
        _awardOverlayFadeTimer.Start();
    }

    private void OnAwardOverlayFadeTick(object? sender, EventArgs e)
    {
        if (_awardOverlayFadeTimer is null)
        {
            return;
        }

        var elapsedSeconds = (Stopwatch.GetTimestamp() - _awardOverlayFadeStartTimestamp) / (double)Stopwatch.Frequency;
        var progress = Math.Clamp(elapsedSeconds / AwardOverlayFadeDuration.TotalSeconds, 0, 1);
        var eased = EaseInOutCubic(progress);
        AwardOverlayRoot.Opacity =
            _awardOverlayFadeStartOpacity + ((_awardOverlayFadeTargetOpacity - _awardOverlayFadeStartOpacity) * eased);

        if (progress >= 1)
        {
            AwardOverlayRoot.Opacity = _awardOverlayFadeTargetOpacity;
            AwardOverlayRoot.IsVisible = _awardOverlayFadeTargetOpacity > 0;
            _awardOverlayFadeTimer.Stop();
        }
    }

    private void StopAwardOverlayFadeAnimation()
    {
        if (_awardOverlayFadeTimer is not null)
        {
            _awardOverlayFadeTimer.Stop();
        }
    }

    private void SetAwardOverlayVisibilityImmediate(bool visible)
    {
        StopAwardOverlayFadeAnimation();
        AwardOverlayRoot.IsVisible = visible;
        AwardOverlayRoot.Opacity = visible ? 1 : 0;
    }

    private void CompleteMoveUpAnimation(ActiveMoveUpAnimation animation)
    {
        if (MoveUpOverlay.Children.Contains(animation.OverlayVisual))
        {
            MoveUpOverlay.Children.Remove(animation.OverlayVisual);
        }

        if (animation.HiddenRow is not null)
        {
            animation.HiddenRow.Opacity = 1;
        }
        animation.Snapshot?.Dispose();
    }

    private void RemoveExistingDownShiftForRow(Control row)
    {
        for (var i = _activeDownShiftAnimations.Count - 1; i >= 0; i--)
        {
            if (ReferenceEquals(_activeDownShiftAnimations[i].Row, row))
            {
                _activeDownShiftAnimations[i].Transform.Y = 0;
                _activeDownShiftAnimations.RemoveAt(i);
            }
        }
    }

    private static RenderTargetBitmap? CreateRowSnapshot(Control source, double rowWidth, double rowHeight)
    {
        var pixelWidth = Math.Max(1, (int)Math.Ceiling(rowWidth));
        var pixelHeight = Math.Max(1, (int)Math.Ceiling(rowHeight));
        var bitmap = new RenderTargetBitmap(new PixelSize(pixelWidth, pixelHeight));
        try
        {
            bitmap.Render(source);
            return bitmap;
        }
        catch
        {
            Trace.WriteLine("[MoveUpAnim] CreateRowSnapshot failed during bitmap.Render.");
            bitmap.Dispose();
            return null;
        }
    }

    private ContentPresenter? CreateMoveUpOverlayPresenter(
        PreFreezeScoreboardRowViewModel rowData,
        double rowWidth,
        double rowHeight)
    {
        if (ScoreboardList.ItemTemplate is null)
        {
            return null;
        }

        var presenter = new ContentPresenter
        {
            Content = rowData,
            ContentTemplate = ScoreboardList.ItemTemplate,
            Width = rowWidth,
            Height = rowHeight,
            IsHitTestVisible = false
        };

        presenter.Measure(new Size(rowWidth, rowHeight));
        presenter.Arrange(new Rect(0, 0, rowWidth, rowHeight));
        presenter.UpdateLayout();

        return presenter;
    }

    private double GetRowFlyAnimationSeconds()
    {
        if (DataContext is PresentationStageViewModel vm)
        {
            return Math.Max(0.01, vm.RowFlyAnimationSeconds);
        }

        return DefaultRowFlyAnimationSeconds;
    }

    private double GetScrollAnimationSeconds()
    {
        if (DataContext is PresentationStageViewModel vm)
        {
            return Math.Max(0.01, vm.ScrollAnimationSeconds);
        }

        return DefaultScrollAnimationSeconds;
    }

    private sealed record ActiveMoveUpAnimation(
        Control OverlayVisual,
        RenderTargetBitmap? Snapshot,
        Control? HiddenRow,
        double StartY,
        double TargetY,
        long StartTimestamp,
        double DurationSeconds);

    private sealed record ActiveDownShiftAnimation(
        Control Row,
        TranslateTransform Transform,
        double StartY,
        double TargetY,
        long StartTimestamp,
        double DurationSeconds);
}
