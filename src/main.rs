use std::{env, path::PathBuf};

use lssg::{
    renderer::{BlogModule, DefaultModule, DefaultModuleOptions, HtmlRenderer},
    sitetree::SiteTree,
    Lssg, LssgOptions,
};
use simple_logger::SimpleLogger;

fn main() {
    SimpleLogger::new().env().init().unwrap();
    let path = env::args().skip(1).collect::<String>();
    if path.len() == 0 {
        panic!("No content path given");
    }
    let path = PathBuf::try_from(path).expect("No content path given");

    let site_tree = SiteTree::from_index(path).expect("Failed to generate site tree");

    let mut renderer = HtmlRenderer::new();
    renderer.add_module(BlogModule::new());
    renderer.add_module(DefaultModule::new(DefaultModuleOptions {
        global_stylesheet: None,
        not_found_page: None,
        overwrite_default_stylesheet: false,
        stylesheets: vec![],
        title: "".into(),
        language: "en".into(),
        keywords: vec![],
        favicon: None,
    }));
    let html = renderer
        .render(&site_tree, site_tree.root())
        .expect("failed to render");
    println!("{html}");
}
