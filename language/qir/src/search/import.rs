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
pub fn import_package_rank<T>(current_package: T, target_package: Option<T>) -> u8
where
    T: PartialEq,
{
    match target_package {
        Some(target_package) if current_package == target_package => 0,
        Some(_) => 2,
        None => 1,
    }
}

/// Return the fallback import path relevance for incomplete context.
pub fn unknown_import_path_relevance(package_rank: u8) -> ImportPathRelevance {
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
            SymbolSpace::TypeValue => 1,
            SymbolSpace::Value => 2,
            SymbolSpace::Label => 3,
        },
        _ => match space {
            SymbolSpace::Value => 0,
            SymbolSpace::TypeValue => 1,
            SymbolSpace::Type => 2,
            SymbolSpace::Label => 3,
        },
    }
}
