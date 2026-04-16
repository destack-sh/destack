use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

use destack_core::Arena;
use destack_dir as dir;
use destack_dir::GlobalSymbolId;
use destack_source::ModuleId;

use crate::{
    Annotation, Argument, ArrayElement, Block, CatchClause, Declaration, Declarator,
    DependencyItem, EnumField, Expression, GenericParameter, LocalNodeId, Member, Node, NodeType,
    Parameter, Pattern, PatternField, Property, Statement, SwitchCase, TupleElement, Type,
    TypeMember,
};

/// The local binding base name for one synthetic non-code module default.
pub const MODULE_DEFAULT_NAME: &str = "_default";

/// One stable symbol identity in lowered script output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScriptSymbolId {
    /// One symbol lowered directly from source DIR.
    Source(GlobalSymbolId),
    /// One generated default binding for one synthetic non-code script module.
    ModuleDefault(ModuleId),
}

/// Mutable AST Node tree for a single source unit. NOT THREAD-SAFE.
#[derive(Clone)]
pub struct NodeTree {
    /// The next id to allocate.
    pub(crate) next_global_id: u32,
    /// The local ids of all nodes. Index is the global node id.
    pub(crate) local_id_by_node_id: Vec<u32>,
    /// The types of all nodes. Index is the global node id.
    pub(crate) node_type_by_node_id: Vec<NodeType>,
    /// The sources of all nodes. Index is the global node id.
    pub(crate) module_by_node_id: Vec<ModuleId>,
    /// The annotations attached to nodes.
    pub(crate) annotations_by_node_id: HashMap<u32, Vec<LocalNodeId<Annotation>>>,

    /// The DIR ids of all nodes. Index is the global node id.
    pub(crate) source_id_by_node_id: Vec<u32>,
    /// The alias node id by DIR node id.
    pub(crate) alias_node_id_by_dir_id: HashMap<u32, u32>,
    /// The alias node id by JS AST node id.
    pub(crate) alias_node_id_by_node_id: HashMap<u32, u32>,
    /// The symbol identity by JS AST node id.
    pub(crate) symbol_id_by_node_id: Vec<Option<ScriptSymbolId>>,

    // node arenas
    pub(crate) blocks: Arena<Block>,
    pub(crate) catch_clauses: Arena<CatchClause>,
    pub(crate) statements: Arena<Statement>,
    pub(crate) expressions: Arena<Expression>,
    pub(crate) array_elements: Arena<ArrayElement>,
    pub(crate) declarations: Arena<Declaration>,
    pub(crate) declarators: Arena<Declarator>,
    pub(crate) properties: Arena<Property>,
    pub(crate) members: Arena<Member>,
    pub(crate) types: Arena<Type>,
    pub(crate) tuple_elements: Arena<TupleElement>,
    pub(crate) type_members: Arena<TypeMember>,
    pub(crate) enum_fields: Arena<EnumField>,
    pub(crate) dependency_items: Arena<DependencyItem>,
    pub(crate) switch_cases: Arena<SwitchCase>,
    pub(crate) generic_parameters: Arena<GenericParameter>,
    pub(crate) parameters: Arena<Parameter>,
    pub(crate) arguments: Arena<Argument>,
    pub(crate) patterns: Arena<Pattern>,
    pub(crate) pattern_fields: Arena<PatternField>,
    pub(crate) annotations: Arena<Annotation>,
}

impl Debug for NodeTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeTree")
            .field("next_global_id", &self.next_global_id)
            .field("node_count", &self.local_id_by_node_id.len())
            .finish()
    }
}

