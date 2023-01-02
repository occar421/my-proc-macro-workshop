use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use syn::punctuated::Punctuated;
use syn::token::Colon2;
use syn::{
    parse_macro_input, parse_quote, Attribute, Data, DeriveInput, Field, GenericArgument,
    GenericParam, Generics, Lit, Meta, PathArguments, PathSegment, Type,
};

struct CompIdent(Ident);

impl Clone for CompIdent {
    fn clone(&self) -> Self {
        Self(self.0.clone())
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

struct IdentSegments(Vec<Ident>);

impl Debug for IdentSegments {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("IdentSegments").field(&self.0).finish()
    }
}

impl Clone for IdentSegments {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl PartialEq<Self> for IdentSegments {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl Eq for IdentSegments {}

impl Hash for IdentSegments {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for i in self.0.iter() {
            i.hash(state);
        }
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
        .filter_map(|p| wrap_match!(p => GenericParam::Type)?.ident.clone().into())
        .collect();

    let mut used_type_params = HashSet::<IdentSegments>::new();

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

                used_type_params.extend(valid_types.into_iter());

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
        .into_iter()
        .filter(|ident| used_type_params.contains(&IdentSegments(vec![ident.clone()])))
        .map(CompIdent)
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

fn get_valid_types(ty: &Type) -> Vec<IdentSegments> {
    return get_valid_types_inner(ty).unwrap_or(vec![]);

    fn get_valid_types_inner(ty: &Type) -> Option<Vec<IdentSegments>> {
        let tp = match ty {
            Type::Path(tp) => tp,
            Type::Reference(tr) => wrap_match!(tr.elem.as_ref() => Type::Path)?,
            _ => return None,
        };
        let ident_segments = get_ident_segments(&tp.path.segments);
        let ps = tp.path.segments.last()?; // naive
        match ps.ident.to_string().as_str() {
            "PhantomData" => None,
            _ => match &ps.arguments {
                PathArguments::AngleBracketed(ab) => {
                    let mut v = vec![IdentSegments(ident_segments)];
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
                PathArguments::None => vec![IdentSegments(ident_segments)].into(),
            },
        }
    }

    fn get_ident_segments(segments: &Punctuated<PathSegment, Colon2>) -> Vec<Ident> {
        segments.iter().map(|s| s.ident.clone()).collect()
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
