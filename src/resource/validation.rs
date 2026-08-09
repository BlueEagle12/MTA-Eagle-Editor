use super::super::*;

#[derive(Default)]
pub(crate) struct ValidationSummary {
    pub(crate) referenced_dffs: BTreeSet<String>,
    pub(crate) referenced_cols: BTreeSet<String>,
    pub(crate) referenced_txds: BTreeSet<String>,
    pub(crate) img_dffs: BTreeSet<String>,
    pub(crate) img_cols: BTreeSet<String>,
    pub(crate) img_txds: BTreeSet<String>,
    pub(crate) loose_dffs: BTreeSet<String>,
    pub(crate) loose_cols: BTreeSet<String>,
    pub(crate) loose_txds: BTreeSet<String>,
    pub(crate) missing_dffs: Vec<String>,
    pub(crate) missing_cols: Vec<String>,
    pub(crate) missing_txds: Vec<String>,
    pub(crate) unused_dffs: Vec<String>,
    pub(crate) unused_cols: Vec<String>,
    pub(crate) unused_txds: Vec<String>,
    pub(crate) unused_definitions: Vec<String>,
    pub(crate) missing_definition_ids: Vec<String>,
    pub(crate) duplicate_dffs: Vec<String>,
    pub(crate) missing_col_attrs: Vec<String>,
    pub(crate) invalid_texture_formats: Vec<String>,
    pub(crate) invalid_col_loads: Vec<String>,
    pub(crate) breakable_warnings: Vec<String>,
}

impl ValidationSummary {
    pub(crate) fn issue_count(&self) -> usize {
        self.missing_dffs.len()
            + self.missing_cols.len()
            + self.missing_txds.len()
            + self.unused_dffs.len()
            + self.unused_cols.len()
            + self.unused_txds.len()
            + self.unused_definitions.len()
            + self.missing_definition_ids.len()
            + self.duplicate_dffs.len()
            + self.missing_col_attrs.len()
            + self.invalid_texture_formats.len()
            + self.invalid_col_loads.len()
            + self.breakable_warnings.len()
    }
}

pub(crate) fn asset_key(value: &str, ext: &str) -> String {
    lower(with_ext(value.trim(), ext))
}

pub(crate) fn asset_key_opt(value: Option<&String>, fallback: &str, ext: &str) -> String {
    value
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| asset_key(value, ext))
        .unwrap_or_else(|| asset_key(fallback, ext))
}

pub(crate) fn definition_txd_name<'a>(
    definitions: &'a HashMap<String, Definition>,
    id: &str,
) -> Option<&'a str> {
    definitions.get(id).and_then(definition_txd_name_from_attrs)
}

pub(crate) fn definition_txd_name_from_attrs(def: &Definition) -> Option<&str> {
    def.attrs
        .get("txd")
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(crate) fn mesh_key_from_dff_txd(dff_name: &str, txd_name: Option<&str>) -> String {
    let dff_key = asset_key(dff_name, ".dff");
    let txd_key = txd_name
        .map(|txd| asset_key(txd, ".txd"))
        .unwrap_or_default();
    format!("{dff_key}|{txd_key}")
}

pub(crate) fn placement_mesh_key(
    placement: &Placement,
    definitions: &HashMap<String, Definition>,
) -> String {
    mesh_key_from_dff_txd(
        &placement.dff,
        definition_txd_name(definitions, &placement.id),
    )
}

pub(crate) fn build_referenced_dff_queue(
    placements: &[Placement],
    definitions: &HashMap<String, Definition>,
    dff_map: &HashMap<String, ImgEntry>,
) -> Vec<(String, ImgEntry, Option<String>)> {
    let mut queued = HashSet::<String>::new();
    let mut queue = Vec::new();
    for placement in placements {
        let dff_key = asset_key(&placement.dff, ".dff");
        let Some(entry) = dff_map.get(&dff_key) else {
            continue;
        };
        let txd_name = definition_txd_name(definitions, &placement.id).map(ToOwned::to_owned);
        let mesh_key = mesh_key_from_dff_txd(&dff_key, txd_name.as_deref());
        if queued.insert(mesh_key.clone()) {
            queue.push((mesh_key, entry.clone(), txd_name));
        }
    }
    queue
}

pub(crate) fn texture_override_map_for_definition(
    app: &AppState,
    definition_id: &str,
) -> HashMap<String, String> {
    app.texture_overrides
        .iter()
        .filter_map(|((id, texture_name), txd_name)| {
            (id == definition_id).then(|| (texture_name.clone(), txd_name.clone()))
        })
        .collect()
}

pub(crate) fn find_dff_entry_for_app(app: &AppState, dff_name: &str) -> Option<ImgEntry> {
    let target = asset_key(dff_name, ".dff");
    // Staged DFF rewrites are the editor's current source of truth. In
    // particular, asset optimization can remove invalid/duplicate faces and
    // then asks this lookup to recompile the live mesh before Save promotes
    // the replacement. Reading the base IMG first leaves the live mesh on the
    // old topology and makes subsequent vertex-light writeback fail.
    if let Some(entry) = replacement_img_entry(&wip_root_path(&app.root), &target) {
        return Some(entry);
    }
    if let Some(entry) = replacement_img_entry(&app.root, &target) {
        return Some(entry);
    }
    for path in collect_resource_img_files(&app.root)
        .into_iter()
        .chain(gta_sa_img_files(&app.gta_sa_dir))
    {
        for entry in parse_img(&path) {
            if lower(&entry.name) == target {
                return Some(entry);
            }
        }
    }
    None
}

pub(crate) fn find_txd_entry_for_app(app: &AppState, txd_name: &str) -> Option<ImgEntry> {
    let target = asset_key(txd_name, ".txd");
    if let Some(entry) = replacement_img_entry(&wip_root_path(&app.root), &target) {
        return Some(entry);
    }
    if let Some(entry) = replacement_img_entry(&app.root, &target) {
        return Some(entry);
    }
    for path in collect_resource_txd_files(&app.root) {
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| asset_key(name, ".txd") == target)
        {
            return loose_txd_entry(&path);
        }
    }
    for path in collect_resource_img_files(&app.root)
        .into_iter()
        .chain(gta_sa_img_files(&app.gta_sa_dir))
    {
        for entry in parse_img(&path) {
            if lower(&entry.name) == target {
                return Some(entry);
            }
        }
    }
    None
}

pub(crate) fn read_exact_range(path: &Path, offset: u64, size: usize) -> Result<Vec<u8>, String> {
    let mut file = fs::File::open(path).map_err(|err| format!("{}: {err}", path.display()))?;
    file.seek(SeekFrom::Start(offset))
        .map_err(|err| format!("{} @ {offset}: {err}", path.display()))?;
    let mut out = vec![0; size];
    file.read_exact(&mut out)
        .map_err(|err| format!("{} @ {offset}: {err}", path.display()))?;
    Ok(out)
}

pub(crate) fn read_txd_entry_bytes(entry: &ImgEntry) -> Vec<u8> {
    let mut bytes = read_img_entry(entry);
    let len = txd_chunk_len(&bytes);
    bytes.truncate(len);
    bytes
}

pub(crate) fn read_col_entry_bytes(entry: &ImgEntry) -> Vec<u8> {
    let mut bytes = read_img_entry(entry);
    let len = col_chunk_len(&bytes);
    bytes.truncate(len);
    bytes
}

#[derive(Clone)]
pub(crate) struct TxdNativeHeader {
    name: String,
    width: u16,
    height: u16,
    format: Option<TxFormat>,
    has_alpha: bool,
    raster_format: u32,
    depth: u8,
    compression: u8,
    raster_format_offset: usize,
    depth_offset: usize,
    compression_offset: usize,
    mip_count: u8,
    mip_levels: Vec<TxdMipLevel>,
    data_offset: usize,
    data_size: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct TxdMipLevel {
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) data_offset: usize,
    pub(crate) data_size: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TxdOptimizationProfile {
    /// Resize textures whose largest dimension exceeds this limit. The
    /// effective limit is rounded down to a power of two.
    pub(crate) max_texture_dimension: Option<u16>,
    /// Re-encode opaque textures as BC1/DXT1 and alpha textures as BC3/DXT5.
    pub(crate) recompress_to_bc: bool,
    /// Generate every mip level down to 1x1. Callers enable this for streamed
    /// world textures; UI and sprite dictionaries can leave existing mips as-is.
    pub(crate) generate_mipmaps: bool,
}

impl TxdOptimizationProfile {
    pub(crate) const fn lossless() -> Self {
        Self {
            max_texture_dimension: None,
            recompress_to_bc: false,
            generate_mipmaps: false,
        }
    }

