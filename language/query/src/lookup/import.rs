use std::path::{Component, Path};

use destack_dir as dir;
use destack_repository::{Repository, Revision};
use destack_source::{FileId, ModuleId, PackageId, PathExt};

use super::{MatchQuality, match_quality};

/// The declaration use preferred by one import search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SymbolUse {
    /// Type expression reference.
    Type,
    /// Runtime value reference.
    Value,
}

impl SymbolUse {
    /// Return whether this requested use accepts one DIR symbol kind.
    pub(crate) fn accepts_symbol_kind(self, symbol_kind: dir::SymbolKind) -> bool {
        match self {
            Self::Type => symbol_kind.can_be_used_as_type(),
            Self::Value => symbol_kind.can_be_used_as_value(),
        }
    }
}

/// Resolve a module name from a file path.
pub(crate) fn module_name_from_path(path: &Path) -> Option<String> {
    let file_name = path.file_name()?.to_string_lossy();

    Some(strip_module_extension(file_name.as_ref()))
}

/// Strip a code module extension from an import path.
pub(crate) fn strip_module_extension(path: &str) -> String {
    let extensions = [".d.ts", ".d.ds", ".tsx", ".ts", ".jsx", ".js", ".ds"];

    for extension in extensions {
        if let Some(stripped) = path.strip_suffix(extension) {
            return stripped.to_string();
        }
    }

    path.to_string()
}

/// The structural import-path relevance for one import candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ImportPathRelevance {
    /// The same-directory priority.
    pub directory_priority: u8,
    /// The same-package priority.
    pub package_priority: u8,
    /// The relative path distance.
    pub distance: u32,
    /// The relative depth difference.
    pub depth: u32,
}

/// The lexical and path relevance for one import candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ImportRelevance {
    /// The lexical match quality for the exported name.
    pub lexical: MatchQuality,
    /// The declaration-use priority.
    pub use_priority: u8,
    /// The same-directory priority.
    pub directory_priority: u8,
    /// The same-package priority.
    pub package_priority: u8,
    /// The relative path distance.
    pub distance: u32,
    /// The relative depth difference.
    pub depth: u32,
}

/// One import candidate being ranked for auto import.
struct ImportCandidate<'a> {
    /// The repository used to read source and target metadata.
    repository: &'a Repository,
    /// The repository revision being queried.
    revision: Revision,
    /// The current source file id.
    file_id: FileId,
    /// The package containing the current source file.
    current_package_id: Option<PackageId>,
    /// The user query text.
    query: &'a str,
    /// The exported symbol name.
    export_name: &'a str,
    /// The requested symbol use when known.
    expected_use: Option<SymbolUse>,
    /// The exported DIR symbol kind.
    kind: dir::SymbolKind,
    /// The target module id.
    module_id: ModuleId,
    /// The target module path.
    module_path: &'a str,
}

/// The stable order for one import candidate.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ImportOrder {
    /// The coarse use ordering priority.
    pub use_priority: u8,
    /// The same-directory ordering priority.
    pub directory_priority: u8,
    /// The same-package ordering priority.
    pub package_priority: u8,
    /// The path distance.
    pub distance: u32,
    /// The path depth difference.
    pub depth: u32,
    /// The display-path length.
    pub display_path_length: usize,
    /// The display path.
    pub display_path: String,
    /// The export name.
    pub export_name: String,
}

impl ImportOrder {
    /// Build the stable order for one import candidate.
    pub(crate) fn new(relevance: &ImportRelevance, display_path: &str, export_name: &str) -> Self {
        Self {
            use_priority: relevance.use_priority,
            directory_priority: relevance.directory_priority,
            package_priority: relevance.package_priority,
            distance: relevance.distance,
            depth: relevance.depth,
            display_path_length: display_path.chars().count(),
            display_path: display_path.to_string(),
            export_name: export_name.to_string(),
        }
    }

    /// Build stable LSP sort text for this import candidate.
    pub(crate) fn text(&self) -> String {
        format!(
            "{}:{}:{}:{:04}:{:04}:{:04}:{}:{}",
            self.use_priority,
            self.directory_priority,
            self.package_priority,
            self.distance,
            self.depth,
            self.display_path_length,
            self.display_path,
            self.export_name,
        )
    }
}

