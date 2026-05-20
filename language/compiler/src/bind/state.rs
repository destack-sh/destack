use std::sync::Arc;

use destack_artifact::{DirBound, DirParsed};
use destack_dir as dir;
use destack_source::ModuleId;

use crate::Compiler;

/// Symbol context applied while binding declaration patterns.
#[derive(Debug, Clone, Copy)]
pub(in crate::bind) struct BindingContext {
    /// The export attached to introduced symbols.
    pub(in crate::bind) export: Option<dir::ExportKind>,
    /// The mutability attached to introduced value symbols.
    pub(in crate::bind) mutability: Option<dir::Mutability>,
}

/// State for one bind phase provider run.
pub(in crate::bind) struct BindState<'a> {
    /// The compiler running the bind pass.
    pub(in crate::bind) compiler: &'a Compiler,
    /// The parsed DIR artifact.
    pub(in crate::bind) parsed: &'a DirParsed,

    /// The visitor options.
    pub(in crate::bind) options: dir::NodeVisitorOptions,
    /// The lexical scope stack.
    pub(in crate::bind) scope_stack: Vec<dir::LocalScopeId>,
    /// The active binding context stack.
    pub(in crate::bind) binding_stack: Vec<BindingContext>,

    /// The binding table being built.
    pub(in crate::bind) bindings: dir::BindingSegment,
    /// The type table being initialized.
    pub(in crate::bind) types: dir::TypeSegment,
    /// The static table being initialized.
    pub(in crate::bind) statics: dir::StaticSegment,
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
        module: ModuleId,
        parsed: &'a DirParsed,
    ) -> Self {
        // create root scopes
        let mut bindings = dir::BindingSegment::new(module);
        let namespace_scope = bindings.insert_scope(dir::ScopeKind::Module, None, None);
        let namespace = dir::LocalScope::new(namespace_scope, dir::LocalScopeMark::end());
        let global_scope = bindings.insert_scope(dir::ScopeKind::Global, None, None);

        // create namespace owner
        let (namespace_symbol, _) = bindings.insert_symbol(
            dir::SymbolRole::Namespace,
            dir::SymbolForm::Variable,
            None,
            namespace,
            None,
        );
        bindings.get_scope_by_id_mut(namespace_scope).owner = Some(namespace_symbol);

        Self {
            compiler,
            parsed,
            options: dir::NodeVisitorOptions::default(),
            scope_stack: vec![namespace_scope],
            binding_stack: vec![BindingContext {
                export: None,
                mutability: None,
            }],
            bindings,
            types: dir::TypeSegment::new(module),
            statics: dir::StaticSegment::new(module),
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
            bindings: Arc::new(self.bindings),
            types: Arc::new(self.types),
            statics: Arc::new(self.statics),
            roots: self.roots,
            module_node: self.parsed.anchor_expression.into_any(),
            namespace_scope: self.namespace_scope,
        }
    }

    /// Return the current lexical scope.
    pub(in crate::bind) fn scope(&self) -> dir::LocalScope {
        let scope_id = self.current_scope_id();
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

    /// Attach the global scope at the current module cursor.
    pub(in crate::bind) fn attach_global_scope(&mut self) {
        let parent = self.scope();
        let global = self.bindings.get_scope_by_id(self.global_scope);

        // keep the first global block as the visibility root
        if global.parent.is_some() {
            return;
        }

        self.bindings.get_scope_by_id_mut(self.global_scope).parent = Some(parent);
        self.bindings
            .get_scope_by_id_mut(parent.id)
            .append_child(self.global_scope);
    }

    /// Push one scope while visiting children.
    pub(in crate::bind) fn push_scope(&mut self, scope_id: dir::LocalScopeId) {
        self.scope_stack.push(scope_id);
    }

    /// Pop one child scope after visiting children.
    pub(in crate::bind) fn pop_scope(&mut self) {
        self.scope_stack.pop().expect("bind scope stack underflow");
    }

    /// Push one binding context while visiting a pattern subtree.
    pub(in crate::bind) fn push_binding(&mut self, binding: BindingContext) {
        self.binding_stack.push(binding);
    }

    /// Pop one binding context after visiting a pattern subtree.
    pub(in crate::bind) fn pop_binding(&mut self) {
        self.binding_stack
            .pop()
            .expect("bind binding stack underflow");
    }

    /// Return the current binding context.
    pub(in crate::bind) fn binding(&self) -> BindingContext {
        *self
            .binding_stack
            .last()
            .expect("bind binding stack is empty")
    }

    /// Insert one symbol in the current lexical scope.
    pub(in crate::bind) fn insert_symbol(
        &mut self,
        role: dir::SymbolRole,
        form: dir::SymbolForm,
        key: Option<dir::StaticKey>,
        export: Option<dir::ExportKind>,
    ) -> dir::LocalSymbolId {
        let scope = self.scope();
        let symbol_id = self
            .bindings
            .insert_symbol(role, form, key, scope, export)
            .0;
        if scope.id == self.global_scope {
            self.bindings.get_symbol_mut(symbol_id).origin = dir::SymbolOrigin::Global;
        }

        symbol_id
    }

    /// Insert one symbol with an owned child scope.
    pub(in crate::bind) fn insert_symbol_with_scope(
        &mut self,
        role: dir::SymbolRole,
        form: dir::SymbolForm,
        key: Option<dir::StaticKey>,
        export: Option<dir::ExportKind>,
        scope_kind: dir::ScopeKind,
    ) -> (dir::LocalSymbolId, dir::LocalScopeId) {
        let symbol_id = self.insert_symbol(role, form, key, export);
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

    /// Return the current scope id.
    fn current_scope_id(&self) -> dir::LocalScopeId {
        *self.scope_stack.last().expect("bind scope stack is empty")
    }
}
