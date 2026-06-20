mod config;
mod console;
mod grep;
mod parse;
mod process;
mod sort;
mod util;

use std::{io::Read, path::Path};

use clap::{Args, Parser, ValueEnum};

use crate::{
    config::Config,
    console::{Console, OutputColor},
    grep::grep,
    process::process,
    sort::{derive_line_numbers, sort_source, sort_stdin},
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

    /// Suppress output to stderr
    #[clap(short, long)]
    quiet: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Color {
    Auto,
    Always,
    Never,
}

impl From<Color> for OutputColor {
    fn from(color: Color) -> Self {
        match color {
            Color::Auto => OutputColor::Auto,
            Color::Always => OutputColor::Always,
            Color::Never => OutputColor::Never,
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
    let config = Config::load(&args.config);

    let custom_order = read_custom_order(&config, &args)?;
    let preserve = read_preserve(&config, &args);
    let exclude = read_exclude(&config);
    let path = args.path;
    let check = args.check;
    let stdin = args.stdin;
    let quiet = args.quiet;
    let output_color = args.color.into();

    let output_color = if std::env::var_os("NO_COLOR").is_some() {
        OutputColor::Never
    } else {
        output_color
    };

    let console = Console::new(quiet, output_color);
    console.print_config(&custom_order, preserve, &exclude);

    let mut total = 0usize;
    let mut changed_count = 0usize;
    let mut unchanged_count = 0usize;

    if stdin {
        let mut input = String::new();
        std::io::stdin().read_to_string(&mut input)?;
        let (old_lines, new_lines, attrs, per_attr_changed) =
            sort_stdin(&input, &custom_order, preserve)?;
        let derive_lines = derive_line_numbers(&input, &attrs);

        total = attrs.len();
        for &changed in &per_attr_changed {
            if changed {
                changed_count += 1;
            } else {
                unchanged_count += 1;
            }
        }

        let no_diff = process(
            Path::new("<stdin>"),
            old_lines,
            new_lines.clone(),
            check,
            &derive_lines,
            &mut |dc| console.on_derive(&dc),
        )?;

        if check {
            if !no_diff {
                console.print_check_diff(Path::new("<stdin>"), &input, &new_lines);
                std::process::exit(1);
            }
        } else {
            print!("{}", new_lines);
        }

        console.print_summary(changed_count, unchanged_count, total);
        return Ok(());
    }

    let mut no_diff = true;
    for (file_path, attrs) in grep(path, exclude)? {
        let content = std::fs::read_to_string(&file_path)
            .map_err(|e| format!("{}: {}", file_path.display(), e))?;
        let derive_lines = derive_line_numbers(&content, &attrs);
        let (old_lines, new_lines, per_attr_changed) =
            sort_source(content, &attrs, &custom_order, preserve);

        total += attrs.len();
        for &changed in &per_attr_changed {
            if changed {
                changed_count += 1;
            } else {
                unchanged_count += 1;
            }
        }

        let file_no_diff = process(
            &file_path,
            old_lines.clone(),
            new_lines.clone(),
            check,
            &derive_lines,
            &mut |dc| console.on_derive(&dc),
        )?;

        if check && !file_no_diff {
            console.print_check_diff(&file_path, &old_lines, &new_lines);
        }

        no_diff &= file_no_diff;
    }

    console.print_summary(changed_count, unchanged_count, total);

    if !no_diff {
        std::process::exit(1);
    }

    Ok(())
}
