use destack_core::StringPool;
use destack_fir::format::{FormatContext, FormatResult, format};
use destack_fir::prelude::*;
use destack_fir::write;
use destack_source::{File, FileType};

use super::CssFormatOptions;
use crate::print::TokenRenderer;
use crate::{
    AnySelector, BlockKind, Combinator, ComponentValue, ComponentValueList, ConditionOperator,
    ContainerCondition, ContainerRule, ContainerScrollStateQuery, ContainerStyleQuery,
    CustomMediaRule, DeclarationBlock, EnvironmentVariable, EnvironmentVariableName,
    FeatureComparison, FeatureName, FeatureValue, FontFeatureSubruleKind, FontFeatureValuesRule,
    Function, ImportLayer, ImportRule, KeyframeRule, KeyframeSelector, KeyframeSelectorList,
    KeyframesRule, LayerBlockRule, LayerNameList, LayerStatementRule, LocalNodeId, MediaCondition,
    MediaQualifier, MediaQuery, MediaQueryList, MediaType, NamespaceRule, NamespaceUrl,
    NthOfSelector, NthSelector, NthSelectorKind, Number, PageMarginBox, PageMarginRule,
    PagePseudoClass, PageRule, PageSelector, PageSelectorList, PropertyName, PropertyRule,
    PropertySyntax, PropertySyntaxComponent, PropertySyntaxComponentKind, PropertySyntaxMultiplier,
    PseudoArgument, PseudoClass, PseudoElement, QueryFeature, RatioValue, Rule, ScopeRule,
    Selector, SelectorComponent, SelectorList, SimpleBlock, SimpleSelector, Stylesheet,
    SupportsCondition, TimelineRangeName, Token, Tree, UnknownRule, VendorPrefix,
};

/// Format one stylesheet as pretty CSS.
pub fn format_stylesheet(
    tree: &Tree,
    stylesheet: LocalNodeId<Stylesheet>,
    options: CssFormatOptions,
) -> FormatResult<String> {
    let context = CssFormatContext::new(options, tree.strings.clone());
    let formatted = format(
        context,
        destack_fir::format_args![format_with(|f| write_stylesheet(tree, stylesheet, f))],
    )?;

    Ok(formatted.print()?.into_str())
}

/// One CSS FIR formatting context.
#[derive(Debug, Clone)]
struct CssFormatContext {
    /// The format options.
    options: CssFormatOptions,
    /// The virtual CSS file used by FIR printing.
    file: File,
    /// The pooled css strings for this formatting pass.
    strings: StringPool,
}

impl CssFormatContext {
    /// Create one CSS formatting context.
    fn new(options: CssFormatOptions, strings: StringPool) -> Self {
        Self {
            options,
            file: File::empty_text(FileType::Css),
            strings,
        }
    }

    /// Render one property name.
    fn render_property_name<'s>(&self, property_name: &'s PropertyName) -> &'s str {
        match property_name {
            PropertyName::Standard(name) | PropertyName::Custom(name) => name,
        }
    }

    /// Render one component token as canonical CSS.
    fn render_component_token(&self, token: &Token) -> String {
        TokenRenderer::new(&self.strings).render_token(token)
    }
}

impl FormatContext for CssFormatContext {
    type Options = CssFormatOptions;

    fn options(&self) -> &Self::Options {
        &self.options
    }

    fn file(&self) -> &File {
        &self.file
    }
}

/// Write one stylesheet.
fn write_stylesheet(
    tree: &Tree,
    stylesheet_id: LocalNodeId<Stylesheet>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    let stylesheet = tree.get(stylesheet_id);

    // top level rules
    for (index, rule_id) in stylesheet.rules.iter().enumerate() {
        if index > 0 {
            write!(f, [hard_line_break(), hard_line_break()])?;
        }

        write_rule(tree, *rule_id, f)?;
    }

    // trailing newline
    if !stylesheet.rules.is_empty() {
        write!(f, [hard_line_break()])?;
    }

    Ok(())
}

/// Write one CSS rule.
fn write_rule(
    tree: &Tree,
    rule_id: LocalNodeId<Rule>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    let rule = tree.get(rule_id);

    match rule {
        Rule::Import(rule) => write_import_rule(tree, rule, f),
        // declaration style rules
        Rule::Style(rule) => {
            write_selector_block_rule(tree, rule.prelude, rule.declarations, &rule.rules, f)
        }
        Rule::Nesting(rule) => {
            write_selector_block_rule(tree, rule.prelude, rule.declarations, &rule.rules, f)
        }

        // grouped rule blocks
        Rule::Media(rule) => write_media_rule(tree, rule.query, &rule.rules, f),
        Rule::Supports(rule) => write_supports_rule(tree, rule.condition, &rule.rules, f),
        Rule::LayerBlock(rule) => write_layer_block_rule(tree, rule, &rule.rules, f),
        Rule::Container(rule) => write_container_rule(tree, rule, &rule.rules, f),
        Rule::Scope(rule) => write_scope_rule(tree, rule, &rule.rules, f),
        Rule::StartingStyle(rule) => {
            write_group_block_rule(tree, "@starting-style", &rule.rules, f)
        }
        Rule::Keyframes(rule) => write_keyframes_group_rule(tree, rule, &rule.rules, f),
        Rule::MozDocument(rule) => {
            write_group_block_rule(tree, "@-moz-document url-prefix()", &rule.rules, f)
        }
        Rule::LayerStatement(rule) => write_layer_statement_rule(rule, f),
        Rule::FontFeatureValues(rule) => write_font_feature_values_rule(rule, f),
        Rule::Namespace(rule) => write_namespace_rule(rule, f),
        Rule::CustomMedia(rule) => write_custom_media_rule(tree, rule, f),
        Rule::Property(rule) => write_property_rule(rule, f),
        Rule::Unknown(rule) => write_unknown_rule(rule, f),
        Rule::Custom(rule) => write_component_value_list(&rule.components, f),
        Rule::Ignored(_) => Ok(()),

        // declaration only rules
        Rule::FontFace(rule) => {
            write_named_declaration_rule(tree, "@font-face", "", rule.declarations, f)
        }
        Rule::FontPaletteValues(rule) => write_named_declaration_rule(
            tree,
            "@font-palette-values",
            &rule.name.name,
            rule.declarations,
            f,
        ),
        Rule::CounterStyle(rule) => write_named_declaration_rule(
            tree,
            "@counter-style",
            &rule.name.name,
            rule.declarations,
            f,
        ),
        Rule::Viewport(rule) => {
            write_named_declaration_rule(tree, "@viewport", "", rule.declarations, f)
        }
        Rule::ViewTransition(rule) => {
            write_named_declaration_rule(tree, "@view-transition", "", rule.declarations, f)
        }
        Rule::NestedDeclarations(rule) => {
            write_named_declaration_rule(tree, "@nest", "", rule.declarations, f)
        }
        Rule::Keyframe(rule) => write_keyframe_rule(tree, rule, f),

        // page stays structural too
        Rule::Page(rule) => write_page_rule(tree, rule, f),
    }
}

