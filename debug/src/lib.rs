use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use std::collections::HashSet;
use syn::{
    parse_macro_input, parse_quote, Attribute, Data, DeriveInput, Field, GenericArgument,
    GenericParam, Generics, Lit, Meta, PathArguments, Type,
};

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let target_ident = input.ident;
    let Data::Struct(data) = input.data else {
        return syn::Error::new(Span::call_site(), "Unsupported".to_string()).into_compile_error().into();
    };

    let target_generics_params: HashSet<_> = input
        .generics
        .params
        .iter()
        .filter_map(|p| {
            wrap_match!(p => GenericParam::Type)?
                .ident
                .to_string()
                .into()
        })
        .collect();

    let mut used_type_params = HashSet::<String>::new();

    let field_supplies: Vec<_> = data
        .fields
        .iter()
        .map(
            |Field {
                 ident, attrs, ty, ..
             }| {
                let Some(valid_names) = get_valid_types(ty) else {
                    return quote!();
                };

                used_type_params.extend(valid_names.into_iter());

                let debug_format = get_debug_format(attrs);

                match debug_format {
                    Some(debug_format) => quote! {
                        .field(stringify!(#ident), &format_args!(#debug_format, &self.#ident))
                    },
                    None => quote! {
                        .field(stringify!(#ident), &self.#ident)
                    },
                }
            },
        )
        .collect();

    let used_generics_names: HashSet<_> = target_generics_params
        .intersection(&used_type_params)
        .into_iter()
        .cloned()
        .collect();

    let generics = add_trait_bounds(input.generics, &used_generics_names);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let extend = quote! {
        impl #impl_generics std::fmt::Debug for #target_ident #ty_generics #where_clause {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
                f.debug_struct(stringify!(#target_ident))
                    #(#field_supplies)*
                    .finish()
            }
        }
    };

    extend.into()
}

fn add_trait_bounds(mut generics: Generics, used_generics_names: &HashSet<String>) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            if used_generics_names.contains(&type_param.ident.to_string()) {
                type_param.bounds.push(parse_quote!(std::fmt::Debug));
            }
        }
    }
    generics
}

fn get_valid_types(ty: &Type) -> Option<Vec<String>> {
    let tp = match ty {
        Type::Path(tp) => tp,
        Type::Reference(tr) => wrap_match!(tr.elem.as_ref() => Type::Path)?,
        _ => return None,
    };
    let ps = tp.path.segments.first()?;
    let node_name = ps.ident.to_string();
    match node_name.as_str() {
        "PhantomData" => None,
        _ => match &ps.arguments {
            PathArguments::AngleBracketed(ab) => {
                let mut v = vec![node_name];
                v.extend(
                    ab.args
                        .iter()
                        .filter_map(|a| get_valid_types(wrap_match!(a => GenericArgument::Type)?))
                        .flatten(),
                );
                v
            }
            .into(),
            PathArguments::Parenthesized(_) => unimplemented!(),
            PathArguments::None => vec![node_name].into(),
        },
    }
}

fn get_debug_format(attrs: &Vec<Attribute>) -> Option<String> {
    let attr = attrs.first()?;
    let meta = attr.parse_meta().ok()?;
    let nv = wrap_match!(meta => Meta::NameValue)?;
    match nv.path.get_ident()?.to_string().as_str() {
        "debug" => wrap_match!(nv.lit => Lit::Str)?.value().into(),
        _ => None,
    }
}

macro_rules! wrap_match {
    ($e:expr => $p:path) => {
        if let $p(v) = $e {
            Some(v)
        } else {
            None
        }
    };
}

use wrap_match;
