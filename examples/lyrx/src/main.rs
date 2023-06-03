use std::path::Path;

use lssg::Lssg;

fn main() {
    let mut lssg = Lssg::new();
    lssg.add_index(Path::new("./content/index.md"));
}
