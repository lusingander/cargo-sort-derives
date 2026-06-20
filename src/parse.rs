use std::collections::HashSet;

use proc_macro2::TokenStream;
use syn::Meta;
use syn::punctuated::Punctuated;

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
        .map(|l| l.len() + 1) // +1 for '\n'
        .sum::<usize>();
    line_start + col
}

/// Parse `source` and return all sortable derive attributes with byte spans.
///
/// `disabled_lines` is the set of 1-indexed source line numbers where sorting
/// is suppressed (pre-computed by the caller from disable comments).
pub fn collect_derive_attrs(
    source: &str,
    disabled_lines: &HashSet<usize>,
) -> syn::Result<Vec<DeriveAttr>> {
    let file = syn::parse_file(source)?;
    let mut attrs = Vec::new();
    visit_items(&file.items, &mut attrs, source, disabled_lines)?;
    attrs.sort_by_key(|a| a.start);
    Ok(attrs)
}

fn visit_items(
    items: &[syn::Item],
    results: &mut Vec<DeriveAttr>,
    source: &str,
    disabled_lines: &HashSet<usize>,
) -> syn::Result<()> {
    for item in items {
        visit_item(item, results, source, disabled_lines)?;
    }
    Ok(())
}

fn visit_item(
    item: &syn::Item,
    results: &mut Vec<DeriveAttr>,
    source: &str,
    disabled_lines: &HashSet<usize>,
) -> syn::Result<()> {
    visit_attrs(&item_attrs(item), results, source, disabled_lines)?;

    match item {
        syn::Item::Mod(item_mod) => {
            if let Some((_, items)) = &item_mod.content {
                visit_items(items, results, source, disabled_lines)?;
            }
        }
        syn::Item::Impl(item_impl) => {
            for impl_item in &item_impl.items {
                visit_attrs(&impl_item_attrs(impl_item), results, source, disabled_lines)?;
            }
        }
        syn::Item::Trait(item_trait) => {
            for trait_item in &item_trait.items {
                visit_attrs(
                    &trait_item_attrs(trait_item),
                    results,
                    source,
                    disabled_lines,
                )?;
            }
        }
        syn::Item::Enum(item_enum) => {
            for variant in &item_enum.variants {
                visit_attrs(&variant.attrs, results, source, disabled_lines)?;
            }
        }
        syn::Item::Struct(item_struct) => {
            for field in item_struct.fields.iter() {
                visit_attrs(&field.attrs, results, source, disabled_lines)?;
            }
        }
        syn::Item::Union(item_union) => {
            for field in item_union.fields.named.iter() {
                visit_attrs(&field.attrs, results, source, disabled_lines)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Extract attrs from any syn::Item variant.
fn item_attrs(item: &syn::Item) -> &[syn::Attribute] {
    match item {
        syn::Item::Const(i) => &i.attrs,
        syn::Item::Enum(i) => &i.attrs,
        syn::Item::ExternCrate(i) => &i.attrs,
        syn::Item::Fn(i) => &i.attrs,
        syn::Item::ForeignMod(i) => &i.attrs,
        syn::Item::Impl(i) => &i.attrs,
        syn::Item::Macro(i) => &i.attrs,
        syn::Item::Mod(i) => &i.attrs,
        syn::Item::Static(i) => &i.attrs,
        syn::Item::Struct(i) => &i.attrs,
        syn::Item::Trait(i) => &i.attrs,
        syn::Item::TraitAlias(i) => &i.attrs,
        syn::Item::Type(i) => &i.attrs,
        syn::Item::Union(i) => &i.attrs,
        syn::Item::Use(i) => &i.attrs,
        _ => &[],
    }
}

fn impl_item_attrs(item: &syn::ImplItem) -> &[syn::Attribute] {
    match item {
        syn::ImplItem::Const(i) => &i.attrs,
        syn::ImplItem::Fn(i) => &i.attrs,
        syn::ImplItem::Type(i) => &i.attrs,
        syn::ImplItem::Macro(i) => &i.attrs,
        _ => &[],
    }
}

fn trait_item_attrs(item: &syn::TraitItem) -> &[syn::Attribute] {
    match item {
        syn::TraitItem::Const(i) => &i.attrs,
        syn::TraitItem::Fn(i) => &i.attrs,
        syn::TraitItem::Type(i) => &i.attrs,
        syn::TraitItem::Macro(i) => &i.attrs,
        _ => &[],
    }
}

fn visit_attrs(
    attrs: &[syn::Attribute],
    results: &mut Vec<DeriveAttr>,
    source: &str,
    disabled_lines: &HashSet<usize>,
) -> syn::Result<()> {
    for attr in attrs {
        process_attribute(attr, results, source, disabled_lines)?;
    }
    Ok(())
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

        let condition: TokenStream = tt_vec[..idx].iter().cloned().collect();
        let derive_tokens: TokenStream = tt_vec[idx + 1..].iter().cloned().collect();

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
        let cond_str = attrs[0].condition.as_ref().unwrap().to_string();
        assert!(cond_str.contains("feature"));

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
