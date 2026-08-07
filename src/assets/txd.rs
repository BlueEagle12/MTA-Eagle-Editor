use super::super::*;
use std::cell::RefCell;

// ---------------------------------------------------------------------------
// In-engine TXD texture decoding.
//
// Many resources (e.g. Liberty_City) ship their textures only as undecoded
// RenderWare TXD dictionaries packed inside `*.img` archives, with no
// pre-rendered `txd_build/*.png` folder. We index those archives at load time
// and decode the rasters on demand (DXT1/3/5, raw 32-bit BGRA/BGR, 16-bit
// 565/1555/4444, and 8-bit palettized) straight into GL textures.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TxFormat {
    Dxt1,
    Dxt3,
    Dxt5,
    Bgra8888,
    Bgr888,
    Rgb565,
    Argb1555,
    Argb4444,
    Pal8,
}

pub(crate) type TextureContentFingerprint = [u64; 2];

#[derive(Clone)]
pub(crate) struct TxdTexture {
    pub(crate) txd_name: String,
    pub(crate) img_path: PathBuf,
    pub(crate) native_offset: u64,
    pub(crate) native_size: u32,
    pub(crate) data_offset: u64,
    pub(crate) data_size: u32,
    pub(crate) palette_offset: u64,
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) format: TxFormat,
    /// RenderWare's native-raster flag which determines whether decoded alpha
    /// is meaningful. DXT payloads can contain incidental alpha bits even when
    /// this is false, and must then be rendered as fully opaque.
    pub(crate) has_alpha: bool,
    /// Deterministic fingerprint of the name-independent top-mip payload,
    /// dimensions, format, alpha semantics, and palette (when present).
    pub(crate) content_fingerprint: TextureContentFingerprint,
}

pub(crate) type TxdTextureIndex = HashMap<String, Vec<TxdTexture>>;

pub(crate) fn remove_txd_dictionary_from_index(index: &mut TxdTextureIndex, txd_name: &str) {
    let txd_key = asset_key(txd_name, ".txd");
    index.retain(|_, entries| {
        entries.retain(|entry| !entry.txd_name.eq_ignore_ascii_case(&txd_key));
        !entries.is_empty()
    });
}

pub(crate) fn remove_img_txd_dictionaries_from_index(
    index: &mut TxdTextureIndex,
    entries: &[ImgEntry],
) {
    for entry in entries {
        if lower(&entry.name).ends_with(".txd") {
            remove_txd_dictionary_from_index(index, &entry.name);
        }
    }
}

thread_local! {
    // Texture decoding already visits every alpha byte. Remember the result by
    // GL texture ID so mesh compilation does not decode the same PNG/TXD a
    // second time solely to choose opaque/cutout/blend rendering.
    static LOADED_TEXTURE_TRANSPARENCY: RefCell<HashMap<u32, TransparencyMode>> =
        RefCell::new(HashMap::new());
}

fn remember_loaded_texture_transparency(texture: u32, mode: TransparencyMode) {
    if texture == 0 {
        return;
    }
    LOADED_TEXTURE_TRANSPARENCY.with(|cache| {
        cache.borrow_mut().insert(texture, mode);
    });
}

pub(crate) fn loaded_texture_transparency_mode(
    texture: u32,
    name: &str,
    txd_scope: Option<&str>,
    texture_files: &HashMap<String, PathBuf>,
    txd_textures: &TxdTextureIndex,
    enabled: bool,
) -> TransparencyMode {
    if texture != 0
        && let Some(mode) =
            LOADED_TEXTURE_TRANSPARENCY.with(|cache| cache.borrow().get(&texture).copied())
    {
        return mode;
    }
    let mode = texture_transparency_mode(name, txd_scope, texture_files, txd_textures, enabled);
    remember_loaded_texture_transparency(texture, mode);
    mode
}

fn texture_fingerprint_chunks(chunks: &[&[u8]]) -> TextureContentFingerprint {
    let mut a = 0xcbf2_9ce4_8422_2325u64;
    let mut b = 0x9e37_79b9_7f4a_7c15u64;
    let mut index = 0u64;
    for chunk in chunks {
        for value in chunk.iter().copied() {
            a ^= value as u64;
            a = a.wrapping_mul(0x1000_0000_01b3);
            b ^= (value as u64).wrapping_add(index.rotate_left(17));
            b = b.rotate_left(9).wrapping_mul(0x9ddf_ea08_eb38_2d69);
            index = index.wrapping_add(1);
        }
    }
    [a, b]
}

fn native_texture_content_fingerprint(
    width: u16,
    height: u16,
    format: TxFormat,
    has_alpha: bool,
    data: &[u8],
    palette: &[u8],
) -> TextureContentFingerprint {
    let metadata = [
        width.to_le_bytes()[0],
        width.to_le_bytes()[1],
        height.to_le_bytes()[0],
        height.to_le_bytes()[1],
        format as u8,
        u8::from(has_alpha),
    ];
    texture_fingerprint_chunks(&[&metadata, palette, data])
}