/// Write one selector-based block rule prelude and body.
fn write_selector_block_rule(
    tree: &Tree,
    selectors: LocalNodeId<SelectorList>,
    declarations: Option<LocalNodeId<DeclarationBlock>>,
    rules: &[LocalNodeId<Rule>],
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    // prelude
    write_selector_list(tree, selectors, f)?;

    // body
    write_block_rule_body(tree, declarations, rules, f)
}

/// Write one media rule prelude and body.
fn write_media_rule(
    tree: &Tree,
    media: LocalNodeId<MediaQueryList>,
    rules: &[LocalNodeId<Rule>],
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    // prelude
    write!(f, [text("@media"), space()])?;
    write_media_query_list(tree, media, f)?;

    // body
    write_group_block_rule_body(tree, rules, f)
}

/// Write one supports rule prelude and body.
fn write_supports_rule(
    tree: &Tree,
    condition: LocalNodeId<SupportsCondition>,
    rules: &[LocalNodeId<Rule>],
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    // prelude
    write!(f, [text("@supports"), space()])?;
    write_supports_condition(tree, condition, f)?;

    // body
    write_group_block_rule_body(tree, rules, f)
}

/// Write one layer block rule prelude and body.
fn write_layer_block_rule(
    tree: &Tree,
    rule: &LayerBlockRule,
    rules: &[LocalNodeId<Rule>],
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    write!(f, [text("@layer")])?;

    if let Some(name) = &rule.name {
        write!(f, [space()])?;
        write_layer_name_list(name, f)?;
    }

    write_group_block_rule_body(tree, rules, f)
}

/// Write one container rule prelude and body.
fn write_container_rule(
    tree: &Tree,
    rule: &ContainerRule,
    rules: &[LocalNodeId<Rule>],
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    // prelude
    write!(f, [text("@container")])?;

    if let Some(name) = &rule.name {
        write!(f, [space(), text(&name.name)])?;
    }

    if let Some(condition) = rule.condition {
        write!(f, [space()])?;
        write_container_condition(tree, condition, f)?;
    }

    // body
    write_group_block_rule_body(tree, rules, f)
}

/// Write one scope rule prelude and body.
fn write_scope_rule(
    tree: &Tree,
    rule: &ScopeRule,
    rules: &[LocalNodeId<Rule>],
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    // prelude
    write!(f, [text("@scope")])?;

    if rule.scope_start.is_some() || rule.scope_end.is_some() {
        write!(f, [space(), token("(")])?;

        if let Some(scope_start) = rule.scope_start {
            write_selector_list(tree, scope_start, f)?;
        }

        if let Some(scope_end) = rule.scope_end {
            write!(f, [token(")"), space(), text("to"), space(), token("(")])?;
            write_selector_list(tree, scope_end, f)?;
        }

        write!(f, [token(")")])?;
    }

    // body
    write_group_block_rule_body(tree, rules, f)
}

/// Write one custom media rule.
fn write_custom_media_rule(
    tree: &Tree,
    rule: &CustomMediaRule,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    // prelude
    write!(
        f,
        [
            text("@custom-media"),
            space(),
            text(&rule.name.name),
            space()
        ]
    )?;
    write_media_query_list(tree, rule.query, f)?;

    // terminator
    write!(f, [token(";")])
}

/// Write one grouped block rule body after one prelude.
fn write_group_block_rule_body(
    tree: &Tree,
    rules: &[LocalNodeId<Rule>],
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    write!(f, [space(), token("{")])?;

    if !rules.is_empty() {
        write!(
            f,
            [
                indent(&format_with(|f| write_rule_list(tree, rules, f))),
                hard_line_break()
            ]
        )?;
    }

    write!(f, [token("}")])
}

/// Write one style-like block rule body after one prelude.
fn write_block_rule_body(
    tree: &Tree,
    declarations: Option<LocalNodeId<DeclarationBlock>>,
    rules: &[LocalNodeId<Rule>],
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    write!(f, [space(), token("{")])?;

    let has_declarations = declarations.is_some();
    let has_rules = !rules.is_empty();

    if has_declarations || has_rules {
        write!(
            f,
            [
                indent(&format_with(|f| {
                    write!(f, [hard_line_break()])?;
                    write_rule_body(tree, declarations, rules, f)
                })),
                hard_line_break()
            ]
        )?;
    }

    write!(f, [token("}")])
}

/// Write one selector list.
fn write_selector_list(
    tree: &Tree,
    selectors: LocalNodeId<SelectorList>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    let selectors = tree.get(selectors);

    for (index, selector_id) in selectors.selectors.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), soft_line_break_or_space()])?;
        }

        write_selector(tree, *selector_id, f)?;
    }

    Ok(())
}

/// Write one identifier token.
fn write_identifier(value: &str, f: &mut Formatter<'_, CssFormatContext>) -> FormatResult<()> {
    let source = TokenRenderer::render_identifier_source(value);

    write!(f, [text(&source)])
}

/// Write one selector.
fn write_selector(
    tree: &Tree,
    selector: LocalNodeId<Selector>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    let selector = tree.get(selector);

    for component_id in &selector.components {
        write_selector_component(tree, *component_id, f)?;
    }

    Ok(())
}

/// Write one selector component.
fn write_selector_component(
    tree: &Tree,
    component: LocalNodeId<SelectorComponent>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    match tree.get(component) {
        SelectorComponent::Combinator(combinator) => write_selector_combinator(*combinator, f),
        SelectorComponent::Simple(simple) => write_simple_selector(tree, *simple, f),
    }
}

/// Write one selector combinator.
fn write_selector_combinator(
    combinator: Combinator,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    match combinator {
        Combinator::Child => write!(f, [space(), token(">"), soft_line_break_or_space()]),
        Combinator::Descendant => write!(f, [soft_line_break_or_space()]),
        Combinator::NextSibling => write!(f, [space(), token("+"), soft_line_break_or_space()]),
        Combinator::LaterSibling => write!(f, [space(), token("~"), soft_line_break_or_space()]),
        Combinator::PseudoElement => write!(f, [token("::")]),
        Combinator::SlotAssignment => {
            write!(f, [space(), token("/deep/"), soft_line_break_or_space()])
        }
        Combinator::Part => write!(f, [token("::part")]),
        Combinator::DeepDescendant => {
            write!(f, [space(), token(">>>"), soft_line_break_or_space()])
        }
        Combinator::Deep => write!(f, [space(), token(">>"), soft_line_break_or_space()]),
    }
}