impl Default for NodeTree {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeTree {
    /// Create a new NodeTree.
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Create a new NodeTree with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            next_global_id: 0,
            local_id_by_node_id: Vec::with_capacity(capacity),
            node_type_by_node_id: Vec::with_capacity(capacity),
            module_by_node_id: Vec::with_capacity(capacity),
            annotations_by_node_id: HashMap::new(),
            source_id_by_node_id: Vec::with_capacity(capacity),
            alias_node_id_by_dir_id: HashMap::new(),
            alias_node_id_by_node_id: HashMap::new(),
            symbol_id_by_node_id: Vec::with_capacity(capacity),
            // node arenas
            blocks: Arena::new(),
            catch_clauses: Arena::new(),
            statements: Arena::new(),
            expressions: Arena::new(),
            array_elements: Arena::new(),
            declarations: Arena::new(),
            declarators: Arena::new(),
            properties: Arena::new(),
            members: Arena::new(),
            types: Arena::new(),
            tuple_elements: Arena::new(),
            type_members: Arena::new(),
            enum_fields: Arena::new(),
            dependency_items: Arena::new(),
            switch_cases: Arena::new(),
            generic_parameters: Arena::new(),
            parameters: Arena::new(),
            arguments: Arena::new(),
            patterns: Arena::new(),
            pattern_fields: Arena::new(),
            annotations: Arena::new(),
        }
    }

    /// Allocate a new node in the JS AST tree.
    fn insert<T>(&mut self, node: T, module_id: ModuleId) -> LocalNodeId<T>
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        let global_id = self.next_global_id;
        self.next_global_id = global_id + 1;
        self.node_type_by_node_id.push(T::TYPE);
        let local_id = <Self as NodeTreeImpl<T>>::allocate(self, node);
        self.local_id_by_node_id.push(local_id);
        self.module_by_node_id.push(module_id);
        self.symbol_id_by_node_id.push(None);
        LocalNodeId::new(global_id)
    }

    /// Allocate a new node in the JS AST tree derived from another JS AST node.
    pub fn insert_from_source<T, U>(
        &mut self,
        node: T,
        module_id: ModuleId,
        dir_node_id: dir::LocalNodeId<U>,
    ) -> LocalNodeId<T>
    where
        T: Node,
        Self: NodeTreeImpl<T>,
        U: dir::Node,
        dir::NodeTree: dir::NodeTreeImpl<U>,
    {
        let node_id = self.insert(node, module_id);
        self.source_id_by_node_id.push(dir_node_id.id);
        node_id
    }

    /// Allocate a new node in the JS AST tree derived from a DIR node.
    pub fn insert_from_source_any<T>(
        &mut self,
        node: T,
        module_id: ModuleId,
        dir_node_id: dir::LocalNodeIdAny,
    ) -> LocalNodeId<T>
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        let node_id = self.insert(node, module_id);
        self.source_id_by_node_id.push(dir_node_id.id);
        node_id
    }

    /// Allocate a new node in the JS AST tree derived from another DIR node.
    pub fn insert_from<T, U>(&mut self, node: T, dir_node_id: LocalNodeId<U>) -> LocalNodeId<T>
    where
        T: Node,
        Self: NodeTreeImpl<T>,
        U: Node,
    {
        let module_id = self.module_by_node_id[dir_node_id.id as usize];
        let node_id = self.insert(node, module_id);
        let source_id = self.source_id_by_node_id[dir_node_id.id as usize];
        self.source_id_by_node_id.push(source_id);

        if let Some(symbol_id) = self.symbol_by_id(dir_node_id.id) {
            self.symbol_id_by_node_id[node_id.id as usize] = Some(symbol_id);
        }

        self.alias_node_id_by_dir_id
            .insert(dir_node_id.id, node_id.id);
        node_id
    }

    /// Alias a node in the JS AST tree from a DIR node.
    pub fn alias_from_source<T>(&mut self, dir_id: u32, alias: LocalNodeId<T>)
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        self.alias_node_id_by_dir_id.insert(dir_id, alias.id);
    }

    /// Alias a node in the JS AST tree from a JS AST node.
    pub fn alias_from<T>(&mut self, node_id: u32, alias: LocalNodeId<T>)
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        self.alias_node_id_by_node_id.insert(node_id, alias.id);
    }

    /// Get the next id.
    #[inline]
    pub fn next_id(&self) -> u32 {
        self.next_global_id
    }

    /// Get the type of an untyped node id.
    #[inline]
    pub fn get_node_type(&self, id: u32) -> NodeType {
        self.node_type_by_node_id[id as usize]
    }

    /// Get an immutable reference to the node with the given NodeId.
    #[inline]
    pub fn get<T>(&self, id: LocalNodeId<T>) -> &T
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        let local_id = self.local_id_by_node_id[id.id as usize];
        <Self as NodeTreeImpl<T>>::get(self, local_id)
    }

    /// Get a mutable reference to the node with the given NodeId.
    #[inline]
    pub fn get_mut<T>(&mut self, id: LocalNodeId<T>) -> &mut T
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        let local_id = self.local_id_by_node_id[id.id as usize];
        <Self as NodeTreeImpl<T>>::get_mut(self, local_id)
    }

    /// Get the nodes for all nodes of a given type.
    #[inline]
    pub fn get_nodes<T>(&self) -> Vec<LocalNodeId<T>>
    where
        T: Node,
    {
        let mut nodes = Vec::new();
        for (idx, ty) in self.node_type_by_node_id.iter().enumerate() {
            if *ty == T::TYPE {
                nodes.push(LocalNodeId::new(idx as u32));
            }
        }
        nodes
    }

    /// Get the source and DIR id of a node by its global id.
    pub fn get_source(&self, node_id: u32) -> (ModuleId, u32) {
        (
            self.module_by_node_id[node_id as usize],
            self.source_id_by_node_id[node_id as usize],
        )
    }

    /// Store one symbol identity for one JS AST node.
    pub fn set_symbol<T>(&mut self, node_id: LocalNodeId<T>, symbol_id: ScriptSymbolId)
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        self.symbol_id_by_node_id[node_id.id as usize] = Some(symbol_id);
    }

    /// Return the symbol identity for one JS AST node when one exists.
    pub fn symbol<T>(&self, node_id: LocalNodeId<T>) -> Option<ScriptSymbolId>
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        self.symbol_id_by_node_id[node_id.id as usize]
    }

    /// Return the symbol identity for one untyped node id when one exists.
    pub fn symbol_by_id(&self, node_id: u32) -> Option<ScriptSymbolId> {
        self.symbol_id_by_node_id[node_id as usize]
    }

    /// Get the annotations for a node.
    pub fn get_annotations(&self, node_id: u32) -> Vec<LocalNodeId<Annotation>> {
        self.annotations_by_node_id
            .get(&node_id)
            .cloned()
            .unwrap_or_else(Vec::new)
    }
}

