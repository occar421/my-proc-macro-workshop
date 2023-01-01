use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, Attribute, Data, DeriveInput, Field, GenericParam, Generics,
    Lit, Meta,
};

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let target_ident = input.ident;
    let Data::Struct(data) = input.data else {
        return syn::Error::new(Span::call_site(), "Unsupported".to_string()).into_compile_error().into();
    };

    let field_supplies = data.fields.iter().map(|Field { ident, attrs, .. }| {
        let debug_format = get_debug_format(attrs);

        match debug_format {
            Some(debug_format) => quote! {
                .field(stringify!(#ident), &format_args!(#debug_format, &self.#ident))
            },
            None => quote! {
                .field(stringify!(#ident), &self.#ident)
            },
        }
    });

    let generics = add_trait_bounds(input.generics);
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

fn add_trait_bounds(mut generics: Generics) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(parse_quote!(std::fmt::Debug));
        }
    }
    generics
}

fn get_debug_format(attrs: &Vec<Attribute>) -> Option<String> {
    let attr = attrs.first()?;

    let Meta::NameValue(nv) = attr.parse_meta().ok()? else {
        return None;
    };
    match nv.path.get_ident()?.to_string().as_str() {
        "debug" => {
            if let Lit::Str(str) = nv.lit {
                return Some(str.value());
            }
        }
        _ => {}
    }
    None
}
