use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{Data, DeriveInput, Field, Ident, parse_macro_input};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let visibility = input.vis;
    let builder_ident = Ident::new(&format!("{}Builder", input.ident), Span::call_site());
    let Data::Struct(data) = input.data else {
        return TokenStream::new();
    };

    let field_defs = data.fields.iter().map(|Field { vis, ident, colon_token, ty, .. }| quote! {
        #vis #ident #colon_token Option<#ty>
    });

    let field_setters = data.fields.iter().map(|Field { vis, ident, ty, .. }| quote! {
        #vis fn #ident(&mut self, #ident: #ty) -> &mut Self {
            self.#ident = Some(#ident);
            self
        }
    });

    let field_inits = data.fields.iter().map(|Field { ident, .. }| quote! {
        #ident: None
    });

    let expanded = quote! {
        #visibility struct #builder_ident {
            #(#field_defs ,)*
        }

        impl #builder_ident {
            #(#field_setters)*
        }

        impl Command {
            pub fn builder() -> #builder_ident {
                #builder_ident {
                    #(#field_inits ,)*
                }
            }
        }
    };

    expanded.into()
}
