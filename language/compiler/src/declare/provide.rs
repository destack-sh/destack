use destack_artifact::{ArtifactKey, ArtifactPayload, DirDeclared};
use destack_dir::{
    BindingTable, ExportKind, Expression, LocalNodeIdAny, LocalScopeMark, NodeType, ScopeKind,
    SymbolBinding, SymbolForm, SymbolRole, SymbolSpace, Tree, TypeLiteral, TypeTable,
};
use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

use crate::declare::DeclareState;
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Build declared DIR for one module profile.
    pub(crate) fn provide_dir_declared(
        &self,
        module: ModuleId,
        _profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let state = DeclareState::new(module, context);
        let revision = state.context.revision();
        state
            .context
            .require(ArtifactKey::ast(state.module))
            .map_err(CompilerError::from)?;
        let ast = self
            .ast(state.context, state.module)
            .map_err(CompilerError::from)?;

        let module_handle = self.module(revision, state.module);
        let module_guard = module_handle.as_ref();
        let module_expression = ast.anchor_expression;
        let default_symbol_kind = if module_guard.is_code() {
            SymbolRole::Namespace
        } else {
            SymbolRole::Item
        };

        // module scopes
        let mut bindings = BindingTable::new(state.module);
        let namespace_scope = bindings.insert_scope(ScopeKind::Module, None, None);
        let global_scope = bindings.insert_scope(
            ScopeKind::Namespace,
            Some((namespace_scope, LocalScopeMark::end())),
            None,
        );

        // module symbols
        let namespace_binding = (namespace_scope, LocalScopeMark::end());
        let (namespace_symbol, _) = bindings.insert_symbol(
            SymbolRole::Namespace,
            SymbolForm::Variable,
            SymbolSpace::Value,
            SymbolBinding::Runtime,
            None,
            namespace_binding,
            Some(ExportKind::Named),
        );
        bindings.get_scope_by_id_mut(namespace_scope).owner = Some(namespace_symbol);

        let (default_symbol, _) = bindings.insert_symbol(
            default_symbol_kind,
            SymbolForm::Variable,
            SymbolSpace::Value,
            SymbolBinding::Runtime,
            None,
            namespace_binding,
            Some(ExportKind::Default),
        );
        let (export_assignment_symbol, _) = bindings.insert_symbol(
            SymbolRole::Namespace,
            SymbolForm::Variable,
            SymbolSpace::Value,
            SymbolBinding::Runtime,
            None,
            namespace_binding,
            None,
        );

        // declared DIR tables
        let mut tree = Tree::new(state.module);
        let mut types = TypeTable::new(state.module);
        let mut roots = Vec::new();
        let mut declared_modules = Vec::new();
        let module_node = self.create_module_node(&mut tree, namespace_scope, module_expression.id);

        // binding pass
        self.declare_module_bind(
            state.module,
            &ast,
            namespace_scope,
            global_scope,
            &mut declared_modules,
            &mut tree,
            &mut bindings,
            &mut types,
            &mut roots,
            state.context,
        )?;

        // syntax normalization
        self.declare_module_normalize(state.module, &mut tree, state.context)?;

        let dir = DirDeclared {
            tree,
            bindings,
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

        Ok(ArtifactPayload::DirDeclared(dir))
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