    pub(crate) const fn balanced(max_texture_dimension: u16, generate_mipmaps: bool) -> Self {
        Self {
            max_texture_dimension: Some(max_texture_dimension),
            recompress_to_bc: true,
            generate_mipmaps,
        }
    }
}

impl Default for TxdOptimizationProfile {
    fn default() -> Self {
        Self::lossless()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TxdTextureOptimizationReport {
    pub(crate) texture_name: String,
    pub(crate) old_dimensions: (u16, u16),
    pub(crate) new_dimensions: (u16, u16),
    pub(crate) old_format: String,
    pub(crate) new_format: String,
    pub(crate) old_mip_count: u8,
    pub(crate) new_mip_count: u8,
    pub(crate) old_native_bytes: usize,
    pub(crate) new_native_bytes: usize,
    pub(crate) bytes_saved: i64,
    pub(crate) details: Vec<String>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TxdOptimizationResult {
    pub(crate) bytes: Vec<u8>,
    pub(crate) textures: Vec<TxdTextureOptimizationReport>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct TxdTextureContent {
    pub(crate) name: String,
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) sampler: u32,
    /// RenderWare's native-raster alpha flag is part of appearance, even when
    /// the decoded pixels are identical. DRIV3R commonly stores specular masks
    /// in an opaque texture's alpha channel.
    pub(crate) has_alpha: bool,
    pub(crate) rgba_len: usize,
    pub(crate) fingerprint: [u64; 2],
    pub(crate) native: Vec<u8>,
}

fn texture_content_fingerprint(bytes: &[u8]) -> [u64; 2] {
    // Two independent deterministic 64-bit streams keep comparison compact
    // without relying on randomized HashMap state. Consolidation re-parses both
    // dictionaries immediately before applying a merge.
    let mut a = 0xcbf2_9ce4_8422_2325u64;
    let mut b = 0x9e37_79b9_7f4a_7c15u64;
    for (idx, value) in bytes.iter().copied().enumerate() {
        a ^= value as u64;
        a = a.wrapping_mul(0x1000_0000_01b3);
        b ^= (value as u64).wrapping_add((idx as u64).rotate_left(17));
        b = b.rotate_left(9).wrapping_mul(0x9ddf_ea08_eb38_2d69);
    }
    [a, b]
}

pub(crate) fn parse_txd_texture_contents(
    txd_bytes: &[u8],
) -> Result<Vec<TxdTextureContent>, String> {
    parse_txd_texture_contents_impl(txd_bytes, false)
}

pub(crate) fn parse_txd_texture_contents_with_natives(
    txd_bytes: &[u8],
) -> Result<Vec<TxdTextureContent>, String> {
    parse_txd_texture_contents_impl(txd_bytes, true)
}

fn parse_txd_texture_contents_impl(
    txd_bytes: &[u8],
    include_natives: bool,
) -> Result<Vec<TxdTextureContent>, String> {
    if txd_bytes.len() < 12 || rd32(txd_bytes, 0) != 0x16 {
        return Err("TXD is not a RenderWare texture dictionary".to_string());
    }
    let dict_start = 12usize;
    let dict_end = dict_start.saturating_add(rd32(txd_bytes, 4) as usize);
    if dict_end > txd_bytes.len() {
        return Err("TXD dictionary size is invalid".to_string());
    }
    let mut out = Vec::new();
    let mut names = HashSet::new();
    let mut chunk = dict_start;
    while chunk + 12 <= dict_end {
        let size = rd32(txd_bytes, chunk + 4) as usize;
        let end = chunk.saturating_add(12).saturating_add(size);
        if end > dict_end {
            return Err("TXD child chunk is invalid".to_string());
        }
        if rd32(txd_bytes, chunk) == 0x15 {
            let header = parse_txd_native_header(txd_bytes, chunk, end)?;
            if !names.insert(header.name.clone()) {
                return Err(format!(
                    "{}: duplicate texture name prevents safe consolidation",
                    header.name
                ));
            }
            let rgba = txd_texture_rgba(txd_bytes, &header).ok_or_else(|| {
                format!(
                    "{}: texture format cannot be decoded for safe comparison",
                    header.name
                )
            })?;
            // The second word in the texture-native struct contains filtering
            // and U/V addressing. Treat it as part of texture identity so a
            // consolidation cannot subtly change material sampling behavior.
            let sampler = rd32(txd_bytes, chunk + 12 + 12 + 4);
            out.push(TxdTextureContent {
                name: header.name,
                width: header.width,
                height: header.height,
                sampler,
                has_alpha: header.has_alpha,
                rgba_len: rgba.len(),
                fingerprint: texture_content_fingerprint(&rgba),
                native: if include_natives {
                    txd_bytes[chunk..end].to_vec()
                } else {
                    Vec::new()
                },
            });
        }
        chunk = end;
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

pub(crate) fn expected_txd_data_size(format: TxFormat, width: u16, height: u16) -> usize {
    let w = width as usize;
    let h = height as usize;
    match format {
        TxFormat::Dxt1 => w.div_ceil(4) * h.div_ceil(4) * 8,
        TxFormat::Dxt3 | TxFormat::Dxt5 => w.div_ceil(4) * h.div_ceil(4) * 16,
        TxFormat::Bgra8888 | TxFormat::Bgr888 => w * h * 4,
        TxFormat::Rgb565 | TxFormat::Argb1555 | TxFormat::Argb4444 => w * h * 2,
        TxFormat::Pal8 => w * h,
    }
}

pub(crate) fn parse_txd_native_header(
    bytes: &[u8],
    native_start: usize,
    native_end: usize,
) -> Result<TxdNativeHeader, String> {
    let mut chunk = native_start + 12;
    while chunk + 12 <= native_end {
        let cid = rd32(bytes, chunk);
        let csz = rd32(bytes, chunk + 4) as usize;
        let cs = chunk + 12;
        let ce = cs.saturating_add(csz);
        if ce > native_end {
            return Err("child chunk exceeds native size".to_string());
        }
        if cid != 0x01 {
            chunk = ce;
            continue;
        }
        if cs + 92 > ce {
            return Err("texture struct is too small".to_string());
        }
        let name_bytes = &bytes[cs + 8..cs + 40];
        let name_end = name_bytes.iter().position(|b| *b == 0).unwrap_or(32);
        let name = lower(&String::from_utf8_lossy(&name_bytes[..name_end]));
        let raster_format = rd32(bytes, cs + 72);
        let d3d_format = rd32(bytes, cs + 76);
        let fourcc = &bytes[cs + 76..cs + 80];
        let width = rd16(bytes, cs + 80);
        let height = rd16(bytes, cs + 82);
        let depth = bytes[cs + 84];
        let compression = bytes[cs + 87];
        let has_alpha = compression & 1 != 0;
        if name.is_empty() {
            return Err("texture has no name".to_string());
        }
        if width == 0 || height == 0 {
            return Err(format!("{name}: zero-size texture"));
        }

        let is_pal8 = raster_format & 0x2000 != 0;
        let is_pal4 = raster_format & 0x4000 != 0;
        let mut data_size_offset = cs + 88;
        if is_pal8 {
            data_size_offset += 256 * 4;
        } else if is_pal4 {
            return Err(format!("{name}: PAL4 textures are not supported"));
        }
        if data_size_offset + 4 > ce {
            return Err(format!("{name}: missing raster data size"));
        }
        let mip_count = bytes[cs + 85];
        if mip_count == 0 {
            return Err(format!("{name}: texture has zero mip levels"));
        }
        let mut mip_levels = Vec::with_capacity(mip_count as usize);
        let mut level_size_offset = data_size_offset;
        let mut level_width = width;
        let mut level_height = height;
        for level in 0..mip_count {
            if level_size_offset + 4 > ce {
                return Err(format!("{name}: mip {level} has no raster data size"));
            }
            let data_size = rd32(bytes, level_size_offset) as usize;
            let data_offset = level_size_offset + 4;
            if data_offset.saturating_add(data_size) > ce {
                return Err(format!(
                    "{name}: mip {level} raster data exceeds texture native"
                ));
            }
            mip_levels.push(TxdMipLevel {
                width: level_width,
                height: level_height,
                data_offset,
                data_size,
            });
            level_size_offset = data_offset + data_size;
            level_width = (level_width / 2).max(1);
            level_height = (level_height / 2).max(1);
        }
        let data_offset = mip_levels[0].data_offset;
        let data_size = mip_levels[0].data_size;

        let format = if fourcc == b"DXT1" {
            Some(TxFormat::Dxt1)
        } else if fourcc == b"DXT3" {
            Some(TxFormat::Dxt3)
        } else if fourcc == b"DXT5" {
            Some(TxFormat::Dxt5)
        } else if is_pal8 {
            Some(TxFormat::Pal8)
        } else if d3d_format == 21 {
            Some(TxFormat::Bgra8888)
        } else if d3d_format == 22 {
            Some(TxFormat::Bgr888)
        } else if d3d_format == 23 {
            Some(TxFormat::Rgb565)
        } else if d3d_format == 25 {
            Some(TxFormat::Argb1555)
        } else if d3d_format == 26 {
            Some(TxFormat::Argb4444)
        } else if d3d_format != 0 {
            None
        } else if depth == 32 {
            if raster_format & 0xF00 == 0x600 {
                Some(TxFormat::Bgr888)
            } else {
                Some(TxFormat::Bgra8888)
            }
        } else if depth == 16 {
            match raster_format & 0xF00 {
                0x200 => Some(TxFormat::Rgb565),
                0x300 => Some(TxFormat::Argb4444),
                _ => Some(TxFormat::Argb1555),
            }
        } else {
            None
        };

        return Ok(TxdNativeHeader {
            name,
            width,
            height,
            format,
            has_alpha,
            raster_format,
            depth,
            compression,
            raster_format_offset: cs + 72,
            depth_offset: cs + 84,
            compression_offset: cs + 87,
            mip_count,
            mip_levels,
            data_offset,
            data_size,
        });
    }
    Err("texture native has no struct chunk".to_string())
}

pub(crate) fn txd_texture_rgba(bytes: &[u8], header: &TxdNativeHeader) -> Option<Vec<u8>> {
    let format = header.format?;
    let width = header.width as usize;
    let height = header.height as usize;
    let data = bytes.get(header.data_offset..header.data_offset + header.data_size)?;
    match format {
        TxFormat::Dxt1 => Some(decode_dxt(data, width, height, 1)),
        TxFormat::Dxt3 => Some(decode_dxt(data, width, height, 3)),
        TxFormat::Dxt5 => Some(decode_dxt(data, width, height, 5)),
        TxFormat::Bgra8888 | TxFormat::Bgr888 => {
            let expected = width.checked_mul(height)?.checked_mul(4)?;
            if data.len() < expected {
                return None;
            }
            let mut rgba = vec![0u8; expected];
            let force_opaque = matches!(format, TxFormat::Bgr888);
            for i in 0..(width * height) {
                rgba[i * 4] = data[i * 4 + 2];
                rgba[i * 4 + 1] = data[i * 4 + 1];
                rgba[i * 4 + 2] = data[i * 4];
                rgba[i * 4 + 3] = if force_opaque { 255 } else { data[i * 4 + 3] };
            }
            Some(rgba)
        }
        _ => None,
    }
}

pub(crate) fn is_power_of_two_dimension(value: u16) -> bool {
    value != 0 && value.is_power_of_two()
}

pub(crate) fn next_power_of_two_dimension(value: u16) -> Option<u16> {
    let next = (value as u32).next_power_of_two();
    (next <= u16::MAX as u32).then_some(next as u16)
}

pub(crate) fn resize_rgba_to_power_of_two(
    rgba: &[u8],
    width: u16,
    height: u16,
) -> Result<(Vec<u8>, u16, u16), String> {
    let target_width = next_power_of_two_dimension(width)
        .ok_or_else(|| format!("Could not resize {width}x{height}: width exceeds u16"))?;
    let target_height = next_power_of_two_dimension(height)
        .ok_or_else(|| format!("Could not resize {width}x{height}: height exceeds u16"))?;
    if target_width == width && target_height == height {
        return Ok((rgba.to_vec(), width, height));
    }
    let image = image::RgbaImage::from_raw(width as u32, height as u32, rgba.to_vec())
        .ok_or_else(|| format!("Could not build RGBA image {width}x{height}"))?;
    let resized = image::imageops::resize(
        &image,
        target_width as u32,
        target_height as u32,
        image::imageops::FilterType::Triangle,
    );
    Ok((resized.into_raw(), target_width, target_height))
}

pub(crate) fn texture_native_from_rgba(
    rgba: &[u8],
    width: u16,
    height: u16,
    texture_name: &str,
) -> Vec<u8> {
    texture_native_from_rgba_with_mip_count(rgba, width, height, texture_name, 1)
}

fn full_mip_count(width: u16, height: u16) -> u8 {
    let mut width = width;
    let mut height = height;
    let mut count = 1u8;
    while width > 1 || height > 1 {
        width = (width / 2).max(1);
        height = (height / 2).max(1);
        count = count.saturating_add(1);
    }
    count
}

fn texture_native_from_rgba_with_mip_count(
    rgba: &[u8],
    width: u16,
    height: u16,
    texture_name: &str,
    mip_count: u8,
) -> Vec<u8> {
    let width_u32 = width as u32;
    let height_u32 = height as u32;
    let has_alpha = rgba_has_alpha(rgba);
    let (d3d_format, mut raster_format, raster_depth, alpha_flag, bc_format) = if has_alpha {
        (*b"DXT5", 0x0300u32, 16u8, 9u8, texpresso::Format::Bc3)
    } else {
        (*b"DXT1", 0x0200u32, 16u8, 8u8, texpresso::Format::Bc1)
    };
    let mip_count = mip_count.clamp(1, full_mip_count(width, height));
    if mip_count > 1 {
        raster_format |= 0x8000;
    }

    let mut data = Vec::new();
    data.extend_from_slice(&9u32.to_le_bytes());
    data.extend_from_slice(&0x1101u32.to_le_bytes());
    let mut raw_name = [0u8; 32];
    let name = sanitize_texture_name(texture_name);
    let name_bytes = name.as_bytes();
    raw_name[..name_bytes.len().min(32)].copy_from_slice(&name_bytes[..name_bytes.len().min(32)]);
    data.extend_from_slice(&raw_name);
    data.extend_from_slice(&[0u8; 32]);
    data.extend_from_slice(&raster_format.to_le_bytes());
    data.extend_from_slice(&d3d_format);
    data.extend_from_slice(&width.to_le_bytes());
    data.extend_from_slice(&height.to_le_bytes());
    data.push(raster_depth);
    data.push(mip_count);
    data.push(4);
    data.push(alpha_flag);
    let mut level_rgba = rgba.to_vec();
    let mut level_width = width_u32;
    let mut level_height = height_u32;
    for level in 0..mip_count {
        let texels = encode_bc_texture(&level_rgba, level_width, level_height, bc_format);
        data.extend_from_slice(&(texels.len() as u32).to_le_bytes());
        data.extend_from_slice(&texels);
        if level + 1 < mip_count {
            let image = image::RgbaImage::from_raw(level_width, level_height, level_rgba)
                .expect("validated RGBA dimensions");
            level_width = (level_width / 2).max(1);
            level_height = (level_height / 2).max(1);
            level_rgba = image::imageops::resize(
                &image,
                level_width,
                level_height,
                image::imageops::FilterType::Triangle,
            )
            .into_raw();
        }
    }
    rw_chunk(0x15, rw_chunk(0x01, data))
}

pub(crate) fn texture_native_from_rgba_with_full_mips(
    rgba: &[u8],
    width: u16,
    height: u16,
    texture_name: &str,
) -> Vec<u8> {
    texture_native_from_rgba_with_mip_count(
        rgba,
        width,
        height,
        texture_name,
        full_mip_count(width, height),
    )
}

fn compressed_header_repair(format: TxFormat, has_alpha: bool) -> Option<(u32, u8, u8)> {
    let alpha_flag = if has_alpha { 9 } else { 8 };
    match format {
        TxFormat::Dxt1 => Some((0x0200, 16, alpha_flag)),
        TxFormat::Dxt3 | TxFormat::Dxt5 => Some((0x0300, 16, alpha_flag)),
        _ => None,
    }
}

fn compressed_header_needs_repair(header: &TxdNativeHeader) -> Option<(u32, u8, u8)> {
    let format = header.format?;
    // Compression format does not determine render transparency. DRIV3R
    // intentionally stores opaque specular/gloss masks in DXT3/5 alpha data,
    // so retain the native raster's authoritative alpha bit during cleanup.
    let (expected_raster, expected_depth, expected_compression) =
        compressed_header_repair(format, header.has_alpha)?;
    let raster_base = header.raster_format & 0x0F00;
    // Existing DXT1/1555 entries are valid cutout-alpha style textures. The
    // broken writer emitted compressed textures as 8888, which the game rejects.
    if matches!(format, TxFormat::Dxt1) && raster_base == 0x0100 {
        return None;
    }
    (raster_base != expected_raster
        || header.depth != expected_depth
        || header.compression != expected_compression)
        .then_some((expected_raster, expected_depth, expected_compression))
}

fn repair_native_header(
    native: &[u8],
    native_start: usize,
    header: &TxdNativeHeader,
    compressed_fix: Option<(u32, u8, u8)>,
    set_mipmapped_flag: bool,
) -> Vec<u8> {
    let mut out = native.to_vec();
    let raster_offset = header.raster_format_offset - native_start;
    let depth_offset = header.depth_offset - native_start;
    let compression_offset = header.compression_offset - native_start;
    let mut raster = compressed_fix
        .map(|(expected_raster, _, _)| (header.raster_format & !0x0F00) | expected_raster)
        .unwrap_or(header.raster_format);
    if set_mipmapped_flag {
        raster |= 0x8000;
    }
    out[raster_offset..raster_offset + 4].copy_from_slice(&raster.to_le_bytes());
    if let Some((_, expected_depth, expected_compression)) = compressed_fix {
        out[depth_offset] = expected_depth;
        out[compression_offset] = expected_compression;
    }
    out
}

pub(crate) fn optimize_txd_texture_formats(
    txd_bytes: Vec<u8>,
) -> Result<(Vec<u8>, Vec<String>, Vec<String>), String> {
    if txd_bytes.len() < 12 || rd32(&txd_bytes, 0) != 0x16 {
        return Err("TXD is not a RenderWare texture dictionary".to_string());
    }
    let dict_start = 12usize;
    let dict_end = dict_start.saturating_add(rd32(&txd_bytes, 4) as usize);
    if dict_end > txd_bytes.len() {
        return Err("TXD dictionary size is invalid".to_string());
    }

    let mut out = Vec::with_capacity(txd_bytes.len());
    out.extend_from_slice(&txd_bytes[..dict_start]);
    let mut chunk = dict_start;
    let mut fixes = Vec::new();
    let mut warnings = Vec::new();
    while chunk + 12 <= dict_end {
        let cid = rd32(&txd_bytes, chunk);
        let csz = rd32(&txd_bytes, chunk + 4) as usize;
        let ce = chunk + 12 + csz;
        if ce > dict_end {
            return Err("TXD child chunk is invalid".to_string());
        }
        if cid == 0x15 {
            match parse_txd_native_header(&txd_bytes, chunk, ce) {
                Ok(header) => {
                    let raw_format =
                        matches!(header.format, Some(TxFormat::Bgra8888 | TxFormat::Bgr888));
                    let npot_dimensions = !is_power_of_two_dimension(header.width)
                        || !is_power_of_two_dimension(header.height);
                    let compressed_header_fix = compressed_header_needs_repair(&header);
                    let missing_mipmapped_flag = header.format.is_some()
                        && header.mip_count > 1
                        && header.raster_format & 0x8000 == 0;
                    if raw_format || npot_dimensions {
                        if let Some(mut rgba) = txd_texture_rgba(&txd_bytes, &header) {
                            if !header.has_alpha {
                                for pixel in rgba.chunks_exact_mut(4) {
                                    pixel[3] = 255;
                                }
                            }
                            let (rgba, width, height) =
                                resize_rgba_to_power_of_two(&rgba, header.width, header.height)?;
                            let old_format =
                                header.format.map(tx_format_label).unwrap_or("unknown");
                            let new_format = if rgba_has_alpha(&rgba) {
                                "DXT5"
                            } else {
                                "DXT1"
                            };
                            fixes.push(format!(
                                "{}: {} {}x{} -> {} {}x{}",
                                header.name,
                                old_format,
                                header.width,
                                header.height,
                                new_format,
                                width,
                                height
                            ));
                            out.extend_from_slice(&texture_native_from_rgba(
                                &rgba,
                                width,
                                height,
                                &header.name,
                            ));
                        } else {
                            warnings.push(format!(
                                "{}: could not decode texture for optimization",
                                header.name
                            ));
                            out.extend_from_slice(&txd_bytes[chunk..ce]);
                        }
                    } else if compressed_header_fix.is_some() || missing_mipmapped_flag {
                        if let Some((expected_raster, _, _)) = compressed_header_fix {
                            let old_raster = header.raster_format & 0x0F00;
                            let format_label =
                                header.format.map(tx_format_label).unwrap_or("unknown");
                            fixes.push(format!(
                                "{}: {} header raster 0x{:03x} -> 0x{:03x}",
                                header.name, format_label, old_raster, expected_raster
                            ));
                        }
                        if missing_mipmapped_flag {
                            fixes.push(format!(
                                "{}: set mipmapped raster flag for {} mip levels",
                                header.name, header.mip_count
                            ));
                        }
                        out.extend_from_slice(&repair_native_header(
                            &txd_bytes[chunk..ce],
                            chunk,
                            &header,
                            compressed_header_fix,
                            missing_mipmapped_flag,
                        ));
                    } else if header.format.is_none() {
                        warnings.push(format!("{} has unsupported texture format", header.name));
                        out.extend_from_slice(&txd_bytes[chunk..ce]);
                    } else {
                        out.extend_from_slice(&txd_bytes[chunk..ce]);
                    }
                }
                Err(err) => {
                    warnings.push(err);
                    out.extend_from_slice(&txd_bytes[chunk..ce]);
                }
            }
        } else {
            out.extend_from_slice(&txd_bytes[chunk..ce]);
        }
        chunk = ce;
    }

    if fixes.is_empty() {
        return Ok((txd_bytes[..dict_end].to_vec(), fixes, warnings));
    }
    let new_size = out.len().saturating_sub(12);
    out[4..8].copy_from_slice(&(new_size as u32).to_le_bytes());
    Ok((out, fixes, warnings))
}

fn power_of_two_limit(limit: u16) -> u16 {
    if limit == 0 {
        return 1;
    }
    1u16 << (15 - limit.leading_zeros() as u16)
}

fn profiled_texture_dimensions(width: u16, height: u16, limit: Option<u16>) -> (u16, u16) {
    let Some(limit) = limit else {
        return (width, height);
    };
    if limit == 0 {
        return (width, height);
    }
    let limit = power_of_two_limit(limit);
    let mut width = width;
    let mut height = height;
    // Optimizable inputs have power-of-two dimensions after the lossless pass.
    // Halving both axes preserves their aspect ratio exactly and never upscales.
    while width.max(height) > limit {
        width = (width / 2).max(1);
        height = (height / 2).max(1);
    }
    (width, height)
}

fn mip_chain_is_complete_and_valid(header: &TxdNativeHeader) -> bool {
    let Some(format) = header.format else {
        return false;
    };
    header.mip_count == full_mip_count(header.width, header.height)
        && header.mip_levels.iter().all(|level| {
            level.data_size == expected_txd_data_size(format, level.width, level.height)
        })
}

#[derive(Clone)]
struct TxdNativeSnapshot {
    width: u16,
    height: u16,
    format: String,
    mip_count: u8,
    native: Vec<u8>,
}

fn txd_native_snapshots(txd_bytes: &[u8]) -> Result<BTreeMap<String, TxdNativeSnapshot>, String> {
    if txd_bytes.len() < 12 || rd32(txd_bytes, 0) != 0x16 {
        return Err("TXD is not a RenderWare texture dictionary".to_string());
    }
    let dict_start = 12usize;
    let dict_end = dict_start.saturating_add(rd32(txd_bytes, 4) as usize);
    if dict_end > txd_bytes.len() {
        return Err("TXD dictionary size is invalid".to_string());
    }
    let mut snapshots = BTreeMap::new();
    let mut chunk = dict_start;
    while chunk + 12 <= dict_end {
        let size = rd32(txd_bytes, chunk + 4) as usize;
        let end = chunk.saturating_add(12).saturating_add(size);
        if end > dict_end {
            return Err("TXD child chunk is invalid".to_string());
        }
        if rd32(txd_bytes, chunk) == 0x15 {
            if let Ok(header) = parse_txd_native_header(txd_bytes, chunk, end) {
                snapshots.insert(
                    header.name,
                    TxdNativeSnapshot {
                        width: header.width,
                        height: header.height,
                        format: header
                            .format
                            .map(tx_format_label)
                            .unwrap_or("unknown")
                            .to_string(),
                        mip_count: header.mip_count,
                        native: txd_bytes[chunk..end].to_vec(),
                    },
                );
            }
        }
        chunk = end;
    }
    Ok(snapshots)
}

/// Apply the existing safe cleanup first, then apply optional size/format/mip
/// policy. A default profile therefore has exactly the same byte behavior as
/// `optimize_txd_texture_formats`, including leaving valid DXT natives alone.
pub(crate) fn optimize_txd_texture_formats_with_profile(
    txd_bytes: Vec<u8>,
    profile: TxdOptimizationProfile,
) -> Result<TxdOptimizationResult, String> {
    let original = txd_native_snapshots(&txd_bytes)?;
    let (lossless_bytes, lossless_fixes, mut warnings) = optimize_txd_texture_formats(txd_bytes)?;

    let dict_start = 12usize;
    let dict_end = dict_start.saturating_add(rd32(&lossless_bytes, 4) as usize);
    let mut out = Vec::with_capacity(lossless_bytes.len());
    out.extend_from_slice(&lossless_bytes[..dict_start]);
    let mut details = BTreeMap::<String, Vec<String>>::new();
    for fix in lossless_fixes {
        let texture_name = fix.split(':').next().unwrap_or("").to_string();
        details.entry(texture_name).or_default().push(fix);
    }
    if let Some(limit) = profile.max_texture_dimension {
        if limit == 0 {
            warnings.push("Texture dimension limit 0 is invalid; ignoring the limit".to_string());
        } else if !limit.is_power_of_two() {
            warnings.push(format!(
                "Texture dimension limit {limit} is not a power of two; using {}",
                power_of_two_limit(limit)
            ));
        }
    }

    let mut chunk = dict_start;
    while chunk + 12 <= dict_end {
        let cid = rd32(&lossless_bytes, chunk);
        let csz = rd32(&lossless_bytes, chunk + 4) as usize;
        let ce = chunk.saturating_add(12).saturating_add(csz);
        if ce > dict_end {
            return Err("TXD child chunk is invalid".to_string());
        }
        if cid != 0x15 {
            out.extend_from_slice(&lossless_bytes[chunk..ce]);
            chunk = ce;
            continue;
        }

        let header = match parse_txd_native_header(&lossless_bytes, chunk, ce) {
            Ok(header) => header,
            Err(err) => {
                if !warnings.contains(&err) {
                    warnings.push(err);
                }
                out.extend_from_slice(&lossless_bytes[chunk..ce]);
                chunk = ce;
                continue;
            }
        };
        let target_dimensions =
            profiled_texture_dimensions(header.width, header.height, profile.max_texture_dimension);
        let dimensions_change = target_dimensions != (header.width, header.height);
        let mip_change = profile.generate_mipmaps && !mip_chain_is_complete_and_valid(&header);
        let may_need_format_change = profile.recompress_to_bc;
        if !dimensions_change && !mip_change && !may_need_format_change {
            out.extend_from_slice(&lossless_bytes[chunk..ce]);
            chunk = ce;
            continue;
        }

        let Some(mut rgba) = txd_texture_rgba(&lossless_bytes, &header) else {
            warnings.push(format!(
                "{}: could not decode texture for profiled optimization",
                header.name
            ));
            out.extend_from_slice(&lossless_bytes[chunk..ce]);
            chunk = ce;
            continue;
        };
        // Opaque native rasters can contain incidental BC1 alpha selector bits.
        // The RenderWare alpha flag is authoritative for their appearance.
        if !header.has_alpha {
            for pixel in rgba.chunks_exact_mut(4) {
                pixel[3] = 255;
            }
        }
        let desired_format = if rgba_has_alpha(&rgba) {
            TxFormat::Dxt5
        } else {
            TxFormat::Dxt1
        };
        let format_change = profile.recompress_to_bc && header.format != Some(desired_format);
        if !dimensions_change && !mip_change && !format_change {
            out.extend_from_slice(&lossless_bytes[chunk..ce]);
            chunk = ce;
            continue;
        }

        if dimensions_change {
            let image = image::RgbaImage::from_raw(header.width as u32, header.height as u32, rgba)
                .ok_or_else(|| format!("{}: invalid RGBA dimensions", header.name))?;
            rgba = image::imageops::resize(
                &image,
                target_dimensions.0 as u32,
                target_dimensions.1 as u32,
                image::imageops::FilterType::Triangle,
            )
            .into_raw();
            details
                .entry(header.name.clone())
                .or_default()
                .push(format!(
                    "{}: {}x{} -> {}x{} (max-size policy)",
                    header.name,
                    header.width,
                    header.height,
                    target_dimensions.0,
                    target_dimensions.1
                ));
        }
        if format_change {
            details
                .entry(header.name.clone())
                .or_default()
                .push(format!(
                    "{}: {} -> {} (BC format policy)",
                    header.name,
                    header.format.map(tx_format_label).unwrap_or("unknown"),
                    tx_format_label(desired_format)
                ));
        }
        if mip_change {
            details
                .entry(header.name.clone())
                .or_default()
                .push(format!(
                    "{}: {} -> {} mip levels",
                    header.name,
                    header.mip_count,
                    full_mip_count(target_dimensions.0, target_dimensions.1)
                ));
        }
        let target_mip_count = if profile.generate_mipmaps {
            full_mip_count(target_dimensions.0, target_dimensions.1)
        } else {
            header
                .mip_count
                .min(full_mip_count(target_dimensions.0, target_dimensions.1))
        };
        out.extend_from_slice(&texture_native_from_rgba_with_mip_count(
            &rgba,
            target_dimensions.0,
            target_dimensions.1,
            &header.name,
            target_mip_count,
        ));
        chunk = ce;
    }

    let new_size = out.len().saturating_sub(12);
    out[4..8].copy_from_slice(&(new_size as u32).to_le_bytes());
    let optimized = txd_native_snapshots(&out)?;
    let mut textures = Vec::new();
    for (name, new) in optimized {
        let Some(old) = original.get(&name) else {
            continue;
        };
        if old.native == new.native {
            continue;
        }
        let old_native_bytes = old.native.len();
        let new_native_bytes = new.native.len();
        textures.push(TxdTextureOptimizationReport {
            texture_name: name.clone(),
            old_dimensions: (old.width, old.height),
            new_dimensions: (new.width, new.height),
            old_format: old.format.clone(),
            new_format: new.format,
            old_mip_count: old.mip_count,
            new_mip_count: new.mip_count,
            old_native_bytes,
            new_native_bytes,
            bytes_saved: old_native_bytes as i64 - new_native_bytes as i64,
            details: details.remove(&name).unwrap_or_default(),
            warnings: warnings
                .iter()
                .filter(|warning| warning.starts_with(&format!("{name}:")))
                .cloned()
                .collect(),
        });
    }
    Ok(TxdOptimizationResult {
        bytes: out,
        textures,
        warnings,
    })
}

pub(crate) fn validate_txd_texture_formats(txd_name: &str, txd_bytes: &[u8]) -> Vec<String> {
    let mut warnings = Vec::new();
    if txd_bytes.len() < 12 || rd32(txd_bytes, 0) != 0x16 {
        return vec![format!("{txd_name}: not a TXD dictionary")];
    }
    let dict_start = 12usize;
    let dict_end = dict_start.saturating_add(rd32(txd_bytes, 4) as usize);
    if dict_end > txd_bytes.len() {
        return vec![format!("{txd_name}: invalid dictionary size")];
    }
    let mut chunk = dict_start;
    while chunk + 12 <= dict_end {
        let cid = rd32(txd_bytes, chunk);
        let csz = rd32(txd_bytes, chunk + 4) as usize;
        let ce = chunk + 12 + csz;
        if ce > dict_end {
            warnings.push(format!("{txd_name}: child chunk exceeds dictionary"));
            break;
        }
        if cid == 0x15 {
            match parse_txd_native_header(txd_bytes, chunk, ce) {
                Ok(header) => {
                    if !is_power_of_two_dimension(header.width)
                        || !is_power_of_two_dimension(header.height)
                    {
                        let fixed_width =
                            next_power_of_two_dimension(header.width).unwrap_or(header.width);
                        let fixed_height =
                            next_power_of_two_dimension(header.height).unwrap_or(header.height);
                        warnings.push(format!(
                            "{txd_name}: {} is {}x{}, expected power-of-two size (Optimize -> {}x{})",
                            header.name,
                            header.width,
                            header.height,
                            fixed_width,
                            fixed_height
                        ));
                    }
                    if let Some(format) = header.format {
                        if let Some((expected_raster, expected_depth, expected_compression)) =
                            compressed_header_needs_repair(&header)
                        {
                            warnings.push(format!(
                                "{txd_name}: {} {} has invalid compressed header (raster 0x{:03x}, depth {}, flag {}; TXD Cleanup -> raster 0x{:03x}, depth {}, flag {})",
                                header.name,
                                tx_format_label(format),
                                header.raster_format & 0x0F00,
                                header.depth,
                                header.compression,
                                expected_raster,
                                expected_depth,
                                expected_compression
                            ));
                        }
                        let maximum_mips = full_mip_count(header.width, header.height);
                        if header.mip_count > maximum_mips {
                            warnings.push(format!(
                                "{txd_name}: {} declares {} mip levels, maximum is {}",
                                header.name, header.mip_count, maximum_mips
                            ));
                        }
                        if header.mip_count > 1 && header.raster_format & 0x8000 == 0 {
                            warnings.push(format!(
                                "{txd_name}: {} has {} mip levels without the mipmapped raster flag",
                                header.name, header.mip_count
                            ));
                        }
                        for (level, mip) in header.mip_levels.iter().enumerate() {
                            let expected = expected_txd_data_size(format, mip.width, mip.height);
                            if mip.data_size != expected {
                                warnings.push(format!(
                                    "{txd_name}: {} {} mip {} ({}x{}) has {} bytes, expected {}",
                                    header.name,
                                    tx_format_label(format),
                                    level,
                                    mip.width,
                                    mip.height,
                                    mip.data_size,
                                    expected
                                ));
                            }
                        }
                    } else {
                        warnings.push(format!(
                            "{txd_name}: {} has unsupported texture format",
                            header.name
                        ));
                    }
                }
                Err(err) => warnings.push(format!("{txd_name}: {err}")),
            }
        }
        chunk = ce;
    }
    warnings
}

pub(crate) fn txd_contains_texture_native(bytes: &[u8], texture_name: &str) -> bool {
    let target = lower(texture_name);
    if bytes.len() < 12 || rd32(bytes, 0) != 0x16 {
        return false;
    }
    let dict_start = 12usize;
    let dict_end = (dict_start + rd32(bytes, 4) as usize).min(bytes.len());
    let mut chunk = dict_start;
    while chunk + 12 <= dict_end {
        let cid = rd32(bytes, chunk);
        let csz = rd32(bytes, chunk + 4) as usize;
        let cs = chunk + 12;
        let ce = cs.saturating_add(csz);
        if ce > dict_end {
            break;
        }
        if cid == 0x15 {
            let mut o = cs;
            while o + 12 <= ce {
                let child_id = rd32(bytes, o);
                let child_size = rd32(bytes, o + 4) as usize;
                let child_start = o + 12;
                let child_end = child_start.saturating_add(child_size);
                if child_end > ce {
                    break;
                }
                if child_id == 0x01 && child_start + 72 <= child_end {
                    let name_start = child_start + 8;
                    let name = &bytes[name_start..name_start + 32];
                    let end = name.iter().position(|value| *value == 0).unwrap_or(32);
                    if lower(&String::from_utf8_lossy(&name[..end])) == target {
                        return true;
                    }
                    break;
                }
                o = child_end;
            }
        }
        chunk = ce;
    }
    false
}

pub(crate) fn texture_native_name(
    bytes: &[u8],
    native_start: usize,
    native_end: usize,
) -> Option<String> {
    if native_start + 12 > native_end || rd32(bytes, native_start) != 0x15 {
        return None;
    }
    let mut o = native_start + 12;
    while o + 12 <= native_end {
        let child_id = rd32(bytes, o);
        let child_size = rd32(bytes, o + 4) as usize;
        let child_start = o + 12;
        let child_end = child_start.saturating_add(child_size);
        if child_end > native_end {
            break;
        }
        if child_id == 0x01 && child_start + 72 <= child_end {
            let name_start = child_start + 8;
            let name = &bytes[name_start..name_start + 32];
            let end = name.iter().position(|value| *value == 0).unwrap_or(32);
            return Some(lower(&String::from_utf8_lossy(&name[..end])));
        }
        o = child_end;
    }
    None
}

pub(crate) fn rename_texture_native(
    texture_native: &[u8],
    texture_name: &str,
) -> Result<Vec<u8>, String> {
    let name = sanitize_texture_name(texture_name);
    rename_texture_native_exact(texture_native, &name)
}

pub(crate) fn rename_texture_native_exact(
    texture_native: &[u8],
    texture_name: &str,
) -> Result<Vec<u8>, String> {
    if texture_native.len() < 12 || rd32(texture_native, 0) != 0x15 {
        return Err("Source texture is not a RenderWare texture-native chunk".to_string());
    }
    let texture_name = texture_name.trim();
    if texture_name.is_empty() || texture_name.as_bytes().contains(&0) {
        return Err("Texture name cannot be empty or contain a NUL byte".to_string());
    }
    let native_end = texture_native.len();
    let mut out = texture_native.to_vec();
    let mut o = 12usize;
    while o + 12 <= native_end {
        let child_id = rd32(&out, o);
        let child_size = rd32(&out, o + 4) as usize;
        let child_start = o + 12;
        let child_end = child_start.saturating_add(child_size);
        if child_end > native_end {
            break;
        }
        if child_id == 0x01 && child_start + 72 <= child_end {
            let name_start = child_start + 8;
            let mut raw_name = [0u8; 32];
            let name_bytes = texture_name.as_bytes();
            let count = name_bytes.len().min(32);
            raw_name[..count].copy_from_slice(&name_bytes[..count]);
            out[name_start..name_start + 32].copy_from_slice(&raw_name);
            return Ok(out);
        }
        o = child_end;
    }
    Err("Source texture native has no readable texture struct".to_string())
}

pub(crate) fn write_txd_texture_count(
    txd_bytes: &mut [u8],
    dict_start: usize,
    dict_end: usize,
    count: u16,
) {
    let mut chunk = dict_start;
    while chunk + 12 <= dict_end {
        let cid = rd32(txd_bytes, chunk);
        let csz = rd32(txd_bytes, chunk + 4) as usize;
        let cs = chunk + 12;
        let ce = cs.saturating_add(csz);
        if ce > dict_end {
            return;
        }
        if cid == 0x01 && csz >= 2 {
            txd_bytes[cs..cs + 2].copy_from_slice(&count.to_le_bytes());
            return;
        }
        chunk = ce;
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn txd_texture_count(bytes: &[u8], dict_start: usize, dict_end: usize) -> Option<u16> {
    let mut chunk = dict_start;
    while chunk + 12 <= dict_end {
        let cid = rd32(bytes, chunk);
        let csz = rd32(bytes, chunk + 4) as usize;
        let cs = chunk + 12;
        let ce = cs.saturating_add(csz);
        if ce > dict_end {
            return None;
        }
        if cid == 0x01 && csz >= 2 {
            return Some(rd16(bytes, cs));
        }
        chunk = ce;
    }
    None
}

pub(crate) fn append_texture_native_to_txd(
    mut txd_bytes: Vec<u8>,
    texture_native: &[u8],
    texture_name: &str,
) -> Result<Vec<u8>, String> {
    if txd_bytes.len() < 12 || rd32(&txd_bytes, 0) != 0x16 {
        return Err("Destination TXD is not a RenderWare texture dictionary".to_string());
    }
    if texture_native.len() < 12 || rd32(texture_native, 0) != 0x15 {
        return Err("Source texture is not a RenderWare texture-native chunk".to_string());
    }
    let dict_start = 12usize;
    let dict_size = rd32(&txd_bytes, 4) as usize;
    let dict_end = dict_start.saturating_add(dict_size);
    if dict_end > txd_bytes.len() {
        return Err("Destination TXD dictionary size is invalid".to_string());
    }
    if txd_contains_texture_native(&txd_bytes, texture_name) {
        return Ok(txd_bytes[..dict_end].to_vec());
    }
    let texture_native = rename_texture_native(texture_native, texture_name)?;

    let mut struct_payload = None;
    let mut chunk = dict_start;
    while chunk + 12 <= dict_end {
        let cid = rd32(&txd_bytes, chunk);
        let csz = rd32(&txd_bytes, chunk + 4) as usize;
        let cs = chunk + 12;
        let ce = cs.saturating_add(csz);
        if ce > dict_end {
            break;
        }
        if cid == 0x01 && csz >= 2 {
            struct_payload = Some(cs);
            break;
        }
        chunk = ce;
    }
    let Some(count_offset) = struct_payload else {
        return Err("Destination TXD has no texture dictionary struct".to_string());
    };

    let count = rd16(&txd_bytes, count_offset).saturating_add(1);
    txd_bytes[count_offset..count_offset + 2].copy_from_slice(&count.to_le_bytes());
    let new_size = dict_size.saturating_add(texture_native.len());
    txd_bytes[4..8].copy_from_slice(&(new_size as u32).to_le_bytes());
    txd_bytes.truncate(dict_end);
    txd_bytes.extend_from_slice(&texture_native);
    Ok(txd_bytes)
}

pub(crate) fn replace_or_append_texture_native_in_txd(
    txd_bytes: Vec<u8>,
    texture_native: &[u8],
    texture_name: &str,
) -> Result<Vec<u8>, String> {
    let texture_native = rename_texture_native(texture_native, texture_name)?;
    replace_or_append_texture_native_in_txd_impl(txd_bytes, &texture_native, texture_name)
}

pub(crate) fn replace_or_append_texture_native_in_txd_exact(
    txd_bytes: Vec<u8>,
    texture_native: &[u8],
    texture_name: &str,
) -> Result<Vec<u8>, String> {
    let texture_native = rename_texture_native_exact(texture_native, texture_name)?;
    replace_or_append_texture_native_in_txd_impl(txd_bytes, &texture_native, texture_name)
}

fn replace_or_append_texture_native_in_txd_impl(
    txd_bytes: Vec<u8>,
    texture_native: &[u8],
    texture_name: &str,
) -> Result<Vec<u8>, String> {
    if txd_bytes.len() < 12 || rd32(&txd_bytes, 0) != 0x16 {
        return Err("Destination TXD is not a RenderWare texture dictionary".to_string());
    }
    if texture_native.len() < 12 || rd32(texture_native, 0) != 0x15 {
        return Err("Source texture is not a RenderWare texture-native chunk".to_string());
    }
    let dict_start = 12usize;
    let dict_size = rd32(&txd_bytes, 4) as usize;
    let dict_end = dict_start.saturating_add(dict_size);
    if dict_end > txd_bytes.len() {
        return Err("Destination TXD dictionary size is invalid".to_string());
    }
    let target = lower(texture_name);
    let mut out = Vec::with_capacity(txd_bytes.len() + texture_native.len());
    out.extend_from_slice(&txd_bytes[..dict_start]);
    let mut chunk = dict_start;
    let mut replaced = false;
    let mut native_count = 0u16;
    while chunk + 12 <= dict_end {
        let cid = rd32(&txd_bytes, chunk);
        let csz = rd32(&txd_bytes, chunk + 4) as usize;
        let ce = chunk + 12 + csz;
        if ce > dict_end {
            return Err("Destination TXD child chunk is invalid".to_string());
        }
        if cid == 0x15 {
            if texture_native_name(&txd_bytes, chunk, ce).is_some_and(|name| name == target) {
                if !replaced {
                    out.extend_from_slice(&texture_native);
                    native_count = native_count.saturating_add(1);
                    replaced = true;
                }
            } else {
                out.extend_from_slice(&txd_bytes[chunk..ce]);
                native_count = native_count.saturating_add(1);
            }
        } else {
            out.extend_from_slice(&txd_bytes[chunk..ce]);
        }
        chunk = ce;
    }
    if !replaced {
        out.extend_from_slice(&texture_native);
        native_count = native_count.saturating_add(1);
    }
    let new_size = out.len().saturating_sub(12);
    out[4..8].copy_from_slice(&(new_size as u32).to_le_bytes());
    let new_dict_end = out.len();
    write_txd_texture_count(&mut out, dict_start, new_dict_end, native_count);
    Ok(out)
}

pub(crate) fn rename_texture_native_in_txd(
    txd_bytes: Vec<u8>,
    old_texture_name: &str,
    new_texture_name: &str,
) -> Result<Vec<u8>, String> {
    if txd_bytes.len() < 12 || rd32(&txd_bytes, 0) != 0x16 {
        return Err("TXD is not a RenderWare texture dictionary".to_string());
    }
    let old_key = lower(old_texture_name.trim());
    let new_name = sanitize_texture_name(new_texture_name);
    let new_key = lower(&new_name);
    if old_key.is_empty() || new_key.is_empty() {
        return Err("Texture names cannot be empty".to_string());
    }
    if old_key == new_key {
        return Err("The new texture name must differ from the current name".to_string());
    }
    if txd_contains_texture_native(&txd_bytes, &new_key) {
        return Err(format!("Texture '{new_name}' already exists in this TXD"));
    }

    let dict_start = 12usize;
    let dict_end = dict_start.saturating_add(rd32(&txd_bytes, 4) as usize);
    if dict_end > txd_bytes.len() {
        return Err("TXD dictionary size is invalid".to_string());
    }
    let mut out = txd_bytes[..dict_end].to_vec();
    let mut renamed = 0usize;
    let mut chunk = dict_start;
    while chunk + 12 <= dict_end {
        let size = rd32(&out, chunk + 4) as usize;
        let end = chunk.saturating_add(12).saturating_add(size);
        if end > dict_end {
            return Err("TXD child chunk is invalid".to_string());
        }
        if rd32(&out, chunk) == 0x15
            && texture_native_name(&out, chunk, end).is_some_and(|name| name == old_key)
        {
            let renamed_native = rename_texture_native(&out[chunk..end], &new_name)?;
            out[chunk..end].copy_from_slice(&renamed_native);
            renamed += 1;
        }
        chunk = end;
    }
    match renamed {
        0 => Err(format!(
            "Texture '{old_texture_name}' was not found in this TXD"
        )),
        1 => Ok(out),
        count => Err(format!(
            "Texture '{old_texture_name}' occurs {count} times in this TXD; rename is ambiguous"
        )),
    }
}

pub(crate) fn purge_unused_texture_natives_from_txd(
    txd_bytes: Vec<u8>,
    used_textures: &HashSet<String>,
) -> Result<(Vec<u8>, usize, usize), String> {
    if txd_bytes.len() < 12 || rd32(&txd_bytes, 0) != 0x16 {
        return Err("TXD is not a RenderWare texture dictionary".to_string());
    }
    let dict_start = 12usize;
    let dict_end = dict_start.saturating_add(rd32(&txd_bytes, 4) as usize);
    if dict_end > txd_bytes.len() {
        return Err("TXD dictionary size is invalid".to_string());
    }
    let mut out = Vec::with_capacity(txd_bytes.len());
    out.extend_from_slice(&txd_bytes[..dict_start]);
    let mut chunk = dict_start;
    let mut kept = 0u16;
    let mut removed = 0usize;
    let mut removed_bytes = 0usize;
    while chunk + 12 <= dict_end {
        let cid = rd32(&txd_bytes, chunk);
        let csz = rd32(&txd_bytes, chunk + 4) as usize;
        let ce = chunk + 12 + csz;
        if ce > dict_end {
            return Err("TXD child chunk is invalid".to_string());
        }
        let remove_native = cid == 0x15
            && texture_native_name(&txd_bytes, chunk, ce)
                .is_some_and(|name| !used_textures.contains(&name));
        if remove_native {
            removed += 1;
            removed_bytes = removed_bytes.saturating_add(ce.saturating_sub(chunk));
        } else {
            if cid == 0x15 {
                kept = kept.saturating_add(1);
            }
            out.extend_from_slice(&txd_bytes[chunk..ce]);
        }
        chunk = ce;
    }
    if removed == 0 {
        return Ok((txd_bytes[..dict_end].to_vec(), 0, 0));
    }
    let new_size = out.len().saturating_sub(12);
    out[4..8].copy_from_slice(&(new_size as u32).to_le_bytes());
    let out_len = out.len();
    write_txd_texture_count(&mut out, dict_start, out_len, kept);
    Ok((out, removed, removed_bytes))
}

pub(crate) fn source_texture_native(
    app: &AppState,
    texture_name: &str,
    txd_name: &str,
) -> Result<Vec<u8>, String> {
    let key = lower(texture_name);
    let txd_key = asset_key(txd_name, ".txd");
    let texture = app
        .txd_textures
        .get(&key)
        .and_then(|entries| {
            entries
                .iter()
                .find(|entry| entry.txd_name.eq_ignore_ascii_case(&txd_key))
        })
        .ok_or_else(|| format!("Could not locate {texture_name} in {txd_key}"))?;
    read_exact_range(
        &texture.img_path,
        texture.native_offset,
        texture.native_size as usize,
    )
}

pub(crate) fn txd_texture_entries(app: &AppState, txd_name: &str) -> Vec<TextureArchiveEntry> {
    let txd_key = asset_key(txd_name, ".txd");
    let mut entries = Vec::new();
    let mut seen = HashSet::<String>::new();
    for (name, textures) in &app.txd_textures {
        if let Some(texture) = textures
            .iter()
            .find(|texture| texture.txd_name.eq_ignore_ascii_case(&txd_key))
        {
            if seen.insert(name.clone()) {
                entries.push(TextureArchiveEntry {
                    name: name.clone(),
                    width: texture.width,
                    height: texture.height,
                    format: texture.format,
                    fingerprint: Some(texture.content_fingerprint),
                    thumbnail: txd_texture_thumbnail(texture),
                });
            }
        }
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}

pub(crate) fn txd_texture_thumbnail(texture: &TxdTexture) -> Option<Texture2D> {
    let (width, height, rgba) = decode_txd_texture(texture)?;
    let thumb = Texture2D::from_rgba8(width as u16, height as u16, &rgba);
    thumb.set_filter(FilterMode::Nearest);
    Some(thumb)
}

pub(crate) fn tx_format_label(format: TxFormat) -> &'static str {
    match format {
        TxFormat::Dxt1 => "DXT1",
        TxFormat::Dxt3 => "DXT3",
        TxFormat::Dxt5 => "DXT5",
        TxFormat::Bgra8888 => "BGRA",
        TxFormat::Bgr888 => "BGR",
        TxFormat::Rgb565 => "565",
        TxFormat::Argb1555 => "1555",
        TxFormat::Argb4444 => "4444",
        TxFormat::Pal8 => "PAL8",
    }
}

pub(crate) fn sanitize_texture_name(name: &str) -> String {
    let mut out = String::new();
    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch.to_ascii_lowercase());
        } else if ch.is_whitespace() {
            out.push('_');
        }
        if out.len() >= GTA_SA_TEXTURE_NAME_MAX {
            break;
        }
    }
    if out.is_empty() {
        "texture".to_string()
    } else {
        out
    }
}

/// `RwTexture::name` is a 32-byte C string in GTA:SA's RenderWare build.
/// Keep one byte for the required terminator.
pub(crate) const GTA_SA_TEXTURE_NAME_MAX: usize = 31;

pub(crate) fn shorten_overlong_txd_texture_names(
    mut txd_bytes: Vec<u8>,
) -> Result<(Vec<u8>, HashMap<String, String>), String> {
    if txd_bytes.len() < 12 || rd32(&txd_bytes, 0) != 0x16 {
        return Err("TXD is not a RenderWare texture dictionary".to_string());
    }
    let dict_start = 12usize;
    let dict_end = dict_start.saturating_add(rd32(&txd_bytes, 4) as usize);
    if dict_end > txd_bytes.len() {
        return Err("TXD dictionary size is invalid".to_string());
    }

    let mut names = Vec::<(String, usize)>::new();
    let mut chunk = dict_start;
    while chunk + 12 <= dict_end {
        let size = rd32(&txd_bytes, chunk + 4) as usize;
        let end = chunk.saturating_add(12).saturating_add(size);
        if end > dict_end {
            return Err("TXD child chunk is invalid".to_string());
        }
        if rd32(&txd_bytes, chunk) == 0x15 {
            let mut child = chunk + 12;
            while child + 12 <= end {
                let child_size = rd32(&txd_bytes, child + 4) as usize;
                let data = child + 12;
                let child_end = data.saturating_add(child_size);
                if child_end > end {
                    return Err("TXD texture native child chunk is invalid".to_string());
                }
                if rd32(&txd_bytes, child) == 0x01 && data + 40 <= child_end {
                    let name_offset = data + 8;
                    let field = &txd_bytes[name_offset..name_offset + 32];
                    let name_end = field.iter().position(|byte| *byte == 0).unwrap_or(32);
                    let name = lower(&String::from_utf8_lossy(&field[..name_end]));
                    if !name.is_empty() {
                        names.push((name, name_offset));
                    }
                    break;
                }
                child = child_end;
            }
        }
        chunk = end;
    }

    // Reserve every already-valid name first. This prevents a truncated long
    // name from stealing a short name that appears later in the dictionary.
    let mut occupied = names
        .iter()
        .filter(|(name, _)| name.len() <= GTA_SA_TEXTURE_NAME_MAX)
        .map(|(name, _)| lower(name))
        .collect::<HashSet<_>>();
    let mut renames = HashMap::new();
    for (name, _) in names
        .iter()
        .filter(|(name, _)| name.len() > GTA_SA_TEXTURE_NAME_MAX)
    {
        let old = lower(name);
        if renames.contains_key(&old) {
            continue;
        }
        let mut number = 0usize;
        let new_name = loop {
            let suffix = if number == 0 {
                String::new()
            } else {
                format!("_{number}")
            };
            let keep = GTA_SA_TEXTURE_NAME_MAX.saturating_sub(suffix.len());
            let mut stem = old
                .chars()
                .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-')
                .take(keep)
                .collect::<String>();
            if stem.is_empty() {
                stem = "texture".chars().take(keep).collect();
            }
            let candidate = format!("{stem}{suffix}");
            if occupied.insert(lower(&candidate)) {
                break candidate;
            }
            number += 1;
        };
        renames.insert(old, new_name);
    }

    for (name, name_offset) in &names {
        let Some(new_name) = renames.get(&lower(name)) else {
            continue;
        };
        let field = &mut txd_bytes[*name_offset..*name_offset + 32];
        field.fill(0);
        field[..new_name.len()].copy_from_slice(new_name.as_bytes());
    }
    Ok((txd_bytes, renames))
}

pub(crate) fn texture_name_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .map(sanitize_texture_name)
        .unwrap_or_else(|| "texture".to_string())
}

pub(crate) fn rgba_has_alpha(rgba: &[u8]) -> bool {
    rgba.chunks_exact(4).any(|px| px[3] < 255)
}

pub(crate) fn encode_bc_texture(
    rgba: &[u8],
    width: u32,
    height: u32,
    format: texpresso::Format,
) -> Vec<u8> {
    let width = width as usize;
    let height = height as usize;
    let mut out = vec![0u8; format.compressed_size(width, height)];
    format.compress(
        rgba,
        width,
        height,
        texpresso::Params {
            algorithm: texpresso::Algorithm::IterativeClusterFit,
            ..texpresso::Params::default()
        },
        &mut out,
    );
    out
}

pub(crate) fn imported_image_texture_native(
    path: &Path,
    texture_name: &str,
) -> Result<Vec<u8>, String> {
    let bytes = fs::read(path).map_err(|err| format!("{}: {err}", path.display()))?;
    let image = image::load_from_memory(&bytes)
        .map_err(|err| format!("{}: {err}", path.display()))?
        .to_rgba8();
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 || width > u16::MAX as u32 || height > u16::MAX as u32 {
        return Err(format!("Unsupported texture size {width}x{height}"));
    }
    let (rgba, width, height) =
        resize_rgba_to_power_of_two(image.as_raw(), width as u16, height as u16)?;
    Ok(texture_native_from_rgba(&rgba, width, height, texture_name))
}

pub(crate) struct GifFlipbook {
    pub(crate) rgba: Vec<u8>,
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) cols: usize,
    pub(crate) rows: usize,
    pub(crate) frame_count: usize,
    /// Start time (seconds) of each frame within one loop.
    pub(crate) frame_starts: Vec<f32>,
    pub(crate) duration: f32,
}

/// Decode an animated GIF and bake its frames into a single power-of-two
/// sprite-sheet atlas laid out on a roughly square grid, returning the atlas
/// plus the timing/layout needed to drive a UV-animation flipbook.
pub(crate) fn bake_gif_flipbook(path: &Path) -> Result<GifFlipbook, String> {
    use image::AnimationDecoder;

    const MAX_FRAMES: usize = 64;
    const MAX_ATLAS: u32 = 2048;

    let file = fs::File::open(path).map_err(|err| format!("{}: {err}", path.display()))?;
    let decoder = image::codecs::gif::GifDecoder::new(std::io::BufReader::new(file))
        .map_err(|err| format!("{}: {err}", path.display()))?;
    let frames = decoder
        .into_frames()
        .collect_frames()
        .map_err(|err| format!("{}: {err}", path.display()))?;
    if frames.is_empty() {
        return Err("GIF has no frames".to_string());
    }

    let frame_count = frames.len().min(MAX_FRAMES);
    let mut delays: Vec<f32> = Vec::with_capacity(frame_count);
    for frame in frames.iter().take(frame_count) {
        let (numer, denom) = frame.delay().numer_denom_ms();
        let ms = if denom == 0 {
            100.0
        } else {
            numer as f32 / denom as f32
        };
        delays.push(ms.max(20.0) / 1000.0);
    }

    let canvas_w = frames[0].buffer().width();
    let canvas_h = frames[0].buffer().height();
    if canvas_w == 0 || canvas_h == 0 {
        return Err("GIF has zero size".to_string());
    }

    let cols = (frame_count as f32).sqrt().ceil().max(1.0) as usize;
    let rows = frame_count.div_ceil(cols).max(1);

    let mut cell_w = canvas_w;
    let mut cell_h = canvas_h;
    let atlas_w0 = cell_w * cols as u32;
    let atlas_h0 = cell_h * rows as u32;
    let scale = (MAX_ATLAS as f32 / atlas_w0 as f32)
        .min(MAX_ATLAS as f32 / atlas_h0 as f32)
        .min(1.0);
    if scale < 1.0 {
        cell_w = ((cell_w as f32 * scale).floor() as u32).max(1);
        cell_h = ((cell_h as f32 * scale).floor() as u32).max(1);
    }

    let atlas_w = cell_w * cols as u32;
    let atlas_h = cell_h * rows as u32;
    let mut atlas = vec![0u8; (atlas_w * atlas_h * 4) as usize];
    for (i, frame) in frames.iter().take(frame_count).enumerate() {
        let col = (i % cols) as u32;
        let row = (i / cols) as u32;
        let buf = frame.buffer();
        let cell = if buf.width() == cell_w && buf.height() == cell_h {
            image::RgbaImage::from_raw(cell_w, cell_h, buf.as_raw().clone())
                .ok_or_else(|| "Could not read GIF frame".to_string())?
        } else {
            image::imageops::resize(buf, cell_w, cell_h, image::imageops::FilterType::Triangle)
        };
        let ox = col * cell_w;
        let oy = row * cell_h;
        let row_bytes = (cell_w * 4) as usize;
        for y in 0..cell_h {
            let src_off = (y * cell_w * 4) as usize;
            let dst_off = (((oy + y) * atlas_w + ox) * 4) as usize;
            atlas[dst_off..dst_off + row_bytes]
                .copy_from_slice(&cell.as_raw()[src_off..src_off + row_bytes]);
        }
    }

    if atlas_w > u16::MAX as u32 || atlas_h > u16::MAX as u32 {
        return Err(format!("Atlas too large {atlas_w}x{atlas_h}"));
    }
    let (rgba, width, height) =
        resize_rgba_to_power_of_two(&atlas, atlas_w as u16, atlas_h as u16)?;

    let mut frame_starts = Vec::with_capacity(frame_count);
    let mut acc = 0.0f32;
    for delay in &delays {
        frame_starts.push(acc);
        acc += *delay;
    }
    let duration = acc.max(0.05);

    Ok(GifFlipbook {
        rgba,
        width,
        height,
        cols,
        rows,
        frame_count,
        frame_starts,
        duration,
    })
}

pub(crate) fn remove_txd_from_texture_index(app: &mut AppState, txd_name: &str) {
    let txd_key = asset_key(txd_name, ".txd");
    app.txd_textures.retain(|_, entries| {
        entries.retain(|entry| !entry.txd_name.eq_ignore_ascii_case(&txd_key));
        !entries.is_empty()
    });
}

pub(crate) fn reindex_staged_txd(app: &mut AppState, txd_name: &str) {
    remove_txd_from_texture_index(app, txd_name);
    let replacement_img = wip_root_path(&app.root).join("imgs").join(REPLACEMENT_IMG);
    if replacement_img.is_file() {
        refresh_txd_archive_index(&replacement_img, &mut app.txd_textures);
    }
}

/// Re-index every dictionary in an archive after it has been rewritten.
///
/// Rebuilding an IMG can move entries even when only one TXD changed. Keeping
/// the old entries for the other TXDs would leave their native/data offsets
/// pointing at unrelated bytes in the new archive.
fn refresh_txd_archive_index(path: &Path, index: &mut TxdTextureIndex) {
    let entries = parse_img(path);
    remove_img_txd_dictionaries_from_index(index, &entries);
    index_txd_entries(path, &entries, index);
}

pub(crate) fn invalidate_cached_txd_textures(
    app: &mut AppState,
    txd_name: &str,
    texture_name: Option<&str>,
) {
    let txd_key = asset_key(txd_name, ".txd");
    let keys: Vec<String> = match texture_name {
        Some(texture_name) => {
            let key = lower(texture_name);
            vec![format!("{txd_key}|{key}")]
        }
        None => app
            .textures
            .keys()
            .filter(|key| key.starts_with(&format!("{txd_key}|")))
            .cloned()
            .collect(),
    };
    for key in keys {
        if let Some(texture) = app.textures.remove(&key) {
            let is_shared_missing_texture = app
                .textures
                .get(MISSING_TEXTURE_KEY)
                .is_some_and(|missing_texture| *missing_texture == texture);
            if texture != 0 && !is_shared_missing_texture {
                unsafe {
                    gl::DeleteTextures(1, &texture);
                }
            }
        }
    }
}

#[cfg(test)]
mod txd_import_tests {
    use super::*;

    fn write_test_png_sized(name: &str, rgba: &[u8], width: u32, height: u32) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "eagle_txd_import_{}_{}.png",
            name,
            std::process::id()
        ));
        image::save_buffer(&path, rgba, width, height, image::ColorType::Rgba8).unwrap();
        path
    }

