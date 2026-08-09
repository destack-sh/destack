use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::{SmallVec, smallvec};

use crate::export::ExportLookup;
use crate::resolve::state::{PathReference, ResolveState};
use crate::{CompilerError, CompilerResult};

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
        let PathReference { source, path } = reference;
        let segments = path.segments;
        let (root, tail) = segments
            .split_first()
            .ok_or_else(|| CompilerError::Internal {
                message: format!("reference {source:?} has an empty path"),
            })?;

        let (root, declarations) = self.resolve_path_root(source.local_id, *root)?;
        let mut prefixes: SmallVec<[dir::Reference; 4]> = smallvec![root.clone()];

        // walk exports only while the running prefix is a namespace
        if let Some(mut module) = self.namespace_module(&root) {
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
                        let targets = targets.into_iter().map(dir::ImportTarget::from).collect();
                        prefixes.push(dir::Reference::Ambiguous(targets));
                        break;
                    }

                    // stop at a missing export
                    ExportLookup::Missing => {
                        prefixes.push(dir::Reference::Missing);
                        break;
                    }
                }
            }
        }

        // record flat references unless the source is an expression member chain
        if source.local_id.ty != dir::NodeType::Expression {
            self.record_flat_reference(source, &prefixes, segments.len(), &declarations)?;
        } else {
            self.record_chain_references(source, &prefixes, &declarations)?;
        }

        Ok(())
    }

    /// Resolve the first segment of one source path.
    fn resolve_path_root(
        &self,
        source: dir::LocalNodeIdAny,
        root: dir::StringId,
    ) -> CompilerResult<(dir::Reference, SmallVec<[dir::GlobalSymbolId; 2]>)> {
        let key = dir::StaticKey::Name(root);
        let symbols = self.visible_symbols(source, key);
        if !symbols.is_empty() {
            let target = self.reference_from_symbols(&symbols)?;
            let has_import = symbols.iter().any(|symbol| {
                symbol.module_id == self.module
                    && self.bindings.get_symbol(symbol.local_id).kind == dir::SymbolKind::Import
            });
            let declarations = if has_import { symbols } else { SmallVec::new() };

            return Ok((target, declarations));
        }

        let Some(targets) = self.imports.global_targets(key) else {
            return Ok((dir::Reference::Missing, SmallVec::new()));
        };

        let target = dir::Reference::from_targets(targets.iter().copied());

        Ok((target, SmallVec::new()))
    }

    /// Return the semantic target of one visible declaration set.
    fn reference_from_symbols(
        &self,
        symbols: &[dir::GlobalSymbolId],
    ) -> CompilerResult<dir::Reference> {
        let mut targets = SmallVec::<[dir::ImportTarget; 2]>::new();
        let mut has_failed_import = false;

        // follow local import declarations to their resolved targets
        for symbol in symbols {
            let is_local_import = symbol.module_id == self.module
                && self.bindings.get_symbol(symbol.local_id).kind == dir::SymbolKind::Import;

            // follow the complete import binding resolution
            if is_local_import {
                let resolution =
                    self.imports
                        .symbol_resolution(symbol.local_id)
                        .ok_or_else(|| CompilerError::Internal {
                            message: format!("import symbol {symbol:?} has no resolution"),
                        })?;
                has_failed_import |= !matches!(resolution, dir::ImportResolution::Resolved(_));
                targets.extend(resolution.targets());
            }
            // retain regular declarations directly
            else {
                targets.push(dir::ImportTarget::Symbol(*symbol));
            }
        }

        // preserve explicit conflicts after removing duplicate targets
        let target = dir::Reference::from_targets(targets);
        match target {
            dir::Reference::Bound(symbols) if has_failed_import => {
                let targets = symbols.into_iter().map(dir::ImportTarget::Symbol).collect();

                Ok(dir::Reference::Ambiguous(targets))
            }
            dir::Reference::Namespace(module) if has_failed_import => {
                Ok(dir::Reference::Ambiguous(smallvec![
                    dir::ImportTarget::Namespace(module)
                ]))
            }
            target => Ok(target),
        }
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

    /// Record the single reference carried by one flat type reference node.
    ///
    /// Retain a resolved prefix when later segments require member selection.
    fn record_flat_reference(
        &mut self,
        source: dir::GlobalNodeIdAny,
        prefixes: &[dir::Reference],
        segment_count: usize,
        declarations: &[dir::GlobalSymbolId],
    ) -> CompilerResult<()> {
        let final_prefix = prefixes.last().ok_or_else(|| CompilerError::Internal {
            message: format!("reference {source:?} has no resolved prefix"),
        })?;
        let is_complete =
            prefixes.len() == segment_count && !matches!(final_prefix, dir::Reference::Missing);

        // retain the nearest resolved prefix before an unresolved tail
        let mut projection = None;
        if !is_complete && !matches!(final_prefix, dir::Reference::Ambiguous(_)) {
            for (index, prefix) in prefixes.iter().enumerate().rev() {
                if index + 1 >= segment_count {
                    continue;
                }
                let targets = match prefix {
                    dir::Reference::Bound(symbols) => symbols
                        .iter()
                        .copied()
                        .map(dir::ImportTarget::Symbol)
                        .collect::<SmallVec<[_; 2]>>(),
                    dir::Reference::Namespace(module) => {
                        smallvec![dir::ImportTarget::Namespace(*module)]
                    }
                    _ => continue,
                };
                projection = Some((targets, index as u32 + 1));

                break;
            }
        }

        // project only one exact base and preserve multiple bases as ambiguous
        let reference = match projection {
            Some((targets, from)) => match targets.as_slice() {
                [base] => dir::Reference::Projected { base: *base, from },
                _ => dir::Reference::Ambiguous(targets),
            },
            None => final_prefix.clone(),
        };

        self.references.insert(source, reference);
        self.references
            .insert_declarations(source, declarations.iter().copied());

        Ok(())
    }

    /// Record one reference per member node in a value path chain.
    fn record_chain_references(
        &mut self,
        source: dir::GlobalNodeIdAny,
        prefixes: &[dir::Reference],
        declarations: &[dir::GlobalSymbolId],
    ) -> CompilerResult<()> {
        // collect the member nodes from root to leaf, aligned with the segments
        let leaf = dir::LocalNodeId::<dir::Expression>::new(source.local_id.id);
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

        // require every resolved prefix to retain its authored expression node
        if nodes.len() < prefixes.len() {
            return Err(CompilerError::Internal {
                message: format!(
                    "reference {source:?} has {} resolved prefixes but only {} expression nodes",
                    prefixes.len(),
                    nodes.len()
                ),
            });
        }

        // attach import declarations to the root identifier occurrence
        let root = nodes.first().ok_or_else(|| CompilerError::Internal {
            message: format!("reference {source:?} has no expression nodes"),
        })?;
        self.references.insert_declarations(
            (*root).into_global_any(self.module),
            declarations.iter().copied(),
        );

        // place each resolved prefix on its member node, leaving type-directed tails unrecorded
        for (node, reference) in nodes.iter().zip(prefixes) {
            self.references
                .insert(node.into_global_any(self.module), reference.clone());
        }

        Ok(())
    }

    /// Return the namespace module one local namespace-import symbol selects.
    fn local_namespace_symbol_module(&self, symbol: dir::GlobalSymbolId) -> Option<ModuleId> {
        if symbol.module_id != self.module {
            return None;
        }

        match self.imports.symbol_resolution(symbol.local_id) {
            Some(dir::ImportResolution::Resolved(dir::ImportTarget::Namespace(module))) => {
                Some(*module)
            }
            Some(
                dir::ImportResolution::Resolved(dir::ImportTarget::Symbol(_))
                | dir::ImportResolution::Ambiguous(_)
                | dir::ImportResolution::Missing,
            )
            | None => None,
        }
    }
}
