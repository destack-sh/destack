#![allow(unused_variables)]

use super::walk::{
    walk_any_selector, walk_attribute_selector, walk_component_fragment, walk_container_condition,
    walk_container_scroll_state_query, walk_container_style_query, walk_declaration,
    walk_declaration_block, walk_environment_variable, walk_feature_name, walk_feature_value,
    walk_media_condition, walk_media_query, walk_media_query_list, walk_nth_of_selector,
    walk_nth_selector, walk_page_margin_rule, walk_pseudo_class, walk_pseudo_element,
    walk_query_feature, walk_ratio_value, walk_rule, walk_selector, walk_selector_component,
    walk_selector_list, walk_simple_selector, walk_stylesheet, walk_supports_condition,
};
use crate::{
    AnySelector, AttributeSelector, ComponentFragment, ContainerCondition,
    ContainerScrollStateQuery, ContainerStyleQuery, Declaration, DeclarationBlock,
    EnvironmentVariable, FeatureName, FeatureValue, LocalNodeId, MediaCondition, MediaQuery,
    MediaQueryList, NodeType, NthOfSelector, NthSelector, PageMarginRule, PseudoClass,
    PseudoElement, QueryFeature, RatioValue, Rule, Selector, SelectorComponent, SelectorList,
    SimpleSelector, Stylesheet, SupportsCondition, Tree,
};

/// One CSS node visitor configuration.
#[derive(Debug, Clone, Default)]
pub struct NodeVisitorOptions {}

/// One CSS node visitor.
pub trait NodeVisitor {
    /// Return the visitor options.
    fn options(&self) -> &NodeVisitorOptions;

    /// Visit one arbitrary node id.
    #[inline]
    fn visit_any(&mut self, tree: &Tree, ty: NodeType, id: u32) {}

    /// Visit one stylesheet node.
    fn visit_stylesheet(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Stylesheet>,
        stylesheet: &Stylesheet,
    ) {
        destack_core::ensure_sufficient_stack(|| walk_stylesheet(self, tree, id, stylesheet));
    }

    /// Visit one component fragment root.
    fn visit_component_fragment(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<ComponentFragment>,
        fragment: &ComponentFragment,
    ) {
        walk_component_fragment(self, tree, id, fragment);
    }

    /// Visit one CSS rule node.
    fn visit_rule(&mut self, tree: &Tree, id: LocalNodeId<Rule>, rule: &Rule) {
        destack_core::ensure_sufficient_stack(|| walk_rule(self, tree, id, rule));
    }

    /// Visit one page margin rule node.
    fn visit_page_margin_rule(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<PageMarginRule>,
        rule: &PageMarginRule,
    ) {
        walk_page_margin_rule(self, tree, id, rule);
    }

    /// Visit one declaration block node.
    fn visit_declaration_block(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<DeclarationBlock>,
        declarations: &DeclarationBlock,
    ) {
        walk_declaration_block(self, tree, id, declarations);
    }

    /// Visit one declaration node.
    fn visit_declaration(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Declaration>,
        declaration: &Declaration,
    ) {
        walk_declaration(self, tree, id, declaration);
    }

    /// Visit one selector list node.
    fn visit_selector_list(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<SelectorList>,
        selector_list: &SelectorList,
    ) {
        walk_selector_list(self, tree, id, selector_list);
    }

    /// Visit one selector node.
    fn visit_selector(&mut self, tree: &Tree, id: LocalNodeId<Selector>, selector: &Selector) {
        walk_selector(self, tree, id, selector);
    }

    /// Visit one selector component node.
    fn visit_selector_component(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<SelectorComponent>,
        component: &SelectorComponent,
    ) {
        walk_selector_component(self, tree, id, component);
    }

    /// Visit one simple selector node.
    fn visit_simple_selector(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<SimpleSelector>,
        selector: &SimpleSelector,
    ) {
        walk_simple_selector(self, tree, id, selector);
    }

    /// Visit one attribute selector node.
    fn visit_attribute_selector(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<AttributeSelector>,
        selector: &AttributeSelector,
    ) {
        walk_attribute_selector(self, tree, id, selector);
    }

    /// Visit one nth selector node.
    fn visit_nth_selector(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<NthSelector>,
        selector: &NthSelector,
    ) {
        walk_nth_selector(self, tree, id, selector);
    }

