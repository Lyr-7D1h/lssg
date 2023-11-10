use std::path::{Path, PathBuf};

use crate::LssgError;

pub trait PathExtension {
    fn canonicalize_nonexistent_path(&self) -> PathBuf;
    fn filestem_from_path(&self) -> Result<String, LssgError>;
    fn filename_from_path(&self) -> Result<String, LssgError>;
}

impl PathExtension for Path {
    fn canonicalize_nonexistent_path(&self) -> PathBuf {
        let mut canonicalized_path = vec![];
        let path = self.to_string_lossy();
        let parts = path.split("/");
        for p in parts {
            if p == "." {
                continue;
            }
            if p == ".." && canonicalized_path.len() > 0 {
                canonicalized_path.pop();
                continue;
            }
            canonicalized_path.push(p);
        }
        PathBuf::from(canonicalized_path.join("/"))
    }

    fn filestem_from_path(&self) -> Result<String, LssgError> {
        Ok(self
            .file_stem()
            .ok_or(LssgError::io(&format!("{self:?} does not have a filename")))?
            .to_str()
            .ok_or(LssgError::io(&format!("{self:?} is non unicode")))?
            .to_owned())
    }

    fn filename_from_path(&self) -> Result<String, LssgError> {
        Ok(self
            .file_name()
            .ok_or(LssgError::io(&format!("{self:?} does not have a filename")))?
            .to_str()
            .ok_or(LssgError::io(&format!("{self:?} is non unicode")))?
            .to_owned())
    }
}
