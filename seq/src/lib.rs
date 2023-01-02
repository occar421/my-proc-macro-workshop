use proc_macro::TokenStream;
use proc_macro2::Delimiter::Parenthesis;
use proc_macro2::{Group, Ident, Literal, TokenStream as TokenStream2, TokenTree};
use quote::{quote, TokenStreamExt};
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

    // let (pre, seq_target, post) = get_specific_part(input.body.clone()).unwrap_or((
    //     TokenStream2::new(),
    //     input.body,
    //     TokenStream2::new(),
    // ));
    //
    // dbg!(&pre, &seq_target, &post);

    replace(input.body.clone(), (input.var.clone(), None), start, end).into()
}

// fn get_specific_part(ts: TokenStream2) -> Option<(TokenStream2, TokenStream2, TokenStream2)> {
//     let ts: Vec<_> = ts.into_iter().collect();
//     for (i, slice) in ts.clone().windows(3).enumerate() {
//         match (&slice[0], &slice[1], &slice[2]) {
//             (TokenTree::Punct(sharp), TokenTree::Group(paren_group), TokenTree::Punct(star))
//                 if sharp.as_char() == '#'
//                     && paren_group.delimiter() == Parenthesis
//                     && star.as_char() == '*' =>
//             {
//                 // maybe low perf here
//
//                 // before sharp
//                 let pre = TokenStream2::from_iter(ts.clone().into_iter().take(i));
//
//                 // after sharp
//                 let post = TokenStream2::from_iter(ts.into_iter().skip(i + 3));
//
//                 return Some((pre, paren_group.stream(), post));
//             }
//             _ => {}
//         }
//     }
//
//     ts.iter().find_map(|x| match x {
//         TokenTree::Group(g) => get_specific_part(g.stream()),
//         _ => None,
//     })
// }

fn replace(
    ts: TokenStream2,
    var: (Ident, Option<usize>),
    start: usize,
    end: usize,
) -> TokenStream2 {
    let mut iter = ts.into_iter().peekable();
    let mut ts = TokenStream2::new();

    while let Some(t0) = iter.next() {
        match &t0 {
            TokenTree::Group(g) => {
                let mut new_group =
                    Group::new(g.delimiter(), replace(g.stream(), var.clone(), start, end));
                new_group.set_span(g.span());
                ts.append(TokenTree::Group(new_group));
            }
            TokenTree::Ident(ident) if ident.to_string() == var.0.to_string() => ts.append(
                TokenTree::Literal(Literal::usize_unsuffixed(var.1.expect("aaa"))),
            ),
            TokenTree::Ident(ident) => {
                if let Some(tilde) = iter.next_if(|t1| match t1 {
                    TokenTree::Punct(punct) if punct.as_char() == '~' => true,
                    _ => false,
                }) {
                    match iter.next() {
                        Some(TokenTree::Ident(v)) if v.to_string() == var.0.to_string() => {
                            let new_ident = Ident::new(
                                format!("{}{}", ident.to_string(), var.1.expect("vbb")).as_str(),
                                ident.span(),
                            );
                            ts.append(new_ident);
                        }
                        _ => {
                            return syn::Error::new_spanned(tilde, "invalid usage of `~`")
                                .into_compile_error()
                        }
                    }
                } else {
                    ts.append(t0);
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
                        let specific_part = paren_group.stream();
                        for i in start..end {
                            let mut new_group = Group::new(
                                paren_group.delimiter(),
                                replace(
                                    specific_part.clone(),
                                    (var.0.clone(), Some(i)),
                                    start,
                                    end,
                                ),
                            );
                            new_group.set_span(paren_group.span());
                            ts.append(new_group);
                        }
                    } else {
                        let mut new_group = Group::new(
                            paren_group.delimiter(),
                            replace(paren_group.stream(), var.clone(), start, end),
                        );
                        new_group.set_span(paren_group.span());
                        ts.append(paren_group);
                    }
                }

                dbg!(&iter.peek());
            }
            _ => ts.append(t0),
        }
    }

    ts
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

// #()* があるときだけ繰り返しスコープを狭める。無いときは全体がスコープ。