    fn write_test_png(name: &str, rgba: &[u8]) -> PathBuf {
        write_test_png_sized(name, rgba, 4, 4)
    }

    fn native_header(native: &[u8]) -> (&[u8], u32, u16, u16, u8, u8, u32) {
        let struct_start = 24usize;
        let raster_format = rd32(native, struct_start + 72);
        let d3d_format = &native[struct_start + 76..struct_start + 80];
        let width = rd16(native, struct_start + 80);
        let height = rd16(native, struct_start + 82);
        let depth = native[struct_start + 84];
        let alpha = native[struct_start + 87];
        let data_size = rd32(native, struct_start + 88);
        (
            d3d_format,
            raster_format,
            width,
            height,
            depth,
            alpha,
            data_size,
        )
    }

    fn raw_bgra_native_sized(name: &str, fourcc: [u8; 4], width: u16, height: u16) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&9u32.to_le_bytes());
        data.extend_from_slice(&0x1101u32.to_le_bytes());
        let mut raw_name = [0u8; 32];
        raw_name[..name.len()].copy_from_slice(name.as_bytes());
        data.extend_from_slice(&raw_name);
        data.extend_from_slice(&[0u8; 32]);
        data.extend_from_slice(&0x0500u32.to_le_bytes());
        data.extend_from_slice(&fourcc);
        data.extend_from_slice(&width.to_le_bytes());
        data.extend_from_slice(&height.to_le_bytes());
        data.push(32);
        data.push(1);
        data.push(4);
        data.push(0);
        let data_size = width as usize * height as usize * 4;
        data.extend_from_slice(&(data_size as u32).to_le_bytes());
        data.extend_from_slice(&vec![255u8; data_size]);
        rw_chunk(0x15, rw_chunk(0x01, data))
    }

    fn raw_bgra_native(name: &str, fourcc: [u8; 4]) -> Vec<u8> {
        raw_bgra_native_sized(name, fourcc, 4, 4)
    }

    fn raw_bgra_native_d3d(name: &str, d3d_format: u32, width: u16, height: u16) -> Vec<u8> {
        raw_bgra_native_sized(name, d3d_format.to_le_bytes(), width, height)
    }

    fn bad_compressed_header_native(name: &str, with_alpha: bool) -> Vec<u8> {
        let mut rgba = vec![255u8; 4 * 4 * 4];
        if with_alpha {
            rgba[3] = 128;
        }
        let mut native = texture_native_from_rgba(&rgba, 4, 4, name);
        let struct_start = 24usize;
        native[struct_start + 72..struct_start + 76].copy_from_slice(&0x0500u32.to_le_bytes());
        if with_alpha {
            native[struct_start + 84] = 32;
            native[struct_start + 87] = 1;
        } else {
            native[struct_start + 87] = 0;
        }
        native
    }

    fn one_texture_txd(native: Vec<u8>) -> Vec<u8> {
        rw_chunk(
            0x16,
            [rw_chunk(0x01, 1u16.to_le_bytes().to_vec()), native].concat(),
        )
    }

    #[test]
    fn imported_opaque_textures_write_dxt1_format() {
        let rgba = vec![255u8; 4 * 4 * 4];
        let path = write_test_png("opaque", &rgba);

        let native = imported_image_texture_native(&path, "opaque_tex").unwrap();
        let _ = fs::remove_file(path);
        let (d3d_format, raster_format, width, height, depth, alpha, data_size) =
            native_header(&native);

        assert_eq!(d3d_format, b"DXT1");
        assert_eq!(raster_format & 0x0F00, 0x0200);
        assert_eq!((width, height), (4, 4));
        assert_eq!(depth, 16);
        assert_eq!(alpha, 8);
        assert_eq!(data_size, 8);
    }

    #[test]
    fn imported_alpha_textures_write_dxt5_format() {
        let mut rgba = vec![255u8; 4 * 4 * 4];
        rgba[3] = 128;
        let path = write_test_png("alpha", &rgba);

        let native = imported_image_texture_native(&path, "alpha_tex").unwrap();
        let _ = fs::remove_file(path);
        let (d3d_format, raster_format, width, height, depth, alpha, data_size) =
            native_header(&native);

        assert_eq!(d3d_format, b"DXT5");
        assert_eq!(raster_format & 0x0F00, 0x0300);
        assert_eq!((width, height), (4, 4));
        assert_eq!(depth, 16);
        assert_eq!(alpha, 9);
        assert_eq!(data_size, 16);
    }

    #[test]
    fn imported_non_power_of_two_textures_resize_before_compression() {
        let rgba = vec![255u8; 5 * 5 * 4];
        let path = write_test_png_sized("npot", &rgba, 5, 5);

        let native = imported_image_texture_native(&path, "npot_tex").unwrap();
        let _ = fs::remove_file(path);
        let (d3d_format, raster_format, width, height, depth, alpha, data_size) =
            native_header(&native);

        assert_eq!(d3d_format, b"DXT1");
        assert_eq!(raster_format & 0x0F00, 0x0200);
        assert_eq!((width, height), (8, 8));
        assert_eq!(depth, 16);
        assert_eq!(alpha, 8);
        assert_eq!(data_size, 32);
    }

    #[test]
    fn optimize_repairs_invalid_compressed_headers() {
        let txd = rw_chunk(
            0x16,
            [
                rw_chunk(0x01, 2u16.to_le_bytes().to_vec()),
                bad_compressed_header_native("bad_dxt1", false),
                bad_compressed_header_native("bad_dxt5", true),
            ]
            .concat(),
        );
        let warnings = validate_txd_texture_formats("test.txd", &txd);
        assert!(warnings.iter().any(|warning| warning.contains("bad_dxt1")));
        assert!(warnings.iter().any(|warning| warning.contains("bad_dxt5")));

        let (updated, fixes, warnings) = optimize_txd_texture_formats(txd).unwrap();

        assert_eq!(fixes.len(), 2);
        assert!(warnings.is_empty());
        assert!(validate_txd_texture_formats("test.txd", &updated).is_empty());
        let first = parse_txd_native_header(&updated, 26, updated.len()).unwrap();
        assert_eq!(first.raster_format & 0x0F00, 0x0200);
        assert_eq!(first.depth, 16);
        assert_eq!(first.compression, 8);
    }

    #[test]
    fn cleanup_sets_missing_mipmapped_flag_without_changing_mip_data() {
        let rgba = vec![255u8; 16 * 16 * 4];
        let mut native =
            texture_native_from_rgba_with_mip_count(&rgba, 16, 16, "missing_mip_flag", 5);
        let native_header = parse_txd_native_header(&native, 0, native.len()).unwrap();
        let raster_offset = native_header.raster_format_offset;
        native[raster_offset..raster_offset + 4]
            .copy_from_slice(&(native_header.raster_format & !0x8000).to_le_bytes());
        let txd = one_texture_txd(native);
        let txd_for_profile = txd.clone();
        let before = parse_txd_native_header(&txd, 26, txd.len()).unwrap();
        let before_mips = before
            .mip_levels
            .iter()
            .map(|mip| txd[mip.data_offset..mip.data_offset + mip.data_size].to_vec())
            .collect::<Vec<_>>();

        assert!(
            validate_txd_texture_formats("test.txd", &txd)
                .iter()
                .any(|warning| warning.contains("without the mipmapped raster flag"))
        );

        let (updated, fixes, warnings) = optimize_txd_texture_formats(txd).unwrap();
        let after = parse_txd_native_header(&updated, 26, updated.len()).unwrap();
        let after_mips = after
            .mip_levels
            .iter()
            .map(|mip| updated[mip.data_offset..mip.data_offset + mip.data_size].to_vec())
            .collect::<Vec<_>>();

        assert_eq!(fixes.len(), 1);
        assert!(fixes[0].contains("set mipmapped raster flag for 5 mip levels"));
        assert!(warnings.is_empty());
        assert_ne!(after.raster_format & 0x8000, 0);
        assert_eq!(after_mips, before_mips);
        assert!(validate_txd_texture_formats("test.txd", &updated).is_empty());

        let (repeated, repeated_fixes, repeated_warnings) =
            optimize_txd_texture_formats(updated.clone()).unwrap();
        assert_eq!(repeated, updated);
        assert!(repeated_fixes.is_empty());
        assert!(repeated_warnings.is_empty());

        let profiled = optimize_txd_texture_formats_with_profile(
            txd_for_profile,
            TxdOptimizationProfile::balanced(1024, true),
        )
        .unwrap();
        let profiled_header =
            parse_txd_native_header(&profiled.bytes, 26, profiled.bytes.len()).unwrap();
        assert_ne!(profiled_header.raster_format & 0x8000, 0);
        assert_eq!(profiled.textures.len(), 1);
        assert!(
            profiled.textures[0]
                .details
                .iter()
                .any(|detail| detail.contains("set mipmapped raster flag"))
        );
    }

    #[test]
    fn cleanup_preserves_opaque_dxt5_specular_mask() {
        let rgba = [70, 90, 110, 96].repeat(16);
        let mut native = texture_native_from_rgba(&rgba, 4, 4, "road_specular");
        native[24 + 87] = 8;
        let txd = one_texture_txd(native);

        assert!(validate_txd_texture_formats("road.txd", &txd).is_empty());
        let (updated, fixes, warnings) = optimize_txd_texture_formats(txd.clone()).unwrap();

        assert_eq!(updated, txd);
        assert!(fixes.is_empty());
        assert!(warnings.is_empty());
        let header = parse_txd_native_header(&updated, 26, updated.len()).unwrap();
        assert_eq!(header.format, Some(TxFormat::Dxt5));
        assert!(!header.has_alpha);
        assert_eq!(header.compression, 8);
    }

    #[test]
    fn cleanup_shortens_texture_names_and_avoids_existing_names() {
        let long_name = "abcdefghijklmnopqrstuvwxyz123456";
        let reserved = &long_name[..GTA_SA_TEXTURE_NAME_MAX];
        let txd = rw_chunk(
            0x16,
            [
                rw_chunk(0x01, 2u16.to_le_bytes().to_vec()),
                raw_bgra_native(reserved, *b"DXT1"),
                raw_bgra_native(long_name, *b"DXT1"),
            ]
            .concat(),
        );

        let (updated, renames) = shorten_overlong_txd_texture_names(txd).unwrap();
        let renamed = renames.get(long_name).unwrap();
        assert_eq!(renamed.len(), GTA_SA_TEXTURE_NAME_MAX);
        assert!(renamed.ends_with("_1"));
        assert_ne!(renamed, reserved);
        assert!(txd_contains_texture_native(&updated, reserved));
        assert!(txd_contains_texture_native(&updated, renamed));
    }

    #[test]
    fn rename_texture_native_changes_the_name_without_duplicating_it() {
        let txd = rw_chunk(
            0x16,
            [
                rw_chunk(0x01, 1u16.to_le_bytes().to_vec()),
                raw_bgra_native("road", *b"DXT1"),
            ]
            .concat(),
        );

        let updated = rename_texture_native_in_txd(txd, "road", "city_road_markings_01").unwrap();

        assert!(!txd_contains_texture_native(&updated, "road"));
        assert!(txd_contains_texture_native(
            &updated,
            "city_road_markings_01"
        ));
        assert_eq!(txd_texture_count(&updated, 12, updated.len()), Some(1));
    }

    #[test]
    fn rename_texture_native_rejects_an_existing_destination() {
        let txd = rw_chunk(
            0x16,
            [
                rw_chunk(0x01, 2u16.to_le_bytes().to_vec()),
                raw_bgra_native("road", *b"DXT1"),
                raw_bgra_native("wall", *b"DXT1"),
            ]
            .concat(),
        );

        let error = rename_texture_native_in_txd(txd, "road", "wall").unwrap_err();

        assert!(error.contains("already exists"));
    }

    #[test]
    fn optimize_converts_raw_argb_to_dxt1() {
        let txd = rw_chunk(
            0x16,
            [
                rw_chunk(0x01, 1u16.to_le_bytes().to_vec()),
                raw_bgra_native("raw_tex", [0, 0, 0, 0]),
            ]
            .concat(),
        );

        let (updated, fixes, warnings) = optimize_txd_texture_formats(txd).unwrap();
        assert_eq!(fixes.len(), 1);
        assert!(fixes[0].contains("raw_tex"));
        assert!(fixes[0].contains("BGRA"));
        assert!(fixes[0].contains("DXT1"));
        assert!(warnings.is_empty());
        assert!(validate_txd_texture_formats("test.txd", &updated).is_empty());

        let native_start = 26usize;
        let header = parse_txd_native_header(&updated, native_start, updated.len()).unwrap();
        assert_eq!(header.format, Some(TxFormat::Dxt1));
        assert_eq!(header.data_size, 8);
    }

    #[test]
    fn optimize_converts_numeric_a8r8g8b8_to_dxt1() {
        let txd = rw_chunk(
            0x16,
            [
                rw_chunk(0x01, 1u16.to_le_bytes().to_vec()),
                raw_bgra_native_d3d("argb_tex", 21, 4, 4),
            ]
            .concat(),
        );

        assert!(validate_txd_texture_formats("test.txd", &txd).is_empty());
        let (updated, fixes, warnings) = optimize_txd_texture_formats(txd).unwrap();

        assert_eq!(fixes.len(), 1);
        assert!(fixes[0].contains("argb_tex"));
        assert!(warnings.is_empty());
        let native_start = 26usize;
        let header = parse_txd_native_header(&updated, native_start, updated.len()).unwrap();
        assert_eq!(header.format, Some(TxFormat::Dxt1));
        assert_eq!(header.data_size, 8);
    }

    #[test]
    fn optimize_resizes_non_power_of_two_textures() {
        let txd = rw_chunk(
            0x16,
            [
                rw_chunk(0x01, 1u16.to_le_bytes().to_vec()),
                raw_bgra_native_sized("npot_tex", [0, 0, 0, 0], 5, 5),
            ]
            .concat(),
        );
        assert!(
            validate_txd_texture_formats("test.txd", &txd)
                .iter()
                .any(|warning| warning.contains("5x5"))
        );

        let (updated, fixes, warnings) = optimize_txd_texture_formats(txd).unwrap();
        assert_eq!(fixes.len(), 1);
        assert!(fixes[0].contains("5x5"));
        assert!(fixes[0].contains("8x8"));
        assert!(warnings.is_empty());
        assert!(validate_txd_texture_formats("test.txd", &updated).is_empty());

        let native_start = 26usize;
        let header = parse_txd_native_header(&updated, native_start, updated.len()).unwrap();
        assert_eq!((header.width, header.height), (8, 8));
        assert_eq!(header.format, Some(TxFormat::Dxt1));
        assert_eq!(header.data_size, 32);
    }

    #[test]
    fn profiled_optimizer_downscales_to_maximum_without_changing_aspect() {
        let rgba = vec![255u8; 1024 * 512 * 4];
        let txd = one_texture_txd(texture_native_from_rgba(&rgba, 1024, 512, "world_large"));

        let result = optimize_txd_texture_formats_with_profile(
            txd,
            TxdOptimizationProfile::balanced(256, false),
        )
        .unwrap();
        let header = parse_txd_native_header(&result.bytes, 26, result.bytes.len()).unwrap();

        assert_eq!((header.width, header.height), (256, 128));
        assert_eq!(header.format, Some(TxFormat::Dxt1));
        assert_eq!(result.textures.len(), 1);
        assert!(result.textures[0].bytes_saved > 0);
        assert!(
            result.textures[0]
                .details
                .iter()
                .any(|detail| detail.contains("1024x512 -> 256x128"))
        );
    }

    #[test]
    fn profiled_optimizer_chooses_bc1_for_opaque_and_bc3_for_alpha() {
        let opaque = vec![255u8; 8 * 8 * 4];
        let mut alpha = opaque.clone();
        alpha[3] = 96;
        let txd = rw_chunk(
            0x16,
            [
                rw_chunk(0x01, 2u16.to_le_bytes().to_vec()),
                raw_bgra_native_sized("opaque", [0, 0, 0, 0], 8, 8),
                {
                    let mut native = raw_bgra_native_sized("alpha", [0, 0, 0, 0], 8, 8);
                    native[24 + 87] = 9;
                    let header = parse_txd_native_header(&native, 0, native.len()).unwrap();
                    let data =
                        &mut native[header.data_offset..header.data_offset + header.data_size];
                    for (source, bgra) in alpha.chunks_exact(4).zip(data.chunks_exact_mut(4)) {
                        bgra.copy_from_slice(&[source[2], source[1], source[0], source[3]]);
                    }
                    native
                },
            ]
            .concat(),
        );

        let result = optimize_txd_texture_formats_with_profile(
            txd,
            TxdOptimizationProfile::balanced(1024, false),
        )
        .unwrap();
        let contents = parse_txd_texture_contents_with_natives(&result.bytes).unwrap();
        let opaque_native = contents
            .iter()
            .find(|texture| texture.name == "opaque")
            .unwrap();
        let alpha_native = contents
            .iter()
            .find(|texture| texture.name == "alpha")
            .unwrap();
        let opaque_header =
            parse_txd_native_header(&opaque_native.native, 0, opaque_native.native.len()).unwrap();
        let alpha_header =
            parse_txd_native_header(&alpha_native.native, 0, alpha_native.native.len()).unwrap();

        assert_eq!(opaque_header.format, Some(TxFormat::Dxt1));
        assert_eq!(alpha_header.format, Some(TxFormat::Dxt5));
    }

    #[test]
    fn profiled_optimizer_writes_complete_valid_mip_chain() {
        let rgba = vec![255u8; 8 * 4 * 4];
        let txd = one_texture_txd(texture_native_from_rgba(&rgba, 8, 4, "world_mips"));

        let result = optimize_txd_texture_formats_with_profile(
            txd,
            TxdOptimizationProfile::balanced(1024, true),
        )
        .unwrap();
        let header = parse_txd_native_header(&result.bytes, 26, result.bytes.len()).unwrap();

        assert_eq!(header.mip_count, 4);
        assert_eq!(
            header
                .mip_levels
                .iter()
                .map(|mip| (mip.width, mip.height, mip.data_size))
                .collect::<Vec<_>>(),
            vec![(8, 4, 16), (4, 2, 8), (2, 1, 8), (1, 1, 8)]
        );
        assert_ne!(header.raster_format & 0x8000, 0);
        assert!(validate_txd_texture_formats("mips.txd", &result.bytes).is_empty());
    }

    #[test]
    fn profiled_optimizer_is_byte_identical_on_repeat_run() {
        let mut rgba = vec![255u8; 64 * 32 * 4];
        rgba[3] = 128;
        let txd = one_texture_txd(texture_native_from_rgba(&rgba, 64, 32, "repeat"));
        let profile = TxdOptimizationProfile::balanced(32, true);

        let first = optimize_txd_texture_formats_with_profile(txd, profile).unwrap();
        let second =
            optimize_txd_texture_formats_with_profile(first.bytes.clone(), profile).unwrap();

        assert_eq!(second.bytes, first.bytes);
        assert!(second.textures.is_empty());
    }

    #[test]
    fn lossless_profile_preserves_valid_dxt_bytes() {
        let rgba = vec![255u8; 16 * 16 * 4];
        let txd = one_texture_txd(texture_native_from_rgba(&rgba, 16, 16, "valid"));

        let result = optimize_txd_texture_formats_with_profile(
            txd.clone(),
            TxdOptimizationProfile::lossless(),
        )
        .unwrap();

        assert_eq!(result.bytes, txd);
        assert!(result.textures.is_empty());
    }

    #[test]
    fn validation_warns_on_unsupported_texture_format() {
        let txd = rw_chunk(
            0x16,
            [
                rw_chunk(0x01, 1u16.to_le_bytes().to_vec()),
                raw_bgra_native("bad_tex", *b"BAD!"),
            ]
            .concat(),
        );

        let warnings = validate_txd_texture_formats("test.txd", &txd);
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("unsupported texture format"))
        );
    }

    #[test]
    fn replace_texture_collapses_duplicate_destination_natives() {
        let old_a = raw_bgra_native("road", [0, 0, 0, 0]);
        let old_b = raw_bgra_native("road", [0, 0, 0, 0]);
        let replacement = raw_bgra_native("source", *b"DXT1");
        let txd = rw_chunk(
            0x16,
            [rw_chunk(0x01, 2u16.to_le_bytes().to_vec()), old_a, old_b].concat(),
        );

        let updated = replace_or_append_texture_native_in_txd(txd, &replacement, "road").unwrap();
        let textures = parse_txd_texture_contents_with_natives(&updated).unwrap();

        assert_eq!(
            textures
                .iter()
                .filter(|texture| texture.name == "road")
                .count(),
            1
        );
        assert_eq!(txd_texture_count(&updated, 12, updated.len()), Some(1));
        assert_eq!(
            parse_txd_native_header(&textures[0].native, 0, textures[0].native.len())
                .unwrap()
                .format,
            Some(TxFormat::Dxt1)
        );
    }

    #[test]
    fn missing_texture_copy_preserves_legacy_spaces_in_native_name() {
        let destination = rw_chunk(0x16, rw_chunk(0x01, 0u16.to_le_bytes().to_vec()));
        let source = raw_bgra_native("new road", *b"DXT1");

        let updated =
            replace_or_append_texture_native_in_txd_exact(destination, &source, "new road")
                .unwrap();

        assert!(txd_contains_texture_native(&updated, "new road"));
        assert!(!txd_contains_texture_native(&updated, "new_road"));
        assert_eq!(txd_texture_count(&updated, 12, updated.len()), Some(1));

        let mut index = TxdTextureIndex::new();
        index_one_txd(
            &updated,
            0,
            updated.len(),
            Path::new("replacement.img"),
            "starroad.txd",
            &mut index,
        );
        assert!(index.get("new road").is_some_and(|entries| {
            entries.iter().any(|entry| entry.txd_name == "starroad.txd")
        }));
        assert!(!index.contains_key("new_road"));
    }

    #[test]
    fn missing_texture_search_matches_sanitized_aliases_in_both_directions() {
        assert!(missing_texture_names_match("new_road", "new road"));
        assert!(missing_texture_names_match("new road", "new_road"));
        assert!(!missing_texture_names_match("new_road", "new-road"));

        let indexed = |txd_name: &str| TxdTexture {
            txd_name: txd_name.to_string(),
            img_path: PathBuf::from("textures.img"),
            native_offset: 0,
            native_size: 128,
            data_offset: 96,
            data_size: 8,
            palette_offset: 0,
            width: 4,
            height: 4,
            format: TxFormat::Dxt1,
            has_alpha: false,
            content_fingerprint: [1, 1],
        };
        let index = TxdTextureIndex::from([
            (
                "new_road".to_string(),
                vec![indexed("starroad.txd"), indexed("roads2.txd")],
            ),
            ("new road".to_string(), vec![indexed("starroad.txd")]),
        ]);

        let candidates = missing_texture_candidate_sources(&index, "new_road", "starroad.txd");

        assert_eq!(candidates.len(), 2);
        assert!(candidates.iter().any(|candidate| {
            candidate.txd_name == "roads2.txd" && candidate.source_texture_name == "new_road"
        }));
        assert!(candidates.iter().any(|candidate| {
            candidate.txd_name == "starroad.txd" && candidate.source_texture_name == "new road"
        }));
        assert!(!candidates.iter().any(|candidate| {
            candidate.txd_name == "starroad.txd" && candidate.source_texture_name == "new_road"
        }));
    }

    #[test]
    fn missing_texture_copy_renames_alias_to_exact_dff_reference() {
        let destination = rw_chunk(0x16, rw_chunk(0x01, 0u16.to_le_bytes().to_vec()));
        let source = raw_bgra_native("new road", *b"DXT1");

        let updated =
            replace_or_append_texture_native_in_txd_exact(destination, &source, "new_road")
                .unwrap();

        assert!(txd_contains_texture_native(&updated, "new_road"));
        assert!(!txd_contains_texture_native(&updated, "new road"));
    }

    #[test]
    fn staged_archive_reindex_refreshes_offsets_for_every_txd() {
        let path = std::env::temp_dir().join(format!(
            "eagle_reindex_staged_txd_{}_{}.img",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let first = one_texture_txd(raw_bgra_native("first", *b"DXT1"));
        let source = one_texture_txd(raw_bgra_native("littleha_hardware", *b"DXT1"));
        write_img_archive(
            &path,
            &[
                ("first.txd".to_string(), first.clone()),
                ("source.txd".to_string(), source.clone()),
            ],
        )
        .unwrap();

        let mut index = TxdTextureIndex::new();
        refresh_txd_archive_index(&path, &mut index);
        let old_offset = index["littleha_hardware"][0].native_offset;

        // Growing an earlier member moves source.txd to a later IMG sector.
        let mut grown_first = first;
        grown_first.extend_from_slice(&vec![0; 4096]);
        write_img_archive(
            &path,
            &[
                ("first.txd".to_string(), grown_first),
                ("source.txd".to_string(), source),
            ],
        )
        .unwrap();
        refresh_txd_archive_index(&path, &mut index);

        let refreshed = &index["littleha_hardware"][0];
        assert_ne!(refreshed.native_offset, old_offset);
        let native = read_exact_range(
            &refreshed.img_path,
            refreshed.native_offset,
            refreshed.native_size as usize,
        )
        .unwrap();
        assert_eq!(rd32(&native, 0), 0x15);

        fs::remove_file(path).unwrap();
    }
}

pub(crate) fn recompile_definitions_using_txd(app: &mut AppState, txd_name: &str) -> usize {
    let txd_key = asset_key(txd_name, ".txd");
    let ids: Vec<String> = app
        .definitions
        .iter()
        .filter_map(|(id, def)| {
            definition_txd_name_from_attrs(def)
                .is_some_and(|txd| asset_key(txd, ".txd") == txd_key)
                .then(|| id.clone())
        })
        .collect();
    let mut count = 0usize;
    for id in ids {
        if recompile_definition_mesh(app, &id) {
            count += 1;
        }
    }
    if count > 0 {
        rebuild_render_cells(app);
    }
    count
}

pub(crate) fn refresh_texture_archive_dialog(app: &mut AppState) {
    let Some((txd_name, selected_name)) = app.texture_archive_dialog.as_ref().map(|dialog| {
        (
            dialog.txd_name.clone(),
            dialog
                .textures
                .get(dialog.selected)
                .map(|entry| entry.name.clone()),
        )
    }) else {
        return;
    };
    let textures = txd_texture_entries(app, &txd_name);
    if let Some(dialog) = app.texture_archive_dialog.as_mut() {
        dialog.textures = textures;
        dialog.selected = selected_name
            .and_then(|name| dialog.textures.iter().position(|entry| entry.name == name))
            .unwrap_or(0)
            .min(dialog.textures.len().saturating_sub(1));
    }
    update_texture_archive_preview(app);
}

pub(crate) fn texture_archive_preview(
    app: &AppState,
    txd_name: &str,
    texture_name: &str,
) -> Option<Texture2D> {
    let txd_key = asset_key(txd_name, ".txd");
    let texture = app
        .txd_textures
        .get(&lower(texture_name))
        .and_then(|entries| {
            entries
                .iter()
                .find(|entry| entry.txd_name.eq_ignore_ascii_case(&txd_key))
        })?;
    let (width, height, rgba) = decode_txd_texture(texture)?;
    let preview = Texture2D::from_rgba8(width as u16, height as u16, &rgba);
    preview.set_filter(FilterMode::Nearest);
    Some(preview)
}

pub(crate) fn update_texture_archive_preview(app: &mut AppState) {
    let Some((txd_name, texture_name)) = app.texture_archive_dialog.as_ref().and_then(|dialog| {
        dialog
            .textures
            .get(dialog.selected)
            .map(|entry| (dialog.txd_name.clone(), entry.name.clone()))
    }) else {
        if let Some(dialog) = app.texture_archive_dialog.as_mut() {
            dialog.preview_texture = None;
        }
        return;
    };
    let preview = texture_archive_preview(app, &txd_name, &texture_name);
    if let Some(dialog) = app.texture_archive_dialog.as_mut() {
        dialog.preview_texture = preview;
    }
}

pub(crate) fn stage_texture_native_into_txd(
    app: &mut AppState,
    txd_name: &str,
    texture_name: &str,
    native: &[u8],
) -> Result<usize, String> {
    let txd_key = asset_key(txd_name, ".txd");
    let dest_entry = find_txd_entry_for_app(app, &txd_key)
        .ok_or_else(|| format!("Could not locate destination TXD {txd_key}"))?;
    let dest_bytes = read_txd_entry_bytes(&dest_entry);
    let updated = replace_or_append_texture_native_in_txd(dest_bytes, native, texture_name)?;
    let wip_root = wip_root_path(&app.root);
    upsert_replacement_txd(&wip_root, &txd_key, &updated)?;
    app.pending_txd_writes.insert(txd_key.clone());
    app.loaded_wip = true;
    reindex_staged_txd(app, &txd_key);
    invalidate_cached_txd_textures(app, &txd_key, None);
    let recompiled = recompile_definitions_using_txd(app, &txd_key);
    invalidate_validation_cache(app);
    refresh_texture_archive_dialog(app);
    Ok(recompiled)
}

pub(crate) fn recompile_definition_mesh(app: &mut AppState, definition_id: &str) -> bool {
    let Some(placement) = app
        .placements
        .iter()
        .find(|placement| placement.id == definition_id)
        .cloned()
    else {
        return false;
    };
    let Some(entry) = find_dff_entry_for_app(app, &placement.dff) else {
        return false;
    };
    let raw = parse_dff_mesh(&read_img_entry(&entry));
    let txd_scope = definition_txd_name(&app.definitions, definition_id).map(ToOwned::to_owned);
    let overrides = texture_override_map_for_definition(app, definition_id);
    let texture_files = collect_texture_files(&app.root);
    let ambient_lift = scene_ambient_lift_from_timecyc(&app.timecyc);
    let Some(mesh) = compile_render_mesh(
        raw,
        txd_scope.as_deref(),
        Some(&overrides),
        None,
        &texture_files,
        &app.txd_textures,
        &mut app.textures,
        &mut app.textured_parts,
        app.options.textures,
        ambient_lift,
    ) else {
        return false;
    };
    let mesh_key = placement_mesh_key(&placement, &app.definitions);
    app.meshes.insert(mesh_key, mesh);
    true
}

pub(crate) fn stage_missing_texture_into_definition_txd(
    app: &mut AppState,
    choice: &MissingTextureChoice,
    source_txd_name: &str,
    source_texture_name: &str,
) -> Result<String, String> {
    let dest_txd = definition_txd_name(&app.definitions, &choice.definition_id)
        .map(|txd| asset_key(txd, ".txd"))
        .ok_or_else(|| format!("{} has no TXD definition", choice.definition_id))?;
    let native = source_texture_native(app, source_texture_name, source_txd_name)?;
    let dest_entry = find_txd_entry_for_app(app, &dest_txd)
        .ok_or_else(|| format!("Could not locate destination TXD {dest_txd}"))?;
    let dest_bytes = read_txd_entry_bytes(&dest_entry);
    let updated =
        replace_or_append_texture_native_in_txd_exact(dest_bytes, &native, &choice.texture_name)?;
    let wip_root = wip_root_path(&app.root);
    upsert_replacement_txd(&wip_root, &dest_txd, &updated)?;

    reindex_staged_txd(app, &dest_txd);
    invalidate_cached_txd_textures(app, &dest_txd, Some(&choice.texture_name));
    app.texture_overrides
        .remove(&(choice.definition_id.clone(), choice.texture_name.clone()));
    app.pending_txd_writes.insert(dest_txd.clone());
    app.loaded_wip = true;
    Ok(dest_txd)
}

pub(crate) fn missing_texture_candidates_for_definition(
    app: &AppState,
    definition_id_filter: Option<&str>,
) -> Vec<MissingTextureChoice> {
    let mut seen = HashSet::<(String, String)>::new();
    let mut choices = Vec::new();
    for placement in &app.placements {
        let definition_id = placement.id.clone();
        if definition_id_filter.is_some_and(|id| id != definition_id) {
            continue;
        }
        let current_txd = definition_txd_name(&app.definitions, &definition_id)
            .map(|txd| asset_key(txd, ".txd"))
            .unwrap_or_default();
        let Some(mesh) = element_mesh(app, placement) else {
            continue;
        };
        for part in &mesh.parts {
            let texture_name = lower(part.texture_name.trim());
            if texture_name.is_empty() || !part.texture_missing {
                continue;
            }
            if app
                .texture_overrides
                .contains_key(&(definition_id.clone(), texture_name.clone()))
            {
                continue;
            }
            if !seen.insert((definition_id.clone(), texture_name.clone())) {
                continue;
            }
            let candidates =
                missing_texture_candidate_sources(&app.txd_textures, &texture_name, &current_txd);
            if !candidates.is_empty() {
                choices.push(MissingTextureChoice {
                    definition_id: definition_id.clone(),
                    texture_name,
                    candidates,
                });
            }
        }
    }
    choices
}

fn missing_texture_names_match(requested: &str, indexed: &str) -> bool {
    let requested = lower(requested.trim());
    let indexed = lower(indexed.trim());
    requested == indexed || sanitize_texture_name(&requested) == sanitize_texture_name(&indexed)
}

fn missing_texture_candidate_sources(
    txd_textures: &TxdTextureIndex,
    requested_texture_name: &str,
    current_txd: &str,
) -> Vec<MissingTextureCandidate> {
    let requested = lower(requested_texture_name.trim());
    let mut candidates = Vec::<MissingTextureCandidate>::new();
    let mut seen = HashSet::<(String, String)>::new();
    for (source_texture_name, entries) in txd_textures {
        let source_texture_name = lower(source_texture_name.trim());
        if !missing_texture_names_match(&requested, &source_texture_name) {
            continue;
        }
        for entry in entries {
            let txd_name = asset_key(&entry.txd_name, ".txd");
            // An exact hit in the assigned TXD cannot be missing. A sanitized
            // alias in that same TXD is useful: copying it under the exact DFF
            // name repairs the lookup without requiring another dictionary.
            if txd_name.eq_ignore_ascii_case(current_txd) && source_texture_name == requested {
                continue;
            }
            if seen.insert((txd_name.clone(), source_texture_name.clone())) {
                candidates.push(MissingTextureCandidate {
                    txd_name,
                    source_texture_name: source_texture_name.clone(),
                });
            }
        }
    }
    candidates.sort_by(|a, b| {
        let a_alias = a.source_texture_name != requested;
        let b_alias = b.source_texture_name != requested;
        (a_alias, &a.txd_name, &a.source_texture_name).cmp(&(
            b_alias,
            &b.txd_name,
            &b.source_texture_name,
        ))
    });
    candidates
}

pub(crate) fn selected_definition_missing_texture_candidates(
    app: &AppState,
) -> Vec<MissingTextureChoice> {
    let Some(placement) = selected_placement(app) else {
        return Vec::new();
    };
    missing_texture_candidates_for_definition(app, Some(&placement.id))
}

pub(crate) fn selected_definition_has_missing_textures(app: &AppState) -> bool {
    if !app.options.textures {
        return false;
    }
    let Some(placement) = selected_placement(app) else {
        return false;
    };
    let Some(mesh) = element_mesh(app, placement) else {
        return false;
    };
    mesh.parts.iter().any(|part| {
        let texture_name = lower(part.texture_name.trim());
        !texture_name.is_empty()
            && part.texture_missing
            && !app
                .texture_overrides
                .contains_key(&(placement.id.clone(), texture_name))
    })
}

pub(crate) fn missing_texture_candidate_previews(
    app: &AppState,
    choice: &MissingTextureChoice,
) -> Vec<MissingTextureCandidatePreview> {
    let mut out = Vec::<MissingTextureCandidatePreview>::new();
    let mut seen = HashMap::<(u32, u32, u64), usize>::new();
    for candidate in &choice.candidates {
        let txd_key = asset_key(&candidate.txd_name, ".txd");
        let texture_key = lower(&candidate.source_texture_name);
        let Some(texture) = app.txd_textures.get(&texture_key).and_then(|entries| {
            entries
                .iter()
                .find(|entry| entry.txd_name.eq_ignore_ascii_case(&txd_key))
        }) else {
            out.push(MissingTextureCandidatePreview {
                txd_name: txd_key,
                source_texture_name: texture_key,
                width: 0,
                height: 0,
                format: TxFormat::Bgra8888,
                thumbnail: None,
                duplicate_count: 1,
            });
            continue;
        };
        let signature = missing_texture_signature_for_texture(texture);
        if let Some(signature) = signature {
            if let Some(existing) = seen.get(&signature).copied() {
                out[existing].duplicate_count += 1;
                continue;
            }
            seen.insert(signature, out.len());
        }
        out.push(MissingTextureCandidatePreview {
            txd_name: txd_key,
            source_texture_name: texture_key,
            width: texture.width,
            height: texture.height,
            format: texture.format,
            thumbnail: txd_texture_thumbnail(texture),
            duplicate_count: 1,
        });
    }
    out
}

fn missing_texture_signature_for_texture(texture: &TxdTexture) -> Option<(u32, u32, u64)> {
    let (width, height, rgba) = decode_txd_texture(texture)?;
    let mut hasher = DefaultHasher::new();
    width.hash(&mut hasher);
    height.hash(&mut hasher);
    rgba.hash(&mut hasher);
    Some((width, height, hasher.finish()))
}

fn auto_missing_texture_candidate(
    app: &AppState,
    choice: &MissingTextureChoice,
) -> Option<MissingTextureCandidate> {
    let first = choice.candidates.first()?.clone();
    if choice.candidates.len() == 1 {
        return Some(first);
    }
    let grouped = missing_texture_candidate_previews(app, choice);
    (grouped.len() == 1).then(|| MissingTextureCandidate {
        txd_name: grouped[0].txd_name.clone(),
        source_texture_name: grouped[0].source_texture_name.clone(),
    })
}

pub(crate) fn open_missing_texture_dialog(
    app: &mut AppState,
    choice: MissingTextureChoice,
    remaining: Vec<MissingTextureChoice>,
    applied: usize,
) {
    let candidates = missing_texture_candidate_previews(app, &choice);
    app.missing_texture_dialog = Some(MissingTextureDialog {
        choice,
        candidates,
        remaining,
        applied,
    });
}

pub(crate) fn apply_missing_texture_override(
    app: &mut AppState,
    choice: &MissingTextureChoice,
    candidate: &MissingTextureCandidate,
) -> bool {
    let dest_txd = match stage_missing_texture_into_definition_txd(
        app,
        choice,
        &candidate.txd_name,
        &candidate.source_texture_name,
    ) {
        Ok(dest_txd) => dest_txd,
        Err(err) => {
            app.status_message = format!(
                "Could not write {} into {}'s TXD: {err}",
                choice.texture_name, choice.definition_id
            );
            return false;
        }
    };
    if recompile_definition_mesh(app, &choice.definition_id) {
        rebuild_render_cells(app);
        let still_missing = app
            .placements
            .iter()
            .find(|placement| placement.id == choice.definition_id)
            .and_then(|placement| element_mesh(app, placement))
            .is_none_or(|mesh| {
                mesh.parts.iter().any(|part| {
                    part.texture_missing
                        && part
                            .texture_name
                            .trim()
                            .eq_ignore_ascii_case(&choice.texture_name)
                })
            });
        if still_missing {
            app.status_message = format!(
                "Staged {} into {}, but preview verification still reports it missing for {}",
                choice.texture_name, dest_txd, choice.definition_id
            );
            return false;
        }
        app.status_message = format!(
            "Copied {} from {} as {} into {} for {}",
            candidate.source_texture_name,
            candidate.txd_name,
            choice.texture_name,
            dest_txd,
            choice.definition_id
        );
        true
    } else {
        app.status_message = format!("Could not recompile {}", choice.definition_id);
        false
    }
}

pub(crate) fn continue_missing_texture_fixes(
    app: &mut AppState,
    choices: Vec<MissingTextureChoice>,
    mut applied: usize,
) {
    let mut iter = choices.into_iter();
    while let Some(choice) = iter.next() {
        if let Some(candidate) = auto_missing_texture_candidate(app, &choice) {
            if apply_missing_texture_override(app, &choice, &candidate) {
                applied += 1;
            } else {
                return;
            }
            continue;
        }
        let remaining: Vec<MissingTextureChoice> = iter.collect();
        open_missing_texture_dialog(app, choice, remaining, applied);
        app.status_message = format!(
            "Applied {applied} automatic texture fix(es). Choose a TXD for the next differing match."
        );
        return;
    }
    if applied == 0 {
        app.status_message = "No missing texture matches found for this definition.".to_string();
    } else {
        app.status_message = format!("Applied {applied} missing texture fix(es).");
    }
}

pub(crate) fn find_missing_textures_for_selected_definition(app: &mut AppState) {
    let choices = selected_definition_missing_texture_candidates(app);
    continue_missing_texture_fixes(app, choices, 0);
}

pub(crate) fn find_missing_texture_for_selected_definition(app: &mut AppState, texture_name: &str) {
    let requested = lower(texture_name.trim());
    let choice = selected_definition_missing_texture_candidates(app)
        .into_iter()
        .find(|choice| choice.texture_name.eq_ignore_ascii_case(&requested));
    let Some(choice) = choice else {
        app.status_message = format!(
            "No texture match was found for '{}' on the selected model.",
            texture_name
        );
        return;
    };
    continue_missing_texture_fixes(app, vec![choice], 0);
}

pub(crate) fn collect_archive_asset_names(
    root: &Path,
    dffs: &mut BTreeSet<String>,
    cols: &mut BTreeSet<String>,
    txds: &mut BTreeSet<String>,
) {
    collect_archive_asset_names_from_files(collect_resource_img_files(root), dffs, cols, txds);
}

pub(crate) fn collect_archive_asset_names_from_files(
    img_files: impl IntoIterator<Item = PathBuf>,
    dffs: &mut BTreeSet<String>,
    cols: &mut BTreeSet<String>,
    txds: &mut BTreeSet<String>,
) {
    for path in img_files {
        for entry in parse_img(&path) {
            let name = lower(&entry.name);
            if name.ends_with(".dff") {
                dffs.insert(name);
            } else if name.ends_with(".col") {
                cols.insert(name);
                for col_name in collect_col_model_names(&read_img_entry(&entry)) {
                    cols.insert(col_name);
                }
            } else if name.ends_with(".txd") {
                txds.insert(name);
            }
        }
    }
}

pub(crate) fn collect_loose_asset_names(
    root: &Path,
    dffs: &mut BTreeSet<String>,
    cols: &mut BTreeSet<String>,
    txds: &mut BTreeSet<String>,
) {
    for dir in [
        "imgs",
        "Imgs",
        "models",
        "Models",
        "textures",
        "Textures",
        "txd_build",
        "TXD_Build",
    ] {
        let path = root.join(dir);
        if !path.exists() {
            continue;
        }
        for entry in WalkDir::new(path).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }
            let ext = lower(
                entry
                    .path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or(""),
            );
            let Some(name) = entry.path().file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            match ext.as_str() {
                "dff" => {
                    dffs.insert(asset_key(name, ".dff"));
                }
                "col" => {
                    cols.insert(asset_key(name, ".col"));
                }
                "txd" => {
                    txds.insert(asset_key(name, ".txd"));
                }
                _ => {}
            }
        }
    }
}

