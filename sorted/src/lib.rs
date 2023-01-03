use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, Item};

#[proc_macro_attribute]
pub fn sorted(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;
    let item = parse_macro_input!(input as syn::Item);

    match validate(&item) {
        Ok(x) => x,
        Err(e) => {
            let compile_error = e.into_compile_error();
            return (quote! {
                #item

                #compile_error
            })
            .into();
        }
    };

    (quote! { #item }).into()
}

fn validate(item: &Item) -> syn::Result<()> {
    let item = match item {
        Item::Enum(e) => e,
        _ => {
            return Err(syn::Error::new(
                Span::call_site(),
                "expected enum or match expression",
            ))
        }
    };

    let mut variants: Vec<_> = item.variants.iter().collect();
    variants.sort_unstable_by_key(|x| &x.ident);

    for (actual, &right) in item.variants.iter().zip(&variants) {
        if actual.ident != right.ident {
            return Err(syn::Error::new_spanned(
                &right.ident,
                format!("{} should sort before {}", right.ident, actual.ident),
            ));
        }
    }

    Ok(())
}
