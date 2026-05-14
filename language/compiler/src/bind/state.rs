use destack_artifact::{DirBound, DirParsed};
use destack_dir as dir;
use destack_workspace::Module;

use crate::Compiler;

/// Symbol context applied while binding declaration patterns.
#[derive(Debug, Clone, Copy)]
pub(in crate::bind) struct BindingContext {
    /// The export attached to introduced symbols.
    pub(in crate::bind) export: Option<dir::ExportKind>,
    /// The binding mode for introduced symbols.
    pub(in crate::bind) binding: dir::SymbolBinding,
    /// The mutability attached to introduced value symbols.
    pub(in crate::bind) mutability: Option<dir::Mutability>,
    /// The declared type attached to introduced value symbols.
    pub(in crate::bind) declared_type: Option<dir::LocalNodeId<dir::TypeExpression>>,
}

/// State for one bind phase provider run.
pub(in crate::bind) struct BindState<'a> {
    /// The compiler running the bind pass.
    pub(in crate::bind) compiler: &'a Compiler,
    /// The module being bound.
    pub(in crate::bind) module: &'a Module,
    /// The parsed DIR artifact.
    pub(in crate::bind) parsed: &'a DirParsed,

    /// The visitor options.
    pub(in crate::bind) options: dir::NodeVisitorOptions,
    /// The lexical scope stack.
    pub(in crate::bind) scope_stack: Vec<dir::LocalScopeId>,
    /// The active symbol origin stack.
    pub(in crate::bind) origin_stack: Vec<dir::SymbolOrigin>,
    /// The active binding context stack.
    pub(in crate::bind) binding_stack: Vec<BindingContext>,

    /// The binding table being built.
    pub(in crate::bind) bindings: dir::BindingTable,
    /// The type table being initialized.
    pub(in crate::bind) types: dir::TypeTable,
    /// The bound module roots.
    pub(in crate::bind) roots: Vec<dir::LocalNodeId<dir::Expression>>,
    /// The module namespace scope.
    pub(in crate::bind) namespace_scope: dir::LocalScopeId,
    /// The ambient global block scope.
    pub(in crate::bind) global_scope: dir::LocalScopeId,
}

impl<'a> BindState<'a> {
    /// Create bind state for one parsed DIR module.
    pub(in crate::bind) fn new(
        compiler: &'a Compiler,
        module: &'a Module,
        parsed: &'a DirParsed,
    ) -> Self {
        // create root scopes
        let mut bindings = dir::BindingTable::new(module.id);
        let namespace_scope = bindings.insert_scope(dir::ScopeKind::Module, None, None);
        let namespace = dir::LocalScope::new(namespace_scope, dir::LocalScopeMark::end());
        let global_scope = bindings.insert_scope(dir::ScopeKind::Namespace, Some(namespace), None);

        // create namespace owner
        let (namespace_symbol, _) = bindings.insert_symbol(
            dir::SymbolRole::Namespace,
            dir::SymbolForm::Variable,
            dir::SymbolSpace::Value,
            dir::SymbolBinding::Runtime,
            None,
            namespace,
            None,
        );
        bindings.get_scope_by_id_mut(namespace_scope).owner = Some(namespace_symbol);

        Self {
            compiler,
            module,
            parsed,
            options: dir::NodeVisitorOptions::default(),
            scope_stack: vec![namespace_scope],
            origin_stack: vec![dir::SymbolOrigin::Module],
            binding_stack: vec![BindingContext {
                export: None,
                binding: dir::SymbolBinding::Runtime,
                mutability: None,
                declared_type: None,
            }],
            bindings,
            types: dir::TypeTable::new(module.id),
            roots: Vec::new(),
            namespace_scope,
            global_scope,
        }
    }

    /// Record one bound root expression.
    pub(in crate::bind) fn push_root(&mut self, root: dir::LocalNodeId<dir::Expression>) {
        self.roots.push(root);
    }

    /// Finish the bound DIR artifact.
    pub(in crate::bind) fn finish(self) -> DirBound {
        DirBound {
            bindings: self.bindings,
            types: self.types,
            roots: self.roots,
            module_node: self.parsed.anchor_expression.into_any(),
            namespace_scope: self.namespace_scope,
        }
    }

    /// Return the current lexical scope.
    pub(in crate::bind) fn scope(&self) -> dir::LocalScope {
        let scope_id = *self
            .scope_stack
            .last()
            .unwrap_or_else(|| panic!("bind scope stack is empty"));
        let mark = self.bindings.get_scope_mark(scope_id);

        dir::LocalScope::new(scope_id, mark)
    }

    /// Bind one node to the current lexical scope.
    pub(in crate::bind) fn bind_node(&mut self, node_id: dir::LocalNodeIdAny) {
        self.bindings.bind_scope_any(node_id, self.scope());
    }

    /// Insert one child scope below the current lexical scope.
    pub(in crate::bind) fn insert_child_scope(
        &mut self,
        kind: dir::ScopeKind,
    ) -> dir::LocalScopeId {
        self.bindings.insert_scope(kind, Some(self.scope()), None)
    }