pub(crate) fn texture_content_fingerprint(
    index: &TxdTextureIndex,
    texture_name: &str,
    txd_name: Option<&str>,
) -> Option<TextureContentFingerprint> {
    let texture_key = lower(texture_name.trim());
    let txd_key = txd_name.map(|name| asset_key(name, ".txd"));
    index.get(&texture_key).and_then(|entries| {
        txd_key
            .as_ref()
            .and_then(|txd| {
                entries
                    .iter()
                    .find(|entry| entry.txd_name.eq_ignore_ascii_case(txd))
            })
            .or_else(|| entries.first())
            .map(|entry| entry.content_fingerprint)
    })
}

/// The D3D native-raster alpha field uses bit 0 to signal meaningful alpha.
/// RenderWare writes `8` for opaque textures and `9` for alpha textures.
fn native_alpha_enabled(flags: u8) -> bool {
    flags & 1 != 0
}

/// Scan `.img` archives for RenderWare TXD dictionaries and record the location
/// and format of every texture native so it can be decoded lazily.
/// Index every TXD dictionary in a single `.img` archive into `map`.
pub(crate) fn index_txd_file(path: &Path, map: &mut TxdTextureIndex) {
    let entries = parse_img(path);
    index_txd_entries(path, &entries, map);
}

pub(crate) fn index_txd_entries(path: &Path, entries: &[ImgEntry], map: &mut TxdTextureIndex) {
    let txd_entries = entries
        .iter()
        .filter(|entry| lower(&entry.name).ends_with(".txd"))
        .collect::<Vec<_>>();
    if txd_entries.is_empty() {
        return;
    }

    let available = thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1);
    let workers = available.saturating_sub(1).max(1).min(txd_entries.len());
    let chunk_size = txd_entries.len().div_ceil(workers);
    let (tx, rx) = mpsc::channel();
    thread::scope(|scope| {
        for (chunk_index, chunk) in txd_entries.chunks(chunk_size).enumerate() {
            let tx = tx.clone();
            scope.spawn(move || {
                let mut indexed = TxdTextureIndex::new();
                let Ok(mut file) = fs::File::open(path) else {
                    let _ = tx.send((chunk_index, indexed));
                    return;
                };
                for entry in chunk {
                    // Read only the TXD member rather than the complete IMG
                    // archive. Fingerprinting then runs concurrently per chunk.
                    let bytes = read_img_entry_from(&mut file, entry);
                    let name = lower(&entry.name);
                    let mut member_index = TxdTextureIndex::new();
                    index_one_txd(&bytes, 0, bytes.len(), path, &name, &mut member_index);
                    adjust_and_merge_txd_index(&mut indexed, member_index, entry.offset as u64);
                }
                let _ = tx.send((chunk_index, indexed));
            });
        }
    });
    drop(tx);

    let mut chunks = rx.into_iter().collect::<BTreeMap<_, _>>();
    for indexed in chunks.values_mut() {
        adjust_and_merge_txd_index(map, std::mem::take(indexed), 0);
    }
}

fn adjust_and_merge_txd_index(
    destination_index: &mut TxdTextureIndex,
    source_index: TxdTextureIndex,
    base_offset: u64,
) {
    for (texture_name, entries) in source_index {
        let destination = destination_index.entry(texture_name).or_default();
        for mut texture in entries {
            texture.native_offset += base_offset;
            texture.data_offset += base_offset;
            if texture.palette_offset > 0 {
                texture.palette_offset += base_offset;
            }
            if !destination
                .iter()
                .any(|existing| existing.txd_name.eq_ignore_ascii_case(&texture.txd_name))
            {
                destination.push(texture);
            }
        }
    }
}

pub(crate) fn index_standalone_txd_file(path: &Path, map: &mut TxdTextureIndex) {
    let Ok(bytes) = fs::read(path) else {
        return;
    };
    let txd_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| asset_key(name, ".txd"))
        .unwrap_or_default();
    index_one_txd(&bytes, 0, bytes.len(), path, &txd_name, map);
}

/// Parse a single TXD region (already located inside the full archive buffer)
/// and register each texture native it contains.
pub(crate) fn index_one_txd(
    b: &[u8],
    start: usize,
    end: usize,
    path: &Path,
    txd_name: &str,
    map: &mut TxdTextureIndex,
) {
    if start + 12 > end || end > b.len() {
        return;
    }
    // Top chunk must be Texture Dictionary (0x16).
    if rd32(b, start) != 0x16 {
        return;
    }
    let dict_size = rd32(b, start + 4) as usize;
    let dict_start = start + 12;
    let dict_end = (dict_start + dict_size).min(end);
    let mut o = dict_start;
    while o + 12 <= dict_end {
        let cid = rd32(b, o);
        let csz = rd32(b, o + 4) as usize;
        let cs = o + 12;
        let ce = cs.saturating_add(csz);
        if ce > dict_end {
            break;
        }
        if cid == 0x15 {
            index_texture_native(b, o, ce, path, txd_name, map);
        }
        o = ce;
    }
}

