use super::super::*;

pub(crate) struct Glyph {
    pub(crate) source: Option<Rect>, // None for whitespace / empty glyphs
    pub(crate) w: f32,               // bitmap size, in raster pixels
    pub(crate) h: f32,
    pub(crate) bearing_x: f32, // left offset from pen x
    pub(crate) top: f32,       // distance from baseline up to glyph top
    pub(crate) advance: f32,
}

pub(crate) struct UiFont {
    pub(crate) atlas: Texture2D,
    pub(crate) glyphs: HashMap<char, Glyph>,
    pub(crate) raster_px: f32,
    pub(crate) space_advance: f32,
}

pub(crate) static FONT_REGULAR: OnceLock<UiFont> = OnceLock::new();
pub(crate) static FONT_BOLD: OnceLock<UiFont> = OnceLock::new();
pub(crate) static UI_TEXT_ENABLED: AtomicBool = AtomicBool::new(true);

/// Rasterize the editor's UI character set from a TTF into one atlas texture.
///
/// Latin-1 covers names and paths commonly encountered in user projects. The
/// explicit punctuation set covers the typographic separators, arrows, and
/// disclosure markers used by built-in labels.
pub(crate) fn build_font(font_bytes: &[u8], raster_px: f32) -> Option<UiFont> {
    let font = fontdue::Font::from_bytes(font_bytes, fontdue::FontSettings::default()).ok()?;
    let mut raw = Vec::new();
    let mut max_w = 1usize;
    let mut max_h = 1usize;
    let mut space_advance = raster_px * 0.3;
    let mut characters: Vec<char> = (32u32..=126)
        .chain(160..=255)
        .filter_map(char::from_u32)
        .chain("–—“”•→─▾".chars())
        .collect();
    characters.sort_unstable();
    characters.dedup();
    for ch in characters {
        let (metrics, bitmap) = font.rasterize(ch, raster_px);
        if ch == ' ' {
            space_advance = metrics.advance_width;
        }
        max_w = max_w.max(metrics.width + 2);
        max_h = max_h.max(metrics.height + 2);
        raw.push((ch, metrics, bitmap));
    }

    let cols = 16usize;
    let rows = raw.len().div_ceil(cols).max(1);
    let atlas_w = (cols * max_w).min(u16::MAX as usize);
    let atlas_h = (rows * max_h).min(u16::MAX as usize);
    let mut rgba = vec![0u8; atlas_w * atlas_h * 4];
    for px in rgba.chunks_exact_mut(4) {
        px[0] = 255;
        px[1] = 255;
        px[2] = 255;
    }

    let mut glyphs = HashMap::new();
    for (idx, (ch, metrics, bitmap)) in raw.into_iter().enumerate() {
        let cell_x = (idx % cols) * max_w;
        let cell_y = (idx / cols) * max_h;
        let source = if metrics.width > 0 && metrics.height > 0 {
            for row in 0..metrics.height {
                for col in 0..metrics.width {
                    let a = bitmap[row * metrics.width + col];
                    let dst = ((cell_y + row + 1) * atlas_w + (cell_x + col + 1)) * 4;
                    rgba[dst + 3] = a;
                }
            }
            Some(Rect::new(
                cell_x as f32,
                cell_y as f32,
                (metrics.width + 2) as f32,
                (metrics.height + 2) as f32,
            ))
        } else {
            None
        };
        glyphs.insert(
            ch,
            Glyph {
                source,
                // include the 1px border so dest size matches the padded texture
                w: (metrics.width + 2) as f32,
                h: (metrics.height + 2) as f32,
                bearing_x: metrics.xmin as f32 - 1.0,
                top: (metrics.height as i32 + metrics.ymin) as f32 + 1.0,
                advance: metrics.advance_width,
            },
        );
    }
    let atlas = Texture2D::from_rgba8(atlas_w as u16, atlas_h as u16, &rgba);
    atlas.set_filter(FilterMode::Linear);
    Some(UiFont {
        atlas,
        glyphs,
        raster_px,
        space_advance,
    })
}

