use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use std::error::Error;
use syn::__private::TokenStream2;
use syn::{
    parse_macro_input, Attribute, Data, DataStruct, DeriveInput, Field, GenericArgument, Ident,
    Lit, Meta, NestedMeta, PathArguments, Type, Visibility,
};

struct AnalyzedField {
    vis: Visibility,
    ident: Ident,
    normalized_type: Type,
    kind: FieldKind,
    setter_ident: Ident,
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

    let fields = analyze_fields(&data);

    let builder_ident = format_ident!("{}Builder", input.ident);
    let target_addition_part = generate_target_addition_part(&input.ident, &builder_ident, &fields);
    let builder_part = generate_builder_part(&input.vis, &input.ident, &builder_ident, &fields);

    let expanded = quote! {
        #target_addition_part

        #builder_part
    };

    expanded.into()
}

fn analyze_fields(data: &DataStruct) -> Vec<Result<AnalyzedField, Box<dyn Error>>> {
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
                let ident = ident.clone().expect("anonymous field is unsupported");
                let setter_name = get_designated_setter_name(attrs)?.unwrap_or(ident.to_string());
                let (kind, normalized_type) =
                    check_special_type(ty).unwrap_or((FieldKind::Normal, ty));

                Ok(AnalyzedField {
                    vis: vis.clone(),
                    ident,
                    normalized_type: normalized_type.clone(),
                    kind,
                    setter_ident: Ident::new(&setter_name, Span::call_site()),
                })
            },
        )
        .collect()
}

fn get_designated_setter_name(attrs: &Vec<Attribute>) -> Result<Option<String>, Box<dyn Error>> {
    let Some(attr) = attrs.first() else {return Ok(None)};
    let Ok(meta) = attr.parse_meta() else { return Ok(None)};
    let Some(segment) = meta.path().segments.first() else {return Ok(None)};
    match segment.ident.to_string().as_str() {
        "builder" => {
            if let Meta::List(ml) = meta {
                let Some(item) = ml.nested.first() else { return Ok(None)};
                if let NestedMeta::Meta(Meta::NameValue(nv)) = item {
                    let Some(path_segment) = nv.path.segments.first() else {return Ok(None)};
                    if &path_segment.ident.to_string() == "each" {
                        if let Lit::Str(str) = &nv.lit {
                            return Ok(Some(str.value()));
                        }
                    } else {
                        return Err("expected `builder(each = \"...\")".into());
                    }
                }
            }
        }
        _ => {}
    }
    Ok(None)
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
    fields: &Vec<Result<AnalyzedField, Box<dyn Error>>>,
) -> TokenStream2 {
    let field_inits = fields.iter().filter_map(|f| {
        f.as_ref()
            .map(|AnalyzedField { ident, kind, .. }| match kind {
                FieldKind::Multiple => quote! {
                #ident: vec![]
                },
                _ => quote! {
                #ident: None
                },
            })
            .ok()
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
    fields: &Vec<Result<AnalyzedField, Box<dyn Error>>>,
) -> TokenStream2 {
    let field_defs = fields.iter().filter_map(|f| {
        f.as_ref()
            .map(
                |AnalyzedField {
                     vis,
                     ident: name,
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
            )
            .ok()
    });

    let field_setters = fields.iter().filter_map(|f| f.as_ref().map(
        |AnalyzedField {
             vis,
             ident,
             normalized_type,
             setter_ident,
             kind,
             ..
         }| {
            match kind {
                FieldKind::Multiple => {
                    let each = (ident.to_string() != setter_ident.to_string()).then_some(
                        quote!{
                            #vis fn #setter_ident(&mut self, #setter_ident: #normalized_type) -> &mut Self {
                                self.#ident.push(#setter_ident);
                                self
                            }
                        });
                    let direct = quote!{
                        #vis fn #ident(&mut self, #setter_ident: Vec<#normalized_type>) -> &mut Self {
                            self.#ident = #setter_ident;
                            self
                        }
                    };

                    quote! {
                        #each

                        #direct
                    }
                }
                _ => quote! {
                    #vis fn #ident(&mut self, #ident: #normalized_type) -> &mut Self {
                        self.#ident = Some(#setter_ident);
                        self
                    }
                },
            }
        }).ok()
    );

    let field_guards = fields.iter().filter_map(|f| f.as_ref().map(|AnalyzedField { ident, kind, .. }| {
        match kind {
            FieldKind::Normal => quote! {
                let Some(#ident) = self.#ident.clone() else { return Err(stringify!(#ident).to_string().into()); };
            },
            FieldKind::Optional | FieldKind::Multiple => quote! {
                let #ident = self.#ident.clone();
            },
        }
    }).ok());

    let field_idents = fields
        .iter()
        .filter_map(|f| f.as_ref().map(|f| &f.ident).ok());

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
