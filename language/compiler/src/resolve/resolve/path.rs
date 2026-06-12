use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::export::{ExportLookup, ExportTarget};
use crate::resolve::state::{PathReference, ResolveState};

impl ResolveState<'_> {
    /// Resolve namespace path references collected during the resolve walk.
    ///
    /// Example:
    /// ```ds
    /// import * as dep from "./dep.ds";
    ///
    /// dep.api.value;
    /// // dep and dep.api are namespace prefixes, dep.api.value is the final symbol
    /// ```
    pub(in crate::resolve) fn resolve_path_references(&mut self) -> CompilerResult<()> {
        let references = std::mem::take(&mut self.path_references);

        for reference in references {
            self.resolve_path_reference(reference)?;
        }

        Ok(())
    }

    /// Resolve one namespace path reference.
    ///
    /// Example:
    /// ```ds
    /// dep.api.value;
    /// ```
    fn resolve_path_reference(&mut self, reference: PathReference) -> CompilerResult<()> {
        // select the root namespace
        let Some((root, tail)) = reference.path.segments.split_first() else {
            return Ok(());
        };
        let Some(mut module) = self.namespace_root_module(&reference, *root) else {
            return Ok(());
        };

        // record the root namespace
        self.paths.insert(
            dir::PathKey::new(reference.source, 1),
            dir::PathResolution::Found(dir::PathTarget::Namespace(module)),
        );

        // resolve each namespace prefix
        for (index, segment) in tail.iter().enumerate() {
            let length = index as u32 + 2;
            let key = dir::ExportKey::named(dir::StaticKey::Name(*segment));
            let resolution = self.resolve_export_target(module, key)?;
            let is_final = index + 1 == tail.len();

            match resolution {
                // record one symbol prefix
                ExportLookup::Found(ExportTarget::Symbol(symbol)) => {
                    self.paths.insert(
                        dir::PathKey::new(reference.source, length),
                        dir::PathResolution::Found(dir::PathTarget::Symbol(symbol)),
                    );
                    if is_final {
                        return Ok(());
                    }

                    // continue only when the symbol itself names another namespace
                    let Some(next) = self.local_namespace_symbol_module(symbol) else {
                        return Ok(());
                    };

                    module = next;
                }

                // record one namespace prefix
                ExportLookup::Found(ExportTarget::Namespace(next)) => {
                    self.paths.insert(
                        dir::PathKey::new(reference.source, length),
                        dir::PathResolution::Found(dir::PathTarget::Namespace(next)),
                    );

                    module = next;
                }

                // record an ambiguous prefix
                ExportLookup::Ambiguous(targets) => {
                    let targets = targets
                        .into_iter()
                        .map(|target| target.path_target())
                        .collect();

                    self.paths.insert(
                        dir::PathKey::new(reference.source, length),
                        dir::PathResolution::Ambiguous(targets),
                    );

                    return Ok(());
                }

                // record a missing prefix
                ExportLookup::Missing => {
                    self.paths.insert(
                        dir::PathKey::new(reference.source, length),
                        dir::PathResolution::Missing,
                    );

                    return Ok(());
                }
            }
        }

        Ok(())
    }

    /// Return the namespace module selected by one path root.
    ///
    /// Example:
    /// ```ds
    /// import * as dep from "./dep.ds";
    ///
    /// dep.value;
    /// // dep selects the namespace module
    /// ```
    fn namespace_root_module(
        &self,
        reference: &PathReference,
        root: dir::StringId,
    ) -> Option<ModuleId> {
        let key = dir::StaticKey::Name(root);
        let lookup = self
            .bindings
            .lookup_symbol_at(reference.source, key, dir::SymbolSpace::Value);

        // prefer lexical namespace imports
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

        // fall back when no lexical binding shadows the ambient namespace
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

    /// Return the namespace module selected by one local namespace import symbol.
    ///
    /// Example:
    /// ```ds
    /// import * as api from "./api.ds";
    ///
    /// api.value;
    /// // api selects the namespace module
    /// ```
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
