use proc_macro2::{Span, TokenStream, TokenTree};
use quote::quote;
use syn::{parse::Parse, parse_macro_input, token::Brace, Block, Expr, ExprPath, Stmt};
use virtual_dom::{parse_html, Html};

// using https://github.com/chinedufn/percy/blob/master/crates/html-macro/src/lib.rs as example
/// Parse a string into virtual_dom::DomNode's with minimal variable interpolation
///
/// Because of the nature of macros whitespace is fairly arbitrary and might spawn spaces or
/// newlines in between text one way to prevent this it to explicitly add quotes around your text.
///
/// eg.
///
/// ```
/// use proc_html::html;
/// html!(<div>" This is my text with preserved whitespace "</div>);
/// ```
///
/// # Examples
///
/// ```
/// use proc_html::html;
/// let title = "My beautiful website";
/// let content = html! {
///     <div>{title}</div>
/// };
/// ```
#[proc_macro]
pub fn html(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let parsed_content = input.to_string();
    let template = parse_macro_input!(input as Template);

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

/// collect all interpolated variables
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
            // parse other expressions
            let _ = input.parse::<TokenTree>();
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
                // if text has interpolated variables add
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
