use std::path::PathBuf;

use serde::Deserialize;
use serde_extensions::Overwrite;

#[derive(Overwrite, Clone, Debug, Deserialize)]
pub struct RssOptions {
    /// Rss is enabled for this page and all of its children
    pub enabled: bool,
    pub title: String,
    pub description: String,
    /// This footer is applied to this page and all of its children
    pub footer: bool,
    /// Path to the rss feed
    pub path: PathBuf,
    pub host: Option<String>,
}
impl Default for RssOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            title: "Feed".to_string(),
            description: "My feed".to_string(),
            footer: true,
            path: PathBuf::from("feed.xml"),
            host: None,
        }
    }
}
