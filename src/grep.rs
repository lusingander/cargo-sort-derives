use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::mpsc,
};

use ignore::{WalkBuilder, WalkParallel, overrides::OverrideBuilder, types::TypesBuilder};

pub type Matches = Vec<(PathBuf, Vec<crate::parse::DeriveAttr>)>;

pub fn grep<P: AsRef<Path>>(path: Option<P>, exclude: Vec<String>) -> Result<Matches, String> {
    match path {
        Some(path) => grep_single_file(path),
        None => grep_all_files(".", exclude),
    }
}

fn grep_all_files<P: AsRef<Path>>(root: P, exclude: Vec<String>) -> Result<Matches, String> {
    let mut type_builder = TypesBuilder::new();
    type_builder.add_defaults().select("rust");

    let mut override_builder = OverrideBuilder::new(root.as_ref());
    for glob in exclude {
        override_builder
            .add(&format!("!{glob}"))
            .map_err(|e| e.to_string())?;
    }

    let walker = WalkBuilder::new(root.as_ref())
        .standard_filters(true)
        .types(type_builder.build().unwrap())
        .overrides(override_builder.build().unwrap())
        .build_parallel();

    exec_grep(walker)
}

fn grep_single_file<P: AsRef<Path>>(path: P) -> Result<Matches, String> {
    let path = path.as_ref();

    if path.is_dir() {
        return Err(format!("{} is a directory", path.display()));
    }

    if path.extension().is_none_or(|ext| ext != "rs") {
        return Err(format!("{} is not a Rust source file", path.display()));
    }

    let walker = WalkBuilder::new(path).build_parallel();

    exec_grep(walker)
}

fn exec_grep(walker: WalkParallel) -> Result<Matches, String> {
    let (tx, rx) = mpsc::channel();

    walker.run(|| {
        let tx = tx.clone();
        Box::new(move |result| match result {
            Ok(entry) => {
                if let Some(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        tx.send(Ok(entry.into_path())).unwrap();
                    }
                }
                ignore::WalkState::Continue
            }
            Err(err) => {
                tx.send(Err(err.to_string())).unwrap();
                ignore::WalkState::Quit
            }
        })
    });

    drop(tx);

    let paths: Result<Vec<PathBuf>, String> = rx.into_iter().collect();
    let paths = paths?;

    let mut results: Matches = Vec::new();
    for path in paths {
        let content =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let disabled_lines = compute_disabled_lines(&content);
        let attrs = crate::parse::collect_derive_attrs(&content, &disabled_lines)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        if !attrs.is_empty() {
            results.push((path, attrs));
        }
    }

    results.sort_by(|(a, _), (b, _)| a.cmp(b));
    Ok(results)
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

#[cfg(test)]
mod tests {
    use assert_fs::prelude::{FileWriteStr, PathChild};

    use super::*;

    #[test]
    fn test_grep_all_files() {
        let files = &[
            ("a.rs", rs_file_1(), true),
            ("b.rs", rs_file_1(), true),
            ("c.rs", rs_file_2(), false), // no derive
            ("x/xa.rs", rs_file_1(), true),
            ("x/xb.txt", rs_file_1(), false), // not .rs
            ("x/y/ya.rs", rs_file_1(), true),
            ("x/z/za.rs", rs_file_1(), true),
        ];
        let exclude = vec![];

        let tmp_root_dir = setup_tmp_files(files);
        let expected = expected_paths(tmp_root_dir.path(), files);

        let actual = grep_all_files(tmp_root_dir.path(), exclude).unwrap();
        let actual_paths: Vec<&PathBuf> = actual.iter().map(|(p, _)| p).collect();
        assert_eq!(actual_paths, expected.iter().collect::<Vec<_>>());
        for (_, attrs) in &actual {
            assert!(!attrs.is_empty());
            assert_eq!(attrs.len(), 4, "rs_file_1 should yield 4 derive attrs");
        }
    }

    #[test]
    fn test_grep_all_files_with_exclude() {
        let files = &[
            ("a.rs", rs_file_1(), true),
            ("b.rs", rs_file_1(), false),
            ("x/xa.rs", rs_file_1(), false),
            ("x/xb.rs", rs_file_1(), false),
            ("x/y/ya.rs", rs_file_1(), false),
            ("x/z/za.rs", rs_file_1(), false),
            ("o/oa.rs", rs_file_1(), true),
            ("o/p/pa.rs", rs_file_1(), false),
            ("o/p/pb.rs", rs_file_1(), true),
            ("k/l/m/n/na.rs", rs_file_1(), false),
        ];
        let exclude = vec![
            "b.rs".into(),
            "x/*".into(),
            "pa.rs".into(),
            "k/**/na.rs".into(),
        ];

        let tmp_root_dir = setup_tmp_files(files);
        let expected = expected_paths(tmp_root_dir.path(), files);

        let actual = grep_all_files(tmp_root_dir.path(), exclude).unwrap();
        let actual_paths: Vec<&PathBuf> = actual.iter().map(|(p, _)| p).collect();
        assert_eq!(actual_paths, expected.iter().collect::<Vec<_>>());
        for (_, attrs) in &actual {
            assert!(!attrs.is_empty());
            assert_eq!(attrs.len(), 4, "rs_file_1 should yield 4 derive attrs");
        }
    }

