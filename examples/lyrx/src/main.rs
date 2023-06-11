use std::path::{Path, PathBuf};

use lssg::{renderer::Rel, Link, Lssg, LssgOptions};
use simple_logger::SimpleLogger;

fn main() {
    SimpleLogger::new().init().unwrap();
    let lssg = Lssg::new(LssgOptions {
        output_directory: PathBuf::from("./build"),
        global_stylesheet: None,
        links: vec![Link {
            rel: Rel::Stylesheet,
            path: PathBuf::from("./content/lib/regular.css"),
        }],
        title: "LyrX".into(),
        language: "en".into(),
        keywords: vec![],
    });
    lssg.render(Path::new("./content/index.md")).unwrap();
}
