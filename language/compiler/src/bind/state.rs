use std::sync::Arc;

use destack_artifact::{DirBound, DirParsed};
use destack_dir as dir;
use destack_source::{ModuleId, StringId};
use dir::NodeVisitor as _;
use indexmap::IndexMap;

use crate::Compiler;
use crate::bind::stats::BindStats;

/// Source modifiers inherited by one binding symbol.
#[derive(Debug, Clone, Copy, Default)]
pub(in crate::bind) struct BindingModifiers {
    /// The export attached to introduced symbols.
    pub(in crate::bind) export: Option<dir::ExportKind>,
    /// The mutability attached to introduced value symbols.
    pub(in crate::bind) mutability: Option<dir::Mutability>,
    /// Whether introduced value symbols live in shared storage.
    pub(in crate::bind) is_shared: bool,
    /// The symbol kind of introduced value symbols.
    pub(in crate::bind) kind: dir::SymbolKind,
}

/// State for one bind phase provider run.
pub(in crate::bind) struct BindState<'a> {
    /// The compiler running the bind pass.
    pub(in crate::bind) compiler: &'a Compiler,
    /// The parsed DIR artifact.
    pub(in crate::bind) parsed: &'a DirParsed,

    /// The lexical scope stack.
    pub(in crate::bind) scope_stack: Vec<dir::LocalScopeId>,
    /// The conditional type scopes that hold active `infer` binders.
    pub(in crate::bind) infer_scope_stack: Vec<dir::LocalScopeId>,
    /// The active binding modifier stack.
    binding_modifier_stack: Vec<BindingModifiers>,
    /// Shared binding symbols for active union patterns.
    pub(in crate::bind) union_pattern_symbols: Vec<IndexMap<dir::StaticKey, dir::LocalSymbolId>>,
    /// Shared symbols for repeated `infer` binders per conditional scope.
    pub(in crate::bind) infer_symbols: IndexMap<(dir::LocalScopeId, StringId), dir::LocalSymbolId>,

    /// The binding table being built.
    pub(in crate::bind) bindings: dir::BindingSegment,
    /// The type table being initialized.
    pub(in crate::bind) types: dir::TypeSegment,
    /// The static table being initialized.
    pub(in crate::bind) statics: dir::StaticSegment,
    /// The work stats accumulated while binding.
    pub(in crate::bind) stats: BindStats,
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
        let namespace_symbol = bindings.insert_symbol(
            dir::SymbolRole::Namespace,
            dir::SymbolKind::Variable,
            None,
            namespace,
            None,
            dir::SymbolVisibility::Scope,
        );
        bindings.get_scope_by_id_mut(namespace_scope).owner = Some(namespace_symbol);

        Self {
            compiler,
            parsed,
            scope_stack: vec![namespace_scope],
            infer_scope_stack: Vec::new(),
            infer_symbols: IndexMap::new(),
            binding_modifier_stack: vec![BindingModifiers::default()],
            union_pattern_symbols: Vec::new(),
            bindings,
            types: dir::TypeSegment::new(module),
            statics: dir::StaticSegment::new(module),
            stats: BindStats::default(),
            roots: Vec::new(),
            namespace_scope,
            global_scope,
        }
    }

    /// Record one bound root expression.
    pub(in crate::bind) fn push_root(&mut self, root: dir::LocalNodeId<dir::Expression>) {
        self.stats.roots += 1;
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

    /// Bind one node and its decorators in the current lexical scope.
    pub(in crate::bind) fn bind_node(&mut self, node_id: dir::LocalNodeIdAny) {
        // bind the target before any target-owned scope is entered
        let scope = self.scope();
        self.bindings.bind_scope_any(node_id, scope);

        // bind attached decorator expressions at the same lexical cursor
        let parsed = self.parsed;
        let tree = &parsed.tree;
        for &decorator_id in tree.get_decorators_ref(node_id.id) {
            let decorator = tree.get(decorator_id);
            self.visit_decorator(tree, decorator_id, decorator);
        }
    }

    /// Insert one child scope below the current lexical scope.
    pub(in crate::bind) fn insert_child_scope(
        &mut self,
        kind: dir::ScopeKind,
    ) -> dir::LocalScopeId {
        self.bindings.insert_scope(kind, Some(self.scope()), None)
    }

    /// Bind one node to the scope it introduces, at that scope's full mark.
    pub(in crate::bind) fn bind_node_to_scope(
        &mut self,
        node_id: dir::LocalNodeIdAny,
        scope_id: dir::LocalScopeId,
    ) {
        self.attach_node_to_scope(node_id, scope_id);
        self.bindings.introduce_scope(node_id, scope_id);
    }

    /// Bind one node to a scope it shares, at that scope's full mark.
    pub(in crate::bind) fn attach_node_to_scope(
        &mut self,
        node_id: dir::LocalNodeIdAny,
        scope_id: dir::LocalScopeId,
    ) {
        let scope = dir::LocalScope::new(scope_id, dir::LocalScopeMark::end());

        self.bindings.bind_scope_any(node_id, scope);
    }

    /// Attach the global scope at the current module cursor.
    pub(in crate::bind) fn attach_global_scope(&mut self) {
        // keep the first attachment as the visibility root
        if self
            .bindings
            .get_scope_by_id(self.namespace_scope)
            .parent
            .is_some()
        {
            return;
        }

        // route module lookups through the ancestor global scope
        let global = dir::LocalScope::new(self.global_scope, dir::LocalScopeMark::end());
        self.bindings
            .get_scope_by_id_mut(self.namespace_scope)
            .parent = Some(global);
        self.bindings
            .get_scope_by_id_mut(self.global_scope)
            .append_child(self.namespace_scope);
    }

    /// Push one scope while visiting children.
    pub(in crate::bind) fn push_scope(&mut self, scope_id: dir::LocalScopeId) {
        self.scope_stack.push(scope_id);
    }

    /// Pop one child scope after visiting children.
    pub(in crate::bind) fn pop_scope(&mut self) {
        self.scope_stack.pop().expect("bind scope stack underflow");
    }

    /// Push one conditional type scope while visiting its infer pattern.
    pub(in crate::bind) fn push_infer_scope(&mut self, scope_id: dir::LocalScopeId) {
        self.infer_scope_stack.push(scope_id);
    }

    /// Pop one conditional type scope after visiting its infer pattern.
    pub(in crate::bind) fn pop_infer_scope(&mut self) {
        self.infer_scope_stack
            .pop()
            .expect("bind infer scope stack underflow");
    }

    /// Return the conditional type scope that holds active `infer` binders.
    pub(in crate::bind) fn infer_scope(&self) -> Option<dir::LocalScopeId> {
        self.infer_scope_stack.last().copied()
    }

    /// Push binding modifiers while visiting a pattern subtree.
    pub(in crate::bind) fn push_binding_modifiers(&mut self, modifiers: BindingModifiers) {
        self.binding_modifier_stack.push(modifiers);
    }

    /// Pop binding modifiers after visiting a pattern subtree.
    pub(in crate::bind) fn pop_binding_modifiers(&mut self) {
        self.binding_modifier_stack
            .pop()
            .expect("bind binding modifier stack underflow");
    }

    /// Return the current binding modifiers.
    pub(in crate::bind) fn binding_modifiers(&self) -> BindingModifiers {
        *self
            .binding_modifier_stack
            .last()
            .expect("bind binding modifier stack is empty")
    }

    /// Push shared symbols for one union pattern.
    pub(in crate::bind) fn push_union_pattern_symbols(
        &mut self,
        symbols: IndexMap<dir::StaticKey, dir::LocalSymbolId>,
    ) {
        self.union_pattern_symbols.push(symbols);
    }

    /// Pop shared symbols for one union pattern.
    pub(in crate::bind) fn pop_union_pattern_symbols(&mut self) {
        self.union_pattern_symbols
            .pop()
            .expect("bind union pattern symbol stack underflow");
    }

    /// Return the shared symbol for one active union pattern binding.
    pub(in crate::bind) fn union_pattern_symbol(
        &self,
        key: dir::StaticKey,
    ) -> Option<dir::LocalSymbolId> {
        self.union_pattern_symbols
            .last()
            .and_then(|symbols| symbols.get(&key).copied())
    }

    /// Return whether shared union pattern symbols are active.
    pub(in crate::bind) fn has_union_pattern_symbols(&self) -> bool {
        !self.union_pattern_symbols.is_empty()
    }

    /// Insert one symbol in the current lexical scope.
    pub(in crate::bind) fn insert_symbol(
        &mut self,
        role: dir::SymbolRole,
        kind: dir::SymbolKind,
        key: Option<dir::StaticKey>,
        export: Option<dir::ExportKind>,
        visibility: dir::SymbolVisibility,
    ) -> dir::LocalSymbolId {
        let scope = self.scope();
        let symbol_id = self
            .bindings
            .insert_symbol(role, kind, key, scope, export, visibility);
        if scope.id == self.global_scope {
            self.bindings.get_symbol_mut(symbol_id).origin = dir::SymbolOrigin::Global;
        }

        symbol_id
    }

    /// Insert one source binding symbol in the current lexical scope.
    pub(in crate::bind) fn insert_binding_symbol(
        &mut self,
        key: dir::StaticKey,
        modifiers: BindingModifiers,
    ) -> dir::LocalSymbolId {
        // module statics hoist for resolution, local bindings rebind in order
        let scope = self.bindings.get_scope_by_id(self.current_scope_id()).kind;
        let is_static_scope = matches!(
            scope,
            dir::ScopeKind::Module | dir::ScopeKind::Global | dir::ScopeKind::Namespace
        );
        let visibility = match modifiers.kind {
            dir::SymbolKind::Variable if is_static_scope => dir::SymbolVisibility::Scope,
            _ => dir::SymbolVisibility::Forward,
        };
        let symbol_id = self.insert_symbol(
            dir::SymbolRole::Local,
            modifiers.kind,
            Some(key),
            modifiers.export,
            visibility,
        );

        // retain source modifiers on the durable symbol
        let symbol = self.bindings.get_symbol_mut(symbol_id);
        symbol.binding_mutability = modifiers.mutability;
        symbol.is_shared = modifiers.is_shared;

        symbol_id
    }

    /// Insert one symbol in the given lexical scope.
    pub(in crate::bind) fn insert_symbol_in_scope(
        &mut self,
        scope_id: dir::LocalScopeId,
        role: dir::SymbolRole,
        kind: dir::SymbolKind,
        key: Option<dir::StaticKey>,
        export: Option<dir::ExportKind>,
        visibility: dir::SymbolVisibility,
    ) -> dir::LocalSymbolId {
        let mark = self.bindings.get_scope_mark(scope_id);
        let scope = dir::LocalScope::new(scope_id, mark);
        let symbol_id = self
            .bindings
            .insert_symbol(role, kind, key, scope, export, visibility);
        if scope_id == self.global_scope {
            self.bindings.get_symbol_mut(symbol_id).origin = dir::SymbolOrigin::Global;
        }

        symbol_id
    }

    /// Insert one symbol with an owned child scope.
    pub(in crate::bind) fn insert_symbol_with_scope(
        &mut self,
        role: dir::SymbolRole,
        kind: dir::SymbolKind,
        key: Option<dir::StaticKey>,
        export: Option<dir::ExportKind>,
        scope_kind: dir::ScopeKind,
        visibility: dir::SymbolVisibility,
    ) -> (dir::LocalSymbolId, dir::LocalScopeId) {
        let symbol_id = self.insert_symbol(role, kind, key, export, visibility);
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

    /// Declare one control label without exposing it to lexical lookup.
    pub(in crate::bind) fn declare_control_label(
        &mut self,
        name: dir::StringId,
        node_id: dir::LocalNodeId<dir::Expression>,
    ) {
        let symbol_id = self.insert_symbol(
            dir::SymbolRole::Local,
            dir::SymbolKind::Label,
            Some(dir::StaticKey::Name(name)),
            None,
            dir::SymbolVisibility::Hidden,
        );
        self.declare_symbol(symbol_id, node_id);
    }

    /// Return the current scope id.
    fn current_scope_id(&self) -> dir::LocalScopeId {
        *self.scope_stack.last().expect("bind scope stack is empty")
    }
}