pub(crate) fn collect_resource_txd_entries(root: &Path) -> BTreeMap<String, ImgEntry> {
    let wip_root = wip_root_path(root);
    let mut entries = BTreeMap::new();
    for path in collect_resource_img_files(root) {
        let is_wip = path.starts_with(&wip_root);
        for entry in parse_img(&path) {
            if !lower(&entry.name).ends_with(".txd") {
                continue;
            }
            let name = asset_key(&entry.name, ".txd");
            if is_wip || !entries.contains_key(&name) {
                entries.insert(name, entry);
            }
        }
    }
    for path in collect_resource_txd_files(root) {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let key = asset_key(name, ".txd");
        let is_wip = path.starts_with(&wip_root);
        if (is_wip || !entries.contains_key(&key))
            && let Some(entry) = loose_txd_entry(&path)
        {
            entries.insert(key, entry);
        }
    }
    entries
}

pub(crate) fn collect_txd_format_warnings(root: &Path) -> Vec<String> {
    let mut warnings = Vec::new();
    for (txd_name, entry) in collect_resource_txd_entries(root) {
        let bytes = read_txd_entry_bytes(&entry);
        warnings.extend(validate_txd_texture_formats(&txd_name, &bytes));
    }
    warnings.sort();
    warnings.dedup();
    warnings
}

