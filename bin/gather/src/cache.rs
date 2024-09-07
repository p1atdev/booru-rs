use std::path::{Path, PathBuf};

struct SearchCache {
    path: PathBuf,
}

impl SearchCache {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }
}
