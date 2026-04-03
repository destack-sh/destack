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
use destack_source::{FileId, NodeSourceMap, NodeSpanType, Span};
use serde::{Deserialize, Serialize};

/// One mutable CSS node tree.
#[derive(Clone, Serialize, Deserialize)]
pub struct NodeTree {
    /// The next global node id.
    pub(crate) next_global_id: u32,
    /// The local ids of all nodes.
    pub(crate) local_id_by_node_id: Vec<u32>,
    /// The types of all nodes.
    pub(crate) node_type_by_node_id: Vec<NodeType>,
    /// The source spans for all nodes.
    pub source_map: NodeSourceMap,
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
    /// Create one empty node tree.
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Create one empty node tree with one initial capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            next_global_id: 0,
            local_id_by_node_id: Vec::with_capacity(capacity),
            node_type_by_node_id: Vec::with_capacity(capacity),
            source_map: NodeSourceMap::with_capacity(capacity),
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
        Self: NodeTreeImpl<T>,
    {
        let global_id = self.next_global_id;
        self.next_global_id = global_id + 1;
        self.node_type_by_node_id.push(T::TYPE);
        let local_id = <Self as NodeTreeImpl<T>>::allocate(self, node);
        self.local_id_by_node_id.push(local_id);
        self.source_map.append(span);

        LocalNodeId::new(global_id)
    }

    /// Get one typed node by id.
    pub fn get<T>(&self, id: LocalNodeId<T>) -> &T
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        let local_id = self.local_id_by_node_id[id.id as usize];
        <Self as NodeTreeImpl<T>>::get(self, local_id)
    }

    /// Get one mutable typed node by id.
    pub fn get_mut<T>(&mut self, id: LocalNodeId<T>) -> &mut T
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        let local_id = self.local_id_by_node_id[id.id as usize];
        <Self as NodeTreeImpl<T>>::get_mut(self, local_id)
    }

    /// Return the main span for one node.
    pub fn span<T>(&self, id: LocalNodeId<T>) -> Span
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        self.source_map.get(id.id)
    }

    /// Return one side span for one node when it exists.
    pub fn side_span<T>(&self, id: LocalNodeId<T>, span_type: NodeSpanType) -> Option<Span>
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        self.source_map.get_side(id.id, span_type)
    }

    /// Store one side span for one node.
    pub fn set_side_span<T>(&mut self, id: LocalNodeId<T>, span_type: NodeSpanType, span: Span)
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        self.source_map.set_side(id.id, span_type, span);
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
        self.source_map.rebind_file(file_id);
    }
}

/// Map one node type to its arena.
pub trait NodeTreeImpl<T: Node> {
    /// Allocate one node in the correct arena.
    fn allocate(tree: &mut NodeTree, node: T) -> u32;

    /// Read one node from the correct arena.
    fn get(tree: &NodeTree, index: u32) -> &T;

    /// Mutably read one node from the correct arena.
    fn get_mut(tree: &mut NodeTree, index: u32) -> &mut T;
}

macro_rules! impl_node_tree_store {
    ($ty:ty, $field:ident) => {
        impl NodeTreeImpl<$ty> for NodeTree {
            fn allocate(tree: &mut NodeTree, node: $ty) -> u32 {
                tree.$field.allocate(node)
            }

            fn get(tree: &NodeTree, index: u32) -> &$ty {
                tree.$field.get(index)
            }

            fn get_mut(tree: &mut NodeTree, index: u32) -> &mut $ty {
                tree.$field.get_mut(index)
            }
        }
    };
}

impl_node_tree_store!(Stylesheet, stylesheets);
impl_node_tree_store!(ComponentFragment, component_fragments);
impl_node_tree_store!(Rule, rules);
impl_node_tree_store!(PageMarginRule, page_margin_rules);
impl_node_tree_store!(DeclarationBlock, declaration_blocks);
impl_node_tree_store!(Declaration, declarations);
impl_node_tree_store!(SelectorList, selector_lists);
impl_node_tree_store!(Selector, selectors);
impl_node_tree_store!(SelectorComponent, selector_components);
impl_node_tree_store!(SimpleSelector, simple_selectors);
impl_node_tree_store!(AttributeSelector, attribute_selectors);
impl_node_tree_store!(NthSelector, nth_selectors);
impl_node_tree_store!(NthOfSelector, nth_of_selectors);
impl_node_tree_store!(PseudoClass, pseudo_classes);
impl_node_tree_store!(AnySelector, any_selectors);
impl_node_tree_store!(PseudoElement, pseudo_elements);
impl_node_tree_store!(MediaQueryList, media_query_lists);
impl_node_tree_store!(MediaQuery, media_queries);
impl_node_tree_store!(MediaCondition, media_conditions);
impl_node_tree_store!(FeatureName, feature_names);
impl_node_tree_store!(QueryFeature, query_features);
impl_node_tree_store!(FeatureValue, feature_values);
impl_node_tree_store!(RatioValue, ratio_values);
impl_node_tree_store!(EnvironmentVariable, environment_variables);
impl_node_tree_store!(SupportsCondition, supports_conditions);
impl_node_tree_store!(ContainerCondition, container_conditions);
impl_node_tree_store!(ContainerStyleQuery, container_style_queries);
impl_node_tree_store!(ContainerScrollStateQuery, container_scroll_state_queries);