/// Parse the struct (0x01) inside a Texture Native (0x15) chunk and record the
/// top-mip location + decoded format.
pub(crate) fn index_texture_native(
    b: &[u8],
    native_start: usize,
    end: usize,
    path: &Path,
    txd_name: &str,
    map: &mut TxdTextureIndex,
) {
    // Find the 0x01 struct child.
    let mut o = native_start + 12;
    while o + 12 <= end {
        let cid = rd32(b, o);
        let csz = rd32(b, o + 4) as usize;
        let cs = o + 12;
        let ce = cs.saturating_add(csz);
        if ce > end {
            break;
        }
        if cid != 0x01 {
            o = ce;
            continue;
        }
        // Parse the D3D8/9 texture-native header.
        let mut p = cs;
        if p + 88 > ce {
            return;
        }
        p += 4; // platformId
        p += 4; // filter / addressing flags
        let name_bytes = &b[p..p + 32];
        let nend = name_bytes.iter().position(|v| *v == 0).unwrap_or(32);
        let tex_name = lower(&String::from_utf8_lossy(&name_bytes[..nend]));
        p += 32; // name
        p += 32; // mask name
        let raster_format = rd32(b, p);
        p += 4;
        let fourcc = &b[p..p + 4];
        p += 4;
        let width = rd16(b, p);
        let height = rd16(b, p + 2);
        p += 4;
        let depth = b[p];
        let _levels = b[p + 1];
        let _rtype = b[p + 2];
        let has_alpha = native_alpha_enabled(b[p + 3]);
        p += 4; // depth, levels, rtype, compression

        let is_pal8 = raster_format & 0x2000 != 0;
        let is_pal4 = raster_format & 0x4000 != 0;
        let mut palette_offset = 0u64;
        if is_pal8 {
            palette_offset = p as u64;
            p += 256 * 4;
        } else if is_pal4 {
            // PAL4 unsupported for now; skip its palette and bail out.
            return;
        }
        if p + 4 > ce {
            return;
        }
        let data_size = rd32(b, p);
        p += 4;
        let data_offset = p as u64;
        if data_offset as usize + data_size as usize > b.len() {
            return;
        }

        let format = if fourcc == b"DXT1" {
            TxFormat::Dxt1
        } else if fourcc == b"DXT3" {
            TxFormat::Dxt3
        } else if fourcc == b"DXT5" {
            TxFormat::Dxt5
        } else if is_pal8 {
            TxFormat::Pal8
        } else if depth == 32 {
            if raster_format & 0xF00 == 0x600 {
                TxFormat::Bgr888
            } else {
                TxFormat::Bgra8888
            }
        } else if depth == 16 {
            match raster_format & 0xF00 {
                0x200 => TxFormat::Rgb565,
                0x300 => TxFormat::Argb4444,
                _ => TxFormat::Argb1555,
            }
        } else {
            return;
        };

        if !tex_name.is_empty() && width > 0 && height > 0 {
            let entries = map.entry(tex_name).or_default();
            if entries
                .iter()
                .any(|entry| entry.txd_name.eq_ignore_ascii_case(txd_name))
            {
                return;
            }
            let data_start = data_offset as usize;
            let data_end = data_start + data_size as usize;
            let data = &b[data_start..data_end];
            let palette = if palette_offset > 0 {
                let start = palette_offset as usize;
                b.get(start..start.saturating_add(256 * 4))
                    .unwrap_or_default()
            } else {
                &[]
            };
            let content_fingerprint =
                native_texture_content_fingerprint(width, height, format, has_alpha, data, palette);
            entries.push(TxdTexture {
                txd_name: txd_name.to_string(),
                img_path: path.to_path_buf(),
                native_offset: native_start as u64,
                native_size: end.saturating_sub(native_start) as u32,
                data_offset,
                data_size,
                palette_offset,
                width,
                height,
                format,
                has_alpha,
                content_fingerprint,
            });
        }
        return;
    }
}

pub(crate) fn rgb565_to_rgb(v: u16) -> (u8, u8, u8) {
    let r = ((v >> 11) & 31) as u32;
    let g = ((v >> 5) & 63) as u32;
    let b = (v & 31) as u32;
    (
        (r * 255 / 31) as u8,
        (g * 255 / 63) as u8,
        (b * 255 / 31) as u8,
    )
}

/// Decode one 4x4 DXT color block into 16 RGBA texels (row-major). `dxt1`
/// enables the 1-bit punch-through alpha mode.
pub(crate) fn decode_dxt_color_block(blk: &[u8], dxt1: bool, out: &mut [[u8; 4]; 16]) {
    let c0 = u16::from_le_bytes([blk[0], blk[1]]);
    let c1 = u16::from_le_bytes([blk[2], blk[3]]);
    let bits = u32::from_le_bytes([blk[4], blk[5], blk[6], blk[7]]);
    let (r0, g0, b0) = rgb565_to_rgb(c0);
    let (r1, g1, b1) = rgb565_to_rgb(c1);
    let mut col = [[0u8; 4]; 4];
    col[0] = [r0, g0, b0, 255];
    col[1] = [r1, g1, b1, 255];
    if !dxt1 || c0 > c1 {
        col[2] = [
            ((2 * r0 as u32 + r1 as u32) / 3) as u8,
            ((2 * g0 as u32 + g1 as u32) / 3) as u8,
            ((2 * b0 as u32 + b1 as u32) / 3) as u8,
            255,
        ];
        col[3] = [
            ((r0 as u32 + 2 * r1 as u32) / 3) as u8,
            ((g0 as u32 + 2 * g1 as u32) / 3) as u8,
            ((b0 as u32 + 2 * b1 as u32) / 3) as u8,
            255,
        ];
    } else {
        col[2] = [
            ((r0 as u32 + r1 as u32) / 2) as u8,
            ((g0 as u32 + g1 as u32) / 2) as u8,
            ((b0 as u32 + b1 as u32) / 2) as u8,
            255,
        ];
        col[3] = [0, 0, 0, 0];
    }
    for i in 0..16 {
        out[i] = col[((bits >> (2 * i)) & 3) as usize];
    }
}

