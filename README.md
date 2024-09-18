# cargo-sort-derives

[![Crate Status](https://img.shields.io/crates/v/cargo-sort-derives.svg)](https://crates.io/crates/cargo-sort-derives)

Cargo subcommand to sort derive attributes

## About

A Cargo subcommand that helps you consistently order `derive` attributes in your Rust code.

This tool ensures that the `derive` attributes in your structs and enums are sorted in a consistent.

## Installation

```
$ cargo install --locked cargo-sort-derives
```

## Usage

```
Usage: cargo sort-derives [OPTIONS]

Options:
      --order <VALUE>  Define the custom order of derive attributes, separated by commas (e.g. "Debug, Clone, Copy")
                       Any derives not listed will appear at the end in alphabetical order by default
      --preserve       Preserve the original order for unspecified derive attributes (only applies when --order is used)
      --check          Check if the derive attributes are sorted
      --color <TYPE>   Use colored output [default: auto] [possible values: auto, always, never]
  -h, --help           Print help
  -V, --version        Print version
```

### Basic

<img src="./img/basic.gif" width=600>

The following command sorts the `derive` attributes in the `.rs` files in the current directory:

```
$ cargo sort-derives
```

This will reorder the `derive` attributes as follows:

```rs
// Before:
#[derive(Debug, Clone, Copy, Default, Eq)]
struct Example;

// After: sorted alphabetically
#[derive(Clone, Copy, Debug, Default, Eq)]
struct Example;
```

By default, it is sorted alphabetically.

### Specifying the order

<img src="./img/order.gif" width=600>

```
$ cargo sort-derives --order "Eq, Clone, Default"
```

This will reorder the `derive` attributes as follows:

```rs
// Before:
#[derive(Debug, Clone, Copy, Default, Eq)]
struct Example;

// After: "Eq, Clone, Default" are sorted in that order, the rest are sorted alphabetically
#[derive(Eq, Clone, Default, Copy, Debug)]
struct Example;
```

Any derives not listed will appear at the end in alphabetical order.

The `--preserve` option allows you to maintain the original order of `derive` attributes that are not specified in the `--order` option.

```
$ cargo sort-derives --order "Eq, Clone, Default" --preserve
```

This will reorder the `derive` attributes as follows:

```rs
// Before:
#[derive(Debug, Clone, Copy, Default, Eq)]
struct Example;

// After: "Eq, Clone, Default" are sorted in that order, the rest keep the original order
#[derive(Eq, Clone, Default, Debug, Copy)]
struct Example;
```

### Check without updates

<img src="./img/check.gif" width=600>

```
$ cargo sort-derives --check
```

This checks if the `derive` attributes in your `.rs` files are sorted correctly.

If the attributes are out of order, the command will exit with a non-zero status code, indicating that the files need to be updated.

### Exclude targets

If the `.gitignore` or `.ignore` file exists, the files listed there will be excluded.

You can specify files to exclude in the `exclude` section of the [config file](#config).

### Config

If `.sort-derives.toml` exists in the current directory, the config will be loaded.

#### Format

The `.sort-derives.toml` file uses the following format:

```toml
# Define the custom order of derive attributes, separated by commas (e.g. "Debug, Clone, Copy")
# The command line option `--order` will override this setting if specified.
# type: string
order = "Eq, Clone, Default"
# Preserve the original order for unspecified derive attributes (only applies when custom order is used)
# The command line option `--preserve` will override this setting if specified.
# type: boolean
preserve = true

# Specify file path patterns to exclude from processing using the .gitignore format.
# https://git-scm.com/docs/gitignore/en#_pattern_format
# type: array of strings
exclude = [
  "generated.rs",
  "/tests/*",
]
```

## License

MIT
