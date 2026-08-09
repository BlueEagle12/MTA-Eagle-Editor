use super::super::*;

static HOVER_SUPPRESSED: AtomicBool = AtomicBool::new(false);
static UI_INTERACTION_SUPPRESSED: AtomicBool = AtomicBool::new(false);

/// Wheel deltas are not expressed in consistent units by miniquad's native
/// backends: Windows forwards `WHEEL_DELTA` units (120 per notch), while Linux
/// forwards steps and macOS may forward high-resolution trackpad deltas.
/// Normalize that mismatch and cap a single frame so an unusual device event
/// or a stalled frame cannot send a scrollable control straight to its end.
const MAX_WHEEL_STEPS_PER_FRAME: f32 = 3.0;

fn normalize_wheel_axis(raw: f32, windows_wheel_delta_units: bool) -> f32 {
    if !raw.is_finite() {
        return 0.0;
    }
    let steps = if windows_wheel_delta_units {
        raw / 120.0
    } else {
        raw
    };
    steps.clamp(-MAX_WHEEL_STEPS_PER_FRAME, MAX_WHEEL_STEPS_PER_FRAME)
}

pub(crate) fn safe_mouse_wheel() -> (f32, f32) {
    let (x, y) = mouse_wheel();
    let windows_wheel_delta_units = cfg!(windows);
    (
        normalize_wheel_axis(x, windows_wheel_delta_units),
        normalize_wheel_axis(y, windows_wheel_delta_units),
    )
}

pub(crate) fn set_scrollbar_hover_suppressed(value: bool) {
    HOVER_SUPPRESSED.store(value, Ordering::Relaxed);
}

pub(crate) fn scrollbar_hover_suppressed() -> bool {
    HOVER_SUPPRESSED.load(Ordering::Relaxed)
}

/// Suppresses visual hover affordances while pointer input belongs to a
/// viewport gesture, such as freecam look.
pub(crate) fn set_ui_interaction_suppressed(value: bool) {
    UI_INTERACTION_SUPPRESSED.store(value, Ordering::Relaxed);
}

pub(crate) fn ui_interaction_suppressed() -> bool {
    UI_INTERACTION_SUPPRESSED.load(Ordering::Relaxed)
}

/// The sole visual and geometry implementation for custom editor scrollbars.
/// Callers retain only their scroll value and an optional `ScrollbarDrag`.
const SCROLLBAR_WIDTH: f32 = 6.0;
#[derive(Clone, Copy)]
pub(crate) struct ScrollbarMetrics {
    pub(crate) track: Rect,
    pub(crate) thumb: Rect,
    pub(crate) max_scroll: f32,
}

