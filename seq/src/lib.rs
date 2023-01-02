use proc_macro::TokenStream;
use proc_macro2::{Group, Ident, Literal, TokenStream as TokenStream2, TokenTree};
use quote::{quote, TokenStreamExt};
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

    dbg!(&input.body);

    let generated_codes = (start..end).map(|i| replace(input.body.clone(), (input.var.clone(), i)));

    (quote! {
        #(#generated_codes)*
    })
    .into()
}

fn replace(ts: TokenStream2, var: (Ident, usize)) -> TokenStream2 {
    let mut iter = ts.into_iter().peekable();
    let mut ts = TokenStream2::new();

    while let Some(t) = iter.next() {
        match t {
            TokenTree::Group(g) => {
                let mut new_group = Group::new(g.delimiter(), replace(g.stream(), var.clone()));
                new_group.set_span(g.span());
                ts.append(TokenTree::Group(new_group));
            }
            TokenTree::Ident(ident) if ident.to_string() == var.0.to_string() => {
                ts.append(TokenTree::Literal(Literal::usize_unsuffixed(var.1)))
            }
            x => ts.append(x),
        }
    }

    ts
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
