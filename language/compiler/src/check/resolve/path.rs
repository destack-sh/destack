use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{CheckState, Condition};

use super::name::{NameLookup, NameTarget};

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

impl PathLookup {
    /// Return candidates whose availability is guaranteed by one active guard.
    pub(in crate::check) fn available_under(self, guard: &Condition) -> Self {
        // collect visible candidates
        let candidates = match self {
            Self::Found(candidate) => smallvec::smallvec![candidate],
            Self::Missing => return Self::Missing,
            Self::Ambiguous(candidates) => candidates,
        };

        // remove candidates not guaranteed in this branch
        let mut candidates = candidates
            .into_iter()
            .filter(|candidate| candidate.condition().is_guaranteed_by(guard))
            .collect::<SmallVec<[PathCandidate; 4]>>();

        // preserve lookup cardinality after filtering
        if candidates.is_empty() {
            Self::Missing
        } else if candidates.len() == 1 {
            Self::Found(candidates.remove(0))
        } else {
            Self::Ambiguous(candidates)
        }
    }
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
    ) -> PathLookup {
        // split path into lexical root and namespace tail
        let Some((name, tail)) = path.segments.split_first() else {
            return PathLookup::Missing;
        };

        // resolve multi-segment roots through value space
        let root_space = if tail.is_empty() {
            space
        } else {
            dir::SymbolSpace::Value
        };

        // resolve the root binding before consulting namespace paths
        let root = match self.lookup_name_by_name(module, source, *name, root_space) {
            // exactly one root target
            NameLookup::Found(candidate) => match candidate.target {
                NameTarget::Symbol(symbol) => PathCandidate::Symbol {
                    symbol,
                    condition: candidate.condition,
                },
                NameTarget::Namespace(module) => PathCandidate::Namespace {
                    module,
                    condition: candidate.condition,
                },
            },
            // no root target
            NameLookup::Missing => return PathLookup::Missing,
            // multiple root targets
            NameLookup::Ambiguous(candidates) => {
                let candidates = candidates
                    .into_iter()
                    .map(|candidate| match candidate.target {
                        NameTarget::Symbol(symbol) => PathCandidate::Symbol {
                            symbol,
                            condition: candidate.condition,
                        },
                        NameTarget::Namespace(module) => PathCandidate::Namespace {
                            module,
                            condition: candidate.condition,
                        },
                    })
                    .collect();

                return PathLookup::Ambiguous(candidates);
            }
        };

        // return the root symbol for single segment paths
        if tail.is_empty() {
            return PathLookup::Found(root);
        }

        // use resolve's namespace path table for imported namespace paths
        let key = dir::PathKey::new(source.into_global(module), path.segments.len() as u32);
        let Some(resolution) = self.module(module).resolved.paths.get(key).cloned() else {
            return PathLookup::Missing;
        };

        self.path_resolution_lookup(root.condition(), resolution)
    }

    /// Return the symbol named by one guarded source path.
    pub(in crate::check) fn symbol_by_path_under(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        path: &dir::Path,
        space: dir::SymbolSpace,
        guard: &Condition,
    ) -> Option<dir::GlobalSymbolId> {
        let lookup = self
            .lookup_path(module, source, path, space)
            .available_under(guard);

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
