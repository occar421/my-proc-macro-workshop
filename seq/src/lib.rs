use proc_macro::TokenStream;
use proc_macro2::Delimiter::Parenthesis;
use proc_macro2::{Group, Ident, Literal, TokenStream as TokenStream2, TokenTree};
use quote::TokenStreamExt;
use std::ops::Range;
use syn::parse::{Parse, ParseStream};
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

    let range = start..end;

    match replace(input.body.clone(), (input.var.clone(), None), range.clone()) {
        Ok(ts) => ts.into(),
        Err(_) => {
            let generated_codes = range.clone().map(|i| {
                replace(
                    input.body.clone(),
                    (input.var.clone(), Some(i)),
                    range.clone(),
                )
                .unwrap()
            });

            let mut acc = TokenStream2::new();
            for generated_code in generated_codes {
                acc.extend(generated_code);
            }
            acc.into()
        }
    }
}

#[derive(Debug)]
struct UnmetSpecificArea;

fn replace(
    ts: TokenStream2,
    var: (Ident, Option<usize>),
    range: Range<usize>,
) -> Result<TokenStream2, UnmetSpecificArea> {
    let mut iter = ts.into_iter().peekable();
    let mut ts = TokenStream2::new();

    while let Some(t0) = iter.next() {
        match &t0 {
            TokenTree::Group(g) => {
                let mut new_group = Group::new(
                    g.delimiter(),
                    replace(g.stream(), var.clone(), range.clone())?,
                );
                new_group.set_span(g.span());
                ts.append(TokenTree::Group(new_group));
                continue;
            }
            TokenTree::Ident(ident) if ident.to_string() == var.0.to_string() => {
                ts.append(TokenTree::Literal(Literal::usize_unsuffixed(
                    var.1.ok_or(UnmetSpecificArea)?,
                )));
                continue;
            }
            TokenTree::Ident(ident) => {
                if let Some(tilde) = iter.next_if(|t1| match t1 {
                    TokenTree::Punct(punct) if punct.as_char() == '~' => true,
                    _ => false,
                }) {
                    match iter.next() {
                        Some(TokenTree::Ident(v)) if v.to_string() == var.0.to_string() => {
                            let new_ident = Ident::new(
                                format!("{}{}", ident.to_string(), var.1.ok_or(UnmetSpecificArea)?)
                                    .as_str(),
                                ident.span(),
                            );
                            ts.append(new_ident);
                            continue;
                        }
                        _ => {
                            return Ok(syn::Error::new_spanned(tilde, "invalid usage of `~`")
                                .into_compile_error())
                        }
                    }
                }
            }
            TokenTree::Punct(punct) if punct.as_char() == '#' => {
                if let Some(TokenTree::Group(paren_group)) = iter.next_if(|t1| match t1 {
                    TokenTree::Group(g) if g.delimiter() == Parenthesis => true,
                    _ => false,
                }) {
                    if iter
                        .next_if(|t2| match t2 {
                            TokenTree::Punct(punct) if punct.as_char() == '*' => true,
                            _ => false,
                        })
                        .is_some()
                    {
                        let specific_area = paren_group.stream();
                        for i in range.clone() {
                            let interpolated_part = replace(
                                specific_area.clone(),
                                (var.0.clone(), Some(i)),
                                range.clone(),
                            );
                            ts.append_all(interpolated_part);
                        }
                        continue;
                    } else {
                        let mut new_group = Group::new(
                            paren_group.delimiter(),
                            replace(paren_group.stream(), var.clone(), range.clone())?,
                        );
                        new_group.set_span(paren_group.span());
                        ts.append(paren_group);
                        continue;
                    }
                }
            }
            _ => {}
        }

        ts.append(t0);
    }

    Ok(ts)
}

#[derive(Debug)]
struct SeqInput {
    var: Ident,
    #[allow(dead_code)]
    in_token: Token![in],
    start: LitInt,
    #[allow(dead_code)]
    range_token: Token![..],
    end: LitInt,
    #[allow(dead_code)]
    brace_token: syn::token::Brace,
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
