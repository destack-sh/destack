use destack_serde::Reflect;
use std::fmt::Debug;
use std::sync::Arc;

use destack_core::FxIndexMap as IndexMap;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{
    Arena, ExportKind, GlobalNodeIdAny, LocalNodeId, LocalNodeIdAny, LocalScope, LocalScopeId,
    LocalScopeMark, LocalSymbolId, Node, Scope, ScopeIndex, ScopeKind, SegmentView, StaticKey,
    Symbol, SymbolKind, SymbolLookup, SymbolOrigin, SymbolPath, SymbolRole, SymbolVisibility, View,
};

/// Cumulative lexical scopes and symbols for one DIR module.
#[derive(Debug, Clone)]
pub struct BindingTable<'a> {
    /// The module id of the binding table.
    pub module_id: ModuleId,
    /// The ordered binding table segments.
    segments: SegmentView<'a, BindingSegment>,
}

/// One keyed binding visible from a lexical scope cursor.
#[derive(Debug, Clone, Copy)]
pub struct VisibleBinding<'a> {
    /// The binding key.
    pub key: StaticKey,
    /// The bound symbol id.
    pub symbol_id: LocalSymbolId,
    /// The bound symbol.
    pub symbol: &'a Symbol,
}

impl BindingTable<'static> {
    /// Create a binding table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<BindingSegment>>) -> Self {
        let segments = SegmentView::from_segments(segments);

        Self::from_view(segments)
    }

    /// Create a binding table from one segment.
    pub fn from_segment(segment: Arc<BindingSegment>) -> Self {
        Self::from_segments(vec![segment])
    }
}

impl<'a> BindingTable<'a> {
    /// Create a binding table from a segment view.
    pub fn from_view(segments: SegmentView<'a, BindingSegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("binding table needs at least one segment"));
        let module_id = first.module_id;

        // require a single module owner
        for segment in segments.iter() {
            assert_eq!(
                segment.module_id, module_id,
                "binding table segment belongs to a different module"
            );
        }

        Self {
            module_id,
            segments,
        }
    }

    /// Create a binding table by appending a borrowed tail segment.
    pub fn with_tail<'b>(&'b self, tail: &'b BindingSegment) -> BindingTable<'b> {
        BindingTable::from_view(self.segments.with_tail(tail))
    }

