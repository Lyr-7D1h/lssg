use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{quote, ToTokens, TokenStreamExt};
use virtual_dom::{parse_html, Html};

mod html;

// using https://github.com/chinedufn/percy/blob/master/crates/html-macro/src/lib.rs as example
/// parse html
#[proc_macro]
pub fn html(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let tokens = match parse_html(input.to_string().as_bytes()) {
        Ok(t) => t,
        Err(e) => {
            let e = syn::Error::new(Span::call_site(), e);
            return proc_macro::TokenStream::from(e.to_compile_error());
        }
    };
    let doc = HtmlDocument { tokens };

    quote! {
        {
            use std::collections::HashMap;
            use virtual_dom::*;
            #doc
        }
    }
    .into()
}

struct HtmlDocument {
    tokens: Vec<Html>,
}

impl ToTokens for HtmlDocument {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let parsed: Vec<TokenStream> = self
            .tokens
            .iter()
            .filter_map(|t| match t {
                Html::Comment { text } => None,
                Html::Text { text } => Some(quote!(Html::Text { text: #text.to_string() })),
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
                    let children = HtmlDocument {
                        tokens: children.clone(),
                    };

                    let children = if children.tokens.len() > 1 {
                        children.to_token_stream()
                    } else {
                        quote!(vec![#children])
                    };

                    Some(quote!({
                        let mut attributes = HashMap::new();
                        #(#attributes_values)*

                        Html::Element {
                            attributes,
                            children: #children,
                            tag: #tag.to_string()
                        }
                    }))
                }
            })
            .collect();

        if parsed.len() > 1 {
            tokens.extend(quote!(vec![#(#parsed,)*]));
        } else if parsed.len() == 1 {
            let p = &parsed[0];
            tokens.extend(quote!(#p))
        }
    }
}
