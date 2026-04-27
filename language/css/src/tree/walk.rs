use crate::{
    AnySelector, AttributeSelector, ComponentFragment, ContainerCondition,
    ContainerScrollStateQuery, ContainerStyleQuery, Declaration, DeclarationBlock,
    EnvironmentVariable, FeatureName, FeatureValue, KeyframeRule, LocalNodeId, LocalNodeIdAny,
    MediaCondition, MediaQuery, MediaQueryList, NestedDeclarationsRule, NodeType, NodeVisitor,
    NthOfSelector, NthSelector, PageMarginRule, PageRule, PseudoArgument, PseudoClass,
    PseudoElement, QueryFeature, RatioValue, Rule, Selector, SelectorComponent, SelectorList,
    SimpleSelector, Stylesheet, SupportsCondition, Tree,
};

/// Walk one arbitrary CSS node id.
pub fn walk_any<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    node_type: NodeType,
    node_id: u32,
) {
    let local_idx = tree.local_id_for_node_id(node_id);

    match node_type {
        NodeType::Stylesheet => {
            let stylesheet = tree.stylesheets.get(local_idx);
            walk_stylesheet(visitor, tree, LocalNodeId::new(node_id), stylesheet);
        }
        NodeType::ComponentFragment => {
            let fragment = tree.component_fragments.get(local_idx);
            walk_component_fragment(visitor, tree, LocalNodeId::new(node_id), fragment);
        }
        NodeType::Rule => {
            let rule = tree.rules.get(local_idx);
            walk_rule(visitor, tree, LocalNodeId::new(node_id), rule);
        }
        NodeType::PageMarginRule => {
            let rule = tree.page_margin_rules.get(local_idx);
            walk_page_margin_rule(visitor, tree, LocalNodeId::new(node_id), rule);
        }
        NodeType::DeclarationBlock => {
            let declarations = tree.declaration_blocks.get(local_idx);
            walk_declaration_block(visitor, tree, LocalNodeId::new(node_id), declarations);
        }
        NodeType::Declaration => {
            let declaration = tree.declarations.get(local_idx);
            walk_declaration(visitor, tree, LocalNodeId::new(node_id), declaration);
        }
        NodeType::SelectorList => {
            let selector_list = tree.selector_lists.get(local_idx);
            walk_selector_list(visitor, tree, LocalNodeId::new(node_id), selector_list);
        }
        NodeType::Selector => {
            let selector = tree.selectors.get(local_idx);
            walk_selector(visitor, tree, LocalNodeId::new(node_id), selector);
        }
        NodeType::SelectorComponent => {
            let component = tree.selector_components.get(local_idx);
            walk_selector_component(visitor, tree, LocalNodeId::new(node_id), component);
        }
        NodeType::SimpleSelector => {
            let selector = tree.simple_selectors.get(local_idx);
            walk_simple_selector(visitor, tree, LocalNodeId::new(node_id), selector);
        }
        NodeType::AttributeSelector => {
            let selector = tree.attribute_selectors.get(local_idx);
            walk_attribute_selector(visitor, tree, LocalNodeId::new(node_id), selector);
        }
        NodeType::NthSelector => {
            let selector = tree.nth_selectors.get(local_idx);
            walk_nth_selector(visitor, tree, LocalNodeId::new(node_id), selector);
        }
        NodeType::NthOfSelector => {
            let selector = tree.nth_of_selectors.get(local_idx);
            walk_nth_of_selector(visitor, tree, LocalNodeId::new(node_id), selector);
        }
        NodeType::PseudoClass => {
            let selector = tree.pseudo_classes.get(local_idx);
            walk_pseudo_class(visitor, tree, LocalNodeId::new(node_id), selector);
        }
        NodeType::AnySelector => {
            let selector = tree.any_selectors.get(local_idx);
            walk_any_selector(visitor, tree, LocalNodeId::new(node_id), selector);
        }
        NodeType::PseudoElement => {
            let selector = tree.pseudo_elements.get(local_idx);
            walk_pseudo_element(visitor, tree, LocalNodeId::new(node_id), selector);
        }
        NodeType::MediaQueryList => {
            let media_query_list = tree.media_query_lists.get(local_idx);
            walk_media_query_list(visitor, tree, LocalNodeId::new(node_id), media_query_list);
        }
        NodeType::MediaQuery => {
            let query = tree.media_queries.get(local_idx);
            walk_media_query(visitor, tree, LocalNodeId::new(node_id), query);
        }
        NodeType::MediaCondition => {
            let condition = tree.media_conditions.get(local_idx);
            walk_media_condition(visitor, tree, LocalNodeId::new(node_id), condition);
        }
        NodeType::FeatureName => {
            let name = tree.feature_names.get(local_idx);
            walk_feature_name(visitor, tree, LocalNodeId::new(node_id), name);
        }
        NodeType::QueryFeature => {
            let feature = tree.query_features.get(local_idx);
            walk_query_feature(visitor, tree, LocalNodeId::new(node_id), feature);
        }
        NodeType::FeatureValue => {
            let value = tree.feature_values.get(local_idx);
            walk_feature_value(visitor, tree, LocalNodeId::new(node_id), value);
        }
        NodeType::RatioValue => {
            let value = tree.ratio_values.get(local_idx);
            walk_ratio_value(visitor, tree, LocalNodeId::new(node_id), value);
        }
        NodeType::EnvironmentVariable => {
            let value = tree.environment_variables.get(local_idx);
            walk_environment_variable(visitor, tree, LocalNodeId::new(node_id), value);
        }
        NodeType::SupportsCondition => {
            let condition = tree.supports_conditions.get(local_idx);
            walk_supports_condition(visitor, tree, LocalNodeId::new(node_id), condition);
        }
        NodeType::ContainerCondition => {
            let condition = tree.container_conditions.get(local_idx);
            walk_container_condition(visitor, tree, LocalNodeId::new(node_id), condition);
        }
        NodeType::ContainerStyleQuery => {
            let query = tree.container_style_queries.get(local_idx);
            walk_container_style_query(visitor, tree, LocalNodeId::new(node_id), query);
        }
        NodeType::ContainerScrollStateQuery => {
            let query = tree.container_scroll_state_queries.get(local_idx);
            walk_container_scroll_state_query(visitor, tree, LocalNodeId::new(node_id), query);
        }
    }
}

