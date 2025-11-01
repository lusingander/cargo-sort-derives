use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::LazyLock,
};

use regex::Regex;

use crate::ext::BufReadExt;

const DERIVE_PATTERN: &str = r"#\[derive\(\s*([^\)]+?)\s*\)\]";
const CFG_ATTR_PATTERN: &str = r"#\[cfg_attr\((.+),\s*derive\(\s*([^\)]+?)\s*\)\)\]";

static DERIVE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(DERIVE_PATTERN).unwrap());
static CFG_ATTR_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(CFG_ATTR_PATTERN).unwrap());

const DISABLE_NEXT_LINE: &str = "sort-derives-disable-next-line";
const DISABLE_START: &str = "sort-derives-disable-start";
const DISABLE_END: &str = "sort-derives-disable-end";

pub fn sort(
    file_path: &Path,
    line_numbers: HashSet<usize>,
    custom_order: &Option<Vec<String>>,
    preserve: bool,
) -> Result<(Vec<String>, Vec<String>), std::io::Error> {
    let file = std::fs::File::open(file_path)?;
    let reader = std::io::BufReader::new(file);

    let mut old_lines = Vec::with_capacity(line_numbers.len());
    let mut new_lines = Vec::with_capacity(line_numbers.len());

    let mut disable_next_line = false;
    let mut disable_range = false;

    for (i, line) in reader.lines_with_terminator().enumerate() {
        let n = i + 1;
        let line = line?;

        let new_line = if !disable_next_line && !disable_range && line_numbers.contains(&n) {
            if let Some(derives) = parse_derive_traits(&line) {
                let sorted_derives = sort_derive_traits(&derives, custom_order, preserve);
                replace_line(&line, &sorted_derives)
            } else {
                line.clone()
            }
        } else {
            line.clone()
        };

        disable_next_line = false;
        if line.contains(DISABLE_NEXT_LINE) {
            disable_next_line = true;
        }

        if line.contains(DISABLE_START) {
            disable_range = true;
        }
        if line.contains(DISABLE_END) {
            disable_range = false;
        }

        old_lines.push(line);
        new_lines.push(new_line);
    }

    Ok((old_lines, new_lines))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeriveTrait {
    s: String,
    base_name: String,
}

fn parse_derive_traits(line: &str) -> Option<Vec<DeriveTrait>> {
    let derives_str = if let Some(caps) = DERIVE_RE.captures(line) {
        caps.get(1).map(|m| m.as_str())
    } else if let Some(caps) = CFG_ATTR_RE.captures(line) {
        caps.get(2).map(|m| m.as_str())
    } else {
        None
    };

    derives_str.map(|s| {
        s.split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| {
                let base_name = s.split(':').next_back().unwrap_or(s);
                DeriveTrait {
                    s: s.into(),
                    base_name: base_name.into(),
                }
            })
            .collect()
    })
}

