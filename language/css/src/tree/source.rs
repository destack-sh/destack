use crate::{
    Declaration, DeclarationBlock, DeclarationRule, GroupRule, IgnoredRule, ImportRule,
    KeyframeRule, LeafRule, LocalNodeId, NestedDeclarationsRule, Tree, PageMarginRule,
    PageRule, Rule, StyleRule, Stylesheet,
};
use lightningcss::rules::CssRuleList;
use lightningcss::stylesheet::{ParserOptions, Stylesheet as LightningStylesheet};
use lightningcss::traits::IntoOwned;

/// One owned Lightning CSS stylesheet.
pub type OwnedStylesheet = LightningStylesheet<'static, 'static>;

/// One stylesheet reconstruction failure.
#[derive(Debug, Clone)]
pub struct StylesheetError {
    /// The reconstruction error message.
    pub message: String,
}

/// Return one stylesheet subtree as canonical CSS source.
pub fn stylesheet_to_source(tree: &Tree, stylesheet: LocalNodeId<Stylesheet>) -> String {
    let stylesheet = tree.get(stylesheet);
    let mut source = String::new();

    // top level rules
    for rule in &stylesheet.rules {
        let rule = *rule;

        if !source.is_empty() {
            source.push('\n');
        }

        render_rule(tree, rule, &mut source);
    }

    source
}

/// Rebuild one owned Lightning CSS stylesheet from one stylesheet subtree.
pub fn stylesheet_to_lightning_stylesheet(
    tree: &Tree,
    stylesheet: LocalNodeId<Stylesheet>,
) -> Result<OwnedStylesheet, StylesheetError> {
    let stylesheet_node = tree.get(stylesheet);
    let source = stylesheet_to_source(tree, stylesheet);
    let filename = stylesheet_node.sources.first().cloned().unwrap_or_default();
    let stylesheet = LightningStylesheet::parse(
        &source,
        ParserOptions {
            filename: filename.clone(),
            ..ParserOptions::default()
        },
    )
    .map_err(|error| StylesheetError {
        message: error.to_string(),
    })?;
    let sources = stylesheet.sources.into_owned();
    let rules = stylesheet.rules.into_owned();
    let mut stylesheet = LightningStylesheet::new(
        sources,
        CssRuleList(rules.0),
        ParserOptions {
            filename,
            ..ParserOptions::default()
        },
    );

    // preserved metadata
    stylesheet.license_comments = stylesheet_node
        .license_comments
        .iter()
        .cloned()
        .map(Into::into)
        .collect();
    stylesheet.sources = stylesheet_node.sources.clone();

    Ok(stylesheet)
}

/// Render one CSS rule.
fn render_rule(tree: &Tree, id: LocalNodeId<Rule>, source: &mut String) {
    match tree.get(id) {
        Rule::Import(rule) => render_import_rule(rule, source),
        Rule::Style(rule) => render_style_rule(tree, rule, source),
        Rule::Media(rule)
        | Rule::Supports(rule)
        | Rule::LayerBlock(rule)
        | Rule::Container(rule)
        | Rule::Scope(rule)
        | Rule::StartingStyle(rule)
        | Rule::Keyframes(rule)
        | Rule::MozDocument(rule) => render_group_rule(tree, rule, source),
        Rule::LayerStatement(rule)
        | Rule::FontFeatureValues(rule)
        | Rule::Namespace(rule)
        | Rule::CustomMedia(rule)
        | Rule::Property(rule)
        | Rule::Unknown(rule)
        | Rule::Custom(rule) => render_leaf_rule(rule, source),
        Rule::Page(rule) => render_page_rule(tree, rule, source),
        Rule::FontFace(rule)
        | Rule::FontPaletteValues(rule)
        | Rule::CounterStyle(rule)
        | Rule::Viewport(rule)
        | Rule::ViewTransition(rule) => render_declaration_rule(tree, rule, source),
        Rule::Nesting(rule) => render_style_rule(tree, rule, source),
        Rule::NestedDeclarations(rule) => render_nested_declarations_rule(tree, rule, source),
        Rule::Keyframe(rule) => render_keyframe_rule(tree, rule, source),
        Rule::Ignored(rule) => render_ignored_rule(rule, source),
    }
}

