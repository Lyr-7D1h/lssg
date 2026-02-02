use log::LevelFilter;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time::Duration;

use clap::Parser;
use lssg_lib::{
    Lssg,
    lmarkdown::parse_lmarkdown,
    renderer::{
        BlogModule, DefaultModule, ExternalModule, MediaModule, Renderer, model_module::ModelModule,
    },
    sitetree::{Input, SiteTree},
};
use notify_debouncer_full::{DebouncedEvent, new_debouncer, notify::RecursiveMode};
use simple_logger::SimpleLogger;

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
    /// this can either be a path (eg. ./my_blog/index.md)
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

    if args.watch {
        watch_and_regenerate(input, output, args.no_media_optimization);
        return;
    }

    let mut lssg = Lssg::new(input, output, renderer);
    lssg.render().expect("Failed to render");
}

fn create_renderer(no_media_optimization: bool) -> Renderer {
    let mut renderer = Renderer::default();
    renderer.add_module(ModelModule::default());
    renderer.add_module(ExternalModule::default());
    renderer.add_module(BlogModule::default());
    if !no_media_optimization {
        renderer.add_module(MediaModule::default());
    }
    renderer.add_module(DefaultModule::default());
    renderer
}

fn watch_and_regenerate(input: Input, output: PathBuf, no_media_optimization: bool) {
    // Determine the watch path based on input type
    let watch_path = match &input {
        Input::External { .. } => {
            log::error!("Watch mode is not supported for URL inputs");
            return;
        }
        Input::Local { path } => {
            // Watch the parent directory of the input file
            path.parent().unwrap_or(path).to_path_buf()
        }
    };

    log::info!("Watching {:?} for changes...", watch_path);

    // Initial render
    let renderer = create_renderer(no_media_optimization);
    let mut lssg = Lssg::new(input.clone(), output.clone(), renderer);
    match lssg.render() {
        Ok(_) => log::info!("Initial render completed successfully"),
        Err(e) => log::error!("Initial render failed: {}", e),
    }

    // Set up file watcher
    let (tx, rx) = channel();
    let mut debouncer = new_debouncer(
        Duration::from_millis(500),
        None,
        move |result: Result<Vec<DebouncedEvent>, _>| {
            if let Ok(events) = result {
                // Filter out Access events (file reads) - only respond to actual modifications
                let has_modifications = events.iter().any(|event| {
                    use notify_debouncer_full::notify::EventKind;
                    !matches!(event.event.kind, EventKind::Access(_))
                });

                if has_modifications {
                    for event in &events {
                        if !matches!(
                            event.event.kind,
                            notify_debouncer_full::notify::EventKind::Access(_)
                        ) {
                            log::debug!("File change detected: {:?}", event);
                        }
                    }
                    tx.send(()).unwrap();
                }
            }
        },
    )
    .expect("Failed to create file watcher");

    debouncer
        .watch(&watch_path, RecursiveMode::Recursive)
        .expect("Failed to watch directory");

    log::info!("Watching for changes. Press Ctrl+C to stop.");

    // Wait for file changes
    for _ in rx {
        log::info!("Changes detected, regenerating...");
        let renderer = create_renderer(no_media_optimization);
        let mut lssg = Lssg::new(input.clone(), output.clone(), renderer);
        match lssg.render() {
            Ok(_) => log::info!("Regeneration completed successfully"),
            Err(e) => log::error!("Regeneration failed: {}", e),
        }
    }
}
