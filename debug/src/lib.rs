use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use syn::{
    parse_macro_input, parse_quote, Attribute, Data, DeriveInput, Field, Fields, GenericArgument,
    GenericParam, Generics, Lit, Meta, NestedMeta, Path, PathArguments, Type, WherePredicate,
};

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let target_ident = input.ident;
    let Data::Struct(data) = input.data else {
        return syn::Error::new(Span::call_site(), "Unsupported".to_string()).into_compile_error().into();
    };

    let (field_debug_data, valid_types): (Vec<_>, Vec<_>) =
        analyze_fields(&data.fields).into_iter().unzip();

    let target_custom_where_predicates = match get_custom_where_predicates(&input.attrs) {
        Ok(x) => x,
        Err(e) => return e.into_compile_error().into(),
    };

    let generics = if target_custom_where_predicates.is_empty() {
        let target_generics_idents = get_generic_param_idents(&input.generics);

        let mut used_type_params = HashSet::<CompPath>::new();
        for valid_types in valid_types {
            used_type_params.extend(valid_types.into_iter());
        }

        add_trait_bounds(input.generics, &target_generics_idents, &used_type_params)
    } else {
        add_custom_bounds(input.generics, &target_custom_where_predicates)
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let field_supplies = field_debug_data.iter().filter_map(|f| {
        f.clone().map(|(ident, debug_format)| match debug_format {
            Some(debug_format) => quote! {
                .field(stringify!(#ident), &format_args!(#debug_format, &self.#ident))
            },
            None => quote! {
                .field(stringify!(#ident), &self.#ident)
            },
        })
    });

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

fn get_custom_where_predicates(attrs: &Vec<Attribute>) -> syn::Result<Vec<WherePredicate>> {
    attrs
        .iter()
        .map(|a| -> syn::Result<_> {
            match a.parse_meta()? {
                Meta::List(l) => match l.path.get_ident() {
                    Some(ident) if ident == "debug" => {
                        let nm = l.nested.first().ok_or(syn::Error::new_spanned(&l, "?"))?;
                        let meta = wrap_match!(nm => NestedMeta::Meta)
                            .ok_or(syn::Error::new_spanned(nm, "??"))?;
                        let nv = wrap_match!(meta => Meta::NameValue)
                            .ok_or(syn::Error::new_spanned(meta, "???"))?;
                        let ident = nv
                            .path
                            .get_ident()
                            .ok_or(syn::Error::new_spanned(&nv.path, "????"))?;
                        if ident.to_string() != "bound" {
                            return Err(syn::Error::new_spanned(nv, "should be `bound = (...)`"));
                        }
                        if let Lit::Str(str) = &nv.lit {
                            let str = str.token().to_string();
                            let str = str.trim_matches(|c| !char::is_alphanumeric(c));
                            let w = syn::parse_str::<WherePredicate>(&str)?;
                            Ok(w)
                        } else {
                            Err(syn::Error::new_spanned(&nv.lit, "invalid format"))
                        }
                    }
                    _ => unimplemented!(),
                },
                _ => unimplemented!(),
            }
        })
        .collect::<Result<_, _>>()
}

fn analyze_fields(fields: &Fields) -> Vec<(Option<(Ident, Option<String>)>, Vec<CompPath>)> {
    fields
        .iter()
        .map(
            |Field {
                 ident, attrs, ty, ..
             }| {
                let valid_types = get_valid_types(ty);
                if valid_types.is_empty() {
                    (None, valid_types)
                } else {
                    let debug_format = get_debug_format(attrs);

                    (Some((ident.clone().unwrap(), debug_format)), valid_types)
                }
            },
        )
        .collect()
}

fn get_generic_param_idents(generics: &Generics) -> HashSet<CompIdent> {
    generics
        .params
        .iter()
        .filter_map(|p| wrap_match!(p => GenericParam::Type))
        .map(|p| CompIdent(p.ident.clone()))
        .collect()
}

struct CompPath<'a>(&'a Path);

impl<'a> PartialEq<Self> for CompPath<'a> {
    fn eq(&self, other: &Self) -> bool {
        let s: Vec<_> = self.0.segments.iter().rev().collect();
        let o: Vec<_> = other.0.segments.iter().rev().collect();

        for (s, o) in s.iter().zip(o) {
            if s.ident.to_string() != o.ident.to_string() {
                return false;
            }
        }

        true
    }
}

impl<'a> Eq for CompPath<'a> {}

impl<'a> Hash for CompPath<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for seg in &self.0.segments {
            seg.ident.to_string().hash(state);
        }
    }
}

struct CompIdent(Ident);

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

fn add_trait_bounds(
    mut generics: Generics,
    target_generics_idents: &HashSet<CompIdent>,
    used_type_params: &HashSet<CompPath>,
) -> Generics {
    let used_generics_names: HashSet<_> = target_generics_idents
        .into_iter()
        .filter(|CompIdent(ident)| {
            for CompPath(path) in used_type_params {
                if path.segments.len() == 1 {
                    if path.segments.first().unwrap().ident.to_string() == ident.to_string() {
                        return true;
                    }
                }
            }

            false
        })
        .collect();

    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            if used_generics_names.contains(&CompIdent(type_param.ident.clone())) {
                type_param.bounds.push(parse_quote!(std::fmt::Debug));
            }
        }
    }

    let assoc_target_names: Vec<_> = used_type_params
        .iter()
        .filter(|CompPath(path)| match path.segments.first() {
            Some(s) if target_generics_idents.contains(&CompIdent(s.ident.clone())) => true,
            _ => false,
        })
        .collect();

    let punctuated = &mut generics.make_where_clause().predicates;
    for CompPath(path) in assoc_target_names {
        punctuated.push(parse_quote!(#path: std::fmt::Debug));
    }

    generics
}

fn add_custom_bounds(
    mut generics: Generics,
    target_custom_where_predicates: &Vec<WherePredicate>,
) -> Generics {
    let punctuated = &mut generics.make_where_clause().predicates;

    for predicate in target_custom_where_predicates {
        punctuated.push(predicate.clone());
    }

    generics
}

fn get_valid_types(ty: &Type) -> Vec<CompPath> {
    return get_valid_types_inner(ty).unwrap_or(vec![]);

    fn get_valid_types_inner(ty: &Type) -> Option<Vec<CompPath>> {
        let tp = match ty {
            Type::Path(tp) => tp,
            Type::Reference(tr) => wrap_match!(tr.elem.as_ref() => Type::Path)?,
            _ => return None,
        };
        let path = &tp.path;
        let ps = path.segments.last()?; // naive
        match ps.ident.to_string().as_str() {
            "PhantomData" => None,
            _ => match &ps.arguments {
                PathArguments::AngleBracketed(ab) => {
                    let mut v = vec![CompPath(path)];
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
                PathArguments::None => vec![CompPath(path)].into(),
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
