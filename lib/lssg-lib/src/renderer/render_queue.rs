use std::collections::VecDeque;

use log::warn;

use super::{RendererModule, RendererModuleContext};
use crate::{html::DomTree, lmarkdown::Token};

type RenderFunction =
    fn(&mut DomTree, &RendererModuleContext, &mut RenderQueue, usize, &Token) -> bool;

pub struct RenderQueue<'a> {
    tree: &'a mut DomTree,
    modules: &'a mut Vec<Box<dyn RendererModule>>,
    context: &'a RendererModuleContext<'a>,
}

impl<'a> RenderQueue<'a> {
    pub fn new(
        tree: &'a mut DomTree,
        modules: &'a mut Vec<Box<dyn RendererModule>>,
        context: &'a RendererModuleContext,
    ) -> RenderQueue<'a> {
        RenderQueue {
            tree,
            modules,
            context,
        }
    }

    pub fn render(&mut self, tokens: &Vec<Token>, parent_id: usize) {
        let render = |tokens: &Vec<Token>, parent_id: usize| {
            let mut queue = VecDeque::from(
                tokens
                    .into_iter()
                    .map(|t| (t, parent_id))
                    .collect::<Vec<(&Token, usize)>>(),
            );
            'l: while let Some((token, parent_id)) = queue.pop_front() {
                for module in self.modules.iter_mut() {
                    if module.render_body(&mut self.tree, &self.context, render, parent_id, &token)
                    {
                        continue 'l;
                    }
                    warn!("{token:?} not renderered");
                }
            }
        };

        'l: for token in  tokens.iter() {
            for module in self.modules.iter_mut() {
                if module.render_body(&mut self.tree, &self.context, render, parent_id, &token) {
                    continue 'l;
                }
                warn!("{token:?} not renderered");
            }
        }
    }
}
