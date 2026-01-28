use crate::{
    lmarkdown::Token,
    renderer::RendererModule,
    sitetree::{Input, Relation, Resource, SiteNode, SiteTree},
    tree::Node,
};

pub const MODEL_VIEWER_JS: &'static str = include_str!("./model_module/model-viewer-v4.min.js");

/// Implements https://modelviewer.dev/
#[derive(Default)]
pub struct ModelModule {}

impl RendererModule for ModelModule {
    fn id(&self) -> &'static str {
        "model"
    }

    fn init(&mut self, site_tree: &mut SiteTree) -> Result<(), crate::lssg_error::LssgError> {
        let active_pages: Vec<_> = site_tree
            .pages()
            .filter_map(|(id, page)| {
                for t in page.tokens() {
                    match t {
                        Token::Html {
                            tokens,
                            tag,
                            attributes,
                        } if tag == "model-viewer" => return Some((id, attributes.clone())),
                        _ => {}
                    }
                }
                None
            })
            .collect();

        if !active_pages.is_empty() {
            let viewer_resource_id = site_tree.add(SiteNode::resource(
                "model-viewer.min.js",
                site_tree.root(),
                Resource::new_static(MODEL_VIEWER_JS.to_string()),
            ));
            for (page_id, attributes) in active_pages {
                println!("{attributes:?}");
                if let Some(src) = attributes.get("src")
                    && let Ok(input) = Input::from_string(src)
                    && let Ok(name) = input
                        .filename()
                        .inspect_err(|e| log::error!("Failed to get input filename {input}: {e}"))
                {
                    let to = site_tree.add(SiteNode::resource(
                        name,
                        site_tree[page_id].parent().unwrap_or(site_tree.root()),
                        Resource::new_fetched(input),
                    ));
                    site_tree.add_link(
                        page_id,
                        to,
                        crate::sitetree::Relation::Discovered {
                            raw_path: src.clone(),
                        },
                    );
                }

                site_tree.add_link(page_id, viewer_resource_id, Relation::External);
            }
        }

        Ok(())
    }

    // fn render_body<'n>(
    //     &mut self,
    //     document: &mut virtual_dom::Document,
    //     context: &crate::renderer::RenderContext<'n>,
    //     parent: virtual_dom::DomNode,
    //     token: &Token,
    //     tr: &mut crate::renderer::TokenRenderer,
    // ) -> Option<virtual_dom::DomNode> {
    //     match token {
    //         Token::Html {
    //             tokens,
    //             tag,
    //             attributes,
    //         } if tag == "model-viewer"=> {

    //         }
    //         _ => None
    //     }
    // }
}
