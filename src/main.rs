mod ext;
mod grep;
mod sort;

use clap::{Args, Parser};
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
    #[clap(short, long, value_name = "VALUE")]
    order: Option<String>,

    /// Check if the derive attributes are sorted
    #[clap(long)]
    check: bool,
}

const DEFAULT_ORDER: &str = "Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash";

fn parse_order(order: &str) -> Vec<&str> {
    order.split(',').map(str::trim).collect()
}

fn main() {
    let Cli::SortDerives(args) = Cli::parse();

    let order = &parse_order(args.order.as_deref().unwrap_or(DEFAULT_ORDER));
    let check = args.check;

    let mut no_diff = true;
    for (file_path, line_numbers) in grep().unwrap() {
        no_diff &= process_file(&file_path, line_numbers, order, check).unwrap();
    }

    if !no_diff {
        std::process::exit(1);
    }
}
