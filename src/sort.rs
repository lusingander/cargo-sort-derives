use crate::parse::{DeriveAttr, collect_derive_attrs};
use std::collections::{HashMap, HashSet};

/// Sort derive attributes in `source` using pre-collected `attrs`, return `(original, sorted)`.
pub fn sort_source(
    source: String,
    attrs: &[DeriveAttr],
    custom_order: &Option<Vec<String>>,
    preserve: bool,
) -> (String, String) {
    let original = source.clone();
    let mut new_source = source;

    let order_map = build_order_map(custom_order);

    let mut splices: Vec<(usize, usize, String)> = attrs
        .iter()
        .map(|attr| {
            let replacement = sort_and_render_attr(attr, &order_map, preserve);
            (attr.start, attr.end, replacement)
        })
        .collect();
    splices.sort_by(|a, b| b.0.cmp(&a.0));

    for (start, end, replacement) in &splices {
        new_source.replace_range(*start..*end, replacement);
    }

    (original, new_source)
}

fn compute_disabled_lines(source: &str) -> HashSet<usize> {
    let mut disabled = HashSet::new();
    let mut disable_next_line = false;
    let mut disable_range = false;

    for (i, line) in source.lines().enumerate() {
        let n = i + 1;

        if disable_next_line || disable_range {
            disabled.insert(n);
        }

        disable_next_line = false;

        if line.contains("sort-derives-disable-next-line") {
            disable_next_line = true;
        }
        if line.contains("sort-derives-disable-start") {
            disable_range = true;
        }
        if line.contains("sort-derives-disable-end") {
            disable_range = false;
        }
    }

    disabled
}

/// Sort derive attributes from stdin input, return `(original, sorted)`.
pub fn sort_stdin(
    input: &str,
    custom_order: &Option<Vec<String>>,
    preserve: bool,
) -> syn::Result<(String, String)> {
    let disabled_lines = compute_disabled_lines(input);
    let attrs = collect_derive_attrs(input, &disabled_lines)?;
    Ok(sort_source(
        input.to_string(),
        &attrs,
        custom_order,
        preserve,
    ))
}

const IGNORE: usize = 10_000;

fn build_order_map(custom_order: &Option<Vec<String>>) -> HashMap<String, usize> {
    match custom_order {
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
    }
}

fn path_base_name(path: &syn::Path) -> String {
    path.segments.last().unwrap().ident.to_string()
}

