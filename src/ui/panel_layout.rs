use super::super::*;

const TAB_COUNT: usize = AppTab::Simulate as usize + 1;
const MIN_LEFT_WIDTH: f32 = 280.0;
const MIN_RIGHT_WIDTH: f32 = 360.0;
const MIN_VIEWPORT_WIDTH: f32 = 200.0;
const HANDLE_WIDTH: f32 = 10.0;

#[derive(Clone, Copy, Debug, PartialEq)]
struct PanelWidths {
    left: f32,
    right: f32,
}

impl PanelWidths {
    fn defaults(tab: AppTab) -> Self {
        if tab == AppTab::Editing {
            // Includes the outer margin and half of the viewport gutter.
            Self {
                left: 413.0,
                right: 543.0,
            }
        } else {
            Self {
                left: 384.0,
                right: 420.0,
            }
        }
    }

    fn fit(self, tab: AppTab, screen: f32) -> Self {
        let available = panel_width_budget(tab, screen);
        if !panel_left_visible(tab, screen) {
            return Self {
                left: 0.0,
                right: if tab == AppTab::Validation {
                    0.0
                } else {
                    self.right.clamp(MIN_RIGHT_WIDTH.min(available), available)
                },
            };
        }
        if tab == AppTab::Validation {
            // Validation has a single sidebar beside the full-width report.
            return Self {
                left: self.left.clamp(MIN_LEFT_WIDTH.min(available), available),
                right: 0.0,
            };
        }
        let min_total = MIN_LEFT_WIDTH + MIN_RIGHT_WIDTH;
        if available < min_total {
            return Self {
                left: available * MIN_LEFT_WIDTH / min_total,
                right: available * MIN_RIGHT_WIDTH / min_total,
            };
        }
        let left = self.left.max(MIN_LEFT_WIDTH);
        let right = self.right.max(MIN_RIGHT_WIDTH);
        if left + right <= available {
            return Self { left, right };
        }
        let scale = (available - min_total) / (left + right - min_total);
        Self {
            left: MIN_LEFT_WIDTH + (left - MIN_LEFT_WIDTH) * scale,
            right: MIN_RIGHT_WIDTH + (right - MIN_RIGHT_WIDTH) * scale,
        }
    }

    fn resized(self, side: PanelSide, delta: f32, tab: AppTab, screen: f32) -> Self {
        let available = panel_width_budget(tab, screen);
        match side {
            PanelSide::Left => {
                let max = (available - self.right).max(0.0);
                Self {
                    left: (self.left + delta).clamp(MIN_LEFT_WIDTH.min(max), max),
                    ..self
                }
            }
            PanelSide::Right => {
                let max = (available - self.left).max(0.0);
                Self {
                    right: (self.right - delta).clamp(MIN_RIGHT_WIDTH.min(max), max),
                    ..self
                }
            }
        }
    }
}

fn panel_left_visible(tab: AppTab, screen: f32) -> bool {
    match tab {
        AppTab::Editing => editing_archive_visible_at_width(screen),
        AppTab::Vehicles | AppTab::Race | AppTab::TextureReview => true,
        _ => screen >= LEFT_SIDEBAR_MIN_SCREEN_W,
    }
}

fn panel_width_budget(tab: AppTab, screen: f32) -> f32 {
    let gutter = if tab == AppTab::Editing {
        if panel_left_visible(tab, screen) {
            18.0
        } else {
            23.0
        }
    } else {
        0.0
    };
    // On exceptionally small windows, reserve a proportion instead of letting
    // the sidebars overlap or push the viewport outside the window.
    let minimum = if tab == AppTab::Validation {
        // The report needs room for its action buttons and three lists.
        504.0
    } else {
        MIN_VIEWPORT_WIDTH
    };
    let viewport = minimum.min(screen * 0.7);
    (screen - viewport - gutter).max(0.0)
}

#[derive(Clone, Copy, PartialEq)]
enum PanelSide {
    Left,
    Right,
}

#[derive(Clone, Copy)]
struct PanelDrag {
    side: PanelSide,
    start_x: f32,
    start_widths: PanelWidths,
}

struct PanelLayout {
    tab: AppTab,
    preferred: [Option<PanelWidths>; TAB_COUNT],
    widths: PanelWidths,
    drag: Option<PanelDrag>,
    hovered: Option<PanelSide>,
    owns_pointer: bool,
}

thread_local! {
    // Geometry helpers are shared by drawing and hit testing and intentionally
    // have no AppState argument. Keep their active layout on the UI thread;
    // resizing only updates these small values, never assets or project files.
    static PANEL_LAYOUT: std::cell::RefCell<PanelLayout> = std::cell::RefCell::new(PanelLayout {
        tab: AppTab::Preview,
        preferred: [None; TAB_COUNT],
        widths: PanelWidths::defaults(AppTab::Preview),
        drag: None,
        hovered: None,
        owns_pointer: false,
    });
}

