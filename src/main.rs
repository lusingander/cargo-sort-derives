mod config;
mod ext;
mod grep;
mod process;
mod sort;
mod util;

use clap::{Args, Parser, ValueEnum};

use crate::{config::Config, grep::grep, process::process, sort::sort, util::parse_order};

#[derive(Debug, Parser)]
#[command(name = "cargo", bin_name = "cargo")]
enum Cli {
    SortDerives(SortDerivesArgs),
}

#[derive(Debug, Args)]
#[command(version, about, long_about = None)]
struct SortDerivesArgs {
    /// The path to the file to sort
    /// If not specified, all `.rs` files in the current directory will be sorted
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

    /// Use colored output
    #[clap(long, value_name = "TYPE", default_value = "auto")]
    color: Color,
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

fn read_custom_order<'a>(config: &'a Config, args: &'a SortDerivesArgs) -> Option<Vec<String>> {
    args.order.clone().map(parse_order).or(config.order.clone())
}

fn read_preserve(config: &Config, args: &SortDerivesArgs) -> bool {
    args.preserve || config.preserve.unwrap_or(false)
}

fn read_exclude(config: &Config) -> Vec<String> {
    config.exclude.clone().unwrap_or_default()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Cli::SortDerives(args) = Cli::parse();
    let config = Config::load();

    let custom_order = read_custom_order(&config, &args);
    let preserve = read_preserve(&config, &args);
    let exclude = read_exclude(&config);
    let path = args.path;
    let check = args.check;
    let output_color = args.color.into();

    let mut no_diff = true;
    for (file_path, line_numbers) in grep(path, exclude)? {
        let (old_lines, new_lines) = sort(&file_path, line_numbers, &custom_order, preserve)?;
        no_diff &= process(&file_path, old_lines, new_lines, check, output_color)?;
    }

    if !no_diff {
        std::process::exit(1);
    }

    Ok(())
}
