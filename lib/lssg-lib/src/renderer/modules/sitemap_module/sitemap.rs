use crate::sitetree::SiteTree;

use super::SitemapOptions;

/// Represents the Sitemap Protocol XML structure.
pub(super) struct Sitemap {
    urls: Vec<SitemapUrl>,
}

struct SitemapUrl {
    loc: String,
}

impl Sitemap {
    /// Build a sitemap from all pages in the site tree.
    ///
    /// Walks every node in the site tree, collects page nodes,
    /// determines their absolute paths, and optionally prepends a host.
    pub fn build(site_tree: &SiteTree, options: &SitemapOptions) -> Self {
        let mut urls = Vec::new();

        for (site_id, _page) in site_tree.pages() {
            let path = site_tree.path(site_id);

            let loc = match &options.host {
                Some(host) => {
                    let host = host.trim_end_matches('/');
                    format!("{host}{path}")
                }
                None => path,
            };

            urls.push(SitemapUrl { loc });
        }

        Sitemap { urls }
    }
}

impl std::fmt::Display for Sitemap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#)?;
        f.write_str(r#"<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#)?;

        // Sort URLs for deterministic output
        let mut sorted = self.urls.iter().collect::<Vec<_>>();
        sorted.sort_by(|a, b| a.loc.cmp(&b.loc));

        for url in &sorted {
            f.write_str("\n  <url>")?;
            write!(f, "\n    <loc>{}</loc>", escape_xml(&url.loc))?;
            f.write_str("\n  </url>")?;
        }

        f.write_str("\n</urlset>")
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