/// Decode a DXT1/3/5 surface to RGBA. `mode` is 1, 3 or 5.
pub(crate) fn decode_dxt(data: &[u8], w: usize, h: usize, mode: u8) -> Vec<u8> {
    let mut out = vec![0u8; w * h * 4];
    let bx = (w + 3) / 4;
    let by = (h + 3) / 4;
    let block_bytes = if mode == 1 { 8 } else { 16 };
    let mut o = 0usize;
    let mut colors = [[0u8; 4]; 16];
    for cy in 0..by {
        for cx in 0..bx {
            if o + block_bytes > data.len() {
                return out;
            }
            let mut alpha = [255u8; 16];
            if mode == 1 {
                decode_dxt_color_block(&data[o..o + 8], true, &mut colors);
                for i in 0..16 {
                    alpha[i] = colors[i][3];
                }
                o += 8;
            } else {
                let ablk = &data[o..o + 8];
                if mode == 3 {
                    let abits = u64::from_le_bytes(ablk.try_into().unwrap());
                    for i in 0..16 {
                        alpha[i] = (((abits >> (4 * i)) & 0xF) as u8) * 17;
                    }
                } else {
                    let a0 = ablk[0];
                    let a1 = ablk[1];
                    let mut al = [0u8; 8];
                    al[0] = a0;
                    al[1] = a1;
                    if a0 > a1 {
                        for k in 1..7 {
                            al[1 + k] =
                                (((7 - k) as u32 * a0 as u32 + k as u32 * a1 as u32) / 7) as u8;
                        }
                    } else {
                        for k in 1..5 {
                            al[1 + k] =
                                (((5 - k) as u32 * a0 as u32 + k as u32 * a1 as u32) / 5) as u8;
                        }
                        al[6] = 0;
                        al[7] = 255;
                    }
                    let idxbits = u64::from_le_bytes(ablk.try_into().unwrap()) >> 16;
                    for i in 0..16 {
                        alpha[i] = al[((idxbits >> (3 * i)) & 7) as usize];
                    }
                }
                decode_dxt_color_block(&data[o + 8..o + 16], false, &mut colors);
                o += 16;
            }
            for i in 0..16 {
                let px = cx * 4 + (i % 4);
                let py = cy * 4 + (i / 4);
                if px >= w || py >= h {
                    continue;
                }
                let d = (py * w + px) * 4;
                out[d] = colors[i][0];
                out[d + 1] = colors[i][1];
                out[d + 2] = colors[i][2];
                out[d + 3] = if mode == 1 { colors[i][3] } else { alpha[i] };
            }
        }
    }
    out
}

/// Read and decode a TXD texture's top mip into an RGBA buffer.
pub(crate) fn decode_txd_texture(tex: &TxdTexture) -> Option<(u32, u32, Vec<u8>)> {
    let Ok(mut file) = fs::File::open(&tex.img_path) else {
        return None;
    };
    let mut data = vec![0u8; tex.data_size as usize];
    if file.seek(SeekFrom::Start(tex.data_offset)).is_err() {
        return None;
    }
    if file.read_exact(&mut data).is_err() {
        return None;
    }
    let palette = if matches!(tex.format, TxFormat::Pal8) {
        let mut pal = vec![0u8; 256 * 4];
        if file.seek(SeekFrom::Start(tex.palette_offset)).is_err()
            || file.read_exact(&mut pal).is_err()
        {
            return None;
        }
        Some(pal)
    } else {
        None
    };
    decode_txd_rgba(tex, &data, palette.as_deref())
}

/// Decode a texture indexed from an in-memory TXD buffer.
pub(crate) fn decode_txd_texture_from_bytes(
    tex: &TxdTexture,
    bytes: &[u8],
) -> Option<(u32, u32, Vec<u8>)> {
    let data_start = tex.data_offset as usize;
    let data_end = data_start.checked_add(tex.data_size as usize)?;
    let data = bytes.get(data_start..data_end)?;
    let palette = if matches!(tex.format, TxFormat::Pal8) {
        let pal_start = tex.palette_offset as usize;
        let pal_end = pal_start.checked_add(256 * 4)?;
        Some(bytes.get(pal_start..pal_end)?)
    } else {
        None
    };
    decode_txd_rgba(tex, data, palette)
}

