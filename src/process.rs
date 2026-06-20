use std::{collections::HashSet, path::Path};

/// Per-derive-attribute information passed to the callback.
pub struct DeriveChange<'a> {
    pub file_path: &'a Path,
    pub line: usize,
    pub old_text: &'a str,
    pub new_text: &'a str,
}

impl DeriveChange<'_> {
    pub fn changed(&self) -> bool {
        self.old_text != self.new_text
    }
}

/// Process one file: call `on_derive` for each derive attribute,
/// write the sorted source in non-check mode, and return whether
/// the file was already sorted (always `true` in non-check mode).
pub fn process(
    file_path: &Path,
    old: String,
    new: String,
    check: bool,
    derive_lines: &HashSet<usize>,
    on_derive: &mut dyn FnMut(DeriveChange),
) -> Result<bool, std::io::Error> {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    // Sort derive lines so the callback sees them in source order.
    let mut sorted_derive_lines: Vec<usize> = derive_lines.iter().copied().collect();
    sorted_derive_lines.sort_unstable();

    // In non-check mode, notify the callback for every derive attribute.
    if !check {
        for &line in &sorted_derive_lines {
            let i = line.saturating_sub(1);
            let old_text = old_lines.get(i).copied().unwrap_or("");
            let new_text = new_lines.get(i).copied().unwrap_or("");
            on_derive(DeriveChange {
                file_path,
                line,
                old_text,
                new_text,
            });
        }
    }

    if !check {
        if new != old {
            write_file(file_path, new)?;
        }
        return Ok(true);
    }

    // Check mode: return whether anything changed.
    if check && new == old {
        return Ok(true);
    }
    Ok(false)
}

fn write_file(file_path: &Path, new: String) -> Result<(), std::io::Error> {
    std::fs::write(file_path, new)
}
