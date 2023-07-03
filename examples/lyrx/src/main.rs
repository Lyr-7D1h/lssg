use std::{env, path::PathBuf};

use lssg::{renderer::Rel, Link, Lssg, LssgOptions};
use simple_logger::SimpleLogger;

fn main() {
    let path: PathBuf = env::args().skip(1).collect();
    SimpleLogger::new().env().init().unwrap();
    Lssg::new(LssgOptions {
        index: path.join("home.md"),
        output_directory: path.join("../build"),
        global_stylesheet: Some(path.join("./main.css")),
        overwrite_default_stylesheet: false,
        links: vec![
            Link {
                rel: Rel::Stylesheet,
                path: path.join("./lib/fontawesome.css"),
            },
            Link {
                rel: Rel::Stylesheet,
                path: path.join("./lib/fa-solid.css"),
            },
            Link {
                rel: Rel::Stylesheet,
                path: path.join("./lib/fa-brands.css"),
            },
        ],
        title: "LyrX".into(),
        language: "en".into(),
        keywords: vec![],
        favicon: Some(path.join("./favicon.ico")),
    })
    .render()
    .unwrap();
}
