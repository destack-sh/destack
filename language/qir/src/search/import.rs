use std::path::{Component, Path};

use destack_dir::SymbolSpace;

use crate::{MatchQuality, match_quality};

/// The structural import-path relevance for one import candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportPathRelevance {
    /// The same-directory preference rank.
    pub directory_rank: u8,
    /// The same-package preference rank.
    pub package_rank: u8,
    /// The relative path distance rank.
    pub distance_rank: u32,
    /// The relative depth difference rank.
    pub depth_rank: u32,
}

/// The lexical and path relevance for one import candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportRelevance {
    /// The lexical match quality for the exported name.
    pub lexical: MatchQuality,
    /// The space preference rank.
    pub space_rank: u8,
    /// The same-directory preference rank.
    pub directory_rank: u8,
    /// The same-package preference rank.
    pub package_rank: u8,
    /// The relative path distance rank.
    pub distance_rank: u32,
    /// The relative depth difference rank.
    pub depth_rank: u32,
}

/// The stable structured ordering key for one import candidate.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImportSortKey {
    /// The coarse space ordering rank.
    pub space_rank: u8,
    /// The same-directory ordering rank.
    pub directory_rank: u8,
    /// The same-package ordering rank.
    pub package_rank: u8,
    /// The path distance ordering rank.
    pub distance_rank: u32,
    /// The path depth ordering rank.
    pub depth_rank: u32,
    /// The display-path length ordering rank.
    pub display_path_length: usize,
    /// The display path.
    pub display_path: String,
    /// The export name.
    pub export_name: String,
}

/// Compute import relevance for one candidate.
pub fn import_relevance(
    query: &str,
    export_name: &str,
    expected_space: Option<SymbolSpace>,
    space: SymbolSpace,
    path: ImportPathRelevance,
) -> Option<ImportRelevance> {
    let lexical = match_quality(export_name, query)?;
    let space_rank = import_space_rank(space, expected_space);

    Some(ImportRelevance {
        lexical,
        space_rank,
        directory_rank: path.directory_rank,
        package_rank: path.package_rank,
        distance_rank: path.distance_rank,
        depth_rank: path.depth_rank,
    })
}

/// Build the stable sort text for one import candidate.
pub fn import_sort_text(
    relevance: &ImportRelevance,
    display_path: &str,
    export_name: &str,
) -> String {
    let sort_key = import_sort_key(relevance, display_path, export_name);

    format!(
        "{}:{}:{}:{:04}:{:04}:{:04}:{display_path}:{export_name}",
        sort_key.space_rank,
        sort_key.directory_rank,
        sort_key.package_rank,
        sort_key.distance_rank,
        sort_key.depth_rank,
        sort_key.display_path_length,
    )
}

/// Build the stable structured sort key for one import candidate.
pub fn import_sort_key(
    relevance: &ImportRelevance,
    display_path: &str,
    export_name: &str,
) -> ImportSortKey {
    ImportSortKey {
        space_rank: relevance.space_rank,
        directory_rank: relevance.directory_rank,
        package_rank: relevance.package_rank,
        distance_rank: relevance.distance_rank,
        depth_rank: relevance.depth_rank,
        display_path_length: display_path.chars().count(),
        display_path: display_path.to_string(),
        export_name: export_name.to_string(),
    }
}

/// Return one package preference rank for import ordering.
pub fn import_package_rank<T>(current_package: Option<T>, target_package: Option<T>) -> u8
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
pub fn import_path_relevance<T>(
    source_path: &Path,
    target_path: &Path,
    current_package: Option<T>,
    target_package: Option<T>,
) -> ImportPathRelevance
where
    T: PartialEq,
{
    let Some(source_dir) = source_path.parent() else {
        let package_rank = import_package_rank(current_package, target_package);

        return package_import_path_relevance(package_rank);
    };

    // score the relative path shape
    let source_dir = normalize_path(source_dir);
    let target_path = normalize_path(target_path);
    let target_dir = target_path.parent().unwrap_or(&target_path).to_path_buf();
    let package_rank = import_package_rank(current_package, target_package);

    ImportPathRelevance {
        directory_rank: u8::from(source_dir != target_dir),
        package_rank,
        distance_rank: path_distance(&source_dir, &target_path),
        depth_rank: path_component_count(&target_dir)
            .saturating_sub(path_component_count(&source_dir)),
    }
}

/// Return import path relevance when only package relationship is known.
pub fn package_import_path_relevance(package_rank: u8) -> ImportPathRelevance {
    ImportPathRelevance {
        directory_rank: 2,
        package_rank,
        distance_rank: u32::MAX,
        depth_rank: u32::MAX,
    }
}

/// Return one coarse import space rank.
fn import_space_rank(space: SymbolSpace, expected_space: Option<SymbolSpace>) -> u8 {
    match expected_space {
        Some(SymbolSpace::Type) => match space {
            SymbolSpace::Type => 0,
            SymbolSpace::Value => 1,
            SymbolSpace::Label => 2,
        },
        _ => match space {
            SymbolSpace::Value => 0,
            SymbolSpace::Type => 1,
            SymbolSpace::Label => 2,
        },
    }
}

/// Normalize a path through lexical components.
fn normalize_path(path: &Path) -> std::path::PathBuf {
    let mut normalized = std::path::PathBuf::new();

    // collect stable components
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => {
                normalized.push(prefix.as_os_str());
            }
            Component::RootDir => {
                normalized.push(std::path::MAIN_SEPARATOR.to_string());
            }
            Component::Normal(part) => {
                normalized.push(part);
            }
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.push("..");
            }
        }
    }

    normalized
}

/// Compute a heuristic distance between two paths.
fn path_distance(from_dir: &Path, to_path: &Path) -> u32 {
    let to_dir = to_path.parent().unwrap_or(to_path);

    // resolve components for both paths
    let from_components = normal_components(from_dir);
    let to_components = normal_components(to_dir);

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
    normal_components(path).len() as u32
}

/// Collect normalized path components for stable comparisons.
fn normal_components(path: &Path) -> Vec<String> {
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