/// Map node types to arenas.
pub trait NodeTreeImpl<T: Node> {
    /// Allocate a node into the relevant arena.
    fn allocate(tree: &mut NodeTree, node: T) -> u32;
    /// Get a node from the relevant arena.
    fn get(tree: &NodeTree, idx: u32) -> &T;
    /// Get a mutable node from the relevant arena.
    fn get_mut(tree: &mut NodeTree, idx: u32) -> &mut T;
}

macro_rules! impl_node_tree_store {
    ($ty:ty, $field:ident) => {
        impl NodeTreeImpl<$ty> for NodeTree {
            #[inline]
            fn allocate(tree: &mut NodeTree, node: $ty) -> u32 {
                tree.$field.allocate(node)
            }

            #[inline]
            fn get(tree: &NodeTree, idx: u32) -> &$ty {
                tree.$field.get(idx)
            }

            #[inline]
            fn get_mut(tree: &mut NodeTree, idx: u32) -> &mut $ty {
                tree.$field.get_mut(idx)
            }
        }
    };
}

macro_rules! impl_node_tree_stores {
    ( $( $ty:ty => $field:ident ),+ $(,)? ) => {
        $( impl_node_tree_store!($ty, $field); )*
    };
}

impl_node_tree_stores! {
    Block => blocks,
    CatchClause => catch_clauses,
    Statement => statements,
    Expression => expressions,
    ArrayElement => array_elements,
    Declaration => declarations,
    Declarator => declarators,
    Property => properties,
    Member => members,
    Type => types,
    TupleElement => tuple_elements,
    TypeMember => type_members,
    EnumField => enum_fields,
    DependencyItem => dependency_items,
    SwitchCase => switch_cases,
    GenericParameter => generic_parameters,
    Parameter => parameters,
    Argument => arguments,
    Pattern => patterns,
    PatternField => pattern_fields,
    Annotation => annotations,
}