    /// Iterate symbol ids.
    pub fn symbol_ids(&self) -> impl Iterator<Item = LocalSymbolId> + '_ {
        (0..self.symbol_count()).map(LocalSymbolId::new)
    }

    /// Iterate visible symbols.
    pub fn symbols(&self) -> impl Iterator<Item = &Symbol> {
        self.symbol_ids()
            .map(|symbol_id| self.get_symbol(symbol_id))
    }

    /// Get the number of symbols.
    pub fn symbol_count(&self) -> u32 {
        self.segments
            .last()
            .map(|segment| segment.symbol_count())
            .unwrap_or(0)
    }

    /// Get a symbol by its raw id.
    pub fn get_symbol_by_id(&self, symbol_id: u32) -> &Symbol {
        self.get_symbol(LocalSymbolId::new(symbol_id))
    }

    /// Iterate scope ids.
    pub fn scope_ids(&self) -> impl Iterator<Item = LocalScopeId> + '_ {
        (0..self.scope_count()).map(LocalScopeId::new)
    }

    /// Iterate visible scopes.
    pub fn scopes(&self) -> impl Iterator<Item = &Scope> {
        self.scope_ids()
            .map(|scope_id| self.get_scope_by_id(scope_id))
    }

    /// Get the number of scopes.
    pub fn scope_count(&self) -> u32 {
        self.segments
            .last()
            .map(|segment| segment.scope_count())
            .unwrap_or(0)
    }

    /// Return one visible symbol.
    pub fn get_symbol(&self, symbol_id: LocalSymbolId) -> &Symbol {
        self.get_symbol_maybe(symbol_id).unwrap_or_else(|| {
            panic!(
                "DIR symbol {symbol_id:?} is not visible in module {:?}",
                self.module_id
            )
        })
    }

    /// Return one visible symbol when present.
    pub fn get_symbol_maybe(&self, symbol_id: LocalSymbolId) -> Option<&Symbol> {
        for segment in self.segments.iter().rev() {
            if let Some(symbol) = segment.get_symbol_maybe(symbol_id) {
                return Some(symbol);
            }
        }

        None
    }

    /// Return one visible scope.
    pub fn get_scope_by_id(&self, scope_id: LocalScopeId) -> &Scope {
        self.get_scope_maybe(scope_id)
            .unwrap_or_else(|| panic!("DIR scope {scope_id:?} is not visible"))
    }

    /// Return one visible scope by cursor.
    pub fn get_scope(&self, scope: LocalScope) -> &Scope {
        self.get_scope_by_id(scope.id)
    }

    /// Iterate one scope's lexical ancestors, nearest first.
    pub fn scope_ancestors(&self, scope_id: LocalScopeId) -> impl Iterator<Item = LocalScope> + '_ {
        let parent = self.get_scope_by_id(scope_id).parent;

        std::iter::successors(parent, |scope| self.get_scope(*scope).parent)
    }

    /// Iterate keyed bindings visible from a lexical scope cursor.
    pub fn visible_bindings(
        &self,
        scope: LocalScope,
    ) -> impl Iterator<Item = VisibleBinding<'_>> + '_ {
        std::iter::successors(Some(scope), |scope| self.get_scope(*scope).parent).flat_map(
            move |cursor| {
                let scope = self.get_scope(cursor);

                scope
                    .bindings
                    .iter()
                    .enumerate()
                    .rev()
                    .filter_map(move |(index, binding)| {
                        let key = binding.key?;
                        let symbol = self.get_symbol(binding.symbol);
                        let is_visible = symbol.visibility == SymbolVisibility::Scope
                            || symbol.visibility == SymbolVisibility::Forward
                                && index < cursor.mark.0 as usize;

                        is_visible.then_some(VisibleBinding {
                            key,
                            symbol_id: binding.symbol,
                            symbol,
                        })
                    })
            },
        )
    }

    /// Return the module's root namespace scope, with all bindings visible.
    pub fn module_scope(&self) -> LocalScope {
        let scope_id = self
            .scope_ids()
            .find(|scope_id| self.get_scope_by_id(*scope_id).is_root())
            .unwrap_or_else(|| panic!("binding table has no root scope"));

        LocalScope::new(scope_id, LocalScopeMark::end())
    }

    /// Return the owned scope for one visible symbol.
    pub fn scope_for_owner(&self, owner: LocalSymbolId) -> Option<LocalScope> {
        for segment in self.segments.iter().rev() {
            if let Some(scope_id) = segment.scope_for_owner(owner) {
                return Some(LocalScope::new(scope_id, LocalScopeMark::end()));
            }
        }

        None
    }

    /// Return the visible scope that owns one symbol.
    pub fn get_scope_by_symbol(&self, symbol_id: LocalSymbolId) -> &Scope {
        let symbol = self.get_symbol(symbol_id);

        self.get_scope(symbol.scope)
    }

    /// Return the lexical owner path for one symbol.
    pub fn symbol_path(&self, symbol_id: LocalSymbolId) -> SymbolPath {
        let mut symbols = Vec::new();
        let mut seen = Vec::new();
        let mut current = Some(symbol_id);

        // climb lexical owners from leaf to outermost
        while let Some(symbol_id) = current {
            assert!(
                !seen.contains(&symbol_id),
                "binding table contains a cyclic symbol path"
            );
            seen.push(symbol_id);

            symbols.push(symbol_id);
            // anonymous namespaces contribute no path segment
            current = self.symbol_owner(symbol_id).filter(|owner| {
                let owner = self.get_symbol(*owner);

                owner.role != SymbolRole::Namespace || owner.name().is_some()
            });
        }

        symbols.reverse();

        SymbolPath::new(symbols)
    }

    /// Return the nearest lexical declaration owner of one symbol.
    pub fn symbol_owner(&self, symbol_id: LocalSymbolId) -> Option<LocalSymbolId> {
        let symbol = self.get_symbol(symbol_id);
        let mut scope_id = symbol.scope.id;
        let mut seen = Vec::new();

        // climb lexical scopes until a declaration owner is found
        loop {
            assert!(
                !seen.contains(&scope_id),
                "binding table contains a cyclic scope path"
            );
            seen.push(scope_id);

            let scope = self.get_scope_by_id(scope_id);
            if let Some(owner) = scope.owner {
                return Some(owner);
            }

            let parent = scope.parent?;
            scope_id = parent.id;
        }
    }

    /// Return one visible scope when present.
    pub fn get_scope_maybe(&self, scope_id: LocalScopeId) -> Option<&Scope> {
        for segment in self.segments.iter().rev() {
            if let Some(scope) = segment.get_scope_maybe(scope_id) {
                return Some(scope);
            }
        }

        None
    }

    /// Find the symbol declared by one node.
    pub fn declaration_symbol(&self, declaration: GlobalNodeIdAny) -> Option<LocalSymbolId> {
        for segment in self.segments.iter().rev() {
            if let Some(symbol_id) = segment.declaration_symbol(declaration) {
                return Some(symbol_id);
            }
        }

        None
    }

    /// Find the implicit receiver symbol bound for one member node.
    pub fn implicit_receiver_symbol(&self, owner: GlobalNodeIdAny) -> Option<LocalSymbolId> {
        for segment in self.segments.iter().rev() {
            if let Some(symbol_id) = segment.implicit_receiver_symbol(owner) {
                return Some(symbol_id);
            }
        }

        None
    }

    /// Iterate symbols keyed by declaration node.
    pub fn declaration_symbols(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, LocalSymbolId)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.declaration_symbols())
    }

    /// Iterate implicit receiver symbols keyed by member node.
    pub fn implicit_receivers(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, LocalSymbolId)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.implicit_receivers())
    }

    /// Return the lexical scope attached to one global node.
    pub fn scope_for_node(&self, node_id: GlobalNodeIdAny) -> Option<LocalScope> {
        for segment in self.segments.iter().rev() {
            if let Some(scope) = segment.scope_for_node(node_id) {
                return Some(scope);
            }
        }

        None
    }

    /// Return the scope one global node introduces.
    pub fn introduced_scope(&self, node_id: GlobalNodeIdAny) -> Option<LocalScopeId> {
        for segment in self.segments.iter().rev() {
            if let Some(scope) = segment.introduced_scope(node_id) {
                return Some(scope);
            }
        }

        None
    }

    /// Look up one symbol visible at a source node.
    pub fn lookup_symbol_at(
        &self,
        view: &View<'_>,
        node: LocalNodeIdAny,
        key: StaticKey,
    ) -> SymbolLookup {
        let scope = self.scope_at(view, node);

        self.lookup_symbol_from_scope(scope, key)
    }

    /// Return the scope in effect at a source node.
    pub fn scope_at(&self, view: &View<'_>, node: LocalNodeIdAny) -> LocalScope {
        let mut current = Some(node);
        while let Some(node) = current {
            if let Some(scope) = self.scope_for_node(node.into_global(self.module_id)) {
                return scope;
            }
            current = view.get_parent(node.id);
        }

        self.module_scope()
    }

    /// Look up one symbol visible from a lexical scope cursor.
    pub fn lookup_symbol_from_scope(&self, mut scope: LocalScope, key: StaticKey) -> SymbolLookup {
        loop {
            let current = self.get_scope(scope);
            let lookup = self.lookup_symbols_in_scope(current, scope.mark, key);
            if !matches!(lookup, SymbolLookup::Missing) {
                return lookup;
            }

            let Some(parent) = current.parent else {
                return SymbolLookup::Missing;
            };

            scope = LocalScope::new(parent.id, parent.mark);
        }
    }

    /// Look up one keyed member symbol owned by a declaration symbol.
    pub fn lookup_key_member(&self, owner: LocalSymbolId, key: StaticKey) -> SymbolLookup {
        let Some(scope) = self.scope_for_owner(owner) else {
            return SymbolLookup::Missing;
        };
        let scope = self.get_scope(scope);
        let mut lookup = SymbolLookup::Missing;

        // collect matching owner members
        scope.for_symbols_by_key(key, |symbol| lookup.push(symbol));

        lookup
    }

    /// Iterate scopes keyed by owner or member node.
    pub fn node_scopes(&self) -> impl Iterator<Item = (GlobalNodeIdAny, LocalScope)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.node_scopes())
    }

    /// Iterate symbols replaced by later segments.
    pub fn replaced_symbols(&self) -> impl Iterator<Item = (LocalSymbolId, &Symbol)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.replaced_symbols())
    }

    /// Iterate scopes replaced by later segments.
    pub fn replaced_scopes(&self) -> impl Iterator<Item = (LocalScopeId, &Scope)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.replaced_scopes())
    }

    /// Look up named symbols inside one scope cursor.
    fn lookup_symbols_in_scope(
        &self,
        scope: &Scope,
        mark: LocalScopeMark,
        key: StaticKey,
    ) -> SymbolLookup {
        let mut lookup = SymbolLookup::Missing;

        // collect whole-scope bindings
        scope.for_symbols_by_key(key, |symbol_id| {
            let symbol = self.get_symbol(symbol_id);
            if symbol.visibility == SymbolVisibility::Scope {
                lookup.push(symbol_id);
            }
        });

        // collect forward bindings up to the current source mark
        scope.for_symbols_by_key_up_to(key, mark, |symbol_id| {
            let symbol = self.get_symbol(symbol_id);
            if symbol.visibility == SymbolVisibility::Forward {
                lookup.push(symbol_id);
            }
        });

        lookup
    }
}

