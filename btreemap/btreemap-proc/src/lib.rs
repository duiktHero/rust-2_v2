//! Procedural `btreemap!` macro (proc_macro)

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Parse, parse_macro_input, Expr, Token, Result};
use syn::punctuated::Punctuated;

/// Один елемент: `key => value`
struct Kv {
    key: Expr,
    _fat: Token![=>],
    val: Expr,
}

impl Parse for Kv {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        Ok(Self {
            key: input.parse()?,
            _fat: input.parse()?,
            val: input.parse()?,
        })
    }
}

/// Весь ввід: `k1 => v1, k2 => v2, ...` або порожньо
struct MapInput {
    items: Punctuated<Kv, Token![,]>,
}

impl Parse for MapInput {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let items = Punctuated::<Kv, Token![,]>::parse_terminated(input)?;
        Ok(Self { items })
    }
}

#[proc_macro]
pub fn btreemap(input: TokenStream) -> TokenStream {
    let MapInput { items } = parse_macro_input!(input as MapInput);

    if items.is_empty() {
        return quote! { ::std::collections::BTreeMap::new() }.into();
    }

    let inserts = items.iter().map(|Kv { key, val, .. }| {
        quote! { __map.insert(#key, #val); }
    });

    quote! {{
        let mut __map = ::std::collections::BTreeMap::new();
        #( #inserts )*
        __map
    }}
    .into()
}
