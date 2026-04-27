use std::sync::Arc;

use crate::{Compiler, CompilerContext, ImportError, ImportResult, RequirementError};
use destack_artifact::{ArtifactKey, DirBase};
use destack_dir::{
    ExportMode, Expression, LocalNodeIdAny, LocalScopeMark, NodeType, ScopeKind, SymbolBinding,
    SymbolKind, SymbolSpace, SymbolTable, SymbolType, Tree, TypeLiteral, TypeTable,
};
use destack_source::{FileId, ModuleId};

impl CompilerContext<'_> {
    /// Load one module for import work.
    pub(crate) fn import_module(
        &self,
        module_id: ModuleId,
    ) -> Result<std::sync::Arc<destack_workspace::Module>, ImportError> {
        Ok(self.module(module_id))
    }

    /// Load one file for import work.
    pub(crate) fn import_file(
        &self,
        file_id: FileId,
    ) -> Result<std::sync::Arc<destack_source::File>, ImportError> {
        Ok(self.file(file_id))
    }
}

impl Compiler {
    /// Build the parsed syntax tree for one module.
    pub fn process_ast(&self, module: ModuleId, context: &CompilerContext<'_>) -> ImportResult<()> {
        let artifact_key = ArtifactKey::ast(module);
        let artifact_stamp = context.artifact_stamp(&artifact_key);
        self.import_module_parse(module, artifact_stamp, context)?;
        context.ast(module).expect("missing AST on parsed module");

        Ok(())
    }

    /// Build base DIR for one module.
    pub fn process_dir_base(
        &self,
        module: ModuleId,
        context: &CompilerContext<'_>,
    ) -> ImportResult<()> {
        let revision = context.revision();
        let artifact_key = ArtifactKey::dir_base(module);
        let artifact_stamp = context.artifact_stamp(&artifact_key);
        self.require_ast(revision, module)?;

        // reuse one persisted base dir image when available
        if context
            .restore_cached_artifact(
                artifact_key,
                |compiler| compiler.load_dir_base_image(revision, module, artifact_stamp),
                |store, version, payload| store.publish_dir_base(version, payload),
            )
            .is_some()
        {
            return Ok(());
        }

        // read the committed AST artifact directly
        let ast = context.ast(module);

        // build one transient base DIR from the current AST
        let (
            ast,
            mut tree,
            mut symbols,
            mut types,
            mut roots,
            anchor_node,
            namespace_symbol,
            namespace_scope,
            global_augmentation_scope,
            default_symbol,
            export_assignment_symbol,
            mut module_bindings,
        ) = {
            let module_handle = context.import_module(module)?;
            let module_guard = module_handle.as_ref();
            let ast = ast.expect("missing AST on parsed module");
            let anchor_id = ast
                .anchor_expression
                .expect("missing anchor expression on parsed module");
            let default_symbol_kind = if module_guard.is_code() {
                SymbolKind::Namespace
            } else {
                SymbolKind::Item
            };

            // set up the namespace, scopes, and module symbols
            let mut symbols = SymbolTable::new(module);
            let namespace_scope = symbols.insert_scope(ScopeKind::Namespace, None, None);
            let global_augmentation_scope = symbols.insert_scope(
                ScopeKind::Namespace,
                Some((namespace_scope, LocalScopeMark::end())),
                None,
            );
            let (namespace_symbol, _) = symbols.insert_symbol(
                SymbolKind::Namespace,
                SymbolType::Void,
                SymbolSpace::Value,
                SymbolBinding::Runtime,
                None,
                (namespace_scope, LocalScopeMark::end()),
                Some(ExportMode::Named),
            );
            symbols.get_scope_by_id_mut(namespace_scope).owner_id = Some(namespace_symbol);
            let (default_symbol, _) = symbols.insert_symbol(
                default_symbol_kind,
                SymbolType::Void,
                SymbolSpace::Value,
                SymbolBinding::Runtime,
                None,
                (namespace_scope, LocalScopeMark::end()),
                Some(ExportMode::Default),
            );
            let (export_assignment_symbol, _) = symbols.insert_symbol(
                SymbolKind::Namespace,
                SymbolType::Void,
                SymbolSpace::Value,
                SymbolBinding::Runtime,
                None,
                (namespace_scope, LocalScopeMark::end()),
                None,
            );

            // seed the local mutable tables
            let mut tree = Tree::new(module);
            let anchor_node = self.create_base_dir_anchor(&mut tree, namespace_scope, anchor_id.id);

            (
                ast,
                tree,
                symbols,
                TypeTable::new(module),
                Vec::new(),
                anchor_node,
                namespace_symbol,
                namespace_scope,
                global_augmentation_scope,
                default_symbol,
                export_assignment_symbol,
                Vec::new(),
            )
        };

        // run bind, desugar, and validate on the transient base DIR
        self.import_module_bind(
            module,
            &ast,
            namespace_scope,
            global_augmentation_scope,
            &mut module_bindings,
            &mut tree,
            &mut symbols,
            &mut types,
            &mut roots,
            context,
        )?;
        self.import_module_desugar(module, &mut tree, context)?;
        self.import_module_validate(
            module,
            &tree,
            &symbols,
            &roots,
            global_augmentation_scope,
            context,
        )?;
        if context.is_code_module(module) {
            self.stats.record_bind();
        }

        // publish the final base artifact
        let dir = DirBase {
            id: module,
            tree: Arc::new(tree),
            symbols: Arc::new(symbols),
            types: Arc::new(types),
            roots: Arc::new(roots),
            anchor_node,
            namespace_symbol,
            namespace_scope,
            global_augmentation_scope,
            default_symbol,
            export_assignment_symbol,
            module_bindings: Arc::new(module_bindings),
        };

        context.publish_artifact(artifact_key, dir.clone(), |store, version, payload| {
            store.publish_dir_base(version, payload)
        });
        context.store_artifact(&artifact_key, &dir, |compiler, artifact_stamp, dir| {
            compiler.store_dir_base_image(revision, module, artifact_stamp, dir)
        });

        Ok(())
    }

    /// Ensure a module AST exists.
    pub fn require_ast(
        &self,
        revision: destack_workspace::Revision,
        module: ModuleId,
    ) -> Result<(), RequirementError> {
        self.require_artifact(revision, ArtifactKey::ast(module))
    }

    /// Ensure a module base DIR exists.
    pub fn require_dir_base(
        &self,
        revision: destack_workspace::Revision,
        module: ModuleId,
    ) -> Result<(), RequirementError> {
        self.require_artifact(revision, ArtifactKey::dir_base(module))
    }

    /// Create a stable module-level anchor node.
    fn create_base_dir_anchor(
        &self,
        tree: &mut Tree,
        namespace_scope: destack_dir::LocalScopeId,
        anchor_source_id: u32,
    ) -> LocalNodeIdAny {
        // anchor nodes should always point to a real AST id
        let scope = (namespace_scope, LocalScopeMark::end());
        let anchor_slot =
            tree.reserve_from_source(NodeType::Expression, anchor_source_id, scope, None);
        let expression = Expression::TypeLiteral {
            value: TypeLiteral::Void,
        };

        tree.insert(anchor_slot, expression).into_any()
    }
}
