use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::Parse,
    parse_macro_input,
    token::{Brace, Token},
    Block, Expr, ExprLit, ExprPath, Lit, LitStr, PatPath, Stmt,
};
use virtual_dom::{parse_html, Html};

// using https://github.com/chinedufn/percy/blob/master/crates/html-macro/src/lib.rs as example
/// parse html
#[proc_macro]
pub fn html(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let parsed_content = input.to_string();
    let template = parse_macro_input!(input as Template);
    // let input = parse_macro_input!(input as syn::Expr);
    // let interpolated_input = interpolate_html(&input.to_string());

    let tokens = match parse_html(parsed_content.to_string().as_bytes()) {
        Ok(t) => t,
        Err(e) => {
            let e = syn::Error::new(Span::call_site(), e);
            return proc_macro::TokenStream::from(e.to_compile_error());
        }
    };

    let mut template_token = 0;
    let doc = HtmlDocument { tokens };

    let html = to_tokens(&doc, &template, &mut template_token);

    quote! {
            {
                use std::collections::HashMap;
                use virtual_dom::*;
                #html
    }
        }
    .into()
}

/// used for interpolating variables
#[derive(Clone)]
struct Template {
    tokens: Vec<ExprPath>,
}

impl Parse for Template {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut tokens = vec![];
        while input.is_empty() == false {
            if input.peek(Brace) {
                let content;
                syn::braced!(content in input);
                for s in content.call(Block::parse_within)? {
                    match s {
                        Stmt::Expr(e, _) => match e {
                            _ => match e {
                                Expr::Path(p) => tokens.push(p),
                                _ => {}
                            },
                        },
                        Stmt::Local(_) | Stmt::Item(_) | Stmt::Macro(_) => {
                            return Err(input.error("unexpected statement"));
                        }
                    }
                }
                continue;
            }
            let tt: TokenTree = input.parse()?;
        }

        Ok(Template { tokens })
    }
}

