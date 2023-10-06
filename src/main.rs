use log::LevelFilter;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use lssg_lib::{
    renderer::{BlogModule, DefaultModule, DefaultModuleOptions, HtmlRenderer},
    sitetree::SiteTree,
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
    #[command(subcommand)]
    command: Command,

    /// "TRACE", "DEBUG", "INFO", "WARN", "ERROR"
    #[clap(long, short, global = true)]
    log: Option<LevelFilter>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Render a single markdown page to html
    Render { input: PathBuf },
}

fn main() {
    let args: Args = Args::parse();
    SimpleLogger::new()
        .with_level(args.log.unwrap_or(LevelFilter::Info))
        .init()
        .unwrap();

    match args.command {
        Command::Render { input } => {
            let site_tree = SiteTree::from_index(input).expect("Failed to generate site tree");

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
    }
}