/// Default body text. `y` is the text baseline (matches the old API position).
pub(crate) fn ui_text(_font: &Font, text: &str, x: f32, y: f32, color: Color) {
    draw_ui_text(FONT_REGULAR.get(), text, x, y, 16, color);
}

/// Sized text. `size` is the target pixel font size; `y` is the baseline.
pub(crate) fn ui_text_size(_font: &Font, text: &str, x: f32, y: f32, size: u16, color: Color) {
    draw_ui_text(FONT_REGULAR.get(), text, x, y, size, color);
}

/// Bold variant for titles / emphasis.
pub(crate) fn ui_text_bold(text: &str, x: f32, y: f32, size: u16, color: Color) {
    draw_ui_text(
        FONT_BOLD.get().or_else(|| FONT_REGULAR.get()),
        text,
        x,
        y,
        size,
        color,
    );
}

/// Width in pixels that `ui_text_size` would occupy for `text` at `size`.
pub(crate) fn ui_text_width(text: &str, size: u16) -> f32 {
    let Some(font) = FONT_REGULAR.get() else {
        return 0.0;
    };
    ui_text_width_with_font(font, text, size)
}

/// Returns a font size that keeps `text` within `max_width` without truncating it.
pub(crate) fn ui_text_size_to_fit(text: &str, preferred_size: u16, max_width: f32) -> u16 {
    if text.is_empty() || max_width <= 0.0 {
        return preferred_size.max(1);
    }
    let preferred_width = ui_text_width(text, preferred_size);
    if preferred_width <= max_width || preferred_width <= 0.0 {
        return preferred_size;
    }
    let mut size = ((preferred_size as f32 * max_width / preferred_width).floor() as u16).max(1);
    while size > 1 && ui_text_width(text, size) > max_width {
        size -= 1;
    }
    size
}

pub(crate) fn glyph_advance(font: &UiFont, ch: char, scale: f32) -> f32 {
    if ch == '\t' {
        font.space_advance * scale * 4.0
    } else if let Some(g) = font.glyphs.get(&ch).or_else(|| font.glyphs.get(&'?')) {
        g.advance * scale
    } else {
        font.space_advance * scale
    }
}

pub(crate) fn ui_text_width_with_font(font: &UiFont, text: &str, size: u16) -> f32 {
    let scale = size as f32 / font.raster_px;
    let mut pen = 0.0;
    for ch in text.chars() {
        pen += glyph_advance(font, ch, scale);
    }
    pen
}

pub(crate) fn draw_ui_text(
    font: Option<&UiFont>,
    text: &str,
    x: f32,
    y: f32,
    size: u16,
    color: Color,
) {
    if !UI_TEXT_ENABLED.load(Ordering::Relaxed) {
        return;
    }
    let Some(font) = font else { return };
    let scale = size as f32 / font.raster_px;
    let mut pen = x.round();
    let baseline = y.round();
    for ch in text.chars() {
        if ch == '\t' {
            pen += font.space_advance * scale * 4.0;
            continue;
        }
        let glyph = font.glyphs.get(&ch).or_else(|| font.glyphs.get(&'?'));
        if let Some(g) = glyph {
            if let Some(source) = g.source {
                draw_texture_ex(
                    &font.atlas,
                    pen + g.bearing_x * scale,
                    baseline - g.top * scale,
                    color,
                    DrawTextureParams {
                        source: Some(source),
                        dest_size: Some(vec2(g.w * scale, g.h * scale)),
                        ..Default::default()
                    },
                );
            }
            pen += g.advance * scale;
        } else {
            pen += font.space_advance * scale;
        }
    }
}

/// Filled rounded rectangle (macroquad has no native primitive for this).
pub(crate) fn draw_rrect(x: f32, y: f32, w: f32, h: f32, r: f32, color: Color) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    if r <= 0.5 {
        draw_rectangle(x, y, w, h, color);
        return;
    }
    draw_rectangle(x + r, y, w - 2.0 * r, h, color);
    draw_rectangle(x, y + r, r, h - 2.0 * r, color);
    draw_rectangle(x + w - r, y + r, r, h - 2.0 * r, color);
    draw_circle(x + r, y + r, r, color);
    draw_circle(x + w - r, y + r, r, color);
    draw_circle(x + r, y + h - r, r, color);
    draw_circle(x + w - r, y + h - r, r, color);
}