fn to_tokens(doc: &HtmlDocument, template: &Template, template_token: &mut usize) -> TokenStream {
    let mut stream = TokenStream::new();
    let parsed: Vec<TokenStream> = doc
        .tokens
        .iter()
        .filter_map(|t| match t {
            Html::Comment { .. } => None,
            Html::Text { text } => {
                // if text starts with quotes and ends with quotes remove quotes
                let text = if text.starts_with("\"") && text.ends_with("\"") {
                    let text = text.get(1..text.len() - 1).unwrap_or("".into());
                    if text.len() == 0 {
                        return None;
                    }
                    text
                } else {
                    text
                };

                let mut chars = text.chars();
                let mut variables = vec![];
                let mut text = String::new();
                while let Some(c) = chars.next() {
                    if c == '{' {
                        let mut variable_name = String::new();
                        let mut var_has_whitespace = false;
                        while let Some(c) = chars.next() {
                            match c {
                                // if whitespace in between alphabetical not a valid block
                                ' ' | '\n' if variable_name.len() > 0 => {
                                    var_has_whitespace = true;
                                }
                                // ignore whitespace
                                ' ' | '\n' => {}
                                '}' => {
                                    let variable = &template.tokens[*template_token];
                                    *template_token += 1;
                                    text.push_str("{}");
                                    variables.push(quote!(#variable));
                                    break;
                                }
                                // if not alphatic character not valid interpolation
                                c if !c.is_alphabetic() => {
                                    text.push_str(&variable_name);
                                    text.push(c);
                                    break;
                                }
                                _ => {
                                    // if whitespace in between characters not valid
                                    if var_has_whitespace {
                                        text.push_str(&variable_name);
                                        text.push(c);
                                        break;
                                    }
                                    variable_name.push(c)
                                }
                            }
                        }
                    } else {
                        text.push(c);
                    }
                }
                Some(quote!(Html::Text {
                    text: format!(#text, #(#variables,)*)
                }))
            }
            Html::Element {
                tag,
                attributes,
                children,
            } => {
                let attributes_values = attributes.iter().map(|(key, value)| {
                    quote! {
                        attributes.insert(#key.to_string(), #value.to_string());
                    }
                });

                let parse_children = to_tokens(
                    &HtmlDocument {
                        tokens: children.clone(),
                    },
                    template,
                    template_token,
                );

                let parsed_children = if children.len() > 1 {
                    parse_children
                } else {
                    quote!(vec![#parse_children])
                };

                Some(quote!({
                    let mut attributes = HashMap::new();
                    #(#attributes_values)*

                    Html::Element {
                        attributes,
                        children: #parsed_children,
                        tag: #tag.to_string()
                    }
                }))
            }
        })
        .collect();

    if parsed.len() > 1 {
        stream.extend(quote!(vec![#(#parsed,)*]));
    } else if parsed.len() == 1 {
        let p = &parsed[0];
        stream.extend(quote!(#p))
    }
    stream
}

struct HtmlDocument {
    tokens: Vec<Html>,
}

// fn interpolate_html(html_content: &str) -> proc_macro2::TokenStream {
//     let mut tokens = proc_macro2::TokenStream::new();
//     let mut buffer = String::new();
//     let mut in_interpolation = false;
//
//     for c in html_content.chars() {
//         if c == '{' {
//             if in_interpolation {
//                 // Nested opening brace
//                 buffer.push(c);
//             } else {
//                 // Start of interpolation
//                 in_interpolation = true;
//                 if !buffer.is_empty() {
//                     // If there's text before the interpolation, add it as a string literal
//                     let lit = LitStr::new(&buffer, proc_macro2::Span::call_site());
//                     tokens.extend(std::iter::once(TokenTree::Literal(lit)));
//                     buffer.clear();
//                 }
//             }
//         } else if c == '}' {
//             if in_interpolation {
//                 // End of interpolation
//                 in_interpolation = false;
//                 if !buffer.is_empty() {
//                     // Process the interpolation
//                     tokens.extend(parse_interpolation(&buffer));
//                     buffer.clear();
//                 }
//             } else {
//                 // Nested closing brace
//                 buffer.push(c);
//             }
//         } else {
//             // Regular character
//             buffer.push(c);
//         }
//     }
//     println!("{buffer:?}");
//
//     // If there's remaining text after the loop, add it as a string literal
//     if !buffer.is_empty() {
//         let t = TokenTree::Literal(Literal::string(&buffer));
//         tokens.extend(std::iter::once(t));
//     }
//
//     tokens
// }
// fn parse_interpolation(interpolation: &str) -> proc_macro2::TokenStream {
//     println!("AAA");
//     // Here you can extend this function to handle parsing of Rust expressions inside the interpolation
//     // For simplicity, let's just return the interpolation as a string literal
//     let lit = Lit::Str(LitStr::new(interpolation, proc_macro2::Span::call_site()));
//     quote!(#lit)
// }

// fn interpolate_html(input: Expr) -> proc_macro2::TokenStream {
//     println!("AA ");
//     match input {
//         Expr::Lit(ExprLit {
//             lit: Lit::Str(ref lit_str),
//             ..
//         }) => {
//             // If the expression is a string literal, return it as is
//             let lit_str = lit_str.value();
//             quote! {
//                 #lit_str
//             }
//         }
//         Expr::Path(expr_path) => {
//             // If the expression is a path, get its string representation
//             let path_str = expr_path.path.into_token_stream().to_string();
//             quote! {
//                 #path_str
//             }
//         }
//         Expr::Group(expr_group) => {
//             // If the expression is a group, recursively interpolate its content
//             let inner_tokens = interpolate_html(*expr_group.expr);
//             quote! {
//                 (#inner_tokens)
//             }
//         }
//         Expr::Block(block) => {
//             // If the expression is a block, recursively interpolate its content
//             let content = match &block.block.stmts.first() {
//                 Some(expr) => match expr {
//                     Stmt::Expr(expr, _) => interpolate_html(expr.clone()),
//                     _ => panic!("Unsupported expression type inside html! macro."),
//                 },
//                 None => quote! { "" },
//             };
//             println!("AA {content}");
//             quote! {
//                 #content
//             }
//         }
//         Expr::Unary(_) => {
//             // If the expression is a unary operator, recursively interpolate its operand
//             let inner_tokens = interpolate_html(input);
//             quote! {
//                 #inner_tokens
//             }
//         }
//         _ => {
//             // For other types of expressions, return an empty string
//             quote! {
//                 ""
//             }
//         }
//     }
// }
