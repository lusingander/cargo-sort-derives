use std::{fmt::Write, path::Path};

use console::Style;
use similar::{ChangeTag, TextDiff};

use crate::process::DeriveChange;

#[derive(Debug, Clone, Copy)]
pub enum OutputColor {
    Auto,
    Always,
    Never,
}

pub struct Console {
    quiet: bool,
    file_style: Style,
    del_style: Style,
    ins_style: Style,
    eq_style: Style,
}

impl Console {
    pub fn new(quiet: bool, output_color: OutputColor) -> Self {
        let (file_style, del_style, ins_style, eq_style) = match output_color {
            OutputColor::Auto => (
                Style::new().color256(244),
                Style::new().red(),
                Style::new().green(),
                Style::new().blue(),
            ),
            OutputColor::Always => (
                Style::new().force_styling(true).color256(244),
                Style::new().force_styling(true).red(),
                Style::new().force_styling(true).green(),
                Style::new().force_styling(true).blue(),
            ),
            OutputColor::Never => (Style::new(), Style::new(), Style::new(), Style::new()),
        };
        Console {
            quiet,
            file_style,
            del_style,
            ins_style,
            eq_style,
        }
    }

    pub fn print_config(&self, order: &Option<Vec<String>>, preserve: bool, exclude: &[String]) {
        if self.quiet {
            return;
        }
        let order = order.as_ref().map_or("None".to_owned(), |o| o.join(", "));
        eprintln!();
        eprintln!("order: {order}");
        eprintln!("preserve: {preserve}");
        eprintln!("exclude: {exclude:?}");
        eprintln!();
    }

    pub fn print_summary(&self, changed: usize, unchanged: usize, total: usize) {
        if self.quiet {
            return;
        }
        eprintln!("Changed: {changed}, Unchanged: {unchanged}, Total: {total}");
    }

    /// Called by `process` for every derive attribute in non-check mode.
    pub fn on_derive(&self, dc: &DeriveChange) {
        if self.quiet {
            return;
        }
        let mut entry = String::new();
        let header =
            self.file_style
                .apply_to(format!("--- {}:{}", dc.file_path.display(), dc.line));
        writeln!(entry, "{header}").unwrap();
        if dc.changed() {
            writeln!(
                entry,
                "{}",
                self.del_style.apply_to(format!("- {}", dc.old_text))
            )
            .unwrap();
            writeln!(
                entry,
                "{}",
                self.ins_style.apply_to(format!("+ {}", dc.new_text))
            )
            .unwrap();
        } else {
            writeln!(
                entry,
                "{}",
                self.eq_style.apply_to(format!("= {}", dc.old_text))
            )
            .unwrap();
        }
        entry.push('\n');
        eprint!("{entry}");
    }

    /// Print a TextDiff-based diff for check mode.
    pub fn print_check_diff(&self, file_path: &Path, old: &str, new: &str) {
        if self.quiet {
            return;
        }
        let diff = TextDiff::from_lines(old, new);
        if diff.ratio() == 1.0 {
            return;
        }

        for group in diff.grouped_ops(0).iter() {
            let mut entry = String::new();
            let mut first_old_line = 0usize;
            for op in group {
                for change in diff.iter_changes(op) {
                    if change.tag() == ChangeTag::Delete && first_old_line == 0 {
                        first_old_line = change.old_index().unwrap() + 1;
                        writeln!(
                            entry,
                            "{}",
                            self.format_diff_header(file_path, first_old_line)
                        )
                        .unwrap();
                    }
                    writeln!(
                        entry,
                        "{}",
                        self.format_diff_change(change.tag(), change.value())
                    )
                    .unwrap();
                }
            }
            if first_old_line > 0 {
                entry.push('\n');
            }
            print!("{entry}");
        }
    }

    fn format_diff_header(&self, file_path: &Path, line: usize) -> String {
        self.file_style
            .apply_to(format!("@ {}:{}", file_path.display(), line))
            .to_string()
    }

    fn format_diff_change(&self, tag: ChangeTag, value: &str) -> String {
        let (prefix, style) = match tag {
            ChangeTag::Delete => ("-", &self.del_style),
            ChangeTag::Insert => ("+", &self.ins_style),
            ChangeTag::Equal => unreachable!(),
        };
        style
            .apply_to(format!("{prefix} {}", value.trim()))
            .to_string()
    }
}
