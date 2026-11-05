use std::collections::HashSet;

use proc_macro2::{Delimiter, TokenTree};
use syn::Meta;
use syn::parse::Parser;
use syn::punctuated::Punctuated;

/// The condition text of a `cfg_attr(CONDITION, derive(...))` attribute,
/// or `None` for a plain `derive(...)` attribute.
pub type CfgCondition = Option<String>;

/// One sortable derive attribute, resolved to byte offsets in the source file.
pub struct DeriveAttr {
    /// Byte offset of `#` (inclusive).
    pub start: usize,
    /// Byte offset just past the closing `]` (exclusive).
    pub end: usize,
    /// Derive paths in original order.
    pub paths: Vec<syn::Path>,
    /// `Some(condition_text)` for `cfg_attr` form, `None` for plain `derive`.
    pub condition: CfgCondition,
}

/// Converts a `proc_macro2::LineColumn` to a byte index in `source`.
/// `line` is 1-indexed, `col` is 0-indexed byte offset within the line.
fn line_col_to_byte(source: &str, line: usize, col: usize) -> usize {
    let line_start = source
        .split('\n')
        .take(line - 1)
        .map(|l| l.len() + 1) // +1 for '\n'
        .sum::<usize>();
    line_start + col
}

/// Parse `source` and return all sortable derive attributes with byte spans.
///
/// `disabled_lines` is the set of 1-indexed source line numbers where sorting
/// is suppressed (pre-computed by the caller from disable comments).
///
/// Uses `proc_macro2` token-level scanning so that files with custom proc-macro
/// syntax (e.g. `duplicate_item`) are handled correctly — only individual
/// `#[…]` attribute tokens are parsed with `syn`, not the entire file.
pub fn collect_derive_attrs(
    source: &str,
    disabled_lines: &HashSet<usize>,
) -> syn::Result<Vec<DeriveAttr>> {
    let tokens: proc_macro2::TokenStream = source.parse().map_err(|e: proc_macro2::LexError| {
        syn::Error::new(proc_macro2::Span::call_site(), e.to_string())
    })?;

    let mut attrs = Vec::new();
    collect_from_tokens(&mut tokens.into_iter(), &mut attrs, source, disabled_lines)?;
    attrs.sort_by_key(|a| a.start);
    Ok(attrs)
}

