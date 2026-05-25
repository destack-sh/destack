use std::fmt::{Debug, Formatter};

use crate::{
    AnySelector, AttributeSelector, ComponentFragment, ContainerCondition,
    ContainerScrollStateQuery, ContainerStyleQuery, Declaration, DeclarationBlock,
    EnvironmentVariable, FeatureName, FeatureValue, LocalNodeId, MediaCondition, MediaQuery,
    MediaQueryList, Node, NodeType, NthOfSelector, NthSelector, PageMarginRule, PseudoClass,
    PseudoElement, QueryFeature, RatioValue, Rule, Selector, SelectorComponent, SelectorList,
    SimpleSelector, Stylesheet, SupportsCondition,
};
use destack_core::{Arena, StringId, StringPool, StringRef};
use destack_source::{FileId, NodeSpanType, SourceIndex, Span};
use serde::{Deserialize, Serialize};

/// Dense metadata for one CSS node id.
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
            "CSS node local id exceeds packed index capacity: {local_id}"
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
            0 => NodeType::Stylesheet,
            1 => NodeType::ComponentFragment,
            2 => NodeType::Rule,
            3 => NodeType::PageMarginRule,
            4 => NodeType::DeclarationBlock,
            5 => NodeType::Declaration,
            6 => NodeType::SelectorList,
            7 => NodeType::Selector,
            8 => NodeType::SelectorComponent,
            9 => NodeType::SimpleSelector,
            10 => NodeType::AttributeSelector,
            11 => NodeType::NthSelector,
            12 => NodeType::NthOfSelector,
            13 => NodeType::PseudoClass,
            14 => NodeType::AnySelector,
            15 => NodeType::PseudoElement,
            16 => NodeType::MediaQueryList,
            17 => NodeType::MediaQuery,
            18 => NodeType::MediaCondition,
            19 => NodeType::FeatureName,
            20 => NodeType::QueryFeature,
            21 => NodeType::FeatureValue,
            22 => NodeType::RatioValue,
            23 => NodeType::EnvironmentVariable,
            24 => NodeType::SupportsCondition,
            25 => NodeType::ContainerCondition,
            26 => NodeType::ContainerStyleQuery,
            27 => NodeType::ContainerScrollStateQuery,
            _ => unreachable!("invalid CSS node type tag in packed node index"),
        }
    }
}

/// One mutable CSS tree.
#[derive(Clone, Serialize, Deserialize)]
pub struct Tree {
    /// The next global node id.
    pub(crate) next_global_id: u32,
    /// Dense local id and node type metadata by node id.
    pub(crate) node_index_by_node_id: Vec<NodeIndexEntry>,
    /// The source ranges and anchors for all nodes.
    pub source_index: SourceIndex,
    /// The interned strings used by pooled css identifiers.
    pub strings: StringPool,

