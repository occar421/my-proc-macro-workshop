#![feature(int_roundings)]

use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

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
    let field_lengths: Vec<_> = item
        .fields
        .iter()
        .map(|f| match &f.ty {
            syn::Type::Path(tp) => &tp.path,
            _ => unimplemented!(),
        })
        .map(|p| &p.segments.last().unwrap().ident)
        .filter_map(|i| {
            i.to_string()
                .strip_prefix("B")
                .map(|s| s.parse::<usize>().ok())
                .flatten()
        })
        .collect();

    let n_bits: usize = field_lengths.iter().sum();
    let n_bytes = n_bits.div_ceil(8);

    let field_names = item.fields.iter().filter_map(|f| f.ident.as_ref());
    let field_range = field_lengths.iter().scan(0, |offset, length| {
        let current_offset = offset.clone();
        *offset += length;
        Some((current_offset, length))
    });

    let accessors = field_names
        .zip(field_range)
        .into_iter()
        .map(|(i, (offset, length))| {
            let getter_name = syn::Ident::new(&format!("get_{}", i), i.span());
            let setter_name = syn::Ident::new(&format!("set_{}", i), i.span());
            let data_most = offset / 8;
            let data_least = (offset + length).div_ceil(8) - 1;
            let data_byte_length = data_least - data_most + 1;
            let array_most = 8 - data_byte_length;
            let array_least = 7usize;

            // ║ most_byte_padding | ~ | least_byte_padding ║
            let most_byte_padding = offset % 8;
            let most_byte_content_mask: u8 = !0 << most_byte_padding >> most_byte_padding;
            let least_byte_padding = 8 - (offset + length - data_least * 8);
            let least_byte_content_mask:u8 = !0 >> least_byte_padding << least_byte_padding;

            let array_to_data_offset = least_byte_padding;
            let array_to_data_offset_stored_mask: u8 = if array_to_data_offset == 0 {
                0
            }  else {
                !0 >> (8 - array_to_data_offset) << (8 - array_to_data_offset)
            };

            let getter_logic = if data_byte_length <= 1 {
                quote! {
                    let mut array_be = [0; 8];

                    array_be[#array_least] = (self.data[#data_least] & #most_byte_content_mask) >> #array_to_data_offset;

                    u64::from_be_bytes(array_be)
                }
            } else {
                if array_to_data_offset == 0 {
                    quote! {
                        let mut array_be = [0; 8];

                        for i in 0..#data_byte_length {
                            array_be[#array_least - i] =self.data[#data_least - i];
                        }
                        array_be[#array_most] &= #most_byte_content_mask;

                        u64::from_be_bytes(array_be)
                    }
                } else {
                    quote! {
                        let mut array_be = [0; 8];

                        for i in 0..#data_byte_length {
                            array_be[#array_least - i] =
                                ((self.data[#data_least - 1 - i] & (!0 >> #array_to_data_offset)) << #array_to_data_offset)
                                + (self.data[#data_least - i] >> #array_to_data_offset);
                        }
                        array_be[#array_most] &= (#most_byte_content_mask >> #array_to_data_offset);
                        dbg!(&array_be);

                        u64::from_be_bytes(array_be)
                    }
                }
            };

            let setter_logic = if data_byte_length <= 1 {
                quote! {
                    let array_be = u64::to_be_bytes(value);
                    let stored = self.data[#data_least] & !(#most_byte_content_mask & #least_byte_content_mask);
                    let value = (array_be[#array_least] & #most_byte_content_mask) << #array_to_data_offset;
                    self.data[#data_least] = stored | value;
                }
            } else {
                if array_to_data_offset == 0 {
                    quote! {
                        let array_be = u64::to_be_bytes(value);

                        for i in 0..#data_byte_length {
                            self.data[#data_least] = array_be[#array_least - i];
                        }

                        let stored = self.data[#data_most] & !#most_byte_content_mask;
                        self.data[#data_most] = (array_be[#array_most] & #most_byte_content_mask) | stored;
                    }
                } else {
                    quote! {
                        let array_be = u64::to_be_bytes(value);

                        for i in 0..#data_byte_length {
                            let stored = self.data[#data_least - i] & #array_to_data_offset_stored_mask;
                            self.data[#data_least] = (array_be[#array_least - i] << #array_to_data_offset) | stored;
                        }

                        let stored = self.data[#data_most] & (!#most_byte_content_mask | #array_to_data_offset_stored_mask);
                        self.data[#data_most] = ((array_be[#array_most] & #most_byte_content_mask) << #array_to_data_offset) | stored;
                    }
                }
            };

            quote! {
                pub fn #getter_name(&self) -> u64 {
                    #getter_logic
                }

                pub fn #setter_name(&mut self, value: u64) {
                    #setter_logic
                }
            }
        });

    let result = quote! {
       #[repr(C)]
        pub struct #name {
            data: [u8; #n_bytes],
        }

        impl #name {
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
