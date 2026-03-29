use crate::{
    AnySelector, AttributeSelector, Combinator, ComponentValue, ComponentValueList, ContainerName,
    ContainerRule, CounterStyleName, CounterStyleRule, CustomMediaName, CustomMediaRule,
    CustomPropertyName, CustomRule, Declaration, DeclarationBlock, DeclarationValue, FontFaceRule,
    FontFeatureDeclaration, FontFeatureFamilyName, FontFeatureSubrule, FontFeatureSubruleKind,
    FontFeatureValuesRule, FontPaletteName, FontPaletteValuesRule, IgnoredRule, ImportLayer,
    ImportRule, KeyframeRule, KeyframeSelector, KeyframeSelectorList, KeyframesName, KeyframesRule,
    LayerBlockRule, LayerNameList, LayerStatementRule, LocalName, LocalNodeId, MediaRule,
    MozDocumentRule, NamespacePrefix, NamespaceRule, NamespaceUrl, NestedDeclarationsRule,
    NestingRule, Node, NodeSpanType, NodeTree, NodeTreeImpl, NthOfSelector, NthSelector,
    NthSelectorKind, Number, PageMarginBox, PageMarginRule, PagePseudoClass, PageRule,
    PageSelector, PageSelectorList, PropertyName, PropertyRule, PropertySyntax,
    PropertySyntaxComponent, PropertySyntaxComponentKind, PropertySyntaxMultiplier, PseudoArgument,
    PseudoClass, PseudoElement, Rule, ScopeRule, Selector, SelectorComponent, SelectorList,
    SimpleSelector, StartingStyleRule, StyleRule, StyleSheet, SupportsRule, Symbol,
    TimelineRangeName, TimelineRangePercentage, Token, UnknownRule, VendorPrefix,
    ViewTransitionRule, ViewportRule,
};
use destack_source::{File, Span};

use super::core::{
    import_url_span_for_rule, prelude_span_for_rule, serialize_rule, serialize_value, span_for_rule,
};
use super::parse::Parser;
use super::{lightning, parcel};

/// One CSS AST lowerer.
pub(crate) struct Lowerer<'a> {
    /// The authored source file.
    file: &'a File,
    /// The authored source text.
    source: &'a str,
    /// The output CSS tree.
    tree: NodeTree,
}

impl<'a> Lowerer<'a> {
    /// Create one CSS AST lowerer.
    pub(crate) fn new(file: &'a File, source: &'a str) -> Self {
        Self {
            file,
            source,
            tree: NodeTree::new(),
        }
    }

