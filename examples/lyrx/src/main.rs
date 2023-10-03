use std::{env, path::PathBuf};

use lssg::{renderer::DefaultModuleOptions, Lssg, LssgOptions};
use simple_logger::SimpleLogger;

fn main() {
    let path = env::args().skip(1).collect::<String>();
    if path.len() == 0 {
        panic!("No content path given");
    }
    let path = PathBuf::try_from(path).expect("No content path given");
    SimpleLogger::new().env().init().unwrap();
    Lssg::new(LssgOptions {
        index: path.join("home.md"),
        output_directory: path.join("../build"),
        render_options: DefaultModuleOptions {
            global_stylesheet: Some(path.join("./main.css")),
            not_found_page: Some(path.join("./404.md")),
            overwrite_default_stylesheet: false,
            stylesheets: vec![
                path.join("./lib/fontawesome.css"),
                path.join("./lib/fa-solid.css"),
                path.join("./lib/fa-brands.css"),
            ],
            title: "LyrX".into(),
            language: "en".into(),
            keywords: vec![],
            favicon: Some(path.join("./favicon.ico")),
        },
    })
    .render()
    .unwrap()
}
