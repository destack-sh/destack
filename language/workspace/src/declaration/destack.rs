use std::path::PathBuf;
use std::sync::Arc;

use destack_source::{File, FileContent, FileId};

use crate::config::DestackJson;
use crate::options::{PackageOptions, WorkspaceOptions};

/// Parsed `destack.json` declaration.
#[derive(Debug, Clone)]
pub struct DestackDeclaration {
    /// The id of the `destack.json` file.
    pub file_id: FileId,
    /// Path to the `destack.json` file.
    pub path: PathBuf,
    /// The directory containing the `destack.json` file.
    pub directory: PathBuf,
    /// The raw JSON content of the `destack.json` file.
    pub json: DestackJson,
}

impl DestackDeclaration {
    /// Parse one `destack.json` declaration from one file.
    pub fn parse(file: &Arc<File>) -> Result<Self, serde_json::Error> {
        let FileContent::Json { value, .. } = &file.content else {
            return Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "file is not JSON",
            )));
        };

        let json: DestackJson = serde_json::from_value(value.clone())?;

        json.linter.validate().map_err(|error| {
            serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::InvalidData, error))
        })?;

        let path = file
            .path
            .clone()
            .or_else(|| file.uri.to_path_buf())
            .expect("destack.json must have a valid path");
        let directory = path
            .parent()
            .expect("destack.json must have a parent directory")
            .to_path_buf();

        Ok(Self {
            file_id: file.id,
            path,
            directory,
            json,
        })
    }

    /// Derive effective package options from this declaration.
    pub fn package_options(&self) -> PackageOptions {
        PackageOptions::from(&self.json)
    }

    /// Derive effective workspace options from this declaration.
    pub fn workspace_options(&self) -> WorkspaceOptions {
        WorkspaceOptions::from(&self.json)
    }
}