/// Render one import rule.
fn render_import_rule(rule: &ImportRule, source: &mut String) {
    source.push_str(&rule.source);
}

/// Render one leaf rule.
fn render_leaf_rule(rule: &LeafRule, source: &mut String) {
    source.push_str(&rule.source);
}

/// Render one ignored rule.
fn render_ignored_rule(_rule: &IgnoredRule, _source: &mut String) {}

/// Render one grouped rule.
fn render_group_rule(tree: &Tree, rule: &GroupRule, source: &mut String) {
    source.push_str(&rule.header);
    source.push('{');
    render_rule_list(tree, &rule.rules, source);
    source.push('}');
}

/// Render one declaration rule.
fn render_declaration_rule(tree: &Tree, rule: &DeclarationRule, source: &mut String) {
    if let Some(raw_source) = &rule.source {
        source.push_str(raw_source);

        return;
    }

    source.push_str(&rule.header);
    source.push('{');
    render_declaration_block_contents(tree, rule.declarations, source);
    source.push('}');
}

/// Render one style rule.
fn render_style_rule(tree: &Tree, rule: &StyleRule, source: &mut String) {
    source.push_str(&rule.header);
    source.push('{');
    render_declaration_and_rule_contents(tree, rule.declarations, &rule.rules, source);
    source.push('}');
}

/// Render one keyframe rule.
fn render_keyframe_rule(tree: &Tree, rule: &KeyframeRule, source: &mut String) {
    source.push_str(&rule.header);
    source.push('{');
    render_declaration_block_contents(tree, rule.declarations, source);
    source.push('}');
}

/// Render one nested declarations rule.
fn render_nested_declarations_rule(
    tree: &Tree,
    rule: &NestedDeclarationsRule,
    source: &mut String,
) {
    source.push('{');
    render_declaration_block_contents(tree, rule.declarations, source);
    source.push('}');
}

/// Render one page rule.
fn render_page_rule(tree: &Tree, rule: &PageRule, source: &mut String) {
    source.push_str(&rule.header);
    source.push('{');

    // page declarations
    render_declaration_block_contents(tree, rule.declarations, source);

    // page margin rules
    for page_margin_rule in &rule.page_margin_rules {
        render_page_margin_rule(tree, *page_margin_rule, source);
    }

    source.push('}');
}

/// Render one page margin rule.
fn render_page_margin_rule(tree: &Tree, id: LocalNodeId<PageMarginRule>, source: &mut String) {
    let rule = tree.get(id);

    source.push_str(&rule.header);
    source.push('{');
    render_declaration_block_contents(tree, rule.declarations, source);
    source.push('}');
}

/// Render one rule list.
fn render_rule_list(tree: &Tree, rules: &[LocalNodeId<Rule>], source: &mut String) {
    for rule in rules {
        render_rule(tree, *rule, source);
    }
}

/// Render one declaration block and one nested rule list.
fn render_declaration_and_rule_contents(
    tree: &Tree,
    declarations: Option<LocalNodeId<DeclarationBlock>>,
    rules: &[LocalNodeId<Rule>],
    source: &mut String,
) {
    render_declaration_block_contents(tree, declarations, source);
    render_rule_list(tree, rules, source);
}

/// Render one declaration block.
fn render_declaration_block_contents(
    tree: &Tree,
    declarations: Option<LocalNodeId<DeclarationBlock>>,
    source: &mut String,
) {
    let Some(declarations) = declarations else {
        return;
    };

    let declarations = tree.get(declarations);

    // declarations
    for declaration in &declarations.declarations {
        render_declaration(tree, *declaration, source);
    }
}

/// Render one declaration.
fn render_declaration(tree: &Tree, id: LocalNodeId<Declaration>, source: &mut String) {
    let declaration = tree.get(id);

    source.push_str(&declaration.source);

    if !declaration.source.ends_with(';') {
        source.push(';');
    }
}
