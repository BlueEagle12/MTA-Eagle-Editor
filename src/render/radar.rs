use super::super::*;

pub(crate) const RADAR_IMAGE_SIZE: u32 = 2048;
pub(crate) const RADAR_WORLD_SIZE: f32 = 6000.0;
pub(crate) const RADAR_WORLD_CENTER: f32 = 0.0;

fn delete_radar_targets(framebuffer: u32, color_texture: u32, depth_buffer: u32) {
    unsafe {
        if depth_buffer != 0 {
            gl::DeleteRenderbuffers(1, &depth_buffer);
        }
        if color_texture != 0 {
            gl::DeleteTextures(1, &color_texture);
        }
        if framebuffer != 0 {
            gl::DeleteFramebuffers(1, &framebuffer);
        }
    }
}

fn flip_rgba_rows(pixels: &mut [u8], width: u32, height: u32) {
    let row_bytes = width as usize * 4;
    for y in 0..height as usize / 2 {
        let opposite = height as usize - 1 - y;
        let (head, tail) = pixels.split_at_mut(opposite * row_bytes);
        head[y * row_bytes..(y + 1) * row_bytes].swap_with_slice(&mut tail[..row_bytes]);
    }
}

fn draw_radar_placement(placement: &Placement, mesh: &RenderMesh) {
    let model = placement_matrix(placement).to_cols_array();
    let placement_alpha = placement_alpha(placement);
    unsafe {
        gl::PushMatrix();
        gl::MultMatrixf(model.as_ptr());
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::CULL_FACE);
        for part in &mesh.parts {
            let alpha = placement_alpha * part.alpha;
            if alpha <= 0.01 {
                continue;
            }
            if part.transparency == TransparencyMode::Blend || alpha < 0.999 {
                gl::Enable(gl::BLEND);
                gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
                gl::Enable(gl::ALPHA_TEST);
                gl::AlphaFunc(gl::GREATER, 0.02);
            } else {
                gl::Disable(gl::BLEND);
                gl::Enable(gl::ALPHA_TEST);
                gl::AlphaFunc(gl::GREATER, 0.08);
            }
            // A radar is a surface map, so even translucent map geometry needs
            // to establish its topmost depth instead of exposing layers below.
            gl::DepthMask(gl::TRUE);
            if part.texture != 0 {
                gl::Enable(gl::TEXTURE_2D);
                gl::BindTexture(gl::TEXTURE_2D, part.texture);
                gl::Color4f(1.0, 1.0, 1.0, alpha);
            } else {
                gl::Disable(gl::TEXTURE_2D);
                let color = part.material_color;
                gl::Color4f(
                    (0.30 + color.x * 0.70).clamp(0.0, 1.0),
                    (0.30 + color.y * 0.70).clamp(0.0, 1.0),
                    (0.30 + color.z * 0.70).clamp(0.0, 1.0),
                    alpha,
                );
            }
            gl::Begin(gl::TRIANGLES);
            for vertex in &part.cpu_vertices {
                gl::TexCoord2f(vertex.uv.u, vertex.uv.v);
                gl::Vertex3f(vertex.pos.x, vertex.pos.y, vertex.pos.z);
            }
            gl::End();
        }
        gl::PopMatrix();
    }
}