fn decode_txd_rgba(
    tex: &TxdTexture,
    data: &[u8],
    palette: Option<&[u8]>,
) -> Option<(u32, u32, Vec<u8>)> {
    let w = tex.width as usize;
    let h = tex.height as usize;
    let mut rgba = match tex.format {
        TxFormat::Dxt1 => decode_dxt(&data, w, h, 1),
        TxFormat::Dxt3 => decode_dxt(&data, w, h, 3),
        TxFormat::Dxt5 => decode_dxt(&data, w, h, 5),
        TxFormat::Bgra8888 | TxFormat::Bgr888 => {
            let force_opaque = matches!(tex.format, TxFormat::Bgr888);
            let mut out = vec![0u8; w * h * 4];
            for i in 0..(w * h) {
                if i * 4 + 3 >= data.len() {
                    break;
                }
                out[i * 4] = data[i * 4 + 2];
                out[i * 4 + 1] = data[i * 4 + 1];
                out[i * 4 + 2] = data[i * 4];
                out[i * 4 + 3] = if force_opaque { 255 } else { data[i * 4 + 3] };
            }
            out
        }
        TxFormat::Rgb565 => {
            let mut out = vec![0u8; w * h * 4];
            for i in 0..(w * h) {
                if i * 2 + 1 >= data.len() {
                    break;
                }
                let v = u16::from_le_bytes([data[i * 2], data[i * 2 + 1]]);
                let (r, g, b) = rgb565_to_rgb(v);
                out[i * 4] = r;
                out[i * 4 + 1] = g;
                out[i * 4 + 2] = b;
                out[i * 4 + 3] = 255;
            }
            out
        }
        TxFormat::Argb1555 => {
            let mut out = vec![0u8; w * h * 4];
            for i in 0..(w * h) {
                if i * 2 + 1 >= data.len() {
                    break;
                }
                let v = u16::from_le_bytes([data[i * 2], data[i * 2 + 1]]);
                let r = ((v >> 10) & 31) as u32;
                let g = ((v >> 5) & 31) as u32;
                let b = (v & 31) as u32;
                out[i * 4] = (r * 255 / 31) as u8;
                out[i * 4 + 1] = (g * 255 / 31) as u8;
                out[i * 4 + 2] = (b * 255 / 31) as u8;
                out[i * 4 + 3] = if v & 0x8000 != 0 { 255 } else { 0 };
            }
            out
        }
        TxFormat::Argb4444 => {
            let mut out = vec![0u8; w * h * 4];
            for i in 0..(w * h) {
                if i * 2 + 1 >= data.len() {
                    break;
                }
                let v = u16::from_le_bytes([data[i * 2], data[i * 2 + 1]]);
                let a = ((v >> 12) & 0xF) as u8;
                let r = ((v >> 8) & 0xF) as u8;
                let g = ((v >> 4) & 0xF) as u8;
                let b = (v & 0xF) as u8;
                out[i * 4] = r * 17;
                out[i * 4 + 1] = g * 17;
                out[i * 4 + 2] = b * 17;
                out[i * 4 + 3] = a * 17;
            }
            out
        }
        TxFormat::Pal8 => {
            let pal = palette?;
            let mut out = vec![0u8; w * h * 4];
            for i in 0..(w * h) {
                if i >= data.len() {
                    break;
                }
                let idx = data[i] as usize * 4;
                // Palette entries are stored RGBA in SA TXDs.
                out[i * 4] = pal[idx];
                out[i * 4 + 1] = pal[idx + 1];
                out[i * 4 + 2] = pal[idx + 2];
                out[i * 4 + 3] = pal[idx + 3];
            }
            out
        }
    };
    // RenderWare distinguishes a texture whose pixels happen to carry alpha
    // data from one whose alpha is actually enabled. In particular, opaque
    // DXT1 textures use the same 3-color encoding that the generic decoder
    // would otherwise interpret as punch-through transparency.
    if !tex.has_alpha {
        for pixel in rgba.chunks_exact_mut(4) {
            pixel[3] = 255;
        }
    }
    Some((w as u32, h as u32, rgba))
}

/// Upload an RGBA buffer as a GL texture and return its id (0 on failure).
pub(crate) fn upload_rgba_texture(width: u32, height: u32, rgba: &[u8]) -> u32 {
    if width == 0 || height == 0 || rgba.len() < (width * height * 4) as usize {
        return 0;
    }
    let mut texture_id = 0u32;
    unsafe {
        gl::GenTextures(1, &mut texture_id);
        gl::BindTexture(gl::TEXTURE_2D, texture_id);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA as i32,
            width as i32,
            height as i32,
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            rgba.as_ptr().cast(),
        );
        if gl::GenerateMipmap::is_loaded() {
            gl::GenerateMipmap(gl::TEXTURE_2D);
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MIN_FILTER,
                gl::LINEAR_MIPMAP_LINEAR as i32,
            );
        } else {
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        }
        gl::BindTexture(gl::TEXTURE_2D, 0);
    }
    texture_id
}

