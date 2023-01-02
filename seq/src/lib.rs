use proc_macro::TokenStream;
use proc_macro2::{Group, Ident, Literal, TokenStream as TokenStream2, TokenTree};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::token::Brace;
use syn::{braced, parse_macro_input, LitInt, Token};

#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as SeqInput);

    let start = match input.start.base10_parse::<usize>() {
        Ok(x) => x,
        Err(x) => return x.into_compile_error().into(),
    };

    let end = match input.end.base10_parse::<usize>() {
        Ok(x) => x,
        Err(x) => return x.into_compile_error().into(),
    };

    let generated_codes = (start..end).map(|i| replace(input.body.clone(), (input.var.clone(), i)));

    (quote! {
        #(#generated_codes)*
    })
    .into()
}

fn replace(ts: TokenStream2, var: (Ident, usize)) -> TokenStream2 {
    ts.into_iter()
        .map(|t| match t {
            TokenTree::Group(g) => {
                TokenTree::Group(Group::new(g.delimiter(), replace(g.stream(), var.clone())))
            }
            TokenTree::Ident(ident) if ident.to_string() == var.0.to_string() => {
                TokenTree::Literal(Literal::usize_unsuffixed(var.1))
            }
            x => x,
        })
        .collect()
}

#[derive(Debug)]
struct SeqInput {
    var: Ident,
    in_token: Token![in],
    start: LitInt,
    range_token: Token![..],
    end: LitInt,
    brace_token: Brace,
    body: TokenStream2,
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
            body: content.parse()?,
        })
    }
}
