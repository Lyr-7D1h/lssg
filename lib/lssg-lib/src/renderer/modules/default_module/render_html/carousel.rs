use std::collections::HashMap;

use proc_virtual_dom::dom;
use virtual_dom::{Document, DomNode};

use crate::{
    lmarkdown::Token,
    renderer::{RenderContext, TokenRenderer, util::tokens_to_text},
};

fn first_non_empty_text(candidates: &[Option<String>]) -> Option<String> {
    candidates
        .iter()
        .flatten()
        .map(|s| s.trim())
        .find(|s| !s.is_empty())
        .map(|s| s.to_owned())
}

fn token_carousel_title(token: &Token) -> Option<String> {
    match token {
        Token::Image { tokens, title, .. } => {
            first_non_empty_text(&[title.clone(), Some(tokens_to_text(tokens))])
        }
        Token::Link { tokens, title, .. } => {
            let nested_image_title = tokens.iter().find_map(|t| {
                if let Token::Image { title, tokens, .. } = t {
                    first_non_empty_text(&[title.clone(), Some(tokens_to_text(tokens))])
                } else {
                    None
                }
            });

            first_non_empty_text(&[
                title.clone(),
                nested_image_title,
                Some(tokens_to_text(tokens)),
            ])
        }
        Token::Html {
            attributes, tokens, ..
        } => first_non_empty_text(&[
            attributes.get("title").cloned(),
            attributes.get("alt").cloned(),
            attributes.get("aria-label").cloned(),
            Some(tokens_to_text(tokens)),
        ]),
        _ => None,
    }
}

pub fn carousel(
    document: &mut Document,
    context: &RenderContext,
    parent: &DomNode,
    tr: &mut TokenRenderer,
    attributes: &HashMap<String, String>,
    tokens: &[Token],
) {
    if tokens.is_empty() {
        return;
    }

    let carousel_tokens: Vec<&Token> = tokens
        .iter()
        .filter(|t| {
            matches!(
                t,
                Token::Link { .. } | Token::Image { .. } | Token::Html { .. }
            )
        })
        .collect();

    if carousel_tokens.is_empty() {
        return;
    }

    let show_slide_titles = attributes.contains_key("title");

    let carousel = dom!(<div class="default__carousel"></div>);

    // Main viewport
    let viewport = dom!(<div class="default__carousel_viewport"></div>);
    let container = dom!(<div class="default__carousel_container"></div>);

    // Create slides for main viewer
    let mut total = 0;
    for t in &carousel_tokens {
        let rendered = document.create_element("div");
        tr.render(
            document,
            context,
            rendered.clone(),
            std::slice::from_ref(*t),
        );
        for item in rendered.children() {
            let idx = total;
            total += 1;
            let slide = dom!(<div class="default__carousel_slide" data-index="{idx}"></div>);
            let inner = dom!(<div class="default__carousel_slide_inner"></div>);
            inner.append_child(item);
            if show_slide_titles && let Some(title) = token_carousel_title(t) {
                inner.append_child(dom!(
                    <div class="default__carousel_slide_title">{title}</div>
                ));
            }
            slide.append_child(inner);
            container.append_child(slide);
        }
    }

    if total == 0 {
        return;
    }

    viewport.append_child(container);

    if total > 1 {
        let navigation = dom!(<div class="default__carousel_navigation"></div>);
        let prev = dom!(
            <button
                class="default__carousel_btn default__carousel_btn_prev"
                onclick="default__carouselPrev(event)"
                aria-label="Previous slide"
            ></button>
        );
        let next = dom!(
            <button
                class="default__carousel_btn default__carousel_btn_next"
                onclick="default__carouselNext(event)"
                aria-label="Next slide"
            ></button>
        );
        let zoom = dom!(
            <button
                class="default__carousel_btn default__carousel_btn_zoom"
                onclick="default__carouselZoom(event)"
                aria-label="Zoom image"
            ></button>
        );
        navigation.append_child(prev);
        navigation.append_child(next);
        navigation.append_child(zoom);
        viewport.append_child(navigation);
    }

    carousel.append_child(viewport);

    // Thumbnails
    if total > 1 {
        let thumbs_viewport = dom!(<div class="default__carousel_thumbs_viewport"></div>);
        let thumbs_container = dom!(<div class="default__carousel_thumbs_container"></div>);

        let mut thumb_idx = 0;
        for t in &carousel_tokens {
            let rendered = document.create_element("div");
            tr.render(
                document,
                context,
                rendered.clone(),
                std::slice::from_ref(*t),
            );
            for item in rendered.children() {
                let idx = thumb_idx;
                thumb_idx += 1;
                let thumb = dom!(<button class="default__carousel_thumb" onclick="default__carouselGoTo(event, {idx})" data-index="{idx}"></button>);
                let thumb_inner = dom!(<div class="default__carousel_thumb_inner"></div>);
                thumb_inner.append_child(item);
                thumb.append_child(thumb_inner);
                thumbs_container.append_child(thumb);
            }
        }

        thumbs_viewport.append_child(thumbs_container);
        carousel.append_child(thumbs_viewport);
    }

    parent.append_child(carousel);
}
