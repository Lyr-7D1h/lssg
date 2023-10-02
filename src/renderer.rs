mod blog_module;
pub use blog_module::BlogModule;
mod default_module;
pub use default_module::{DefaultModule, DefaultModuleOptions};

use std::collections::{HashMap, VecDeque};

use log::{error, warn};

use crate::{
    domtree::DomTree,
    parser::lexer::Token,
    sitetree::{SiteNodeKind, SiteTree},
    LssgError,
};

pub trait RendererModule {
    /// Return a static id
    fn id(&self) -> &'static str;

    /// This gets run just after site_tree has been created
    fn site_init(&mut self, site_tree: &mut SiteTree) -> Result<(), LssgError>;

    // TODO modify site_tree on init too
    /// Modify DomTree on init
    fn init<'n>(&mut self, tree: &mut DomTree, context: &RendererModuleContext<'n>);

    /// Render a token before default token renderer returns true if it parsed this token otherwise false
    fn body<'n>(
        &mut self,
        tree: &mut DomTree,
        context: &RendererModuleContext<'n>,
        render_queue: &mut RenderQueue,
        parent_dom_id: usize,
        token: &Token,
    ) -> bool;
}

pub struct RenderQueue {
    tokens: VecDeque<(Token, usize)>,
}

impl RenderQueue {
    pub fn pop_front(&mut self) -> Option<(Token, usize)> {
        self.tokens.pop_front()
    }

    pub fn from_tokens(tokens: Vec<Token>, parent_id: usize) -> Self {
        Self {
            tokens: VecDeque::from(
                tokens
                    .into_iter()
                    .map(|t| (t, parent_id))
                    .collect::<Vec<(Token, usize)>>(),
            ),
        }
    }

    pub fn push_tokens_front(&mut self, tokens: &Vec<Token>, parent_id: usize) {
        self.tokens
            .extend(tokens.clone().into_iter().map(|t| (t, parent_id)).rev());
    }
}

pub struct RendererModuleContext<'n> {
    pub site_tree: &'n SiteTree,
    pub site_id: usize,
    pub tokens: &'n Vec<Token>,
    pub metadata: HashMap<String, String>,
}

/// HtmlRenderer is responsible for the process of converting the site tree into the final HTML output.
/// It does this by managing a queue of tokens to be rendered and delegating the rendering process to different modules.
pub struct HtmlRenderer {
    modules: Vec<Box<dyn RendererModule>>,
}

impl HtmlRenderer {
    pub fn new() -> HtmlRenderer {
        HtmlRenderer { modules: vec![] }
    }

    pub fn add_module(&mut self, module: impl RendererModule + 'static) {
        self.modules.push(Box::new(module));
    }

    /// Will run site_init on all modules, will remove modules if it fails
    pub fn site_init(&mut self, site_tree: &mut SiteTree) {
        let failed: Vec<usize> = (&mut self.modules)
            .into_iter()
            .enumerate()
            .filter_map(|(i, module)| match module.site_init(site_tree) {
                Ok(_) => None,
                Err(e) => {
                    error!("Failed to do site_init on {}: {e}", module.id());
                    Some(i)
                }
            })
            .collect();
        for i in failed.into_iter().rev() {
            self.modules.remove(i);
        }
    }

    /// Transform site id into a html page
    pub fn render(&mut self, site_tree: &SiteTree, site_id: usize) -> Result<String, LssgError> {
        // get the site node
        let site_node = site_tree.get(site_id)?;
        let (tokens, ..) = match &site_node.kind {
            SiteNodeKind::Page { tokens, input, .. } => (tokens, input),
            _ => return Err(LssgError::render("Invalid node type given")),
        };

        let mut tree = DomTree::new();

        // first comment of a page is seen as metadata
        let metadata = if let Some(Token::Comment { text: _, map }) = tokens.first() {
            map.into_iter()
                .map(|(k, v)| (k.trim().to_owned(), v.trim().to_owned()))
                .collect()
        } else {
            HashMap::new()
        };

        let context = RendererModuleContext {
            site_tree: site_tree,
            site_id,
            tokens,
            metadata,
        };

        // initialize modules
        for module in &mut self.modules {
            module.init(&mut tree, &context);
        }

        // create body
        let body = tree.get_elements_by_tag_name("body")[0];
        let mut queue = RenderQueue::from_tokens(context.tokens.clone(), body);
        'l: while let Some((token, parent_id)) = queue.pop_front() {
            for module in &mut self.modules {
                if module.body(&mut tree, &context, &mut queue, parent_id, &token) {
                    continue 'l;
                }
            }
            warn!("{token:?} not renderered");
        }

        println!("{tree}");
        return Ok(tree.to_html_string());
    }
}
