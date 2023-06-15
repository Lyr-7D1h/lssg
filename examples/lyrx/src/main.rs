use std::path::PathBuf;

use lssg::{renderer::Rel, Link, Lssg, LssgOptions};
use simple_logger::SimpleLogger;

fn main() {
    SimpleLogger::new().init().unwrap();
    Lssg::new(LssgOptions {
        index: PathBuf::from("./content/index.md"),
        output_directory: PathBuf::from("./build"),
        global_stylesheet: None,
        links: vec![Link {
            rel: Rel::Stylesheet,
            path: PathBuf::from("./content/lib/regular.css"),
        }],
        title: "LyrX".into(),
        language: "en".into(),
        keywords: vec![],
    })
    .render()
    .unwrap();
}
