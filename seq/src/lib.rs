use proc_macro::TokenStream;
use syn::parse::{Parse, ParseStream};
use syn::{braced, parse_macro_input, Token};

#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {
    let _ = parse_macro_input!(input as SeqInput);

    TokenStream::new()
}

struct SeqInput {
    var: syn::Ident,
    in_token: Token![in],
    start: syn::LitInt,
    range_token: Token![..],
    end: syn::LitInt,
    brace_token: syn::token::Brace,
}

impl Parse for SeqInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(SeqInput {
            var: input.parse()?,
            in_token: input.parse()?,
            start: input.parse()?,
            range_token: input.parse()?,
            end: input.parse()?,
            brace_token: braced!(content in input),
        })
    }
}
