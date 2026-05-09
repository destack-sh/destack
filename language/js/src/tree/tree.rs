use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

use destack_core::Arena;
use destack_dir as dir;
use destack_dir::GlobalSymbolId;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{
    Annotation, Argument, ArrayElement, AssignPattern, AssignPatternField, Block, CatchClause,
    Declaration, Declarator, DependencyItem, EnumField, Expression, GenericParameter, LocalNodeId,
    Member, Node, NodeType, Parameter, Pattern, PatternField, Property, Statement, SwitchCase,
    TupleElement, TypeExpression, TypeMember,
};

/// The local binding base name for one synthetic non-code module default.
pub const MODULE_DEFAULT_NAME: &str = "_default";

/// Dense metadata for one JS node id.
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub(crate) struct NodeIndexEntry {
    /// The packed local id and node type.
    packed: u32,
}

impl NodeIndexEntry {
    const NODE_TYPE_SHIFT: u32 = 24;
    const LOCAL_ID_MASK: u32 = (1 << Self::NODE_TYPE_SHIFT) - 1;

    /// Pack one local id and node type into a dense entry.
    #[inline]
    pub(crate) fn new(local_id: u32, node_type: NodeType) -> Self {
        debug_assert!(
            local_id < Self::LOCAL_ID_MASK,
            "JS node local id exceeds packed index capacity: {local_id}"
        );

        Self {
            packed: local_id | (Self::node_type_tag(node_type) << Self::NODE_TYPE_SHIFT),
        }
    }

    /// Return the local arena id for this entry.
    #[inline]
    pub(crate) fn local_id(self) -> u32 {
        self.packed & Self::LOCAL_ID_MASK
    }

    /// Return the concrete node type for this entry.
    #[inline]
    pub(crate) fn node_type(self) -> NodeType {
        match (self.packed >> Self::NODE_TYPE_SHIFT) as u8 {
            0 => NodeType::Block,
            1 => NodeType::CatchClause,
            2 => NodeType::Statement,
            3 => NodeType::Expression,
            4 => NodeType::ArrayElement,
            5 => NodeType::Declaration,
            6 => NodeType::Declarator,
            7 => NodeType::Property,
            8 => NodeType::Member,
            9 => NodeType::TypeExpression,
            10 => NodeType::TupleElement,
            11 => NodeType::TypeMember,
            12 => NodeType::EnumField,
            13 => NodeType::DependencyItem,
            14 => NodeType::SwitchCase,
            15 => NodeType::Pattern,
            16 => NodeType::PatternField,
            17 => NodeType::AssignPattern,
            18 => NodeType::AssignPatternField,
            19 => NodeType::GenericParameter,
            20 => NodeType::Parameter,
            21 => NodeType::Argument,
            22 => NodeType::Annotation,
            _ => unreachable!("invalid JS node type tag in packed node index"),
        }
    }

    /// Return the stable packed tag for one node type.
    #[inline]
    fn node_type_tag(node_type: NodeType) -> u32 {
        match node_type {
            NodeType::Block => 0,
            NodeType::CatchClause => 1,
            NodeType::Statement => 2,
            NodeType::Expression => 3,
            NodeType::ArrayElement => 4,
            NodeType::Declaration => 5,
            NodeType::Declarator => 6,
            NodeType::Property => 7,
            NodeType::Member => 8,
            NodeType::TypeExpression => 9,
            NodeType::TupleElement => 10,
            NodeType::TypeMember => 11,
            NodeType::EnumField => 12,
            NodeType::DependencyItem => 13,
            NodeType::SwitchCase => 14,
            NodeType::Pattern => 15,
            NodeType::PatternField => 16,
            NodeType::AssignPattern => 17,
            NodeType::AssignPatternField => 18,
            NodeType::GenericParameter => 19,
            NodeType::Parameter => 20,
            NodeType::Argument => 21,
            NodeType::Annotation => 22,
        }
    }
}

