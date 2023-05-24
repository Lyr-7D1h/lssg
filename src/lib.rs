use std::path::Path;

pub enum Styling {}

pub struct Markdown {}

impl Markdown {}

pub struct Lssg {}

impl Lssg {
    pub fn new() -> Lssg {
        Lssg {}
    }

    pub fn add_route(route: String, document: &Path) {}
}