pub(crate) fn collect_resource_col_entries(root: &Path) -> BTreeMap<String, ImgEntry> {
    let wip_root = wip_root_path(root);
    let mut entries = BTreeMap::new();
    for path in collect_resource_img_files(root) {
        let is_wip = path.starts_with(&wip_root);
        for entry in parse_img(&path) {
            if !lower(&entry.name).ends_with(".col") {
                continue;
            }
            let name = asset_key(&entry.name, ".col");
            if is_wip || !entries.contains_key(&name) {
                entries.insert(name, entry);
            }
        }
    }
    for dir in ["imgs", "Imgs", "models", "Models"] {
        let path = root.join(dir);
        if !path.exists() {
            continue;
        }
        for entry in WalkDir::new(path).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }
            if lower(
                entry
                    .path()
                    .extension()
                    .and_then(|value| value.to_str())
                    .unwrap_or(""),
            ) != "col"
            {
                continue;
            }
            let Some(name) = entry.path().file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            let key = asset_key(name, ".col");
            let is_wip = entry.path().starts_with(&wip_root);
            if (is_wip || !entries.contains_key(&key))
                && let Ok(meta) = fs::metadata(entry.path())
            {
                entries.insert(
                    key,
                    ImgEntry {
                        img_path: entry.path().to_path_buf(),
                        name: name.to_string(),
                        offset: 0,
                        size: meta.len().min(u32::MAX as u64) as u32,
                    },
                );
            }
        }
    }
    entries
}