    // node arenas
    pub(crate) stylesheets: Arena<Stylesheet>,
    pub(crate) component_fragments: Arena<ComponentFragment>,
    pub(crate) rules: Arena<Rule>,
    pub(crate) page_margin_rules: Arena<PageMarginRule>,
    pub(crate) declaration_blocks: Arena<DeclarationBlock>,
    pub(crate) declarations: Arena<Declaration>,
    pub(crate) selector_lists: Arena<SelectorList>,
    pub(crate) selectors: Arena<Selector>,
    pub(crate) selector_components: Arena<SelectorComponent>,
    pub(crate) simple_selectors: Arena<SimpleSelector>,
    pub(crate) attribute_selectors: Arena<AttributeSelector>,
    pub(crate) nth_selectors: Arena<NthSelector>,
    pub(crate) nth_of_selectors: Arena<NthOfSelector>,
    pub(crate) pseudo_classes: Arena<PseudoClass>,
    pub(crate) any_selectors: Arena<AnySelector>,
    pub(crate) pseudo_elements: Arena<PseudoElement>,
    pub(crate) media_query_lists: Arena<MediaQueryList>,
    pub(crate) media_queries: Arena<MediaQuery>,
    pub(crate) media_conditions: Arena<MediaCondition>,
    pub(crate) feature_names: Arena<FeatureName>,
    pub(crate) query_features: Arena<QueryFeature>,
    pub(crate) feature_values: Arena<FeatureValue>,
    pub(crate) ratio_values: Arena<RatioValue>,
    pub(crate) environment_variables: Arena<EnvironmentVariable>,
    pub(crate) supports_conditions: Arena<SupportsCondition>,
    pub(crate) container_conditions: Arena<ContainerCondition>,
    pub(crate) container_style_queries: Arena<ContainerStyleQuery>,
    pub(crate) container_scroll_state_queries: Arena<ContainerScrollStateQuery>,
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
    /// Create one empty tree.
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Create one empty tree with one initial capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            next_global_id: 0,
            node_index_by_node_id: Vec::with_capacity(capacity),
            source_index: SourceIndex::with_capacity(capacity),
            strings: StringPool::new(),
            stylesheets: Arena::new(),
            component_fragments: Arena::new(),
            rules: Arena::new(),
            page_margin_rules: Arena::new(),
            declaration_blocks: Arena::new(),
            declarations: Arena::new(),
            selector_lists: Arena::new(),
            selectors: Arena::new(),
            selector_components: Arena::new(),
            simple_selectors: Arena::new(),
            attribute_selectors: Arena::new(),
            nth_selectors: Arena::new(),
            nth_of_selectors: Arena::new(),
            pseudo_classes: Arena::new(),
            any_selectors: Arena::new(),
            pseudo_elements: Arena::new(),
            media_query_lists: Arena::new(),
            media_queries: Arena::new(),
            media_conditions: Arena::new(),
            feature_names: Arena::new(),
            query_features: Arena::new(),
            feature_values: Arena::new(),
            ratio_values: Arena::new(),
            environment_variables: Arena::new(),
            supports_conditions: Arena::new(),
            container_conditions: Arena::new(),
            container_style_queries: Arena::new(),
            container_scroll_state_queries: Arena::new(),
        }
    }

    /// Insert one node with one main span.
    pub fn insert<T>(&mut self, node: T, span: Span) -> LocalNodeId<T>
    where
        T: Node,
        Self: TreeImpl<T>,
    {
        let global_id = self.next_global_id;
        self.next_global_id = global_id + 1;
        let local_id = <Self as TreeImpl<T>>::allocate(self, node);
        self.node_index_by_node_id
            .push(NodeIndexEntry::new(local_id, T::TYPE));
        self.source_index.append(span);

        LocalNodeId::new(global_id)
    }

    /// Get one typed node by id.
    pub fn get<T>(&self, id: LocalNodeId<T>) -> &T
    where
        T: Node,
        Self: TreeImpl<T>,
    {
        let local_id = self.local_id_for_node_id(id.id);
        <Self as TreeImpl<T>>::get(self, local_id)
    }

    /// Get one mutable typed node by id.
    pub fn get_mut<T>(&mut self, id: LocalNodeId<T>) -> &mut T
    where
        T: Node,
        Self: TreeImpl<T>,
    {
        let local_id = self.local_id_for_node_id(id.id);
        <Self as TreeImpl<T>>::get_mut(self, local_id)
    }

    /// Return the main span for one node.
    pub fn span<T>(&self, id: LocalNodeId<T>) -> Span
    where
        T: Node,
        Self: TreeImpl<T>,
    {
        self.source_index.get(id.id)
    }

    /// Return one side span for one node when it exists.
    pub fn side_span<T>(&self, id: LocalNodeId<T>, span_type: NodeSpanType) -> Option<Span>
    where
        T: Node,
        Self: TreeImpl<T>,
    {
        self.source_index.get_side(id.id, span_type)
    }

    /// Store one side span for one node.
    pub fn set_side_span<T>(&mut self, id: LocalNodeId<T>, span_type: NodeSpanType, span: Span)
    where
        T: Node,
        Self: TreeImpl<T>,
    {
        self.source_index.set_side(id.id, span_type, span);
    }

    /// Return the number of nodes stored in this tree.
    #[inline]
    pub fn node_count(&self) -> usize {
        self.node_index_by_node_id.len()
    }

    /// Return the type of one untyped node id.
    #[inline]
    pub fn get_node_type(&self, id: u32) -> NodeType {
        self.node_index_by_node_id[id as usize].node_type()
    }

    /// Return the local arena id for one untyped node id.
    #[inline]
    pub(crate) fn local_id_for_node_id(&self, id: u32) -> u32 {
        self.node_index_by_node_id[id as usize].local_id()
    }

    /// Intern one pooled css string.
    pub fn intern(&self, value: &str) -> StringId {
        self.strings.intern(value)
    }

    /// Return one pooled css string.
    pub fn string(&self, id: StringId) -> StringRef<'_> {
        self.strings.get(id)
    }

    /// Rebind every stored span to one file id.
    pub fn rebind_file(&mut self, file_id: FileId) {
        self.source_index.rebind_file(file_id);
    }
}

