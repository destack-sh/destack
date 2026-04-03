use super::printer::Printer;
use crate::{
    ComponentValueList, ContainerRule, DeclarationBlock, FontFeatureSubruleKind,
    FontFeatureValuesRule, ImportRule, KeyframeRule, LayerStatementRule, LocalNodeId,
    NamespaceRule, NamespaceUrl, NestedDeclarationsRule, PageMarginRule, PageRule, PropertyRule,
    PropertySyntax, PropertySyntaxComponent, PropertySyntaxComponentKind, PropertySyntaxMultiplier,
    Rule, ScopeRule, SelectorList, StyleRule, UnknownRule, VendorPrefix,
};

impl<'a> Printer<'a> {
    /// Print one CSS rule node.
    pub(crate) fn print_rule_id(&mut self, rule_id: LocalNodeId<Rule>) {
        match self.tree.get(rule_id) {
            Rule::Import(rule) => self.print_import_rule(rule),
            Rule::Style(rule) => self.print_style_rule(rule),
            Rule::Media(rule) => self.print_group_rule(
                "@media ",
                &self.render_media_query_list(rule.query),
                &rule.rules,
            ),
            Rule::Supports(rule) => self.print_group_rule(
                "@supports ",
                &self.render_supports_condition(rule.condition),
                &rule.rules,
            ),
            Rule::LayerBlock(rule) => {
                let name = rule
                    .name
                    .as_ref()
                    .map(Self::render_layer_name_list)
                    .unwrap_or_default();
                self.print_group_rule("@layer", &name, &rule.rules);
            }
            Rule::Container(rule) => self.print_container_rule(rule),
            Rule::Scope(rule) => self.print_scope_rule(rule),
            Rule::StartingStyle(rule) => self.print_group_rule("@starting-style", "", &rule.rules),
            Rule::Keyframes(rule) => {
                self.source.push('@');
                self.source.push_str(match &rule.vendor_prefix {
                    VendorPrefix::None => "",
                    VendorPrefix::Webkit => "-webkit-",
                    VendorPrefix::Moz => "-moz-",
                    VendorPrefix::Ms => "-ms-",
                    VendorPrefix::O => "-o-",
                    VendorPrefix::Other(prefix) => prefix,
                });
                self.source.push_str("keyframes ");
                self.source.push_str(&rule.name.name);
                self.source.push('{');
                self.print_rule_list(&rule.rules);
                self.source.push('}');
            }
            Rule::MozDocument(rule) => {
                self.print_group_rule("@-moz-document url-prefix()", "", &rule.rules)
            }
            Rule::LayerStatement(rule) => self.print_layer_statement_rule(rule),
            Rule::FontFeatureValues(rule) => self.print_font_feature_values_rule(rule),
            Rule::Namespace(rule) => self.print_namespace_rule(rule),
            Rule::CustomMedia(rule) => {
                self.source.push_str("@custom-media ");
                self.source.push_str(&rule.name.name);
                self.source.push(' ');
                self.source
                    .push_str(&self.render_media_query_list(rule.query));
                self.source.push(';');
            }
            Rule::Property(rule) => self.print_property_rule(rule),
            Rule::Unknown(rule) => self.print_unknown_rule(rule),
            Rule::Custom(rule) => self.print_component_value_rule(&rule.components),
            Rule::Page(rule) => self.print_page_rule(rule),
            Rule::FontFace(rule) => self.print_declaration_rule("@font-face", rule.declarations),
            Rule::FontPaletteValues(rule) => {
                self.print_named_declaration_rule(
                    "@font-palette-values ",
                    &rule.name.name,
                    rule.declarations,
                );
            }
            Rule::CounterStyle(rule) => {
                self.print_named_declaration_rule(
                    "@counter-style ",
                    &rule.name.name,
                    rule.declarations,
                );
            }
            Rule::Viewport(rule) => {
                self.source.push('@');
                self.source.push_str(match &rule.vendor_prefix {
                    VendorPrefix::None => "",
                    VendorPrefix::Webkit => "-webkit-",
                    VendorPrefix::Moz => "-moz-",
                    VendorPrefix::Ms => "-ms-",
                    VendorPrefix::O => "-o-",
                    VendorPrefix::Other(prefix) => prefix,
                });
                self.source.push_str("viewport");
                self.source.push('{');
                if let Some(declarations) = rule.declarations {
                    self.print_declaration_block_id(declarations);
                }
                self.source.push('}');
            }
            Rule::ViewTransition(rule) => {
                self.print_declaration_rule("@view-transition", rule.declarations)
            }
            Rule::Nesting(rule) => {
                self.print_style_like_rule(rule.prelude, rule.declarations, &rule.rules)
            }
            Rule::NestedDeclarations(rule) => self.print_nested_declarations_rule(rule),
            Rule::Keyframe(rule) => self.print_keyframe_rule(rule),
            Rule::Ignored(_) => {}
        }
    }

