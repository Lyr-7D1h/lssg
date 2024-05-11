use log::warn;

use crate::{
    lmarkdown::Token,
    renderer::RenderContext,
    sitetree::{Page, Relation},
};

pub fn tokens_to_text(tokens: &Vec<Token>) -> String {
    let mut result = String::new();
    for t in tokens {
        if let Some(text) = t.to_text() {
            result.push_str(&text)
        }
    }
    return result;
}

pub fn process_href(href: &String, context: &RenderContext) -> String {
    if Page::is_href_to_page(href) {
        let to_id = context
            .site_tree
            .links_from(context.site_id)
            .into_iter()
            .find_map(|l| {
                if let Relation::Discovered { raw_path: path } = &l.relation {
                    if path == href {
                        return Some(l.to);
                    }
                }
                None
            });

        if let Some(to_id) = to_id {
            let rel_path = context.site_tree.path(to_id);
            rel_path
        } else {
            warn!("Could not find node where {href:?} points to");
            href.to_owned()
        }
    } else {
        href.to_owned()
    }
}
