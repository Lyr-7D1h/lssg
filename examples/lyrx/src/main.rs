use std::path::{Path, PathBuf};

use lssg::{Lssg, LssgOptions};
use simple_logger::SimpleLogger;

fn main() {
    SimpleLogger::new().init().unwrap();
    let lssg = Lssg::new(LssgOptions {
        output_directory: PathBuf::from("./build"),
        global_stylesheet: None,
        title: "LyrX".into(),
        language: "en".into(),
        keywords: vec![],
    });
    lssg.render(Path::new("./content/index.md")).unwrap();
}
