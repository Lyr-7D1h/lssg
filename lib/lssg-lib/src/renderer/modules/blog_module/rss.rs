use serde::Deserialize;
use serde_extensions::Overwrite;

#[derive(Overwrite, Clone, Debug, Deserialize)]
pub struct RssOptions {
    /// Rss is enabled for this page and all of its children
    pub enabled: bool,
    /// This footer is applied to this page and all of its children
    pub footer: bool,
}
impl Default for RssOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            footer: true,
        }
    }
}