/// Walk one CSS root node.
pub fn walk_root<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &Tree, root: &LocalNodeIdAny) {
    match root.ty {
        NodeType::Stylesheet => {
            let stylesheet_id = LocalNodeId::<Stylesheet>::new(root.id);
            let stylesheet = tree.get(stylesheet_id);
            visitor.visit_stylesheet(tree, stylesheet_id, stylesheet);
        }
        NodeType::ComponentFragment => {
            let fragment_id = LocalNodeId::<ComponentFragment>::new(root.id);
            let fragment = tree.get(fragment_id);
            visitor.visit_component_fragment(tree, fragment_id, fragment);
        }
        NodeType::Rule => {
            let rule_id = LocalNodeId::<Rule>::new(root.id);
            let rule = tree.get(rule_id);
            visitor.visit_rule(tree, rule_id, rule);
        }
        NodeType::PageMarginRule => {
            let rule_id = LocalNodeId::<PageMarginRule>::new(root.id);
            let rule = tree.get(rule_id);
            visitor.visit_page_margin_rule(tree, rule_id, rule);
        }
        NodeType::DeclarationBlock => {
            let declarations_id = LocalNodeId::<DeclarationBlock>::new(root.id);
            let declarations = tree.get(declarations_id);
            visitor.visit_declaration_block(tree, declarations_id, declarations);
        }
        NodeType::Declaration => {
            let declaration_id = LocalNodeId::<Declaration>::new(root.id);
            let declaration = tree.get(declaration_id);
            visitor.visit_declaration(tree, declaration_id, declaration);
        }
        NodeType::SelectorList => {
            let selector_list_id = LocalNodeId::<SelectorList>::new(root.id);
            let selector_list = tree.get(selector_list_id);
            visitor.visit_selector_list(tree, selector_list_id, selector_list);
        }
        NodeType::Selector => {
            let selector_id = LocalNodeId::<Selector>::new(root.id);
            let selector = tree.get(selector_id);
            visitor.visit_selector(tree, selector_id, selector);
        }
        NodeType::SelectorComponent => {
            let component_id = LocalNodeId::<SelectorComponent>::new(root.id);
            let component = tree.get(component_id);
            visitor.visit_selector_component(tree, component_id, component);
        }
        NodeType::SimpleSelector => {
            let selector_id = LocalNodeId::<SimpleSelector>::new(root.id);
            let selector = tree.get(selector_id);
            visitor.visit_simple_selector(tree, selector_id, selector);
        }
        NodeType::AttributeSelector => {
            let selector_id = LocalNodeId::<AttributeSelector>::new(root.id);
            let selector = tree.get(selector_id);
            visitor.visit_attribute_selector(tree, selector_id, selector);
        }
        NodeType::NthSelector => {
            let selector_id = LocalNodeId::<NthSelector>::new(root.id);
            let selector = tree.get(selector_id);
            visitor.visit_nth_selector(tree, selector_id, selector);
        }
        NodeType::NthOfSelector => {
            let selector_id = LocalNodeId::<NthOfSelector>::new(root.id);
            let selector = tree.get(selector_id);
            visitor.visit_nth_of_selector(tree, selector_id, selector);
        }
        NodeType::PseudoClass => {
            let selector_id = LocalNodeId::<PseudoClass>::new(root.id);
            let selector = tree.get(selector_id);
            visitor.visit_pseudo_class(tree, selector_id, selector);
        }
        NodeType::AnySelector => {
            let selector_id = LocalNodeId::<AnySelector>::new(root.id);
            let selector = tree.get(selector_id);
            visitor.visit_any_selector(tree, selector_id, selector);
        }
        NodeType::PseudoElement => {
            let selector_id = LocalNodeId::<PseudoElement>::new(root.id);
            let selector = tree.get(selector_id);
            visitor.visit_pseudo_element(tree, selector_id, selector);
        }
        NodeType::MediaQueryList => {
            let media_query_list_id = LocalNodeId::<MediaQueryList>::new(root.id);
            let media_query_list = tree.get(media_query_list_id);
            visitor.visit_media_query_list(tree, media_query_list_id, media_query_list);
        }
        NodeType::MediaQuery => {
            let query_id = LocalNodeId::<MediaQuery>::new(root.id);
            let query = tree.get(query_id);
            visitor.visit_media_query(tree, query_id, query);
        }
        NodeType::MediaCondition => {
            let condition_id = LocalNodeId::<MediaCondition>::new(root.id);
            let condition = tree.get(condition_id);
            visitor.visit_media_condition(tree, condition_id, condition);
        }
        NodeType::FeatureName => {
            let name_id = LocalNodeId::<FeatureName>::new(root.id);
            let name = tree.get(name_id);
            visitor.visit_feature_name(tree, name_id, name);
        }
        NodeType::QueryFeature => {
            let feature_id = LocalNodeId::<QueryFeature>::new(root.id);
            let feature = tree.get(feature_id);
            visitor.visit_query_feature(tree, feature_id, feature);
        }
        NodeType::FeatureValue => {
            let value_id = LocalNodeId::<FeatureValue>::new(root.id);
            let value = tree.get(value_id);
            visitor.visit_feature_value(tree, value_id, value);
        }
        NodeType::RatioValue => {
            let value_id = LocalNodeId::<RatioValue>::new(root.id);
            let value = tree.get(value_id);
            visitor.visit_ratio_value(tree, value_id, value);
        }
        NodeType::EnvironmentVariable => {
            let value_id = LocalNodeId::<EnvironmentVariable>::new(root.id);
            let value = tree.get(value_id);
            visitor.visit_environment_variable(tree, value_id, value);
        }
        NodeType::SupportsCondition => {
            let condition_id = LocalNodeId::<SupportsCondition>::new(root.id);
            let condition = tree.get(condition_id);
            visitor.visit_supports_condition(tree, condition_id, condition);
        }
        NodeType::ContainerCondition => {
            let condition_id = LocalNodeId::<ContainerCondition>::new(root.id);
            let condition = tree.get(condition_id);
            visitor.visit_container_condition(tree, condition_id, condition);
        }
        NodeType::ContainerStyleQuery => {
            let query_id = LocalNodeId::<ContainerStyleQuery>::new(root.id);
            let query = tree.get(query_id);
            visitor.visit_container_style_query(tree, query_id, query);
        }
        NodeType::ContainerScrollStateQuery => {
            let query_id = LocalNodeId::<ContainerScrollStateQuery>::new(root.id);
            let query = tree.get(query_id);
            visitor.visit_container_scroll_state_query(tree, query_id, query);
        }
    }
}

