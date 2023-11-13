use log::LevelFilter;
use std::path::PathBuf;

use clap::Parser;
use lssg_lib::{
    renderer::{BlogModule, DefaultModule, HtmlRenderer},
    sitetree::SiteTree,
    Lssg, LssgOptions,
};
use simple_logger::SimpleLogger;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(
    author = "Lyr",
    version = "0.1.0",
    about = "Lyr's Static Site Generator - Command Line Interface",
    long_about = "Generate static websites using the command line"
)]
struct Args {
    input: PathBuf,

    output: PathBuf,

    /// Print output of a single page
    #[clap(long, short, global = true)]
    single_page: bool,

    /// "TRACE", "DEBUG", "INFO", "WARN", "ERROR"
    #[clap(long, short)]
    log: Option<LevelFilter>,
}

fn main() {
    let args: Args = Args::parse();
    SimpleLogger::new()
        .with_level(args.log.unwrap_or(LevelFilter::Info))
        .init()
        .unwrap();

    let input = args.input;

    if args.single_page {
        let site_tree = SiteTree::from_index(input.clone()).expect("Failed to generate site tree");

        let mut renderer = HtmlRenderer::new();
        renderer.add_module(BlogModule::new());
        renderer.add_module(DefaultModule::new());
        let html = renderer
            .render(&site_tree, site_tree.root())
            .expect("failed to render");
        println!("{html}");
        return;
    }

    Lssg::new(LssgOptions {
        output_directory: args.output,
        index: input,
    })
    .render()
    .unwrap()
}
