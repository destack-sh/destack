use std::path::Path;

use destack_dir::SymbolSpace;
use destack_qir::{
    ImportPathRelevance, ImportRelevance, fallback_import_path_relevance, import_package_rank,
    import_path_relevance, import_relevance,
};
use destack_source::{FileId, ModuleId, PackageId};
use destack_workspace::{Repository, Revision};

/// Compute import relevance for one candidate.
#[allow(clippy::too_many_arguments)]
pub(crate) fn repository_import_relevance(
    repository: &Repository,
    revision: Revision,
    file_id: FileId,
    current_package_id: Option<PackageId>,
    query: &str,
    export_name: &str,
    expected_space: Option<SymbolSpace>,
    space: SymbolSpace,
    module_id: ModuleId,
    module_path: &str,
) -> Option<ImportRelevance> {
    let path = path_relevance(
        repository,
        revision,
        file_id,
        current_package_id,
        module_id,
        module_path,
    );

    import_relevance(query, export_name, expected_space, space, path)
}

/// Compute structural path relevance for one import candidate.
fn path_relevance(
    repository: &Repository,
    revision: Revision,
    file_id: FileId,
    current_package_id: Option<PackageId>,
    target_module_id: ModuleId,
    module_path: &str,
) -> ImportPathRelevance {
    // resolve the target package
    let Some(module) = repository.module(revision, target_module_id).ok().flatten() else {
        let package_rank = import_package_rank(current_package_id, None);

        return fallback_import_path_relevance(package_rank);
    };
    let target_package_id = module.package_id;

    // resolve the source path
    let Some(source_file) = repository.file(revision, file_id).ok().flatten() else {
        let package_rank = import_package_rank(current_package_id, Some(target_package_id));

        return fallback_import_path_relevance(package_rank);
    };
    let Some(source_path) = source_file.path.as_ref() else {
        let package_rank = import_package_rank(current_package_id, Some(target_package_id));

        return fallback_import_path_relevance(package_rank);
    };

    import_path_relevance(
        source_path,
        Path::new(module_path),
        current_package_id,
        Some(target_package_id),
    )
}
