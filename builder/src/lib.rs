use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Field, Ident};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident;
    let visibility = input.vis;
    let builder_ident = Ident::new(&format!("{}Builder", ident), Span::call_site());
    let Data::Struct(data) = input.data else {
        return TokenStream::new();
    };

    let field_defs = data.fields.iter().map(
        |Field {
             vis,
             ident,
             colon_token,
             ty,
             ..
         }| {
            quote! {
                #vis #ident #colon_token Option<#ty>
            }
        },
    );

    let field_setters = data.fields.iter().map(|Field { vis, ident, ty, .. }| {
        quote! {
            #vis fn #ident(&mut self, #ident: #ty) -> &mut Self {
                self.#ident = Some(#ident);
                self
            }
        }
    });

    let field_guards = data.fields.iter().map(|Field { ident, .. }| {
        quote! {
            let Some(#ident) = self.#ident.clone() else { return Err("".to_string().into()); };
        }
    });

    let fields = data.fields.iter().map(|Field { ident, .. }| {
        quote! {
            #ident
        }
    });
    let fields2 = fields.clone();

    let expanded = quote! {
        #visibility struct #builder_ident {
            #(#field_defs ,)*
        }

        impl #builder_ident {
            #(#field_setters)*

            pub fn build(&mut self) -> Result<#ident, Box<dyn std::error::Error>> {
                #(#field_guards)*

                Ok(#ident {
                    #(#fields,)*
                })
            }
        }

        impl Command {
            pub fn builder() -> #builder_ident {
                #builder_ident {
                    #(#fields2: None,)*
                }
            }
        }
    };

    expanded.into()
}
