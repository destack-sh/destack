use smallvec::{SmallVec, smallvec};
use tspp_dir as dir;

use crate::export::ExportLookup;
use crate::resolve::state::{PathReference, ResolveState};
use crate::{CompilerError, CompilerResult};

/// One resolved source path segment.
#[derive(Debug, Clone)]
struct PathSegmentResolution {
    /// The authored declaration selected by the segment.
    declaration: dir::Reference,
    /// The final compiler target.
    target: dir::Reference,
}

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

        let root = self.resolve_path_root(source.local_id, *root)?;
        let mut prefixes: SmallVec<[PathSegmentResolution; 4]> = smallvec![root.clone()];

        // walk exports only while the running prefix is a namespace
        if let Some(mut module) = root.target.namespace() {
            for segment in tail.iter().copied() {
                let key = dir::ExportKey::named(dir::StaticKey::Name(segment));
                match self.resolve_export_target(module, key)? {
                    // retain both the public declaration and final target
                    ExportLookup::Found(resolution) => {
                        let target = dir::Reference::from(&resolution.target);
                        let declaration = dir::Reference::from(&resolution.declaration);
                        let next = resolution.target.namespace();
                        prefixes.push(PathSegmentResolution {
                            declaration,
                            target,
                        });

                        match next {
                            Some(next) => module = next,
                            None => break,
                        }
                    }

                    // stop at conflicting symbol targets
                    ExportLookup::Ambiguous(resolutions) => {
                        let resolution =
                            dir::ImportResolution::Ambiguous(resolutions.into_iter().collect());
                        prefixes.push(PathSegmentResolution {
                            declaration: resolution.declaration_reference(),
                            target: resolution.target_reference(),
                        });
                        break;
                    }

                    // stop at a missing export
                    ExportLookup::Missing => {
                        prefixes.push(PathSegmentResolution {
                            declaration: dir::Reference::Missing,
                            target: dir::Reference::Missing,
                        });
                        break;
                    }
                }
            }
        }

        // record expression paths on their individual nodes
        if source.local_id.ty == dir::NodeType::Expression {
            self.record_chain_references(source, &prefixes)?;
        }
        // record a single type name on its node
        else if segments.len() == 1 {
            self.references
                .insert_resolution(source, root.declaration, root.target);
        }
        // record qualified type names on their path segments
        else {
            self.record_qualified_reference(source, &prefixes, segments.len())?;
        }

        Ok(())
    }

    /// Resolve the first segment of one source path.
    fn resolve_path_root(
        &self,
        source: dir::LocalNodeIdAny,
        root: dir::StringId,
    ) -> CompilerResult<PathSegmentResolution> {
        let key = dir::StaticKey::Name(root);
        let symbols = self.visible_symbols(source, key);
        if !symbols.is_empty() {
            let target = self.resolve_symbols(&symbols)?;
            let has_import = symbols.iter().any(|symbol| {
                if symbol.module_id != self.module {
                    return false;
                }

                self.bindings.get_symbol(symbol.local_id).kind == dir::SymbolKind::Import
            });
            let declaration = if has_import {
                dir::Reference::from_symbols(symbols)
            } else {
                target.clone()
            };

            return Ok(PathSegmentResolution {
                declaration,
                target,
            });
        }

        let Some(resolutions) = self.imports.global_resolutions(key) else {
            // an unbound name spelling a builtin type literal resolves to it
            if let Some(literal) = dir::TypeLiteral::from_name(self.strings.get(root)) {
                return Ok(PathSegmentResolution {
                    declaration: dir::Reference::TypeLiteral(literal.clone()),
                    target: dir::Reference::TypeLiteral(literal),
                });
            }

            return Ok(PathSegmentResolution {
                declaration: dir::Reference::Missing,
                target: dir::Reference::Missing,
            });
        };
        let resolution = match resolutions {
            [resolution] => dir::ImportResolution::Resolved(resolution.clone()),
            resolutions => dir::ImportResolution::Ambiguous(resolutions.iter().cloned().collect()),
        };

        Ok(PathSegmentResolution {
            declaration: resolution.declaration_reference(),
            target: resolution.target_reference(),
        })
    }

    /// Return the final target of one visible declaration set.
    fn resolve_symbols(&self, symbols: &[dir::GlobalSymbolId]) -> CompilerResult<dir::Reference> {
        let mut targets = SmallVec::<[dir::ReferenceTarget; 2]>::new();
        let mut has_import_conflict = false;

        // follow local imports to their final targets
        for symbol in symbols {
            let is_import = symbol.module_id == self.module
                && self.bindings.get_symbol(symbol.local_id).kind == dir::SymbolKind::Import;

            // follow one imported declaration
            if is_import {
                let resolution =
                    self.imports
                        .symbol_resolution(symbol.local_id)
                        .ok_or_else(|| CompilerError::Internal {
                            message: format!("import symbol {symbol:?} has no resolution"),
                        })?;
                has_import_conflict |= !matches!(resolution, dir::ImportResolution::Resolved(_));
                targets.extend(resolution.targets());
            }
            // retain an ordinary declaration
            else {
                targets.push(dir::ReferenceTarget::Symbol(*symbol));
            }
        }

        // preserve explicit conflicts after removing duplicate targets
        let target = dir::Reference::from_targets(targets);
        match target {
            dir::Reference::Bound(symbols) if has_import_conflict => {
                let targets = symbols
                    .into_iter()
                    .map(dir::ReferenceTarget::Symbol)
                    .collect();

                Ok(dir::Reference::Ambiguous(targets))
            }
            dir::Reference::Namespace { module, .. } if has_import_conflict => {
                Ok(dir::Reference::Ambiguous(smallvec![
                    dir::ReferenceTarget::Namespace(module)
                ]))
            }
            target => Ok(target),
        }
    }

    /// Record the segments and complete target of a qualified type path.
    fn record_qualified_reference(
        &mut self,
        source: dir::GlobalNodeIdAny,
        prefixes: &[PathSegmentResolution],
        segment_count: usize,
    ) -> CompilerResult<()> {
        // require a resolved prefix
        let final_prefix = prefixes.last().ok_or_else(|| CompilerError::Internal {
            message: format!("reference {source:?} has no resolved prefix"),
        })?;

        // record qualified names on their path segments
        for (segment, resolution) in prefixes.iter().enumerate() {
            let segment = u16::try_from(segment).map_err(|_| CompilerError::Internal {
                message: format!("reference {source:?} has too many path segments"),
            })?;
            let site = dir::ReferenceSite::Path {
                node: source,
                segment,
            };
            self.references.insert_resolution(
                site,
                resolution.declaration.clone(),
                resolution.target.clone(),
            );
        }

        // identify paths whose final target remains type dependent
        let is_complete = prefixes.len() == segment_count
            && !matches!(final_prefix.target, dir::Reference::Missing);

        // retain the nearest resolved prefix before an unresolved tail
        let mut projection = None;
        if !is_complete && !matches!(final_prefix.target, dir::Reference::Ambiguous(_)) {
            for (index, prefix) in prefixes.iter().enumerate().rev() {
                if index + 1 >= segment_count {
                    continue;
                }
                let targets = match &prefix.target {
                    dir::Reference::Bound(symbols) => symbols
                        .iter()
                        .copied()
                        .map(dir::ReferenceTarget::Symbol)
                        .collect::<SmallVec<[_; 2]>>(),
                    dir::Reference::Namespace { module, .. } => {
                        smallvec![dir::ReferenceTarget::Namespace(*module)]
                    }
                    _ => continue,
                };
                projection = Some((targets, index as u32 + 1));

                break;
            }
        }

        // project only one exact base and preserve multiple bases as ambiguous
        let target = match projection {
            Some((targets, from)) => match targets.as_slice() {
                [base] => dir::Reference::Projected { base: *base, from },
                _ => dir::Reference::Ambiguous(targets),
            },
            None => final_prefix.target.clone(),
        };

        // record the complete path's target on its node
        self.references
            .insert_resolution(source, target.clone(), target);

        Ok(())
    }

    /// Record one reference per member node in a value path chain.
    fn record_chain_references(
        &mut self,
        source: dir::GlobalNodeIdAny,
        prefixes: &[PathSegmentResolution],
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

        // place each resolved prefix on its member node, leaving type-directed tails unrecorded
        for (node, resolution) in nodes.iter().zip(prefixes) {
            let source = node.into_global_any(self.module);
            self.references.insert_resolution(
                source,
                resolution.declaration.clone(),
                resolution.target.clone(),
            );
        }

        Ok(())
    }
}
