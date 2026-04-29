use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use destack_builtin::BuiltinLibraryKind;
use destack_source::{FileId, LanguageType, Loader, ModuleId, PackageId, Uri};

use crate::config::{ModuleTarget, SourceType};

/// The source of a module.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModuleSource {
    /// User or project code.
    User,
    /// Builtin library code.
    Builtin(BuiltinLibraryKind),
}

impl Hash for ModuleSource {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::User => {
                0_u8.hash(state);
            }
            Self::Builtin(kind) => {
                1_u8.hash(state);
                let tag = match kind {
                    BuiltinLibraryKind::Intrinsic => 0_u8,
                    BuiltinLibraryKind::Language => 1_u8,
                    BuiltinLibraryKind::Library => 2_u8,
                };
                tag.hash(state);
            }
        }
    }
}

/// The runtime module system format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ModuleFormat {
    /// ECMAScript module format.
    #[default]
    Esm,
    /// CommonJS module format.
    CommonJs,
}

impl ModuleFormat {
    /// Detect a module format from extension and package context.
    pub fn detect(
        path: Option<&Path>,
        language_type: LanguageType,
        source_type: SourceType,
        package_type: Option<&str>,
        tsconfig_format: Option<Self>,
    ) -> Self {
        // destack modules always use esm semantics
        if language_type.is_destack() {
            return Self::Esm;
        }

        // extension based module formats are authoritative
        if let Some(path) = path
            && let Some(format) = Self::from_extension(path)
        {
            return format;
        }

        // tsconfig module targets can force commonjs or esm semantics
        if let Some(format) = tsconfig_format {
            return format;
        }

        // package json type defines js or ts module format defaults
        if let Some(format) = Self::from_package_type(package_type) {
            return format;
        }

        // default typescript modules to esm semantics
        if language_type.is_typescript() {
            return Self::Esm;
        }

        // fall back to script or module source semantics
        if source_type.is_module() {
            Self::Esm
        } else {
            Self::CommonJs
        }
    }

    /// Detect a module format from one file extension.
    pub fn from_extension(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?;
        match extension {
            "mjs" | "mts" | "ds" => Some(Self::Esm),
            "cjs" | "cts" => Some(Self::CommonJs),
            _ => None,
        }
    }

    /// Detect a module format from one package type value.
    pub fn from_package_type(package_type: Option<&str>) -> Option<Self> {
        match package_type {
            Some("module") => Some(Self::Esm),
            Some("commonjs") => Some(Self::CommonJs),
            _ => None,
        }
    }

    /// Detect a module format from one tsconfig module target.
    pub fn from_tsconfig_target(target: ModuleTarget) -> Option<Self> {
        match target {
            ModuleTarget::CommonJs => Some(Self::CommonJs),
            ModuleTarget::Es2015
            | ModuleTarget::Es2020
            | ModuleTarget::Es2022
            | ModuleTarget::EsNext
            | ModuleTarget::Preserve => Some(Self::Esm),
            ModuleTarget::Node16 | ModuleTarget::NodeNext | ModuleTarget::None => None,
        }
    }

    /// Return true when this module format is CommonJS.
    pub fn is_commonjs(self) -> bool {
        matches!(self, Self::CommonJs)
    }
}

/// One source module.
#[derive(Debug, Clone)]
pub struct Module {
    /// The module id.
    pub id: ModuleId,
    /// The backing file id.
    pub file_id: FileId,
    /// The module uri.
    pub uri: Uri,
    /// The module path when physical.
    pub path: Option<PathBuf>,
    /// The owning package id.
    pub package_id: PackageId,
    /// The active tsconfig file id when one applies.
    pub tsconfig_file_id: Option<FileId>,
    /// The source language type.
    pub language_type: LanguageType,
    /// The active source type.
    pub source_type: SourceType,
    /// The active runtime module format.
    pub module_format: ModuleFormat,
    /// The loader used to interpret the module.
    pub loader: Loader,
    /// The module source.
    pub source: ModuleSource,
}

impl Module {
    /// Build one blank module identity.
    pub fn blank(
        id: ModuleId,
        file_id: FileId,
        uri: Uri,
        path: Option<PathBuf>,
        package_id: PackageId,
        language_type: LanguageType,
        loader: Loader,
        source: ModuleSource,
    ) -> Self {
        Self {
            id,
            file_id,
            uri,
            path,
            package_id,
            tsconfig_file_id: None,
            language_type,
            source_type: SourceType::default(),
            module_format: ModuleFormat::default(),
            loader,
            source,
        }
    }

    /// Return true when this module is user code.
    pub fn is_user(&self) -> bool {
        matches!(self.source, ModuleSource::User)
    }

    /// Return true when this module is builtin code.
    pub fn is_builtin(&self) -> bool {
        matches!(self.source, ModuleSource::Builtin(_))
    }

    /// Return true when this module contains code.
    pub fn is_code(&self) -> bool {
        self.loader.is_code()
    }
}
