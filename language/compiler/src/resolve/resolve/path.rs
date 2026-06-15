use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::{SmallVec, smallvec};

use crate::CompilerResult;
use crate::export::{ExportLookup, ExportTarget};
use crate::resolve::state::{PathReference, ResolveState};

impl ResolveState<'_> {
    /// Resolve the namespace paths collected during the resolve walk.
    pub(in crate::resolve) fn resolve_path_references(&mut self) -> CompilerResult<()> {
        let references = std::mem::take(&mut self.path_references);

        for reference in references {
            self.resolve_path_reference(reference)?;
        }

        Ok(())
    }

    /// Resolve one namespace path into its per node references.
    fn resolve_path_reference(&mut self, reference: PathReference) -> CompilerResult<()> {
        let segments = reference.path.segments.clone();
        let Some((root, tail)) = segments.split_first() else {
            return Ok(());
        };

        // stop when the root names no namespace: such members resolve by type in check
        let Some(mut module) = self.namespace_root_module(reference.source.local_id, *root) else {
            return Ok(());
        };

        // resolve each segment against the running namespace, stopping at the first non-namespace
        let mut prefixes: SmallVec<[dir::Reference; 4]> =
            smallvec![dir::Reference::Namespace(module)];
        for segment in tail.iter().copied() {
            let key = dir::ExportKey::named(dir::StaticKey::Name(segment));
            match self.resolve_export_target(module, key)? {
                // follow a symbol prefix only while it keeps naming a namespace
                ExportLookup::Found(ExportTarget::Symbol(symbol)) => {
                    prefixes.push(dir::Reference::Bound(smallvec![symbol]));
                    match self.local_namespace_symbol_module(symbol) {
                        Some(next) => module = next,
                        None => break,
                    }
                }

                // follow a nested namespace prefix
                ExportLookup::Found(ExportTarget::Namespace(next)) => {
                    prefixes.push(dir::Reference::Namespace(next));
                    module = next;
                }

                // stop at conflicting symbol targets
                ExportLookup::Ambiguous(targets) => {
                    let symbols = targets
                        .into_iter()
                        .filter_map(|target| match target {
                            ExportTarget::Symbol(symbol) => Some(symbol),
                            ExportTarget::Namespace(_) => None,
                        })
                        .collect();
                    prefixes.push(dir::Reference::Ambiguous(symbols));
                    break;
                }

                // stop at a missing export
                ExportLookup::Missing => {
                    prefixes.push(dir::Reference::Missing);
                    break;
                }
            }
        }

        // record one reference for a flat type node, or one per member node for a value chain
        if reference.source.local_id.ty == dir::NodeType::TypeExpression {
            self.record_flat_reference(reference.source, &prefixes, segments.len());
        } else {
            self.record_chain_references(reference.source, &prefixes);
        }

        Ok(())
    }

    /// Record the single reference carried by one flat type reference node.
    ///
    /// `total` is the full segment count, which exceeds the prefix count when the
    /// walk stops early on a symbol whose tail projects as members.
    fn record_flat_reference(
        &mut self,
        source: dir::GlobalNodeIdAny,
        prefixes: &[dir::Reference],
        total: usize,
    ) {
        // project the tail off the first symbol reached before the final segment
        let projected = prefixes
            .iter()
            .enumerate()
            .find_map(|(index, prefix)| match prefix {
                dir::Reference::Bound(symbols) if index + 1 < total => {
                    symbols.first().map(|symbol| (*symbol, index as u32 + 1))
                }
                _ => None,
            });
        if let Some((base, from)) = projected {
            self.references
                .insert(source, dir::Reference::Projected { base, from });

            return;
        }

        // otherwise the final segment carries the reference, unless it names a namespace
        match prefixes.last() {
            Some(dir::Reference::Namespace(_)) | None => {}
            Some(reference) => self.references.insert(source, reference.clone()),
        }
    }

    /// Record one reference per member node in a value path chain.
    fn record_chain_references(
        &mut self,
        source: dir::GlobalNodeIdAny,
        prefixes: &[dir::Reference],
    ) {
        // collect the member nodes from root to leaf, aligned with the segments
        let Ok(leaf) = dir::LocalNodeId::<dir::Expression>::try_from(source.local_id) else {
            return;
        };
        let mut nodes: SmallVec<[dir::LocalNodeId<dir::Expression>; 4]> = SmallVec::new();
        let mut current = Some(leaf);
        while let Some(node) = current {
            nodes.push(node);
            current = match self.view.get(node) {
                dir::Expression::Member { left, .. } => Some(*left),
                _ => None,
            };
        }
        nodes.reverse();

        // place each resolved prefix on its member node, leaving type-directed tails unrecorded
        for (node, reference) in nodes.iter().zip(prefixes) {
            self.references
                .insert(node.into_global_any(self.module), reference.clone());
        }
    }

    /// Return the namespace module one path root selects.
    fn namespace_root_module(
        &self,
        source: dir::LocalNodeIdAny,
        root: dir::StringId,
    ) -> Option<ModuleId> {
        let key = dir::StaticKey::Name(root);
        let lookup =
            self.bindings
                .lookup_symbol_at(&self.view, source, key, dir::SymbolSpace::Value);

        // prefer a lexical namespace import
        match lookup {
            dir::SymbolLookup::Found(symbol) => {
                let Some(dir::ImportTarget::Namespace(module)) = self.imports.symbol_target(symbol)
                else {
                    return None;
                };

                return Some(module);
            }
            dir::SymbolLookup::Ambiguous(_) => return None,
            dir::SymbolLookup::Missing => {}
        }

        // otherwise fall back to a single ambient namespace global
        let targets = self.imports.global_targets(key)?;
        let mut modules = targets.iter().filter_map(|target| match target {
            dir::ImportTarget::Namespace(module) => Some(*module),
            dir::ImportTarget::Symbol(_) => None,
        });

        let module = modules.next()?;
        if modules.next().is_some() {
            return None;
        }

        Some(module)
    }

    /// Return the namespace module one local namespace-import symbol selects.
    fn local_namespace_symbol_module(&self, symbol: dir::GlobalSymbolId) -> Option<ModuleId> {
        if symbol.module_id != self.module {
            return None;
        }

        match self.imports.symbol_target(symbol.local_id) {
            Some(dir::ImportTarget::Namespace(module)) => Some(module),
            Some(dir::ImportTarget::Symbol(_)) | None => None,
        }
    }
}
