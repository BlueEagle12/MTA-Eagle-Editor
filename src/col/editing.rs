use super::super::*;

fn pending_col_target_path(source_root: &Path, root: &Path, source: &Path) -> PathBuf {
    if let Ok(relative) = source.strip_prefix(source_root) {
        root.join(relative)
    } else {
        source.to_path_buf()
    }
}

pub(crate) fn apply_pending_col_writes(app: &AppState, root: &Path) -> Result<usize, Vec<String>> {
    apply_pending_col_writes_from_parts(&app.root, root, &app.pending_col_writes)
}

pub(crate) fn apply_pending_col_writes_from_parts(
    source_root: &Path,
    root: &Path,
    pending_writes: &HashMap<(PathBuf, u64), u8>,
) -> Result<usize, Vec<String>> {
    if pending_writes.is_empty() {
        return Ok(0);
    }
    let mut grouped: HashMap<PathBuf, Vec<(u64, u8)>> = HashMap::new();
    for ((source, offset), value) in pending_writes {
        grouped
            .entry(pending_col_target_path(source_root, root, source))
            .or_default()
            .push((*offset, *value));
    }
    let mut written = 0usize;
    let mut errors = Vec::new();
    for (path, mut writes) in grouped {
        writes.sort_by_key(|(offset, _)| *offset);
        let Ok(mut file) = fs::OpenOptions::new().write(true).open(&path) else {
            errors.push(format!(
                "{}: failed to open IMG for COL writeback",
                path.display()
            ));
            continue;
        };
        for (offset, value) in writes {
            if let Err(err) = file.seek(SeekFrom::Start(offset)) {
                errors.push(format!("{} @ {offset}: {err}", path.display()));
                continue;
            }
            if let Err(err) = file.write_all(&[value]) {
                errors.push(format!("{} @ {offset}: {err}", path.display()));
                continue;
            }
            written += 1;
        }
    }
    if errors.is_empty() {
        Ok(written)
    } else {
        Err(errors)
    }
}
