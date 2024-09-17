mod config;
mod ext;
mod grep;
mod sort;

use clap::{Args, Parser, ValueEnum};
use config::Config;
use grep::grep;
use sort::process_file;

#[derive(Debug, Parser)]
#[command(name = "cargo", bin_name = "cargo")]
enum Cli {
    SortDerives(SortDerivesArgs),
}

#[derive(Debug, Args)]
#[command(version, about, long_about = None)]
struct SortDerivesArgs {
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

impl From<Color> for sort::OutputColor {
    fn from(color: Color) -> Self {
        match color {
            Color::Auto => sort::OutputColor::Auto,
            Color::Always => sort::OutputColor::Always,
            Color::Never => sort::OutputColor::Never,
        }
    }
}

fn read_custom_order<'a>(config: &'a Config, args: &'a SortDerivesArgs) -> Option<Vec<&'a str>> {
    let order_str = args.order.as_deref().or(config.order.as_deref());
    order_str.map(parse_order)
}

fn parse_order(order: &str) -> Vec<&str> {
    order.split(',').map(str::trim).collect()
}

fn read_preserve(config: &Config, args: &SortDerivesArgs) -> bool {
    args.preserve || config.preserve.unwrap_or(false)
}

fn read_exclude(config: &Config) -> Vec<String> {
    config.exclude.clone().unwrap_or_default()
}

fn main() {
    let Cli::SortDerives(args) = Cli::parse();
    let config = Config::load();

    let custom_order = read_custom_order(&config, &args);
    let preserve = read_preserve(&config, &args);
    let exclude = read_exclude(&config);
    let check = args.check;
    let output_color = args.color.into();

    let mut no_diff = true;
    for (file_path, line_numbers) in grep(exclude).unwrap() {
        no_diff &= process_file(
            &file_path,
            line_numbers,
            &custom_order,
            preserve,
            check,
            output_color,
        )
        .unwrap();
    }

    if !no_diff {
        std::process::exit(1);
    }
}
