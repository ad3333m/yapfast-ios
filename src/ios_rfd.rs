//! rfd's file dialogs as the iPhone build sees them: none yet, so each one
//! picks nothing. rfd has no iOS backend; the code that opens a dialog stays
//! the same. See docs/ios.md.

use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct AsyncFileDialog;

pub struct FileHandle(PathBuf);

impl FileHandle {
    pub fn path(&self) -> &Path {
        &self.0
    }

    pub fn file_name(&self) -> String {
        self.0
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
}

impl AsyncFileDialog {
    pub fn new() -> Self {
        Self
    }

    pub fn set_title(self, _: impl Into<String>) -> Self {
        self
    }

    pub fn add_filter(self, _: impl Into<String>, _: &[impl ToString]) -> Self {
        self
    }

    pub fn set_directory(self, _: impl AsRef<Path>) -> Self {
        self
    }

    pub fn set_file_name(self, _: impl Into<String>) -> Self {
        self
    }

    pub async fn pick_file(self) -> Option<FileHandle> {
        None
    }

    pub async fn pick_files(self) -> Option<Vec<FileHandle>> {
        None
    }

    pub async fn save_file(self) -> Option<FileHandle> {
        None
    }
}
