use log::LevelFilter;
use std::{fs, path::PathBuf};

use clap::Parser;
use lssg_lib::{
    lmarkdown::parse_lmarkdown,
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
    /// a reference to the first markdown input file
    /// this can either be a path (eg. ./my_blog/index.md)
    /// or an url (eg. http://github.com/project/readme.md)
    #[clap(value_parser = Input::from_string)]
    input: Input,

    /// path to put the static files into, any needed parent folders are automatically created
    output: PathBuf,

    /// Print output of a single page
    #[clap(long, short, global = true)]
    single_page: bool,

    /// Print ast tokens of a single page
    #[clap(long, short, global = true)]
    ast: bool,

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
        let mut site_tree =
            SiteTree::from_input(input.clone()).expect("Failed to generate site tree");

        let mut renderer = Renderer::new();
        renderer.add_module(BlogModule::new());
        renderer.add_module(DefaultModule::new());
        renderer.init(&mut site_tree);
        renderer.after_init(&site_tree);
        let html = renderer
            .render(&site_tree, site_tree.root())
            .expect("failed to render");
        println!("{html}");
        fs::write(args.output, html).expect("failed to write to file");
        return;
    }

    if args.ast {
        let read = input.readable().expect("failed to fetch input");
        let out = parse_lmarkdown(read).expect("failed to parse input");
        println!("{out:#?}");
        return;
    }

    let mut lssg = Lssg::new(input, args.output);
    lssg.add_module(BlogModule::new());
    lssg.add_module(DefaultModule::new());
    lssg.render().unwrap()
}
