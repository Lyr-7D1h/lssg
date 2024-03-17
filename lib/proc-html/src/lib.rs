use std::{collections::HashMap, fmt::write, str::Chars};

use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, parse_quote, parse_str,
    token::Brace,
    Block, Expr, ExprPath, Ident, Stmt, Token,
};
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
    // tokens: Vec<ExprPath>,
    variables: HashMap<String, Ident>,
}

impl Parse for Template {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // let mut tokens = vec![];
        let mut variables = HashMap::new();

        while input.is_empty() == false {
            if input.peek(Brace) {
                let content;
                syn::braced!(content in input);
                for s in content.call(Block::parse_within)? {
                    match s {
                        Stmt::Expr(e, _) => match e {
                            _ => match e {
                                Expr::Path(p) => {
                                    let a = p.path.segments[0].ident.to_string();
                                    let ident = p
                                        .path
                                        .segments
                                        .first()
                                        .expect("path does not include ident")
                                        .ident
                                        .clone();
                                    variables.insert(a, ident);
                                }
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
            let t = input.parse::<TokenTree>()?;

            // also check literals for brackets
            if let TokenTree::Literal(l) = t {
                let text = l.to_string();
                let mut chars = text.chars();
                while let Some(c) = chars.next() {
                    if c == '{' {
                        if let Ok(var) = parse_braces(&mut chars) {
                            let ident = Ident::new(&var, l.span());
                            variables.insert(var, ident);
                        }
                    }
                }
            }
        }

        Ok(Template { variables })
    }
}

/// parse a text with braces and return the variable name if syntax is valid otherwise return raw
/// string
fn parse_braces(chars: &mut Chars) -> Result<String, String> {
    let mut raw = String::new();
    let mut variable_name = String::new();
    let mut var_has_whitespace = false;
    while let Some(c) = chars.next() {
        raw.push(c);
        match c {
            // if whitespace in between alphabetical not a valid block
            ' ' | '\n' if variable_name.len() > 0 => {
                var_has_whitespace = true;
            }
            // ignore whitespace
            ' ' | '\n' => {}
            '}' => {
                return Ok(variable_name);
            }
            // if not alphatic character not valid interpolation
            c if !c.is_alphabetic() => {
                return Err(raw);
            }
            _ => {
                // if whitespace in between characters not valid
                if var_has_whitespace {
                    return Err(raw);
                }
                variable_name.push(c)
            }
        }
    }

    Err(raw)
}

/// add variables into a piece of string
fn interpolate(text: &str, template: &Template) -> TokenStream {
    let mut chars = text.chars();
    let mut variables = vec![];
    let mut text = String::new();
    // if text has interpolated variables add
    while let Some(c) = chars.next() {
        if c == '{' {
            match parse_braces(&mut chars) {
                Ok(variable_name) => {
                    let variable = template.variables.get(&variable_name).expect(&format!(
                        "failed to parse or find variable '{variable_name}'"
                    ));
                    text.push_str("{}");
                    variables.push(quote!(#variable));
                }
                Err(t) => text.push_str(&t),
            }
        } else {
            text.push(c);
        }
    }
    quote!(format!(#text, #(#variables,)*))
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

                let text = interpolate(&text, template);

                Some(quote!(Html::Text {
                    text: #text
                }))
            }
            Html::Element {
                tag,
                attributes,
                children,
            } => {
                let attributes_values = attributes.iter().map(|(key, value)| {
                    let key = interpolate(key, template);
                    let value = interpolate(value, template);
                    quote! {
                        attributes.insert(#key, #value);
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