/// Walk one component fragment root.
pub fn walk_component_fragment<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<ComponentFragment>,
    _fragment: &ComponentFragment,
) {
    visitor.visit_any(tree, NodeType::ComponentFragment, id.id);
}

/// Walk one CSS root list through the visitor entry points.
pub fn walk_roots<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &Tree, roots: &[LocalNodeIdAny]) {
    for root in roots {
        walk_root(visitor, tree, root);
    }
}

/// Walk one stylesheet node.
pub fn walk_stylesheet<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Stylesheet>,
    stylesheet: &Stylesheet,
) {
    visitor.visit_any(tree, NodeType::Stylesheet, id.id);

    for rule in &stylesheet.rules {
        let rule_node = tree.get(*rule);

        visitor.visit_rule(tree, *rule, rule_node);
    }
}

/// Walk one CSS rule node.
pub fn walk_rule<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Rule>,
    rule: &Rule,
) {
    visitor.visit_any(tree, NodeType::Rule, id.id);

    match rule {
        Rule::Import(rule) => {
            if let Some(supports) = rule.supports {
                walk_supports_condition_child(visitor, tree, supports);
            }

            if let Some(media) = rule.media {
                walk_media_query_list_child(visitor, tree, media);
            }
        }
        Rule::LayerStatement(_)
        | Rule::FontFeatureValues(_)
        | Rule::Namespace(_)
        | Rule::Property(_)
        | Rule::Unknown(_)
        | Rule::Custom(_)
        | Rule::Ignored(_) => {}
        Rule::CustomMedia(rule) => {
            walk_media_query_list_child(visitor, tree, rule.query);
        }
        Rule::Style(rule) => {
            walk_selector_list_child(visitor, tree, rule.prelude);
            walk_style_rule_children(visitor, tree, rule.declarations, &rule.rules)
        }
        Rule::Nesting(rule) => {
            walk_selector_list_child(visitor, tree, rule.prelude);
            walk_style_rule_children(visitor, tree, rule.declarations, &rule.rules);
        }
        Rule::Media(rule) => {
            walk_media_query_list_child(visitor, tree, rule.query);
            walk_rule_list_children(visitor, tree, &rule.rules);
        }
        Rule::Supports(rule) => {
            walk_supports_condition_child(visitor, tree, rule.condition);
            walk_rule_list_children(visitor, tree, &rule.rules);
        }
        Rule::LayerBlock(rule) => walk_rule_list_children(visitor, tree, &rule.rules),
        Rule::Container(rule) => {
            if let Some(condition) = rule.condition {
                walk_container_condition_child(visitor, tree, condition);
            }

            walk_rule_list_children(visitor, tree, &rule.rules);
        }
        Rule::Scope(rule) => {
            if let Some(scope_start) = rule.scope_start {
                walk_selector_list_child(visitor, tree, scope_start);
            }

            if let Some(scope_end) = rule.scope_end {
                walk_selector_list_child(visitor, tree, scope_end);
            }

            walk_rule_list_children(visitor, tree, &rule.rules);
        }
        Rule::StartingStyle(rule) => walk_rule_list_children(visitor, tree, &rule.rules),
        Rule::Keyframes(rule) => walk_rule_list_children(visitor, tree, &rule.rules),
        Rule::MozDocument(rule) => walk_rule_list_children(visitor, tree, &rule.rules),
        Rule::Page(rule) => {
            walk_page_rule_children(visitor, tree, rule);
        }
        Rule::FontFace(rule) => walk_declaration_children(visitor, tree, rule.declarations),
        Rule::FontPaletteValues(rule) => {
            walk_declaration_children(visitor, tree, rule.declarations);
        }
        Rule::CounterStyle(rule) => walk_declaration_children(visitor, tree, rule.declarations),
        Rule::Viewport(rule) => walk_declaration_children(visitor, tree, rule.declarations),
        Rule::ViewTransition(rule) => walk_declaration_children(visitor, tree, rule.declarations),
        Rule::NestedDeclarations(rule) => {
            walk_nested_declarations_rule_children(visitor, tree, rule);
        }
        Rule::Keyframe(rule) => walk_keyframe_rule_children(visitor, tree, rule),
    }
}

