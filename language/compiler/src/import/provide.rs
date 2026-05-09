use crate::{Compiler, CompilerError, CompilerResult};
use destack_artifact::{ArtifactKey, ArtifactPayload, DirDeclared};
use destack_dir::{
    BindingTable, ExportKind, Expression, LocalNodeIdAny, LocalScopeMark, NodeType, ScopeKind,
    SymbolBinding, SymbolForm, SymbolRole, SymbolSpace, Tree, TypeLiteral, TypeTable,
};
use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext, ProviderError};

impl Compiler {
    /// Build declared DIR for one module profile.
    pub(crate) fn provide_dir_declared(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let revision = context.revision();
        let artifact_key = ArtifactKey::dir_declared(module, profile);
        let ast = self.ast(context, module).map_err(CompilerError::from)?;

        // build one transient declared DIR from the current AST
        let (
            ast,
            mut tree,
            mut symbols,
            mut types,
            mut roots,
            module_node,
            namespace_symbol,
            namespace_scope,
            global_scope,
            default_symbol,
            export_assignment_symbol,
            mut declared_modules,
        ) = {
            let module_handle = self.module(revision, module);
            let module_guard = module_handle.as_ref();
            let module_expression = ast.anchor_expression;
            let default_symbol_kind = if module_guard.is_code() {
                SymbolRole::Namespace
            } else {
                SymbolRole::Item
            };

            // set up the namespace, scopes, and module symbols
            let mut symbols = BindingTable::new(module);
            let namespace_scope = symbols.insert_scope(ScopeKind::Module, None, None);
            let global_scope = symbols.insert_scope(
                ScopeKind::Namespace,
                Some((namespace_scope, LocalScopeMark::end())),
                None,
            );
            let (namespace_symbol, _) = symbols.insert_symbol(
                SymbolRole::Namespace,
                SymbolForm::Variable,
                SymbolSpace::Value,
                SymbolBinding::Runtime,
                None,
                (namespace_scope, LocalScopeMark::end()),
                Some(ExportKind::Named),
            );
            symbols.get_scope_by_id_mut(namespace_scope).owner = Some(namespace_symbol);
            let (default_symbol, _) = symbols.insert_symbol(
                default_symbol_kind,
                SymbolForm::Variable,
                SymbolSpace::Value,
                SymbolBinding::Runtime,
                None,
                (namespace_scope, LocalScopeMark::end()),
                Some(ExportKind::Default),
            );
            let (export_assignment_symbol, _) = symbols.insert_symbol(
                SymbolRole::Namespace,
                SymbolForm::Variable,
                SymbolSpace::Value,
                SymbolBinding::Runtime,
                None,
                (namespace_scope, LocalScopeMark::end()),
                None,
            );

            // seed the local mutable tables
            let mut tree = Tree::new(module);
            let module_node =
                self.create_module_node(&mut tree, namespace_scope, module_expression.id);

            (
                ast,
                tree,
                symbols,
                TypeTable::new(module),
                Vec::new(),
                module_node,
                namespace_symbol,
                namespace_scope,
                global_scope,
                default_symbol,
                export_assignment_symbol,
                Vec::new(),
            )
        };

        // run bind and desugar on the transient base DIR
        self.import_module_bind(
            module,
            &ast,
            namespace_scope,
            global_scope,
            &mut declared_modules,
            &mut tree,
            &mut symbols,
            &mut types,
            &mut roots,
            context,
        )?;
        self.import_module_desugar(module, &mut tree, context)?;

        let dir = DirDeclared {
            tree,
            bindings: symbols,
            types,
            roots,
            module_node,
            namespace_symbol,
            namespace_scope,
            default_symbol,
            export_assignment_symbol,
            export_assignment: None,
            declared_modules,
        };

        assert_eq!(
            artifact_key,
            context.artifact_key(),
            "compiler attempted to provide the wrong artifact"
        );

        Ok(ArtifactPayload::DirDeclared(dir))
    }

    /// Ensure a module AST exists.
    pub fn require_ast(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
    ) -> Result<(), ProviderError> {
        context.require(ArtifactKey::ast(module))?;

        Ok(())
    }

    /// Create a stable module-level DIR node.
    fn create_module_node(
        &self,
        tree: &mut Tree,
        namespace_scope: destack_dir::LocalScopeId,
        module_source_id: u32,
    ) -> LocalNodeIdAny {
        // module nodes should always point to a real AST id
        let scope = (namespace_scope, LocalScopeMark::end());
        let module_slot =
            tree.reserve_from_source(NodeType::Expression, module_source_id, scope, None);
        let expression = Expression::TypeLiteral {
            value: TypeLiteral::Void,
        };

        tree.insert(module_slot, expression).into_any()
    }
}
