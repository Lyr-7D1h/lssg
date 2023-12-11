use std::collections::HashMap;
use std::path::Path;
use std::{fs::write, io::Read};

use log::info;
use regex::Regex;

use crate::{sitetree::Input, LssgError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StylesheetLink {
    Import(String),
    Url(String),
}

/// Stylesheet representation for resource discovering and condensing multiple stylesheets into one
#[derive(Debug, Clone)]
pub struct Stylesheet {
    content: String,
    /// map from raw matching string to path
    links: HashMap<String, StylesheetLink>,
}

fn links(content: &str) -> HashMap<String, StylesheetLink> {
    let mut resources = HashMap::new();
    let re = Regex::new(
        r#"@import ['"](.*)['"]|@import url\(['"]([^")]*)['"]\)|url\(['"]([^")]*)['"]\)"#,
    )
    .unwrap();
    for r in re.captures_iter(&content).into_iter() {
        if r[0].starts_with("@import") {
            let path = r
                .get(1)
                .unwrap_or_else(|| r.get(2).unwrap())
                .as_str()
                .to_string();

            // skip if external link
            if path.starts_with("http") {
                continue;
            }

            resources.insert(r[0].into(), StylesheetLink::Import(path));
        } else {
            resources.insert(r[0].into(), StylesheetLink::Url(r[3].to_string()));
        }
    }
    return resources;
}

impl Stylesheet {
    pub fn from_readable(mut readable: impl Read) -> Result<Stylesheet, LssgError> {
        let mut content = String::new();
        readable.read_to_string(&mut content)?;
        let links = links(&content);
        Ok(Stylesheet { content, links })
    }

    pub fn links(&self) -> Vec<&StylesheetLink> {
        return self.links.values().collect();
    }

    /// Append stylesheet and discover local referenced resources
    pub fn append(&mut self, _stylesheet: Stylesheet) -> Result<(), LssgError> {
        todo!()
    }

    /// Update a resource input path to a new one
    pub fn update_resource(&mut self, raw_path: &str, updated_path: &str) {
        self.content = self.content.replace(raw_path, updated_path);
    }

    pub fn write(&mut self, path: &Path) -> Result<(), LssgError> {
        info!("Writing stylesheet {path:?}",);
        write(path, &mut self.content)?;
        Ok(())
    }
}

impl TryFrom<&Input> for Stylesheet {
    type Error = LssgError;

    fn try_from(value: &Input) -> Result<Self, Self::Error> {
        Self::from_readable(value.readable()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stylesheet_links() {
        let resources = links(
            r#"@import "test.css"
@import 'test.css'
@import 'http:://test.com/test.css'
@import url("test.css")

@font-face {
  font-family: "Ubuntu Mono";
  src: url("lib/UbuntuMono-Regular.ttf") format("truetype");
}
* {
  background-image: url('test.jpg')
}"#,
        );

        assert_eq!(
            resources.get("@import \"test.css\"").unwrap(),
            &StylesheetLink::Import("test.css".to_owned())
        );
        assert_eq!(
            resources.get("@import \'test.css\'").unwrap(),
            &StylesheetLink::Import("test.css".to_owned())
        );
        assert_eq!(resources.get("@import 'http:://test.com/test.css'"), None);
        assert_eq!(
            resources.get("@import url(\"test.css\")").unwrap(),
            &StylesheetLink::Import("test.css".to_owned())
        );
        assert_eq!(
            resources
                .get(r#"url("lib/UbuntuMono-Regular.ttf")"#)
                .unwrap(),
            &StylesheetLink::Url("lib/UbuntuMono-Regular.ttf".to_owned())
        );
        assert_eq!(
            resources.get(r#"url('test.jpg')"#).unwrap(),
            &StylesheetLink::Url("test.jpg".to_owned())
        );
    }
}
