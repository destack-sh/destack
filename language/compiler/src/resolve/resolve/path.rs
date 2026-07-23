use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::{SmallVec, smallvec};

use crate::CompilerResult;
use crate::export::ExportLookup;
use crate::resolve::state::{PathReference, ResolveState};

impl ResolveState<'_> {
    /// Resolve the source paths collected during the resolve walk.
    pub(in crate::resolve) fn resolve_path_references(&mut self) -> CompilerResult<()> {
        let references = std::mem::take(&mut self.path_references);

        for reference in references {
            self.resolve_path_reference(reference)?;
        }

        Ok(())
    }

    /// Resolve one source path into its per node references.
    fn resolve_path_reference(&mut self, reference: PathReference) -> CompilerResult<()> {
        let segments = reference.path.segments.clone();
        let Some((root, tail)) = segments.split_first() else {
            return Ok(());
        };

        let root = self.resolve_path_root(reference.source.local_id, *root);
        let mut prefixes: SmallVec<[dir::Reference; 4]> = smallvec![root.clone()];

        // walk exports only while the running prefix is a namespace
        let Some(mut module) = self.namespace_module(&root) else {
            self.record_reference(reference.source, &prefixes, segments.len());

            return Ok(());
        };
        for segment in tail.iter().copied() {
            let key = dir::ExportKey::named(dir::StaticKey::Name(segment));
            match self.resolve_export_target(module, key)? {
                // follow a symbol prefix only while it keeps naming a namespace
                ExportLookup::Found(dir::ExportTarget::Symbol(symbol)) => {
                    prefixes.push(dir::Reference::Bound(smallvec![symbol]));
                    match self.local_namespace_symbol_module(symbol) {
                        Some(next) => module = next,
                        None => break,
                    }
                }

                // follow a nested namespace prefix
                ExportLookup::Found(dir::ExportTarget::Namespace(next)) => {
                    prefixes.push(dir::Reference::Namespace(next));
                    module = next;
                }

                // stop at conflicting symbol targets
                ExportLookup::Ambiguous(targets) => {
                    let symbols = targets
                        .into_iter()
                        .filter_map(|target| match target {
                            dir::ExportTarget::Symbol(symbol) => Some(symbol),
                            dir::ExportTarget::Namespace(_) => None,
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

        self.record_reference(reference.source, &prefixes, segments.len());

        Ok(())
    }

    /// Resolve the first segment of one source path.
    fn resolve_path_root(
        &self,
        source: dir::LocalNodeIdAny,
        root: dir::StringId,
    ) -> dir::Reference {
        let key = dir::StaticKey::Name(root);
        let symbols = self.visible_symbols(source, key, dir::SymbolSpace::Declaration);
        if !symbols.is_empty() {
            return self.reference_from_symbols(symbols);
        }

        let Some(targets) = self.imports.global_targets(key) else {
            return dir::Reference::Missing;
        };

        self.reference_from_targets(targets.iter().copied())
    }

    /// Return the namespace module named by one resolved prefix.
    fn namespace_module(&self, reference: &dir::Reference) -> Option<destack_source::ModuleId> {
        match reference {
            dir::Reference::Namespace(module) => Some(*module),
            dir::Reference::Bound(symbols) => match symbols.as_slice() {
                [symbol] => self.local_namespace_symbol_module(*symbol),
                _ => None,
            },
            dir::Reference::Ambiguous(_)
            | dir::Reference::Missing
            | dir::Reference::Projected { .. } => None,
        }
    }

    /// Record one resolved source path in the reference table.
    fn record_reference(
        &mut self,
        source: dir::GlobalNodeIdAny,
        prefixes: &[dir::Reference],
        total: usize,
    ) {
        // record flat references unless the source is an expression member chain
        if source.local_id.ty != dir::NodeType::Expression {
            self.record_flat_reference(source, prefixes, total);
        } else {
            self.record_chain_references(source, prefixes);
        }
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

        // otherwise the final segment carries the reference
        if let Some(reference) = prefixes.last() {
            self.references.insert(source, reference.clone());
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