#[derive(Clone, Copy)]
pub(crate) struct ScrollbarDrag {
    pub(crate) grab_offset_y: f32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScrollbarVisualState {
    Idle,
    Hovered,
    Dragging,
}

pub(crate) fn scrollbar_hit_area(track: Rect) -> Rect {
    Rect::new(track.x - 6.0, track.y, track.w + 12.0, track.h)
}

pub(crate) fn scrollbar_scroll_from_pointer(
    track: Rect,
    thumb_h: f32,
    max_scroll: f32,
    pointer: Vec2,
) -> f32 {
    let metrics = ScrollbarMetrics {
        track,
        thumb: Rect::new(track.x, track.y, track.w, thumb_h),
        max_scroll,
    };
    scrollbar_scroll_for_drag(
        metrics,
        ScrollbarDrag {
            grab_offset_y: thumb_h * 0.5,
        },
        pointer,
    )
}

pub(crate) fn scrollbar_metrics(
    track: Rect,
    viewport: f32,
    content: f32,
    min_thumb: f32,
    scroll: f32,
) -> Option<ScrollbarMetrics> {
    // Hit-testing, dragging, and rendering must use the exact same rail.
    let track = Rect::new(
        track.x + (track.w - SCROLLBAR_WIDTH) * 0.5,
        track.y,
        SCROLLBAR_WIDTH,
        track.h,
    );
    let content = content.max(viewport);
    let max_scroll = (content - viewport).max(0.0);
    if max_scroll <= f32::EPSILON || track.h <= 0.0 {
        return None;
    }
    let thumb_h = (track.h * viewport / content).clamp(min_thumb.min(track.h), track.h);
    let travel = (track.h - thumb_h).max(0.0);
    let thumb_y = track.y + travel * (scroll / max_scroll).clamp(0.0, 1.0);
    Some(ScrollbarMetrics {
        track,
        thumb: Rect::new(track.x, thumb_y, track.w, thumb_h),
        max_scroll,
    })
}

pub(crate) fn scrollbar_begin_drag(
    metrics: ScrollbarMetrics,
    mouse: Vec2,
) -> Option<ScrollbarDrag> {
    metrics.thumb.contains(mouse).then(|| ScrollbarDrag {
        grab_offset_y: mouse.y - metrics.thumb.y,
    })
}

pub(crate) fn scrollbar_scroll_for_drag(
    metrics: ScrollbarMetrics,
    drag: ScrollbarDrag,
    mouse: Vec2,
) -> f32 {
    let travel = (metrics.track.h - metrics.thumb.h).max(1.0);
    let y = (mouse.y - drag.grab_offset_y).clamp(
        metrics.track.y,
        metrics.track.y + metrics.track.h - metrics.thumb.h,
    );
    (((y - metrics.track.y) / travel) * metrics.max_scroll).clamp(0.0, metrics.max_scroll)
}

pub(crate) fn draw_scrollbar(metrics: ScrollbarMetrics, state: ScrollbarVisualState) {
    let thumb_color = match state {
        ScrollbarVisualState::Idle => Color::new(0.25, 0.35, 0.48, 1.0),
        ScrollbarVisualState::Hovered => Color::new(0.31, 0.57, 0.82, 1.0),
        ScrollbarVisualState::Dragging => WHITE,
    };
    let x = metrics.track.x + (metrics.track.w - SCROLLBAR_WIDTH) * 0.5;
    draw_rrect(
        x,
        metrics.track.y,
        SCROLLBAR_WIDTH,
        metrics.track.h,
        2.0,
        Color::new(0.055, 0.070, 0.090, 1.0),
    );
    draw_rrect(
        x,
        metrics.thumb.y,
        SCROLLBAR_WIDTH,
        metrics.thumb.h,
        2.0,
        thumb_color,
    );
}

/// Derives the common visual state without allowing individual panels to
/// reimplement hover colors or alter the rail geometry.
pub(crate) fn scrollbar_visual_state(track: Rect, dragging: bool) -> ScrollbarVisualState {
    if HOVER_SUPPRESSED.load(Ordering::Relaxed) && !dragging {
        return ScrollbarVisualState::Idle;
    }
    let hovered = scrollbar_hit_area(track).contains(mouse_position().into());
    if dragging {
        ScrollbarVisualState::Dragging
    } else if hovered {
        ScrollbarVisualState::Hovered
    } else {
        ScrollbarVisualState::Idle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wheel_delta_is_normalized_across_native_backends() {
        assert_eq!(normalize_wheel_axis(120.0, true), 1.0);
        assert_eq!(normalize_wheel_axis(-240.0, true), -2.0);
        assert_eq!(normalize_wheel_axis(1.0, false), 1.0);
    }

    #[test]
    fn wheel_delta_rejects_invalid_and_runaway_input() {
        assert_eq!(normalize_wheel_axis(f32::NAN, false), 0.0);
        assert_eq!(normalize_wheel_axis(f32::INFINITY, false), 0.0);
        assert_eq!(normalize_wheel_axis(50.0, false), MAX_WHEEL_STEPS_PER_FRAME);
        assert_eq!(
            normalize_wheel_axis(-600.0, true),
            -MAX_WHEEL_STEPS_PER_FRAME
        );
    }
}