    #[test]
    fn test_grep_all_files_with_ignore_file() {
        let files = &[
            ("a.rs", rs_file_1(), true),
            ("b.rs", rs_file_1(), false),
            ("x/xa.rs", rs_file_1(), true),
            ("x/xb.rs", rs_file_1(), true),
            ("x/y/ya.rs", rs_file_1(), false),
            ("x/z/za.rs", rs_file_1(), true),
        ];
        let exclude = vec![];

        let tmp_root_dir = setup_tmp_files(files);

        setup_ignore_file(&tmp_root_dir, ".ignore", vec!["b.rs", "x/y/*"]);

        let expected = expected_paths(tmp_root_dir.path(), files);

        let actual = grep_all_files(tmp_root_dir.path(), exclude).unwrap();
        let actual_paths: Vec<&PathBuf> = actual.iter().map(|(p, _)| p).collect();
        assert_eq!(actual_paths, expected.iter().collect::<Vec<_>>());
        for (_, attrs) in &actual {
            assert!(!attrs.is_empty());
            assert_eq!(attrs.len(), 4, "rs_file_1 should yield 4 derive attrs");
        }
    }

    #[test]
    fn test_grep_single_file() {
        let files = &[
            ("a.rs", rs_file_1(), false),
            ("b.rs", rs_file_1(), false),
            ("x/xa.rs", rs_file_1(), true),
            ("x/xb.rs", rs_file_1(), false),
            ("x/y/ya.rs", rs_file_1(), false),
            ("x/z/za.rs", rs_file_1(), false),
        ];

        let tmp_root_dir = setup_tmp_files(files);

        let expected = expected_paths(tmp_root_dir.path(), files);

        let actual = grep_single_file(tmp_root_dir.child("x/xa.rs")).unwrap();
        let actual_paths: Vec<&PathBuf> = actual.iter().map(|(p, _)| p).collect();
        assert_eq!(actual_paths, expected.iter().collect::<Vec<_>>());
        for (_, attrs) in &actual {
            assert!(!attrs.is_empty());
            assert_eq!(attrs.len(), 4, "rs_file_1 should yield 4 derive attrs");
        }
    }

    #[test]
    fn test_grep_single_file_with_ignore_file() {
        let files = &[
            ("a.rs", rs_file_1(), false),
            ("b.rs", rs_file_1(), false),
            ("x/xa.rs", rs_file_1(), true),
            ("x/xb.rs", rs_file_1(), false),
            ("x/y/ya.rs", rs_file_1(), false),
            ("x/z/za.rs", rs_file_1(), false),
        ];

        let tmp_root_dir = setup_tmp_files(files);

        setup_ignore_file(&tmp_root_dir, ".ignore", vec!["x/xa.rs"]);

        let expected = expected_paths(tmp_root_dir.path(), files);

        let actual = grep_single_file(tmp_root_dir.child("x/xa.rs")).unwrap();
        let actual_paths: Vec<&PathBuf> = actual.iter().map(|(p, _)| p).collect();
        assert_eq!(actual_paths, expected.iter().collect::<Vec<_>>());
        for (_, attrs) in &actual {
            assert!(!attrs.is_empty());
            assert_eq!(attrs.len(), 4, "rs_file_1 should yield 4 derive attrs");
        }
    }

    #[test]
    fn test_grep_single_file_path_is_dir() {
        let files = &[
            ("a.rs", rs_file_1(), false),
            ("b.rs", rs_file_1(), false),
            ("x/xa.rs", rs_file_1(), true),
            ("x/xb.rs", rs_file_1(), false),
        ];

        let tmp_root_dir = setup_tmp_files(files);

        let actual = grep_single_file(tmp_root_dir.child("x/"));

        assert!(actual.is_err());
    }

    #[test]
    fn test_grep_single_file_path_is_not_rs() {
        let files = &[
            ("a.rs", rs_file_1(), false),
            ("b.rs", rs_file_1(), false),
            ("x/xa.txt", rs_file_1(), true),
            ("x/xb.rs", rs_file_1(), false),
        ];

        let tmp_root_dir = setup_tmp_files(files);

        let actual = grep_single_file(tmp_root_dir.child("x/xa.txt"));

        assert!(actual.is_err());
    }

    fn rs_file_1() -> &'static str {
        "#[derive(Debug)]
struct A;

#[derive(Clone, Copy)]
struct B;

#[cfg_attr(feature = \"extra\", derive(PartialEq, Eq))]
struct C;

#[cfg_attr(all(feature = \"serde\", not(test)), derive(Deserialize, Serialize))]
struct D;
"
    }

    fn rs_file_2() -> &'static str {
        "struct A;\n"
    }

    type Files<'a> = &'a [(&'a str, &'a str, bool)];

    fn setup_tmp_files(files: Files) -> assert_fs::TempDir {
        let tmp_root_dir = assert_fs::TempDir::new().unwrap();
        for (path, content, _) in files.iter() {
            tmp_root_dir.child(path).write_str(*content).unwrap();
        }
        tmp_root_dir
    }

    fn expected_paths(tmp_root_path: &Path, files: Files) -> Vec<PathBuf> {
        files
            .iter()
            .filter(|(_, _, is_match)| *is_match)
            .map(|(p, _, _)| tmp_root_path.join(p))
            .collect()
    }

    fn setup_ignore_file(tmp_root_dir: &assert_fs::TempDir, ignore_file: &str, exclude: Vec<&str>) {
        tmp_root_dir
            .child(ignore_file)
            .write_str(&exclude.join("\n"))
            .unwrap();
    }
}
