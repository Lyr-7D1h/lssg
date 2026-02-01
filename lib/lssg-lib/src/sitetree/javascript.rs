use crate::LssgError;
use crate::sitetree::Input;
use log::info;
use regex::Regex;
use std::collections::HashMap;
use std::fmt;
use std::fs::write;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JavascriptLink {
    Import(String),
    DynamicImport(String),
}

impl fmt::Display for JavascriptLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            JavascriptLink::Import(s) => s,
            JavascriptLink::DynamicImport(s) => s,
        };
        write!(f, "{s}")
    }
}

fn parse_links(content: &str) -> HashMap<String, JavascriptLink> {
    let mut resources = HashMap::new();
    // Match ES6 imports: import ... from '...'
    // Match dynamic imports: import('...')
    let re = Regex::new(r#"(?:import\s+.*?\s+from\s+['"]([^'"]*)['"]|import\(['"]([^'"]*)['"]\))"#)
        .unwrap();

    for r in re.captures_iter(content) {
        if let Some(static_import) = r.get(1) {
            let path = static_import.as_str().to_string();
            resources.insert(r[0].into(), JavascriptLink::Import(path));
        } else if let Some(dynamic_import) = r.get(2) {
            let path = dynamic_import.as_str().to_string();
            resources.insert(r[0].into(), JavascriptLink::DynamicImport(path));
        }
    }
    resources
}

/// Defines how a JavaScript file should be loaded in HTML
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScriptMode {
    /// `<script src="..." defer></script>` - Classic deferred (default)
    #[default]
    Defer,
    /// `<script src="..." async></script>` - Classic async
    Async,
    /// `<script src="..."></script>` - Classic blocking
    Blocking,
    /// `<script type="module" src="..."></script>` - ES6 module (auto-deferred)
    Module,
    /// `<script type="module" src="..."></script>` in body before content
    ModuleBlocking,
}

impl ScriptMode {
    /// Returns the HTML attributes for this script mode
    pub fn attributes(&self) -> &[(&'static str, &'static str)] {
        match self {
            ScriptMode::Defer => &[("defer", "")],
            ScriptMode::Async => &[("async", "")],
            ScriptMode::Blocking => &[],
            ScriptMode::Module => &[("type", "module")],
            ScriptMode::ModuleBlocking => &[("type", "module")],
        }
    }

    /// Returns true if this script should be placed in body (before content)
    pub fn in_body(&self) -> bool {
        matches!(self, ScriptMode::ModuleBlocking)
    }
}

/// JavaScript representation for resource discovering and HTML generation
#[derive(Debug, Clone)]
pub struct Javascript {
    input: Option<Input>,
    content: String,
    /// map from raw matching string to path
    links: HashMap<String, JavascriptLink>,
    mode: ScriptMode,
}

impl Javascript {
    pub fn from_readable(mut readable: impl Read) -> Result<Javascript, LssgError> {
        let mut content = String::new();
        readable.read_to_string(&mut content)?;
        let links = parse_links(&content);
        Ok(Javascript {
            input: None,
            content,
            links,
            mode: ScriptMode::default(),
        })
    }

    pub fn input(&self) -> Option<&Input> {
        self.input.as_ref()
    }

    pub fn mode(&self) -> ScriptMode {
        self.mode
    }

    pub fn with_mode(mut self, mode: ScriptMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn links(&self) -> Vec<&JavascriptLink> {
        self.links.values().collect()
    }

    /// Update a resource input path to a new one
    pub fn update_resource(&mut self, raw_path: &str, updated_path: &str) {
        self.content = self.content.replace(raw_path, updated_path);
    }

    pub fn write(&mut self, path: &Path) -> Result<(), LssgError> {
        info!("Writing javascript {path:?}",);
        write(path, &mut self.content)?;
        Ok(())
    }

    /// Returns the HTML attributes for the script tag
    pub fn attributes(&self) -> &[(&'static str, &'static str)] {
        self.mode.attributes()
    }
}

impl TryFrom<&Input> for Javascript {
    type Error = LssgError;

    fn try_from(value: &Input) -> Result<Self, Self::Error> {
        Self::from_readable(value.readable()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_javascript_links() {
        let resources = parse_links(
            r#"import { something } from './module.js';
import * as module from "./another.js";
import defaultExport from './default.js'
import 'https://cdn.example.com/lib.js'

const lazyLoad = () => import('./lazy.js');
const dynamic = import("./dynamic.js");

// Should be ignored - external
import { external } from 'https://example.com/external.js';
// Should be ignored - bare module
import { bare } from 'lodash';
"#,
        );

        assert_eq!(
            resources
                .get("import { something } from './module.js'")
                .unwrap(),
            &JavascriptLink::Import("./module.js".to_owned())
        );
        assert_eq!(
            resources
                .get("import * as module from \"./another.js\"")
                .unwrap(),
            &JavascriptLink::Import("./another.js".to_owned())
        );
        assert_eq!(
            resources
                .get("import defaultExport from './default.js'")
                .unwrap(),
            &JavascriptLink::Import("./default.js".to_owned())
        );
        assert_eq!(
            resources.get("import('./lazy.js')").unwrap(),
            &JavascriptLink::DynamicImport("./lazy.js".to_owned())
        );
        assert_eq!(
            resources.get("import(\"./dynamic.js\")").unwrap(),
            &JavascriptLink::DynamicImport("./dynamic.js".to_owned())
        );

        assert_eq!(
            resources
                .get("import { external } from 'https://example.com/external.js'")
                .unwrap(),
            &JavascriptLink::Import("https://example.com/external.js".to_owned())
        );
        assert_eq!(
            resources.get("import { bare } from 'lodash'").unwrap(),
            &JavascriptLink::Import("lodash".to_owned())
        );
    }
}