    /// Print one nested rule list.
    pub(crate) fn print_rule_list(&mut self, rules: &[LocalNodeId<Rule>]) {
        for (index, rule_id) in rules.iter().enumerate() {
            if index > 0 {
                self.source.push(';');
            }

            self.print_rule_id(*rule_id);
        }
    }

    /// Print one generic group rule.
    fn print_group_rule(&mut self, prefix: &str, prelude: &str, rules: &[LocalNodeId<Rule>]) {
        self.source.push_str(prefix);

        if !prelude.is_empty() {
            if !prefix.ends_with(' ') {
                self.source.push(' ');
            }

            self.source.push_str(prelude);
        }

        self.source.push('{');
        self.print_rule_list(rules);
        self.source.push('}');
    }

    /// Print one component-value-backed rule.
    fn print_component_value_rule(&mut self, components: &ComponentValueList) {
        self.source
            .push_str(&self.render_component_value_list(components));
    }

    /// Print one unknown at-rule.
    fn print_unknown_rule(&mut self, rule: &UnknownRule) {
        self.source.push('@');
        self.source.push_str(&rule.name);

        if !rule.prelude.values.is_empty() {
            self.source.push(' ');
            self.source
                .push_str(&self.render_component_value_list(&rule.prelude));
        }

        if let Some(block) = &rule.block {
            self.source.push('{');
            self.source
                .push_str(&self.render_component_value_list(block));
            self.source.push('}');
        } else {
            self.source.push(';');
        }
    }

    /// Print one import rule.
    fn print_import_rule(&mut self, rule: &ImportRule) {
        self.source.push_str("@import ");
        self.source.push('"');
        self.source.push_str(&rule.url);
        self.source.push('"');

        if let Some(layer) = &rule.layer {
            self.source.push(' ');
            self.source.push_str(&Self::render_import_layer(layer));
        }

        if let Some(supports) = &rule.supports {
            self.source.push(' ');
            self.source.push_str("supports ");
            self.source
                .push_str(&self.render_supports_condition(*supports));
        }

        if let Some(media) = &rule.media {
            self.source.push(' ');
            self.source.push_str(&self.render_media_query_list(*media));
        }

        self.source.push(';');
    }

    /// Print one style rule.
    fn print_style_rule(&mut self, rule: &StyleRule) {
        self.print_style_like_rule(rule.prelude, rule.declarations, &rule.rules);
    }

    /// Print one style-like rule.
    fn print_style_like_rule(
        &mut self,
        prelude: LocalNodeId<SelectorList>,
        declarations: Option<LocalNodeId<DeclarationBlock>>,
        rules: &[LocalNodeId<Rule>],
    ) {
        self.source.push_str(&self.render_selector_list(prelude));
        self.source.push('{');

        if let Some(declarations) = declarations {
            self.print_declaration_block_id(declarations);
        }

        if !rules.is_empty() {
            if declarations.is_some() {
                self.source.push(';');
            }

            self.print_rule_list(rules);
        }

        self.source.push('}');
    }

    /// Print one nested declarations rule.
    fn print_nested_declarations_rule(&mut self, rule: &NestedDeclarationsRule) {
        if let Some(declarations) = rule.declarations {
            self.print_declaration_block_id(declarations);
        }
    }

    /// Print one container rule.
    fn print_container_rule(&mut self, rule: &ContainerRule) {
        self.source.push_str("@container");

        if let Some(name) = &rule.name {
            self.source.push(' ');
            self.source.push_str(Self::render_container_name(name));
        }

        if let Some(condition) = &rule.condition {
            self.source.push(' ');
            self.source
                .push_str(&self.render_container_condition(*condition));
        }

        self.source.push('{');
        self.print_rule_list(&rule.rules);
        self.source.push('}');
    }