pub(crate) const MISSING_TEXTURE_KEY: &str = "missing_texture";

pub(crate) fn load_missing_texture_cached(textures: &mut HashMap<String, u32>) -> u32 {
    if let Some(texture_id) = textures.get(MISSING_TEXTURE_KEY) {
        return *texture_id;
    }
    let texture_id = image::load_from_memory(include_bytes!("../../assets/missing_texture.png"))
        .map(|image| {
            let rgba = image.to_rgba8();
            let (width, height) = rgba.dimensions();
            upload_rgba_texture(width, height, rgba.as_raw())
        })
        .unwrap_or(0);
    textures.insert(MISSING_TEXTURE_KEY.to_string(), texture_id);
    texture_id
}

fn rgba_transparency_mode(rgba: &[u8]) -> TransparencyMode {
    // Texture conversion and DXT/palette round-trips commonly turn nominally
    // opaque alpha (255) into 254. Treat that tiny quantization error as opaque;
    // otherwise one 254 texel promotes an entire DFF material to the blend pass
    // and disables its depth writes. Values near zero receive the same tolerance
    // for binary cutout textures.
    const TRANSPARENT_MAX: u8 = 5;
    const OPAQUE_MIN: u8 = 250;
    let mut has_zero = false;
    let mut has_partial = false;
    for px in rgba.chunks_exact(4) {
        match px[3] {
            OPAQUE_MIN..=255 => {}
            0..=TRANSPARENT_MAX => has_zero = true,
            _ => has_partial = true,
        }
        if has_partial {
            return TransparencyMode::Blend;
        }
    }
    if has_zero {
        TransparencyMode::Cutout
    } else {
        TransparencyMode::Opaque
    }
}

pub(crate) fn texture_transparency_mode(
    name: &str,
    txd_scope: Option<&str>,
    texture_files: &HashMap<String, PathBuf>,
    txd_textures: &TxdTextureIndex,
    enabled: bool,
) -> TransparencyMode {
    if !enabled {
        return TransparencyMode::Opaque;
    }
    let name = name.trim();
    if name.is_empty() {
        return TransparencyMode::Opaque;
    }
    let key = lower(name);
    let txd_key = txd_scope
        .map(|txd| asset_key(txd, ".txd"))
        .filter(|txd| !txd.is_empty());
    if txd_key.is_none() {
        if let Some(path) = texture_files.get(&key) {
            if let Ok(bytes) = fs::read(path) {
                if let Ok(image) = image::load_from_memory(&bytes) {
                    let rgba = image.to_rgba8();
                    return rgba_transparency_mode(rgba.as_raw());
                }
            }
        }
    }
    let tex = txd_textures.get(&key).and_then(|entries| {
        if let Some(txd_key) = &txd_key {
            entries
                .iter()
                .find(|entry| entry.txd_name.eq_ignore_ascii_case(txd_key))
        } else {
            entries.first()
        }
    });
    tex.and_then(decode_txd_texture)
        .map(|(_, _, rgba)| rgba_transparency_mode(&rgba))
        .unwrap_or(TransparencyMode::Opaque)
}

pub(crate) fn texture_source_exists(
    name: &str,
    txd_scope: Option<&str>,
    texture_files: &HashMap<String, PathBuf>,
    txd_textures: &TxdTextureIndex,
) -> bool {
    let name = name.trim();
    if name.is_empty() {
        return false;
    }
    let key = lower(name);
    let txd_key = txd_scope
        .map(|txd| asset_key(txd, ".txd"))
        .filter(|txd| !txd.is_empty());
    if txd_key.is_none() && texture_files.contains_key(&key) {
        return true;
    }
    txd_textures.get(&key).is_some_and(|entries| {
        if let Some(txd_key) = &txd_key {
            entries
                .iter()
                .any(|entry| entry.txd_name.eq_ignore_ascii_case(txd_key))
        } else {
            !entries.is_empty()
        }
    })
}

