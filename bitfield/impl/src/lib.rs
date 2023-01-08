#![feature(int_roundings)]

use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

#[proc_macro_attribute]
pub fn bitfield(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;
    let item = parse_macro_input!(input as syn::Item);
    let item = match item {
        syn::Item::Struct(is) => is,
        _ => {
            return syn::Error::new_spanned(item, "not supported")
                .into_compile_error()
                .into()
        }
    };

    let name = item.ident;
    let n_bits: usize = item
        .fields
        .iter()
        .map(|f| match &f.ty {
            syn::Type::Path(tp) => &tp.path,
            _ => unimplemented!(),
        })
        .map(|p| &p.segments.last().unwrap().ident)
        .filter_map(|i| {
            i.to_string()
                .strip_prefix("B")
                .map(|s| s.parse::<usize>().ok())
                .flatten()
        })
        .sum();
    let n_bytes = n_bits.div_ceil(8);

    let result = quote! {
       #[repr(C)]
        pub struct #name {
            data: [u8; #n_bytes],
        }
    };

    result.into()
}
