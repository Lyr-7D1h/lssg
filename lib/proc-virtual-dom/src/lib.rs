use std::{collections::HashMap, str::Chars};

use proc_macro2::{Span, TokenStream, TokenTree};
use quote::quote;
use syn::{parse::Parse, parse_macro_input, token::Brace, Block, Expr, Ident, Stmt};
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
/// use proc_virtual_dom::dom;
/// dom!(<div>" This is my text with preserved whitespace "</div>);
/// ```
///
/// # Examples
///
/// ```
/// use proc_virtual_dom::dom;
/// let title = "My beautiful website";
/// let content = dom! {
///     <div>{title}</div>
/// };
/// ```
#[proc_macro]
pub fn dom(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let parsed_content = input
        .to_string()
        // Normalize newlines within HTML tags to spaces to fix parsing issues
        // when TokenStream.to_string() inserts newlines in tag attributes
        .replace("\n", " ");
    let template = parse_macro_input!(input as Template);

    let tokens = match parse_html(parsed_content.to_string().as_bytes()) {
        Ok(t) => t,
        Err(e) => {
            let e = syn::Error::new(Span::call_site(), e);
            return proc_macro::TokenStream::from(e.to_compile_error());
        }
    };

    let html = to_tokens(&tokens, &template, None, 0);

    if tokens.len() > 1 {
        let children = quote!(vec![#({#html},)*]);
        return quote! {
            {
                use ::std::collections::HashMap;
                use ::virtual_dom::*;
                #children
            }
        }
        .into();
    }

    let html = html.into_iter().next().expect("no html given");
    quote! {
        {
            use ::std::collections::HashMap;
            use ::virtual_dom::*;
            #html
        }
    }
    .into()
}

/// collect all interpolated variables
#[derive(Clone)]
struct Template {
    variables: HashMap<String, Ident>,
}

impl Parse for Template {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut variables = HashMap::new();

        while !input.is_empty() {
            if input.peek(Brace) {
                let content;
                syn::braced!(content in input);
                for s in content.call(Block::parse_within)? {
                    match s {
                        Stmt::Expr(e, _) => {
                            if let Expr::Path(p) = e {
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
    for c in chars.by_ref() {
        raw.push(c);
        match c {
            // if whitespace in between alphabetical not a valid block
            ' ' | '\n' if !variable_name.is_empty() => {
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

/// check if string has interpolated character if so add it
fn interpolate_string(text: &str, template: &Template) -> TokenStream {
    let mut chars = text.chars();
    let mut variables = vec![];
    let mut text = String::new();
    // if text has interpolated variables add
    while let Some(c) = chars.next() {
        if c == '{' {
            match parse_braces(&mut chars) {
                Ok(variable_name) => {
                    let variable = template.variables.get(&variable_name).unwrap_or_else(|| panic!(
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
    if !variables.is_empty() {
        return quote!(format!(#text, #(#variables,)*));
    }
    quote!(String::from(#text))
}

fn to_tokens(
    tokens: &Vec<Html>,
    template: &Template,
    parent: Option<&Ident>,
    mut i: usize,
) -> Vec<TokenStream> {
    let mut items = vec![];

    for t in tokens {
        i += 1;
        match t {
            Html::Comment { .. } => {}
            Html::Text { text } => {
                // if text starts with quotes and ends with quotes remove quotes
                let text = if text.starts_with("\"") && text.ends_with("\"") {
                    let text = text.get(1..text.len() - 1).unwrap_or("");
                    if text.is_empty() {
                        continue;
                    }
                    text
                } else {
                    text
                };

                let mut chars = text.chars();
                let mut text = String::new();
                while let Some(c) = chars.next() {
                    if c == '{' {
                        match parse_braces(&mut chars) {
                            Ok(variable_name) => {
                                let variable = template.variables.get(&variable_name).unwrap_or_else(|| panic!(
                                    "failed to parse or find variable '{variable_name}'"
                                ));
                                if let Some(parent) = parent {
                                    if !text.is_empty() {
                                        items.push(
                                            quote!(#parent.append_child(DomNode::create_text(#text))),
                                        );
                                        text.clear();
                                    }
                                    items.push(quote!(#parent.append_child(#variable)));
                                } else {
                                    if !text.is_empty() {
                                        items.push(quote!(DomNode::create_text(#text)));
                                        text.clear();
                                    }
                                    items.push(quote!(#variable));
                                }
                            }
                            Err(t) => text.push_str(&t),
                        }
                    } else {
                        text.push(c);
                    }
                }
                if !text.is_empty() {
                    if let Some(parent) = parent {
                        items.push(quote!(#parent.append_child(DomNode::create_text(#text))));
                    } else {
                        items.push(quote!(DomNode::create_text(#text)));
                    }
                }
            }
            Html::Element {
                tag,
                attributes,
                children,
            } => {
                let attributes_values = attributes.iter().map(|(key, value)| {
                    let key = interpolate_string(key, template);
                    let value = interpolate_string(value, template);
                    quote! {
                        attributes.insert(#key, #value);
                    }
                });

                let id = Ident::new(&format!("node_{i}"), Span::call_site());

                let el = if !children.is_empty() {
                    let children = to_tokens(children, template, Some(&id), i + tokens.len());
                    quote!(
                        let mut attributes = HashMap::new();
                        #(#attributes_values)*
                        let #id = DomNode::create_element_with_attributes(#tag, attributes);
                        #({#children})*;
                    )
                } else {
                    quote!(
                        let mut attributes = HashMap::new();
                        #(#attributes_values)*
                        let #id = DomNode::create_element_with_attributes(#tag, attributes);
                    )
                };

                if let Some(parent) = parent {
                    items.push(quote!(
                        #el
                        #parent.append_child(#id);
                    ))
                } else {
                    items.push(quote!(
                        #el
                        #id
                    ))
                }
            }
        }
    }

    items
}
