mod grep;

use std::{collections::HashSet, io::BufRead, path::PathBuf, sync::LazyLock};

use grep::grep;
use regex::Regex;

const PATTERN: &str = r"#\[derive\(\s*([^\)]+?)\s*\)\]";
static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(PATTERN).unwrap());

fn process_file(file_path: PathBuf, line_numbers: HashSet<usize>) -> Result<(), std::io::Error> {
    let path = file_path.clone();
    let file = std::fs::File::open(file_path)?;
    let reader = std::io::BufReader::new(file);

    for (i, line) in reader.lines().enumerate() {
        let n = i + 1;
        let line = line?;

        if line_numbers.contains(&n) {
            let derives = parse_derive_traits(&line);
            println!("{}:{}: {:?}", path.display(), n, derives);
        }
    }

    Ok(())
}

fn parse_derive_traits(line: &str) -> Vec<&str> {
    let caps = RE.captures(line).unwrap();
    caps.get(1)
        .unwrap()
        .as_str()
        .split(',')
        .map(|s| s.trim())
        .collect()
}

fn main() {
    for (file_path, line_numbers) in grep().unwrap() {
        process_file(file_path, line_numbers).unwrap();
    }
}