pub(crate) fn collect_col_load_warnings(root: &Path) -> Vec<String> {
    let mut warnings = Vec::new();
    for (col_name, entry) in collect_resource_col_entries(root) {
        let bytes = read_col_entry_bytes(&entry);
        for issue in validate_col_for_game_load(&col_name, &bytes) {
            warnings.push(issue.label(&col_name));
        }
    }
    warnings.sort();
    warnings.dedup();
    warnings
}

fn col_entry_has_collision_shapes(entry: &ImgEntry) -> bool {
    let bytes = read_col_entry_bytes(entry);
    let local = ImgEntry {
        img_path: entry.img_path.clone(),
        name: entry.name.clone(),
        offset: 0,
        size: bytes.len().min(u32::MAX as usize) as u32,
    };
    parse_col_mesh(&bytes, &local).is_some_and(|mesh| {
        !mesh.faces.is_empty() || !mesh.boxes.is_empty() || !mesh.spheres.is_empty()
    })
}

pub(crate) fn validation_summary(app: &AppState) -> ValidationSummary {
    let mut out = ValidationSummary::default();
    let placement_ids: BTreeSet<String> = app
        .placements
        .iter()
        .filter(|p| !p.id.trim().is_empty())
        .map(|p| p.id.to_ascii_lowercase())
        .collect();
    let definition_ids: BTreeSet<String> = app
        .definitions
        .keys()
        .map(|id| id.to_ascii_lowercase())
        .collect();

    for placement in &app.placements {
        if !definition_ids.contains(&placement.id.to_ascii_lowercase()) {
            out.missing_definition_ids.push(placement.id.clone());
        }
        if physics_root_value_from_attrs(&placement.attrs).is_some() {
            out.breakable_warnings.push(format!(
                "PHYSICS {}: per-object physicsRoot cannot be represented by MTA; move it to the global definition",
                placement.id
            ));
        }
        let definition = app.definitions.get(&placement.id);
        let physics_root = physics_root_value_from_attrs(&placement.attrs).or_else(|| {
            definition.and_then(|definition| physics_root_value_from_attrs(&definition.attrs))
        });
        let breakable_root = physics_root.and_then(|model_id| {
            app.physics_root_properties
                .get(&model_id)
                .or_else(|| physics_root_spec(model_id).map(|spec| &spec.fallback))
                .filter(|properties| properties.is_breakable())
                .map(|_| model_id)
        });
        if let Some(model_id) = breakable_root
            && !placement.tag.eq_ignore_ascii_case("object")
        {
            out.breakable_warnings.push(format!(
                "BREAKABLE {}: physics root {model_id} requires an object placement, not {}",
                placement.id, placement.tag
            ));
        }
    }
    let used_definition_ids: HashSet<String> =
        app.placements.iter().map(|p| p.id.clone()).collect();
    for def in app.definitions.values() {
        let readonly = app.readonly_definition_ids.contains(&def.id);
        if readonly && !used_definition_ids.contains(&def.id) {
            continue;
        }
        if !placement_ids.contains(&def.id.to_ascii_lowercase()) {
            out.unused_definitions.push(def.id.clone());
        }
        let dff_key = asset_key_opt(def.attrs.get("dff"), &def.id, ".dff");
        out.referenced_dffs.insert(dff_key.clone());
        let col_value = def.attrs.get("col").map(|value| value.trim()).unwrap_or("");
        if col_value.is_empty() {
            if readonly {
                out.referenced_cols.insert(
                    dff_key
                        .strip_suffix(".dff")
                        .map(|base| format!("{base}.col"))
                        .unwrap_or_else(|| asset_key(&def.id, ".col")),
                );
            } else {
                out.missing_col_attrs.push(def.id.clone());
            }
        } else {
            out.referenced_cols.insert(asset_key(col_value, ".col"));
        }
        if let Some(txd) = def
            .attrs
            .get("txd")
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
        {
            out.referenced_txds.insert(asset_key(txd, ".txd"));
        }
    }

    collect_archive_asset_names(
        &app.root,
        &mut out.img_dffs,
        &mut out.img_cols,
        &mut out.img_txds,
    );
    collect_loose_asset_names(
        &app.root,
        &mut out.loose_dffs,
        &mut out.loose_cols,
        &mut out.loose_txds,
    );
    out.invalid_texture_formats = collect_txd_format_warnings(&app.root);
    out.invalid_col_loads = collect_col_load_warnings(&app.root);
    for key in &app.pending_asset_deletes {
        out.img_dffs.remove(key);
        out.img_cols.remove(key);
        out.img_txds.remove(key);
        out.loose_dffs.remove(key);
        out.loose_cols.remove(key);
        out.loose_txds.remove(key);
    }
    let mut sa_dffs = BTreeSet::new();
    let mut sa_cols = BTreeSet::new();
    let mut sa_txds = BTreeSet::new();
    collect_archive_asset_names_from_files(
        gta_sa_img_files(&app.gta_sa_dir),
        &mut sa_dffs,
        &mut sa_cols,
        &mut sa_txds,
    );

    let all_dffs: BTreeSet<_> = out.img_dffs.union(&out.loose_dffs).cloned().collect();
    let all_cols: BTreeSet<_> = out.img_cols.union(&out.loose_cols).cloned().collect();
    let all_txds: BTreeSet<_> = out.img_txds.union(&out.loose_txds).cloned().collect();
    let available_dffs: BTreeSet<_> = all_dffs.union(&sa_dffs).cloned().collect();
    let available_cols: BTreeSet<_> = all_cols.union(&sa_cols).cloned().collect();
    let available_txds: BTreeSet<_> = all_txds.union(&sa_txds).cloned().collect();

    out.missing_dffs = out
        .referenced_dffs
        .difference(&available_dffs)
        .cloned()
        .collect();
    out.missing_cols = out
        .referenced_cols
        .difference(&available_cols)
        .cloned()
        .collect();
    out.missing_txds = out
        .referenced_txds
        .difference(&available_txds)
        .cloned()
        .collect();
    let local_col_entries = collect_resource_col_entries(&app.root);
    let missing_cols: BTreeSet<_> = out.missing_cols.iter().cloned().collect();
    for col_key in out.referenced_cols.difference(&missing_cols) {
        if let Some(entry) = local_col_entries.get(col_key) {
            let bytes = read_col_entry_bytes(entry);
            let internal_names: BTreeSet<_> = collect_col_model_names(&bytes).into_iter().collect();
            if !internal_names.contains(col_key) {
                out.invalid_col_loads.push(format!(
                    "WARN COL {col_key}: IMG/loose filename exists but internal COL model names are [{}]; game binds by internal name",
                    internal_names.into_iter().collect::<Vec<_>>().join(", ")
                ));
            }
        }
    }
    for def in app.definitions.values() {
        if !definition_flag_enabled(def, "disable_collisions") {
            continue;
        }
        let col_value = def.attrs.get("col").map(|value| value.trim()).unwrap_or("");
        let col_key = if col_value.is_empty() {
            asset_key_opt(def.attrs.get("dff"), &def.id, ".col")
        } else {
            asset_key(col_value, ".col")
        };
        if let Some(entry) = local_col_entries.get(&col_key)
            && col_entry_has_collision_shapes(entry)
        {
            out.invalid_col_loads.push(format!(
                "WARN COL {col_key}: definition {} has disable_collisions but resolves to non-empty collision geometry",
                def.id
            ));
        }
    }
    out.unused_dffs = all_dffs.difference(&out.referenced_dffs).cloned().collect();
    out.unused_cols = all_cols.difference(&out.referenced_cols).cloned().collect();
    out.unused_txds = all_txds.difference(&out.referenced_txds).cloned().collect();
    out.duplicate_dffs = out
        .img_dffs
        .intersection(&out.loose_dffs)
        .cloned()
        .collect();
    if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref() {
        let referenced_roots = app
            .placements
            .iter()
            .filter(|placement| asset_key(&placement.dff, ".dff") == asset_key(&dff.name, ".dff"))
            .filter_map(|placement| {
                physics_root_value_from_attrs(&placement.attrs).or_else(|| {
                    app.definitions
                        .get(&placement.id)
                        .and_then(|definition| physics_root_value_from_attrs(&definition.attrs))
                })
            })
            .collect::<BTreeSet<_>>();
        let expects_breakable = referenced_roots.iter().any(|model_id| {
            app.physics_root_properties
                .get(model_id)
                .or_else(|| physics_root_spec(*model_id).map(|spec| &spec.fallback))
                .is_some_and(PhysicsRootProperties::is_breakable)
        });
        let breakables = dff
            .raw
            .components
            .iter()
            .filter_map(|component| component.breakable.as_ref())
            .collect::<Vec<_>>();
        if expects_breakable && breakables.is_empty() {
            out.breakable_warnings.push(format!(
                "BREAKABLE {}: breakable physics root is set but the open DFF has no fragment mesh",
                dff.name
            ));
        }
        if !expects_breakable && !breakables.is_empty() {
            out.breakable_warnings.push(format!(
                "BREAKABLE {}: fragment mesh exists but no referencing definition uses a breakable physics root",
                dff.name
            ));
        }
        for (component, breakable) in dff
            .raw
            .components
            .iter()
            .filter_map(|component| component.breakable.as_ref().map(|data| (component, data)))
        {
            for issue in validate_breakable_geometry(breakable) {
                out.breakable_warnings.push(format!(
                    "BREAKABLE {} / {}: {issue}",
                    dff.name,
                    if component.name.trim().is_empty() {
                        "geometry"
                    } else {
                        component.name.as_str()
                    }
                ));
            }
        }
    }
    out.missing_definition_ids.sort();
    out.missing_definition_ids.dedup();
    out.unused_definitions.sort();
    out.missing_col_attrs.sort();
    out.invalid_col_loads.sort();
    out.invalid_col_loads.dedup();
    out.breakable_warnings.sort();
    out.breakable_warnings.dedup();
    out
}