/// Map one node type to its arena.
pub trait TreeImpl<T: Node> {
    /// Allocate one node in the correct arena.
    fn allocate(tree: &mut Tree, node: T) -> u32;

    /// Read one node from the correct arena.
    fn get(tree: &Tree, index: u32) -> &T;

    /// Mutably read one node from the correct arena.
    fn get_mut(tree: &mut Tree, index: u32) -> &mut T;
}

macro_rules! impl_tree_store {
    ($ty:ty, $field:ident) => {
        impl TreeImpl<$ty> for Tree {
            fn allocate(tree: &mut Tree, node: $ty) -> u32 {
                tree.$field.allocate(node)
            }

            fn get(tree: &Tree, index: u32) -> &$ty {
                tree.$field.get(index)
            }

            fn get_mut(tree: &mut Tree, index: u32) -> &mut $ty {
                tree.$field.get_mut(index)
            }
        }
    };
}

impl_tree_store!(Stylesheet, stylesheets);
impl_tree_store!(ComponentFragment, component_fragments);
impl_tree_store!(Rule, rules);
impl_tree_store!(PageMarginRule, page_margin_rules);
impl_tree_store!(DeclarationBlock, declaration_blocks);
impl_tree_store!(Declaration, declarations);
impl_tree_store!(SelectorList, selector_lists);
impl_tree_store!(Selector, selectors);
impl_tree_store!(SelectorComponent, selector_components);
impl_tree_store!(SimpleSelector, simple_selectors);
impl_tree_store!(AttributeSelector, attribute_selectors);
impl_tree_store!(NthSelector, nth_selectors);
impl_tree_store!(NthOfSelector, nth_of_selectors);
impl_tree_store!(PseudoClass, pseudo_classes);
impl_tree_store!(AnySelector, any_selectors);
impl_tree_store!(PseudoElement, pseudo_elements);
impl_tree_store!(MediaQueryList, media_query_lists);
impl_tree_store!(MediaQuery, media_queries);
impl_tree_store!(MediaCondition, media_conditions);
impl_tree_store!(FeatureName, feature_names);
impl_tree_store!(QueryFeature, query_features);
impl_tree_store!(FeatureValue, feature_values);
impl_tree_store!(RatioValue, ratio_values);
impl_tree_store!(EnvironmentVariable, environment_variables);
impl_tree_store!(SupportsCondition, supports_conditions);
impl_tree_store!(ContainerCondition, container_conditions);
impl_tree_store!(ContainerStyleQuery, container_style_queries);
impl_tree_store!(ContainerScrollStateQuery, container_scroll_state_queries);