/// One stable symbol identity in lowered script output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScriptSymbolId {
    /// One symbol lowered directly from source DIR.
    Source(GlobalSymbolId),
    /// One generated default binding for one non-code script module.
    ModuleDefault(ModuleId),
}

/// One DIR node that produced a JS node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeOrigin {
    /// The origin module.
    pub module_id: ModuleId,
    /// The origin DIR node id.
    pub node_id: u32,
}

/// Mutable AST tree for a single source unit. NOT THREAD-SAFE.
#[derive(Clone, Serialize, Deserialize)]
pub struct Tree {
    /// The next id to allocate.
    pub(crate) next_global_id: u32,
    /// Dense local id and node type metadata by node id.
    pub(crate) node_index_by_node_id: Vec<NodeIndexEntry>,
    /// The annotations attached to nodes.
    pub(crate) annotations_by_node_id: HashMap<u32, Vec<LocalNodeId<Annotation>>>,

    /// The origin node for each JS node.
    pub(crate) origin_by_node_id: Vec<Option<NodeOrigin>>,
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
    pub(crate) type_expressions: Arena<TypeExpression>,
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
    pub(crate) assign_patterns: Arena<AssignPattern>,
    pub(crate) assign_pattern_fields: Arena<AssignPatternField>,
    pub(crate) annotations: Arena<Annotation>,
}

impl Debug for Tree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tree")
            .field("next_global_id", &self.next_global_id)
            .field("node_count", &self.node_index_by_node_id.len())
            .finish()
    }
}

impl Default for Tree {
    fn default() -> Self {
        Self::new()
    }
}