/// Lexical scopes and symbols added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct BindingSegment {
    /// The module id of the binding segment.
    pub module_id: ModuleId,
    /// The first symbol id owned by this table segment.
    pub(crate) first_symbol_id: u32,
    /// The first scope id owned by this table segment.
    pub(crate) first_scope_id: u32,

    /// The symbols in the table.
    pub(crate) symbols: Arena<Symbol>,
    /// The scopes in the table.
    pub(crate) scopes: Arena<Scope>,
    /// Symbols keyed by their declaration node.
    pub(crate) symbol_by_declaration: IndexMap<GlobalNodeIdAny, LocalSymbolId>,
    /// Implicit receiver symbols keyed by their owner node.
    pub(crate) implicit_receiver_by_node: IndexMap<GlobalNodeIdAny, LocalSymbolId>,
    /// The scope in effect at each bound node.
    pub(crate) scope_by_node: IndexMap<GlobalNodeIdAny, LocalScope>,
    /// Scopes keyed by the node introducing them.
    pub(crate) scope_by_introducer: IndexMap<GlobalNodeIdAny, LocalScopeId>,
    /// Scopes keyed by their owner symbol.
    pub(crate) scope_by_owner: IndexMap<LocalSymbolId, LocalScopeId>,

    /// Replacements for visible symbols copied into this segment.
    pub(crate) replaced_symbol_by_id: IndexMap<LocalSymbolId, Symbol>,
    /// Replacements for visible scopes copied into this segment.
    pub(crate) replaced_scope_by_id: IndexMap<LocalScopeId, Scope>,
}

