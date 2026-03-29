use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

/// The type of one CSS node.
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum NodeType {
    StyleSheet,
    Rule,
    PageMarginRule,
    DeclarationBlock,
    Declaration,
    SelectorList,
    Selector,
    SelectorComponent,
    SimpleSelector,
    AttributeSelector,
    NthSelector,
    NthOfSelector,
    PseudoClass,
    AnySelector,
    PseudoElement,
    MediaQueryList,
    MediaQuery,
    MediaCondition,
    FeatureName,
    QueryFeature,
    FeatureValue,
    RatioValue,
    EnvironmentVariable,
    SupportsCondition,
    ContainerCondition,
    ContainerStyleQuery,
    ContainerScrollStateQuery,
}

impl NodeType {
    /// Return the human readable node type name.
    pub fn name(&self) -> &'static str {
        match self {
            NodeType::StyleSheet => "stylesheet",
            NodeType::Rule => "rule",
            NodeType::PageMarginRule => "page margin rule",
            NodeType::DeclarationBlock => "declaration block",
            NodeType::Declaration => "declaration",
            NodeType::SelectorList => "selector list",
            NodeType::Selector => "selector",
            NodeType::SelectorComponent => "selector component",
            NodeType::SimpleSelector => "simple selector",
            NodeType::AttributeSelector => "attribute selector",
            NodeType::NthSelector => "nth selector",
            NodeType::NthOfSelector => "nth of selector",
            NodeType::PseudoClass => "pseudo class",
            NodeType::AnySelector => "any selector",
            NodeType::PseudoElement => "pseudo element",
            NodeType::MediaQueryList => "media query list",
            NodeType::MediaQuery => "media query",
            NodeType::MediaCondition => "media condition",
            NodeType::FeatureName => "feature name",
            NodeType::QueryFeature => "query feature",
            NodeType::FeatureValue => "feature value",
            NodeType::RatioValue => "ratio value",
            NodeType::EnvironmentVariable => "environment variable",
            NodeType::SupportsCondition => "supports condition",
            NodeType::ContainerCondition => "container condition",
            NodeType::ContainerStyleQuery => "container style query",
            NodeType::ContainerScrollStateQuery => "container scroll state query",
        }
    }
}

/// One untyped local node id.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalNodeIdAny {
    /// The raw node id.
    pub id: u32,
    /// The node type.
    pub ty: NodeType,
}

impl LocalNodeIdAny {
    /// Create one untyped local node id.
    pub fn new(id: u32, ty: NodeType) -> Self {
        Self { id, ty }
    }
}

impl Debug for LocalNodeIdAny {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalNodeIdAny")
            .field("id", &self.id)
            .field("type", &self.ty)
            .finish()
    }
}

/// One typed local node id.
#[repr(transparent)]
#[derive(Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalNodeId<T: Node> {
    /// The raw node id.
    pub id: u32,
    #[serde(skip)]
    _ty: PhantomData<fn() -> T>,
}

impl<T: Node> LocalNodeId<T> {
    /// Create one typed local node id.
    pub fn new(id: u32) -> Self {
        Self {
            id,
            _ty: PhantomData,
        }
    }

    /// Convert this node id into its untyped form.
    pub fn into_any(self) -> LocalNodeIdAny {
        LocalNodeIdAny {
            id: self.id,
            ty: T::TYPE,
        }
    }
}

impl<T: Node> Debug for LocalNodeId<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalNodeId").field("id", &self.id).finish()
    }
}

impl<T: Clone + Node> Copy for LocalNodeId<T> {}

impl<T: Node> From<LocalNodeId<T>> for LocalNodeIdAny {
    fn from(id: LocalNodeId<T>) -> Self {
        id.into_any()
    }
}

/// One typed node.
pub trait Node: Sized {
    const TYPE: NodeType;
}
