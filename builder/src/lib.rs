use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::__private::TokenStream2;
use syn::{
    parse_macro_input, Attribute, Data, DataStruct, DeriveInput, Field, GenericArgument, Ident,
    Lit, Meta, NestedMeta, PathArguments, Type, Visibility,
};

struct AnalyzedField {
    vis: Visibility,
    name: String,
    normalized_type: Type,
    kind: FieldKind,
    setter_name: String,
}

enum FieldKind {
    Normal,
    Optional,
    Multiple,
}

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let Data::Struct(data) = input.data else {
        return TokenStream::new();
    };

    let fields: Vec<_> = analyze_fields(&data);

    let builder_ident = format_ident!("{}Builder", input.ident);
    let target_addition_part = generate_target_addition_part(&input.ident, &builder_ident, &fields);
    let builder_part = generate_builder_part(&input.vis, &input.ident, &builder_ident, &fields);

    let expanded = quote! {
        #target_addition_part

        #builder_part
    };

    expanded.into()
}

fn analyze_fields(data: &DataStruct) -> Vec<AnalyzedField> {
    data.fields
        .iter()
        .map(
            |Field {
                 attrs,
                 vis,
                 ident,
                 ty,
                 ..
             }| {
                let name = ident
                    .clone()
                    .expect("anonymous field is unsupported")
                    .to_string();
                let setter_name = get_designated_setter_name(attrs).unwrap_or(name.clone());
                let (kind, normalized_type) =
                    check_special_type(ty).unwrap_or((FieldKind::Normal, ty));

                AnalyzedField {
                    vis: vis.clone(),
                    name,
                    normalized_type: normalized_type.clone(),
                    kind,
                    setter_name,
                }
            },
        )
        .collect()
}

fn get_designated_setter_name(attrs: &Vec<Attribute>) -> Option<String> {
    let meta = attrs.first()?.parse_meta().ok()?;
    match meta.path().segments.first()?.ident.to_string().as_str() {
        "builder" => {
            if let Meta::List(ml) = meta {
                if let NestedMeta::Meta(Meta::NameValue(nv)) = ml.nested.first()? {
                    if &nv.path.segments.first()?.ident.to_string() == "each" {
                        if let Lit::Str(str) = &nv.lit {
                            return Some(str.value());
                        }
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn check_special_type(ty: &Type) -> Option<(FieldKind, &Type)> {
    if let Type::Path(tp) = ty {
        let seg = tp.path.segments.first()?;
        let kind = match seg.ident.to_string().as_str() {
            "Option" => FieldKind::Optional,
            "Vec" => FieldKind::Multiple,
            _ => return None,
        };
        if let PathArguments::AngleBracketed(ab) = &seg.arguments {
            if let GenericArgument::Type(ty) = ab.args.first()? {
                return Some((kind, ty));
            }
        }
    }
    None
}

fn generate_target_addition_part(
    target_ident: &Ident,
    builder_ident: &Ident,
    fields: &Vec<AnalyzedField>,
) -> TokenStream2 {
    let field_inits = fields
        .iter()
        .map(|AnalyzedField { name, kind, .. }| match kind {
            FieldKind::Multiple => quote! {
                #name: vec![]
            },
            _ => quote! {
                #name: None
            },
        });

    quote! {
        impl #target_ident {
            pub fn builder() -> #builder_ident {
                #builder_ident {
                    #(#field_inits,)*
                }
            }
        }
    }
}

fn generate_builder_part(
    target_vis: &Visibility,
    target_ident: &Ident,
    builder_ident: &Ident,
    fields: &Vec<AnalyzedField>,
) -> TokenStream2 {
    let field_defs = fields.iter().map(
        |AnalyzedField {
             vis,
             name,
             normalized_type,
             kind,
             ..
         }| {
            match kind {
                FieldKind::Multiple => quote! {
                    #vis #name: Vec<#normalized_type>
                },
                _ => quote! {
                    #vis #name: Option<#normalized_type>
                },
            }
        },
    );

    let field_setters = fields.iter().map(
        |AnalyzedField {
             vis,
             name,
             normalized_type,
             setter_name,
             kind,
             ..
         }| {
            match kind {
                FieldKind::Multiple => {
                    let each = (name != setter_name).then_some(
                        quote!{
                            #vis fn #setter_name(&mut self, #setter_name: #normalized_type) -> &mut Self {
                                self.#name.push(#setter_name);
                                self
                            }
                        });
                    let direct = quote!{
                        #vis fn #name(&mut self, #setter_name: Vec<#normalized_type>) -> &mut Self {
                            self.#name = #setter_name;
                            self
                        }
                    };

                    quote! {
                        #each

                        #direct
                    }
                }
                _ => quote! {
                    #vis fn #name(&mut self, #name: #normalized_type) -> &mut Self {
                        self.#name = Some(#setter_name);
                        self
                    }
                },
            }
        },
    );

    let field_guards = fields
        .iter()
        .map(|AnalyzedField { name, kind, .. }| match kind {
            FieldKind::Normal => quote! {
                let Some(#name) = self.#name.clone() else { return Err(stringify!(#name).to_string().into()); };
            },
            FieldKind::Optional | FieldKind::Multiple => quote! {
                let #name = self.#name.clone();
            },
        });

    let field_idents = fields.iter().map(|f| &f.name);

    quote! {
        #target_vis struct #builder_ident {
            #(#field_defs ,)*
        }

        impl #builder_ident {
            #(#field_setters)*

            pub fn build(&mut self) -> Result<#target_ident, Box<dyn std::error::Error>> {
                #(#field_guards)*

                Ok(#target_ident {
                    #(#field_idents,)*
                })
            }
        }
    }
}
