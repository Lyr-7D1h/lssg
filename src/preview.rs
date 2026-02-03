use std::{fs, io::Read, path::PathBuf};

use tiny_http::{Header, Response, Server};

pub fn start_preview_server(output: PathBuf, port: u16) {
    let addr = format!("0.0.0.0:{}", port);

    let server = Server::http(&addr).expect("Failed to create server");

    for request in server.incoming_requests() {
        let url_path = request.url().to_string();
        let mut path = output.clone();

        // Handle root path
        let file_path = if url_path == "/" {
            "index.html"
        } else {
            url_path.trim_start_matches('/')
        };

        path.push(file_path);

        // If path is a directory, try to serve index.html
        if path.is_dir() {
            path.push("index.html");
        }

        log::debug!("Request: {} -> {:?}", url_path, path);

        if path.exists() && path.is_file() {
            match fs::File::open(&path) {
                Ok(mut file) => {
                    let mut contents = Vec::new();
                    if file.read_to_end(&mut contents).is_ok() {
                        // Determine content type based on file extension
                        let content_type = match path.extension().and_then(|s| s.to_str()) {
                            Some("html") => "text/html; charset=utf-8",
                            Some("css") => "text/css; charset=utf-8",
                            Some("js") => "application/javascript; charset=utf-8",
                            Some("json") => "application/json; charset=utf-8",
                            Some("png") => "image/png",
                            Some("jpg") | Some("jpeg") => "image/jpeg",
                            Some("gif") => "image/gif",
                            Some("svg") => "image/svg+xml",
                            Some("webp") => "image/webp",
                            Some("ico") => "image/x-icon",
                            Some("woff") => "font/woff",
                            Some("woff2") => "font/woff2",
                            Some("ttf") => "font/ttf",
                            Some("xml") => "application/xml; charset=utf-8",
                            _ => "application/octet-stream",
                        };

                        let header =
                            Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes())
                                .expect("Failed to create header");
                        let response = Response::from_data(contents).with_header(header);
                        if request.respond(response).is_ok() {
                            log::info!("200 {}", url_path);
                        }
                    } else {
                        let response =
                            Response::from_string("Internal Server Error").with_status_code(500);
                        let _ = request.respond(response);
                        log::error!("500 {} - Failed to read file", url_path);
                    }
                }
                Err(_) => {
                    let response =
                        Response::from_string("Internal Server Error").with_status_code(500);
                    let _ = request.respond(response);
                    log::error!("500 {} - Failed to open file", url_path);
                }
            }
        } else {
            // Try to serve 404.html if it exists
            let mut not_found_path = output.clone();
            not_found_path.push("404/index.html");

            let response = if not_found_path.exists() {
                match fs::read_to_string(&not_found_path) {
                    Ok(content) => {
                        let header =
                            Header::from_bytes(&b"Content-Type"[..], b"text/html; charset=utf-8")
                                .expect("Failed to create header");
                        Response::from_string(content)
                            .with_status_code(404)
                            .with_header(header)
                    }
                    Err(_) => Response::from_string("404 Not Found").with_status_code(404),
                }
            } else {
                Response::from_string("404 Not Found").with_status_code(404)
            };

            let _ = request.respond(response);
            log::warn!("404 {}", url_path);
        }
    }
}
