use std::{
    io,
    path::{Path, PathBuf},
};

use crate::LssgError;

pub fn canonicalize_nonexistent_path(path: &Path) -> PathBuf {
    let mut canonicalized_path = vec![];
    let path = path.to_string_lossy();
    let parts = path.split("/");
    for p in parts {
        if p == ".." {
            canonicalized_path.pop();
        } else {
            canonicalized_path.push(p);
        }
    }
    PathBuf::from(canonicalized_path.join("/"))
}

pub fn filestem_from_path(path: &Path) -> Result<String, LssgError> {
    Ok(path
        .file_stem()
        .ok_or(LssgError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{path:?} does not have a filename"),
        )))?
        .to_str()
        .ok_or(LssgError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{path:?} is non unicode"),
        )))?
        .to_owned())
}

pub fn filename_from_path(path: &Path) -> Result<String, LssgError> {
    Ok(path
        .file_name()
        .ok_or(LssgError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{path:?} does not have a filename"),
        )))?
        .to_str()
        .ok_or(LssgError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{path:?} is non unicode"),
        )))?
        .to_owned())
}