    /// Visit one nth-of selector node.
    fn visit_nth_of_selector(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<NthOfSelector>,
        selector: &NthOfSelector,
    ) {
        walk_nth_of_selector(self, tree, id, selector);
    }

    /// Visit one pseudo class node.
    fn visit_pseudo_class(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<PseudoClass>,
        selector: &PseudoClass,
    ) {
        walk_pseudo_class(self, tree, id, selector);
    }

    /// Visit one vendor any selector node.
    fn visit_any_selector(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<AnySelector>,
        selector: &AnySelector,
    ) {
        walk_any_selector(self, tree, id, selector);
    }

    /// Visit one pseudo element node.
    fn visit_pseudo_element(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<PseudoElement>,
        selector: &PseudoElement,
    ) {
        walk_pseudo_element(self, tree, id, selector);
    }

    /// Visit one media query list node.
    fn visit_media_query_list(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<MediaQueryList>,
        media_query_list: &MediaQueryList,
    ) {
        walk_media_query_list(self, tree, id, media_query_list);
    }

    /// Visit one media query node.
    fn visit_media_query(&mut self, tree: &Tree, id: LocalNodeId<MediaQuery>, query: &MediaQuery) {
        walk_media_query(self, tree, id, query);
    }

    /// Visit one media condition node.
    fn visit_media_condition(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<MediaCondition>,
        condition: &MediaCondition,
    ) {
        walk_media_condition(self, tree, id, condition);
    }

    /// Visit one feature name node.
    fn visit_feature_name(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<FeatureName>,
        name: &FeatureName,
    ) {
        walk_feature_name(self, tree, id, name);
    }

    /// Visit one query feature node.
    fn visit_query_feature(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<QueryFeature>,
        feature: &QueryFeature,
    ) {
        walk_query_feature(self, tree, id, feature);
    }

    /// Visit one feature value node.
    fn visit_feature_value(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<FeatureValue>,
        value: &FeatureValue,
    ) {
        walk_feature_value(self, tree, id, value);
    }

    /// Visit one ratio value node.
    fn visit_ratio_value(&mut self, tree: &Tree, id: LocalNodeId<RatioValue>, value: &RatioValue) {
        walk_ratio_value(self, tree, id, value);
    }

    /// Visit one environment variable node.
    fn visit_environment_variable(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<EnvironmentVariable>,
        value: &EnvironmentVariable,
    ) {
        walk_environment_variable(self, tree, id, value);
    }

    /// Visit one supports condition node.
    fn visit_supports_condition(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<SupportsCondition>,
        condition: &SupportsCondition,
    ) {
        walk_supports_condition(self, tree, id, condition);
    }

    /// Visit one container condition node.
    fn visit_container_condition(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<ContainerCondition>,
        condition: &ContainerCondition,
    ) {
        walk_container_condition(self, tree, id, condition);
    }

    /// Visit one container style query node.
    fn visit_container_style_query(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<ContainerStyleQuery>,
        query: &ContainerStyleQuery,
    ) {
        walk_container_style_query(self, tree, id, query);
    }

    /// Visit one container scroll state query node.
    fn visit_container_scroll_state_query(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<ContainerScrollStateQuery>,
        query: &ContainerScrollStateQuery,
    ) {
        walk_container_scroll_state_query(self, tree, id, query);
    }
}

/// One capturing CSS node visitor.
#[derive(Debug, Clone, Default)]
pub struct CapturingNodeVisitor {
    /// The visited node ids.
    visited: Vec<u32>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl CapturingNodeVisitor {
    /// Build one capturing node visitor.
    pub fn new(options: NodeVisitorOptions) -> Self {
        Self {
            visited: Vec::new(),
            options,
        }
    }

    /// Reset the visited node ids.
    pub fn reset(&mut self) {
        self.visited.clear();
    }

    /// Return the visited node ids.
    pub fn visited(&self) -> &[u32] {
        &self.visited
    }
}

impl NodeVisitor for CapturingNodeVisitor {
    #[inline]
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_any(&mut self, _tree: &Tree, _ty: NodeType, id: u32) {
        self.visited.push(id);
    }

    fn visit_stylesheet(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Stylesheet>,
        _stylesheet: &Stylesheet,
    ) {
        self.visit_any(tree, NodeType::Stylesheet, id.id);
    }

    fn visit_rule(&mut self, tree: &Tree, id: LocalNodeId<Rule>, _rule: &Rule) {
        self.visit_any(tree, NodeType::Rule, id.id);
    }