/// Write one simple selector.
fn write_simple_selector(
    tree: &Tree,
    selector: LocalNodeId<SimpleSelector>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    match tree.get(selector) {
        SimpleSelector::ExplicitAnyNamespace => write!(f, [token("*|")]),
        SimpleSelector::ExplicitNoNamespace => write!(f, [token("|")]),
        SimpleSelector::DefaultNamespace => Ok(()),
        SimpleSelector::Namespace(prefix) => {
            write_identifier(prefix, f)?;
            write!(f, [token("|")])
        }
        SimpleSelector::ExplicitUniversalType => write!(f, [token("*")]),
        SimpleSelector::LocalName(name) => write_identifier(&name.name, f),
        SimpleSelector::Id(value) => {
            write!(f, [token("#")])?;
            write_identifier(value, f)
        }
        SimpleSelector::Class(value) => {
            write!(f, [token(".")])?;
            write_identifier(value, f)
        }
        SimpleSelector::Attribute(selector) => {
            let selector = tree.get(*selector);

            write!(
                f,
                [
                    token("["),
                    format_with(|f| write_component_value_list(&selector.components, f)),
                    token("]")
                ]
            )
        }
        SimpleSelector::Negation(selectors) => {
            write!(f, [text(":not(")])?;
            write_selector_list(tree, *selectors, f)?;
            write!(f, [token(")")])
        }
        SimpleSelector::Root => write!(f, [text(":root")]),
        SimpleSelector::Empty => write!(f, [text(":empty")]),
        SimpleSelector::Scope => write!(f, [text(":scope")]),
        SimpleSelector::Nth(selector) => write_nth_selector(tree, *selector, f),
        SimpleSelector::NthOf(selector) => write_nth_of_selector(tree, *selector, f),
        SimpleSelector::PseudoClass(selector) => write_pseudo_class(tree, *selector, f),
        SimpleSelector::Slotted(selector) => {
            write!(f, [text("::slotted(")])?;
            write_selector(tree, *selector, f)?;
            write!(f, [token(")")])
        }
        SimpleSelector::Part(parts) => {
            write!(f, [text("::part(")])?;

            for (index, part) in parts.iter().enumerate() {
                if index > 0 {
                    write!(f, [space()])?;
                }

                write_identifier(part, f)?;
            }

            write!(f, [token(")")])
        }
        SimpleSelector::Host(selector) => {
            write!(f, [text(":host")])?;

            if let Some(selector) = selector {
                write!(f, [token("(")])?;
                write_selector(tree, *selector, f)?;
                write!(f, [token(")")])?;
            }

            Ok(())
        }
        SimpleSelector::Where(selectors) => {
            write!(f, [text(":where(")])?;
            write_selector_list(tree, *selectors, f)?;
            write!(f, [token(")")])
        }
        SimpleSelector::Is(selectors) => {
            write!(f, [text(":is(")])?;
            write_selector_list(tree, *selectors, f)?;
            write!(f, [token(")")])
        }
        SimpleSelector::Any(selector) => write_any_selector(tree, *selector, f),
        SimpleSelector::Has(selectors) => {
            write!(f, [text(":has(")])?;
            write_selector_list(tree, *selectors, f)?;
            write!(f, [token(")")])
        }
        SimpleSelector::PseudoElement(selector) => write_pseudo_element(tree, *selector, f),
        SimpleSelector::Nesting => write!(f, [token("&")]),
    }
}

/// Write one pseudo class.
fn write_pseudo_class(
    tree: &Tree,
    selector: LocalNodeId<PseudoClass>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    let selector = tree.get(selector);

    write!(f, [token(":"), text(&selector.name)])?;

    if let Some(arguments) = &selector.arguments {
        write!(f, [token("(")])?;
        write_pseudo_argument(tree, arguments, f)?;
        write!(f, [token(")")])?;
    }

    Ok(())
}

/// Write one pseudo element.
fn write_pseudo_element(
    tree: &Tree,
    selector: LocalNodeId<PseudoElement>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    let selector = tree.get(selector);

    write!(f, [text(&selector.name)])?;

    if let Some(arguments) = &selector.arguments {
        write!(f, [token("(")])?;
        write_pseudo_argument(tree, arguments, f)?;
        write!(f, [token(")")])?;
    }

    Ok(())
}

/// Write one vendor any selector.
fn write_any_selector(
    tree: &Tree,
    selector: LocalNodeId<AnySelector>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    let selector = tree.get(selector);
    let prefix = match selector.vendor_prefix {
        VendorPrefix::None => "",
        VendorPrefix::Webkit => "-webkit-",
        VendorPrefix::Moz => "-moz-",
        VendorPrefix::Ms => "-ms-",
        VendorPrefix::O => "-o-",
        VendorPrefix::Other(ref prefix) => prefix,
    };

    write!(f, [token(":"), text(prefix), text("any(")])?;
    write_selector_list(tree, selector.selectors, f)?;
    write!(f, [token(")")])
}

/// Write one pseudo argument.
fn write_pseudo_argument(
    tree: &Tree,
    argument: &PseudoArgument,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    match argument {
        PseudoArgument::Components(arguments) => write_component_value_list(arguments, f),
        PseudoArgument::Selector(selector) => write_selector(tree, *selector, f),
        PseudoArgument::ViewTransitionPart(argument) => {
            if let Some(name) = &argument.name {
                write!(f, [text(name)])?;
            }

            for class in &argument.classes {
                write!(f, [token("."), text(class)])?;
            }

            Ok(())
        }
    }
}

/// Write one nth selector.
fn write_nth_selector(
    tree: &Tree,
    selector: LocalNodeId<NthSelector>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    let selector = tree.get(selector);
    let name = match selector.kind {
        NthSelectorKind::Child if selector.is_function => "nth-child(",
        NthSelectorKind::Child => "first-child",
        NthSelectorKind::LastChild if selector.is_function => "nth-last-child(",
        NthSelectorKind::LastChild => "last-child",
        NthSelectorKind::OfType if selector.is_function => "nth-of-type(",
        NthSelectorKind::OfType => "first-of-type",
        NthSelectorKind::LastOfType if selector.is_function => "nth-last-of-type(",
        NthSelectorKind::LastOfType => "last-of-type",
        NthSelectorKind::OnlyChild => "only-child",
        NthSelectorKind::OnlyOfType => "only-of-type",
        NthSelectorKind::Column => "nth-col(",
        NthSelectorKind::LastColumn => "nth-last-col(",
    };

    write!(f, [token(":"), text(name)])?;

    if selector.is_function {
        write_affine(selector.a, selector.b, f)?;
        write!(f, [token(")")])?;
    }

    Ok(())
}

/// Write one nth-of selector.
fn write_nth_of_selector(
    tree: &Tree,
    selector: LocalNodeId<NthOfSelector>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    let selector = tree.get(selector);
    let nth = tree.get(selector.nth);
    let name = match nth.kind {
        NthSelectorKind::Child => "nth-child(",
        NthSelectorKind::LastChild => "nth-last-child(",
        NthSelectorKind::OfType => "nth-of-type(",
        NthSelectorKind::LastOfType => "nth-last-of-type(",
        NthSelectorKind::OnlyChild => "only-child",
        NthSelectorKind::OnlyOfType => "only-of-type",
        NthSelectorKind::Column => "nth-col(",
        NthSelectorKind::LastColumn => "nth-last-col(",
    };

    write!(f, [token(":"), text(name)])?;
    write_affine(nth.a, nth.b, f)?;
    write!(f, [space(), text("of"), soft_line_break_or_space()])?;
    write_selector_list(tree, selector.selectors, f)?;
    write!(f, [token(")")])
}