/// Walk one nested rule list.
fn walk_rule_list_children<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    rules: &[LocalNodeId<Rule>],
) {
    for child in rules {
        let child_rule = tree.get(*child);

        visitor.visit_rule(tree, *child, child_rule);
    }
}

/// Walk one selector list child node.
fn walk_selector_list_child<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    selector_list_id: LocalNodeId<SelectorList>,
) {
    let selector_list = tree.get(selector_list_id);
    visitor.visit_selector_list(tree, selector_list_id, selector_list);
}

/// Walk one media query list child node.
fn walk_media_query_list_child<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    media_query_list_id: LocalNodeId<MediaQueryList>,
) {
    let media_query_list = tree.get(media_query_list_id);
    visitor.visit_media_query_list(tree, media_query_list_id, media_query_list);
}

/// Walk one supports condition child node.
fn walk_supports_condition_child<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    condition_id: LocalNodeId<SupportsCondition>,
) {
    let condition = tree.get(condition_id);
    visitor.visit_supports_condition(tree, condition_id, condition);
}

/// Walk one container condition child node.
fn walk_container_condition_child<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    condition_id: LocalNodeId<ContainerCondition>,
) {
    let condition = tree.get(condition_id);
    visitor.visit_container_condition(tree, condition_id, condition);
}

