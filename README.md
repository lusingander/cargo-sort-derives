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
  -p, --path <FILE>    The path to the file to sort
                       If not specified, all .rs files in the current directory will be sorted
      --order <VALUE>  Define the custom order of derive attributes, separated by commas (e.g. "Debug, Clone, Copy")
                       Any derives not listed will appear at the end in alphabetical order by default
      --preserve       Preserve the original order for unspecified derive attributes (only applies when --order is used)
      --check          Check if the derive attributes are sorted
      --color <TYPE>   Use colored output [default: auto] [possible values: auto, always, never]
      --config <FILE>  The path to the config file
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

You can also specify the order in the `order` section of the [config file](#config).

#### Ellipsis

If you specify an ellipsis (`...`), the `derive` attributes before it will be placed at the beginning, the elements after it will be placed at the end, and the remaining will be sorted alphabetically (if `--preserve` option is not specified).

```
$ cargo sort-derives --order "Eq, ..., Default, Clone"
```

This will reorder the `derive` attributes as follows:

```rs
// Before:
#[derive(Debug, Clone, Copy, Default, Eq)]
struct Example;

// After: "Eq" at the beginning, "Default, Clone" at the end, in that order, with the rest in between in alphabetical order.
#[derive(Eq, Copy, Debug, Default, Clone)]
struct Example;
```

Ellipsis (`...`) cannot be specified multiple times.

### Check without updates

<img src="./img/check.gif" width=600>

```
$ cargo sort-derives --check
```

This checks if the `derive` attributes in your `.rs` files are sorted correctly.

If the attributes are out of order, the command will exit with a non-zero status code, indicating that the files need to be updated.

### Process only specific files

```
$ cargo sort-derives --path ./path/to/file.rs
```

You can sort only the files specified by the `--path` option.

You cannot specify a directory or multiple paths. Also, all [exclusions](#exclude-targets) are ignored.

### Exclude targets

If the `.gitignore` or `.ignore` file exists, the files listed there will be excluded.

You can specify files to exclude in the `exclude` section of the [config file](#config).

### Config

If `.sort-derives.toml` or `sort-derives.toml` exists in the current directory, the config will be loaded.

You can also specify a configuration file with the `--config` option. In this case, the specified file will be read with priority.

#### Format

The config file uses the following format:

```toml
# Define the custom order of derive attributes.
# The command line option `--order` will override this setting if specified.
# type: array of strings | string
order = [
  "Eq",
  "Clone",
  "Default",
]
# Alternatively, it can be set as a comma separated string, similar to the `--order`.
# order = "Eq, Clone, Default"

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