/// Renders the complete -3000..3000 map into the conventional race radar path.
///
/// The image is north-up: +X points right and +Y points toward the top edge.
pub(crate) fn generate_map_radar_png(app: &AppState) -> Result<PathBuf, String> {
    let size = RADAR_IMAGE_SIZE as i32;
    let mut max_texture_size = 0;
    let mut max_renderbuffer_size = 0;
    unsafe {
        gl::GetIntegerv(gl::MAX_TEXTURE_SIZE, &mut max_texture_size);
        gl::GetIntegerv(gl::MAX_RENDERBUFFER_SIZE, &mut max_renderbuffer_size);
    }
    if max_texture_size < size || max_renderbuffer_size < size {
        return Err(format!(
            "GPU cannot create a {size}x{size} radar target (texture limit {max_texture_size}, renderbuffer limit {max_renderbuffer_size})"
        ));
    }

    let mut previous_framebuffer = 0;
    let mut framebuffer = 0;
    let mut color_texture = 0;
    let mut depth_buffer = 0;
    unsafe {
        let mut internal = get_internal_gl();
        internal.flush();
        drop(internal);

        gl::GetIntegerv(gl::FRAMEBUFFER_BINDING, &mut previous_framebuffer);
        gl::GenFramebuffers(1, &mut framebuffer);
        gl::GenTextures(1, &mut color_texture);
        gl::GenRenderbuffers(1, &mut depth_buffer);
        if framebuffer == 0 || color_texture == 0 || depth_buffer == 0 {
            delete_radar_targets(framebuffer, color_texture, depth_buffer);
            return Err("OpenGL could not allocate the radar render target".to_string());
        }

        gl::BindTexture(gl::TEXTURE_2D, color_texture);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA8 as i32,
            size,
            size,
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            std::ptr::null(),
        );

        gl::BindRenderbuffer(gl::RENDERBUFFER, depth_buffer);
        gl::RenderbufferStorage(gl::RENDERBUFFER, gl::DEPTH_COMPONENT24, size, size);

        gl::BindFramebuffer(gl::FRAMEBUFFER, framebuffer);
        gl::FramebufferTexture2D(
            gl::FRAMEBUFFER,
            gl::COLOR_ATTACHMENT0,
            gl::TEXTURE_2D,
            color_texture,
            0,
        );
        gl::FramebufferRenderbuffer(
            gl::FRAMEBUFFER,
            gl::DEPTH_ATTACHMENT,
            gl::RENDERBUFFER,
            depth_buffer,
        );
        if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
            gl::BindFramebuffer(gl::FRAMEBUFFER, previous_framebuffer as u32);
            gl::BindRenderbuffer(gl::RENDERBUFFER, 0);
            gl::BindTexture(gl::TEXTURE_2D, 0);
            delete_radar_targets(framebuffer, color_texture, depth_buffer);
            return Err("OpenGL reported an incomplete radar framebuffer".to_string());
        }

        gl::PushAttrib(gl::ALL_ATTRIB_BITS);
        gl::MatrixMode(gl::PROJECTION);
        gl::PushMatrix();
        gl::MatrixMode(gl::MODELVIEW);
        gl::PushMatrix();

        gl::UseProgram(0);
        gl::Viewport(0, 0, size, size);
        gl::Disable(gl::SCISSOR_TEST);
        gl::Disable(gl::MULTISAMPLE);
        gl::ClearColor(0.075, 0.125, 0.17, 1.0);
        gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        gl::Enable(gl::DEPTH_TEST);
        gl::DepthFunc(gl::LEQUAL);
        gl::DepthMask(gl::TRUE);
        gl::Enable(gl::CULL_FACE);
        gl::CullFace(gl::BACK);
        gl::FrontFace(gl::CW);
        gl::Disable(gl::BLEND);
        gl::Enable(gl::ALPHA_TEST);
        gl::AlphaFunc(gl::GREATER, 0.08);
        gl::Enable(gl::COLOR_MATERIAL);
        gl::ColorMaterial(gl::FRONT_AND_BACK, gl::AMBIENT_AND_DIFFUSE);
        gl::Enable(gl::LIGHT0);
        gl::LightModelfv(gl::LIGHT_MODEL_AMBIENT, [0.68, 0.70, 0.73, 1.0].as_ptr());

        let half = RADAR_WORLD_SIZE * 0.5;
        let eye = vec3(RADAR_WORLD_CENTER, RADAR_WORLD_CENTER, 10_000.0);
        let view = Mat4::look_at_rh(
            eye,
            vec3(RADAR_WORLD_CENTER, RADAR_WORLD_CENTER, 0.0),
            Vec3::Y,
        );
        let projection = Mat4::orthographic_rh_gl(-half, half, -half, half, 0.1, 24_000.0);
        gl::MatrixMode(gl::PROJECTION);
        gl::LoadMatrixf(projection.to_cols_array().as_ptr());
        gl::MatrixMode(gl::MODELVIEW);
        gl::LoadMatrixf(view.to_cols_array().as_ptr());

        // Light positions are transformed by the current model-view matrix.
        gl::Lightfv(gl::LIGHT0, gl::POSITION, [0.35, -0.45, 1.0, 0.0].as_ptr());
        gl::Lightfv(gl::LIGHT0, gl::AMBIENT, [0.60, 0.62, 0.65, 1.0].as_ptr());
        gl::Lightfv(gl::LIGHT0, gl::DIFFUSE, [0.72, 0.70, 0.66, 1.0].as_ptr());
        gl::Lightfv(gl::LIGHT0, gl::SPECULAR, [0.0, 0.0, 0.0, 1.0].as_ptr());

        // The viewport's pre-batched cells retain RenderWare's camera-facing
        // culling assumptions. From a straight-down orthographic camera that
        // can reject terrain and roof triangles, leaving only the background.
        // Export live detail placements explicitly and make every part
        // double-sided so all top-facing map surfaces participate.
        for (index, placement) in app.placements.iter().enumerate() {
            if app
                .element_states
                .get(index)
                .is_some_and(|state| state.deleted)
                || placement_is_app_lod(app, placement)
            {
                continue;
            }
            let Some(mesh) = element_mesh(app, placement) else {
                continue;
            };
            draw_radar_placement(placement, mesh);
        }

        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        gl::BindTexture(gl::TEXTURE_2D, 0);
        gl::Disable(gl::TEXTURE_2D);
        gl::DisableClientState(gl::COLOR_ARRAY);
        gl::DisableClientState(gl::TEXTURE_COORD_ARRAY);
        gl::DisableClientState(gl::NORMAL_ARRAY);
        gl::DisableClientState(gl::VERTEX_ARRAY);

        // Water is intentionally rendered after terrain so coastlines remain
        // legible while open water still gets a useful blue tint.
        draw_water_planes(app);

        gl::Finish();
        let mut pixels = vec![0u8; RADAR_IMAGE_SIZE as usize * RADAR_IMAGE_SIZE as usize * 4];
        gl::ReadBuffer(gl::COLOR_ATTACHMENT0);
        gl::PixelStorei(gl::PACK_ALIGNMENT, 1);
        gl::ReadPixels(
            0,
            0,
            size,
            size,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            pixels.as_mut_ptr().cast(),
        );

        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        gl::BindTexture(gl::TEXTURE_2D, 0);
        gl::DisableClientState(gl::COLOR_ARRAY);
        gl::DisableClientState(gl::TEXTURE_COORD_ARRAY);
        gl::DisableClientState(gl::NORMAL_ARRAY);
        gl::DisableClientState(gl::VERTEX_ARRAY);
        gl::BindRenderbuffer(gl::RENDERBUFFER, 0);
        gl::MatrixMode(gl::MODELVIEW);
        gl::PopMatrix();
        gl::MatrixMode(gl::PROJECTION);
        gl::PopMatrix();
        gl::MatrixMode(gl::MODELVIEW);
        gl::PopAttrib();
        gl::BindFramebuffer(gl::FRAMEBUFFER, previous_framebuffer as u32);
        delete_radar_targets(framebuffer, color_texture, depth_buffer);

        // OpenGL returns the bottom row first; PNG decoders expect the top row
        // first. Flipping here preserves the race editor's north-up convention.
        flip_rgba_rows(&mut pixels, RADAR_IMAGE_SIZE, RADAR_IMAGE_SIZE);

        let output_dir = race_dir(&app.root);
        fs::create_dir_all(&output_dir)
            .map_err(|error| format!("create {}: {error}", output_dir.display()))?;
        let output = output_dir.join("radar.png");
        let temporary = output_dir.join("radar.generated.tmp.png");
        image::save_buffer(
            &temporary,
            &pixels,
            RADAR_IMAGE_SIZE,
            RADAR_IMAGE_SIZE,
            image::ColorType::Rgba8,
        )
        .map_err(|error| format!("write {}: {error}", temporary.display()))?;
        fs::rename(&temporary, &output).map_err(|error| {
            format!("replace {} with generated radar: {error}", output.display())
        })?;
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_flip_makes_north_the_first_png_row() {
        let mut pixels = vec![
            1, 0, 0, 255, 2, 0, 0, 255, // south
            3, 0, 0, 255, 4, 0, 0, 255, // north
        ];
        flip_rgba_rows(&mut pixels, 2, 2);
        assert_eq!(&pixels[..8], &[3, 0, 0, 255, 4, 0, 0, 255]);
        assert_eq!(&pixels[8..], &[1, 0, 0, 255, 2, 0, 0, 255]);
    }
}
