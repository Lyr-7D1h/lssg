use log::LevelFilter;
use std::path::PathBuf;

use clap::Parser;
use lssg_lib::{
    renderer::{BlogModule, DefaultModule, Renderer},
    sitetree::{Input, SiteTree},
    Lssg,
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
    #[clap(value_parser = Input::from_string)]
    input: Input,

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
        let site_tree = SiteTree::from_input(input.clone()).expect("Failed to generate site tree");

        let mut renderer = Renderer::new();
        renderer.add_module(BlogModule::new());
        renderer.add_module(DefaultModule::new());
        let html = renderer
            .render(&site_tree, site_tree.root())
            .expect("failed to render");
        println!("{html}");
        return;
    }

    let mut lssg = Lssg::new(input, args.output);
    lssg.add_module(BlogModule::new());
    lssg.add_module(DefaultModule::new());
    lssg.render().unwrap()
}
