use std::collections::HashMap;
use std::path::PathBuf;

use destack_core::StringId;
use destack_dir as dir;
use destack_source::{ModuleId, PathExt};

use crate::ModuleQueryContext;

/// Builder for one specifier index from checked DIR.
pub(super) struct SpecifierIndexer<'context, 'query> {
    /// The indexed module context.
    module: &'context ModuleQueryContext<'query>,
    /// The collected index entries.
    entries: Vec<dir::SpecifierEntry>,
    /// The semantic target module for each resolved specifier expression.
    target_modules: HashMap<u32, ModuleId>,
}

/// One syntactic module specifier.
#[derive(Debug, Clone, Copy)]
struct ModuleSpecifier {
    /// The specifier text id.
    text: StringId,
    /// The specifier kind.
    kind: dir::SpecifierKind,
}

impl<'context, 'query> SpecifierIndexer<'context, 'query> {
    /// Build the specifier index.
    pub(super) fn build(module: &'context ModuleQueryContext<'query>) -> dir::SpecifierIndex {
        let mut indexer = Self {
            module,
            entries: Vec::new(),
            target_modules: HashMap::new(),
        };

        // collect semantic targets before syntactic specifier rows
        indexer.collect_target_modules();
        indexer.collect_specifiers();

        dir::SpecifierIndex::new(indexer.entries)
    }

    /// Collect semantic targets for resolved module specifiers.
    fn collect_target_modules(&mut self) {
        let view = self.module.view();
        let module_id = self.module.module_id();

        // scan import and re-export expressions with checked module edges
        for (expression_id, expression) in view.iter_nodes_of_type::<dir::Expression>() {
            let target_module = match expression {
                dir::Expression::Import { .. } => {
                    let node_id = expression_id.into_global_any(module_id);

                    self.module
                        .modules()
                        .target_for_source(node_id, dir::ModuleRelation::Import)
                }
                dir::Expression::Export { .. } => {
                    let node_id = expression_id.into_global_any(module_id);

                    self.module
                        .modules()
                        .target_for_source(node_id, dir::ModuleRelation::ReExport)
                }
                _ => None,
            };

            // skip expressions without checked module edges
            let Some(target_module) = target_module else {
                continue;
            };

            // remember semantic target by local expression id
            self.target_modules.insert(expression_id.id, target_module);
        }
    }

    /// Collect syntactic module specifier entries.
    fn collect_specifiers(&mut self) {
        let module_id = self.module.module_id();

        // index each syntactic module specifier with its semantic target when known
        for expression_id in self.module.tree().iter_nodes::<dir::Expression>() {
            let expression = self.module.tree().get(expression_id);

            // skip expressions without module specifier text
            let Some(specifier) = Self::module_specifier(expression) else {
                continue;
            };

            // resolve source metadata and semantic target
            let text = self.module.strings().get(specifier.text).to_string();
            let source = expression_id.into_global_any(module_id);
            let span = self
                .module
                .get_main_span(self.module.view(), expression_id.into());
            let target_module = self.target_modules.get(&expression_id.id).copied();
            let target_path =
                target_module.and_then(|target_module| self.target_path(target_module));

            // emit specifier row
            self.entries.push(dir::SpecifierEntry {
                file: self.module.file_id(),
                source,
                span,
                kind: specifier.kind,
                text,
                target_module,
                target_path,
            });
        }
    }

    /// Return the normalized source path for one target module.
    fn target_path(&self, target_module: ModuleId) -> Option<PathBuf> {
        let module = self
            .module
            .repository()
            .module(self.module.revision(), target_module)
            .unwrap_or_else(|error| {
                panic!("failed to read resolved target module {target_module:?}: {error}")
            })
            .unwrap_or_else(|| panic!("missing resolved target module {target_module:?}"));

        module.path.as_ref().map(|path| path.normalize())
    }

    /// Return the module specifier text and kind for one expression.
    fn module_specifier(expression: &dir::Expression) -> Option<ModuleSpecifier> {
        match expression {
            dir::Expression::Import { target, .. } => Some(ModuleSpecifier {
                text: *target,
                kind: dir::SpecifierKind::Import,
            }),
            dir::Expression::Export {
                target: Some(target),
                ..
            } => Some(ModuleSpecifier {
                text: *target,
                kind: dir::SpecifierKind::Export,
            }),
            _ => None,
        }
    }
}
