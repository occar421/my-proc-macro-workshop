use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use syn::{
    parse_macro_input, parse_quote, Attribute, Data, DeriveInput, Field, GenericArgument,
    GenericParam, Generics, Lit, Meta, PathArguments, Type,
};

struct CompIdent(Ident);

impl Clone for CompIdent {
    fn clone(&self) -> Self {
        CompIdent(self.0.clone())
    }
}

impl PartialEq<Self> for CompIdent {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_string().eq(&other.0.to_string())
    }
}

impl Eq for CompIdent {}

impl Hash for CompIdent {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_string().hash(state);
    }
}

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
        .filter_map(|p| CompIdent(wrap_match!(p => GenericParam::Type)?.ident.clone()).into())
        .collect();

    let mut used_type_params = HashSet::<CompIdent>::new();

    let field_supplies: Vec<_> = data
        .fields
        .iter()
        .map(
            |Field {
                 ident, attrs, ty, ..
             }| {
                let valid_types = get_valid_types(ty);
                if valid_types.len() == 0 {
                    return quote!();
                };

                used_type_params.extend(valid_types.into_iter().map(CompIdent));

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

fn add_trait_bounds(mut generics: Generics, used_generics_names: &HashSet<CompIdent>) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            if used_generics_names.contains(&CompIdent(type_param.ident.clone())) {
                type_param.bounds.push(parse_quote!(std::fmt::Debug));
            }
        }
    }
    generics
}

fn get_valid_types(ty: &Type) -> Vec<Ident> {
    return get_valid_types_inner(ty).unwrap_or(vec![]);

    fn get_valid_types_inner(ty: &Type) -> Option<Vec<Ident>> {
        let tp = match ty {
            Type::Path(tp) => tp,
            Type::Reference(tr) => wrap_match!(tr.elem.as_ref() => Type::Path)?,
            _ => return None,
        };
        let ps = tp.path.segments.first()?;
        let ident = ps.ident.clone();
        match ident.to_string().as_str() {
            "PhantomData" => None,
            _ => match &ps.arguments {
                PathArguments::AngleBracketed(ab) => {
                    let mut v = vec![ident];
                    v.extend(
                        ab.args
                            .iter()
                            .filter_map(|a| {
                                get_valid_types_inner(wrap_match!(a => GenericArgument::Type)?)
                            })
                            .flatten(),
                    );
                    v
                }
                .into(),
                PathArguments::Parenthesized(_) => unimplemented!(),
                PathArguments::None => vec![ident].into(),
            },
        }
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
