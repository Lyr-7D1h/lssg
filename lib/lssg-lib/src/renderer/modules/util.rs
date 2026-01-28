use log::warn;
use serde::de::IntoDeserializer;
use serde::{Deserialize, Serialize};
use serde_extensions::Overwrite;
use serde_value::Value;

use crate::{
    lmarkdown::Token,
    renderer::RenderContext,
    sitetree::{Page, Relation},
};

pub fn tokens_to_text(tokens: &Vec<Token>) -> String {
    let mut result = String::new();
    for t in tokens {
        if let Some(text) = t.to_text() {
            result.push_str(&text)
        }
    }
    result
}

/// Translate href to page path
pub fn process_href(href: &String, context: &RenderContext) -> String {
    if Page::is_href_to_page(href) {
        let to_id = context
            .site_tree
            .links_from(context.site_id)
            .into_iter()
            .find_map(|l| {
                if let Relation::Discovered { raw_path: path } = &l.relation
                    && path == href
                {
                    return Some(l.to);
                }
                None
            });

        if let Some(to_id) = to_id {
            context.site_tree.path(to_id)
        } else {
            warn!("Could not find node where {href:?} points to");
            href.to_owned()
        }
    } else {
        href.to_owned()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OneOrManyOption<T> {
    Many(Vec<T>),
    One(T),
}

impl<T> Overwrite for OneOrManyOption<T>
where
    T: for<'de> serde::Deserialize<'de>,
    T: serde::Serialize,
    T: Overwrite,
    T: Default,
{
    fn overwrite<'de, D>(&mut self, d: D) -> Result<(), D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let new: OneOrManyOption<Value> = Deserialize::deserialize(d)?;
        match (&mut *self, new) {
            (OneOrManyOption::One(t_old), OneOrManyOption::One(t_new)) => {
                t_old
                    .overwrite(t_new.into_deserializer())
                    .map_err(serde::de::Error::custom)?;
            }
            (_, new) => {
                *self = match new {
                    OneOrManyOption::Many(items) => OneOrManyOption::Many(
                        items
                            .into_iter()
                            .map(|item| {
                                let mut t = T::default();
                                t.overwrite(item.into_deserializer())
                                    .map_err(serde::de::Error::custom)?;
                                Ok(t)
                            })
                            .collect::<Result<Vec<_>, _>>()?,
                    ),
                    OneOrManyOption::One(item) => OneOrManyOption::One({
                        let mut t = T::default();
                        t.overwrite(item).map_err(serde::de::Error::custom)?;
                        t
                    }),
                };
            }
        }
        Ok(())
    }
}
impl<T> OneOrManyOption<T> {
    /// Convert to a vector, consuming self
    pub fn into_vec(self) -> Vec<T> {
        match self {
            OneOrManyOption::Many(vec) => vec,
            OneOrManyOption::One(item) => vec![item],
        }
    }

    /// Get as a slice
    pub fn as_slice(&self) -> &[T] {
        match self {
            OneOrManyOption::Many(vec) => vec.as_slice(),
            OneOrManyOption::One(item) => std::slice::from_ref(item),
        }
    }

    /// Get the number of items
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            OneOrManyOption::Many(vec) => vec.len(),
            OneOrManyOption::One(_) => 1,
        }
    }
    /// Check if empty
    #[must_use]
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        match self {
            OneOrManyOption::Many(vec) => vec.is_empty(),
            OneOrManyOption::One(_) => false,
        }
    }
}