impl ImportCandidate<'_> {
    /// Compute relevance for this import candidate.
    fn relevance(&self) -> Option<ImportRelevance> {
        let path = self.path_relevance();
        let lexical = match_quality(self.export_name, self.query)?;
        let use_priority = import_use_priority(self.kind, self.expected_use);

        Some(ImportRelevance {
            lexical,
            use_priority,
            directory_priority: path.directory_priority,
            package_priority: path.package_priority,
            distance: path.distance,
            depth: path.depth,
        })
    }

    /// Compute structural path relevance for this import candidate.
    fn path_relevance(&self) -> ImportPathRelevance {
        // resolve the target package
        let module = self
            .repository
            .module(self.revision, self.module_id)
            .unwrap_or_else(|error| {
                panic!(
                    "failed to read import target module {:?}: {error}",
                    self.module_id
                )
            })
            .unwrap_or_else(|| panic!("missing import target module {:?}", self.module_id));
        let target_package_id = module.package_id;

        // resolve the source path
        let source_file = self
            .repository
            .file(self.revision, self.file_id)
            .unwrap_or_else(|error| {
                panic!(
                    "failed to read import source file {:?}: {error}",
                    self.file_id
                )
            })
            .unwrap_or_else(|| panic!("missing import source file {:?}", self.file_id));
        let Some(source_path) = source_file.path.as_ref() else {
            let package_priority =
                import_package_priority(self.current_package_id, Some(target_package_id));

            return package_import_path_relevance(package_priority);
        };

        import_path_relevance(
            source_path,
            Path::new(self.module_path),
            self.current_package_id,
            Some(target_package_id),
        )
    }
}

/// Compute import relevance for one candidate.
#[allow(clippy::too_many_arguments)]
pub(crate) fn repository_import_relevance(
    repository: &Repository,
    revision: Revision,
    file_id: FileId,
    current_package_id: Option<PackageId>,
    query: &str,
    export_name: &str,
    expected_use: Option<SymbolUse>,
    kind: dir::SymbolKind,
    module_id: ModuleId,
    module_path: &str,
) -> Option<ImportRelevance> {
    let candidate = ImportCandidate {
        repository,
        revision,
        file_id,
        current_package_id,
        query,
        export_name,
        expected_use,
        kind,
        module_id,
        module_path,
    };

    candidate.relevance()
}

/// Return one package preference for import ordering.
fn import_package_priority<T>(current_package: Option<T>, target_package: Option<T>) -> u8
where
    T: PartialEq,
{
    match (current_package.as_ref(), target_package.as_ref()) {
        (Some(current_package), Some(target_package)) if current_package == target_package => 0,
        (Some(_), _) => 2,
        (None, _) => 1,
    }
}

/// Compute structural path relevance for one import candidate.
fn import_path_relevance<T>(
    source_path: &Path,
    target_path: &Path,
    current_package: Option<T>,
    target_package: Option<T>,
) -> ImportPathRelevance
where
    T: PartialEq,
{
    let Some(source_dir) = source_path.parent() else {
        let package_priority = import_package_priority(current_package, target_package);

        return package_import_path_relevance(package_priority);
    };

    // score the relative path shape
    let source_dir = source_dir.normalize();
    let target_path = target_path.normalize();
    let target_dir = target_path.parent().unwrap_or(&target_path).to_path_buf();
    let package_priority = import_package_priority(current_package, target_package);

    ImportPathRelevance {
        directory_priority: u8::from(source_dir != target_dir),
        package_priority,
        distance: path_distance(&source_dir, &target_path),
        depth: path_component_count(&target_dir).saturating_sub(path_component_count(&source_dir)),
    }
}

/// Return import path relevance when only package relationship is known.
fn package_import_path_relevance(package_priority: u8) -> ImportPathRelevance {
    ImportPathRelevance {
        directory_priority: 2,
        package_priority,
        distance: u32::MAX,
        depth: u32::MAX,
    }
}

/// Return one coarse import use priority.
fn import_use_priority(kind: dir::SymbolKind, expected_use: Option<SymbolUse>) -> u8 {
    match expected_use {
        Some(SymbolUse::Type) if kind.can_be_used_as_type() => 0,
        Some(SymbolUse::Value) if kind.can_be_used_as_value() => 0,
        None if kind.can_be_used_as_value() => 0,
        None if kind.can_be_used_as_type() => 1,
        _ => 2,
    }
}

/// Compute a heuristic distance between two paths.
fn path_distance(from_dir: &Path, to_path: &Path) -> u32 {
    let to_dir = to_path.parent().unwrap_or(to_path);

    // resolve components for both paths
    let from_components = normalized_components(from_dir);
    let to_components = normalized_components(to_dir);

    // compute the shared prefix length
    let mut common = 0usize;
    while common < from_components.len()
        && common < to_components.len()
        && from_components[common] == to_components[common]
    {
        common += 1;
    }

    // compute the number of path steps
    let ups = from_components.len().saturating_sub(common);
    let downs = to_components.len().saturating_sub(common);

    (ups + downs) as u32
}

/// Count normalized components for a path.
fn path_component_count(path: &Path) -> u32 {
    normalized_components(path).len() as u32
}

/// Collect normalized path components for stable comparisons.
fn normalized_components(path: &Path) -> Vec<String> {
    let mut components = Vec::new();

    // translate platform specific components into normalized strings
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => {
                components.push(prefix.as_os_str().to_string_lossy().to_string());
            }
            Component::RootDir => {
                components.push("/".to_string());
            }
            Component::Normal(part) => {
                components.push(part.to_string_lossy().to_string());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                components.push("..".to_string());
            }
        }
    }

    components
}
