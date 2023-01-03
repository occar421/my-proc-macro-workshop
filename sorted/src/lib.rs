use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, Item, ItemEnum};

#[proc_macro_attribute]
pub fn sorted(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;
    let item = parse_macro_input!(input as syn::Item);

    let item = match validate(&item) {
        Ok(x) => x,
        Err(e) => {
            return e.into_compile_error().into();
        }
    };

    (quote! { #item }).into()
}

fn validate(item: &Item) -> syn::Result<&ItemEnum> {
    let item = match item {
        Item::Enum(e) => e,
        _ => {
            return Err(syn::Error::new(
                Span::call_site(),
                "expected enum or match expression",
            ))
        }
    };

    Ok(item)
}
