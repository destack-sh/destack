use std::fmt::{Debug, Formatter};

use destack_core::Arena;
use destack_serde::Reflect;
use destack_source::{ProvenanceId, ProvenanceJournal};
use serde::{Deserialize, Serialize};

use crate::{
    Argument, ArrayAssignPatternField, ArrayElement, ArrayPatternField, AssignPattern, Block,
    CatchClause, Declaration, Declarator, ExportSpecifier, Expression, ImportAttribute,
    ImportSpecifier, LocalNodeId, LocalNodeIdAny, Member, Node, NodeType, ObjectAssignPatternField,
    ObjectPatternField, Parameter, Pattern, Place, Property, ReExportSpecifier, Statement,
    SwitchCase,
};

/// Dense metadata for one JS node id.
#[derive(Debug, Copy, Clone, Serialize, Deserialize, Reflect)]
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
        assert!(
            local_id <= Self::LOCAL_ID_MASK,
            "JS node local id exceeds packed index capacity: {local_id}"
        );

        Self {
            packed: local_id | ((node_type as u32) << Self::NODE_TYPE_SHIFT),
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
            1 => NodeType::Block,
            2 => NodeType::CatchClause,
            3 => NodeType::Statement,
            4 => NodeType::Expression,
            5 => NodeType::ArrayElement,
            6 => NodeType::Declaration,
            7 => NodeType::Declarator,
            8 => NodeType::Property,
            9 => NodeType::Member,
            10 => NodeType::ImportSpecifier,
            11 => NodeType::ExportSpecifier,
            12 => NodeType::ReExportSpecifier,
            13 => NodeType::ImportAttribute,
            14 => NodeType::SwitchCase,
            15 => NodeType::Pattern,
            16 => NodeType::ArrayPatternField,
            17 => NodeType::ObjectPatternField,
            18 => NodeType::Place,
            19 => NodeType::AssignPattern,
            20 => NodeType::ArrayAssignPatternField,
            21 => NodeType::ObjectAssignPatternField,
            22 => NodeType::Parameter,
            23 => NodeType::Argument,
            _ => unreachable!("invalid JS node type tag in packed node index"),
        }
    }
}

/// One mutable JavaScript tree.
#[derive(Clone, Serialize, Deserialize, Reflect)]
pub struct Tree {
    /// Dense local id and node type metadata by node id.
    pub(crate) node_index: Vec<NodeIndexEntry>,
    /// The provenance of each JS node.
    pub(crate) provenance: Vec<ProvenanceId>,

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
    pub(crate) import_specifiers: Arena<ImportSpecifier>,
    pub(crate) export_specifiers: Arena<ExportSpecifier>,
    pub(crate) re_export_specifiers: Arena<ReExportSpecifier>,
    pub(crate) import_attributes: Arena<ImportAttribute>,
    pub(crate) switch_cases: Arena<SwitchCase>,
    pub(crate) parameters: Arena<Parameter>,
    pub(crate) arguments: Arena<Argument>,
    pub(crate) patterns: Arena<Pattern>,
    pub(crate) array_pattern_fields: Arena<ArrayPatternField>,
    pub(crate) object_pattern_fields: Arena<ObjectPatternField>,
    pub(crate) places: Arena<Place>,
    pub(crate) assign_patterns: Arena<AssignPattern>,
    pub(crate) array_assign_pattern_fields: Arena<ArrayAssignPatternField>,
    pub(crate) object_assign_pattern_fields: Arena<ObjectAssignPatternField>,
}