/// Walk one style rule node.
fn walk_style_rule_children<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    declarations: Option<LocalNodeId<DeclarationBlock>>,
    rules: &[LocalNodeId<Rule>],
) {
    if let Some(declarations) = declarations {
        let declaration_block_id = declarations;
        let declarations = tree.get(declaration_block_id);

        visitor.visit_declaration_block(tree, declaration_block_id, declarations);
    }

    walk_rule_list_children(visitor, tree, rules);
}

/// Walk one declaration rule node.
fn walk_declaration_children<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    declarations: Option<LocalNodeId<DeclarationBlock>>,
) {
    if let Some(declarations) = declarations {
        let declaration_block_id = declarations;
        let declarations = tree.get(declaration_block_id);

        visitor.visit_declaration_block(tree, declaration_block_id, declarations);
    }
}

/// Walk one keyframe rule node.
fn walk_keyframe_rule_children<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    rule: &KeyframeRule,
) {
    walk_declaration_children(visitor, tree, rule.declarations);
}

/// Walk one page rule node.
fn walk_page_rule_children<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &Tree, rule: &PageRule) {
    walk_declaration_children(visitor, tree, rule.declarations);

    for page_margin_rule in &rule.page_margin_rules {
        let page_margin_rule_id = *page_margin_rule;
        let page_margin_rule = tree.get(page_margin_rule_id);

        visitor.visit_page_margin_rule(tree, page_margin_rule_id, page_margin_rule);
    }
}

/// Walk one nested declarations rule node.
fn walk_nested_declarations_rule_children<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    rule: &NestedDeclarationsRule,
) {
    walk_declaration_children(visitor, tree, rule.declarations);
}

/// Walk one CSS page margin rule node.
pub fn walk_page_margin_rule<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<PageMarginRule>,
    rule: &PageMarginRule,
) {
    visitor.visit_any(tree, NodeType::PageMarginRule, id.id);

    walk_declaration_children(visitor, tree, rule.declarations);
}

/// Walk one declaration block node.
pub fn walk_declaration_block<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<DeclarationBlock>,
    declarations: &DeclarationBlock,
) {
    visitor.visit_any(tree, NodeType::DeclarationBlock, id.id);

    for declaration in &declarations.declarations {
        let declaration_id = *declaration;
        let declaration = tree.get(declaration_id);

        visitor.visit_declaration(tree, declaration_id, declaration);
    }
}

/// Walk one declaration node.
pub fn walk_declaration<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Declaration>,
    _declaration: &Declaration,
) {
    visitor.visit_any(tree, NodeType::Declaration, id.id);
}