pub(crate) fn ensure_validation_cache(app: &mut AppState) {
    if app.validation_cache.is_none() {
        refresh_validation_cache(app);
    }
}

pub(crate) fn invalidate_validation_cache(app: &mut AppState) {
    app.validation_cache = None;
}

pub(crate) fn refresh_validation_cache(app: &mut AppState) {
    app.validation_cache = Some(validation_summary(app));
    clamp_validation_list_scroll(app);
}

#[derive(Clone, Debug)]
struct DuplicatePlacementSnapshot {
    index: usize,
    id: String,
    pos: V3,
    rot: V3,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct DuplicatePlacementKey {
    id: String,
    pos: [u32; 3],
    rot: [u32; 3],
}

#[derive(Debug)]
pub(crate) struct DuplicatePlacementScanResult {
    groups: Vec<Vec<usize>>,
}

#[derive(Debug)]
pub(crate) enum DuplicatePlacementScanUpdate {
    Progress { scanned: usize, total: usize },
    Complete(DuplicatePlacementScanResult),
    Failed(String),
}

fn canonical_float_bits(value: f32) -> u32 {
    if value == 0.0 { 0 } else { value.to_bits() }
}

fn canonical_rotation_bits(value: f32) -> u32 {
    if !value.is_finite() {
        return value.to_bits();
    }
    canonical_float_bits(value.rem_euclid(360.0))
}

fn duplicate_placement_key(id: &str, pos: V3, rot: V3) -> Option<DuplicatePlacementKey> {
    let id = id.trim().to_ascii_lowercase();
    if id.is_empty() {
        return None;
    }
    Some(DuplicatePlacementKey {
        id,
        pos: [
            canonical_float_bits(pos.x),
            canonical_float_bits(pos.y),
            canonical_float_bits(pos.z),
        ],
        rot: [
            canonical_rotation_bits(rot.x),
            canonical_rotation_bits(rot.y),
            canonical_rotation_bits(rot.z),
        ],
    })
}

fn find_duplicate_placement_groups(
    snapshots: &[DuplicatePlacementSnapshot],
    progress_tx: Option<&mpsc::Sender<DuplicatePlacementScanUpdate>>,
) -> DuplicatePlacementScanResult {
    let mut by_transform = HashMap::<DuplicatePlacementKey, Vec<usize>>::new();
    for (scanned, snapshot) in snapshots.iter().enumerate() {
        if let Some(key) = duplicate_placement_key(&snapshot.id, snapshot.pos, snapshot.rot) {
            by_transform.entry(key).or_default().push(snapshot.index);
        }
        if (scanned + 1) % 2048 == 0
            && let Some(tx) = progress_tx
        {
            let _ = tx.send(DuplicatePlacementScanUpdate::Progress {
                scanned: scanned + 1,
                total: snapshots.len(),
            });
        }
    }
    let mut groups = by_transform
        .into_values()
        .filter(|indices| indices.len() > 1)
        .collect::<Vec<_>>();
    for group in &mut groups {
        group.sort_unstable();
    }
    groups.sort_unstable_by_key(|group| group[0]);
    DuplicatePlacementScanResult { groups }
}

/// Starts a read-only background scan for live placements that share the same
/// element/definition ID and the same position and orientation. Rotation is
/// canonicalized so equivalent full turns (for example 0 and 360 degrees)
/// compare as the same orientation.
pub(crate) fn request_duplicate_placement_scan(app: &mut AppState) {
    if app.duplicate_placement_scan_rx.is_some() {
        app.status_message = "Duplicate placement scan is already running".to_string();
        return;
    }
    let snapshots = app
        .placements
        .iter()
        .enumerate()
        .filter(|(index, _)| is_live_element(app, *index))
        .map(|(index, placement)| DuplicatePlacementSnapshot {
            index,
            id: placement.id.clone(),
            pos: placement.pos,
            rot: placement.rot,
        })
        .collect::<Vec<_>>();
    let total = snapshots.len();
    let (tx, rx) = mpsc::channel();
    let spawn = thread::Builder::new()
        .name("duplicate-placement-scan".to_string())
        .spawn(move || {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                find_duplicate_placement_groups(&snapshots, Some(&tx))
            }));
            let update = match outcome {
                Ok(result) => DuplicatePlacementScanUpdate::Complete(result),
                Err(_) => DuplicatePlacementScanUpdate::Failed(
                    "Duplicate placement scan crashed".to_string(),
                ),
            };
            let _ = tx.send(update);
        });
    match spawn {
        Ok(_) => {
            app.duplicate_placement_scan_rx = Some(rx);
            app.status_message = format!(
                "Scanning {total} live placements for matching ID, position, and rotation..."
            );
        }
        Err(err) => {
            app.status_message = format!("Could not start duplicate placement scan: {err}");
        }
    }
}