pub(crate) fn load_texture_cached(
    name: &str,
    txd_scope: Option<&str>,
    texture_files: &HashMap<String, PathBuf>,
    txd_textures: &TxdTextureIndex,
    textures: &mut HashMap<String, u32>,
    enabled: bool,
) -> u32 {
    if !enabled {
        return 0;
    }
    let name = name.trim();
    if name.is_empty() {
        return load_missing_texture_cached(textures);
    }
    let key = lower(name);
    let txd_key = txd_scope
        .map(|txd| asset_key(txd, ".txd"))
        .filter(|txd| !txd.is_empty());
    let cache_key = if let Some(txd_key) = &txd_key {
        format!("{txd_key}|{key}")
    } else {
        key.clone()
    };
    if let Some(tex) = textures.get(&cache_key) {
        return *tex;
    }
    // Prefer a pre-rendered PNG (txd_build/) when present.
    if txd_key.is_none() {
        if let Some(path) = texture_files.get(&key) {
            if let Ok(bytes) = fs::read(path) {
                if let Ok(image) = image::load_from_memory(&bytes) {
                    let rgba = image.to_rgba8();
                    let (width, height) = rgba.dimensions();
                    let transparency = rgba_transparency_mode(rgba.as_raw());
                    let texture_id = upload_rgba_texture(width, height, rgba.as_raw());
                    remember_loaded_texture_transparency(texture_id, transparency);
                    textures.insert(cache_key, texture_id);
                    return texture_id;
                }
            }
        }
    }
    // Otherwise decode straight from an indexed TXD archive.
    let tex = txd_textures.get(&key).and_then(|entries| {
        if let Some(txd_key) = &txd_key {
            entries
                .iter()
                .find(|entry| entry.txd_name.eq_ignore_ascii_case(txd_key))
        } else {
            entries.first()
        }
    });
    if let Some(tex) = tex {
        if let Some((width, height, rgba)) = decode_txd_texture(tex) {
            let transparency = rgba_transparency_mode(&rgba);
            let texture_id = upload_rgba_texture(width, height, &rgba);
            remember_loaded_texture_transparency(texture_id, transparency);
            textures.insert(cache_key, texture_id);
            return texture_id;
        }
    }
    let texture_id = load_missing_texture_cached(textures);
    remember_loaded_texture_transparency(texture_id, TransparencyMode::Opaque);
    textures.insert(cache_key, texture_id);
    texture_id
}

pub(crate) fn txd_name_from_folder(source_dir: &Path) -> Result<String, String> {
    if !source_dir.is_dir() {
        return Err(format!("{} is not a readable folder", source_dir.display()));
    }
    let folder_name = source_dir
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .ok_or_else(|| "The selected folder has no usable name".to_string())?;
    let txd_stem = if Path::new(folder_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("txd"))
    {
        Path::new(folder_name)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or(folder_name)
    } else {
        folder_name
    };
    if txd_stem.trim().is_empty() {
        return Err("The selected folder has no usable TXD name".to_string());
    }
    Ok(format!("{txd_stem}.txd"))
}

/// Build the RenderWare TXD bytes shared by standalone and IMG destinations.
pub(crate) fn build_txd_from_folder(source_dir: &Path) -> Result<(String, Vec<u8>), String> {
    let txd_name = txd_name_from_folder(source_dir)?;

    let mut images: Vec<PathBuf> = WalkDir::new(source_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
        })
        .collect();
    images.sort();
    if images.is_empty() {
        return Err(format!(
            "{} does not contain any PNG textures",
            source_dir.display()
        ));
    }

    let mut texture_paths: HashMap<String, PathBuf> = HashMap::new();
    let mut txd = rw_chunk(0x16, rw_chunk(0x01, vec![0, 0, 0, 0]));
    for path in &images {
        let texture_name = texture_name_from_path(path);
        let texture_key = texture_name.to_ascii_lowercase();
        if let Some(first_path) = texture_paths.get(&texture_key) {
            let first_display = first_path
                .strip_prefix(source_dir)
                .unwrap_or(first_path)
                .display();
            let duplicate_display = path.strip_prefix(source_dir).unwrap_or(path).display();
            return Err(format!(
                "Multiple PNG files produce the texture name '{texture_name}':\n  {first_display}\n  {duplicate_display}\nRename one of these files and try again."
            ));
        }
        texture_paths.insert(texture_key, path.clone());
        let native = imported_image_texture_native(path, &texture_name)?;
        txd = append_texture_native_to_txd(txd, &native, &texture_name)?;
    }
    Ok((txd_name, txd))
}

/// Write a folder-built TXD as a loose file in the project's `textures/` dir.
pub(crate) fn generate_txd_from_folder(
    project_root: &Path,
    source_dir: &Path,
) -> Result<PathBuf, String> {
    let (txd_name, txd) = build_txd_from_folder(source_dir)?;
    let texture_dir = project_root.join("textures");
    fs::create_dir_all(&texture_dir)
        .map_err(|err| format!("Could not create {}: {err}", texture_dir.display()))?;
    let output = texture_dir.join(&txd_name);
    let temporary = texture_dir.join(format!(".{txd_name}.{}.tmp", std::process::id()));
    fs::write(&temporary, txd)
        .map_err(|err| format!("Could not write {}: {err}", temporary.display()))?;
    if let Err(err) = fs::rename(&temporary, &output) {
        let _ = fs::remove_file(&temporary);
        return Err(format!("Could not write {}: {err}", output.display()));
    }
    Ok(output)
}

#[cfg(test)]
mod generation_tests {
    use super::*;

    fn dxt1_test_texture(has_alpha: bool) -> TxdTexture {
        TxdTexture {
            txd_name: "test.txd".to_string(),
            img_path: PathBuf::new(),
            native_offset: 0,
            native_size: 0,
            data_offset: 0,
            data_size: 8,
            palette_offset: 0,
            width: 4,
            height: 4,
            format: TxFormat::Dxt1,
            has_alpha,
            content_fingerprint: [0, 0],
        }
    }