/// Walk one selector list node.
pub fn walk_selector_list<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<SelectorList>,
    selector_list: &SelectorList,
) {
    visitor.visit_any(tree, NodeType::SelectorList, id.id);

    for selector_id in &selector_list.selectors {
        let selector = tree.get(*selector_id);
        visitor.visit_selector(tree, *selector_id, selector);
    }
}

/// Walk one selector node.
pub fn walk_selector<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Selector>,
    selector: &Selector,
) {
    visitor.visit_any(tree, NodeType::Selector, id.id);

    for component_id in &selector.components {
        let component = tree.get(*component_id);
        visitor.visit_selector_component(tree, *component_id, component);
    }
}

/// Walk one selector component node.
pub fn walk_selector_component<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<SelectorComponent>,
    component: &SelectorComponent,
) {
    visitor.visit_any(tree, NodeType::SelectorComponent, id.id);

    match component {
        SelectorComponent::Combinator(_) => {}
        SelectorComponent::Simple(simple_id) => {
            let simple = tree.get(*simple_id);
            visitor.visit_simple_selector(tree, *simple_id, simple);
        }
    }
}

/// Walk one simple selector node.
pub fn walk_simple_selector<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<SimpleSelector>,
    simple: &SimpleSelector,
) {
    visitor.visit_any(tree, NodeType::SimpleSelector, id.id);

    match simple {
        SimpleSelector::Negation(selectors)
        | SimpleSelector::Where(selectors)
        | SimpleSelector::Is(selectors)
        | SimpleSelector::Has(selectors) => walk_selector_list_child(visitor, tree, *selectors),
        SimpleSelector::Attribute(selector_id) => {
            let selector = tree.get(*selector_id);
            visitor.visit_attribute_selector(tree, *selector_id, selector);
        }
        SimpleSelector::Slotted(selector) => {
            let selector_node = tree.get(*selector);
            visitor.visit_selector(tree, *selector, selector_node);
        }
        SimpleSelector::Host(Some(selector)) => {
            let selector_node = tree.get(*selector);
            visitor.visit_selector(tree, *selector, selector_node);
        }
        SimpleSelector::Host(None) => {}
        SimpleSelector::NthOf(selector_id) => {
            let selector = tree.get(*selector_id);
            visitor.visit_nth_of_selector(tree, *selector_id, selector);
        }
        SimpleSelector::Any(selector_id) => {
            let selector = tree.get(*selector_id);
            visitor.visit_any_selector(tree, *selector_id, selector);
        }
        SimpleSelector::Nth(selector_id) => {
            let selector = tree.get(*selector_id);
            visitor.visit_nth_selector(tree, *selector_id, selector);
        }
        SimpleSelector::PseudoClass(selector_id) => {
            let selector = tree.get(*selector_id);
            visitor.visit_pseudo_class(tree, *selector_id, selector);
        }
        SimpleSelector::PseudoElement(selector_id) => {
            let selector = tree.get(*selector_id);
            visitor.visit_pseudo_element(tree, *selector_id, selector);
        }
        _ => {}
    }
}

/// Walk one attribute selector node.
pub fn walk_attribute_selector<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<AttributeSelector>,
    _selector: &AttributeSelector,
) {
    visitor.visit_any(tree, NodeType::AttributeSelector, id.id);
}

/// Walk one nth selector node.
pub fn walk_nth_selector<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<NthSelector>,
    _selector: &NthSelector,
) {
    visitor.visit_any(tree, NodeType::NthSelector, id.id);
}

/// Walk one nth-of selector node.
pub fn walk_nth_of_selector<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<NthOfSelector>,
    selector: &NthOfSelector,
) {
    visitor.visit_any(tree, NodeType::NthOfSelector, id.id);

    let nth = tree.get(selector.nth);
    visitor.visit_nth_selector(tree, selector.nth, nth);

    walk_selector_list_child(visitor, tree, selector.selectors);
}

/// Walk one pseudo class node.
pub fn walk_pseudo_class<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<PseudoClass>,
    selector: &PseudoClass,
) {
    visitor.visit_any(tree, NodeType::PseudoClass, id.id);

    if let Some(arguments) = &selector.arguments {
        if let PseudoArgument::Selector(selector_id) = arguments {
            let selector_node = tree.get(*selector_id);
            visitor.visit_selector(tree, *selector_id, selector_node);
        }
    }
}

