use std::path::Path;

use console::Style;
use similar::{ChangeTag, TextDiff};

#[derive(Debug, Clone, Copy)]
pub enum OutputColor {
    Auto,
    Always,
    Never,
}

pub fn process(
    file_path: &Path,
    old_lines: Vec<String>,
    new_lines: Vec<String>,
    check: bool,
    output_color: OutputColor,
) -> Result<bool, std::io::Error> {
    if !check {
        write_file(file_path, new_lines)?;
        return Ok(true);
    }

    let diffs = calc_diff_lines(file_path, old_lines, new_lines, output_color);
    if diffs.is_empty() {
        return Ok(true);
    }

    for line in diffs {
        print!("{line}");
    }

    Ok(false)
}

fn write_file(file_path: &Path, new_lines: Vec<String>) -> Result<(), std::io::Error> {
    std::fs::write(file_path, new_lines.concat())
}

fn calc_diff_lines(
    file_path: &Path,
    old_lines: Vec<String>,
    new_lines: Vec<String>,
    output_color: OutputColor,
) -> Vec<String> {
    let old = old_lines.concat();
    let new = new_lines.concat();
    let diff = TextDiff::from_lines(&old, &new);

    if diff.ratio() == 1.0 {
        // no changes
        return Vec::new();
    }

    let mut lines = Vec::new();
    let (file_style, del_style, ins_style) = output_style(output_color);

    for group in diff.grouped_ops(0).iter() {
        for op in group {
            for change in diff.iter_changes(op) {
                if change.tag() == ChangeTag::Delete {
                    // always consists of a pair of delete and insert lines, so we only need to print the file path once
                    let line = format!(
                        "--- {}:{}\n",
                        file_path.display(),
                        change.old_index().unwrap() + 1
                    );
                    lines.push(format!("{}", file_style.apply_to(line)));
                }

                let (line, style) = match change.tag() {
                    ChangeTag::Delete => (format!("- {}", change.value()), del_style.clone()),
                    ChangeTag::Insert => (format!("+ {}", change.value()), ins_style.clone()),
                    ChangeTag::Equal => unreachable!(),
                };
                lines.push(format!("{}", style.apply_to(line)));
            }
        }
    }

    lines
}

fn output_style(output_color: OutputColor) -> (Style, Style, Style) {
    match output_color {
        OutputColor::Auto => (
            Style::new().color256(244),
            Style::new().red(),
            Style::new().green(),
        ),
        OutputColor::Always => (
            Style::new().force_styling(true).color256(244),
            Style::new().force_styling(true).red(),
            Style::new().force_styling(true).green(),
        ),
        OutputColor::Never => (Style::new(), Style::new(), Style::new()),
    }
}
