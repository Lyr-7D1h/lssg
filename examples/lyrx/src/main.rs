use std::path::{Path, PathBuf};

use lssg::Lssg;

fn main() {
    let lssg = Lssg::new(PathBuf::from("./build"));
    lssg.render(Path::new("./content/index.md")).unwrap();
}