/// Walk one vendor any selector node.
pub fn walk_any_selector<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<AnySelector>,
    selector: &AnySelector,
) {
    visitor.visit_any(tree, NodeType::AnySelector, id.id);

    walk_selector_list_child(visitor, tree, selector.selectors);
}

/// Walk one pseudo element node.
pub fn walk_pseudo_element<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<PseudoElement>,
    selector: &PseudoElement,
) {
    visitor.visit_any(tree, NodeType::PseudoElement, id.id);

    if let Some(arguments) = &selector.arguments {
        if let PseudoArgument::Selector(selector_id) = arguments {
            let selector_node = tree.get(*selector_id);
            visitor.visit_selector(tree, *selector_id, selector_node);
        }
    }
}

/// Walk one media query list node.
pub fn walk_media_query_list<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<MediaQueryList>,
    media_query_list: &MediaQueryList,
) {
    visitor.visit_any(tree, NodeType::MediaQueryList, id.id);

    for query_id in &media_query_list.queries {
        let query = tree.get(*query_id);
        visitor.visit_media_query(tree, *query_id, query);
    }
}

/// Walk one media query node.
pub fn walk_media_query<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<MediaQuery>,
    query: &MediaQuery,
) {
    visitor.visit_any(tree, NodeType::MediaQuery, id.id);

    if let Some(condition_id) = query.condition {
        let condition = tree.get(condition_id);
        visitor.visit_media_condition(tree, condition_id, condition);
    }
}

/// Walk one media condition node.
pub fn walk_media_condition<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<MediaCondition>,
    condition: &MediaCondition,
) {
    visitor.visit_any(tree, NodeType::MediaCondition, id.id);

    match condition {
        MediaCondition::Feature(feature_id) => {
            let feature = tree.get(*feature_id);
            visitor.visit_query_feature(tree, *feature_id, feature);
        }
        MediaCondition::Unknown(_) => {}
        MediaCondition::Not(condition_id) => {
            let condition_node = tree.get(*condition_id);
            visitor.visit_media_condition(tree, *condition_id, condition_node);
        }
        MediaCondition::Operation { conditions, .. } => {
            for condition_id in conditions {
                let condition_node = tree.get(*condition_id);
                visitor.visit_media_condition(tree, *condition_id, condition_node);
            }
        }
    }
}

/// Walk one feature name node.
pub fn walk_feature_name<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<FeatureName>,
    _name: &FeatureName,
) {
    visitor.visit_any(tree, NodeType::FeatureName, id.id);
}

/// Walk one query feature node.
pub fn walk_query_feature<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<QueryFeature>,
    feature: &QueryFeature,
) {
    visitor.visit_any(tree, NodeType::QueryFeature, id.id);

    match feature {
        QueryFeature::Plain { name, value } | QueryFeature::Range { name, value, .. } => {
            let name_node = tree.get(*name);
            visitor.visit_feature_name(tree, *name, name_node);

            let value_node = tree.get(*value);
            visitor.visit_feature_value(tree, *value, value_node);
        }
        QueryFeature::Boolean { name } => {
            let name_node = tree.get(*name);
            visitor.visit_feature_name(tree, *name, name_node);
        }
        QueryFeature::Interval {
            name, start, end, ..
        } => {
            let name_node = tree.get(*name);
            visitor.visit_feature_name(tree, *name, name_node);

            let start_node = tree.get(*start);
            visitor.visit_feature_value(tree, *start, start_node);

            let end_node = tree.get(*end);
            visitor.visit_feature_value(tree, *end, end_node);
        }
    }
}

/// Walk one feature value node.
pub fn walk_feature_value<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<FeatureValue>,
    _value: &FeatureValue,
) {
    visitor.visit_any(tree, NodeType::FeatureValue, id.id);

    match _value {
        FeatureValue::Ratio(value_id) => {
            let value = tree.get(*value_id);
            visitor.visit_ratio_value(tree, *value_id, value);
        }
        FeatureValue::EnvironmentVariable(value_id) => {
            let value = tree.get(*value_id);
            visitor.visit_environment_variable(tree, *value_id, value);
        }
        _ => {}
    }
}

