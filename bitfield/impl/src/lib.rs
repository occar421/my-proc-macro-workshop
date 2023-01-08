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

                let accessor = quote! {
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
