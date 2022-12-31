use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse_macro_input, Data, DeriveInput, GenericArgument, Ident, PathArguments, Type, Visibility,
};

struct AnalyzedField<'a> {
    vis: &'a Visibility,
    ident: &'a Option<Ident>,
    normalized_type: &'a Type,
    is_optional: bool,
}

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident;
    let visibility = input.vis;
    let builder_ident = Ident::new(&format!("{}Builder", ident), Span::call_site());
    let Data::Struct(data) = input.data else {
        return TokenStream::new();
    };

    let fields: Vec<_> = data
        .fields
        .iter()
        .map(|f| {
            if let Type::Path(tp) = &f.ty {
                if let Some(seg) = tp.path.segments.first() {
                    if seg.ident.to_string().as_str() == "Option" {
                        if let PathArguments::AngleBracketed(ab) = &seg.arguments {
                            if let Some(GenericArgument::Type(ty)) = ab.args.first() {
                                return AnalyzedField {
                                    vis: &f.vis,
                                    ident: &f.ident,
                                    normalized_type: ty,
                                    is_optional: true,
                                };
                            }
                        }
                    }
                }
            }
            AnalyzedField {
                vis: &f.vis,
                ident: &f.ident,
                normalized_type: &f.ty,
                is_optional: false,
            }
        })
        .collect();

    let field_defs = fields.iter().map(
        |AnalyzedField {
             vis,
             ident,
             normalized_type,
             ..
         }| {
            quote! {
                #vis #ident: Option<#normalized_type>
            }
        },
    );

    let field_setters = fields.iter().map(
        |AnalyzedField {
             vis,
             ident,
             normalized_type,
             ..
         }| {
            quote! {
                #vis fn #ident(&mut self, #ident: #normalized_type) -> &mut Self {
                    self.#ident = Some(#ident);
                    self
                }
            }
        },
    );

    let field_guards = fields.iter().map(|AnalyzedField { ident, is_optional,.. }| {
        if *is_optional {
            quote! {
                let #ident = self.#ident.clone();
            }
        } else {
            quote! {
                let Some(#ident) = self.#ident.clone() else { return Err("".to_string().into()); };
            }
        }
    });

    let field_idents: Vec<_> = fields.iter().map(|f| f.ident).collect();

    let expanded = quote! {
        #visibility struct #builder_ident {
            #(#field_defs ,)*
        }

        impl #builder_ident {
            #(#field_setters)*

            pub fn build(&mut self) -> Result<#ident, Box<dyn std::error::Error>> {
                #(#field_guards)*

                Ok(#ident {
                    #(#field_idents,)*
                })
            }
        }

        impl Command {
            pub fn builder() -> #builder_ident {
                #builder_ident {
                    #(#field_idents: None,)*
                }
            }
        }
    };

    expanded.into()
}