    /// Print one scope rule.
    fn print_scope_rule(&mut self, rule: &ScopeRule) {
        self.source.push_str("@scope");

        if let Some(scope_start) = &rule.scope_start {
            self.source.push(' ');
            self.source.push('(');
            self.source
                .push_str(&self.render_selector_list(*scope_start));
            self.source.push(')');
        }

        if let Some(scope_end) = &rule.scope_end {
            self.source.push_str(" to (");
            self.source.push_str(&self.render_selector_list(*scope_end));
            self.source.push(')');
        }

        self.source.push('{');
        self.print_rule_list(&rule.rules);
        self.source.push('}');
    }

    /// Print one layer statement rule.
    fn print_layer_statement_rule(&mut self, rule: &LayerStatementRule) {
        self.source.push_str("@layer ");

        for (index, name) in rule.names.iter().enumerate() {
            if index > 0 {
                self.source.push_str(", ");
            }

            self.source.push_str(&Self::render_layer_name_list(name));
        }

        self.source.push(';');
    }

    /// Print one property rule.
    fn print_property_rule(&mut self, rule: &PropertyRule) {
        self.source.push_str("@property ");
        self.source.push_str(&rule.name.name);
        self.source.push('{');
        self.source.push_str("syntax: ");
        self.print_property_syntax(&rule.syntax);
        self.source.push(';');
        self.source.push_str(" inherits: ");
        self.source
            .push_str(if rule.inherits { "true" } else { "false" });

        if let Some(initial_value) = &rule.initial_value {
            self.source.push(';');
            self.source.push_str(" initial-value: ");
            self.source
                .push_str(&self.render_component_value_list(initial_value.components()));
        }

        self.source.push(';');
        self.source.push('}');
    }

    /// Print one property syntax definition.
    fn print_property_syntax(&mut self, syntax: &PropertySyntax) {
        match syntax {
            PropertySyntax::Universal => self.source.push('*'),
            PropertySyntax::Components(components) => {
                for (index, component) in components.iter().enumerate() {
                    if index > 0 {
                        self.source.push_str(" | ");
                    }

                    self.print_property_syntax_component(component);
                }
            }
        }
    }

    /// Print one property syntax component.
    fn print_property_syntax_component(&mut self, component: &PropertySyntaxComponent) {
        self.print_property_syntax_component_kind(&component.kind);
        self.print_property_syntax_multiplier(component.multiplier);
    }

    /// Print one property syntax component kind.
    fn print_property_syntax_component_kind(&mut self, kind: &PropertySyntaxComponentKind) {
        match kind {
            PropertySyntaxComponentKind::Length => self.source.push_str("<length>"),
            PropertySyntaxComponentKind::Number => self.source.push_str("<number>"),
            PropertySyntaxComponentKind::Percentage => self.source.push_str("<percentage>"),
            PropertySyntaxComponentKind::LengthPercentage => {
                self.source.push_str("<length-percentage>")
            }
            PropertySyntaxComponentKind::String => self.source.push_str("<string>"),
            PropertySyntaxComponentKind::Color => self.source.push_str("<color>"),
            PropertySyntaxComponentKind::Image => self.source.push_str("<image>"),
            PropertySyntaxComponentKind::Url => self.source.push_str("<url>"),
            PropertySyntaxComponentKind::Integer => self.source.push_str("<integer>"),
            PropertySyntaxComponentKind::Angle => self.source.push_str("<angle>"),
            PropertySyntaxComponentKind::Time => self.source.push_str("<time>"),
            PropertySyntaxComponentKind::Resolution => self.source.push_str("<resolution>"),
            PropertySyntaxComponentKind::TransformFunction => {
                self.source.push_str("<transform-function>")
            }
            PropertySyntaxComponentKind::TransformList => self.source.push_str("<transform-list>"),
            PropertySyntaxComponentKind::CustomIdent => self.source.push_str("<custom-ident>"),
            PropertySyntaxComponentKind::Literal(value) => self.source.push_str(value),
        }
    }

    /// Print one property syntax multiplier.
    fn print_property_syntax_multiplier(&mut self, multiplier: PropertySyntaxMultiplier) {
        match multiplier {
            PropertySyntaxMultiplier::None => {}
            PropertySyntaxMultiplier::Space => self.source.push('+'),
            PropertySyntaxMultiplier::Comma => self.source.push('#'),
        }
    }