pub(crate) fn sync_panel_layout(tab: AppTab) {
    PANEL_LAYOUT.with_borrow_mut(|layout| {
        if layout.tab != tab {
            layout.drag = None;
            layout.hovered = None;
        }
        layout.tab = tab;
        layout.widths = layout.preferred[tab as usize]
            .unwrap_or_else(|| PanelWidths::defaults(tab))
            .fit(tab, screen_width());
    });
}

pub(crate) fn begin_panel_layout_frame(tab: AppTab) {
    sync_panel_layout(tab);
    PANEL_LAYOUT.with_borrow_mut(|layout| {
        layout.hovered = None;
        if !is_mouse_button_down(MouseButton::Left) && !is_mouse_button_released(MouseButton::Left)
        {
            layout.drag = None;
        }
        layout.owns_pointer = layout.drag.is_some();
    });
    macroquad::miniquad::window::set_mouse_cursor(macroquad::miniquad::CursorIcon::Default);
}

pub(crate) fn left_panel_width() -> f32 {
    PANEL_LAYOUT.with_borrow(|layout| layout.widths.left)
}

pub(crate) fn right_panel_width() -> f32 {
    PANEL_LAYOUT.with_borrow(|layout| layout.widths.right)
}

pub(crate) fn panel_resize_owns_pointer() -> bool {
    PANEL_LAYOUT.with_borrow(|layout| layout.owns_pointer)
}

fn panel_handle_rect(side: PanelSide, tab: AppTab) -> Rect {
    let x = match side {
        PanelSide::Left => left_panel_width(),
        PanelSide::Right => screen_width() - right_panel_width(),
    };
    let top = TOP_H + if tab == AppTab::Editing { 54.0 } else { 0.0 };
    Rect::new(
        x - HANDLE_WIDTH * 0.5,
        top,
        HANDLE_WIDTH,
        (screen_height() - STATUS_H - top).max(0.0),
    )
}

pub(crate) fn update_panel_resize_input(app: &mut AppState, mouse: Vec2) -> bool {
    // Other modal handlers run before this one. Vehicle dialogs are handled
    // later in the input path, so explicitly leave the pointer to them here.
    if !app.options.ui
        || app.context_menu.is_some()
        || app.vehicle_browser.manage_dictionaries
        || app.vehicle_browser.build_dialog.is_some()
        || app.vehicle_browser.collision_copy_dialog.is_some()
        || (app.active_tab == AppTab::Vehicles && app.vehicle_browser.photo_mode)
        || app.camera.looking
        || app.gizmo_drag.is_some()
        || app.box_select_drag.is_some()
        || app.vehicle_browser.gizmo_drag.is_some()
        || app.scrollbar_pointer_captured
        || app.inspector_scroll_drag
        || app.water_list_scroll_drag
        || app.light_list_scroll_drag
        || app.vehicle_browser.component_scroll_drag
        || app.cull_face_drag.is_some()
        || app.navigation_menu_open
        || app.outliner_scroll_drag.is_some()
        || app.asset_browser.resizing
        || app.water_edge_drag.is_some()
        || app.col_box_face_drag.is_some()
    {
        return false;
    }
    let hovered = [PanelSide::Left, PanelSide::Right]
        .into_iter()
        .find(|side| {
            !(*side == PanelSide::Right && app.active_tab == AppTab::Validation)
                && !(*side == PanelSide::Left
                    && !panel_left_visible(app.active_tab, screen_width()))
                && panel_handle_rect(*side, app.active_tab).contains(mouse)
        });
    let consumed = PANEL_LAYOUT.with_borrow_mut(|layout| {
        layout.hovered = hovered;
        if layout.drag.is_none() && is_mouse_button_pressed(MouseButton::Left) {
            layout.drag = hovered.map(|side| PanelDrag {
                side,
                start_x: mouse.x,
                start_widths: layout.widths,
            });
        }
        if let Some(drag) = layout.drag {
            layout.widths = drag
                .start_widths
                .resized(
                    drag.side,
                    mouse.x - drag.start_x,
                    app.active_tab,
                    screen_width(),
                )
                .fit(app.active_tab, screen_width());
            // Keep the hidden browser's preferred width while resizing the inspector.
            let mut preferred = layout.widths;
            if !panel_left_visible(app.active_tab, screen_width()) {
                preferred.left = layout.preferred[app.active_tab as usize]
                    .unwrap_or_else(|| PanelWidths::defaults(app.active_tab))
                    .left;
            }
            layout.preferred[app.active_tab as usize] = Some(preferred);
            layout.owns_pointer = true;
            if is_mouse_button_released(MouseButton::Left)
                || !is_mouse_button_down(MouseButton::Left)
            {
                layout.drag = None;
            }
        } else {
            // Do not intercept a scene drag that merely crosses a divider.
            layout.owns_pointer = hovered.is_some() && !is_mouse_button_down(MouseButton::Left);
        }
        layout.owns_pointer
    });
    if consumed {
        set_ui_interaction_suppressed(true);
        macroquad::miniquad::window::set_mouse_cursor(macroquad::miniquad::CursorIcon::EWResize);
    }
    consumed
}