/// Rounded rectangle with a 1px-ish border, drawn as border underneath fill.
pub(crate) fn draw_rrect_bordered(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    thick: f32,
    fill: Color,
    border: Color,
) {
    draw_rrect(x, y, w, h, r, border);
    draw_rrect(
        x + thick,
        y + thick,
        w - 2.0 * thick,
        h - 2.0 * thick,
        (r - thick).max(0.0),
        fill,
    );
}

/// Canonical RGB(A) channel slider used by every editor color picker.
pub(crate) fn draw_color_channel_slider(rect: Rect, tint: Color, value: f32, alpha: f32) {
    let alpha = alpha.clamp(0.0, 1.0);
    let value = value.clamp(0.0, 1.0);
    draw_rrect_bordered(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        6.0,
        1.0,
        Color::new(0.050, 0.058, 0.070, 1.0),
        if alpha < 0.999 {
            ui_muted()
        } else {
            ui_border()
        },
    );
    draw_rrect(
        rect.x,
        rect.y,
        rect.w * value,
        rect.h,
        6.0,
        Color::new(tint.r, tint.g, tint.b, alpha),
    );
    let marker_x = rect.x + rect.w * value;
    draw_line(
        marker_x,
        rect.y - 2.0,
        marker_x,
        rect.y + rect.h + 2.0,
        2.0,
        Color::new(WHITE.r, WHITE.g, WHITE.b, alpha),
    );
}

pub(crate) fn ui_temperature_fraction(temperature: f32, minimum: f32, maximum: f32) -> f32 {
    let minimum_log = minimum.max(1.0).ln();
    let maximum_log = maximum.max(minimum + 1.0).ln();
    ((temperature.clamp(minimum, maximum).ln() - minimum_log) / (maximum_log - minimum_log))
        .clamp(0.0, 1.0)
}

pub(crate) fn ui_temperature_from_fraction(fraction: f32, minimum: f32, maximum: f32) -> f32 {
    let minimum_log = minimum.max(1.0).ln();
    let maximum_log = maximum.max(minimum + 1.0).ln();
    (minimum_log + fraction.clamp(0.0, 1.0) * (maximum_log - minimum_log)).exp()
}

/// Canonical warm-to-cool temperature slider matching the RGB slider geometry.
pub(crate) fn draw_temperature_channel_slider(
    rect: Rect,
    temperature: f32,
    minimum: f32,
    maximum: f32,
    alpha: f32,
) {
    let alpha = alpha.clamp(0.0, 1.0);
    let background = Color::new(0.050, 0.058, 0.070, 1.0);
    draw_rrect_bordered(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        6.0,
        1.0,
        background,
        if alpha < 0.999 {
            ui_muted()
        } else {
            ui_border()
        },
    );
    const SEGMENTS: usize = 96;
    let radius = 6.0f32.min(rect.h * 0.5);
    for segment in 0..SEGMENTS {
        let start = segment as f32 / SEGMENTS as f32;
        let end = (segment + 1) as f32 / SEGMENTS as f32;
        let sample = ui_temperature_from_fraction((start + end) * 0.5, minimum, maximum);
        let source = color_from_temperature(sample);
        let color = Color::new(
            source.x * alpha + background.r * (1.0 - alpha),
            source.y * alpha + background.g * (1.0 - alpha),
            source.z * alpha + background.b * (1.0 - alpha),
            1.0,
        );
        let midpoint_x = rect.w * (start + end) * 0.5;
        let edge_distance = midpoint_x.min(rect.w - midpoint_x);
        let vertical_inset = if edge_distance < radius {
            let dx = radius - edge_distance;
            radius - (radius * radius - dx * dx).max(0.0).sqrt()
        } else {
            0.0
        };
        draw_rectangle(
            rect.x + rect.w * start,
            rect.y + vertical_inset,
            rect.w * (end - start) + 0.5,
            (rect.h - vertical_inset * 2.0).max(0.0),
            color,
        );
    }
    let marker_x = rect.x + rect.w * ui_temperature_fraction(temperature, minimum, maximum);
    draw_line(
        marker_x,
        rect.y - 2.0,
        marker_x,
        rect.y + rect.h + 2.0,
        2.0,
        Color::new(WHITE.r, WHITE.g, WHITE.b, alpha),
    );
}