/// Write one affine nth expression.
fn write_affine(a: i32, b: i32, f: &mut Formatter<'_, CssFormatContext>) -> FormatResult<()> {
    // special forms
    match (a, b) {
        (0, 0) => return write!(f, [text("0")]),
        (1, 0) => return write!(f, [text("n")]),
        (-1, 0) => return write!(f, [text("-n")]),
        (2, 1) => return write!(f, [text("odd")]),
        _ => {}
    }

    // pure offset
    if a == 0 {
        return write!(f, [text(&b.to_string())]);
    }

    // coefficient
    match a {
        1 => write!(f, [text("n")])?,
        -1 => write!(f, [text("-n")])?,
        _ => write!(f, [text(&a.to_string()), text("n")])?,
    }

    // optional offset
    if b > 0 {
        write!(f, [text("+"), text(&b.to_string())])?;
    } else if b < 0 {
        write!(f, [text(&b.to_string())])?;
    }

    Ok(())
}

/// Write one media query list.
fn write_media_query_list(
    tree: &Tree,
    media: LocalNodeId<MediaQueryList>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    let media = tree.get(media);

    for (index, query) in media.queries.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), soft_line_break_or_space()])?;
        }

        write_media_query(tree, *query, f)?;
    }

    Ok(())
}

/// Write one media query.
fn write_media_query(
    tree: &Tree,
    query: LocalNodeId<MediaQuery>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    let query = tree.get(query);
    let mut has_prefix = false;

    if let Some(qualifier) = query.qualifier {
        let qualifier = match qualifier {
            MediaQualifier::Only => "only",
            MediaQualifier::Not => "not",
        };

        write!(f, [text(qualifier)])?;
        has_prefix = true;
    }

    let media_type = match &query.media_type {
        MediaType::All => "all",
        MediaType::Print => "print",
        MediaType::Screen => "screen",
        MediaType::Custom(value) => value,
    };

    if has_prefix {
        write!(f, [space()])?;
    }

    write!(f, [text(media_type)])?;

    if let Some(condition) = query.condition {
        write!(f, [space(), text("and"), soft_line_break_or_space()])?;
        write_media_condition(tree, condition, f)?;
    }

    Ok(())
}

/// Write one media condition.
fn write_media_condition(
    tree: &Tree,
    condition: LocalNodeId<MediaCondition>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    match tree.get(condition) {
        MediaCondition::Feature(feature) => write_query_feature(tree, *feature, f),
        MediaCondition::Not(condition) => {
            write!(f, [text("not"), space()])?;
            write_parenthesized_media(tree, *condition, f)
        }
        MediaCondition::Operation {
            operator,
            conditions,
        } => write_condition_sequence(
            conditions,
            match operator {
                ConditionOperator::And => "and",
                ConditionOperator::Or => "or",
            },
            f,
            |condition, f| write_parenthesized_media(tree, *condition, f),
        ),
        MediaCondition::Unknown(condition) => write_component_value_list(&condition.components, f),
    }
}

/// Write one parenthesized media condition when needed.
fn write_parenthesized_media(
    tree: &Tree,
    condition: LocalNodeId<MediaCondition>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    match tree.get(condition) {
        MediaCondition::Feature(_) | MediaCondition::Unknown(_) => {
            write_media_condition(tree, condition, f)
        }
        MediaCondition::Not(_) | MediaCondition::Operation { .. } => {
            write!(f, [token("(")])?;
            write_media_condition(tree, condition, f)?;
            write!(f, [token(")")])
        }
    }
}

/// Write one supports condition.
fn write_supports_condition(
    tree: &Tree,
    condition: LocalNodeId<SupportsCondition>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    match tree.get(condition) {
        SupportsCondition::Not(condition) => {
            write!(f, [text("not"), space()])?;
            write_parenthesized_supports(tree, *condition, f)
        }
        SupportsCondition::And(conditions) => {
            write_condition_sequence(conditions, "and", f, |condition, f| {
                write_parenthesized_supports(tree, *condition, f)
            })
        }
        SupportsCondition::Or(conditions) => {
            write_condition_sequence(conditions, "or", f, |condition, f| {
                write_parenthesized_supports(tree, *condition, f)
            })
        }
        SupportsCondition::Declaration { property, value } => {
            write!(
                f,
                [
                    token("("),
                    text(f.context().render_property_name(property)),
                    token(":"),
                    space()
                ]
            )?;
            write_component_value_list(value.components(), f)?;
            write!(f, [token(")")])
        }
        SupportsCondition::Selector(selector) => {
            write!(f, [text("selector(")])?;
            write_selector_list(tree, selector.selectors, f)?;
            write!(f, [token(")")])
        }
        SupportsCondition::Unknown(condition) => {
            write_component_value_list(&condition.components, f)
        }
    }
}

/// Write one parenthesized supports condition when needed.
fn write_parenthesized_supports(
    tree: &Tree,
    condition: LocalNodeId<SupportsCondition>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    match tree.get(condition) {
        SupportsCondition::Declaration { .. }
        | SupportsCondition::Selector(_)
        | SupportsCondition::Unknown(_) => write_supports_condition(tree, condition, f),
        SupportsCondition::Not(_) | SupportsCondition::And(_) | SupportsCondition::Or(_) => {
            write!(f, [token("(")])?;
            write_supports_condition(tree, condition, f)?;
            write!(f, [token(")")])
        }
    }
}

/// Write one container condition.
fn write_container_condition(
    tree: &Tree,
    condition: LocalNodeId<ContainerCondition>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    match tree.get(condition) {
        ContainerCondition::Feature(feature) => write_query_feature(tree, *feature, f),
        ContainerCondition::Not(condition) => {
            write!(f, [text("not"), space()])?;
            write_parenthesized_container(tree, *condition, f)
        }
        ContainerCondition::Operation {
            operator,
            conditions,
        } => write_condition_sequence(
            conditions,
            match operator {
                ConditionOperator::And => "and",
                ConditionOperator::Or => "or",
            },
            f,
            |condition, f| write_parenthesized_container(tree, *condition, f),
        ),
        ContainerCondition::Style(query) => {
            write!(f, [text("style(")])?;
            write_container_style_query(tree, *query, f)?;
            write!(f, [token(")")])
        }
        ContainerCondition::ScrollState(query) => {
            write!(f, [text("scroll-state(")])?;
            write_container_scroll_state_query(tree, *query, f)?;
            write!(f, [token(")")])
        }
        ContainerCondition::Unknown(condition) => {
            write_component_value_list(&condition.components, f)
        }
    }
}

/// Write one parenthesized container condition when needed.
fn write_parenthesized_container(
    tree: &Tree,
    condition: LocalNodeId<ContainerCondition>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    match tree.get(condition) {
        ContainerCondition::Feature(_)
        | ContainerCondition::Style(_)
        | ContainerCondition::ScrollState(_)
        | ContainerCondition::Unknown(_) => write_container_condition(tree, condition, f),
        ContainerCondition::Not(_) | ContainerCondition::Operation { .. } => {
            write!(f, [token("(")])?;
            write_container_condition(tree, condition, f)?;
            write!(f, [token(")")])
        }
    }
}