fn path_to_string(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|seg| seg.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

fn sort_and_render_attr(
    attr: &DeriveAttr,
    order_map: &HashMap<String, usize>,
    preserve: bool,
) -> String {
    let mut sorted_paths = attr.paths.clone();
    sorted_paths.sort_by(|a, b| {
        let base_a = path_base_name(a);
        let base_b = path_base_name(b);
        let priority_a = order_map.get(&base_a).copied().unwrap_or(IGNORE);
        let priority_b = order_map.get(&base_b).copied().unwrap_or(IGNORE);

        if preserve && priority_a == IGNORE && priority_b == IGNORE {
            std::cmp::Ordering::Equal
        } else {
            priority_a
                .cmp(&priority_b)
                .then_with(|| base_a.cmp(&base_b))
                .then_with(|| path_to_string(a).cmp(&path_to_string(b)))
        }
    });

    let sorted_paths_str = sorted_paths
        .iter()
        .map(|p| path_to_string(p))
        .collect::<Vec<_>>()
        .join(", ");

    match &attr.condition {
        None => format!("#[derive({})]", sorted_paths_str),
        Some(condition) => {
            format!("#[cfg_attr({}, derive({}))]", condition, sorted_paths_str)
        }
    }
}

// sort-derives-disable-start
#[cfg(test)]
mod tests {
    use super::*;
    use syn::Path;

    fn p(s: &str) -> Path {
        syn::parse_str::<Path>(s).unwrap()
    }

    fn paths(strings: &[&str]) -> Vec<Path> {
        strings.iter().map(|s| p(s)).collect()
    }

    fn sorted_path_strings(
        input_paths: &[Path],
        custom_order: &Option<Vec<String>>,
        preserve: bool,
    ) -> Vec<String> {
        let order_map = build_order_map(custom_order);
        let mut sorted = input_paths.to_vec();
        sorted.sort_by(|a, b| {
            let base_a = path_base_name(a);
            let base_b = path_base_name(b);
            let priority_a = order_map.get(&base_a).copied().unwrap_or(IGNORE);
            let priority_b = order_map.get(&base_b).copied().unwrap_or(IGNORE);

            if preserve && priority_a == IGNORE && priority_b == IGNORE {
                std::cmp::Ordering::Equal
            } else {
                priority_a
                    .cmp(&priority_b)
                    .then_with(|| base_a.cmp(&base_b))
                    .then_with(|| path_to_string(a).cmp(&path_to_string(b)))
            }
        });
        sorted.iter().map(|p| path_to_string(p)).collect()
    }

    #[test]
    fn test_sort_paths_without_order() {
        let input = paths(&[
            "Debug",
            "b::Eq",
            "a::Eq",
            "cmp::Eq",
            "Eq",
            "b::Foo",
            "a::Foo",
            "Foo",
            "std::clone::Clone",
            "Hash",
            "cmp::PartialOrd",
            "foo::bar::Bar",
        ]);
        let actual = sorted_path_strings(&input, &None, false);
        let expected: Vec<String> = vec![
            "foo::bar::Bar",
            "std::clone::Clone",
            "Debug",
            "Eq",
            "a::Eq",
            "b::Eq",
            "cmp::Eq",
            "Foo",
            "a::Foo",
            "b::Foo",
            "Hash",
            "cmp::PartialOrd",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_sort_paths_with_order() {
        let input = paths(&[
            "Debug",
            "b::Eq",
            "a::Eq",
            "cmp::Eq",
            "Eq",
            "b::Foo",
            "a::Foo",
            "Foo",
            "std::clone::Clone",
            "Hash",
            "cmp::PartialOrd",
            "foo::bar::Bar",
        ]);
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
            .map(String::from)
            .collect(),
        );
        let actual = sorted_path_strings(&input, &order, false);
        let expected: Vec<String> = vec![
            "Debug",
            "std::clone::Clone",
            "Eq",
            "a::Eq",
            "b::Eq",
            "cmp::Eq",
            "cmp::PartialOrd",
            "Hash",
            "foo::bar::Bar",
            "Foo",
            "a::Foo",
            "b::Foo",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_sort_paths_with_order_and_preserve() {
        let input = paths(&[
            "Debug",
            "b::Eq",
            "a::Eq",
            "cmp::Eq",
            "Eq",
            "b::Foo",
            "a::Foo",
            "Foo",
            "std::clone::Clone",
            "Hash",
            "cmp::PartialOrd",
            "foo::bar::Bar",
        ]);
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
            .map(String::from)
            .collect(),
        );
        let actual = sorted_path_strings(&input, &order, true);
        let expected: Vec<String> = vec![
            "Debug",
            "std::clone::Clone",
            "Eq",
            "a::Eq",
            "b::Eq",
            "cmp::Eq",
            "cmp::PartialOrd",
            "Hash",
            "b::Foo",
            "a::Foo",
            "Foo",
            "foo::bar::Bar",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_sort_paths_with_head_ellipsis_order() {
        let input = paths(&["D", "B", "A", "E", "F", "C", "G"]);
        let order = Some(
            vec!["...", "D", "A"]
                .into_iter()
                .map(String::from)
                .collect(),
        );
        let actual = sorted_path_strings(&input, &order, false);
        let expected: Vec<String> = vec!["B", "C", "E", "F", "G", "D", "A"]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_sort_paths_with_middle_ellipsis_order() {
        let input = paths(&["D", "B", "A", "E", "F", "C", "G"]);
        let order = Some(
            vec!["B", "G", "...", "D", "A"]
                .into_iter()
                .map(String::from)
                .collect(),
        );
        let actual = sorted_path_strings(&input, &order, false);
        let expected: Vec<String> = vec!["B", "G", "C", "E", "F", "D", "A"]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_sort_paths_with_tail_ellipsis_order() {
        let input = paths(&["D", "B", "A", "E", "F", "C", "G"]);
        let order = Some(
            vec!["B", "G", "..."]
                .into_iter()
                .map(String::from)
                .collect(),
        );
        let actual = sorted_path_strings(&input, &order, false);
        let expected: Vec<String> = vec!["B", "G", "A", "C", "D", "E", "F"]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_sort_paths_with_middle_ellipsis_order_and_preserve() {
        let input = paths(&["D", "B", "A", "E", "F", "C", "G"]);
        let order = Some(
            vec!["B", "G", "...", "D", "A"]
                .into_iter()
                .map(String::from)
                .collect(),
        );
        let actual = sorted_path_strings(&input, &order, true);
        let expected: Vec<String> = vec!["B", "G", "E", "F", "C", "D", "A"]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_sort_source_plain_derive() {
        let source = "#[derive(Debug, Clone, Copy)]\nstruct Foo;".to_string();
        let attrs = collect_derive_attrs(&source, &HashSet::new()).unwrap();
        let (original, sorted) = sort_source(source, &attrs, &None, false);
        assert_eq!(original, "#[derive(Debug, Clone, Copy)]\nstruct Foo;");
        assert_eq!(sorted, "#[derive(Clone, Copy, Debug)]\nstruct Foo;");
    }

    #[test]
    fn test_sort_source_cfg_attr() {
        let source =
            "#[cfg_attr(feature = \"serde\", derive(Serialize, Deserialize))]\nstruct Foo;"
                .to_string();
        let attrs = collect_derive_attrs(&source, &HashSet::new()).unwrap();
        let cond_str = attrs[0].condition.as_ref().unwrap().to_string();
        let (original, sorted) = sort_source(source, &attrs, &None, false);
        assert_eq!(
            original,
            "#[cfg_attr(feature = \"serde\", derive(Serialize, Deserialize))]\nstruct Foo;"
        );
        assert_eq!(
            sorted,
            format!(
                "#[cfg_attr({}, derive(Deserialize, Serialize))]\nstruct Foo;",
                cond_str
            )
        );
    }

    #[test]
    fn test_sort_source_cfg_attr_complex() {
        let source = "#[cfg_attr(all(feature = \"serde\", not(test)), derive(serde::Serialize, serde::Deserialize, Debug))]\nstruct Foo;".to_string();
        let attrs = collect_derive_attrs(&source, &HashSet::new()).unwrap();
        let cond_str = attrs[0].condition.as_ref().unwrap().to_string();
        let (original, sorted) = sort_source(source, &attrs, &None, false);
        assert_eq!(
            original,
            "#[cfg_attr(all(feature = \"serde\", not(test)), derive(serde::Serialize, serde::Deserialize, Debug))]\nstruct Foo;"
        );
        assert_eq!(
            sorted,
            format!(
                "#[cfg_attr({}, derive(Debug, serde::Deserialize, serde::Serialize))]\nstruct Foo;",
                cond_str
            )
        );
    }

    #[test]
    fn test_sort_source_with_custom_order() {
        let source = "#[derive(Debug, Clone, Copy)]\nstruct Foo;".to_string();
        let attrs = collect_derive_attrs(&source, &HashSet::new()).unwrap();
        let order = Some(vec!["Copy".to_string(), "Debug".to_string()]);
        let (_, sorted) = sort_source(source, &attrs, &order, false);
        assert_eq!(sorted, "#[derive(Copy, Debug, Clone)]\nstruct Foo;");
    }

    #[test]
    fn test_sort_stdin_plain() {
        let input = "#[derive(Debug, Clone)]\nstruct Foo;";
        let (original, sorted) = sort_stdin(input, &None, false).unwrap();
        assert_eq!(original, "#[derive(Debug, Clone)]\nstruct Foo;");
        assert_eq!(sorted, "#[derive(Clone, Debug)]\nstruct Foo;");
    }

    #[test]
    fn test_sort_source_multiline_derive() {
        let source = "#[derive(\n    Debug,\n    Clone,\n)]\nstruct Foo;".to_string();
        let attrs = collect_derive_attrs(&source, &HashSet::new()).unwrap();
        let (_, sorted) = sort_source(source, &attrs, &None, false);
        assert_eq!(sorted, "#[derive(Clone, Debug)]\nstruct Foo;");
    }
}
// sort-derives-disable-end
