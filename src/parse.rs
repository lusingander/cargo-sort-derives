/// The condition tokens of a `cfg_attr(CONDITION, derive(...))` attribute,
/// or `None` for a plain `derive(...)` attribute.
pub type CfgCondition = Option<proc_macro2::TokenStream>;

/// One sortable derive attribute, resolved to byte offsets in the source file.
pub struct DeriveAttr {
    /// Byte offset of `#` (inclusive).
    pub start: usize,
    /// Byte offset just past the closing `]` (exclusive).
    pub end: usize,
    /// Derive paths in original order.
    pub paths: Vec<syn::Path>,
    /// `Some(tokens)` for `cfg_attr` form, `None` for plain `derive`.
    pub condition: CfgCondition,
}

/// Converts a `proc_macro2::LineColumn` to a byte index in `source`.
/// `line` is 1-indexed, `col` is 0-indexed byte offset within the line.
fn line_col_to_byte(source: &str, line: usize, col: usize) -> usize {
    let line_start = source
        .split('\n')
        .take(line - 1)
        .map(|l| l.len() + 1)   // +1 for '\n'
        .sum::<usize>();
    line_start + col
}
