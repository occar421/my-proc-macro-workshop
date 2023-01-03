use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, Item};

#[proc_macro_attribute]
pub fn sorted(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;
    let item = parse_macro_input!(input as syn::Item);

    let item = match item {
        Item::Enum(e) => e,
        _ => {
            return syn::Error::new(Span::call_site(), "expected enum or match expression")
                .into_compile_error()
                .into()
        }
    };

    (quote! { #item }).into()
}