    fn visit_page_margin_rule(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<PageMarginRule>,
        _rule: &PageMarginRule,
    ) {
        self.visit_any(tree, NodeType::PageMarginRule, id.id);
    }

    fn visit_declaration_block(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<DeclarationBlock>,
        _declarations: &DeclarationBlock,
    ) {
        self.visit_any(tree, NodeType::DeclarationBlock, id.id);
    }

    fn visit_declaration(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Declaration>,
        _declaration: &Declaration,
    ) {
        self.visit_any(tree, NodeType::Declaration, id.id);
    }

    fn visit_selector_list(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<SelectorList>,
        _selector_list: &SelectorList,
    ) {
        self.visit_any(tree, NodeType::SelectorList, id.id);
    }

    fn visit_selector(&mut self, tree: &Tree, id: LocalNodeId<Selector>, _selector: &Selector) {
        self.visit_any(tree, NodeType::Selector, id.id);
    }

    fn visit_selector_component(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<SelectorComponent>,
        _component: &SelectorComponent,
    ) {
        self.visit_any(tree, NodeType::SelectorComponent, id.id);
    }

    fn visit_simple_selector(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<SimpleSelector>,
        _selector: &SimpleSelector,
    ) {
        self.visit_any(tree, NodeType::SimpleSelector, id.id);
    }

    fn visit_attribute_selector(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<AttributeSelector>,
        _selector: &AttributeSelector,
    ) {
        self.visit_any(tree, NodeType::AttributeSelector, id.id);
    }

    fn visit_nth_selector(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<NthSelector>,
        _selector: &NthSelector,
    ) {
        self.visit_any(tree, NodeType::NthSelector, id.id);
    }

    fn visit_nth_of_selector(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<NthOfSelector>,
        _selector: &NthOfSelector,
    ) {
        self.visit_any(tree, NodeType::NthOfSelector, id.id);
    }

    fn visit_pseudo_class(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<PseudoClass>,
        _selector: &PseudoClass,
    ) {
        self.visit_any(tree, NodeType::PseudoClass, id.id);
    }

    fn visit_any_selector(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<AnySelector>,
        _selector: &AnySelector,
    ) {
        self.visit_any(tree, NodeType::AnySelector, id.id);
    }

    fn visit_pseudo_element(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<PseudoElement>,
        _selector: &PseudoElement,
    ) {
        self.visit_any(tree, NodeType::PseudoElement, id.id);
    }

    fn visit_media_query_list(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<MediaQueryList>,
        _media_query_list: &MediaQueryList,
    ) {
        self.visit_any(tree, NodeType::MediaQueryList, id.id);
    }

    fn visit_media_query(&mut self, tree: &Tree, id: LocalNodeId<MediaQuery>, _query: &MediaQuery) {
        self.visit_any(tree, NodeType::MediaQuery, id.id);
    }

    fn visit_media_condition(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<MediaCondition>,
        _condition: &MediaCondition,
    ) {
        self.visit_any(tree, NodeType::MediaCondition, id.id);
    }

    fn visit_feature_name(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<FeatureName>,
        _name: &FeatureName,
    ) {
        self.visit_any(tree, NodeType::FeatureName, id.id);
    }

    fn visit_supports_condition(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<SupportsCondition>,
        _condition: &SupportsCondition,
    ) {
        self.visit_any(tree, NodeType::SupportsCondition, id.id);
    }

    fn visit_ratio_value(&mut self, tree: &Tree, id: LocalNodeId<RatioValue>, _value: &RatioValue) {
        self.visit_any(tree, NodeType::RatioValue, id.id);
    }

    fn visit_environment_variable(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<EnvironmentVariable>,
        _value: &EnvironmentVariable,
    ) {
        self.visit_any(tree, NodeType::EnvironmentVariable, id.id);
    }

    fn visit_container_condition(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<ContainerCondition>,
        _condition: &ContainerCondition,
    ) {
        self.visit_any(tree, NodeType::ContainerCondition, id.id);
    }

    fn visit_container_style_query(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<ContainerStyleQuery>,
        _query: &ContainerStyleQuery,
    ) {
        self.visit_any(tree, NodeType::ContainerStyleQuery, id.id);
    }

    fn visit_container_scroll_state_query(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<ContainerScrollStateQuery>,
        _query: &ContainerScrollStateQuery,
    ) {
        self.visit_any(tree, NodeType::ContainerScrollStateQuery, id.id);
    }
}