    /// Bind one erased node to a scope at its full mark.
    pub(in crate::bind) fn bind_node_to_scope(
        &mut self,
        node_id: dir::LocalNodeIdAny,
        scope_id: dir::LocalScopeId,
    ) {
        let scope = dir::LocalScope::new(scope_id, dir::LocalScopeMark::end());

        self.bindings.bind_scope_any(node_id, scope);
    }

    /// Push one scope while visiting children.
    pub(in crate::bind) fn push_scope(&mut self, scope_id: dir::LocalScopeId) {
        self.scope_stack.push(scope_id);
    }

    /// Pop one child scope after visiting children.
    pub(in crate::bind) fn pop_scope(&mut self) {
        self.scope_stack
            .pop()
            .unwrap_or_else(|| panic!("bind scope stack underflow"));
    }

    /// Push one symbol origin while visiting children.
    pub(in crate::bind) fn push_origin(&mut self, origin: dir::SymbolOrigin) {
        self.origin_stack.push(origin);
    }

    /// Pop one symbol origin after visiting children.
    pub(in crate::bind) fn pop_origin(&mut self) {
        self.origin_stack
            .pop()
            .unwrap_or_else(|| panic!("bind origin stack underflow"));
    }

    /// Return the current symbol origin.
    pub(in crate::bind) fn origin(&self) -> dir::SymbolOrigin {
        *self
            .origin_stack
            .last()
            .unwrap_or_else(|| panic!("bind origin stack is empty"))
    }

    /// Push one binding context while visiting a pattern subtree.
    pub(in crate::bind) fn push_binding(&mut self, binding: BindingContext) {
        self.binding_stack.push(binding);
    }

    /// Pop one binding context after visiting a pattern subtree.
    pub(in crate::bind) fn pop_binding(&mut self) {
        self.binding_stack
            .pop()
            .unwrap_or_else(|| panic!("bind binding stack underflow"));
    }

    /// Return the current binding context.
    pub(in crate::bind) fn binding(&self) -> BindingContext {
        *self
            .binding_stack
            .last()
            .unwrap_or_else(|| panic!("bind binding stack is empty"))
    }

    /// Insert one symbol in the current lexical scope.
    pub(in crate::bind) fn insert_symbol(
        &mut self,
        role: dir::SymbolRole,
        form: dir::SymbolForm,
        space: dir::SymbolSpace,
        binding: dir::SymbolBinding,
        key: Option<dir::StaticKey>,
        export: Option<dir::ExportKind>,
    ) -> dir::LocalSymbolId {
        let symbol_id = self
            .bindings
            .insert_symbol(role, form, space, binding, key, self.scope(), export)
            .0;
        self.bindings.get_symbol_mut(symbol_id).origin = self.origin();

        symbol_id
    }

    /// Insert one symbol with an owned child scope.
    pub(in crate::bind) fn insert_symbol_with_scope(
        &mut self,
        role: dir::SymbolRole,
        form: dir::SymbolForm,
        space: dir::SymbolSpace,
        binding: dir::SymbolBinding,
        key: Option<dir::StaticKey>,
        export: Option<dir::ExportKind>,
        scope_kind: dir::ScopeKind,
    ) -> (dir::LocalSymbolId, dir::LocalScopeId) {
        let symbol_id = self.insert_symbol(role, form, space, binding, key, export);
        let scope_id = self
            .bindings
            .insert_scope(scope_kind, Some(self.scope()), Some(symbol_id));

        (symbol_id, scope_id)
    }

    /// Bind one declared node symbol.
    pub(in crate::bind) fn declare_symbol<T: dir::Node>(
        &mut self,
        symbol_id: dir::LocalSymbolId,
        node_id: dir::LocalNodeId<T>,
    ) {
        self.bindings.declare_symbol(symbol_id, node_id);
    }

    /// Attach binding mutability to one symbol.
    pub(in crate::bind) fn set_binding_mutability(
        &mut self,
        symbol_id: dir::LocalSymbolId,
        mutability: Option<dir::Mutability>,
    ) {
        // ignore unqualified bindings
        let Some(mutability) = mutability else {
            return;
        };

        self.bindings.get_symbol_mut(symbol_id).binding_mutability = Some(mutability);
    }

    /// Attach one declared type expression to one node.
    pub(in crate::bind) fn set_declared_type(
        &mut self,
        node_id: dir::LocalNodeIdAny,
        declared_type: Option<dir::LocalNodeId<dir::TypeExpression>>,
    ) {
        // ignore untyped declarations
        let Some(declared_type) = declared_type else {
            return;
        };

        // create unevaluated type placeholder
        let ty = self.types.insert_type_from(
            dir::Type::Unevaluated(dir::UnevaluatedType {
                expression: declared_type,
            }),
            declared_type,
        );
        let node_id = node_id.into_global(self.module.id);

        self.types.set_declared_type(node_id, ty);
    }

    /// Return the binding mode for one ambient flag.
    pub(in crate::bind) fn binding_for_declaration(&self, is_ambient: bool) -> dir::SymbolBinding {
        // declarations are ambient by module or declaration
        if self.module.is_declaration() || is_ambient {
            dir::SymbolBinding::Ambient
        } else {
            dir::SymbolBinding::Runtime
        }
    }
}
