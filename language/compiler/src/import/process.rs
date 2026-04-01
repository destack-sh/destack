use std::sync::Arc;

use destack_artifact::{ArtifactKey, DirBase};
use destack_dir::{
    DependencyMode, Expression, LocalNodeIdAny, LocalScopeMark, NodeTree, NodeType, ScopeKind,
    SymbolBinding, SymbolKind, SymbolSpace, SymbolTable, SymbolType, TypeLiteral, TypeTable,
};
use destack_source::ModuleId;

use crate::{ArtifactRequirementError, Compiler, ImportError, ImportResult};

impl Compiler {
    /// Create a stable module-level anchor node.
    fn create_base_dir_anchor(
        &self,
        tree: &mut NodeTree,
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

    /// Build the parsed syntax tree for one module.
    pub fn process_ast(&self, module: ModuleId) -> ImportResult<()> {
        let module_version = self.module_version(module);
        self.ensure_module_version_matches::<ImportError>(module, module_version)?;
        self.import_module_parse(module, module_version)?;
        self.artifacts
            .ast(module)
            .expect("missing AST on parsed module");

        Ok(())
    }

    /// Build base DIR for one module.
    pub fn process_dir_base(&self, module: ModuleId) -> ImportResult<()> {
        let module_version = self.module_version(module);
        self.ensure_module_version_matches::<ImportError>(module, module_version)?;
        self.require_ast(module)?;
        let artifact_key = ArtifactKey::dir_base(module);

        // reuse one persisted base dir image when available
        if self
            .load_published_artifact(artifact_key.clone(), |compiler| {
                compiler.load_dir_base_image(module, module_version)
            })
            .is_some()
        {
            return Ok(());
        }

        // read the committed AST artifact directly
        let ast = self.artifacts.ast(module);

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
            let module_handle = self.program.modules.get(module);
            let module_guard = module_handle.as_ref();
            self.ensure_module_version_matches_guard::<ImportError>(module_guard, module_version)?;
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
                Some(DependencyMode::Namespace),
            );
            symbols.get_scope_by_id_mut(namespace_scope).owner_id = Some(namespace_symbol);
            let (default_symbol, _) = symbols.insert_symbol(
                default_symbol_kind,
                SymbolType::Void,
                SymbolSpace::Value,
                SymbolBinding::Runtime,
                None,
                (namespace_scope, LocalScopeMark::end()),
                Some(DependencyMode::Default),
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
            let mut tree = NodeTree::new(module);
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
            module_version,
            &ast,
            namespace_scope,
            global_augmentation_scope,
            &mut module_bindings,
            &mut tree,
            &mut symbols,
            &mut types,
            &mut roots,
        )?;
        self.import_module_desugar(module, module_version, &mut tree)?;
        self.import_module_validate(
            module,
            module_version,
            &tree,
            &symbols,
            &roots,
            global_augmentation_scope,
        )?;
        if self.is_code_module(module) {
            self.stats.record_bind();
        }

        // publish the final base artifact
        let dir = DirBase {
            profile_id: None,
            id: module,
            version: module_version,
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

        self.artifacts.publish(artifact_key.clone(), dir.clone());
        self.store_artifact(&artifact_key, &dir, |compiler, dir| {
            compiler.store_dir_base_image(module, dir)
        });

        Ok(())
    }

    /// Ensure a module AST exists.
    pub fn require_ast(&self, module: ModuleId) -> Result<(), ArtifactRequirementError> {
        self.require_artifact(ArtifactKey::ast(module))
    }

    /// Ensure a module base DIR exists.
    pub fn require_dir_base(&self, module: ModuleId) -> Result<(), ArtifactRequirementError> {
        self.require_artifact(ArtifactKey::dir_base(module))
    }
}