pub(crate) fn draw_panel_resize_handles(app: &AppState) {
    if app.active_tab == AppTab::Vehicles
        && (app.vehicle_browser.photo_mode
            || app.vehicle_browser.manage_dictionaries
            || app.vehicle_browser.build_dialog.is_some()
            || app.vehicle_browser.collision_copy_dialog.is_some())
    {
        return;
    }
    let (hovered, dragging) =
        PANEL_LAYOUT.with_borrow(|layout| (layout.hovered, layout.drag.map(|drag| drag.side)));
    for side in [PanelSide::Left, PanelSide::Right] {
        if (side == PanelSide::Right && app.active_tab == AppTab::Validation)
            || (side == PanelSide::Left && !panel_left_visible(app.active_tab, screen_width()))
        {
            continue;
        }
        let rect = panel_handle_rect(side, app.active_tab);
        let active = hovered == Some(side) || dragging == Some(side);
        let color = if active { ui_accent() } else { ui_border() };
        let x = rect.x + rect.w * 0.5;
        draw_line(
            x,
            rect.y,
            x,
            rect.y + rect.h,
            if active { 2.0 } else { 1.0 },
            color,
        );
        draw_rrect(
            x - 2.0,
            rect.y + (rect.h - 36.0) * 0.5,
            4.0,
            36.0,
            2.0,
            if active { ui_accent() } else { ui_muted() },
        );
    }
    if hovered.is_some() && dragging.is_none() {
        let mouse: Vec2 = mouse_position().into();
        draw_text_tooltip(
            &app.ui_font,
            Rect::new(mouse.x + 10.0, mouse.y, 1.0, 12.0),
            "Drag to resize panel",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fitting_keeps_the_viewport_inside_small_windows() {
        for tab in [
            AppTab::Preview,
            AppTab::Editing,
            AppTab::Validation,
            AppTab::Cull,
            AppTab::Vehicles,
            AppTab::Race,
        ] {
            for screen in [320.0, 640.0, 800.0, 1024.0, 1280.0, 1600.0] {
                let widths = PanelWidths::defaults(tab).fit(tab, screen);
                assert!(widths.left >= 0.0 && widths.right >= 0.0);
                assert!(widths.left + widths.right <= panel_width_budget(tab, screen) + 0.01);
            }
        }
    }

    #[test]
    fn hidden_sidebars_do_not_reserve_viewport_space() {
        let preferred = PanelWidths::defaults(AppTab::Editing);
        let fitted = preferred.fit(AppTab::Editing, 1024.0);
        assert_eq!(fitted.left, 0.0);
        assert_eq!(fitted.right, preferred.right);
        assert_eq!(preferred.fit(AppTab::Editing, 1600.0), preferred);
        assert_eq!(
            PanelWidths::defaults(AppTab::Preview)
                .fit(AppTab::Preview, 800.0)
                .left,
            0.0
        );
        assert!(
            PanelWidths::defaults(AppTab::Vehicles)
                .fit(AppTab::Vehicles, 800.0)
                .left
                > 0.0
        );
    }

    #[test]
    fn dragging_either_side_preserves_the_other_and_limits_the_viewport() {
        let original = PanelWidths::defaults(AppTab::Preview);
        let left = original.resized(PanelSide::Left, 5000.0, AppTab::Preview, 1280.0);
        assert_eq!(left.right, original.right);
        assert_eq!(left.left + left.right, 1080.0);
        let right = original.resized(PanelSide::Right, -5000.0, AppTab::Preview, 1280.0);
        assert_eq!(right.left, original.left);
        assert_eq!(right.left + right.right, 1080.0);
        assert_eq!(
            original
                .resized(PanelSide::Left, -5000.0, AppTab::Preview, 1280.0)
                .left,
            MIN_LEFT_WIDTH
        );
        assert_eq!(
            original
                .resized(PanelSide::Right, 5000.0, AppTab::Preview, 1280.0)
                .right,
            MIN_RIGHT_WIDTH
        );
    }

    #[test]
    fn window_fitting_does_not_destroy_preferred_sizes() {
        let preferred = PanelWidths {
            left: 500.0,
            right: 600.0,
        };
        assert_ne!(preferred.fit(AppTab::Preview, 1024.0), preferred);
        assert_eq!(preferred.fit(AppTab::Preview, 1600.0), preferred);
        assert_eq!(preferred.fit(AppTab::Validation, 1600.0).right, 0.0);
    }
}
