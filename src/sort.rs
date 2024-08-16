use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::LazyLock,
};

use regex::Regex;

use crate::ext::BufReadExt;

const DEFAULT_ORDER: &[&str; 9] = &[
    "Debug",
    "Default",
    "Clone",
    "Copy",
    "PartialEq",
    "Eq",
    "PartialOrd",
    "Ord",
    "Hash",
];

const PATTERN: &str = r"#\[derive\(\s*([^\)]+?)\s*\)\]";
static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(PATTERN).unwrap());

pub fn process_file(file_path: &Path, line_numbers: HashSet<usize>) -> Result<(), std::io::Error> {
    let file = std::fs::File::open(file_path)?;
    let reader = std::io::BufReader::new(file);

    let mut lines = Vec::with_capacity(line_numbers.len());

    for (i, line) in reader.lines_with_terminator().enumerate() {
        let n = i + 1;
        let line = line?;

        if line_numbers.contains(&n) {
            let derives = parse_derive_traits(&line);
            let sorted_derives = sort_derive_traits(&derives, DEFAULT_ORDER);
            let new_line = replace_line(&line, &sorted_derives);
            lines.push(new_line);
        } else {
            lines.push(line);
        }
    }

    std::fs::write(file_path, lines.concat())?;

    Ok(())
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

fn sort_derive_traits(derives: &[DeriveTrait], order: &[&str]) -> Vec<DeriveTrait> {
    let order_map: HashMap<&str, usize> = order.iter().enumerate().map(|(i, &s)| (s, i)).collect();
    let mut derives = derives.to_vec();
    derives.sort_by_key(|d| order_map.get(d.base_name.as_str()).unwrap_or(&usize::MAX));
    derives
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
    fn test_sort_derive_traits() {
        let derives = vec![
            dt("Debug", "Debug"),
            dt("cmp::Eq", "Eq"),
            dt("Foo", "Foo"),
            dt("std::clone::Clone", "Clone"),
            dt("Hash", "Hash"),
            dt("cmp::PartialOrd", "PartialOrd"),
            dt("foo::bar::Bar", "Bar"),
        ];
        let order = &[
            "Debug",
            "Default",
            "Clone",
            "Copy",
            "PartialEq",
            "Eq",
            "PartialOrd",
            "Ord",
            "Hash",
        ];
        let actual = sort_derive_traits(&derives, order);
        let expected = vec![
            dt("Debug", "Debug"),
            dt("std::clone::Clone", "Clone"),
            dt("cmp::Eq", "Eq"),
            dt("cmp::PartialOrd", "PartialOrd"),
            dt("Hash", "Hash"),
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
