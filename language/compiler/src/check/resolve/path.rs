use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{CheckState, Condition};

use super::name::NameLookup;

/// One target resolved through source path lookup.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum PathCandidate {
    /// A symbol target was resolved.
    Symbol {
        /// The resolved symbol.
        symbol: dir::GlobalSymbolId,
        /// The condition under which the symbol exists.
        condition: Condition,
    },
    /// A namespace target was resolved.
    Namespace {
        /// The resolved namespace module.
        module: ModuleId,
        /// The condition under which the namespace exists.
        condition: Condition,
    },
}

impl PathCandidate {
    /// Return the condition under which this candidate exists.
    pub(in crate::check) fn condition(&self) -> Condition {
        match self {
            Self::Symbol { condition, .. } | Self::Namespace { condition, .. } => condition.clone(),
        }
    }

    /// Return the resolved symbol when this is a symbol candidate.
    pub(in crate::check) fn symbol(&self) -> Option<dir::GlobalSymbolId> {
        match self {
            Self::Symbol { symbol, .. } => Some(*symbol),
            Self::Namespace { .. } => None,
        }
    }
}

/// Result of looking up one source path during check.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum PathLookup {
    /// A single path target was resolved.
    Found(PathCandidate),
    /// No matching path target exists.
    Missing,
    /// Multiple path targets exist.
    Ambiguous(SmallVec<[PathCandidate; 4]>),
}

impl CheckState<'_> {
    /// Return whether one source path starts from a resolved namespace root.
    pub(in crate::check) fn path_has_namespace_root(
        &self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) -> bool {
        let key = dir::PathKey::new(source.into_global(module), 1);

        matches!(
            self.module(module).resolved.paths.get(key),
            Some(dir::PathResolution::Found(dir::PathTarget::Namespace(_)))
        )
    }

    /// Look up one source path.
    pub(in crate::check) fn lookup_path(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        path: &dir::Path,
        space: dir::SymbolSpace,
    ) -> CompilerResult<PathLookup> {
        let Some((name, tail)) = path.segments.split_first() else {
            return Ok(PathLookup::Missing);
        };
        let root_space = if tail.is_empty() {
            space
        } else {
            dir::SymbolSpace::Value
        };
        let root = match self.lookup_symbol_by_name(module, source, *name, root_space) {
            // exactly one root symbol
            NameLookup::Found(candidate) => PathCandidate::Symbol {
                symbol: candidate.symbol,
                condition: candidate.condition,
            },
            // no root symbol
            NameLookup::Missing => return Ok(PathLookup::Missing),
            // multiple root symbols
            NameLookup::Ambiguous(candidates) => {
                let candidates = candidates
                    .into_iter()
                    .map(|candidate| PathCandidate::Symbol {
                        symbol: candidate.symbol,
                        condition: candidate.condition,
                    })
                    .collect();

                return Ok(PathLookup::Ambiguous(candidates));
            }
        };

        // return the root symbol for single segment paths
        if tail.is_empty() {
            return Ok(PathLookup::Found(root));
        }

        // use resolve's namespace path table for imported namespace paths
        let key = dir::PathKey::new(source.into_global(module), path.segments.len() as u32);
        let Some(resolution) = self.module(module).resolved.paths.get(key).cloned() else {
            return Ok(PathLookup::Missing);
        };

        Ok(self.path_resolution_lookup(root.condition(), resolution))
    }

    /// Require a symbol named by one source path.
    pub(in crate::check) fn require_symbol_by_path(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        path: &dir::Path,
        space: dir::SymbolSpace,
    ) -> Option<dir::GlobalSymbolId> {
        let lookup = self
            .lookup_path(module, source, path, space)
            .unwrap_or_else(|_| panic!("path lookup failed for checked module {module:?}"));

        match lookup {
            PathLookup::Found(candidate) => match candidate.symbol() {
                Some(symbol) => Some(symbol),
                None => {
                    self.report_unresolved_reference(module, source, path);

                    None
                }
            },
            PathLookup::Missing => {
                self.report_unresolved_reference(module, source, path);

                None
            }
            PathLookup::Ambiguous(_) => {
                self.report_ambiguous_reference(module, source, path);

                None
            }
        }
    }

    /// Return one check lookup from a resolved path table entry.
    fn path_resolution_lookup(
        &self,
        condition: Condition,
        resolution: dir::PathResolution,
    ) -> PathLookup {
        match resolution {
            dir::PathResolution::Found(target) => {
                PathLookup::Found(self.path_target_candidate(condition, target))
            }
            dir::PathResolution::Missing => PathLookup::Missing,
            dir::PathResolution::Ambiguous(targets) => PathLookup::Ambiguous(
                targets
                    .into_iter()
                    .map(|target| self.path_target_candidate(condition.clone(), target))
                    .collect(),
            ),
        }
    }

    /// Return one path candidate from a resolved DIR path target.
    fn path_target_candidate(
        &self,
        condition: Condition,
        target: dir::PathTarget,
    ) -> PathCandidate {
        match target {
            dir::PathTarget::Symbol(symbol) => PathCandidate::Symbol {
                symbol,
                condition: condition.and(self.symbol_availability(symbol)),
            },
            dir::PathTarget::Namespace(module) => PathCandidate::Namespace { module, condition },
        }
    }
}