    fn test_dir(label: &str) -> PathBuf {
        env::temp_dir().join(format!(
            "eagle_txd_folder_{label}_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn write_png(path: &Path, rgba: [u8; 4]) {
        let image = image::RgbaImage::from_pixel(4, 4, image::Rgba(rgba));
        image.save(path).unwrap();
    }

    #[test]
    fn ignores_dxt1_punch_through_bits_when_native_alpha_is_disabled() {
        // Equal DXT1 endpoints select the three-color mode. Index 3 is the
        // transparent entry, but it is only meaningful when the TXD native
        // header explicitly enables alpha.
        let dxt1_punch_through = [0, 0, 0, 0, 0xff, 0xff, 0xff, 0xff];
        let (_, _, opaque) =
            decode_txd_texture_from_bytes(&dxt1_test_texture(false), &dxt1_punch_through).unwrap();
        assert!(opaque.chunks_exact(4).all(|pixel| pixel[3] == 255));

        let (_, _, transparent) =
            decode_txd_texture_from_bytes(&dxt1_test_texture(true), &dxt1_punch_through).unwrap();
        assert!(transparent.chunks_exact(4).all(|pixel| pixel[3] == 0));
    }

    #[test]
    fn native_alpha_bit_distinguishes_opaque_and_alpha_textures() {
        assert!(!native_alpha_enabled(8));
        assert!(native_alpha_enabled(9));
    }

    #[test]
    fn transparency_classification_ignores_alpha_quantization_noise() {
        // MON1_E1019_R0152 contains wall textures whose decoded alpha is only
        // 254/255. They are opaque, not blended materials.
        let quantized_opaque = [20, 30, 40, 254, 50, 60, 70, 255];
        assert_eq!(
            rgba_transparency_mode(&quantized_opaque),
            TransparencyMode::Opaque
        );

        let real_translucency = [20, 30, 40, 194, 50, 60, 70, 255];
        assert_eq!(
            rgba_transparency_mode(&real_translucency),
            TransparencyMode::Blend
        );

        let binary_cutout = [20, 30, 40, 0, 50, 60, 70, 255];
        assert_eq!(
            rgba_transparency_mode(&binary_cutout),
            TransparencyMode::Cutout
        );
    }

    #[test]
    fn generates_named_txd_from_nested_png_folder() {
        let root = test_dir("generate");
        let source = root.join("source").join("city_textures");
        fs::create_dir_all(source.join("nested")).unwrap();
        write_png(&source.join("road.png"), [128, 128, 128, 255]);
        write_png(
            &source.join("nested").join("glass.PNG"),
            [80, 120, 160, 128],
        );

        let output = generate_txd_from_folder(&root.join("project"), &source).unwrap();
        assert_eq!(output.file_name().unwrap(), "city_textures.txd");
        let bytes = fs::read(&output).unwrap();
        assert_eq!(txd_texture_count(&bytes, 12, bytes.len()), Some(2));
        assert!(txd_contains_texture_native(&bytes, "road"));
        assert!(txd_contains_texture_native(&bytes, "glass"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_duplicate_sanitized_texture_names() {
        let root = test_dir("duplicate");
        let source = root.join("duplicate_textures");
        fs::create_dir_all(&source).unwrap();
        write_png(&source.join("brick wall.png"), [255, 0, 0, 255]);
        write_png(&source.join("brick_wall.png"), [0, 255, 0, 255]);

        let err = generate_txd_from_folder(&root.join("project"), &source).unwrap_err();
        assert!(err.contains("produce the texture name 'brick_wall'"));
        assert!(err.contains("brick wall.png"));
        assert!(err.contains("brick_wall.png"));
        assert!(
            !root
                .join("project/textures/duplicate_textures.txd")
                .exists()
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_32_character_names_that_collide_after_safe_truncation() {
        let root = test_dir("long_names");
        let source = root.join("long_name_textures");
        fs::create_dir_all(&source).unwrap();
        let first = "monaco1_stunt21_v1_material_0001";
        let second = "monaco1_stunt21_v1_material_0002";
        assert_eq!(first.len(), 32);
        assert_eq!(second.len(), 32);
        write_png(&source.join(format!("{first}.png")), [255, 0, 0, 255]);
        write_png(&source.join(format!("{second}.png")), [0, 255, 0, 255]);

        let err = generate_txd_from_folder(&root.join("project"), &source).unwrap_err();
        assert!(err.contains("Multiple PNG files produce the texture name"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn removing_a_txd_dictionary_keeps_other_same_name_texture_sources() {
        let texture = |txd_name: &str, path: &str, fingerprint: u64| TxdTexture {
            txd_name: txd_name.to_string(),
            img_path: PathBuf::from(path),
            native_offset: 0,
            native_size: 128,
            data_offset: 64,
            data_size: 64,
            palette_offset: 0,
            width: 4,
            height: 4,
            format: TxFormat::Dxt1,
            has_alpha: false,
            content_fingerprint: [fingerprint, fingerprint],
        };
        let mut index = TxdTextureIndex::from([(
            "road".to_string(),
            vec![
                texture("starroad.txd", "base.img", 1),
                texture("generic.txd", "base.img", 2),
            ],
        )]);

        remove_txd_dictionary_from_index(&mut index, "starroad");

        let remaining = index.get("road").unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].txd_name, "generic.txd");
    }
}