pub(crate) fn ui_dim() -> Color {
    // Secondary text is used extensively in inspectors and tables. Keep it
    // distinctly subordinate to white without making it disappear on dark
    // surfaces.
    Color::new(0.70, 0.72, 0.77, 1.0)
}

pub(crate) fn ui_muted() -> Color {
    // Muted copy still needs to be readable at the small sizes used for
    // descriptions, paths, and inactive metadata.
    Color::new(0.54, 0.56, 0.61, 1.0)
}

pub(crate) fn ui_panel_bg() -> Color {
    Color::new(0.078, 0.082, 0.090, 1.0)
}

pub(crate) fn ui_border() -> Color {
    Color::new(0.165, 0.175, 0.190, 1.0)
}

pub(crate) fn ui_accent() -> Color {
    Color::new(0.30, 0.61, 0.96, 1.0)
}

pub(crate) fn ui_canvas_bg() -> Color {
    Color::new(0.030, 0.032, 0.036, 1.0)
}

pub(crate) fn ui_shell_bg() -> Color {
    Color::new(0.060, 0.064, 0.070, 1.0)
}

pub(crate) fn ui_surface() -> Color {
    Color::new(0.100, 0.105, 0.114, 1.0)
}

pub(crate) fn ui_surface_hover() -> Color {
    if ui_interaction_suppressed() {
        ui_surface()
    } else {
        Color::new(0.140, 0.147, 0.158, 1.0)
    }
}

pub(crate) fn ui_surface_active() -> Color {
    Color::new(0.180, 0.190, 0.205, 1.0)
}

pub(crate) fn ui_input_bg() -> Color {
    Color::new(0.050, 0.053, 0.059, 1.0)
}

pub(crate) fn ui_accent_soft() -> Color {
    Color::new(0.145, 0.152, 0.165, 1.0)
}

pub(crate) fn ui_shadow() -> Color {
    Color::new(0.0, 0.0, 0.0, 0.28)
}

/// A compact, high-contrast warning with a consistent semantic treatment.
/// `title` identifies the decision while `detail` explains its consequence.
pub(crate) fn draw_ui_warning_alert(font: &Font, rect: Rect, title: &str, detail: &str) {
    let border = Color::new(0.88, 0.62, 0.16, 1.0);
    let heading = Color::new(1.0, 0.82, 0.39, 1.0);
    draw_rrect_bordered(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        7.0,
        1.0,
        Color::new(0.19, 0.135, 0.035, 0.96),
        border,
    );
    // The colored marker makes the severity recognizable before reading.
    draw_circle(rect.x + 17.0, rect.y + 17.0, 9.0, border);
    ui_text_bold("!", rect.x + 14.6, rect.y + 21.5, 13, ui_input_bg());
    ui_text_bold(title, rect.x + 32.0, rect.y + 18.0, 14, heading);
    ui_text_size(font, detail, rect.x + 32.0, rect.y + 36.0, 13, ui_dim());
}

pub(crate) fn draw_panel_rect(_font: &Font, rect: Rect, title: Option<&str>) {
    draw_rrect(
        rect.x + 2.0,
        rect.y + 4.0,
        rect.w,
        rect.h,
        14.0,
        ui_shadow(),
    );
    draw_rrect_bordered(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        14.0,
        1.0,
        ui_panel_bg(),
        ui_border(),
    );
    if let Some(title) = title {
        draw_rrect(rect.x + 14.0, rect.y + 11.0, 3.0, 17.0, 1.5, ui_accent());
        ui_text_bold(title, rect.x + 25.0, rect.y + 26.0, 16, WHITE);
        draw_line(
            rect.x + 14.0,
            rect.y + 36.0,
            rect.x + rect.w - 14.0,
            rect.y + 36.0,
            1.0,
            ui_border(),
        );
    }
}

