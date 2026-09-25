use std::path::Path;

use tspp_dir as dir;
use tspp_repository::{Repository, Revision};
use tspp_source::{ModuleId, PackageId, PathExt};

use super::ExportDeclaration;
use crate::source::{directory_distance, path_depth};
use crate::{QueryError, QueryResult};

/// The written use considered by completion and import suggestions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeclarationUse {
    /// A type annotation or argument.
    Type,
    /// An ordinary value reference, including an object shorthand.
    Value,
    /// An expression, including a construction or an associated member access.
    Expression,
    /// A constructor after `new`.
    Constructor,
}

impl DeclarationUse {
    /// Return whether this declaration can begin the requested expression.
    pub(crate) fn accepts_symbol_kind(self, kind: dir::SymbolKind) -> bool {
        match self {
            Self::Type => {
                kind.is_type_definition()
                    || matches!(
                        kind,
                        dir::SymbolKind::AssociatedType
                            | dir::SymbolKind::GenericTypeParameter
                            | dir::SymbolKind::Variant
                    )
            }
            Self::Value => kind.is_value(),
            Self::Expression => {
                Self::Value.accepts_symbol_kind(kind)
                    || matches!(
                        kind,
                        dir::SymbolKind::Class
                            | dir::SymbolKind::Struct
                            | dir::SymbolKind::Newtype
                            | dir::SymbolKind::Enum
                    )
            }
            Self::Constructor => matches!(kind, dir::SymbolKind::Class | dir::SymbolKind::Struct),
        }
    }

    /// Return whether this requested use accepts one exported declaration.
    pub(crate) fn accepts_export(self, declaration: ExportDeclaration) -> bool {
        match declaration {
            ExportDeclaration::Symbol { kind, .. } => self.accepts_symbol_kind(kind),
            ExportDeclaration::Namespace { .. } => self != Self::Value,
        }
    }
}

/// The stable path order for one import candidate.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ImportPathOrder {
    /// A filesystem-backed path with exact structural measurements.
    Physical {
        /// The same-directory ordering priority.
        directory_priority: u8,
        /// The same-package ordering priority.
        package_priority: u8,
        /// The path distance.
        distance: u32,
        /// The path depth difference.
        depth: u32,
    },
    /// A virtual source with only a package relationship.
    Virtual {
        /// The same-package ordering priority.
        package_priority: u8,
    },
}

/// The stable order for one import candidate.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ImportOrder {
    /// The structural path order.
    path: ImportPathOrder,
    /// The display-path length.
    display_path_length: usize,
    /// The display path.
    display_path: String,
    /// The export name.
    export_name: String,
}

impl ImportOrder {
    /// Build the stable order for one import candidate.
    pub(crate) fn new(path: ImportPathOrder, display_path: &str, export_name: &str) -> Self {
        Self {
            path,
            display_path_length: display_path.chars().count(),
            display_path: display_path.to_string(),
            export_name: export_name.to_string(),
        }
    }
}

impl ImportPathOrder {
    /// Compute the structural order between two module paths.
    pub(crate) fn between(
        repository: &Repository,
        revision: Revision,
        current_module_id: ModuleId,
        module_id: ModuleId,
    ) -> QueryResult<Self> {
        // read source and target modules in one repository coordinate system
        let source_module = repository
            .module(revision, current_module_id)
            .map_err(QueryError::from)?
            .ok_or(QueryError::missing(format!(
                "repository module: {current_module_id:?}"
            )))?;
        let target_module = repository
            .module(revision, module_id)
            .map_err(QueryError::from)?
            .ok_or(QueryError::missing(format!(
                "repository module: {module_id:?}"
            )))?;
        let current_package_id = source_module.package_id;
        let target_package_id = target_module.package_id;

        // retain package ordering for virtual modules
        let Some(source_path) = source_module.path.as_ref() else {
            let package_priority = import_package_priority(current_package_id, target_package_id);

            return Ok(Self::Virtual { package_priority });
        };
        let Some(target_path) = target_module.path.as_ref() else {
            let package_priority = import_package_priority(current_package_id, target_package_id);

            return Ok(Self::Virtual { package_priority });
        };

        import_path_order(
            source_path,
            target_path,
            current_package_id,
            target_package_id,
        )
    }
}

/// Return one package preference for import ordering.
fn import_package_priority(current_package: PackageId, target_package: PackageId) -> u8 {
    if current_package == target_package {
        0
    } else {
        2
    }
}

/// Compute structural path order for one import candidate.
fn import_path_order(
    source_path: &Path,
    target_path: &Path,
    current_package: PackageId,
    target_package: PackageId,
) -> QueryResult<ImportPathOrder> {
    let Some(source_dir) = source_path.parent() else {
        let package_priority = import_package_priority(current_package, target_package);

        return Ok(ImportPathOrder::Virtual { package_priority });
    };

    // score the relative path shape
    let source_dir = source_dir.normalize();
    let target_path = target_path.normalize();
    let target_dir = match target_path.parent() {
        Some(parent) => parent.to_path_buf(),
        None => target_path.clone(),
    };
    let package_priority = import_package_priority(current_package, target_package);
    let distance = directory_distance(&source_dir, &target_path)?;
    let target_depth = path_depth(&target_dir)?;
    let source_depth = path_depth(&source_dir)?;

    Ok(ImportPathOrder::Physical {
        directory_priority: u8::from(source_dir != target_dir),
        package_priority,
        distance,
        depth: target_depth.abs_diff(source_depth),
    })
}
