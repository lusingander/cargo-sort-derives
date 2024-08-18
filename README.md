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
                       Any derives not listed will appear at the end in alphabetical order by default
      --check          Check if the derive attributes are sorted
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

### Check without updates

<img src="./img/check.gif" width=600>

```
$ cargo sort-derives --check
```

This checks if the `derive` attributes in your `.rs` files are sorted correctly.

If the attributes are out of order, the command will exit with a non-zero status code, indicating that the files need to be updated.

## License

MIT
