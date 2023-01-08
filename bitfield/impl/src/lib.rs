use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_attribute]
pub fn bitfield(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;
    let item = parse_macro_input!(input as syn::Item);
    let item = match item {
        syn::Item::Struct(is) => is,
        _ => {
            return syn::Error::new_spanned(item, "not supported")
                .into_compile_error()
                .into()
        }
    };

    let name = item.ident;

    let field_names = item.fields.iter().filter_map(|f| f.ident.as_ref());
    let type_paths = item.fields.iter().map(|f| match &f.ty {
        syn::Type::Path(tp) => tp,
        _ => unimplemented!(),
    });
    let accessors =
        field_names
            .zip(type_paths.clone())
            .into_iter()
            .scan(quote! { 0 }, |prev, (i, tp)| {
                let getter_name = syn::Ident::new(&format!("get_{}", i), i.span());
                let setter_name = syn::Ident::new(&format!("set_{}", i), i.span());

                let length = quote! {
                    #tp::BITS
                };

                let offset = quote! {
                    #prev
                };

                let value_type = quote! {
                    <#tp as bitfield::Specifier>::Type
                };

                let from = quote! {
                    <#tp as bitfield::Specifier>::from_be_bytes
                };

                let to = quote! {
                    <#tp as bitfield::Specifier>::to_be_bytes
                };

                let type_bytes = quote! {
                    #tp::BYTES
                };

                let accessor = quote! {
                    pub fn #getter_name(&self) -> #value_type {
                        let mut v = vec![0; #type_bytes];
                        for i in 0..#length {
                            let data_i = i + #offset;
                            let v_i = i + (#type_bytes * 8) - #length;
                            if self.data[data_i / 8] & (0x1 << ((8 - (data_i % 8)) % 8)) > 0 {
                                v[v_i / 8] |= 0x1 << ((8 - (v_i % 8)) % 8);
                            }
                        }
                        #from(v)
                    }

                    pub fn #setter_name(&mut self, value: #value_type) {
                        let v = #to(value);
                        for i in 0..#length {
                            let data_i = i + #offset;
                            let v_i = i + (#type_bytes * 8) - (#offset + #length);
                            self.data[data_i / 8] &= !(0x1 << (data_i % 8)); // reset
                            self.data[data_i / 8] |= v[v_i / 8] & (0x1 << (v_i % 8));
                        }
                    }
                };

                *prev = quote! {
                    (#offset + #length)
                };

                Some(accessor)
            });

    let n_bits = quote! {
        (#(#type_paths::BITS)+*)
    };

    let n_bytes = quote! {
        ((#n_bits) + 8 - 1) / 8
    }; // div_ceil

    let result = quote! {
       #[repr(C)]
        pub struct #name {
            data: [u8; #n_bytes],
        }

        impl #name where
            <bitfield::CG<{#n_bits % 8}> as bitfield::checks::DeductMod>::Mod
                : bitfield::checks::TotalSizeIsMultipleOfEightBits {
            pub fn new() -> Self {
                Self {
                    data: [0; #n_bytes],
                }
            }

            #(#accessors)*
        }
    };

    result.into()
}

#[proc_macro_derive(BitfieldSpecifier)]
pub fn bitfield_specifier(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident;

    let bits = match input.data {
        syn::Data::Enum(de) => (de.variants.len() as f64).log2() as usize,
        _ => unimplemented!(),
    };

    let bytes = (bits + 8 - 1) / 8; // ceil_div

    let extend = quote! {
        impl bitfield::Specifier for #ident {
            type Type = #ident;
            const BITS: usize = #bits;
            const BYTES: usize = #bytes;

            fn from_be_bytes_core(bytes: Vec<u8>) -> Self::Type {
                unimplemented!()
            }

            fn to_be_bytes_core(value: Self::Type) -> Vec<u8> {
                unimplemented!()
            }
        }
    };

    extend.into()
}
