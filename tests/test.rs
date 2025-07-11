use std::path::{Path, PathBuf};

use assert_cmd::Command;
use dircpy::copy_dir;
use tempfile::TempDir;

const BIN_NAME: &str = "cargo-sort-derives";
const BASE_COMMAND_NAME: &str = "sort-derives";
const INPUT_DIR: &str = "fixtures/input";
const EXPECTED_BASE_DIR: &str = "fixtures/expected";
const CONFIG_DIR: &str = "tests/config";

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn test_default() -> Result<()> {
    let dir = setup_input()?;
    execute(&[], dir.path())?;
    compare(dir, "default")
}

#[test]
fn test_order() -> Result<()> {
    let dir = setup_input()?;
    execute(&["--order", "Default, Debug"], dir.path())?;
    compare(dir, "order")
}

#[test]
fn test_order_head_ellipsis() -> Result<()> {
    let dir = setup_input()?;
    execute(&["--order", "..., Serialize, Deserialize"], dir.path())?;
    compare(dir, "order_head_ellipsis")
}

#[test]
fn test_order_middle_ellipsis() -> Result<()> {
    let dir = setup_input()?;
    execute(&["--order", "Eq, ..., Serialize, Deserialize"], dir.path())?;
    compare(dir, "order_middle_ellipsis")
}

#[test]
fn test_order_preserve() -> Result<()> {
    let dir = setup_input()?;
    execute(&["--order", "Default, Debug", "--preserve"], dir.path())?;
    compare(dir, "order_preserve")
}

#[test]
fn test_exclude() -> Result<()> {
    let dir = setup_input()?;
    let config_path = config_file_path("exclude.toml")?;
    execute(&["--config", &config_path], dir.path())?;
    compare(dir, "exclude")
}

fn setup_input() -> Result<TempDir> {
    let temp_dir = tempfile::tempdir()?;
    let temp_dir_path = temp_dir.path();
    let input_dir = Path::new(INPUT_DIR);
    copy_dir(input_dir, temp_dir_path)?;
    Ok(temp_dir)
}

fn config_file_path(file_name: &str) -> Result<String> {
    let current_dir = std::env::current_dir()?;
    let config_path = current_dir.join(CONFIG_DIR).join(file_name);
    Ok(config_path.to_string_lossy().into())
}

fn collect_file_path_pairs(p1: &Path, p2: &Path) -> Result<Vec<(PathBuf, PathBuf)>> {
    fn rec(p1: &Path, p2: &Path, pairs: &mut Vec<(PathBuf, PathBuf)>) -> Result<()> {
        for entry in p1.read_dir()? {
            let p1_path = entry?.path();
            let p2_path = p2.join(p1_path.file_name().unwrap());
            if !p2_path.exists() {
                return Err(format!("{} does not exist", p2_path.display()).into());
            }
            if p1_path.is_dir() {
                rec(&p1_path, &p2_path, pairs)?;
            } else {
                pairs.push((p1_path, p2_path));
            }
        }
        Ok(())
    }
    let mut pairs = vec![];
    rec(p1, p2, &mut pairs)?;
    Ok(pairs)
}

fn execute(args: &[&str], current_dir: &Path) -> Result<()> {
    Command::cargo_bin(BIN_NAME)?
        .arg(BASE_COMMAND_NAME)
        .args(args)
        .current_dir(current_dir)
        .assert()
        .success();
    Ok(())
}

fn compare(temp_dir: TempDir, expected_dir_name: &str) -> Result<()> {
    let expected_dir = Path::new(EXPECTED_BASE_DIR).join(expected_dir_name);
    let pairs = collect_file_path_pairs(temp_dir.path(), &expected_dir)?;

    let mut not_matched_files = vec![];
    for (src_path, dst_path) in pairs {
        let src_content = std::fs::read_to_string(&src_path)?;
        let dst_content = std::fs::read_to_string(&dst_path)?;
        if src_content != dst_content {
            let file_name: String = src_path.file_name().unwrap().to_string_lossy().into();
            not_matched_files.push(file_name);
        }
    }

    if not_matched_files.is_empty() {
        Ok(())
    } else {
        Err(format!("Not matched files: {not_matched_files:?}").into())
    }
}