    /// Print one namespace rule.
    fn print_namespace_rule(&mut self, rule: &NamespaceRule) {
        self.source.push_str("@namespace ");

        if let Some(prefix) = &rule.prefix {
            self.source.push_str(&prefix.name);
            self.source.push(' ');
        }

        match &rule.url {
            NamespaceUrl::String(value) => {
                self.source.push('"');
                self.source.push_str(value);
                self.source.push('"');
            }
            NamespaceUrl::Url(value) => {
                self.source.push_str("url(\"");
                self.source.push_str(value);
                self.source.push_str("\")");
            }
        }
        self.source.push(';');
    }

    /// Print one keyframe rule.
    fn print_keyframe_rule(&mut self, rule: &KeyframeRule) {
        self.source
            .push_str(&Self::render_keyframe_selector_list(&rule.selectors));
        self.source.push('{');

        if let Some(declarations) = rule.declarations {
            self.print_declaration_block_id(declarations);
        }

        self.source.push('}');
    }

    /// Print one page rule.
    fn print_page_rule(&mut self, rule: &PageRule) {
        self.source.push_str("@page");

        let selectors = Self::render_page_selector_list(&rule.selectors);
        if !selectors.is_empty() {
            self.source.push(' ');
            self.source.push_str(&selectors);
        }

        self.source.push('{');

        let mut needs_separator = false;

        if let Some(declarations) = rule.declarations {
            self.print_declaration_block_id(declarations);
            needs_separator = true;
        }

        for page_margin_rule in &rule.page_margin_rules {
            if needs_separator {
                self.source.push(';');
            }

            self.print_page_margin_rule(*page_margin_rule);
            needs_separator = true;
        }

        self.source.push('}');
    }

    /// Print one page margin rule.
    fn print_page_margin_rule(&mut self, page_margin_rule_id: LocalNodeId<PageMarginRule>) {
        let rule = self.tree.get(page_margin_rule_id);

        self.source.push('@');
        self.source
            .push_str(Self::render_page_margin_box(rule.margin_box));
        self.source.push('{');

        if let Some(declarations) = rule.declarations {
            self.print_declaration_block_id(declarations);
        }

        self.source.push('}');
    }

    /// Print one named declaration rule.
    fn print_named_declaration_rule(
        &mut self,
        prefix: &str,
        name: &str,
        declarations: Option<LocalNodeId<DeclarationBlock>>,
    ) {
        self.source.push_str(prefix);
        self.source.push_str(name);
        self.source.push('{');

        if let Some(declarations) = declarations {
            self.print_declaration_block_id(declarations);
        }

        self.source.push('}');
    }

    /// Print one declaration rule without a name.
    fn print_declaration_rule(
        &mut self,
        prefix: &str,
        declarations: Option<LocalNodeId<DeclarationBlock>>,
    ) {
        self.source.push_str(prefix);
        self.source.push('{');

        if let Some(declarations) = declarations {
            self.print_declaration_block_id(declarations);
        }

        self.source.push('}');
    }

    /// Print one font feature values rule.
    fn print_font_feature_values_rule(&mut self, rule: &FontFeatureValuesRule) {
        self.source.push_str("@font-feature-values ");

        for (index, family) in rule.families.iter().enumerate() {
            if index > 0 {
                self.source.push_str(", ");
            }

            self.source.push_str(&family.name);
        }

        self.source.push('{');

        for subrule in &rule.subrules {
            self.source.push('@');
            self.source.push_str(match subrule.kind {
                FontFeatureSubruleKind::Stylistic => "stylistic",
                FontFeatureSubruleKind::HistoricalForms => "historical-forms",
                FontFeatureSubruleKind::Styleset => "styleset",
                FontFeatureSubruleKind::CharacterVariant => "character-variant",
                FontFeatureSubruleKind::Swash => "swash",
                FontFeatureSubruleKind::Ornaments => "ornaments",
                FontFeatureSubruleKind::Annotation => "annotation",
            });
            self.source.push('{');

            for (index, declaration) in subrule.declarations.iter().enumerate() {
                if index > 0 {
                    self.source.push(';');
                }

                self.source.push_str(&declaration.name);
                self.source.push(':');

                for (value_index, value) in declaration.values.iter().enumerate() {
                    if value_index > 0 {
                        self.source.push(' ');
                    }

                    self.source.push_str(&value.to_string());
                }
            }

            self.source.push('}');
        }

        self.source.push('}');
    }
}
