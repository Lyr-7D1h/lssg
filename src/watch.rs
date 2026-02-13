use std::{path::PathBuf, sync::mpsc::channel, time::Duration};

use lssg_lib::{Lssg, sitetree::Input};
use notify_debouncer_full::{DebouncedEvent, new_debouncer, notify::RecursiveMode};

use crate::create_renderer;

pub fn watch_and_regenerate(
    input: Input,
    output: PathBuf,
    watch_path: Option<PathBuf>,
    no_media_optimization: bool,
    port: Option<u16>,
) {
    // Determine the watch path based on input type
    let watch_path = match watch_path {
        Some(path) => path,
        None => match &input {
            Input::External { .. } => {
                log::error!("Watch mode is not supported for URL inputs");
                return;
            }
            Input::Local { path } => {
                // Watch the parent directory of the input file
                path.parent().unwrap_or(path).to_path_buf()
            }
        },
    };

    // Initial render
    let renderer = create_renderer(no_media_optimization);
    let mut lssg = Lssg::new(
        input.clone(),
        output.clone(),
        renderer,
        reqwest::blocking::Client::new(),
    );
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

    println!("\n\n");
    if let Some(port) = port {
        log::info!("Starting preview server at http://localhost:{}", port);
        log::info!("Serving files from {:?}", output);
    }
    log::info!("Watching {:?} for changes", watch_path);
    log::info!("Press Ctrl+C to stop.");

    // Wait for file changes
    for _ in rx {
        log::info!("Changes detected, regenerating...");
        let renderer = create_renderer(no_media_optimization);
        let mut lssg = Lssg::new(
            input.clone(),
            output.clone(),
            renderer,
            reqwest::blocking::Client::new(),
        );
        match lssg.render() {
            Ok(_) => log::info!("Regeneration completed successfully"),
            Err(e) => log::error!("Regeneration failed: {}", e),
        }
    }
}
