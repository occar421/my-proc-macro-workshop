#![feature(int_roundings)]

use proc_macro::TokenStream;
use proc_macro2::Span;
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

            quote! {
                pub fn #getter_name(&self) -> u64 {
                    let mut array_be = [0; 8];
                    for i in 0..#length {
                        let data_i = i + #offset;
                        let array_i = i + 64 - #length;
                        if (self.data[data_i / 8] & (0x1 << ((8 - (data_i % 8)) % 8)) > 0) {
                            array_be[array_i / 8] |= 0x1 << ((8 - (array_i % 8)) % 8);
                        }
                    }
                    u64::from_be_bytes(array_be)
                }

                pub fn #setter_name(&mut self, value: u64) {
                    let array_be = u64::to_be_bytes(value);
                    for i in 0..#length {
                        let data_i = i + #offset;
                        let array_i = i + 64 - (#offset + #length);
                        self.data[data_i / 8] &= !(0x1 << (data_i % 8)); // reset
                        self.data[data_i / 8] |= array_be[array_i / 8] & (0x1 << (array_i % 8));
                    }
                }
            }
        });

    // dbg!(&item.fields);
    // dbg!(&field_lengths);
    let magic_trait_name = match n_bits % 8 {
        0 => "ZeroMod8",
        1 => "OneMod8",
        2 => "TwoMod8",
        3 => "ThreeMod8",
        4 => "FourMod8",
        5 => "FiveMod8",
        6 => "SixMod8",
        7 => "SevenMod8",
        _ => unreachable!(),
    };
    let _magic_trait_ident = syn::Ident::new(magic_trait_name, Span::call_site());

    let result = quote! {
       #[repr(C)]
        pub struct #name {
            data: [u8; #n_bytes],
        }

        #[inline]
        fn assert_type<C: ::bitfield::checks::TotalSizeIsMultipleOfEightBits + ?Sized>() {}

        impl #name {
            pub fn new() -> Self {
                // assert_type::<dyn ::bitfield::checks::#magic_trait_ident>();
                assert_type::<dyn ::bitfield::checks::CG<{(A::BITS + B::BITS + C::BITS + D::BITS) % 8}>>();
                // assert_type::<D>();
                // hoge![A, B];

                Self {
                    data: [0; #n_bytes],
                }
            }

            #(#accessors)*
        }
    };

    result.into()
}