/// Write one boolean condition sequence.
fn write_condition_sequence<T>(
    conditions: &[T],
    operator: &str,
    f: &mut Formatter<'_, CssFormatContext>,
    mut write_condition: impl FnMut(&T, &mut Formatter<'_, CssFormatContext>) -> FormatResult<()>,
) -> FormatResult<()> {
    for (index, condition) in conditions.iter().enumerate() {
        if index > 0 {
            write!(
                f,
                [
                    soft_line_break_or_space(),
                    text(operator),
                    soft_line_break_or_space()
                ]
            )?;
        }

        write_condition(condition, f)?;
    }

    Ok(())
}

/// Write one query feature.
fn write_query_feature(
    tree: &Tree,
    feature: LocalNodeId<QueryFeature>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    match tree.get(feature) {
        QueryFeature::Plain { name, value } => {
            write!(f, [token("(")])?;
            write_feature_name(tree, *name, f)?;
            write!(f, [token(":"), space()])?;
            write_feature_value(tree, *value, f)?;
            write!(f, [token(")")])
        }
        QueryFeature::Boolean { name } => {
            write!(f, [token("(")])?;
            write_feature_name(tree, *name, f)?;
            write!(f, [token(")")])
        }
        QueryFeature::Range {
            name,
            operator,
            value,
        } => {
            write!(f, [token("(")])?;
            write_feature_name(tree, *name, f)?;
            write_feature_comparison(*operator, f)?;
            write_feature_value(tree, *value, f)?;
            write!(f, [token(")")])
        }
        QueryFeature::Interval {
            name,
            start,
            start_operator,
            end,
            end_operator,
        } => {
            write!(f, [token("(")])?;
            write_feature_value(tree, *start, f)?;
            write_feature_comparison(*start_operator, f)?;
            write_feature_name(tree, *name, f)?;
            write_feature_comparison(*end_operator, f)?;
            write_feature_value(tree, *end, f)?;
            write!(f, [token(")")])
        }
    }
}

/// Write one feature name.
fn write_feature_name(
    tree: &Tree,
    name: LocalNodeId<FeatureName>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    match tree.get(name) {
        FeatureName::Standard(name) | FeatureName::Custom(name) | FeatureName::Unknown(name) => {
            write!(f, [text(name)])
        }
    }
}

/// Write one feature comparison.
fn write_feature_comparison(
    comparison: FeatureComparison,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    let source = match comparison {
        FeatureComparison::Equal => " = ",
        FeatureComparison::GreaterThan => " > ",
        FeatureComparison::GreaterThanEqual => " >= ",
        FeatureComparison::LessThan => " < ",
        FeatureComparison::LessThanEqual => " <= ",
    };

    write!(f, [text(source)])
}

/// Write one feature value.
fn write_feature_value(
    tree: &Tree,
    value: LocalNodeId<FeatureValue>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    match tree.get(value) {
        FeatureValue::Length(value) | FeatureValue::Resolution(value) => {
            write_component_value_list(value.components(), f)
        }
        FeatureValue::Number(value) => write_number(*value, f),
        FeatureValue::Integer(value) => write!(f, [text(&value.to_string())]),
        FeatureValue::Boolean(value) => {
            write!(f, [text(if *value { "1" } else { "0" })])
        }
        FeatureValue::Ratio(value) => write_ratio_value(tree, *value, f),
        FeatureValue::Ident(value) => write!(f, [text(value)]),
        FeatureValue::EnvironmentVariable(value) => write_environment_variable(tree, *value, f),
    }
}

/// Write one ratio value.
fn write_ratio_value(
    tree: &Tree,
    value: LocalNodeId<RatioValue>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    let value = tree.get(value);

    write_number(value.numerator, f)?;
    write!(f, [space(), token("/"), space()])?;
    write_number(value.denominator, f)
}

/// Write one environment variable.
fn write_environment_variable(
    tree: &Tree,
    value: LocalNodeId<EnvironmentVariable>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    let value = tree.get(value);

    write!(f, [text("env(")])?;
    write_environment_variable_name(&value.name, f)?;

    for index in &value.indices {
        write!(f, [space(), text(&index.to_string())])?;
    }

    if let Some(fallback) = &value.fallback {
        write!(f, [token(","), space()])?;
        write_component_value_list(fallback, f)?;
    }

    write!(f, [token(")")])
}

/// Write one environment variable name.
fn write_environment_variable_name(
    name: &EnvironmentVariableName,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    match name {
        EnvironmentVariableName::Ua(value)
        | EnvironmentVariableName::Custom(value)
        | EnvironmentVariableName::Unknown(value) => write!(f, [text(value)]),
    }
}

/// Write one style query.
fn write_container_style_query(
    tree: &Tree,
    query: LocalNodeId<ContainerStyleQuery>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    match tree.get(query) {
        ContainerStyleQuery::Declaration { property, value } => {
            write!(
                f,
                [
                    token("("),
                    text(f.context().render_property_name(property)),
                    token(":"),
                    space()
                ]
            )?;
            write_component_value_list(value.components(), f)?;
            write!(f, [token(")")])
        }
        ContainerStyleQuery::Property(property) => {
            write!(
                f,
                [
                    token("("),
                    text(f.context().render_property_name(property)),
                    token(")")
                ]
            )
        }
        ContainerStyleQuery::Not(query) => {
            write!(f, [text("not"), space()])?;
            write_parenthesized_style_query(tree, *query, f)
        }
        ContainerStyleQuery::Operation {
            operator,
            conditions,
        } => write_condition_sequence(
            conditions,
            match operator {
                ConditionOperator::And => "and",
                ConditionOperator::Or => "or",
            },
            f,
            |query, f| write_parenthesized_style_query(tree, *query, f),
        ),
    }
}

/// Write one scroll state query.
fn write_container_scroll_state_query(
    tree: &Tree,
    query: LocalNodeId<ContainerScrollStateQuery>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    match tree.get(query) {
        ContainerScrollStateQuery::Feature(feature) => write_query_feature(tree, *feature, f),
        ContainerScrollStateQuery::Not(query) => {
            write!(f, [text("not"), space()])?;
            write_parenthesized_scroll_state_query(tree, *query, f)
        }
        ContainerScrollStateQuery::Operation {
            operator,
            conditions,
        } => write_condition_sequence(
            conditions,
            match operator {
                ConditionOperator::And => "and",
                ConditionOperator::Or => "or",
            },
            f,
            |query, f| write_parenthesized_scroll_state_query(tree, *query, f),
        ),
    }
}

/// Write one parenthesized style query when needed.
fn write_parenthesized_style_query(
    tree: &Tree,
    query: LocalNodeId<ContainerStyleQuery>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    match tree.get(query) {
        ContainerStyleQuery::Declaration { .. } | ContainerStyleQuery::Property(_) => {
            write_container_style_query(tree, query, f)
        }
        ContainerStyleQuery::Not(_) | ContainerStyleQuery::Operation { .. } => {
            write!(f, [token("(")])?;
            write_container_style_query(tree, query, f)?;
            write!(f, [token(")")])
        }
    }
}

/// Write one parenthesized scroll state query when needed.
fn write_parenthesized_scroll_state_query(
    tree: &Tree,
    query: LocalNodeId<ContainerScrollStateQuery>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    match tree.get(query) {
        ContainerScrollStateQuery::Feature(_) => write_container_scroll_state_query(tree, query, f),
        ContainerScrollStateQuery::Not(_) | ContainerScrollStateQuery::Operation { .. } => {
            write!(f, [token("(")])?;
            write_container_scroll_state_query(tree, query, f)?;
            write!(f, [token(")")])
        }
    }
}

/// Write one import rule.
fn write_import_rule(
    tree: &Tree,
    rule: &ImportRule,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    // import url
    write!(
        f,
        [
            text("@import"),
            space(),
            text("\""),
            text(&rule.url),
            text("\"")
        ]
    )?;

    if let Some(layer) = &rule.layer {
        write!(f, [space()])?;
        write_import_layer(layer, f)?;
    }

    if let Some(supports) = rule.supports {
        write!(f, [space(), text("supports"), space()])?;
        write_supports_condition(tree, supports, f)?;
    }

    if let Some(media) = rule.media {
        write!(f, [space()])?;
        write_media_query_list(tree, media, f)?;
    }

    // terminator
    write!(f, [token(";")])
}

/// Write one grouped block rule.
fn write_group_block_rule(
    tree: &Tree,
    prelude: &str,
    rules: &[LocalNodeId<Rule>],
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    write!(f, [text(prelude), space(), token("{")])?;

    if !rules.is_empty() {
        write!(
            f,
            [
                indent(&format_with(|f| write_rule_list(tree, rules, f))),
                hard_line_break()
            ]
        )?;
    }

    write!(f, [token("}")])
}

/// Write one `@keyframes` grouped rule.
fn write_keyframes_group_rule(
    tree: &Tree,
    rule: &KeyframesRule,
    rules: &[LocalNodeId<Rule>],
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    // prefix
    write!(f, [text("@")])?;

    match &rule.vendor_prefix {
        VendorPrefix::None => {}
        VendorPrefix::Webkit => write!(f, [text("-webkit-")])?,
        VendorPrefix::Moz => write!(f, [text("-moz-")])?,
        VendorPrefix::Ms => write!(f, [text("-ms-")])?,
        VendorPrefix::O => write!(f, [text("-o-")])?,
        VendorPrefix::Other(prefix) => write!(f, [text(prefix.as_str())])?,
    }

    // header
    write!(
        f,
        [
            text("keyframes"),
            space(),
            text(&rule.name.name),
            space(),
            token("{")
        ]
    )?;

    if !rules.is_empty() {
        write!(
            f,
            [
                indent(&format_with(|f| write_rule_list(tree, rules, f))),
                hard_line_break()
            ]
        )?;
    }

    write!(f, [token("}")])
}

/// Write one declaration-only block rule.
fn write_named_declaration_rule(
    tree: &Tree,
    prefix: &str,
    name: &str,
    declarations: Option<LocalNodeId<DeclarationBlock>>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    write!(f, [text(prefix)])?;

    if !name.is_empty() {
        write!(f, [space(), text(name)])?;
    }

    write_declaration_only_block_body(tree, declarations, f)
}

/// Write one declaration-only block body after one prelude.
fn write_declaration_only_block_body(
    tree: &Tree,
    declarations: Option<LocalNodeId<DeclarationBlock>>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    write!(f, [space(), token("{")])?;

    if let Some(declarations) = declarations {
        write!(
            f,
            [
                indent(&format_with(|f| {
                    write!(f, [hard_line_break()])?;
                    write_declaration_block(tree, declarations, f)
                })),
                hard_line_break()
            ]
        )?;
    }

    write!(f, [token("}")])
}

/// Write one layer statement rule.
fn write_layer_statement_rule(
    rule: &LayerStatementRule,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    write!(f, [text("@layer"), space()])?;

    for (index, name) in rule.names.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }

        write_layer_name_list(name, f)?;
    }

    write!(f, [token(";")])
}

