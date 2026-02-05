use log::LevelFilter;
use std::{path::PathBuf, thread};

use clap::Parser;
use lssg_lib::{
    Lssg,
    lmarkdown::parse_lmarkdown,
    renderer::{
        DefaultModule, ExternalModule, MediaModule, PostModule, Renderer, model_module::ModelModule,
    },
    sitetree::{Input, SiteTree},
};
use simple_logger::SimpleLogger;

mod preview;
use preview::start_preview_server;

mod watch;
use watch::watch_and_regenerate;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(
    author = "Lyr",
    version = env!("CARGO_PKG_VERSION"),
    about = "Lyr's Static Site Generator - Command Line Interface",
    long_about = "Generate static websites using the command line",
    disable_version_flag = true
)]
struct Args {
    /// Print version information
    #[arg(short = 'v', long = "version", action = clap::ArgAction::Version)]
    version: (),

    /// a reference to the first markdown input file
    /// this can either be a path (eg. ./my_post/index.md)
    /// or an url (eg. http://github.com/project/readme.md)
    #[clap(value_parser = Input::from_string)]
    input: Input,

    /// path to put the static files into, any needed parent folders are automatically created
    #[clap(required_unless_present_any = ["single_page", "ast"])]
    output: Option<PathBuf>,

    /// Print output of a single page
    #[clap(long, short, global = true)]
    single_page: bool,

    /// Print ast tokens of a single page
    #[clap(long, short, global = true)]
    ast: bool,

    /// "TRACE", "DEBUG", "INFO", "WARN", "ERROR"
    #[clap(long, short)]
    log: Option<LevelFilter>,

    /// Enable media optimization (images and videos)
    #[clap(long, short, default_value = "false")]
    no_media_optimization: bool,

    /// Watch for file changes and regenerate automatically
    #[clap(long, short = 'w')]
    watch: bool,

    /// Custom path to watch for file changes (defaults to input file's parent directory)
    #[clap(long)]
    watch_path: Option<PathBuf>,

    /// Start a preview server to view the generated site (Note: implicitly also runs --watch)
    #[clap(long, short = 'p')]
    preview: bool,

    /// Port for the preview server (default: 8000)
    #[clap(long, default_value = "8000")]
    port: u16,
}

fn main() {
    let args: Args = Args::parse();
    SimpleLogger::default()
        .with_level(args.log.unwrap_or(LevelFilter::Info))
        .init()
        .unwrap();

    let input = args.input;
    if args.ast {
        let read = input.readable().expect("failed to fetch input");
        let out = parse_lmarkdown(read).expect("failed to parse input");
        println!("{out:#?}");
        return;
    }

    let mut renderer = create_renderer(args.no_media_optimization);

    if args.single_page {
        let mut site_tree =
            SiteTree::from_input(input.clone()).expect("Failed to generate site tree");

        renderer.init(&mut site_tree);
        renderer.after_init(&site_tree);
        let html = renderer
            .render(&site_tree, site_tree.root())
            .expect("failed to render");
        println!("{html}");
        return;
    }

    // At this point we know output is Some(_) because of required_unless_present_any
    let output = args.output.unwrap();

    if args.preview {
        let port = args.port;
        {
            let output = output.clone();
            thread::spawn(move || {
                start_preview_server(output, port);
            });
        }

        watch_and_regenerate(
            input,
            output.clone(),
            args.watch_path,
            args.no_media_optimization,
            Some(port),
        );
        return;
    }

    if args.watch {
        watch_and_regenerate(
            input,
            output,
            args.watch_path,
            args.no_media_optimization,
            None,
        );
        return;
    }

    let mut lssg = Lssg::new(input, output, renderer);
    lssg.render().expect("Failed to render");
}

pub fn create_renderer(no_media_optimization: bool) -> Renderer {
    let mut renderer = Renderer::default();
    renderer.add_module(ModelModule::default());
    renderer.add_module(ExternalModule::default());
    renderer.add_module(PostModule::default());
    if !no_media_optimization {
        renderer.add_module(MediaModule::default());
    }
    renderer.add_module(DefaultModule::default());
    renderer
}
