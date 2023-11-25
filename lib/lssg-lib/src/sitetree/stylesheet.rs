use regex::Regex;

use crate::{sitetree::Input, LssgError};

/// Stylesheet representation for resource discovering and condensing multiple stylesheets into one
#[derive(Debug, Clone)]
pub struct Stylesheet {
    content: String,
}

// fn get_resources_from_content(
//     content: &String,
//     input: &Input,
// ) -> Result<HashMap<Input, HashSet<String>>, LssgError> {
//     let mut resources: HashMap<Input, HashSet<String>> = HashMap::new();
//
//     // TODO add `@import` support
//     let re = Regex::new(r#"url\("?(\.[^)"]*)"?\)"#)?;
//     for r in re.captures_iter(&content).into_iter() {
//         let input = Input::from_string(&r[1], Some(input))?;
//
//         let raw = r[0].to_owned();
//         if let Some(set) = resources.get_mut(&input) {
//             set.insert(raw);
//         } else {
//             let mut set = HashSet::new();
//             set.insert(raw);
//             resources.insert(input, set);
//         }
//     }
//     Ok(resources)
// }

impl Stylesheet {
    /// create new empty stylesheet
    pub fn from_input(input: &Input) -> Result<Stylesheet, LssgError> {
        let mut content = String::new();
        let mut readable = input.readable()?;
        readable.read_to_string(&mut content)?;
        Ok(Stylesheet { content })
    }

    pub fn resources(&self) -> Vec<String> {
        let mut resources = vec![];
        // TODO add `@import` support
        let re = Regex::new(r#"url\("?(\.[^)"]*)"?\)"#).unwrap();
        for r in re.captures_iter(&self.content).into_iter() {
            resources.push(r[1].to_string());
        }
        return resources;
    }

    /// Append stylesheet and discover local referenced resources
    pub fn append(&mut self, _stylesheet: Stylesheet) -> Result<(), LssgError> {
        todo!()
    }

    /// Update a resource input path to a new one
    pub fn update_resource(&mut self, raw_path: &str, updated_path: &str) {
        self.content = self.content.replace(raw_path, updated_path);
    }

    pub fn to_string(self) -> String {
        self.content
    }
}