fn current_redundant_duplicate_indices(
    app: &AppState,
    result: &DuplicatePlacementScanResult,
) -> (BTreeSet<usize>, usize) {
    let mut redundant = BTreeSet::new();
    let mut group_count = 0usize;
    for candidates in &result.groups {
        let mut current_groups = HashMap::<DuplicatePlacementKey, Vec<usize>>::new();
        for index in candidates.iter().copied() {
            if !is_live_element(app, index) {
                continue;
            }
            let Some(placement) = app.placements.get(index) else {
                continue;
            };
            if let Some(key) = duplicate_placement_key(&placement.id, placement.pos, placement.rot)
            {
                current_groups.entry(key).or_default().push(index);
            }
        }
        for mut indices in current_groups
            .into_values()
            .filter(|indices| indices.len() > 1)
        {
            indices.sort_unstable();
            group_count += 1;
            redundant.extend(indices.into_iter().skip(1));
        }
    }
    (redundant, group_count)
}

pub(crate) fn poll_duplicate_placement_scan(app: &mut AppState) {
    let mut updates = Vec::new();
    let mut disconnected = false;
    if let Some(rx) = app.duplicate_placement_scan_rx.as_ref() {
        loop {
            match rx.try_recv() {
                Ok(update) => updates.push(update),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    disconnected = true;
                    break;
                }
            }
        }
    }

    let mut finished = false;
    for update in updates {
        match update {
            DuplicatePlacementScanUpdate::Progress { scanned, total } => {
                app.status_message = format!("Scanning duplicate placements... {scanned}/{total}");
            }
            DuplicatePlacementScanUpdate::Complete(result) => {
                let (redundant, group_count) = current_redundant_duplicate_indices(app, &result);
                finished = true;
                if redundant.is_empty() {
                    app.status_message =
                        "No placements share the same ID, position, and rotation".to_string();
                    continue;
                }
                app.selected_elements = redundant;
                app.selected_element_order = app.selected_elements.iter().copied().collect();
                app.selected = app
                    .selected_element_order
                    .last()
                    .copied()
                    .unwrap_or(NO_SELECTION);
                app.selected_group = None;
                app.selected_col_face = None;
                app.status_message = format!(
                    "Found {} overlapping duplicate placement(s) in {group_count} group(s); \
                     selected the redundant copies and kept the first copy of each",
                    app.selected_elements.len()
                );
            }
            DuplicatePlacementScanUpdate::Failed(error) => {
                finished = true;
                app.status_message = error;
            }
        }
    }
    if finished || disconnected {
        app.duplicate_placement_scan_rx = None;
        if disconnected && !finished {
            app.status_message = "Duplicate placement scan stopped unexpectedly".to_string();
        }
    }
}

#[cfg(test)]
mod duplicate_placement_tests {
    use super::*;

    fn snapshot(index: usize, id: &str, pos: V3, rot: V3) -> DuplicatePlacementSnapshot {
        DuplicatePlacementSnapshot {
            index,
            id: id.to_string(),
            pos,
            rot,
        }
    }

    #[test]
    fn duplicate_scan_matches_id_position_and_equivalent_rotation() {
        let snapshots = vec![
            snapshot(
                0,
                "NICE_prop",
                V3 {
                    x: 10.0,
                    y: 20.0,
                    z: 30.0,
                },
                V3::default(),
            ),
            snapshot(
                1,
                "nice_PROP",
                V3 {
                    x: 10.0,
                    y: 20.0,
                    z: 30.0,
                },
                V3 {
                    x: 360.0,
                    y: -0.0,
                    z: -360.0,
                },
            ),
        ];

        let result = find_duplicate_placement_groups(&snapshots, None);
        assert_eq!(result.groups, vec![vec![0, 1]]);
    }

    #[test]
    fn duplicate_scan_requires_both_matching_position_and_rotation() {
        let base = snapshot(0, "prop", V3::default(), V3::default());
        let moved = snapshot(
            1,
            "prop",
            V3 {
                x: 0.001,
                ..V3::default()
            },
            V3::default(),
        );
        let rotated = snapshot(
            2,
            "prop",
            V3::default(),
            V3 {
                z: 0.001,
                ..V3::default()
            },
        );

        let result = find_duplicate_placement_groups(&[base, moved, rotated], None);
        assert!(result.groups.is_empty());
    }

    #[test]
    fn duplicate_scan_reports_each_extra_copy_once() {
        let snapshots = vec![
            snapshot(4, "prop", V3::default(), V3::default()),
            snapshot(7, "prop", V3::default(), V3::default()),
            snapshot(9, "prop", V3::default(), V3::default()),
        ];

        let result = find_duplicate_placement_groups(&snapshots, None);
        assert_eq!(result.groups, vec![vec![4, 7, 9]]);
        assert_eq!(result.groups[0].len() - 1, 2);
    }
}
