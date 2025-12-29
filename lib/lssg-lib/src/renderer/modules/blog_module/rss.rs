use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_extensions::Overwrite;

use crate::sitetree::{SiteId, SiteTree};

use super::collect_roots::{PostPage, RootPage};

#[derive(Overwrite, Clone, Debug, Deserialize)]
pub(super) struct RssOptions {
    pub enabled: bool,
    pub title: String,
    pub description: Option<String>,
    /// Path to the rss feed
    pub path: PathBuf,
    pub host: Option<String>,
    pub language: Option<String>,
    /// Will use the latest post
    pub last_build_date_enabled: Option<bool>,
}
impl Default for RssOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            title: "Feed".to_string(),
            description: Some("My feed".to_string()),
            path: PathBuf::from("feed.xml"),
            host: None,
            language: None,
            last_build_date_enabled: Some(true),
        }
    }
}

pub(super) struct RssItem {
    pub title: String,
    pub description: Option<String>,
    pub link: String,
    pub guid: String,
    pub pub_date: DateTime<Utc>,
}
pub(super) struct RssFeed {
    title: String,
    link: String,
    description: Option<String>,
    last_build_date: Option<DateTime<Utc>>,
    items: Vec<RssItem>,
}
impl RssFeed {
    pub fn new(title: String, link: String, description: Option<String>) -> RssFeed {
        RssFeed {
            title,
            link,
            description,
            last_build_date: None,
            items: vec![],
        }
    }

    pub fn add_item(&mut self, item: RssItem) {
        self.items.push(item)
    }

    /// Build RSS feed from root page and its posts
    pub fn from_root(root_id: SiteId, root: &RootPage, site_tree: &SiteTree) -> RssFeed {
        let rss_opts = &root.options.rss;

        // Determine the base link for the feed
        let base_link = match rss_opts.host.clone() {
            Some(host) => host,
            None => {
                log::error!("blog.rss.host is not defined on {root_id}");
                "".into()
            }
        };
        let feed_link = format!("{}{}", base_link, site_tree.path(root_id));

        let mut feed = RssFeed::new(
            rss_opts.title.clone(),
            feed_link,
            rss_opts.description.clone(),
        );

        // Collect and sort posts by date (newest first)
        let mut posts: Vec<(&SiteId, &PostPage)> = root.posts.iter().collect();
        posts.sort_by(|a, b| {
            let date_a = a.1.dates.created_on.as_ref();
            let date_b = b.1.dates.created_on.as_ref();
            date_b.cmp(&date_a) // Reverse order for newest first
        });

        // Set last build date to the most recent post's date if enabled
        if rss_opts.last_build_date_enabled.unwrap_or(true) {
            feed.last_build_date = posts
                .first()
                .and_then(|(_, post)| post.dates.created_on.clone());
        }

        // Add RSS items for each post
        for (post_id, post) in posts {
            // Skip posts that shouldn't be rendered
            if !post.post_options.render {
                continue;
            }

            // Use created_on date, skip posts without a date
            let Some(pub_date) = &post.dates.created_on else {
                continue;
            };

            let post_path = site_tree.path(*post_id);
            let post_link = format!("{}{}", base_link, post_path);

            // Use title from contents, fall back to path
            let title = post
                .contents
                .title
                .clone()
                .unwrap_or_else(|| post_path.clone());

            // Use description from post options summary
            let description = post.post_options.summary.clone();

            feed.add_item(RssItem {
                title,
                description,
                link: post_link.clone(),
                guid: post_link,
                pub_date: pub_date.clone(),
            });
        }

        feed
    }
}

impl ToString for RssFeed {
    fn to_string(&self) -> String {
        let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        xml.push_str("\n<rss version=\"2.0\">");
        xml.push_str("\n  <channel>");
        xml.push_str(&format!("\n    <title>{}</title>", escape_xml(&self.title)));
        xml.push_str(&format!("\n    <link>{}</link>", escape_xml(&self.link)));
        if let Some(description) = &self.description {
            xml.push_str(&format!(
                "\n    <description>{}</description>",
                escape_xml(description)
            ));
        }

        // Add lastBuildDate if available
        if let Some(last_build_date) = &self.last_build_date {
            xml.push_str(&format!(
                "\n    <lastBuildDate>{}</lastBuildDate>",
                last_build_date.to_rfc2822()
            ));
        }

        for item in &self.items {
            xml.push_str("\n    <item>");
            xml.push_str(&format!(
                "\n      <title>{}</title>",
                escape_xml(&item.title)
            ));
            xml.push_str(&format!("\n      <link>{}</link>", escape_xml(&item.link)));
            if let Some(description) = &item.description {
                xml.push_str(&format!(
                    "\n      <description>{}</description>",
                    escape_xml(description)
                ));
            }
            xml.push_str(&format!("\n      <guid>{}</guid>", escape_xml(&item.guid)));
            xml.push_str(&format!(
                "\n      <pubDate>{}</pubDate>",
                item.pub_date.to_rfc2822()
            ));
            xml.push_str("\n    </item>");
        }

        xml.push_str("\n  </channel>");
        xml.push_str("\n</rss>");
        xml
    }
}

fn escape_xml(s: &str) -> String {
    s.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&apos;")
}
