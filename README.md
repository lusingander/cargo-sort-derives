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
  -o, --order <VALUE>  Define the custom order of derive attributes, separated by commas (e.g. "Debug, Clone, Copy")
      --check          Check if the derive attributes are sorted
  -h, --help           Print help
  -V, --version        Print version
```

### Basic

The following command sorts the `derive` attributes in the `.rs` files in the current directory:

```
$ cargo sort-derives
```

### Specifying the order

```
$ cargo sort-derives --order "Copy, Clone, Default"
```

This will reorder the `derive` attributes as follows:

```rs
// Before:
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Example;

// After:
#[derive(Copy, Clone, Default, Debug, PartialEq)]
struct Example;
```

Any derives not listed will appear at the end in their original order.

If nothing is specified, the default order is `Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash`.

### Check without updates

```
$ cargo sort-derives --check
```

This checks if the `derive` attributes in your `.rs` files are sorted correctly.

If the attributes are out of order, the command will exit with a non-zero status code, indicating that the files need to be updated.

## License

MIT