fn sort_derive_traits(
    derives: &[DeriveTrait],
    custom_order: &Option<Vec<String>>,
    preserve: bool,
) -> Vec<DeriveTrait> {
    const IGNORE: &usize = &10_000; // large enough

    let order_map: HashMap<String, usize> = match custom_order {
        Some(custom_order) => {
            let head_order = custom_order
                .iter()
                .take_while(|s| *s != "...")
                .enumerate()
                .map(|(i, s)| (s.clone(), i));
            let tail_order = custom_order
                .iter()
                .skip_while(|s| *s != "...")
                .skip(1)
                .enumerate()
                .map(|(i, s)| (s.clone(), i + IGNORE + 1));
            head_order.chain(tail_order).collect()
        }
        None => HashMap::new(),
    };

    let mut sorted_derives = derives.to_vec();
    sorted_derives.sort_by(|a, b| {
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

    if let Some(_caps) = DERIVE_RE.captures(line) {
        let replacement = format!("#[derive({sorted_derive_str})]");
        return DERIVE_RE.replace(line, replacement).into();
    }

    if let Some(caps) = CFG_ATTR_RE.captures(line) {
        let condition = caps.get(1).unwrap().as_str();
        let replacement = format!("#[cfg_attr({condition}, derive({sorted_derive_str}))]");
        return CFG_ATTR_RE.replace(line, replacement).into();
    }

    line.to_string()
}

// sort-derives-disable-start
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_derive_traits() {
        let line = "#[derive(Debug, cmp::Eq, Foo, std::clone::Clone, Hash, cmp::PartialOrd, foo::bar::Bar)]";
        let actual = parse_derive_traits(line).unwrap();
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
    fn test_parse_derive_traits_with_cfg_attr() {
        let line = "#[cfg_attr(feature = \"serde\", derive(Serialize, Deserialize))]";
        let actual = parse_derive_traits(line).unwrap();
        let expected = vec![
            dt("Serialize", "Serialize"),
            dt("Deserialize", "Deserialize"),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_derive_traits_with_complex_cfg_attr() {
        let line = "#[cfg_attr(all(feature = \"serde\", not(test)), derive(serde::Serialize, serde::Deserialize, Debug))]";
        let actual = parse_derive_traits(line).unwrap();
        let expected = vec![
            dt("serde::Serialize", "Serialize"),
            dt("serde::Deserialize", "Deserialize"),
            dt("Debug", "Debug"),
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
        let order = Some(
            vec![
                "Debug",
                "Default",
                "Clone",
                "Copy",
                "PartialEq",
                "Eq",
                "PartialOrd",
                "Ord",
                "Hash",
            ]
            .into_iter()
            .map(Into::into)
            .collect(),
        );
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
        let order = Some(
            vec![
                "Debug",
                "Default",
                "Clone",
                "Copy",
                "PartialEq",
                "Eq",
                "PartialOrd",
                "Ord",
                "Hash",
            ]
            .into_iter()
            .map(Into::into)
            .collect(),
        );
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

    #[test]
    fn test_sort_derive_traits_with_head_ellipsis_order() {
        let derives = vec![
            dt("D", "D"),
            dt("B", "B"),
            dt("A", "A"),
            dt("E", "E"),
            dt("F", "F"),
            dt("C", "C"),
            dt("G", "G"),
        ];
        let order = Some(vec!["...", "D", "A"].into_iter().map(Into::into).collect());
        let actual = sort_derive_traits(&derives, &order, false);
        let expected = vec![
            // ellipsis
            dt("B", "B"),
            dt("C", "C"),
            dt("E", "E"),
            dt("F", "F"),
            dt("G", "G"),
            // tail
            dt("D", "D"),
            dt("A", "A"),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_sort_derive_traits_with_middle_ellipsis_order() {
        let derives = vec![
            dt("D", "D"),
            dt("B", "B"),
            dt("A", "A"),
            dt("E", "E"),
            dt("F", "F"),
            dt("C", "C"),
            dt("G", "G"),
        ];
        let order = Some(
            vec!["B", "G", "...", "D", "A"]
                .into_iter()
                .map(Into::into)
                .collect(),
        );
        let actual = sort_derive_traits(&derives, &order, false);
        let expected = vec![
            // head
            dt("B", "B"),
            dt("G", "G"),
            // ellipsis
            dt("C", "C"),
            dt("E", "E"),
            dt("F", "F"),
            // tail
            dt("D", "D"),
            dt("A", "A"),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_sort_derive_traits_with_tail_ellipsis_order() {
        let derives = vec![
            dt("D", "D"),
            dt("B", "B"),
            dt("A", "A"),
            dt("E", "E"),
            dt("F", "F"),
            dt("C", "C"),
            dt("G", "G"),
        ];
        let order = Some(vec!["B", "G", "..."].into_iter().map(Into::into).collect());
        let actual = sort_derive_traits(&derives, &order, false);
        let expected = vec![
            // head
            dt("B", "B"),
            dt("G", "G"),
            // ellipsis
            dt("A", "A"),
            dt("C", "C"),
            dt("D", "D"),
            dt("E", "E"),
            dt("F", "F"),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_sort_derive_traits_with_middle_ellipsis_order_and_preserve() {
        let derives = vec![
            dt("D", "D"),
            dt("B", "B"),
            dt("A", "A"),
            dt("E", "E"),
            dt("F", "F"),
            dt("C", "C"),
            dt("G", "G"),
        ];
        let order = Some(
            vec!["B", "G", "...", "D", "A"]
                .into_iter()
                .map(Into::into)
                .collect(),
        );
        let actual = sort_derive_traits(&derives, &order, true);
        let expected = vec![
            // head
            dt("B", "B"),
            dt("G", "G"),
            // ellipsis
            dt("E", "E"),
            dt("F", "F"),
            dt("C", "C"),
            // tail
            dt("D", "D"),
            dt("A", "A"),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_replace_line_with_cfg_attr() {
        let line = "#[cfg_attr(feature = \"serde\", derive(Serialize, Deserialize))]";
        let sorted_derives = vec![
            dt("Deserialize", "Deserialize"),
            dt("Serialize", "Serialize"),
        ];
        let actual = replace_line(line, &sorted_derives);
        let expected = "#[cfg_attr(feature = \"serde\", derive(Deserialize, Serialize))]";
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_replace_line_with_complex_cfg_attr() {
        let line = "#[cfg_attr(all(feature = \"serde\", not(test)), derive(serde::Serialize, serde::Deserialize, Debug))]";
        let sorted_derives = vec![
            dt("Debug", "Debug"),
            dt("serde::Deserialize", "Deserialize"),
            dt("serde::Serialize", "Serialize"),
        ];
        let actual = replace_line(line, &sorted_derives);
        let expected = "#[cfg_attr(all(feature = \"serde\", not(test)), derive(Debug, serde::Deserialize, serde::Serialize))]";
        assert_eq!(actual, expected);
    }

    fn dt(s: &str, base_name: &str) -> DeriveTrait {
        DeriveTrait {
            s: s.into(),
            base_name: base_name.into(),
        }
    }
}
// sort-derives-disable-end