    /// Lower one parsed Lightning stylesheet into the Destack CSS tree.
    pub(crate) fn lower_stylesheet<'o>(
        mut self,
        stylesheet: lightning::StyleSheet<'a, 'o>,
    ) -> (NodeTree, LocalNodeId<StyleSheet>) {
        let root_span = Span::new(self.file.id, 0, self.source.len() as u32);
        let rules = stylesheet
            .rules
            .0
            .into_iter()
            .map(|rule| self.lower_rule(rule))
            .collect();
        let stylesheet = self.tree.insert(
            StyleSheet {
                sources: stylesheet.sources,
                license_comments: stylesheet
                    .license_comments
                    .into_iter()
                    .map(|comment| comment.to_string())
                    .collect(),
                rules,
            },
            root_span,
        );

        (self.tree, stylesheet)
    }

    /// Return one default span for inner CSS syntax nodes.
    fn inner_span(&self) -> Span {
        Span::empty(self.file.id)
    }

    /// Insert one inner CSS node.
    pub(crate) fn insert_inner<T>(&mut self, node: T) -> LocalNodeId<T>
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.tree.insert(node, self.inner_span())
    }

    /// Lower one Lightning CSS rule into the CSS tree.
    fn lower_rule(&mut self, rule: lightning::CssRule<'a>) -> LocalNodeId<Rule> {
        let span = self.span_for_rule(&rule);
        let node = match rule {
            lightning::CssRule::Import(rule) => Rule::Import(self.lower_import_rule(rule)),
            lightning::CssRule::Style(rule) => Rule::Style(StyleRule {
                prelude: self.lower_selector_list(&rule.selectors),
                declarations: self.lower_style_declaration_block(&rule, span),
                rules: self.lower_rule_list(rule.rules.0),
            }),
            lightning::CssRule::Media(rule) => Rule::Media(MediaRule {
                query: self.lower_media_query_list(&rule.query),
                rules: self.lower_rule_list(rule.rules.0),
            }),
            lightning::CssRule::Supports(rule) => Rule::Supports(SupportsRule {
                condition: self.lower_supports_condition(&rule.condition),
                rules: self.lower_rule_list(rule.rules.0),
            }),
            lightning::CssRule::MozDocument(rule) => Rule::MozDocument(MozDocumentRule {
                rules: self.lower_rule_list(rule.rules.0),
            }),
            lightning::CssRule::LayerBlock(rule) => Rule::LayerBlock(LayerBlockRule {
                name: rule.name.as_ref().map(|name| self.lower_layer_name(name)),
                rules: self.lower_rule_list(rule.rules.0),
            }),
            lightning::CssRule::Container(rule) => Rule::Container(ContainerRule {
                name: rule
                    .name
                    .as_ref()
                    .map(|name| self.lower_container_name(name)),
                condition: rule
                    .condition
                    .as_ref()
                    .map(|condition| self.lower_container_condition(condition)),
                rules: self.lower_rule_list(rule.rules.0),
            }),
            lightning::CssRule::Scope(rule) => Rule::Scope(ScopeRule {
                scope_start: rule
                    .scope_start
                    .as_ref()
                    .map(|selectors| self.lower_selector_list(selectors)),
                scope_end: rule
                    .scope_end
                    .as_ref()
                    .map(|selectors| self.lower_selector_list(selectors)),
                rules: self.lower_rule_list(rule.rules.0),
            }),
            lightning::CssRule::StartingStyle(rule) => Rule::StartingStyle(StartingStyleRule {
                rules: self.lower_rule_list(rule.rules.0),
            }),
            lightning::CssRule::Page(rule) => Rule::Page(PageRule {
                selectors: self.lower_page_selector_list(&rule.selectors),
                declarations: self.lower_declaration_block(&rule.declarations, span),
                page_margin_rules: self.lower_page_margin_rules(rule.rules, span),
            }),
            lightning::CssRule::FontFace(rule) => Rule::FontFace(FontFaceRule {
                declarations: self.lower_font_face_declaration_block(&rule.properties, span),
            }),
            lightning::CssRule::FontPaletteValues(rule) => {
                Rule::FontPaletteValues(FontPaletteValuesRule {
                    name: self.lower_font_palette_name(rule.name.as_ref()),
                    declarations: self
                        .lower_font_palette_values_declaration_block(&rule.properties, span),
                })
            }
            lightning::CssRule::FontFeatureValues(rule) => {
                Rule::FontFeatureValues(FontFeatureValuesRule {
                    families: rule
                        .name
                        .iter()
                        .map(|family| self.lower_font_feature_family_name(family))
                        .collect(),
                    subrules: rule
                        .rules
                        .into_values()
                        .map(|subrule| FontFeatureSubrule {
                            kind: match subrule.name {
                                lightning::FontFeatureSubruleType::Stylistic => {
                                    FontFeatureSubruleKind::Stylistic
                                }
                                lightning::FontFeatureSubruleType::HistoricalForms => {
                                    FontFeatureSubruleKind::HistoricalForms
                                }
                                lightning::FontFeatureSubruleType::Styleset => {
                                    FontFeatureSubruleKind::Styleset
                                }
                                lightning::FontFeatureSubruleType::CharacterVariant => {
                                    FontFeatureSubruleKind::CharacterVariant
                                }
                                lightning::FontFeatureSubruleType::Swash => {
                                    FontFeatureSubruleKind::Swash
                                }
                                lightning::FontFeatureSubruleType::Ornaments => {
                                    FontFeatureSubruleKind::Ornaments
                                }
                                lightning::FontFeatureSubruleType::Annotation => {
                                    FontFeatureSubruleKind::Annotation
                                }
                            },
                            declarations: subrule
                                .declarations
                                .into_iter()
                                .map(|(name, values)| FontFeatureDeclaration {
                                    name: self.serialize_value(&name),
                                    values: values.into_iter().collect(),
                                })
                                .collect(),
                        })
                        .collect(),
                })
            }
            lightning::CssRule::CounterStyle(rule) => Rule::CounterStyle(CounterStyleRule {
                name: self.lower_counter_style_name(rule.name.as_ref()),
                declarations: self.lower_declaration_block(&rule.declarations, span),
            }),
            lightning::CssRule::Namespace(rule) => Rule::Namespace(NamespaceRule {
                prefix: rule
                    .prefix
                    .as_ref()
                    .map(|prefix| self.lower_namespace_prefix(prefix.as_ref())),
                url: NamespaceUrl::String(rule.url.as_ref().to_string()),
            }),
            lightning::CssRule::Nesting(rule) => Rule::Nesting(NestingRule {
                prelude: self.lower_selector_list(&rule.style.selectors),
                declarations: self.lower_style_declaration_block(&rule.style, span),
                rules: self.lower_rule_list(rule.style.rules.0),
            }),
            lightning::CssRule::NestedDeclarations(rule) => {
                Rule::NestedDeclarations(NestedDeclarationsRule {
                    declarations: self.lower_declaration_block(&rule.declarations, span),
                })
            }
            lightning::CssRule::Viewport(rule) => Rule::Viewport(ViewportRule {
                vendor_prefix: self.lower_vendor_prefix(rule.vendor_prefix),
                declarations: self.lower_declaration_block(&rule.declarations, span),
            }),
            lightning::CssRule::CustomMedia(rule) => Rule::CustomMedia(CustomMediaRule {
                name: self.lower_custom_media_name(rule.name.as_ref()),
                query: self.lower_media_query_list(&rule.query),
            }),
            lightning::CssRule::LayerStatement(rule) => Rule::LayerStatement(LayerStatementRule {
                names: rule
                    .names
                    .iter()
                    .map(|name| self.lower_layer_name(name))
                    .collect(),
            }),
            lightning::CssRule::Property(rule) => Rule::Property(PropertyRule {
                name: self.lower_custom_property_name(rule.name.as_ref()),
                syntax: self.lower_property_syntax(&rule.syntax),
                inherits: rule.inherits,
                initial_value: rule
                    .initial_value
                    .as_ref()
                    .map(|value| self.lower_declaration_value_source(&self.serialize_value(value))),
            }),
            lightning::CssRule::Keyframes(rule) => Rule::Keyframes(KeyframesRule {
                name: self.lower_keyframes_name(&rule.name),
                vendor_prefix: self.lower_vendor_prefix(rule.vendor_prefix),
                rules: self.lower_keyframe_rules(rule.keyframes, span),
            }),
            lightning::CssRule::ViewTransition(rule) => Rule::ViewTransition(ViewTransitionRule {
                declarations: self.lower_view_transition_declaration_block(&rule.properties, span),
            }),
            lightning::CssRule::Unknown(rule) => Rule::Unknown(UnknownRule {
                name: rule.name.to_string(),
                prelude: self.lower_component_value_token_list(&rule.prelude),
                block: rule
                    .block
                    .as_ref()
                    .map(|block| self.lower_component_value_token_list(block)),
            }),
            lightning::CssRule::Custom(rule) => Rule::Custom(CustomRule {
                components: self.lower_component_value_list_source(
                    &self.serialize_rule(&lightning::CssRule::Custom(rule)),
                ),
            }),
            lightning::CssRule::Ignored => Rule::Ignored(IgnoredRule {}),
        };
        let prelude_span = prelude_span_for_rule(self.source, span);
        let import_url_span = match &node {
            Rule::Import(import_rule) => {
                import_url_span_for_rule(self.source, span, &import_rule.url)
            }
            Rule::Style(_)
            | Rule::Media(_)
            | Rule::Supports(_)
            | Rule::MozDocument(_)
            | Rule::LayerBlock(_)
            | Rule::Container(_)
            | Rule::Scope(_)
            | Rule::StartingStyle(_)
            | Rule::Page(_)
            | Rule::FontFace(_)
            | Rule::FontPaletteValues(_)
            | Rule::FontFeatureValues(_)
            | Rule::CounterStyle(_)
            | Rule::Namespace(_)
            | Rule::Nesting(_)
            | Rule::NestedDeclarations(_)
            | Rule::Viewport(_)
            | Rule::CustomMedia(_)
            | Rule::LayerStatement(_)
            | Rule::Property(_)
            | Rule::Keyframes(_)
            | Rule::Keyframe(_)
            | Rule::ViewTransition(_)
            | Rule::Unknown(_)
            | Rule::Custom(_)
            | Rule::Ignored(_) => None,
        };
        let rule_id = self.tree.insert(node, span);

        if let Some(prelude_span) = prelude_span {
            self.tree
                .set_side_span(rule_id, NodeSpanType::Segment(0), prelude_span);
        }

        if let Some(import_url_span) = import_url_span {
            self.tree
                .set_side_span(rule_id, NodeSpanType::Segment(1), import_url_span);
        }

        rule_id
    }

    /// Lower one nested CSS rule list.
    fn lower_rule_list(&mut self, rules: Vec<lightning::CssRule<'a>>) -> Vec<LocalNodeId<Rule>> {
        rules
            .into_iter()
            .map(|rule| self.lower_rule(rule))
            .collect()
    }

    /// Lower one keyframe rule list.
    fn lower_keyframe_rules(
        &mut self,
        rules: Vec<lightning::Keyframe<'a>>,
        span: Span,
    ) -> Vec<LocalNodeId<Rule>> {
        rules
            .into_iter()
            .map(|rule| {
                let declarations = self.lower_declaration_block(&rule.declarations, span);

                self.tree.insert(
                    Rule::Keyframe(KeyframeRule {
                        selectors: self.lower_keyframe_selector_list(&rule.selectors),
                        declarations,
                    }),
                    span,
                )
            })
            .collect()
    }

    /// Lower one nested page margin rule list.
    fn lower_page_margin_rules(
        &mut self,
        rules: Vec<lightning::PageMarginRule<'a>>,
        span: Span,
    ) -> Vec<LocalNodeId<PageMarginRule>> {
        rules
            .into_iter()
            .map(|rule| {
                let declarations = self.lower_declaration_block(&rule.declarations, span);

                self.tree.insert(
                    PageMarginRule {
                        margin_box: self.lower_page_margin_box(rule.margin_box),
                        declarations,
                    },
                    span,
                )
            })
            .collect()
    }

    /// Lower one style rule declaration block from authored source order.
    fn lower_style_declaration_block<R>(
        &mut self,
        rule: &lightning::StyleRule<'a, R>,
        span: Span,
    ) -> Option<LocalNodeId<DeclarationBlock>> {
        let mut declarations = Vec::with_capacity(rule.declarations.len());

        // authored source order
        for index in 0..rule.declarations.len() {
            let Some(declaration) = self.lower_style_declaration(rule, index, span) else {
                return self.lower_declaration_block(&rule.declarations, span);
            };

            declarations.push(declaration);
        }

        if declarations.is_empty() {
            return None;
        }

        Some(self.tree.insert(DeclarationBlock { declarations }, span))
    }

    /// Lower one authored style declaration by source index.
    fn lower_style_declaration<R>(
        &mut self,
        rule: &lightning::StyleRule<'a, R>,
        index: usize,
        fallback_span: Span,
    ) -> Option<LocalNodeId<Declaration>> {
        let (name_range, value_range) = rule.property_location(self.source, index).ok()?;
        let name_span = self.span_from_source_positions(
            name_range.start.line as usize,
            name_range.start.column as usize,
            name_range.end.line as usize,
            name_range.end.column as usize,
        );
        let value_span = self.span_from_source_positions(
            value_range.start.line as usize,
            value_range.start.column as usize,
            value_range.end.line as usize,
            value_range.end.column as usize,
        );
        let declaration_span = name_span.merge(value_span);
        let name = self.source_slice(name_span);
        let value = self.source_slice(value_span);
        let (value, is_important) = self.lower_declaration_value_authored_source(value);
        let declaration = Declaration {
            name: self.lower_property_name_source(name),
            value,
            is_important,
        };
        let declaration_id = self.tree.insert(
            declaration,
            if declaration_span.is_empty() {
                fallback_span
            } else {
                declaration_span
            },
        );

        self.tree
            .set_side_span(declaration_id, NodeSpanType::Segment(0), name_span);
        self.tree
            .set_side_span(declaration_id, NodeSpanType::Segment(1), value_span);

        Some(declaration_id)
    }

    /// Lower one declaration block into the CSS tree.
    fn lower_declaration_block(
        &mut self,
        declarations: &lightning::DeclarationBlock<'a>,
        span: Span,
    ) -> Option<LocalNodeId<DeclarationBlock>> {
        let declarations = declarations
            .iter()
            .map(|(property, is_important)| {
                self.tree
                    .insert(self.lower_declaration(property, is_important), span)
            })
            .collect::<Vec<_>>();

        if declarations.is_empty() {
            return None;
        }

        Some(self.tree.insert(DeclarationBlock { declarations }, span))
    }

    /// Lower one direct declaration list into the CSS tree.
    fn lower_declaration_list(
        &mut self,
        declarations: Vec<Declaration>,
        span: Span,
    ) -> Option<LocalNodeId<DeclarationBlock>> {
        let declarations = declarations
            .into_iter()
            .map(|declaration| self.tree.insert(declaration, span))
            .collect::<Vec<_>>();

        if declarations.is_empty() {
            return None;
        }

        Some(self.tree.insert(DeclarationBlock { declarations }, span))
    }

    /// Lower one import rule.
    fn lower_import_rule(&mut self, rule: lightning::ImportRule<'a>) -> ImportRule {
        ImportRule {
            url: rule.url.to_string(),
            layer: rule.layer.map(|layer| ImportLayer {
                name: layer.map(|name| self.lower_layer_name(&name)),
            }),
            supports: rule
                .supports
                .as_ref()
                .map(|condition| self.lower_supports_condition(condition)),
            media: (!rule.media.media_queries.is_empty())
                .then(|| self.lower_media_query_list(&rule.media)),
        }
    }

    /// Lower one declaration.
    fn lower_declaration(
        &self,
        property: &lightning::Property<'a>,
        is_important: bool,
    ) -> Declaration {
        Declaration {
            name: self.lower_property_name_source(property.property_id().name()),
            value: self.lower_declaration_value_property(property),
            is_important,
        }
    }

    /// Lower one `@font-face` property list into one declaration block.
    fn lower_font_face_declaration_block(
        &mut self,
        properties: &[lightning::FontFaceProperty<'a>],
        span: Span,
    ) -> Option<LocalNodeId<DeclarationBlock>> {
        let declarations = properties
            .iter()
            .map(|property| self.lower_font_face_property_declaration(property))
            .collect();

        self.lower_declaration_list(declarations, span)
    }

    /// Lower one `@font-face` property into one declaration.
    fn lower_font_face_property_declaration(
        &self,
        property: &lightning::FontFaceProperty<'a>,
    ) -> Declaration {
        match property {
            lightning::FontFaceProperty::Source(value) => {
                self.lower_raw_declaration("src", self.lower_css_list(value))
            }
            lightning::FontFaceProperty::FontFamily(value) => {
                self.lower_raw_declaration("font-family", self.lower_css_value(value))
            }
            lightning::FontFaceProperty::FontStyle(value) => {
                self.lower_raw_declaration("font-style", self.lower_css_value(value))
            }
            lightning::FontFaceProperty::FontWeight(value) => {
                self.lower_raw_declaration("font-weight", self.lower_css_value(value))
            }
            lightning::FontFaceProperty::FontStretch(value) => {
                self.lower_raw_declaration("font-stretch", self.lower_css_value(value))
            }
            lightning::FontFaceProperty::UnicodeRange(value) => {
                self.lower_raw_declaration("unicode-range", self.lower_css_list(value))
            }
            lightning::FontFaceProperty::Custom(custom) => self.lower_custom_property_declaration(
                &custom.name,
                self.lower_component_value_token_list(&custom.value),
            ),
        }
    }

    /// Lower one `@font-palette-values` property list into one declaration block.
    fn lower_font_palette_values_declaration_block(
        &mut self,
        properties: &[lightning::FontPaletteValuesProperty<'a>],
        span: Span,
    ) -> Option<LocalNodeId<DeclarationBlock>> {
        let declarations = properties
            .iter()
            .map(|property| self.lower_font_palette_values_property_declaration(property))
            .collect();

        self.lower_declaration_list(declarations, span)
    }

    /// Lower one `@font-palette-values` property into one declaration.
    fn lower_font_palette_values_property_declaration(
        &self,
        property: &lightning::FontPaletteValuesProperty<'a>,
    ) -> Declaration {
        match property {
            lightning::FontPaletteValuesProperty::FontFamily(value) => {
                self.lower_raw_declaration("font-family", self.lower_css_value(value))
            }
            lightning::FontPaletteValuesProperty::BasePalette(value) => {
                self.lower_raw_declaration("base-palette", self.lower_css_value(value))
            }
            lightning::FontPaletteValuesProperty::OverrideColors(value) => {
                self.lower_raw_declaration("override-colors", self.lower_css_list(value))
            }
            lightning::FontPaletteValuesProperty::Custom(custom) => self
                .lower_custom_property_declaration(
                    &custom.name,
                    self.lower_component_value_token_list(&custom.value),
                ),
        }
    }

    /// Lower one `@view-transition` property list into one declaration block.
    fn lower_view_transition_declaration_block(
        &mut self,
        properties: &[lightning::ViewTransitionProperty<'a>],
        span: Span,
    ) -> Option<LocalNodeId<DeclarationBlock>> {
        let declarations = properties
            .iter()
            .map(|property| self.lower_view_transition_property_declaration(property))
            .collect();

        self.lower_declaration_list(declarations, span)
    }

    /// Lower one `@view-transition` property into one declaration.
    fn lower_view_transition_property_declaration(
        &self,
        property: &lightning::ViewTransitionProperty<'a>,
    ) -> Declaration {
        match property {
            lightning::ViewTransitionProperty::Navigation(value) => {
                self.lower_raw_declaration("navigation", self.lower_css_value(value))
            }
            lightning::ViewTransitionProperty::Types(value) => {
                self.lower_raw_declaration("types", self.lower_css_value(value))
            }
            lightning::ViewTransitionProperty::Custom(custom) => self
                .lower_custom_property_declaration(
                    &custom.name,
                    self.lower_component_value_token_list(&custom.value),
                ),
        }
    }

    /// Lower one custom property declaration.
    fn lower_custom_property_declaration(
        &self,
        name: &lightning::CustomPropertyName<'a>,
        value: ComponentValueList,
    ) -> Declaration {
        Declaration {
            name: self.lower_property_name_source(name.as_ref()),
            value: DeclarationValue { components: value },
            is_important: false,
        }
    }

    /// Lower one raw declaration from one property name and value.
    fn lower_raw_declaration(&self, name: &str, value: DeclarationValue) -> Declaration {
        Declaration {
            name: self.lower_property_name_source(name),
            value,
            is_important: false,
        }
    }

    /// Lower one printable CSS value into one declaration value.
    fn lower_css_value<T: lightning::ToCss>(&self, value: &T) -> DeclarationValue {
        DeclarationValue {
            components: self.lower_component_value_css_list(value),
        }
    }

    /// Lower one printable CSS list into one declaration value.
    fn lower_css_list<T: lightning::ToCss>(&self, values: &[T]) -> DeclarationValue {
        let mut components = Vec::new();

        for (index, value) in values.iter().enumerate() {
            if index > 0 {
                components.push(ComponentValue::Token(Token::Symbol(Symbol::Comma)));
                components.push(ComponentValue::Token(Token::WhiteSpace(" ".to_string())));
            }

            components.extend(self.lower_component_value_css_list(value).values);
        }

        DeclarationValue {
            components: ComponentValueList { values: components },
        }
    }

    /// Lower one Lightning selector list into one owned selector list.
    fn lower_selector_list(
        &mut self,
        selectors: &lightning::SelectorList<'_>,
    ) -> LocalNodeId<SelectorList> {
        let selectors = selectors
            .0
            .iter()
            .map(|selector| self.lower_selector(selector))
            .collect();

        self.insert_inner(SelectorList { selectors })
    }

    /// Lower one Lightning page selector list into one owned page selector list.
    fn lower_page_selector_list(
        &self,
        selectors: &[lightning::PageSelector<'_>],
    ) -> PageSelectorList {
        PageSelectorList {
            selectors: selectors
                .iter()
                .map(|selector| self.lower_page_selector(selector))
                .collect(),
        }
    }

    /// Lower one Lightning keyframe selector list into one owned keyframe selector list.
    fn lower_keyframe_selector_list(
        &self,
        selectors: &[lightning::KeyframeSelector],
    ) -> KeyframeSelectorList {
        KeyframeSelectorList {
            selectors: selectors
                .iter()
                .map(|selector| self.lower_keyframe_selector(selector))
                .collect(),
        }
    }

    /// Lower one Lightning selector into one owned selector.
    fn lower_selector(&mut self, selector: &lightning::Selector<'_>) -> LocalNodeId<Selector> {
        let components = selector
            .iter_raw_parse_order_from(0)
            .map(|component| {
                let component = self.lower_selector_component(component);

                self.insert_inner(component)
            })
            .collect();

        self.insert_inner(Selector { components })
    }

    /// Lower one Lightning selector component into one owned selector component.
    fn lower_selector_component(
        &mut self,
        component: &lightning::SelectorComponent<'_>,
    ) -> SelectorComponent {
        match component {
            lightning::SelectorComponent::Combinator(combinator) => {
                SelectorComponent::Combinator(self.lower_combinator(*combinator))
            }
            _ => {
                let simple = self.lower_simple_selector(component);
                let simple = self.insert_inner(simple);

                SelectorComponent::Simple(simple)
            }
        }
    }

    /// Lower one Lightning combinator into one owned combinator.
    fn lower_combinator(&self, combinator: lightning::Combinator) -> Combinator {
        match combinator {
            lightning::Combinator::Child => Combinator::Child,
            lightning::Combinator::Descendant => Combinator::Descendant,
            lightning::Combinator::NextSibling => Combinator::NextSibling,
            lightning::Combinator::LaterSibling => Combinator::LaterSibling,
            lightning::Combinator::PseudoElement => Combinator::PseudoElement,
            lightning::Combinator::SlotAssignment => Combinator::SlotAssignment,
            lightning::Combinator::Part => Combinator::Part,
            lightning::Combinator::DeepDescendant => Combinator::DeepDescendant,
            lightning::Combinator::Deep => Combinator::Deep,
        }
    }

    /// Lower one Lightning simple selector into one owned simple selector.
    fn lower_simple_selector(
        &mut self,
        component: &lightning::SelectorComponent<'_>,
    ) -> SimpleSelector {
        match component {
            lightning::SelectorComponent::ExplicitAnyNamespace => {
                SimpleSelector::ExplicitAnyNamespace
            }
            lightning::SelectorComponent::ExplicitNoNamespace => {
                SimpleSelector::ExplicitNoNamespace
            }
            lightning::SelectorComponent::DefaultNamespace(_) => SimpleSelector::DefaultNamespace,
            lightning::SelectorComponent::Namespace(prefix, _) => {
                SimpleSelector::Namespace(prefix.to_string())
            }
            lightning::SelectorComponent::ExplicitUniversalType => {
                SimpleSelector::ExplicitUniversalType
            }
            lightning::SelectorComponent::LocalName(name) => SimpleSelector::LocalName(LocalName {
                name: name.name.to_string(),
                lower_name: name.lower_name.to_string(),
            }),
            lightning::SelectorComponent::ID(value) => SimpleSelector::Id(value.to_string()),
            lightning::SelectorComponent::Class(value) => SimpleSelector::Class(value.to_string()),
            lightning::SelectorComponent::AttributeInNoNamespaceExists { .. }
            | lightning::SelectorComponent::AttributeInNoNamespace { .. }
            | lightning::SelectorComponent::AttributeOther(_) => {
                let selector = self.insert_inner(AttributeSelector {
                    components: self.lower_attribute_selector(component),
                });

                SimpleSelector::Attribute(selector)
            }
            lightning::SelectorComponent::Negation(selectors) => {
                SimpleSelector::Negation(self.lower_selector_slice(selectors))
            }
            lightning::SelectorComponent::Root => SimpleSelector::Root,
            lightning::SelectorComponent::Empty => SimpleSelector::Empty,
            lightning::SelectorComponent::Scope => SimpleSelector::Scope,
            lightning::SelectorComponent::Nth(data) => {
                let selector = self.insert_inner(self.lower_nth_selector(
                    data.ty,
                    data.a,
                    data.b,
                    data.is_function,
                ));

                SimpleSelector::Nth(selector)
            }
            lightning::SelectorComponent::NthOf(data) => {
                let nth = data.nth_data();
                let nth = self.insert_inner(self.lower_nth_selector(nth.ty, nth.a, nth.b, true));
                let selectors = self.lower_selector_slice(data.selectors());
                let selector = self.insert_inner(NthOfSelector { nth, selectors });

                SimpleSelector::NthOf(selector)
            }
            lightning::SelectorComponent::NonTSPseudoClass(pseudo) => {
                let selector = self.lower_pseudo_class(pseudo);
                let selector = self.insert_inner(selector);

                SimpleSelector::PseudoClass(selector)
            }
            lightning::SelectorComponent::Slotted(selector) => {
                SimpleSelector::Slotted(self.lower_selector(selector))
            }
            lightning::SelectorComponent::Part(parts) => {
                SimpleSelector::Part(parts.iter().map(|part| part.to_string()).collect())
            }
            lightning::SelectorComponent::Host(selector) => SimpleSelector::Host(
                selector
                    .as_ref()
                    .map(|selector| self.lower_selector(selector)),
            ),
            lightning::SelectorComponent::Where(selectors) => {
                SimpleSelector::Where(self.lower_selector_slice(selectors))
            }
            lightning::SelectorComponent::Is(selectors) => {
                SimpleSelector::Is(self.lower_selector_slice(selectors))
            }
            lightning::SelectorComponent::Any(prefix, selectors) => {
                let selectors = self.lower_selector_slice(selectors);
                let selector = self.insert_inner(AnySelector {
                    vendor_prefix: self.lower_vendor_prefix(*prefix),
                    selectors,
                });

                SimpleSelector::Any(selector)
            }
            lightning::SelectorComponent::Has(selectors) => {
                SimpleSelector::Has(self.lower_selector_slice(selectors))
            }
            lightning::SelectorComponent::PseudoElement(pseudo) => {
                let selector = self.lower_pseudo_element(pseudo);
                let selector = self.insert_inner(selector);

                SimpleSelector::PseudoElement(selector)
            }
            lightning::SelectorComponent::Nesting => SimpleSelector::Nesting,
            lightning::SelectorComponent::Combinator(_) => unreachable!(),
        }
    }

    /// Lower one Lightning selector slice into one owned selector list.
    fn lower_selector_slice(
        &mut self,
        selectors: &[lightning::Selector<'_>],
    ) -> LocalNodeId<SelectorList> {
        let selectors = selectors
            .iter()
            .map(|selector| self.lower_selector(selector))
            .collect();

        self.insert_inner(SelectorList { selectors })
    }

    /// Lower one canonical selector list source into one owned selector list.
    pub(crate) fn lower_selector_list_source(&mut self, source: &str) -> LocalNodeId<SelectorList> {
        let selectors = Parser::parse_selector_list_source(source);

        self.lower_selector_list(&selectors)
    }

    /// Lower one Lightning keyframe selector into one owned keyframe selector.
    fn lower_keyframe_selector(&self, selector: &lightning::KeyframeSelector) -> KeyframeSelector {
        match selector {
            lightning::KeyframeSelector::Percentage(value) => {
                KeyframeSelector::Percentage(self.lower_percentage_number(value.0))
            }
            lightning::KeyframeSelector::From => KeyframeSelector::From,
            lightning::KeyframeSelector::To => KeyframeSelector::To,
            lightning::KeyframeSelector::TimelineRangePercentage(_) => {
                KeyframeSelector::TimelineRangePercentage(
                    self.lower_timeline_range_percentage_source(
                        &lightning::ToCss::to_css_string(selector, Default::default())
                            .unwrap_or_else(|error| {
                                panic!(
                                    "failed to serialize keyframe timeline range selector: {error}"
                                )
                            }),
                    ),
                )
            }
        }
    }

    /// Lower one Lightning page selector into one owned page selector.
    fn lower_page_selector(&self, selector: &lightning::PageSelector<'_>) -> PageSelector {
        PageSelector {
            name: selector.name.as_ref().map(ToString::to_string),
            pseudo_classes: selector
                .pseudo_classes
                .iter()
                .map(|pseudo_class| self.lower_page_pseudo_class(pseudo_class))
                .collect(),
        }
    }

    /// Lower one Lightning page pseudo class into one owned page pseudo class.
    fn lower_page_pseudo_class(
        &self,
        pseudo_class: &lightning::PagePseudoClass,
    ) -> PagePseudoClass {
        match pseudo_class {
            lightning::PagePseudoClass::Left => PagePseudoClass::Left,
            lightning::PagePseudoClass::Right => PagePseudoClass::Right,
            lightning::PagePseudoClass::First => PagePseudoClass::First,
            lightning::PagePseudoClass::Last => PagePseudoClass::Last,
            lightning::PagePseudoClass::Blank => PagePseudoClass::Blank,
        }
    }

    /// Lower one property name source into one owned property name.
    pub(crate) fn lower_property_name_source(&self, source: &str) -> PropertyName {
        if source.starts_with("--") {
            return PropertyName::Custom(source.to_string());
        }

        PropertyName::Standard(source.to_string())
    }

    /// Lower one declaration value source into one owned declaration value.
    pub(crate) fn lower_declaration_value_source(&self, source: &str) -> DeclarationValue {
        DeclarationValue {
            components: self.lower_component_value_list_source(source),
        }
    }

    /// Lower one authored declaration value source and split one trailing `!important`.
    fn lower_declaration_value_authored_source(&self, source: &str) -> (DeclarationValue, bool) {
        let (components, is_important) = Parser::parse_declaration_value(source);

        (DeclarationValue { components }, is_important)
    }

    /// Lower one Lightning property value into one owned declaration value.
    pub(crate) fn lower_declaration_value_property(
        &self,
        property: &lightning::Property<'_>,
    ) -> DeclarationValue {
        match property {
            lightning::Property::Unparsed(property) => DeclarationValue {
                components: self.lower_component_value_token_list(&property.value),
            },
            lightning::Property::Custom(property) => DeclarationValue {
                components: self.lower_component_value_token_list(&property.value),
            },
            _ => {
                let value = property
                    .value_to_css_string(lightning::PrinterOptions::default())
                    .unwrap_or_else(|error| {
                        panic!("failed to serialize lightning css property value: {error}")
                    });

                self.lower_declaration_value_source(&value)
            }
        }
    }

    /// Lower one layer name source into one owned layer name list.
    fn lower_layer_name(&self, name: &lightning::LayerName<'_>) -> LayerNameList {
        LayerNameList {
            names: name.0.iter().map(ToString::to_string).collect(),
        }
    }

    /// Lower one container name into one owned container name.
    fn lower_container_name(&self, name: &lightning::ContainerName<'_>) -> ContainerName {
        ContainerName {
            name: name.0.as_ref().to_string(),
        }
    }

    /// Lower one font palette name into one owned font palette name.
    fn lower_font_palette_name(&self, name: &str) -> FontPaletteName {
        FontPaletteName {
            name: name.to_string(),
        }
    }

    /// Lower one font feature family name into one owned family name.
    fn lower_font_feature_family_name<Family: lightning::ToCss>(
        &self,
        family: &Family,
    ) -> FontFeatureFamilyName {
        FontFeatureFamilyName {
            name: self.serialize_value(family),
        }
    }

    /// Lower one counter style name into one owned counter style name.
    fn lower_counter_style_name(&self, name: &str) -> CounterStyleName {
        CounterStyleName {
            name: name.to_string(),
        }
    }

    /// Lower one namespace prefix into one owned namespace prefix.
    fn lower_namespace_prefix(&self, prefix: &str) -> NamespacePrefix {
        NamespacePrefix {
            name: prefix.to_string(),
        }
    }

    /// Lower one Lightning vendor prefix into one owned vendor prefix.
    fn lower_vendor_prefix(&self, prefix: lightning::VendorPrefix) -> VendorPrefix {
        match prefix {
            lightning::VendorPrefix::None => VendorPrefix::None,
            lightning::VendorPrefix::WebKit => VendorPrefix::Webkit,
            lightning::VendorPrefix::Moz => VendorPrefix::Moz,
            lightning::VendorPrefix::Ms => VendorPrefix::Ms,
            lightning::VendorPrefix::O => VendorPrefix::O,
            _ => VendorPrefix::Other(self.serialize_value(&prefix)),
        }
    }

    /// Return one canonical vendor prefix source.
    fn vendor_prefix_source(&self, prefix: lightning::VendorPrefix) -> String {
        match prefix {
            lightning::VendorPrefix::None => String::new(),
            lightning::VendorPrefix::WebKit => "-webkit-".to_string(),
            lightning::VendorPrefix::Moz => "-moz-".to_string(),
            lightning::VendorPrefix::Ms => "-ms-".to_string(),
            lightning::VendorPrefix::O => "-o-".to_string(),
            _ => self.serialize_value(&prefix),
        }
    }

    /// Lower one custom media name into one owned custom media name.
    fn lower_custom_media_name(&self, name: &str) -> CustomMediaName {
        CustomMediaName {
            name: name.to_string(),
        }
    }

    /// Lower one custom property name into one owned custom property name.
    fn lower_custom_property_name(&self, name: &str) -> CustomPropertyName {
        CustomPropertyName {
            name: name.to_string(),
        }
    }

    /// Lower one keyframes name into one owned keyframes name.
    fn lower_keyframes_name(&self, name: &lightning::KeyframesName<'_>) -> KeyframesName {
        match name {
            lightning::KeyframesName::Ident(name) => KeyframesName {
                name: name.as_ref().to_string(),
            },
            lightning::KeyframesName::Custom(name) => KeyframesName {
                name: name.to_string(),
            },
        }
    }

    /// Lower one page margin box into one owned page margin box.
    fn lower_page_margin_box(&self, margin_box: lightning::PageMarginBox) -> PageMarginBox {
        match margin_box {
            lightning::PageMarginBox::TopLeftCorner => PageMarginBox::TopLeftCorner,
            lightning::PageMarginBox::TopLeft => PageMarginBox::TopLeft,
            lightning::PageMarginBox::TopCenter => PageMarginBox::TopCenter,
            lightning::PageMarginBox::TopRight => PageMarginBox::TopRight,
            lightning::PageMarginBox::TopRightCorner => PageMarginBox::TopRightCorner,
            lightning::PageMarginBox::LeftTop => PageMarginBox::LeftTop,
            lightning::PageMarginBox::LeftMiddle => PageMarginBox::LeftMiddle,
            lightning::PageMarginBox::LeftBottom => PageMarginBox::LeftBottom,
            lightning::PageMarginBox::RightTop => PageMarginBox::RightTop,
            lightning::PageMarginBox::RightMiddle => PageMarginBox::RightMiddle,
            lightning::PageMarginBox::RightBottom => PageMarginBox::RightBottom,
            lightning::PageMarginBox::BottomLeftCorner => PageMarginBox::BottomLeftCorner,
            lightning::PageMarginBox::BottomLeft => PageMarginBox::BottomLeft,
            lightning::PageMarginBox::BottomCenter => PageMarginBox::BottomCenter,
            lightning::PageMarginBox::BottomRight => PageMarginBox::BottomRight,
            lightning::PageMarginBox::BottomRightCorner => PageMarginBox::BottomRightCorner,
        }
    }

    /// Lower one attribute selector component.
    fn lower_attribute_selector(
        &self,
        component: &lightning::SelectorComponent<'_>,
    ) -> ComponentValueList {
        let mut values = Vec::new();

        // selector form
        match component {
            lightning::SelectorComponent::AttributeInNoNamespaceExists { local_name, .. } => {
                values.push(ComponentValue::Token(Token::Ident(local_name.to_string())));
            }
            lightning::SelectorComponent::AttributeInNoNamespace {
                local_name,
                operator,
                value,
                case_sensitivity,
                ..
            } => {
                values.push(ComponentValue::Token(Token::Ident(local_name.to_string())));
                values.push(ComponentValue::Token(
                    self.lower_attribute_operator(*operator),
                ));
                values.push(ComponentValue::Token(Token::String(
                    value.as_ref().to_string(),
                )));
                self.lower_attribute_case_sensitivity(&mut values, *case_sensitivity);
            }
            lightning::SelectorComponent::AttributeOther(attribute) => {
                // namespace
                if let Some(namespace) = &attribute.namespace {
                    self.lower_attribute_namespace(&mut values, namespace);
                }

                values.push(ComponentValue::Token(Token::Ident(
                    attribute.local_name.to_string(),
                )));

                // operation
                match &attribute.operation {
                    parcel::ParsedAttrSelectorOperation::Exists => {}
                    parcel::ParsedAttrSelectorOperation::WithValue {
                        operator,
                        case_sensitivity,
                        expected_value,
                    } => {
                        values.push(ComponentValue::Token(
                            self.lower_attribute_operator(*operator),
                        ));
                        values.push(ComponentValue::Token(Token::String(
                            expected_value.as_ref().to_string(),
                        )));
                        self.lower_attribute_case_sensitivity(&mut values, *case_sensitivity);
                    }
                }
            }
            _ => unreachable!(),
        }

        ComponentValueList { values }
    }

    /// Convert one 1-indexed source position range into one file span.
    fn span_from_source_positions(
        &self,
        start_line: usize,
        start_column: usize,
        end_line: usize,
        end_column: usize,
    ) -> Span {
        let start = File::byte_offset_from_position(self.source, start_line + 1, start_column);
        let end = File::byte_offset_from_position(self.source, end_line + 1, end_column);

        Span::new(self.file.id, start, end)
    }

    /// Slice one authored source span.
    fn source_slice(&self, span: Span) -> &str {
        &self.source[span.start as usize..span.end as usize]
    }

    /// Lower one attribute selector operator.
    fn lower_attribute_operator(&self, operator: parcel::AttrSelectorOperator) -> Token {
        match operator {
            parcel::AttrSelectorOperator::Equal => Token::Delimiter('='),
            parcel::AttrSelectorOperator::Includes => Token::Symbol(Symbol::IncludeMatch),
            parcel::AttrSelectorOperator::DashMatch => Token::Symbol(Symbol::DashMatch),
            parcel::AttrSelectorOperator::Prefix => Token::Symbol(Symbol::PrefixMatch),
            parcel::AttrSelectorOperator::Substring => Token::Symbol(Symbol::SubstringMatch),
            parcel::AttrSelectorOperator::Suffix => Token::Symbol(Symbol::SuffixMatch),
        }
    }

    /// Lower one attribute namespace constraint.
    fn lower_attribute_namespace(
        &self,
        values: &mut Vec<ComponentValue>,
        namespace: &parcel::NamespaceConstraint<(lightning::Ident<'_>, lightning::CowArcStr<'_>)>,
    ) {
        match namespace {
            parcel::NamespaceConstraint::Any => {
                values.push(ComponentValue::Token(Token::Delimiter('*')));
                values.push(ComponentValue::Token(Token::Delimiter('|')));
            }
            parcel::NamespaceConstraint::Specific((prefix, _)) => {
                values.push(ComponentValue::Token(Token::Ident(prefix.to_string())));
                values.push(ComponentValue::Token(Token::Delimiter('|')));
            }
        }
    }

    /// Lower one attribute case sensitivity suffix.
    fn lower_attribute_case_sensitivity(
        &self,
        values: &mut Vec<ComponentValue>,
        case_sensitivity: parcel::ParsedCaseSensitivity,
    ) {
        match case_sensitivity {
            parcel::ParsedCaseSensitivity::CaseSensitive
            | parcel::ParsedCaseSensitivity::AsciiCaseInsensitiveIfInHtmlElementInHtmlDocument => {}
            parcel::ParsedCaseSensitivity::AsciiCaseInsensitive => {
                values.push(ComponentValue::Token(Token::WhiteSpace(" ".to_string())));
                values.push(ComponentValue::Token(Token::Ident("i".to_string())));
            }
            parcel::ParsedCaseSensitivity::ExplicitCaseSensitive => {
                values.push(ComponentValue::Token(Token::WhiteSpace(" ".to_string())));
                values.push(ComponentValue::Token(Token::Ident("s".to_string())));
            }
        }
    }

    /// Lower one pseudo class.
    fn lower_pseudo_class(&mut self, pseudo: &lightning::PseudoClass<'_>) -> PseudoClass {
        match pseudo {
            lightning::PseudoClass::Lang { languages } => PseudoClass {
                name: "lang".to_string(),
                arguments: Some(self.lower_component_pseudo_argument(
                    self.lower_identifier_list_argument(languages, true),
                )),
            },
            lightning::PseudoClass::Dir { direction } => PseudoClass {
                name: "dir".to_string(),
                arguments: Some(self.lower_component_pseudo_argument(
                    self.lower_identifier_argument(match direction {
                        lightning::Direction::Ltr => "ltr",
                        lightning::Direction::Rtl => "rtl",
                    }),
                )),
            },
            lightning::PseudoClass::Hover => self.lower_simple_pseudo_class("hover"),
            lightning::PseudoClass::Active => self.lower_simple_pseudo_class("active"),
            lightning::PseudoClass::Focus => self.lower_simple_pseudo_class("focus"),
            lightning::PseudoClass::FocusVisible => self.lower_simple_pseudo_class("focus-visible"),
            lightning::PseudoClass::FocusWithin => self.lower_simple_pseudo_class("focus-within"),
            lightning::PseudoClass::Current => self.lower_simple_pseudo_class("current"),
            lightning::PseudoClass::Past => self.lower_simple_pseudo_class("past"),
            lightning::PseudoClass::Future => self.lower_simple_pseudo_class("future"),
            lightning::PseudoClass::Playing => self.lower_simple_pseudo_class("playing"),
            lightning::PseudoClass::Paused => self.lower_simple_pseudo_class("paused"),
            lightning::PseudoClass::Seeking => self.lower_simple_pseudo_class("seeking"),
            lightning::PseudoClass::Buffering => self.lower_simple_pseudo_class("buffering"),
            lightning::PseudoClass::Stalled => self.lower_simple_pseudo_class("stalled"),
            lightning::PseudoClass::Muted => self.lower_simple_pseudo_class("muted"),
            lightning::PseudoClass::VolumeLocked => self.lower_simple_pseudo_class("volume-locked"),
            lightning::PseudoClass::Fullscreen(prefix) => PseudoClass {
                name: match self.lower_vendor_prefix(*prefix) {
                    VendorPrefix::Webkit | VendorPrefix::Moz => {
                        format!("{}full-screen", self.vendor_prefix_source(*prefix))
                    }
                    _ => format!("{}fullscreen", self.vendor_prefix_source(*prefix)),
                },
                arguments: None,
            },
            lightning::PseudoClass::Open => self.lower_simple_pseudo_class("open"),
            lightning::PseudoClass::Closed => self.lower_simple_pseudo_class("closed"),
            lightning::PseudoClass::Modal => self.lower_simple_pseudo_class("modal"),
            lightning::PseudoClass::PictureInPicture => {
                self.lower_simple_pseudo_class("picture-in-picture")
            }
            lightning::PseudoClass::PopoverOpen => self.lower_simple_pseudo_class("popover-open"),
            lightning::PseudoClass::Defined => self.lower_simple_pseudo_class("defined"),
            lightning::PseudoClass::AnyLink(prefix) => PseudoClass {
                name: format!("{}any-link", self.vendor_prefix_source(*prefix)),
                arguments: None,
            },
            lightning::PseudoClass::Link => self.lower_simple_pseudo_class("link"),
            lightning::PseudoClass::LocalLink => self.lower_simple_pseudo_class("local-link"),
            lightning::PseudoClass::Target => self.lower_simple_pseudo_class("target"),
            lightning::PseudoClass::TargetWithin => self.lower_simple_pseudo_class("target-within"),
            lightning::PseudoClass::Visited => self.lower_simple_pseudo_class("visited"),
            lightning::PseudoClass::Enabled => self.lower_simple_pseudo_class("enabled"),
            lightning::PseudoClass::Disabled => self.lower_simple_pseudo_class("disabled"),
            lightning::PseudoClass::ReadOnly(prefix) => PseudoClass {
                name: format!("{}read-only", self.vendor_prefix_source(*prefix)),
                arguments: None,
            },
            lightning::PseudoClass::ReadWrite(prefix) => PseudoClass {
                name: format!("{}read-write", self.vendor_prefix_source(*prefix)),
                arguments: None,
            },
            lightning::PseudoClass::PlaceholderShown(prefix) => PseudoClass {
                name: format!("{}placeholder-shown", self.vendor_prefix_source(*prefix)),
                arguments: None,
            },
            lightning::PseudoClass::Default => self.lower_simple_pseudo_class("default"),
            lightning::PseudoClass::Checked => self.lower_simple_pseudo_class("checked"),
            lightning::PseudoClass::Indeterminate => {
                self.lower_simple_pseudo_class("indeterminate")
            }
            lightning::PseudoClass::Blank => self.lower_simple_pseudo_class("blank"),
            lightning::PseudoClass::Valid => self.lower_simple_pseudo_class("valid"),
            lightning::PseudoClass::Invalid => self.lower_simple_pseudo_class("invalid"),
            lightning::PseudoClass::InRange => self.lower_simple_pseudo_class("in-range"),
            lightning::PseudoClass::OutOfRange => self.lower_simple_pseudo_class("out-of-range"),
            lightning::PseudoClass::Required => self.lower_simple_pseudo_class("required"),
            lightning::PseudoClass::Optional => self.lower_simple_pseudo_class("optional"),
            lightning::PseudoClass::UserValid => self.lower_simple_pseudo_class("user-valid"),
            lightning::PseudoClass::UserInvalid => self.lower_simple_pseudo_class("user-invalid"),
            lightning::PseudoClass::Autofill(prefix) => PseudoClass {
                name: format!("{}autofill", self.vendor_prefix_source(*prefix)),
                arguments: None,
            },
            lightning::PseudoClass::ActiveViewTransition => {
                self.lower_simple_pseudo_class("active-view-transition")
            }
            lightning::PseudoClass::ActiveViewTransitionType { kind } => PseudoClass {
                name: "active-view-transition-type".to_string(),
                arguments: Some(self.lower_component_pseudo_argument(
                    self.lower_custom_ident_list_argument(kind, true),
                )),
            },
            lightning::PseudoClass::State { state } => PseudoClass {
                name: "state".to_string(),
                arguments: Some(self.lower_component_pseudo_argument(
                    self.lower_identifier_argument(state.as_ref()),
                )),
            },
            lightning::PseudoClass::Local { selector } => PseudoClass {
                name: "local".to_string(),
                arguments: Some(self.lower_selector_argument(selector)),
            },
            lightning::PseudoClass::Global { selector } => PseudoClass {
                name: "global".to_string(),
                arguments: Some(self.lower_selector_argument(selector)),
            },
            lightning::PseudoClass::WebKitScrollbar(pseudo) => PseudoClass {
                name: match pseudo {
                    lightning::WebKitScrollbarPseudoClass::Horizontal => "horizontal",
                    lightning::WebKitScrollbarPseudoClass::Vertical => "vertical",
                    lightning::WebKitScrollbarPseudoClass::Decrement => "decrement",
                    lightning::WebKitScrollbarPseudoClass::Increment => "increment",
                    lightning::WebKitScrollbarPseudoClass::Start => "start",
                    lightning::WebKitScrollbarPseudoClass::End => "end",
                    lightning::WebKitScrollbarPseudoClass::DoubleButton => "double-button",
                    lightning::WebKitScrollbarPseudoClass::SingleButton => "single-button",
                    lightning::WebKitScrollbarPseudoClass::NoButton => "no-button",
                    lightning::WebKitScrollbarPseudoClass::CornerPresent => "corner-present",
                    lightning::WebKitScrollbarPseudoClass::WindowInactive => "window-inactive",
                }
                .to_string(),
                arguments: None,
            },
            lightning::PseudoClass::Custom { name } => PseudoClass {
                name: name.to_string(),
                arguments: None,
            },
            lightning::PseudoClass::CustomFunction { name, arguments } => PseudoClass {
                name: name.to_string(),
                arguments: Some(self.lower_component_pseudo_argument(
                    self.lower_component_value_token_list(arguments),
                )),
            },
        }
    }

    /// Lower one pseudo element.
    fn lower_pseudo_element(&mut self, pseudo: &lightning::PseudoElement<'_>) -> PseudoElement {
        match pseudo {
            lightning::PseudoElement::After => self.lower_simple_pseudo_element("after"),
            lightning::PseudoElement::Before => self.lower_simple_pseudo_element("before"),
            lightning::PseudoElement::FirstLine => self.lower_simple_pseudo_element("first-line"),
            lightning::PseudoElement::FirstLetter => {
                self.lower_simple_pseudo_element("first-letter")
            }
            lightning::PseudoElement::DetailsContent => {
                self.lower_simple_pseudo_element("details-content")
            }
            lightning::PseudoElement::TargetText => self.lower_simple_pseudo_element("target-text"),
            lightning::PseudoElement::Selection(prefix) => PseudoElement {
                name: format!("{}selection", self.vendor_prefix_source(*prefix)),
                arguments: None,
            },
            lightning::PseudoElement::Placeholder(prefix) => PseudoElement {
                name: match self.lower_vendor_prefix(*prefix) {
                    VendorPrefix::Webkit | VendorPrefix::Ms => {
                        format!("{}input-placeholder", self.vendor_prefix_source(*prefix))
                    }
                    _ => format!("{}placeholder", self.vendor_prefix_source(*prefix)),
                },
                arguments: None,
            },
            lightning::PseudoElement::Marker => self.lower_simple_pseudo_element("marker"),
            lightning::PseudoElement::Backdrop(prefix) => PseudoElement {
                name: format!("{}backdrop", self.vendor_prefix_source(*prefix)),
                arguments: None,
            },
            lightning::PseudoElement::FileSelectorButton(prefix) => PseudoElement {
                name: match self.lower_vendor_prefix(*prefix) {
                    VendorPrefix::Webkit => {
                        format!("{}file-upload-button", self.vendor_prefix_source(*prefix))
                    }
                    VendorPrefix::Ms => format!("{}browse", self.vendor_prefix_source(*prefix)),
                    _ => format!("{}file-selector-button", self.vendor_prefix_source(*prefix)),
                },
                arguments: None,
            },
            lightning::PseudoElement::WebKitScrollbar(pseudo) => PseudoElement {
                name: match pseudo {
                    lightning::WebKitScrollbarPseudoElement::Scrollbar => "-webkit-scrollbar",
                    lightning::WebKitScrollbarPseudoElement::Button => "-webkit-scrollbar-button",
                    lightning::WebKitScrollbarPseudoElement::Track => "-webkit-scrollbar-track",
                    lightning::WebKitScrollbarPseudoElement::TrackPiece => {
                        "-webkit-scrollbar-track-piece"
                    }
                    lightning::WebKitScrollbarPseudoElement::Thumb => "-webkit-scrollbar-thumb",
                    lightning::WebKitScrollbarPseudoElement::Corner => "-webkit-scrollbar-corner",
                    lightning::WebKitScrollbarPseudoElement::Resizer => "-webkit-resizer",
                }
                .to_string(),
                arguments: None,
            },
            lightning::PseudoElement::Cue => self.lower_simple_pseudo_element("cue"),
            lightning::PseudoElement::CueRegion => self.lower_simple_pseudo_element("cue-region"),
            lightning::PseudoElement::CueFunction { selector } => PseudoElement {
                name: "cue".to_string(),
                arguments: Some(self.lower_selector_argument(selector)),
            },
            lightning::PseudoElement::CueRegionFunction { selector } => PseudoElement {
                name: "cue-region".to_string(),
                arguments: Some(self.lower_selector_argument(selector)),
            },
            lightning::PseudoElement::ViewTransition => {
                self.lower_simple_pseudo_element("view-transition")
            }
            lightning::PseudoElement::ViewTransitionGroup { part } => PseudoElement {
                name: "view-transition-group".to_string(),
                arguments: Some(self.lower_component_pseudo_argument(
                    self.lower_view_transition_part_argument(part),
                )),
            },
            lightning::PseudoElement::ViewTransitionImagePair { part } => PseudoElement {
                name: "view-transition-image-pair".to_string(),
                arguments: Some(self.lower_component_pseudo_argument(
                    self.lower_view_transition_part_argument(part),
                )),
            },
            lightning::PseudoElement::ViewTransitionOld { part } => PseudoElement {
                name: "view-transition-old".to_string(),
                arguments: Some(self.lower_component_pseudo_argument(
                    self.lower_view_transition_part_argument(part),
                )),
            },
            lightning::PseudoElement::ViewTransitionNew { part } => PseudoElement {
                name: "view-transition-new".to_string(),
                arguments: Some(self.lower_component_pseudo_argument(
                    self.lower_view_transition_part_argument(part),
                )),
            },
            lightning::PseudoElement::PickerFunction { identifier } => PseudoElement {
                name: "picker".to_string(),
                arguments: Some(self.lower_component_pseudo_argument(
                    self.lower_identifier_argument(identifier.as_ref()),
                )),
            },
            lightning::PseudoElement::PickerIcon => self.lower_simple_pseudo_element("picker-icon"),
            lightning::PseudoElement::Checkmark => self.lower_simple_pseudo_element("checkmark"),
            lightning::PseudoElement::GrammarError => {
                self.lower_simple_pseudo_element("grammar-error")
            }
            lightning::PseudoElement::SpellingError => {
                self.lower_simple_pseudo_element("spelling-error")
            }
            lightning::PseudoElement::Custom { name } => PseudoElement {
                name: name.to_string(),
                arguments: None,
            },
            lightning::PseudoElement::CustomFunction { name, arguments } => PseudoElement {
                name: name.to_string(),
                arguments: Some(self.lower_component_pseudo_argument(
                    self.lower_component_value_token_list(arguments),
                )),
            },
        }
    }

    /// Lower one pseudo class without arguments.
    fn lower_simple_pseudo_class(&self, name: &str) -> PseudoClass {
        PseudoClass {
            name: name.to_string(),
            arguments: None,
        }
    }

    /// Lower one pseudo element without arguments.
    fn lower_simple_pseudo_element(&self, name: &str) -> PseudoElement {
        PseudoElement {
            name: name.to_string(),
            arguments: None,
        }
    }

    /// Lower one identifier argument into component values.
    fn lower_identifier_argument(&self, identifier: &str) -> ComponentValueList {
        ComponentValueList {
            values: vec![ComponentValue::Token(Token::Ident(identifier.to_string()))],
        }
    }

    /// Lower one identifier list argument into component values.
    fn lower_identifier_list_argument(
        &self,
        identifiers: &[impl AsRef<str>],
        is_comma_separated: bool,
    ) -> ComponentValueList {
        let mut values = Vec::new();

        for (index, identifier) in identifiers.iter().enumerate() {
            if index > 0 {
                values.push(ComponentValue::Token(Token::Symbol(Symbol::Comma)));

                if is_comma_separated {
                    values.push(ComponentValue::Token(Token::WhiteSpace(" ".to_string())));
                }
            }

            values.push(ComponentValue::Token(Token::Ident(
                identifier.as_ref().to_string(),
            )));
        }

        ComponentValueList { values }
    }

    /// Lower one custom identifier list argument into component values.
    fn lower_custom_ident_list_argument(
        &self,
        identifiers: &[lightning::CustomIdent<'_>],
        is_comma_separated: bool,
    ) -> ComponentValueList {
        let identifiers = identifiers
            .iter()
            .map(|identifier| identifier.0.as_ref())
            .collect::<Vec<_>>();

        self.lower_identifier_list_argument(&identifiers, is_comma_separated)
    }

    /// Lower one view-transition part selector argument into component values.
    fn lower_view_transition_part_argument(
        &self,
        part: &lightning::ViewTransitionPartSelector<'_>,
    ) -> ComponentValueList {
        let source = self.serialize_value(part);

        self.lower_component_value_list_source(&source)
    }

    /// Wrap one generic pseudo argument payload.
    fn lower_component_pseudo_argument(&self, arguments: ComponentValueList) -> PseudoArgument {
        PseudoArgument::Components(arguments)
    }

    /// Lower one selector argument into component values.
    fn lower_selector_argument(&mut self, selector: &lightning::Selector<'_>) -> PseudoArgument {
        PseudoArgument::Selector(self.lower_selector(selector))
    }

    /// Lower one nth selector source and affine data.
    fn lower_nth_selector(
        &self,
        selector_type: parcel::NthType,
        a: i32,
        b: i32,
        is_function: bool,
    ) -> NthSelector {
        NthSelector {
            kind: match selector_type {
                parcel::NthType::Child => NthSelectorKind::Child,
                parcel::NthType::LastChild => NthSelectorKind::LastChild,
                parcel::NthType::OfType => NthSelectorKind::OfType,
                parcel::NthType::LastOfType => NthSelectorKind::LastOfType,
                parcel::NthType::OnlyChild => NthSelectorKind::OnlyChild,
                parcel::NthType::OnlyOfType => NthSelectorKind::OnlyOfType,
                parcel::NthType::Col => NthSelectorKind::Column,
                parcel::NthType::LastCol => NthSelectorKind::LastColumn,
            },
            is_function,
            a,
            b,
        }
    }

    /// Lower one numeric percentage payload.
    fn lower_percentage_number(&self, value: f32) -> Number {
        let integer_value = (value.fract() == 0.0).then_some(value as i32);

        Number {
            has_sign: value.is_sign_negative(),
            value,
            integer_value,
        }
    }

    /// Lower one timeline range percentage source.
    fn lower_timeline_range_percentage_source(&self, source: &str) -> TimelineRangePercentage {
        let mut parts = source.split_whitespace();
        let name = parts
            .next()
            .unwrap_or_else(|| panic!("missing timeline range name in {source}"));
        let percentage = parts
            .next()
            .unwrap_or_else(|| panic!("missing timeline range percentage in {source}"));

        TimelineRangePercentage {
            name: match name {
                "cover" => TimelineRangeName::Cover,
                "contain" => TimelineRangeName::Contain,
                "entry" => TimelineRangeName::Entry,
                "exit" => TimelineRangeName::Exit,
                "entry-crossing" => TimelineRangeName::EntryCrossing,
                "exit-crossing" => TimelineRangeName::ExitCrossing,
                _ => panic!("unknown timeline range name: {name}"),
            },
            percentage: self.lower_percentage_number(
                percentage
                    .trim_end_matches('%')
                    .parse::<f32>()
                    .unwrap_or_else(|error| {
                        panic!(
                            "failed to parse keyframe timeline range percentage {source}: {error}"
                        )
                    }),
            ),
        }
    }

    /// Return the serialized form of one CSS value.
    pub(crate) fn serialize_value<T: lightning::ToCss>(&self, value: &T) -> String {
        serialize_value(value)
    }

    /// Return the serialized form of one CSS rule.
    fn serialize_rule(&self, rule: &lightning::CssRule<'a>) -> String {
        serialize_rule(rule)
    }

    /// Return the source span for one CSS rule.
    fn span_for_rule(&self, rule: &lightning::CssRule<'a>) -> Span {
        span_for_rule(self.source, self.file, rule)
    }

    /// Lower one Lightning property syntax definition into owned CSS syntax.
    fn lower_property_syntax(&self, syntax: &lightning::SyntaxString) -> PropertySyntax {
        match syntax {
            lightning::SyntaxString::Universal => PropertySyntax::Universal,
            lightning::SyntaxString::Components(components) => PropertySyntax::Components(
                components
                    .iter()
                    .map(|component| self.lower_property_syntax_component(component))
                    .collect(),
            ),
        }
    }

    /// Lower one Lightning property syntax component into owned CSS syntax.
    fn lower_property_syntax_component(
        &self,
        component: &lightning::SyntaxComponent,
    ) -> PropertySyntaxComponent {
        PropertySyntaxComponent {
            kind: self.lower_property_syntax_component_kind(&component.kind),
            multiplier: self.lower_property_syntax_multiplier(&component.multiplier),
        }
    }

    /// Lower one Lightning property syntax component kind into owned CSS syntax.
    fn lower_property_syntax_component_kind(
        &self,
        kind: &lightning::SyntaxComponentKind,
    ) -> PropertySyntaxComponentKind {
        match kind {
            lightning::SyntaxComponentKind::Length => PropertySyntaxComponentKind::Length,
            lightning::SyntaxComponentKind::Number => PropertySyntaxComponentKind::Number,
            lightning::SyntaxComponentKind::Percentage => PropertySyntaxComponentKind::Percentage,
            lightning::SyntaxComponentKind::LengthPercentage => {
                PropertySyntaxComponentKind::LengthPercentage
            }
            lightning::SyntaxComponentKind::String => PropertySyntaxComponentKind::String,
            lightning::SyntaxComponentKind::Color => PropertySyntaxComponentKind::Color,
            lightning::SyntaxComponentKind::Image => PropertySyntaxComponentKind::Image,
            lightning::SyntaxComponentKind::Url => PropertySyntaxComponentKind::Url,
            lightning::SyntaxComponentKind::Integer => PropertySyntaxComponentKind::Integer,
            lightning::SyntaxComponentKind::Angle => PropertySyntaxComponentKind::Angle,
            lightning::SyntaxComponentKind::Time => PropertySyntaxComponentKind::Time,
            lightning::SyntaxComponentKind::Resolution => PropertySyntaxComponentKind::Resolution,
            lightning::SyntaxComponentKind::TransformFunction => {
                PropertySyntaxComponentKind::TransformFunction
            }
            lightning::SyntaxComponentKind::TransformList => {
                PropertySyntaxComponentKind::TransformList
            }
            lightning::SyntaxComponentKind::CustomIdent => PropertySyntaxComponentKind::CustomIdent,
            lightning::SyntaxComponentKind::Literal(value) => {
                PropertySyntaxComponentKind::Literal(value.clone())
            }
        }
    }

    /// Lower one Lightning property syntax multiplier into owned CSS syntax.
    fn lower_property_syntax_multiplier(
        &self,
        multiplier: &lightning::SyntaxMultiplier,
    ) -> PropertySyntaxMultiplier {
        match multiplier {
            lightning::SyntaxMultiplier::None => PropertySyntaxMultiplier::None,
            lightning::SyntaxMultiplier::Space => PropertySyntaxMultiplier::Space,
            lightning::SyntaxMultiplier::Comma => PropertySyntaxMultiplier::Comma,
        }
    }
}
