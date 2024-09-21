use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::LazyLock,
};

use console::Style;
use regex::Regex;
use similar::{ChangeTag, TextDiff};

use crate::ext::BufReadExt;

const PATTERN: &str = r"#\[derive\(\s*([^\)]+?)\s*\)\]";
static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(PATTERN).unwrap());

#[derive(Debug, Clone, Copy)]
pub enum OutputColor {
    Auto,
    Always,
    Never,
}

pub fn process_file(
    file_path: &Path,
    line_numbers: HashSet<usize>,
    custom_order: &Option<Vec<&str>>,
    preserve: bool,
    check: bool,
    output_color: OutputColor,
) -> Result<bool, std::io::Error> {
    let file = std::fs::File::open(file_path)?;
    let reader = std::io::BufReader::new(file);

    let mut old_lines = Vec::with_capacity(line_numbers.len());
    let mut new_lines = Vec::with_capacity(line_numbers.len());

    for (i, line) in reader.lines_with_terminator().enumerate() {
        let n = i + 1;
        let line = line?;

        let new_line = if line_numbers.contains(&n) {
            let derives = parse_derive_traits(&line);
            let sorted_derives = sort_derive_traits(&derives, custom_order, preserve);
            replace_line(&line, &sorted_derives)
        } else {
            line.clone()
        };

        old_lines.push(line);
        new_lines.push(new_line);
    }

    if !check {
        write_file(file_path, new_lines)?;
        return Ok(true);
    }

    let diffs = calc_diff_lines(file_path, old_lines, new_lines, output_color);
    if diffs.is_empty() {
        return Ok(true);
    }

    for line in diffs {
        print!("{}", line);
    }

    Ok(false)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeriveTrait {
    s: String,
    base_name: String,
}

fn parse_derive_traits(line: &str) -> Vec<DeriveTrait> {
    let caps = RE.captures(line).unwrap();
    caps.get(1)
        .unwrap()
        .as_str()
        .split(',')
        .map(|s| s.trim())
        .map(|s| {
            let base_name = s.split(':').last().unwrap();
            DeriveTrait {
                s: s.into(),
                base_name: base_name.into(),
            }
        })
        .collect()
}

fn sort_derive_traits(
    derives: &[DeriveTrait],
    custom_order: &Option<Vec<&str>>,
    preserve: bool,
) -> Vec<DeriveTrait> {
    let order_map: HashMap<String, usize> =
        custom_order.as_ref().map_or_else(HashMap::new, |order| {
            order
                .iter()
                .enumerate()
                .map(|(i, &s)| (s.to_string(), i))
                .collect()
        });

    let mut sorted_derives = derives.to_vec();
    sorted_derives.sort_by(|a, b| {
        const IGNORE: &usize = &usize::MAX;
        let priority_a = order_map.get(&a.base_name).unwrap_or(IGNORE);
        let priority_b = order_map.get(&b.base_name).unwrap_or(IGNORE);

        if preserve && priority_a == IGNORE && priority_b == IGNORE {
            std::cmp::Ordering::Equal
        } else {
            priority_a
                .cmp(priority_b)
                .then_with(|| a.base_name.cmp(&b.base_name))
                .then_with(|| a.s.cmp(&b.s))
        }
    });

    sorted_derives
}

fn replace_line(line: &str, sorted_derives: &[DeriveTrait]) -> String {
    let sorted_derive_str = sorted_derives
        .iter()
        .map(|d| d.s.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let sorted_derive_str = format!("#[derive({})]", sorted_derive_str);
    RE.replace(line, sorted_derive_str).into()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_derive_traits() {
        let line = "#[derive(Debug, cmp::Eq, Foo, std::clone::Clone, Hash, cmp::PartialOrd, foo::bar::Bar)]";
        let actual = parse_derive_traits(line);
        let expected = vec![
            dt("Debug", "Debug"),
            dt("cmp::Eq", "Eq"),
            dt("Foo", "Foo"),
            dt("std::clone::Clone", "Clone"),
            dt("Hash", "Hash"),
            dt("cmp::PartialOrd", "PartialOrd"),
            dt("foo::bar::Bar", "Bar"),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_sort_derive_traits_without_order() {
        let derives = vec![
            dt("Debug", "Debug"),
            dt("b::Eq", "Eq"),
            dt("a::Eq", "Eq"),
            dt("cmp::Eq", "Eq"),
            dt("Eq", "Eq"),
            dt("b::Foo", "Foo"),
            dt("a::Foo", "Foo"),
            dt("Foo", "Foo"),
            dt("std::clone::Clone", "Clone"),
            dt("Hash", "Hash"),
            dt("cmp::PartialOrd", "PartialOrd"),
            dt("foo::bar::Bar", "Bar"),
        ];
        let order = None;
        let actual = sort_derive_traits(&derives, &order, false);
        let expected = vec![
            dt("foo::bar::Bar", "Bar"),
            dt("std::clone::Clone", "Clone"),
            dt("Debug", "Debug"),
            dt("Eq", "Eq"),
            dt("a::Eq", "Eq"),
            dt("b::Eq", "Eq"),
            dt("cmp::Eq", "Eq"),
            dt("Foo", "Foo"),
            dt("a::Foo", "Foo"),
            dt("b::Foo", "Foo"),
            dt("Hash", "Hash"),
            dt("cmp::PartialOrd", "PartialOrd"),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_sort_derive_traits_with_order() {
        let derives = vec![
            dt("Debug", "Debug"),
            dt("b::Eq", "Eq"),
            dt("a::Eq", "Eq"),
            dt("cmp::Eq", "Eq"),
            dt("Eq", "Eq"),
            dt("b::Foo", "Foo"),
            dt("a::Foo", "Foo"),
            dt("Foo", "Foo"),
            dt("std::clone::Clone", "Clone"),
            dt("Hash", "Hash"),
            dt("cmp::PartialOrd", "PartialOrd"),
            dt("foo::bar::Bar", "Bar"),
        ];
        let order = Some(vec![
            "Debug",
            "Default",
            "Clone",
            "Copy",
            "PartialEq",
            "Eq",
            "PartialOrd",
            "Ord",
            "Hash",
        ]);
        let actual = sort_derive_traits(&derives, &order, false);
        let expected = vec![
            dt("Debug", "Debug"),
            dt("std::clone::Clone", "Clone"),
            dt("Eq", "Eq"),
            dt("a::Eq", "Eq"),
            dt("b::Eq", "Eq"),
            dt("cmp::Eq", "Eq"),
            dt("cmp::PartialOrd", "PartialOrd"),
            dt("Hash", "Hash"),
            dt("foo::bar::Bar", "Bar"),
            dt("Foo", "Foo"),
            dt("a::Foo", "Foo"),
            dt("b::Foo", "Foo"),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_sort_derive_traits_with_order_and_preserve() {
        let derives = vec![
            dt("Debug", "Debug"),
            dt("b::Eq", "Eq"),
            dt("a::Eq", "Eq"),
            dt("cmp::Eq", "Eq"),
            dt("Eq", "Eq"),
            dt("b::Foo", "Foo"),
            dt("a::Foo", "Foo"),
            dt("Foo", "Foo"),
            dt("std::clone::Clone", "Clone"),
            dt("Hash", "Hash"),
            dt("cmp::PartialOrd", "PartialOrd"),
            dt("foo::bar::Bar", "Bar"),
        ];
        let order = Some(vec![
            "Debug",
            "Default",
            "Clone",
            "Copy",
            "PartialEq",
            "Eq",
            "PartialOrd",
            "Ord",
            "Hash",
        ]);
        let actual = sort_derive_traits(&derives, &order, true);
        let expected = vec![
            dt("Debug", "Debug"),
            dt("std::clone::Clone", "Clone"),
            dt("Eq", "Eq"),
            dt("a::Eq", "Eq"),
            dt("b::Eq", "Eq"),
            dt("cmp::Eq", "Eq"),
            dt("cmp::PartialOrd", "PartialOrd"),
            dt("Hash", "Hash"),
            dt("b::Foo", "Foo"),
            dt("a::Foo", "Foo"),
            dt("Foo", "Foo"),
            dt("foo::bar::Bar", "Bar"),
        ];
        assert_eq!(actual, expected);
    }

    fn dt(s: &str, base_name: &str) -> DeriveTrait {
        DeriveTrait {
            s: s.into(),
            base_name: base_name.into(),
        }
    }
}
