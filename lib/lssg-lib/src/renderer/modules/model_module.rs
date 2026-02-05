use crate::{
    lmarkdown::Token,
    renderer::RendererModule,
    sitetree::{Javascript, Relation, Resource, SiteNode, SiteTree},
    tree::Node,
};

const MODEL_VIEWER_JS: &[u8] = include_bytes!("./model_module/model-viewer-v4.min.js");

const RESOURCE_KEYS: [&str; 4] = ["src", "poster", "skybox-image", "environment-image"];

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
            .flat_map(|(id, page)| {
                let mut models = Vec::new();
                for t in page.iter_all_tokens() {
                    match t {
                        Token::Html {
                            tokens,
                            tag,
                            attributes,
                        } if tag == "model-viewer" => {
                            models.push((id, attributes.clone()));
                        }
                        _ => {}
                    }
                }
                models
            })
            .collect();

        if !active_pages.is_empty() {
            let viewer_resource_id = site_tree.add(SiteNode::javascript(
                "model-viewer-v4.min.js",
                site_tree.root(),
                Javascript::from_readable(MODEL_VIEWER_JS)?
                    .with_mode(crate::sitetree::ScriptMode::Module),
            ));
            for (page_id, attributes) in active_pages {
                let Some(page_input) = site_tree.get_input(page_id).cloned() else {
                    log::warn!("No page input for {page_id}");
                    continue;
                };

                let add_resource = |href: &String, site_tree: &mut SiteTree| {
                    if let Ok(input) = page_input.join(href).inspect_err(|e| {
                        log::error!("Failed to join path '{href}' with page input: {e}")
                    }) && let Ok(name) = input
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
                                raw_path: href.clone(),
                            },
                        );
                    }
                };

                // https://modelviewer.dev/docs/index.html
                for key in RESOURCE_KEYS {
                    if let Some(src) = attributes.get(key) {
                        add_resource(src, site_tree);
                    }
                }

                site_tree.add_link(page_id, viewer_resource_id, Relation::External);
            }
        }

        Ok(())
    }

    fn render_token<'n>(
        &mut self,
        document: &mut virtual_dom::Document,
        ctx: &crate::renderer::RenderContext<'n>,
        parent: virtual_dom::DomNode,
        token: &Token,
        tr: &mut crate::renderer::TokenRenderer,
    ) -> Option<virtual_dom::DomNode> {
        match token {
            Token::Html {
                tokens,
                tag,
                attributes,
            } if tag == "model-viewer" => {
                let links = ctx.site_tree.links_from(ctx.site_id);

                let mut attributes = attributes.clone();
                for key in RESOURCE_KEYS {
                    if let Some(href) = attributes.get(key)
                    && let Some(link) = links.iter().find(
                        |l| matches!(&l.relation, Relation::Discovered { raw_path } if raw_path == href),
                    )
                {
                    let path = ctx.site_tree.path(link.to);
                    attributes.insert(key.to_string(), path);
                }
                }

                Some(tr.render_down(
                    self,
                    document,
                    ctx,
                    parent,
                    &[Token::Html {
                        tokens: tokens.clone(),
                        tag: tag.clone(),
                        attributes,
                    }],
                ))
            }
            _ => None,
        }
    }
}