/// Write one namespace rule.
fn write_namespace_rule(
    rule: &NamespaceRule,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    write!(f, [text("@namespace"), space()])?;

    if let Some(prefix) = &rule.prefix {
        write!(f, [text(&prefix.name), space()])?;
    }

    match &rule.url {
        NamespaceUrl::String(value) => {
            write!(f, [text("\""), text(value), text("\"")])?;
        }
        NamespaceUrl::Url(value) => {
            write!(f, [text("url(\""), text(value), text("\")")])?;
        }
    }

    write!(f, [token(";")])
}

/// Write one property rule.
fn write_property_rule(
    rule: &PropertyRule,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    write!(
        f,
        [
            text("@property "),
            text(&rule.name.name),
            space(),
            token("{")
        ]
    )?;
    write!(
        f,
        [
            indent(&format_with(|f| {
                write!(f, [hard_line_break()])?;
                write!(
                    f,
                    [
                        text("syntax"),
                        token(":"),
                        space(),
                        format_with(|f| write_property_syntax(&rule.syntax, f)),
                        token(";")
                    ]
                )?;
                write!(f, [hard_line_break()])?;
                write!(
                    f,
                    [
                        text("inherits"),
                        token(":"),
                        space(),
                        text(if rule.inherits { "true" } else { "false" }),
                        token(";")
                    ]
                )?;

                if let Some(initial_value) = &rule.initial_value {
                    write!(f, [hard_line_break()])?;
                    write!(
                        f,
                        [
                            text("initial-value"),
                            token(":"),
                            space(),
                            format_with(|f| write_component_value_list(
                                initial_value.components(),
                                f
                            )),
                            token(";")
                        ]
                    )?;
                }

                Ok(())
            })),
            hard_line_break(),
            token("}")
        ]
    )
}

/// Write one unknown at-rule.
fn write_unknown_rule(
    rule: &UnknownRule,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    if let Some(block) = &rule.block {
        if rule.prelude.values.is_empty() {
            write!(
                f,
                [
                    text("@"),
                    text(&rule.name),
                    space(),
                    token("{"),
                    format_with(|f| write_component_value_list(block, f)),
                    token("}")
                ]
            )
        } else {
            write!(
                f,
                [
                    text("@"),
                    text(&rule.name),
                    space(),
                    format_with(|f| write_component_value_list(&rule.prelude, f)),
                    space(),
                    token("{"),
                    format_with(|f| write_component_value_list(block, f)),
                    token("}")
                ]
            )
        }
    } else if rule.prelude.values.is_empty() {
        write!(f, [text("@"), text(&rule.name), token(";")])
    } else {
        write!(
            f,
            [
                text("@"),
                text(&rule.name),
                space(),
                format_with(|f| write_component_value_list(&rule.prelude, f)),
                token(";")
            ]
        )
    }
}