pub(crate) fn begin_ui_clip(rect: Rect) {
    let mut gl = unsafe { get_internal_gl() };
    gl.flush();
    gl.quad_gl.scissor(Some((
        rect.x.round() as i32,
        rect.y.round() as i32,
        rect.w.round().max(1.0) as i32,
        rect.h.round().max(1.0) as i32,
    )));
}

pub(crate) fn end_ui_clip() {
    let mut gl = unsafe { get_internal_gl() };
    gl.flush();
    gl.quad_gl.scissor(None);
}

/// Draw already-loaded OpenGL textures in UI pixel coordinates without
/// decoding or uploading duplicate thumbnail copies.
pub(crate) fn draw_raw_texture_quads(textures: &[(u32, Rect)]) {
    if textures.is_empty() {
        return;
    }
    let mut internal = unsafe { get_internal_gl() };
    internal.flush();
    drop(internal);
    unsafe {
        let mut previous_program = 0i32;
        let mut previous_array_buffer = 0i32;
        let mut previous_element_buffer = 0i32;
        let mut previous_active_texture = 0i32;
        let mut previous_texture = 0i32;
        let mut previous_matrix_mode = 0i32;
        gl::GetIntegerv(gl::CURRENT_PROGRAM, &mut previous_program);
        gl::GetIntegerv(gl::ARRAY_BUFFER_BINDING, &mut previous_array_buffer);
        gl::GetIntegerv(
            gl::ELEMENT_ARRAY_BUFFER_BINDING,
            &mut previous_element_buffer,
        );
        gl::GetIntegerv(gl::ACTIVE_TEXTURE, &mut previous_active_texture);
        gl::GetIntegerv(gl::MATRIX_MODE, &mut previous_matrix_mode);
        gl::ActiveTexture(gl::TEXTURE0);
        gl::GetIntegerv(gl::TEXTURE_BINDING_2D, &mut previous_texture);

        gl::PushAttrib(gl::ALL_ATTRIB_BITS);
        gl::UseProgram(0);
        gl::ActiveTexture(gl::TEXTURE0);
        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, 0);
        gl::Disable(gl::DEPTH_TEST);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::CULL_FACE);
        gl::Disable(gl::ALPHA_TEST);
        gl::Enable(gl::TEXTURE_2D);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::Color4f(1.0, 1.0, 1.0, 1.0);

        gl::MatrixMode(gl::PROJECTION);
        gl::PushMatrix();
        gl::LoadIdentity();
        gl::Ortho(
            0.0,
            screen_width() as f64,
            screen_height() as f64,
            0.0,
            -1.0,
            1.0,
        );
        gl::MatrixMode(gl::MODELVIEW);
        gl::PushMatrix();
        gl::LoadIdentity();

        for (texture, rect) in textures {
            if *texture == 0 || rect.w <= 0.0 || rect.h <= 0.0 {
                continue;
            }
            gl::BindTexture(gl::TEXTURE_2D, *texture);
            gl::Begin(gl::QUADS);
            gl::TexCoord2f(0.0, 0.0);
            gl::Vertex2f(rect.x, rect.y);
            gl::TexCoord2f(1.0, 0.0);
            gl::Vertex2f(rect.x + rect.w, rect.y);
            gl::TexCoord2f(1.0, 1.0);
            gl::Vertex2f(rect.x + rect.w, rect.y + rect.h);
            gl::TexCoord2f(0.0, 1.0);
            gl::Vertex2f(rect.x, rect.y + rect.h);
            gl::End();
        }

        gl::BindTexture(gl::TEXTURE_2D, 0);
        gl::PopMatrix();
        gl::MatrixMode(gl::PROJECTION);
        gl::PopMatrix();
        gl::PopAttrib();
        gl::MatrixMode(previous_matrix_mode as u32);
        gl::UseProgram(previous_program as u32);
        gl::BindBuffer(gl::ARRAY_BUFFER, previous_array_buffer as u32);
        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, previous_element_buffer as u32);
        gl::ActiveTexture(gl::TEXTURE0);
        gl::BindTexture(gl::TEXTURE_2D, previous_texture as u32);
        gl::ActiveTexture(previous_active_texture as u32);
    }
}
