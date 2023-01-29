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

                let data_offset = quote! {
                    #prev
                };

                let vec_bits_offset = quote! {
                    (#tp::BYTES * 8 - #tp::BITS)
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
                        let mut val = vec![0; #type_bytes];
                        for i in 0..#length {
                            let data_i = #data_offset + i;
                            let val_i = #vec_bits_offset + i;

                            let data_vec_i = data_i / 8;
                            let data_bits_i = 7 - data_i % 8;
                            let val_vec_i = val_i / 8;
                            let val_bits_i = 7 - val_i % 8;

                            let has_bit = (self.data[data_vec_i] & 0x1 << data_bits_i) > 0;

                            if has_bit {
                                val[val_vec_i] |= 0x1 << val_bits_i;
                            }
                        }
                        #from(val)
                    }

                    pub fn #setter_name(&mut self, value: #value_type) {
                        let val = #to(value);
                        for i in 0..#length {
                            let data_i = #data_offset + i;
                            let val_i = #vec_bits_offset + i;

                            let data_vec_i = data_i / 8;
                            let data_bits_i = 7 - data_i % 8;
                            let val_vec_i = val_i / 8;
                            let val_bits_i = 7 - val_i % 8;

                            let has_bit = (val[val_vec_i] & (0x1 << val_bits_i)) > 0;
                            if has_bit {
                                self.data[data_vec_i] |= 0x1 << data_bits_i;
                            } else {
                                // reset
                                self.data[data_vec_i] &= !(0x1 << data_bits_i);
                            }
                        }
                    }
                };

                *prev = quote! {
                    (#data_offset + #length)
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

    let name_ident = input.ident;

    let variants = match input.data {
        syn::Data::Enum(de) => de.variants,
        _ => unimplemented!(),
    };

    let bits = variants.len().ilog2() as usize;
    let idents = variants.iter().map(|v| &v.ident);

    let extend = quote! {
        impl bitfield::Specifier for #name_ident {
            type Type = #name_ident;
            const BITS: usize = #bits;

            fn from_be_bytes_core(bytes: Vec<u8>) -> Self::Type {
                if bytes.len() != Self::BYTES {
                    unimplemented!();
                }
                let value = bytes[0];
                match value {
                    #(_ if #name_ident::#idents as u8 == value => #name_ident::#idents,)*
                    _ => unimplemented!(),
                }
            }

            fn to_be_bytes_core(value: Self::Type) -> Vec<u8> {
                vec![value as u8]
            }
        }
    };

    extend.into()
}
