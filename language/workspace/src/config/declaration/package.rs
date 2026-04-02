use std::path::PathBuf;
use std::sync::Arc;

use destack_source::{File, FileContent, FileId, Uri};

use crate::config::PackageJson;

/// Parsed `package.json` declaration.
#[derive(Debug, Clone)]
pub struct PackageDeclaration {
    /// The id of the `package.json` file.
    pub file_id: FileId,
    /// The uri of the `package.json` file.
    pub uri: Uri,
    /// The physical path to the `package.json` file.
    pub path: PathBuf,
    /// The realpath to the declaration.
    pub realpath: PathBuf,
    /// The declaration directory.
    pub directory: PathBuf,
    /// The raw JSON content of the declaration.
    pub json: PackageJson,
}

impl PackageDeclaration {
    /// Parse one package declaration from one json file.
    pub fn parse(file: &Arc<File>, realpath: PathBuf) -> Result<Self, serde_json::Error> {
        let FileContent::Json { value, .. } = &file.content else {
            return Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "file is not JSON",
            )));
        };

        let json: PackageJson = serde_json::from_value(value.clone())?;
        let path = file
            .uri
            .to_path_buf()
            .expect("package.json file must have a valid path");
        let directory = path
            .parent()
            .expect("package.json must have a parent directory")
            .to_path_buf();

        Ok(Self {
            file_id: file.id,
            uri: file.uri.clone(),
            path,
            realpath,
            directory,
            json,
        })
    }

    /// Return the declared package name when present.
    pub fn name(&self) -> Option<&str> {
        self.json.name.as_deref()
    }

    /// Return the declared package version when present.
    pub fn version(&self) -> Option<&str> {
        self.json.version.as_deref()
    }

    /// Return the declared package module type when present.
    pub fn module_type(&self) -> Option<&str> {
        self.json.module_type.as_deref()
    }
}