impl Debug for Tree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tree")
            .field("node_count", &self.node_index.len())
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
            node_index: Vec::with_capacity(capacity),
            provenance: Vec::with_capacity(capacity),
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
            import_specifiers: Arena::new(),
            export_specifiers: Arena::new(),
            re_export_specifiers: Arena::new(),
            import_attributes: Arena::new(),
            switch_cases: Arena::new(),
            parameters: Arena::new(),
            arguments: Arena::new(),
            patterns: Arena::new(),
            array_pattern_fields: Arena::new(),
            object_pattern_fields: Arena::new(),
            places: Arena::new(),
            assign_patterns: Arena::new(),
            array_assign_pattern_fields: Arena::new(),
            object_assign_pattern_fields: Arena::new(),
        }
    }

    /// Allocate one JS node with an explicit provenance.
    pub fn insert<T>(&mut self, node: T, provenance: ProvenanceId) -> LocalNodeId<T>
    where
        T: Node,
        Self: TreeStore<T>,
    {
        let node_id = self.node_index.len() as u32;
        let local_id = <Self as TreeStore<T>>::allocate(self, node);
        self.node_index.push(NodeIndexEntry::new(local_id, T::TYPE));
        self.provenance.push(provenance);
        LocalNodeId::new(node_id)
    }

    /// Allocate one JS node derived from another.
    pub fn insert_from<T, U>(
        &mut self,
        node: T,
        source_id: LocalNodeId<U>,
        provenance: &mut ProvenanceJournal<'_>,
    ) -> LocalNodeId<T>
    where
        T: Node,
        Self: TreeStore<T>,
        U: Node,
    {
        let source = self.provenance(source_id);
        self.insert(node, provenance.derive(source))
    }

    /// Replace one JS node and record the transformation that produced it.
    pub fn rewrite<T>(
        &mut self,
        node_id: LocalNodeId<T>,
        replacement: T,
        provenance: &mut ProvenanceJournal<'_>,
    ) where
        T: Node,
        Self: TreeStore<T>,
    {
        let node_index = node_id.id as usize;
        let source = self.provenance(node_id);
        *self.get_mut(node_id) = replacement;
        self.provenance[node_index] = provenance.derive(source);
    }

    /// Return one mutable JS node and record its transformation.
    pub fn edit<T>(
        &mut self,
        node_id: LocalNodeId<T>,
        provenance: &mut ProvenanceJournal<'_>,
    ) -> &mut T
    where
        T: Node,
        Self: TreeStore<T>,
    {
        let node_index = node_id.id as usize;
        let source = self.provenance(node_id);
        self.provenance[node_index] = provenance.derive(source);

        self.get_mut(node_id)
    }

    /// Record one JS node as removed by the active transform.
    pub fn record_removal<T>(&self, node_id: LocalNodeId<T>, provenance: &mut ProvenanceJournal<'_>)
    where
        T: Node,
    {
        provenance.remove(&[self.provenance(node_id)]);
    }

    /// Return the type of one untyped node id.
    #[inline]
    pub fn node_type(&self, id: LocalNodeIdAny) -> NodeType {
        self.node_index[id.id as usize].node_type()
    }

    /// Return one immutable node.
    #[inline]
    pub fn get<T>(&self, id: LocalNodeId<T>) -> &T
    where
        T: Node,
        Self: TreeStore<T>,
    {
        let entry = self.node_index[id.id as usize];
        debug_assert_eq!(entry.node_type(), T::TYPE);
        let local_id = entry.local_id();

        <Self as TreeStore<T>>::get(self, local_id)
    }

    /// Return one mutable node.
    #[inline]
    fn get_mut<T>(&mut self, id: LocalNodeId<T>) -> &mut T
    where
        T: Node,
        Self: TreeStore<T>,
    {
        let entry = self.node_index[id.id as usize];
        debug_assert_eq!(entry.node_type(), T::TYPE);
        let local_id = entry.local_id();

        <Self as TreeStore<T>>::get_mut(self, local_id)
    }

    /// Return every node of one type.
    #[inline]
    pub fn nodes<T>(&self) -> impl Iterator<Item = LocalNodeId<T>> + '_
    where
        T: Node,
    {
        self.node_index
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.node_type() == T::TYPE)
            .map(|(index, _)| LocalNodeId::new(index as u32))
    }

    /// Return the number of nodes stored in this tree.
    #[inline]
    pub fn node_count(&self) -> usize {
        self.node_index.len()
    }

    /// Return the provenance of one JS node.
    pub fn provenance(&self, node: impl Into<LocalNodeIdAny>) -> ProvenanceId {
        let node = node.into();

        self.provenance[node.id as usize]
    }
}

/// Map node types to arenas.
pub trait TreeStore<T: Node> {
    /// Allocate a node into the relevant arena.
    fn allocate(tree: &mut Tree, node: T) -> u32;
    /// Return a node from the relevant arena.
    fn get(tree: &Tree, index: u32) -> &T;
    /// Return a mutable node from the relevant arena.
    fn get_mut(tree: &mut Tree, index: u32) -> &mut T;
}

macro_rules! impl_tree_store {
    ($ty:ty, $field:ident) => {
        impl TreeStore<$ty> for Tree {
            #[inline]
            fn allocate(tree: &mut Tree, node: $ty) -> u32 {
                tree.$field.allocate(node)
            }

            #[inline]
            fn get(tree: &Tree, index: u32) -> &$ty {
                tree.$field.get(index)
            }

            #[inline]
            fn get_mut(tree: &mut Tree, index: u32) -> &mut $ty {
                tree.$field.get_mut(index)
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
    ImportSpecifier => import_specifiers,
    ExportSpecifier => export_specifiers,
    ReExportSpecifier => re_export_specifiers,
    ImportAttribute => import_attributes,
    SwitchCase => switch_cases,
    Parameter => parameters,
    Argument => arguments,
    Pattern => patterns,
    ArrayPatternField => array_pattern_fields,
    ObjectPatternField => object_pattern_fields,
    Place => places,
    AssignPattern => assign_patterns,
    ArrayAssignPatternField => array_assign_pattern_fields,
    ObjectAssignPatternField => object_assign_pattern_fields,
}