#[allow(clippy::too_many_arguments)]
impl BindingSegment {
    /// Create a new binding segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            first_symbol_id: 0,
            first_scope_id: 0,
            symbols: Arena::new(),
            scopes: Arena::new(),
            symbol_by_declaration: IndexMap::default(),
            implicit_receiver_by_node: IndexMap::default(),
            scope_by_node: IndexMap::default(),
            scope_by_introducer: IndexMap::default(),
            scope_by_owner: IndexMap::default(),
            replaced_symbol_by_id: IndexMap::default(),
            replaced_scope_by_id: IndexMap::default(),
        }
    }

    /// Create a new empty segment after an existing binding segment.
    pub fn from_base(base: &Self) -> Self {
        Self {
            module_id: base.module_id,
            first_symbol_id: base.symbol_count(),
            first_scope_id: base.scope_count(),
            symbols: Arena::new(),
            scopes: Arena::new(),
            symbol_by_declaration: IndexMap::default(),
            implicit_receiver_by_node: IndexMap::default(),
            scope_by_node: IndexMap::default(),
            scope_by_introducer: IndexMap::default(),
            scope_by_owner: IndexMap::default(),
            replaced_symbol_by_id: IndexMap::default(),
            replaced_scope_by_id: IndexMap::default(),
        }
    }

    /// Create an empty segment after a cumulative binding table.
    pub fn from_table(base: &BindingTable<'_>) -> Self {
        Self {
            module_id: base.module_id,
            first_symbol_id: base.symbol_count(),
            first_scope_id: base.scope_count(),
            symbols: Arena::new(),
            scopes: Arena::new(),
            symbol_by_declaration: IndexMap::default(),
            implicit_receiver_by_node: IndexMap::default(),
            scope_by_node: IndexMap::default(),
            scope_by_introducer: IndexMap::default(),
            scope_by_owner: IndexMap::default(),
            replaced_symbol_by_id: IndexMap::default(),
            replaced_scope_by_id: IndexMap::default(),
        }
    }

    /// Return the first symbol id owned by this table segment.
    pub fn first_symbol_id(&self) -> u32 {
        self.first_symbol_id
    }

    /// Return the first scope id owned by this table segment.
    pub fn first_scope_id(&self) -> u32 {
        self.first_scope_id
    }

    /// Iterate symbol ids.
    pub fn symbol_ids(&self) -> impl Iterator<Item = LocalSymbolId> + '_ {
        (self.first_symbol_id..self.symbol_count()).map(LocalSymbolId::new)
    }

    /// Get the number of symbols.
    #[inline]
    pub fn symbol_count(&self) -> u32 {
        self.first_symbol_id + self.symbols.len() as u32
    }

    /// Iterate scope ids.
    pub fn scope_ids(&self) -> impl Iterator<Item = LocalScopeId> + '_ {
        (self.first_scope_id..self.scope_count()).map(LocalScopeId::new)
    }

    /// Get the number of scopes.
    #[inline]
    pub fn scope_count(&self) -> u32 {
        self.first_scope_id + self.scopes.len() as u32
    }

    /// Insert a new symbol.
    pub fn insert_symbol(
        &mut self,
        role: SymbolRole,
        kind: SymbolKind,
        key: Option<StaticKey>,
        scope: LocalScope,
        export: Option<ExportKind>,
        visibility: SymbolVisibility,
    ) -> LocalSymbolId {
        // allocate the symbol in declaration order
        let symbol_id = LocalSymbolId::new(self.symbol_count());
        self.symbols.allocate(Symbol {
            role,
            kind,
            visibility,
            binding_mutability: None,
            is_shared: false,
            origin: SymbolOrigin::Module,
            key,
            scope,
            export_kind: export,
            declaration: None,
        });

        // index symbols selected through scope lookup
        match visibility {
            SymbolVisibility::Scope | SymbolVisibility::Forward | SymbolVisibility::Member => {
                self.get_scope_by_id_mut(scope.id).append(key, symbol_id);
            }
            SymbolVisibility::Hidden => {}
        }

        symbol_id
    }

    /// Attach a declaration node to a symbol.
    pub fn declare_symbol<T: Node>(&mut self, symbol_id: LocalSymbolId, node_id: LocalNodeId<T>) {
        let module_id = self.module_id;
        let declaration = node_id.into_global_any(module_id);

        // record the declaration node once
        let symbol = self.get_symbol_mut(symbol_id);
        if symbol.declaration.is_none() {
            symbol.declaration = Some(declaration);
        }
        self.symbol_by_declaration.insert(declaration, symbol_id);
    }

    /// Find the symbol declared by one node.
    #[inline]
    pub fn declaration_symbol(&self, declaration: GlobalNodeIdAny) -> Option<LocalSymbolId> {
        self.symbol_by_declaration.get(&declaration).copied()
    }

    /// Attach an implicit receiver symbol to its owner node.
    pub fn bind_implicit_receiver<T: Node>(
        &mut self,
        owner: LocalNodeId<T>,
        symbol: LocalSymbolId,
    ) {
        let owner = owner.into_global_any(self.module_id);

        self.implicit_receiver_by_node.insert(owner, symbol);
    }

    /// Find the implicit receiver symbol bound for one member node.
    #[inline]
    pub fn implicit_receiver_symbol(&self, owner: GlobalNodeIdAny) -> Option<LocalSymbolId> {
        self.implicit_receiver_by_node.get(&owner).copied()
    }

    /// Iterate symbols keyed by declaration node.
    pub fn declaration_symbols(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, LocalSymbolId)> + '_ {
        self.symbol_by_declaration
            .iter()
            .map(|(node_id, symbol_id)| (*node_id, *symbol_id))
    }

    /// Iterate implicit receiver symbols keyed by member node.
    pub fn implicit_receivers(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, LocalSymbolId)> + '_ {
        self.implicit_receiver_by_node
            .iter()
            .map(|(node_id, symbol_id)| (*node_id, *symbol_id))
    }

    /// Attach a lexical scope to one typed node.
    pub fn bind_scope<T: Node>(&mut self, node_id: LocalNodeId<T>, scope: LocalScope) {
        self.bind_scope_any(node_id.into_any(), scope);
    }

    /// Attach a lexical scope to one erased node.
    pub fn bind_scope_any(&mut self, node_id: LocalNodeIdAny, scope: LocalScope) {
        let node_id = node_id.into_global(self.module_id);

        self.scope_by_node.insert(node_id, scope);
    }

    /// Return the lexical scope attached to one global node.
    #[inline]
    pub fn scope_for_node(&self, node_id: GlobalNodeIdAny) -> Option<LocalScope> {
        self.scope_by_node.get(&node_id).copied()
    }

    /// Record one node as the introducer of a scope.
    pub fn introduce_scope(&mut self, node_id: LocalNodeIdAny, scope_id: LocalScopeId) {
        let node_id = node_id.into_global(self.module_id);

        self.scope_by_introducer.insert(node_id, scope_id);
    }

    /// Return the scope one global node introduces.
    #[inline]
    pub fn introduced_scope(&self, node_id: GlobalNodeIdAny) -> Option<LocalScopeId> {
        self.scope_by_introducer.get(&node_id).copied()
    }

    /// Return the owned scope for one local symbol.
    #[inline]
    pub fn scope_for_owner(&self, owner: LocalSymbolId) -> Option<LocalScopeId> {
        self.scope_by_owner.get(&owner).copied()
    }

    /// Iterate scopes keyed by owner or member node.
    pub fn node_scopes(&self) -> impl Iterator<Item = (GlobalNodeIdAny, LocalScope)> + '_ {
        self.scope_by_node
            .iter()
            .map(|(node_id, scope)| (*node_id, *scope))
    }

    /// Iterate scopes keyed by owner symbol.
    pub fn owner_scopes(&self) -> impl Iterator<Item = (LocalSymbolId, LocalScopeId)> + '_ {
        self.scope_by_owner
            .iter()
            .map(|(owner, scope)| (*owner, *scope))
    }

    /// Insert a new scope.
    pub fn insert_scope(
        &mut self,
        kind: ScopeKind,
        parent: Option<LocalScope>,
        owner: Option<LocalSymbolId>,
    ) -> LocalScopeId {
        let scope_id = LocalScopeId::new(self.scope_count());
        let scope = Scope {
            kind,
            owner,
            parent,
            bindings: Vec::new(),
            index: ScopeIndex::default(),
            children: Vec::new(),
        };
        self.scopes.allocate(scope);
        if let Some(parent) = parent {
            self.get_scope_by_id_mut(parent.id).append_child(scope_id);
        }
        if let Some(owner) = owner {
            self.scope_by_owner.insert(owner, scope_id);
        }

        scope_id
    }

    /// Get a symbol by its id.
    #[inline]
    pub fn get_symbol(&self, symbol_id: LocalSymbolId) -> &Symbol {
        if let Some(symbol) = self.replaced_symbol_by_id.get(&symbol_id) {
            return symbol;
        }

        self.get_local_symbol(symbol_id)
            .unwrap_or_else(|| panic!("DIR symbol {symbol_id:?} is not allocated in this segment"))
    }

    /// Get a symbol by id when this table owns or replaces it.
    #[inline]
    pub fn get_symbol_maybe(&self, symbol_id: LocalSymbolId) -> Option<&Symbol> {
        self.get_local_symbol(symbol_id)
    }

    /// Get the symbol mutable by its id.
    #[inline]
    pub fn get_symbol_mut(&mut self, symbol_id: LocalSymbolId) -> &mut Symbol {
        if self.contains_symbol_id(symbol_id) {
            let slot = self.symbol_slot(symbol_id);
            return self.symbols.get_mut(slot);
        }

        self.replaced_symbol_by_id
            .get_mut(&symbol_id)
            .unwrap_or_else(|| panic!("DIR symbol {symbol_id:?} is not mutable in this segment"))
    }

    /// Get the scope cursor for a scope id.
    #[inline]
    pub fn get_scope_mark(&self, scope_id: LocalScopeId) -> LocalScopeMark {
        self.get_scope_by_id(scope_id).mark()
    }

    /// Get a scope by its id.
    #[inline]
    pub fn get_scope_by_id(&self, scope_id: LocalScopeId) -> &Scope {
        if let Some(scope) = self.replaced_scope_by_id.get(&scope_id) {
            return scope;
        }

        self.get_local_scope(scope_id)
            .unwrap_or_else(|| panic!("DIR scope {scope_id:?} is not allocated in this segment"))
    }

    /// Get a scope by id when this table owns or replaces it.
    #[inline]
    pub fn get_scope_maybe(&self, scope_id: LocalScopeId) -> Option<&Scope> {
        self.get_local_scope(scope_id)
    }

    /// Get the scope mutable by its id.
    #[inline]
    pub fn get_scope_by_id_mut(&mut self, scope_id: LocalScopeId) -> &mut Scope {
        if self.contains_scope_id(scope_id) {
            let slot = self.scope_slot(scope_id);
            return self.scopes.get_mut(slot);
        }

        self.replaced_scope_by_id
            .get_mut(&scope_id)
            .unwrap_or_else(|| panic!("DIR scope {scope_id:?} is not mutable in this segment"))
    }

    /// Get a symbol owned or replaced by this table segment.
    pub(crate) fn get_local_symbol(&self, symbol_id: LocalSymbolId) -> Option<&Symbol> {
        if let Some(symbol) = self.replaced_symbol_by_id.get(&symbol_id) {
            return Some(symbol);
        }

        self.contains_symbol_id(symbol_id)
            .then(|| self.symbols.get(self.symbol_slot(symbol_id)))
    }

    /// Get a scope owned or replaced by this table segment.
    pub(crate) fn get_local_scope(&self, scope_id: LocalScopeId) -> Option<&Scope> {
        if let Some(scope) = self.replaced_scope_by_id.get(&scope_id) {
            return Some(scope);
        }

        self.contains_scope_id(scope_id)
            .then(|| self.scopes.get(self.scope_slot(scope_id)))
    }

    /// Replace one visible symbol in this table segment.
    pub fn replace_symbol(&mut self, symbol_id: LocalSymbolId, symbol: Symbol) {
        if let Some(declaration) = symbol.declaration {
            self.symbol_by_declaration.insert(declaration, symbol_id);
        }

        self.replaced_symbol_by_id.insert(symbol_id, symbol);
    }

    /// Replace one visible scope in this table segment.
    pub fn replace_scope(&mut self, scope_id: LocalScopeId, scope: Scope) {
        self.replaced_scope_by_id.insert(scope_id, scope);
    }

    /// Copy one visible scope into this segment before mutating it.
    pub fn make_scope_mutable(&mut self, scope_id: LocalScopeId, scope: &Scope) {
        if self.contains_scope_id(scope_id) || self.replaced_scope_by_id.contains_key(&scope_id) {
            return;
        }

        self.replaced_scope_by_id.insert(scope_id, scope.clone());
    }

    /// Iterate replaced symbols in this segment.
    pub fn replaced_symbols(&self) -> impl Iterator<Item = (LocalSymbolId, &Symbol)> + '_ {
        self.replaced_symbol_by_id
            .iter()
            .map(|(symbol_id, symbol)| (*symbol_id, symbol))
    }

    /// Iterate replaced scopes in this segment.
    pub fn replaced_scopes(&self) -> impl Iterator<Item = (LocalScopeId, &Scope)> + '_ {
        self.replaced_scope_by_id
            .iter()
            .map(|(scope_id, scope)| (*scope_id, scope))
    }

    /// Return whether this segment contains the given symbol id.
    fn contains_symbol_id(&self, symbol_id: LocalSymbolId) -> bool {
        symbol_id.id >= self.first_symbol_id && symbol_id.id < self.symbol_count()
    }

    /// Return whether this segment contains the given scope id.
    fn contains_scope_id(&self, scope_id: LocalScopeId) -> bool {
        scope_id.0 >= self.first_scope_id && scope_id.0 < self.scope_count()
    }

    /// Return the local arena slot for one symbol id.
    fn symbol_slot(&self, symbol_id: LocalSymbolId) -> u32 {
        symbol_id.id - self.first_symbol_id
    }

    /// Return the local arena slot for one scope id.
    fn scope_slot(&self, scope_id: LocalScopeId) -> u32 {
        scope_id.0 - self.first_scope_id
    }
}
