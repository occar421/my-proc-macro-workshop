use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, quote_spanned};
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

                let length = quote! { #tp::BITS };

                let data_offset = quote! { #prev };

                let vec_bits_offset = quote! { (#tp::BYTES * 8 - #tp::BITS) };

                let value_type = quote! { <#tp as bitfield::Specifier>::Type };

                let from = quote! { <#tp as bitfield::Specifier>::from_be_bytes };
                let to = quote! { <#tp as bitfield::Specifier>::to_be_bytes };

                let type_bytes = quote! { #tp::BYTES };

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

                *prev = quote! { (#data_offset + #length) };

                Some(accessor)
            });

    let bits_attributes = item.fields.iter().filter_map(|f| {
        let ident = f.ident.as_ref()?;
        let ty = &f.ty;
        let lit = f.attrs.iter().find_map(|a| {
            let syn::Meta::NameValue(nv) = a.parse_meta().ok()? else {return None};
            let attr_ident = nv.path.get_ident()?;
            (attr_ident == "bits").then_some(nv.lit)
        })?;
        let syn::Lit::Int(lit) = lit else { return None; };

        Some((ident, ty, lit))
    });

    let bits_size_checks = bits_attributes.map(|(ident, ty, lit)| {
        let check_const_name = format!("_BITS_SIZE_CHECK_FOR_{}", ident.to_string().to_uppercase());
        let name = syn::Ident::new(&check_const_name, Span::call_site());

        let v: usize = lit.base10_parse().unwrap();
        let range = 0..v;

        quote_spanned! {lit.span() =>
            #[doc(hidden)]
            const #name: [usize; <#ty as bitfield::Specifier>::BITS] = [#(#range,)*];
        }
    });

    let n_bits = quote! { (#(#type_paths::BITS)+*) };
    let n_bytes = quote! { ((#n_bits) + 8 - 1) / 8 }; // div_ceil

    let result = quote! {
       #[repr(C)]
        pub struct #name {
            data: [u8; #n_bytes],
        }

        impl #name where
            <bitfield::CGUsize<{#n_bits % 8}> as bitfield::checks::DeductMod>::Mod
                : bitfield::checks::TotalSizeIsMultipleOfEightBits {
            #(#bits_size_checks)*

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

    let bits = variants.len().ilog2();
    let variant_idents: Vec<_> = variants.iter().map(|v| &v.ident).collect();

    if 2usize.pow(bits) != variant_idents.len() {
        // Non power of two
        return syn::Error::new(
            Span::call_site(),
            "BitfieldSpecifier expected a number of variants which is a power of 2",
        )
        .into_compile_error()
        .into();
    }

    let bits = bits as usize;

    let expected_variant_max = variant_idents.len() - 1;

    let out_of_range_checks = variant_idents.iter().map(|ident| quote_spanned!{ident.span() =>
        <bitfield::CGBool<{#name_ident::#ident as usize <= #expected_variant_max}> as bitfield::checks::DeductIsPowOf2>::IsPowOf2: bitfield::checks::DiscriminantInRange
    });

    let extend = quote! {
        impl bitfield::Specifier for #name_ident where
            #(#out_of_range_checks,)* {
            type Type = #name_ident;
            const BITS: usize = #bits;

            fn from_be_bytes_core(bytes: Vec<u8>) -> Self::Type {
                if bytes.len() != Self::BYTES {
                    unimplemented!();
                }
                let value = bytes[0];
                match value {
                    #(_ if #name_ident::#variant_idents as u8 == value => #name_ident::#variant_idents,)*
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
