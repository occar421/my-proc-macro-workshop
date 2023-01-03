use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::visit_mut::VisitMut;
use syn::{parse_macro_input, ExprMatch, Item, Pat};

#[proc_macro_attribute]
pub fn sorted(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;
    let mut item = parse_macro_input!(input as syn::Item);

    handle(&mut item, validate_item_enum)
}

#[proc_macro_attribute]
pub fn check(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;
    let mut item = parse_macro_input!(input as syn::Item);

    handle(&mut item, validate_item_fn)
}

fn handle(item: &mut Item, validate: impl Fn(&mut Item) -> syn::Result<()>) -> TokenStream {
    match validate(item) {
        Ok(_) => quote! { #item },
        Err(e) => {
            let compile_error = e.into_compile_error();

            quote! {
                #item

                #compile_error
            }
        }
    }
    .into()
}

fn validate_item_enum(item: &mut Item) -> syn::Result<()> {
    let item = match item {
        Item::Enum(e) => e,
        _ => {
            return Err(syn::Error::new(
                Span::call_site(),
                "expected enum or match expression",
            ))
        }
    };

    let ident_iter = item.variants.iter().map(|v| &v.ident);
    let mut sorted_idents: Vec<_> = ident_iter.clone().collect();
    sorted_idents.sort_unstable();

    for (actual_ident, right_ident) in ident_iter.zip(&sorted_idents) {
        if &actual_ident != right_ident {
            return Err(syn::Error::new_spanned(
                &right_ident,
                format!("{} should sort before {}", right_ident, actual_ident),
            ));
        }
    }

    Ok(())
}

fn validate_item_fn(item: &mut Item) -> syn::Result<()> {
    let mut item = match item {
        Item::Fn(f) => f,
        _ => return Err(syn::Error::new(Span::call_site(), "aaa")),
    };

    let mut visitor = ExprVisitor::new();
    visitor.visit_item_fn_mut(&mut item);

    let errors = visitor.get_errors();
    if errors.is_empty() {
        Ok(())
    } else {
        let mut result = errors[0].clone();
        for error in errors.iter().skip(1) {
            result.combine(error.clone());
        }
        Err(result)
    }
}

struct ExprVisitor {
    errors: Vec<syn::Error>,
}

impl ExprVisitor {
    fn new() -> Self {
        Self { errors: vec![] }
    }

    fn get_errors(&self) -> &Vec<syn::Error> {
        &self.errors
    }
}

impl VisitMut for ExprVisitor {
    fn visit_expr_match_mut(&mut self, node: &mut ExprMatch) {
        let sorted_attr_pos = node.attrs.iter().position(|a| match a.path.get_ident() {
            Some(ident) if ident == "sorted" => true,
            _ => false,
        });
        if let Some(sorted_attr_pos) = sorted_attr_pos {
            node.attrs.remove(sorted_attr_pos);

            let path_iter = node.arms.iter().map(|a| match &a.pat {
                Pat::TupleStruct(ts) => &ts.path,
                _ => unimplemented!(),
            });
            let mut sorted_paths: Vec<_> = path_iter.clone().collect();
            sorted_paths.sort_unstable_by_key(|x| x.get_ident().unwrap());

            for (actual_path, &right_path) in path_iter.zip(&sorted_paths) {
                let actual_ident = actual_path.get_ident().unwrap();
                let right_ident = right_path.get_ident().unwrap();
                if actual_ident != right_ident {
                    self.errors.push(syn::Error::new_spanned(
                        &right_path,
                        format!("{} should sort before {}", right_ident, actual_ident),
                    ));
                    break;
                }
            }
        }

        syn::visit_mut::visit_expr_match_mut(self, node);
    }
}