/// Walk one ratio value node.
pub fn walk_ratio_value<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<RatioValue>,
    _value: &RatioValue,
) {
    visitor.visit_any(tree, NodeType::RatioValue, id.id);
}

/// Walk one environment variable node.
pub fn walk_environment_variable<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<EnvironmentVariable>,
    _value: &EnvironmentVariable,
) {
    visitor.visit_any(tree, NodeType::EnvironmentVariable, id.id);
}

/// Walk one supports condition node.
pub fn walk_supports_condition<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<SupportsCondition>,
    condition: &SupportsCondition,
) {
    visitor.visit_any(tree, NodeType::SupportsCondition, id.id);

    match condition {
        SupportsCondition::Not(condition_id) => {
            let condition_node = tree.get(*condition_id);
            visitor.visit_supports_condition(tree, *condition_id, condition_node);
        }
        SupportsCondition::And(conditions) | SupportsCondition::Or(conditions) => {
            for condition_id in conditions {
                let condition_node = tree.get(*condition_id);
                visitor.visit_supports_condition(tree, *condition_id, condition_node);
            }
        }
        SupportsCondition::Selector(selector) => {
            walk_selector_list_child(visitor, tree, selector.selectors);
        }
        SupportsCondition::Declaration { .. } | SupportsCondition::Unknown(_) => {}
    }
}

/// Walk one container condition node.
pub fn walk_container_condition<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<ContainerCondition>,
    condition: &ContainerCondition,
) {
    visitor.visit_any(tree, NodeType::ContainerCondition, id.id);

    match condition {
        ContainerCondition::Feature(feature_id) => {
            let feature = tree.get(*feature_id);
            visitor.visit_query_feature(tree, *feature_id, feature);
        }
        ContainerCondition::Unknown(_) => {}
        ContainerCondition::Not(condition_id) => {
            let condition_node = tree.get(*condition_id);
            visitor.visit_container_condition(tree, *condition_id, condition_node);
        }
        ContainerCondition::Operation { conditions, .. } => {
            for condition_id in conditions {
                let condition_node = tree.get(*condition_id);
                visitor.visit_container_condition(tree, *condition_id, condition_node);
            }
        }
        ContainerCondition::Style(query_id) => {
            let query = tree.get(*query_id);
            visitor.visit_container_style_query(tree, *query_id, query);
        }
        ContainerCondition::ScrollState(query_id) => {
            let query = tree.get(*query_id);
            visitor.visit_container_scroll_state_query(tree, *query_id, query);
        }
    }
}

/// Walk one container style query node.
pub fn walk_container_style_query<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<ContainerStyleQuery>,
    query: &ContainerStyleQuery,
) {
    visitor.visit_any(tree, NodeType::ContainerStyleQuery, id.id);

    match query {
        ContainerStyleQuery::Declaration { .. } | ContainerStyleQuery::Property(_) => {}
        ContainerStyleQuery::Not(query_id) => {
            let query_node = tree.get(*query_id);
            visitor.visit_container_style_query(tree, *query_id, query_node);
        }
        ContainerStyleQuery::Operation { conditions, .. } => {
            for query_id in conditions {
                let query_node = tree.get(*query_id);
                visitor.visit_container_style_query(tree, *query_id, query_node);
            }
        }
    }
}

/// Walk one container scroll state query node.
pub fn walk_container_scroll_state_query<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<ContainerScrollStateQuery>,
    query: &ContainerScrollStateQuery,
) {
    visitor.visit_any(tree, NodeType::ContainerScrollStateQuery, id.id);

    match query {
        ContainerScrollStateQuery::Feature(feature_id) => {
            let feature = tree.get(*feature_id);
            visitor.visit_query_feature(tree, *feature_id, feature);
        }
        ContainerScrollStateQuery::Not(query_id) => {
            let query_node = tree.get(*query_id);
            visitor.visit_container_scroll_state_query(tree, *query_id, query_node);
        }
        ContainerScrollStateQuery::Operation { conditions, .. } => {
            for query_id in conditions {
                let query_node = tree.get(*query_id);
                visitor.visit_container_scroll_state_query(tree, *query_id, query_node);
            }
        }
    }
}