/// Write one font feature values rule.
fn write_font_feature_values_rule(
    rule: &FontFeatureValuesRule,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    write!(f, [text("@font-feature-values"), space()])?;

    for (index, family) in rule.families.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }

        write!(f, [text(&family.name)])?;
    }

    write!(f, [space(), token("{")])?;

    if !rule.subrules.is_empty() {
        write!(
            f,
            [
                indent(&format_with(|f: &mut Formatter<'_, CssFormatContext>| {
                    for (subrule_index, subrule) in rule.subrules.iter().enumerate() {
                        write!(f, [hard_line_break()])?;

                        if subrule_index > 0 {
                            write!(f, [hard_line_break()])?;
                        }

                        write_font_feature_subrule_kind(subrule.kind, f)?;
                        write!(f, [space(), token("{")])?;

                        if !subrule.declarations.is_empty() {
                            write!(
                                f,
                                [
                                    indent(&format_with(
                                        |f: &mut Formatter<'_, CssFormatContext>| {
                                            for (declaration_index, declaration) in
                                                subrule.declarations.iter().enumerate()
                                            {
                                                write!(f, [hard_line_break()])?;

                                                if declaration_index > 0 {
                                                    write!(f, [hard_line_break()])?;
                                                }

                                                let values = declaration
                                                    .values
                                                    .iter()
                                                    .map(ToString::to_string)
                                                    .collect::<Vec<_>>()
                                                    .join(" ");
                                                write!(
                                                    f,
                                                    [
                                                        text(&declaration.name),
                                                        token(":"),
                                                        space(),
                                                        text(&values),
                                                        token(";")
                                                    ]
                                                )?;
                                            }

                                            Ok(())
                                        }
                                    )),
                                    hard_line_break()
                                ]
                            )?;
                        }

                        write!(f, [token("}")])?;
                    }

                    Ok(())
                })),
                hard_line_break()
            ]
        )?;
    }

    write!(f, [token("}")])
}

/// Write one page rule.
fn write_page_rule(
    tree: &Tree,
    rule: &PageRule,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    write!(f, [text("@page")])?;

    if !rule.selectors.selectors.is_empty() {
        write!(f, [space()])?;
        write_page_selector_list(&rule.selectors, f)?;
    }

    write!(f, [space(), token("{")])?;

    let has_declarations = rule.declarations.is_some();
    let has_page_margin_rules = !rule.page_margin_rules.is_empty();

    if has_declarations || has_page_margin_rules {
        write!(
            f,
            [
                indent(&format_with(|f| {
                    write!(f, [hard_line_break()])?;
                    let mut is_first = true;

                    if let Some(declarations) = rule.declarations {
                        write_declaration_block(tree, declarations, f)?;
                        is_first = false;
                    }

                    for page_margin_rule_id in &rule.page_margin_rules {
                        if !is_first {
                            write!(f, [hard_line_break()])?;
                        }

                        write_page_margin_rule(tree, *page_margin_rule_id, f)?;
                        is_first = false;
                    }

                    Ok(())
                })),
                hard_line_break()
            ]
        )?;
    }

    write!(f, [token("}")])
}

/// Write one keyframe rule.
fn write_keyframe_rule(
    tree: &Tree,
    rule: &KeyframeRule,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    write_keyframe_selector_list(&rule.selectors, f)?;
    write_declaration_only_block_body(tree, rule.declarations, f)
}

/// Write one keyframe selector list.
fn write_keyframe_selector_list(
    selectors: &KeyframeSelectorList,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    for (index, selector) in selectors.selectors.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }

        write_keyframe_selector(selector, f)?;
    }

    Ok(())
}

/// Write one keyframe selector.
fn write_keyframe_selector(
    selector: &KeyframeSelector,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    match selector {
        KeyframeSelector::Percentage(number) => {
            write_number(*number, f)?;
            write!(f, [token("%")])
        }
        KeyframeSelector::From => write!(f, [text("from")]),
        KeyframeSelector::To => write!(f, [text("to")]),
        KeyframeSelector::TimelineRangePercentage(selector) => {
            let name = match selector.name {
                TimelineRangeName::Cover => "cover",
                TimelineRangeName::Contain => "contain",
                TimelineRangeName::Entry => "entry",
                TimelineRangeName::Exit => "exit",
                TimelineRangeName::EntryCrossing => "entry-crossing",
                TimelineRangeName::ExitCrossing => "exit-crossing",
            };

            write!(f, [text(name), space()])?;
            write_number(selector.percentage, f)?;
            write!(f, [token("%")])
        }
    }
}

/// Write one page margin rule.
fn write_page_margin_rule(
    tree: &Tree,
    rule_id: LocalNodeId<PageMarginRule>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    let rule = tree.get(rule_id);
    write_page_margin_box(rule.margin_box, f)?;
    write!(f, [space(), token("{")])?;

    if let Some(declarations) = rule.declarations {
        write!(
            f,
            [
                indent(&format_with(|f| {
                    write!(f, [hard_line_break()])?;
                    write_declaration_block(tree, declarations, f)
                })),
                hard_line_break()
            ]
        )?;
    }

    write!(f, [token("}")])
}

/// Write one style-like rule body.
fn write_rule_body(
    tree: &Tree,
    declarations: Option<LocalNodeId<DeclarationBlock>>,
    rules: &[LocalNodeId<Rule>],
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    let mut is_first = true;

    // declarations
    if let Some(declarations) = declarations {
        write_declaration_block(tree, declarations, f)?;
        is_first = false;
    }

    // nested rules
    for rule_id in rules {
        if !is_first {
            write!(f, [hard_line_break()])?;
        }

        write_rule(tree, *rule_id, f)?;
        is_first = false;
    }

    Ok(())
}

/// Write one layer name list.
fn write_layer_name_list(
    name: &LayerNameList,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    for (index, segment) in name.names.iter().enumerate() {
        if index > 0 {
            write!(f, [token(".")])?;
        }

        write!(f, [text(segment)])?;
    }

    Ok(())
}

/// Write one import layer clause.
fn write_import_layer(
    layer: &ImportLayer,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    match &layer.name {
        Some(name) => {
            write!(f, [text("layer(")])?;
            write_layer_name_list(name, f)?;
            write!(f, [token(")")])
        }
        None => write!(f, [text("layer")]),
    }
}

/// Write one property syntax definition.
fn write_property_syntax(
    syntax: &PropertySyntax,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    match syntax {
        PropertySyntax::Universal => write!(f, [token("*")]),
        PropertySyntax::Components(components) => {
            for (index, component) in components.iter().enumerate() {
                if index > 0 {
                    write!(f, [space(), token("|"), space()])?;
                }

                write_property_syntax_component(component, f)?;
            }

            Ok(())
        }
    }
}

/// Write one property syntax component.
fn write_property_syntax_component(
    component: &PropertySyntaxComponent,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    write_property_syntax_component_kind(&component.kind, f)?;

    let multiplier = match component.multiplier {
        PropertySyntaxMultiplier::None => "",
        PropertySyntaxMultiplier::Space => "+",
        PropertySyntaxMultiplier::Comma => "#",
    };

    if !multiplier.is_empty() {
        write!(f, [text(multiplier)])?;
    }

    Ok(())
}

