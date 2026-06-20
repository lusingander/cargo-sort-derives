mod config;
mod ext;
mod grep;
mod process;
mod sort;
mod util;

use std::{
    io::Read,
    path::{Path, PathBuf},
};

use clap::{Args, Parser, ValueEnum};

use crate::{
    config::Config,
    grep::grep,
    process::process,
    sort::{build_order_map, sort, sort_stdin},
    util::parse_order,
};

#[derive(Debug, Parser)]
#[command(name = "cargo", bin_name = "cargo")]
enum Cli {
    SortDerives(SortDerivesArgs),
}

#[derive(Debug, Args)]
#[command(version, about, long_about = None)]
struct SortDerivesArgs {
    /// The path to the file to sort
    /// If not specified, all .rs files in the current directory will be sorted
    #[clap(short, long, value_name = "FILE", verbatim_doc_comment)]
    path: Option<String>,

    /// Define the custom order of derive attributes, separated by commas (e.g. "Debug, Clone, Copy")
    /// Any derives not listed will appear at the end in alphabetical order by default
    #[clap(long, value_name = "VALUE", verbatim_doc_comment)]
    order: Option<String>,

    /// Preserve the original order for unspecified derive attributes (only applies when --order is used)
    #[clap(long)]
    preserve: bool,

    /// Check if the derive attributes are sorted
    #[clap(long)]
    check: bool,

    /// Read Rust source from stdin and write formatted source to stdout
    #[clap(long, conflicts_with = "path")]
    stdin: bool,

    /// Use colored output
    #[clap(long, value_name = "TYPE", default_value = "auto")]
    color: Color,

    /// The path to the config file
    #[clap(long, value_name = "FILE")]
    config: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Color {
    Auto,
    Always,
    Never,
}

impl From<Color> for process::OutputColor {
    fn from(color: Color) -> Self {
        match color {
            Color::Auto => process::OutputColor::Auto,
            Color::Always => process::OutputColor::Always,
            Color::Never => process::OutputColor::Never,
        }
    }
}

fn read_custom_order<'a>(
    config: &'a Config,
    args: &'a SortDerivesArgs,
) -> Result<Option<Vec<String>>, String> {
    let order = args.order.clone().map(parse_order).or(config.order.clone());
    if let Some(order) = &order {
        if order.iter().filter(|s| *s == "...").count() > 1 {
            return Err("Only one '...' is allowed in the custom order".to_string());
        }
    }
    Ok(order)
}

fn read_preserve(config: &Config, args: &SortDerivesArgs) -> bool {
    args.preserve || config.preserve.unwrap_or(false)
}

fn read_exclude(config: &Config) -> Vec<String> {
    config.exclude.clone().unwrap_or_default()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Cli::SortDerives(args) = Cli::parse();
    let config = Config::load(args.config.as_ref())?;

    let custom_order = read_custom_order(&config, &args)?;
    let preserve = read_preserve(&config, &args);
    let exclude = read_exclude(&config);
    let path = args.path;
    let check = args.check;
    let stdin = args.stdin;
    let output_color = args.color.into();

    let order_map = build_order_map(custom_order.as_ref());

    if stdin {
        let mut input = String::new();
        std::io::stdin().read_to_string(&mut input)?;
        // stdin input is already the whole target, so file discovery via grep is not needed.
        let (old_lines, new_lines) = sort_stdin(&input, &order_map, preserve)?;

        if check {
            if !process(
                // process only uses this path when rendering check diffs.
                Path::new("<stdin>"),
                old_lines,
                new_lines,
                true,
                output_color,
            )? {
                std::process::exit(1);
            }
        } else {
            print!("{}", new_lines.concat());
        }

        return Ok(());
    }

    // Pass 1: read-only — collect all (file_path, old_lines, new_lines) tuples
    let mut results: Vec<(PathBuf, Vec<String>, Vec<String>)> = Vec::new();
    for (file_path, line_numbers) in grep(path, exclude)? {
        let (old_lines, new_lines) = sort(&file_path, line_numbers, &order_map, preserve)?;
        results.push((file_path, old_lines, new_lines));
    }

    // Pass 2: write/check
    let mut no_diff = true;
    for (file_path, old_lines, new_lines) in results {
        no_diff &= process(&file_path, old_lines, new_lines, check, output_color)?;
    }

    if !no_diff {
        std::process::exit(1);
    }

    Ok(())
}
