use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse_macro_input, Data, DeriveInput, Field, GenericArgument, Ident, Lit, Meta, NestedMeta,
    PathArguments, Type, Visibility,
};

struct AnalyzedField {
    vis: Visibility,
    ident: Option<Ident>,
    normalized_type: Type,
    kind: FieldKind,
    setter_ident: Option<Ident>,
}

enum FieldKind {
    Normal,
    Optional,
    Multiple,
}

#[proc_macro_derive(Builder, attributes(builder))]
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
        .map(
            |Field {
                 attrs,
                 vis,
                 ident,
                 ty,
                 ..
             }| {
                let designated_setter_ident = attrs
                    .first()
                    .map(|a| {
                        let meta = a.parse_meta().ok()?;
                        match meta.path().segments.first()?.ident.to_string().as_str() {
                            "builder" => {
                                if let Meta::List(ml) = meta {
                                    if let NestedMeta::Meta(Meta::NameValue(nv)) =
                                        ml.nested.first()?
                                    {
                                        if nv.path.segments.first()?.ident.to_string().as_str()
                                            == "each"
                                        {
                                            if let Lit::Str(str) = &nv.lit {
                                                return Some(Ident::new(
                                                    &str.value(),
                                                    Span::call_site(),
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                        None
                    })
                    .flatten();

                let setter_ident = designated_setter_ident.map_or(ident.clone(), |i| Some(i));

                if let Type::Path(tp) = ty {
                    if let Some(seg) = tp.path.segments.first() {
                        let kind = match seg.ident.to_string().as_str() {
                            "Option" => Some(FieldKind::Optional),
                            "Vec" => Some(FieldKind::Multiple),
                            _ => None,
                        };
                        if let Some(kind) = kind {
                            if let PathArguments::AngleBracketed(ab) = &seg.arguments {
                                if let Some(GenericArgument::Type(ty)) = ab.args.first() {
                                    return AnalyzedField {
                                        vis: vis.clone(),
                                        ident: ident.clone(),
                                        normalized_type: ty.clone(),
                                        kind,
                                        setter_ident,
                                    };
                                }
                            }
                        }
                    }
                }
                AnalyzedField {
                    vis: vis.clone(),
                    ident: ident.clone(),
                    normalized_type: ty.clone(),
                    kind: FieldKind::Normal,
                    setter_ident,
                }
            },
        )
        .collect();

    let field_defs = fields.iter().map(
        |AnalyzedField {
             vis,
             ident,
             normalized_type,
             kind,
             ..
         }| {
            match kind {
                FieldKind::Multiple => quote! {
                    #vis #ident: Vec<#normalized_type>
                },
                _ => quote! {
                    #vis #ident: Option<#normalized_type>
                },
            }
        },
    );

    let field_setters = fields.iter().map(
        |AnalyzedField {
             vis,
             ident,
             normalized_type,
             setter_ident,
             kind,
             ..
         }| {
            match kind {
                FieldKind::Multiple => quote! {
                    #vis fn #setter_ident(&mut self, #setter_ident: #normalized_type) -> &mut Self {
                        self.#ident.push(#setter_ident);
                        self
                    }
                },
                _ => quote! {
                    #vis fn #setter_ident(&mut self, #setter_ident: #normalized_type) -> &mut Self {
                        self.#ident = Some(#setter_ident);
                        self
                    }
                },
            }
        },
    );

    let field_guards = fields
        .iter()
        .map(|AnalyzedField { ident, kind, .. }| match kind {
            FieldKind::Normal => quote! {
                let Some(#ident) = self.#ident.clone() else { return Err("".to_string().into()); };
            },
            FieldKind::Optional | FieldKind::Multiple => quote! {
                let #ident = self.#ident.clone();
            },
        });

    let field_idents = fields.iter().map(|f| &f.ident);
    let field_inits = fields
        .iter()
        .map(|AnalyzedField { ident, kind, .. }| match kind {
            FieldKind::Multiple => quote! {
                #ident: vec![]
            },
            _ => quote! {
                #ident: None
            },
        });

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
                    #(#field_inits,)*
                }
            }
        }
    };

    expanded.into()
}