/// Write one property syntax component kind.
fn write_property_syntax_component_kind(
    kind: &PropertySyntaxComponentKind,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    let source = match kind {
        PropertySyntaxComponentKind::Length => "<length>",
        PropertySyntaxComponentKind::Number => "<number>",
        PropertySyntaxComponentKind::Percentage => "<percentage>",
        PropertySyntaxComponentKind::LengthPercentage => "<length-percentage>",
        PropertySyntaxComponentKind::String => "<string>",
        PropertySyntaxComponentKind::Color => "<color>",
        PropertySyntaxComponentKind::Image => "<image>",
        PropertySyntaxComponentKind::Url => "<url>",
        PropertySyntaxComponentKind::Integer => "<integer>",
        PropertySyntaxComponentKind::Angle => "<angle>",
        PropertySyntaxComponentKind::Time => "<time>",
        PropertySyntaxComponentKind::Resolution => "<resolution>",
        PropertySyntaxComponentKind::TransformFunction => "<transform-function>",
        PropertySyntaxComponentKind::TransformList => "<transform-list>",
        PropertySyntaxComponentKind::CustomIdent => "<custom-ident>",
        PropertySyntaxComponentKind::Literal(value) => value,
    };

    write!(f, [text(source)])
}

/// Write one font feature values subrule kind.
fn write_font_feature_subrule_kind(
    kind: FontFeatureSubruleKind,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    let source = match kind {
        FontFeatureSubruleKind::Stylistic => "stylistic",
        FontFeatureSubruleKind::HistoricalForms => "historical-forms",
        FontFeatureSubruleKind::Styleset => "styleset",
        FontFeatureSubruleKind::CharacterVariant => "character-variant",
        FontFeatureSubruleKind::Swash => "swash",
        FontFeatureSubruleKind::Ornaments => "ornaments",
        FontFeatureSubruleKind::Annotation => "annotation",
    };

    write!(f, [token("@"), text(source)])
}

/// Write one page selector list.
fn write_page_selector_list(
    selectors: &PageSelectorList,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    for (index, selector) in selectors.selectors.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }

        write_page_selector(selector, f)?;
    }

    Ok(())
}

/// Write one page selector.
fn write_page_selector(
    selector: &PageSelector,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    if let Some(name) = &selector.name {
        write!(f, [text(name)])?;
    }

    for pseudo_class in &selector.pseudo_classes {
        write!(
            f,
            [
                token(":"),
                text(match pseudo_class {
                    PagePseudoClass::Left => "left",
                    PagePseudoClass::Right => "right",
                    PagePseudoClass::First => "first",
                    PagePseudoClass::Last => "last",
                    PagePseudoClass::Blank => "blank",
                })
            ]
        )?;
    }

    Ok(())
}

/// Write one page margin box.
fn write_page_margin_box(
    margin_box: PageMarginBox,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    let source = match margin_box {
        PageMarginBox::TopLeftCorner => "@top-left-corner",
        PageMarginBox::TopLeft => "@top-left",
        PageMarginBox::TopCenter => "@top-center",
        PageMarginBox::TopRight => "@top-right",
        PageMarginBox::TopRightCorner => "@top-right-corner",
        PageMarginBox::LeftTop => "@left-top",
        PageMarginBox::LeftMiddle => "@left-middle",
        PageMarginBox::LeftBottom => "@left-bottom",
        PageMarginBox::RightTop => "@right-top",
        PageMarginBox::RightMiddle => "@right-middle",
        PageMarginBox::RightBottom => "@right-bottom",
        PageMarginBox::BottomLeftCorner => "@bottom-left-corner",
        PageMarginBox::BottomLeft => "@bottom-left",
        PageMarginBox::BottomCenter => "@bottom-center",
        PageMarginBox::BottomRight => "@bottom-right",
        PageMarginBox::BottomRightCorner => "@bottom-right-corner",
    };

    write!(f, [text(source)])
}

/// Write one declaration block.
fn write_declaration_block(
    tree: &Tree,
    declaration_block_id: LocalNodeId<DeclarationBlock>,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    let declaration_block = tree.get(declaration_block_id);

    // declarations
    for (index, declaration_id) in declaration_block.declarations.iter().enumerate() {
        if index > 0 {
            write!(f, [hard_line_break()])?;
        }

        let declaration = tree.get(*declaration_id);
        let name = match &declaration.name {
            PropertyName::Standard(name) | PropertyName::Custom(name) => name.as_str(),
        };

        write!(
            f,
            [
                text(name),
                token(":"),
                space(),
                format_with(|f| write_component_value_list(declaration.value.components(), f))
            ]
        )?;

        if declaration.is_important {
            write!(f, [space(), token("!important")])?;
        }

        write!(f, [token(";")])?;
    }

    Ok(())
}

/// Write one component value list.
fn write_component_value_list(
    components: &ComponentValueList,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    // component values
    for value in &components.values {
        write_component_value(value, f)?;
    }

    Ok(())
}

/// Write one component value.
fn write_component_value(
    value: &ComponentValue,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    match value {
        ComponentValue::Token(token) => write_component_token(token, f),
        ComponentValue::Function(function) => write_component_function(function, f),
        ComponentValue::Block(block) => write_component_block(block, f),
    }
}

/// Write one function component value.
fn write_component_function(
    function: &Function,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    let function_name = f.context().strings.get(function.name).to_string();
    write!(f, [text(&function_name), token("(")])?;

    // nested values
    write_component_value_list(&function.arguments, f)?;

    write!(f, [token(")")])
}

/// Write one simple block component value.
fn write_component_block(
    block: &SimpleBlock,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    let (start, end) = match block.kind {
        BlockKind::Parenthesis => ("(", ")"),
        BlockKind::SquareBracket => ("[", "]"),
        BlockKind::CurlyBracket => ("{", "}"),
    };

    write!(f, [token(start)])?;

    // nested values
    write_component_value_list(&block.value, f)?;

    write!(f, [token(end)])
}

/// Write one token component value.
fn write_component_token(
    token_value: &Token,
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    let source = f.context().render_component_token(token_value);

    write!(f, [text(&source)])
}

/// Write one number token payload.
fn write_number(number: Number, f: &mut Formatter<'_, CssFormatContext>) -> FormatResult<()> {
    write_component_token(&Token::Number(number), f)
}

/// Render one component token as canonical CSS.
/// Write one nested rule list.
fn write_rule_list(
    tree: &Tree,
    rules: &[LocalNodeId<Rule>],
    f: &mut Formatter<'_, CssFormatContext>,
) -> FormatResult<()> {
    for (index, rule_id) in rules.iter().enumerate() {
        write!(f, [hard_line_break()])?;

        if index > 0 {
            write!(f, [hard_line_break()])?;
        }

        write_rule(tree, *rule_id, f)?;
    }

    Ok(())
}