impl Tree {
    /// Create a new Tree.
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Create a new Tree with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            next_global_id: 0,
            node_index_by_node_id: Vec::with_capacity(capacity),
            annotations_by_node_id: HashMap::new(),
            origin_by_node_id: Vec::with_capacity(capacity),
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
            type_expressions: Arena::new(),
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
            assign_patterns: Arena::new(),
            assign_pattern_fields: Arena::new(),
            annotations: Arena::new(),
        }
    }

    /// Allocate a new node in the JS AST tree.
    fn insert<T>(&mut self, node: T, origin: Option<NodeOrigin>) -> LocalNodeId<T>
    where
        T: Node,
        Self: TreeImpl<T>,
    {
        let global_id = self.next_global_id;
        self.next_global_id = global_id + 1;
        let local_id = <Self as TreeImpl<T>>::allocate(self, node);
        self.node_index_by_node_id
            .push(NodeIndexEntry::new(local_id, T::TYPE));
        self.origin_by_node_id.push(origin);
        self.symbol_id_by_node_id.push(None);
        LocalNodeId::new(global_id)
    }

    /// Allocate a generated node in the JS AST tree.
    pub fn insert_generated<T>(&mut self, node: T) -> LocalNodeId<T>
    where
        T: Node,
        Self: TreeImpl<T>,
    {
        self.insert(node, None)
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
        Self: TreeImpl<T>,
        U: dir::Node,
        dir::Tree: dir::TreeStore<U>,
    {
        self.insert(
            node,
            Some(NodeOrigin {
                module_id,
                node_id: dir_node_id.id,
            }),
        )
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
        Self: TreeImpl<T>,
    {
        self.insert(
            node,
            Some(NodeOrigin {
                module_id,
                node_id: dir_node_id.id,
            }),
        )
    }

    /// Allocate a new node in the JS AST tree derived from another DIR node.
    pub fn insert_from<T, U>(&mut self, node: T, dir_node_id: LocalNodeId<U>) -> LocalNodeId<T>
    where
        T: Node,
        Self: TreeImpl<T>,
        U: Node,
    {
        let origin = self.origin_by_node_id[dir_node_id.id as usize];
        let node_id = self.insert(node, origin);

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
        Self: TreeImpl<T>,
    {
        self.alias_node_id_by_dir_id.insert(dir_id, alias.id);
    }

    /// Alias a node in the JS AST tree from a JS AST node.
    pub fn alias_from<T>(&mut self, node_id: u32, alias: LocalNodeId<T>)
    where
        T: Node,
        Self: TreeImpl<T>,
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
        self.node_index_by_node_id[id as usize].node_type()
    }

    /// Get an immutable reference to the node with the given NodeId.
    #[inline]
    pub fn get<T>(&self, id: LocalNodeId<T>) -> &T
    where
        T: Node,
        Self: TreeImpl<T>,
    {
        let local_id = self.local_id_for_node_id(id.id);
        <Self as TreeImpl<T>>::get(self, local_id)
    }

    /// Get a mutable reference to the node with the given NodeId.
    #[inline]
    pub fn get_mut<T>(&mut self, id: LocalNodeId<T>) -> &mut T
    where
        T: Node,
        Self: TreeImpl<T>,
    {
        let local_id = self.local_id_for_node_id(id.id);
        <Self as TreeImpl<T>>::get_mut(self, local_id)
    }

    /// Get the nodes for all nodes of a given type.
    #[inline]
    pub fn get_nodes<T>(&self) -> Vec<LocalNodeId<T>>
    where
        T: Node,
    {
        let mut nodes = Vec::new();
        for (idx, entry) in self.node_index_by_node_id.iter().enumerate() {
            if entry.node_type() == T::TYPE {
                nodes.push(LocalNodeId::new(idx as u32));
            }
        }
        nodes
    }

    /// Return the number of nodes stored in this tree.
    #[inline]
    pub fn node_count(&self) -> usize {
        self.node_index_by_node_id.len()
    }

    /// Return the local arena id for one untyped node id.
    #[inline]
    pub(crate) fn local_id_for_node_id(&self, id: u32) -> u32 {
        self.node_index_by_node_id[id as usize].local_id()
    }

    /// Return the origin node for one JS node when it has one.
    pub fn get_origin(&self, node_id: u32) -> Option<NodeOrigin> {
        self.origin_by_node_id[node_id as usize]
    }

    /// Store one symbol identity for one JS AST node.
    pub fn set_symbol<T>(&mut self, node_id: LocalNodeId<T>, symbol_id: ScriptSymbolId)
    where
        T: Node,
        Self: TreeImpl<T>,
    {
        self.symbol_id_by_node_id[node_id.id as usize] = Some(symbol_id);
    }

    /// Return the symbol identity for one JS AST node when one exists.
    pub fn symbol<T>(&self, node_id: LocalNodeId<T>) -> Option<ScriptSymbolId>
    where
        T: Node,
        Self: TreeImpl<T>,
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
pub trait TreeImpl<T: Node> {
    /// Allocate a node into the relevant arena.
    fn allocate(tree: &mut Tree, node: T) -> u32;
    /// Get a node from the relevant arena.
    fn get(tree: &Tree, idx: u32) -> &T;
    /// Get a mutable node from the relevant arena.
    fn get_mut(tree: &mut Tree, idx: u32) -> &mut T;
}

macro_rules! impl_tree_store {
    ($ty:ty, $field:ident) => {
        impl TreeImpl<$ty> for Tree {
            #[inline]
            fn allocate(tree: &mut Tree, node: $ty) -> u32 {
                tree.$field.allocate(node)
            }

            #[inline]
            fn get(tree: &Tree, idx: u32) -> &$ty {
                tree.$field.get(idx)
            }

            #[inline]
            fn get_mut(tree: &mut Tree, idx: u32) -> &mut $ty {
                tree.$field.get_mut(idx)
            }
        }
    };
}

macro_rules! impl_tree_stores {
    ( $( $ty:ty => $field:ident ),+ $(,)? ) => {
        $( impl_tree_store!($ty, $field); )*
    };
}

impl_tree_stores! {
    Block => blocks,
    CatchClause => catch_clauses,
    Statement => statements,
    Expression => expressions,
    ArrayElement => array_elements,
    Declaration => declarations,
    Declarator => declarators,
    Property => properties,
    Member => members,
    TypeExpression => type_expressions,
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
    AssignPattern => assign_patterns,
    AssignPatternField => assign_pattern_fields,
    Annotation => annotations,
}
