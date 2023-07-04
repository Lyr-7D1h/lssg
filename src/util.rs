use std::{path::Path, io};

use crate::LssgError;

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
