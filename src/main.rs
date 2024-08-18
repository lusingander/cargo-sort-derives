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
    /// Any derives not listed will appear at the end in alphabetical order by default
    #[clap(short, long, value_name = "VALUE", verbatim_doc_comment)]
    order: Option<String>,

    /// Check if the derive attributes are sorted
    #[clap(long)]
    check: bool,
}

fn parse_order(order: &str) -> Vec<&str> {
    order.split(',').map(str::trim).collect()
}

fn main() {
    let Cli::SortDerives(args) = Cli::parse();

    let custom_order = args.order.as_deref().map(parse_order);
    let check = args.check;

    let mut no_diff = true;
    for (file_path, line_numbers) in grep().unwrap() {
        no_diff &= process_file(&file_path, line_numbers, &custom_order, check).unwrap();
    }

    if !no_diff {
        std::process::exit(1);
    }
}