/// Recursively walk a token-tree iterator, collecting derive attributes.
fn collect_from_tokens(
    iter: &mut dyn Iterator<Item = TokenTree>,
    attrs: &mut Vec<DeriveAttr>,
    source: &str,
    disabled_lines: &HashSet<usize>,
) -> syn::Result<()> {
    while let Some(tt) = iter.next() {
        if let TokenTree::Group(g) = &tt {
            if g.delimiter() == Delimiter::Brace
                || g.delimiter() == Delimiter::Bracket
                || g.delimiter() == Delimiter::Parenthesis
            {
                collect_from_tokens(&mut g.stream().into_iter(), attrs, source, disabled_lines)?;
            }
        }

        if !is_punct(&tt, '#') {
            continue;
        }
        if let Some(next) = iter.next() {
            // Skip inner attributes (#![…])
            if is_punct(&next, '!') {
                // Consume the bracket group that follows
                let _group = iter.next();
                continue;
            }
            if is_bracket_group(&next) {
                let attr_tokens: proc_macro2::TokenStream = [tt, next].into_iter().collect();
                if let Ok(parsed) = syn::Attribute::parse_outer.parse2(attr_tokens) {
                    for attr in parsed {
                        process_attribute(&attr, attrs, source, disabled_lines)?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn is_punct(tt: &TokenTree, c: char) -> bool {
    matches!(tt, TokenTree::Punct(p) if p.as_char() == c)
}

fn is_bracket_group(tt: &TokenTree) -> bool {
    matches!(tt, TokenTree::Group(g) if g.delimiter() == Delimiter::Bracket)
}

fn process_attribute(
    attr: &syn::Attribute,
    results: &mut Vec<DeriveAttr>,
    source: &str,
    disabled_lines: &HashSet<usize>,
) -> syn::Result<()> {
    let start_lc = attr.pound_token.span.start();
    let start = line_col_to_byte(source, start_lc.line, start_lc.column);

    // Compute 1-indexed line number of the `#` byte.
    let line = source[..start].chars().filter(|&c| c == '\n').count() + 1;
    if disabled_lines.contains(&line) {
        return Ok(());
    }

    if attr.path().is_ident("derive") {
        process_plain_derive(attr, results, source, start)?;
    } else if attr.path().is_ident("cfg_attr") {
        process_cfg_attr_derive(attr, results, source, start)?;
    }

    Ok(())
}

fn process_plain_derive(
    attr: &syn::Attribute,
    results: &mut Vec<DeriveAttr>,
    source: &str,
    start: usize,
) -> syn::Result<()> {
    if let Meta::List(meta_list) = &attr.meta {
        let paths: Punctuated<syn::Path, syn::Token![,]> =
            meta_list.parse_args_with(Punctuated::parse_terminated)?;
        let end_lc = attr.bracket_token.span.close().end();
        let end = line_col_to_byte(source, end_lc.line, end_lc.column);
        results.push(DeriveAttr {
            start,
            end,
            paths: paths.into_iter().collect(),
            condition: None,
        });
    }
    Ok(())
}

fn process_cfg_attr_derive(
    attr: &syn::Attribute,
    results: &mut Vec<DeriveAttr>,
    source: &str,
    start: usize,
) -> syn::Result<()> {
    use proc_macro2::TokenTree;

    if let Meta::List(meta_list) = &attr.meta {
        let content_tokens = meta_list.tokens.clone();
        let tt_vec: Vec<TokenTree> = content_tokens.into_iter().collect();

        // Find the last top-level comma (commas inside groups are nested and hidden).
        let comma_idx = tt_vec
            .iter()
            .enumerate()
            .rposition(|(_, tt)| matches!(tt, TokenTree::Punct(p) if p.as_char() == ','));

        let Some(idx) = comma_idx else {
            return Ok(());
        };

        // Extract condition text from source using span byte offsets to preserve formatting.
        let comma_span = match &tt_vec[idx] {
            TokenTree::Punct(p) => p.span(),
            _ => return Ok(()),
        };
        let condition_start = tt_vec[0].span().start();
        let condition_start_byte =
            line_col_to_byte(source, condition_start.line, condition_start.column);
        let condition_end_byte =
            line_col_to_byte(source, comma_span.start().line, comma_span.start().column);
        let condition: String = source[condition_start_byte..condition_end_byte].to_string();

        let derive_tokens: proc_macro2::TokenStream = tt_vec[idx + 1..].iter().cloned().collect();

        let derive_meta: Meta = syn::parse2(derive_tokens)?;
        if !derive_meta.path().is_ident("derive") {
            return Ok(());
        }
        if let Meta::List(derive_list) = derive_meta {
            let paths: Punctuated<syn::Path, syn::Token![,]> =
                derive_list.parse_args_with(Punctuated::parse_terminated)?;

            let end_lc = attr.bracket_token.span.close().end();
            let end = line_col_to_byte(source, end_lc.line, end_lc.column);

            results.push(DeriveAttr {
                start,
                end,
                paths: paths.into_iter().collect(),
                condition: Some(condition),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn first_segment(path: &syn::Path) -> String {
        path.segments.first().unwrap().ident.to_string()
    }

    #[test]
    fn test_collect_plain_derive() {
        let source = "#[derive(Debug, Clone)]\nstruct Foo;";
        let attrs = collect_derive_attrs(source, &HashSet::new()).unwrap();
        assert_eq!(attrs.len(), 1);
        assert_eq!(attrs[0].paths.len(), 2);

        assert_eq!(first_segment(&attrs[0].paths[0]), "Debug");
        assert_eq!(first_segment(&attrs[0].paths[1]), "Clone");

        assert!(attrs[0].condition.is_none());
        assert_eq!(attrs[0].start, source.find('#').unwrap());
        assert_eq!(attrs[0].end, source.rfind(']').unwrap() + 1);
    }

    #[test]
    fn test_collect_cfg_attr_derive() {
        let source =
            "#[cfg_attr(feature = \"serde\", derive(Serialize, Deserialize))]\nstruct Foo;";
        let attrs = collect_derive_attrs(source, &HashSet::new()).unwrap();
        assert_eq!(attrs.len(), 1);
        assert_eq!(attrs[0].paths.len(), 2);

        assert_eq!(first_segment(&attrs[0].paths[0]), "Serialize");
        assert_eq!(first_segment(&attrs[0].paths[1]), "Deserialize");

        assert!(attrs[0].condition.is_some());
        assert_eq!(attrs[0].condition.as_ref().unwrap(), "feature = \"serde\"");

        assert_eq!(attrs[0].start, source.find('#').unwrap());
        assert_eq!(attrs[0].end, source.rfind(']').unwrap() + 1);
    }

    #[test]
    fn test_collect_multiline_derive() {
        let source = "#[derive(\n    Debug,\n    Clone,\n)]\nstruct Foo;";
        let attrs = collect_derive_attrs(source, &HashSet::new()).unwrap();
        assert_eq!(attrs.len(), 1);
        assert_eq!(attrs[0].paths.len(), 2);

        assert_eq!(first_segment(&attrs[0].paths[0]), "Debug");
        assert_eq!(first_segment(&attrs[0].paths[1]), "Clone");

        assert_eq!(attrs[0].start, source.find('#').unwrap());
        assert_eq!(attrs[0].end, source.rfind(']').unwrap() + 1);
    }

    #[test]
    fn test_collect_with_disabled_lines() {
        let source = "// cargo-sort-derives-disable\n#[derive(Debug, Clone)]\nstruct Foo;";
        let mut disabled = HashSet::new();
        disabled.insert(2); // line 2 contains the derive
        let attrs = collect_derive_attrs(source, &disabled).unwrap();
        assert!(attrs.is_empty());
    }
}
