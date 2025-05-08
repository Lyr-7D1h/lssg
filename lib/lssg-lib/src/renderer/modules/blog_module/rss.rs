use serde::Deserialize;
use serde_extensions::Overwrite;

#[derive(Overwrite, Debug, Deserialize)]
pub struct RssOptions {
    enabled: bool,
    footer: bool,
}
impl Default for RssOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            footer: true,
        }
    }
}
