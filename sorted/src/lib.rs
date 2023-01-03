use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::token::Colon2;
use syn::visit_mut::VisitMut;
use syn::{parse_macro_input, ExprMatch, Item, Pat, Path};

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
                Pat::Path(pp) => &pp.path,
                Pat::TupleStruct(ts) => &ts.path,
                Pat::Struct(ps) => &ps.path,
                _ => unimplemented!(),
            });
            let mut sorted_paths: Vec<_> = path_iter.clone().collect();
            sorted_paths.sort_unstable_by_key(|p| get_ident(p));

            for (actual_path, &right_path) in path_iter.zip(&sorted_paths) {
                let actual_ident = get_ident(actual_path);
                let right_ident = get_ident(right_path);
                if actual_ident != right_ident {
                    self.errors.push(syn::Error::new_spanned(
                        &right_path,
                        format!(
                            "{} should sort before {}",
                            display_path(right_path),
                            display_path(actual_path)
                        ),
                    ));
                    break;
                }
            }
        }

        syn::visit_mut::visit_expr_match_mut(self, node)
    }
}

fn get_ident(path: &Path) -> &Ident {
    &path.segments.last().unwrap().ident
}

fn display_path(path: &Path) -> String {
    fn display_colon2(colon2: Option<&Colon2>) -> String {
        colon2.map_or(String::new(), |_| "::".to_string())
    }

    path.segments
        .pairs()
        .fold(display_colon2(path.leading_colon.as_ref()), |acc, pair| {
            acc + &pair.value().ident.to_string() + &display_colon2(pair.punct().cloned())
        })
}
