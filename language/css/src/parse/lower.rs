use crate::{
    AnySelector, AttributeSelector, Combinator, ComponentValue, ComponentValueList, ContainerName,
    ContainerRule, CounterStyleName, CounterStyleRule, CustomMediaName, CustomMediaRule,
    CustomPropertyName, CustomRule, Declaration, DeclarationBlock, DeclarationValue, FontFaceRule,
    FontFeatureDeclaration, FontFeatureFamilyName, FontFeatureSubrule, FontFeatureSubruleKind,
    FontFeatureValuesRule, FontPaletteName, FontPaletteValuesRule, Function, IgnoredRule,
    ImportLayer, ImportRule, KeyframeRule, KeyframeSelector, KeyframeSelectorList, KeyframesName,
    KeyframesRule, LayerBlockRule, LayerNameList, LayerStatementRule, LocalName, LocalNodeId,
    MediaRule, MozDocumentRule, NamespacePrefix, NamespaceRule, NamespaceUrl,
    NestedDeclarationsRule, NestingRule, Node, NodeSpanRegion, NodeSpanType, NthOfSelector,
    NthSelector, NthSelectorKind, Number, PageMarginBox, PageMarginRule, PagePseudoClass, PageRule,
    PageSelector, PageSelectorList, PropertyName, PropertyRule, PropertySyntax,
    PropertySyntaxComponent, PropertySyntaxComponentKind, PropertySyntaxMultiplier, PseudoArgument,
    PseudoClass, PseudoElement, Rule, ScopeRule, Selector, SelectorComponent, SelectorList,
    SimpleSelector, StartingStyleRule, StyleRule, Stylesheet, SupportsRule, Symbol,
    TimelineRangeName, TimelineRangePercentage, Token, Tree, TreeImpl, UnknownRule, VendorPrefix,
    ViewTransitionPartArgument, ViewTransitionRule, ViewportRule,
};
use destack_core::StringId;
use destack_source::{File, Span};

use super::parse::Parser;
use super::{lightning, parcel};

/// One CSS AST lowerer.
pub(crate) struct Lowerer<'a> {
    /// The authored source file.
    file: &'a File,
    /// The authored source text.
    source: &'a str,
    /// The output CSS tree.
    tree: Tree,
    /// The next stable resource id.
    next_resource_id: u32,
}

impl<'a> Lowerer<'a> {
    /// Create one CSS AST lowerer.
    pub(crate) fn new(file: &'a File, source: &'a str) -> Self {
        Self {
            file,
            source,
            tree: Tree::new(),
            next_resource_id: 0,
        }
    }

    /// Lower one parsed Lightning stylesheet into the Destack CSS tree.
    pub(crate) fn lower_stylesheet<'o>(
        mut self,
        stylesheet: lightning::LightningStylesheet<'a, 'o>,
    ) -> (Tree, LocalNodeId<Stylesheet>) {
        let root_span = Span::new(self.file.id, 0, self.source.len() as u32);
        let rules = stylesheet
            .rules
            .0
            .into_iter()
            .map(|rule| self.lower_rule(rule))
            .collect();
        let stylesheet = self.tree.insert(
            Stylesheet {
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

    /// Intern one pooled css string into the output tree.
    pub(crate) fn intern_string(&self, value: &str) -> StringId {
        self.tree.intern(value)
    }

    /// Parse one canonical component value list into this tree's string space.
    pub(crate) fn parse_component_value_list_source(&mut self, source: &str) -> ComponentValueList {
        let strings = &self.tree.strings;
        let next_resource_id = &mut self.next_resource_id;

        Parser::parse_component_value_list_with_pool(strings, next_resource_id, source)
    }

    /// Allocate one stable resource id.
    pub(crate) fn allocate_resource_id(&mut self) -> u32 {
        let resource_id = self.next_resource_id;
        self.next_resource_id += 1;
        resource_id
    }

    /// Insert one inner CSS node.
    pub(crate) fn insert_inner<T>(&mut self, node: T) -> LocalNodeId<T>
    where
        T: Node,
        Tree: TreeImpl<T>,
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
                declarations: self.lower_authored_declaration_block(span).or_else(|| {
                    span.is_empty()
                        .then(|| self.lower_declaration_block(&rule.declarations, span))
                        .flatten()
                }),
                page_margin_rules: self.lower_page_margin_rules(rule.rules, span),
            }),
            lightning::CssRule::FontFace(rule) => Rule::FontFace(FontFaceRule {
                declarations: self.lower_authored_declaration_block(span).or_else(|| {
                    span.is_empty()
                        .then(|| self.lower_font_face_declaration_block(&rule.properties, span))
                        .flatten()
                }),
            }),
            lightning::CssRule::FontPaletteValues(rule) => {
                Rule::FontPaletteValues(FontPaletteValuesRule {
                    name: self.lower_font_palette_name(rule.name.as_ref()),
                    declarations: self.lower_authored_declaration_block(span).or_else(|| {
                        span.is_empty()
                            .then(|| {
                                self.lower_font_palette_values_declaration_block(
                                    &rule.properties,
                                    span,
                                )
                            })
                            .flatten()
                    }),
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
                declarations: self.lower_authored_declaration_block(span).or_else(|| {
                    span.is_empty()
                        .then(|| self.lower_declaration_block(&rule.declarations, span))
                        .flatten()
                }),
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
                    declarations: self.lower_authored_declaration_block(span).or_else(|| {
                        span.is_empty()
                            .then(|| self.lower_declaration_block(&rule.declarations, span))
                            .flatten()
                    }),
                })
            }
            lightning::CssRule::Viewport(rule) => Rule::Viewport(ViewportRule {
                vendor_prefix: self.lower_vendor_prefix(rule.vendor_prefix),
                declarations: self.lower_authored_declaration_block(span).or_else(|| {
                    span.is_empty()
                        .then(|| self.lower_declaration_block(&rule.declarations, span))
                        .flatten()
                }),
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
                initial_value: self.lower_property_rule_initial_value(span).or_else(|| {
                    rule.initial_value.as_ref().map(|value| DeclarationValue {
                        components: self.lower_parsed_component(value),
                    })
                }),
            }),
            lightning::CssRule::Keyframes(rule) => Rule::Keyframes(KeyframesRule {
                name: self.lower_keyframes_name(&rule.name),
                vendor_prefix: self.lower_vendor_prefix(rule.vendor_prefix),
                rules: self.lower_keyframe_rules(rule.keyframes, span),
            }),
            lightning::CssRule::ViewTransition(rule) => Rule::ViewTransition(ViewTransitionRule {
                declarations: self.lower_authored_declaration_block(span).or_else(|| {
                    span.is_empty()
                        .then(|| {
                            self.lower_view_transition_declaration_block(&rule.properties, span)
                        })
                        .flatten()
                }),
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
                components: if span.is_empty() {
                    self.lower_component_value_list_source(
                        &self.serialize_rule(&lightning::CssRule::Custom(rule)),
                    )
                } else {
                    {
                        let source = self.source_slice(span).to_string();

                        self.lower_component_value_list_source(&source)
                    }
                },
            }),
            lightning::CssRule::Ignored => Rule::Ignored(IgnoredRule {}),
        };
        let prelude_span = self.prelude_span_for_rule(span);
        let import_url_span = match &node {
            Rule::Import(import_rule) => self.import_url_span_for_rule(span, &import_rule.url),
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
            self.tree.set_side_span(
                rule_id,
                NodeSpanType::Region(NodeSpanRegion::Prelude),
                prelude_span,
            );
        }

        if let Some(import_url_span) = import_url_span {
            self.tree.set_side_span(
                rule_id,
                NodeSpanType::Region(NodeSpanRegion::Value),
                import_url_span,
            );
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
        let Some((body_start, body_end)) = self.rule_block_body_range(span) else {
            return rules
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
                .collect();
        };
        let authored_keyframes = Parser::parse_keyframe_block(&self.source[body_start..body_end]);

        if authored_keyframes.len() != rules.len() {
            return rules
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
                .collect();
        }

        rules
            .into_iter()
            .zip(authored_keyframes)
            .map(|(rule, authored_keyframe)| {
                let keyframe_span = Span::new(
                    self.file.id,
                    (body_start + authored_keyframe.rule_start) as u32,
                    (body_start + authored_keyframe.rule_end) as u32,
                );
                let selectors =
                    self.lower_keyframe_selector_list_source(authored_keyframe.selector);
                let declarations = self
                    .lower_authored_declaration_body(
                        body_start + authored_keyframe.body_start,
                        authored_keyframe.body,
                        keyframe_span,
                    )
                    .or_else(|| {
                        keyframe_span
                            .is_empty()
                            .then(|| {
                                self.lower_declaration_block(&rule.declarations, keyframe_span)
                            })
                            .flatten()
                    });

                self.tree.insert(
                    Rule::Keyframe(KeyframeRule {
                        selectors,
                        declarations,
                    }),
                    keyframe_span,
                )
            })
            .collect()
    }

    /// Lower one nested page margin rule list.
    fn lower_page_margin_rules(
        &mut self,
        rules: Vec<lightning::PageMarginRule<'a>>,
        fallback_span: Span,
    ) -> Vec<LocalNodeId<PageMarginRule>> {
        rules
            .into_iter()
            .map(|rule| {
                let span = self.span_for_location(rule.loc);
                let declarations = self.lower_authored_declaration_block(span).or_else(|| {
                    span.is_empty()
                        .then(|| self.lower_declaration_block(&rule.declarations, fallback_span))
                        .flatten()
                });

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
                return self.lower_authored_declaration_block(span).or_else(|| {
                    span.is_empty()
                        .then(|| self.lower_declaration_block(&rule.declarations, span))
                        .flatten()
                });
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
        let name = self.source_slice(name_span).to_string();
        let value = self.source_slice(value_span).to_string();
        let declaration_id =
            self.lower_declaration_from_source(name_span, &name, value_span, &value, fallback_span);

        Some(declaration_id)
    }

    /// Lower one authored declaration block from one braced rule span.
    fn lower_authored_declaration_block(
        &mut self,
        rule_span: Span,
    ) -> Option<LocalNodeId<DeclarationBlock>> {
        let (body_start, body_end) = self.rule_block_body_range(rule_span)?;
        let body_source = &self.source[body_start..body_end];
        self.lower_authored_declaration_body(body_start, body_source, rule_span)
    }

    /// Lower one authored declaration block from one block body source slice.
    fn lower_authored_declaration_body(
        &mut self,
        body_start: usize,
        body_source: &str,
        fallback_span: Span,
    ) -> Option<LocalNodeId<DeclarationBlock>> {
        let authored_declarations = Parser::parse_declaration_block(body_source);

        let declarations = authored_declarations
            .into_iter()
            .map(|declaration| {
                let name_span = Span::new(
                    self.file.id,
                    (body_start + declaration.name_start) as u32,
                    (body_start + declaration.name_end) as u32,
                );
                let value_span = Span::new(
                    self.file.id,
                    (body_start + declaration.value_start) as u32,
                    (body_start + declaration.value_end) as u32,
                );

                self.lower_declaration_from_source(
                    name_span,
                    declaration.name,
                    value_span,
                    declaration.value,
                    fallback_span,
                )
            })
            .collect::<Vec<_>>();

        if declarations.is_empty() {
            return None;
        }

        Some(
            self.tree
                .insert(DeclarationBlock { declarations }, fallback_span),
        )
    }

    /// Lower one declaration from authored source spans.
    fn lower_declaration_from_source(
        &mut self,
        name_span: Span,
        name_source: &str,
        value_span: Span,
        value_source: &str,
        fallback_span: Span,
    ) -> LocalNodeId<Declaration> {
        let declaration_span = name_span.merge(value_span);
        let (value, is_important) = self.lower_declaration_value_authored_source(value_source);
        let declaration = Declaration {
            name: self.lower_property_name_source(name_source),
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

        self.tree.set_side_span(
            declaration_id,
            NodeSpanType::Region(NodeSpanRegion::Name),
            name_span,
        );
        self.tree.set_side_span(
            declaration_id,
            NodeSpanType::Region(NodeSpanRegion::Value),
            value_span,
        );

        declaration_id
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
                let declaration = self.lower_declaration(property, is_important);

                self.tree.insert(declaration, span)
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
            resource: Some(self.build_import_resource(rule.url.as_ref())),
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
        &mut self,
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
        &mut self,
        property: &lightning::FontFaceProperty<'a>,
    ) -> Declaration {
        match property {
            lightning::FontFaceProperty::Source(value) => {
                let value = self.lower_font_source_list(value);

                self.lower_raw_declaration("src", value)
            }
            lightning::FontFaceProperty::FontFamily(value) => {
                let value = self.lower_font_family_declaration_value(value);

                self.lower_raw_declaration("font-family", value)
            }
            lightning::FontFaceProperty::FontStyle(value) => {
                let value = self.lower_font_face_style_range(value);

                self.lower_raw_declaration("font-style", value)
            }
            lightning::FontFaceProperty::FontWeight(value) => {
                let value = self.lower_font_weight_range(value);

                self.lower_raw_declaration("font-weight", value)
            }
            lightning::FontFaceProperty::FontStretch(value) => {
                let value = self.lower_font_stretch_range(value);

                self.lower_raw_declaration("font-stretch", value)
            }
            lightning::FontFaceProperty::UnicodeRange(value) => {
                let value = self.lower_unicode_range_list(value);

                self.lower_raw_declaration("unicode-range", value)
            }
            lightning::FontFaceProperty::Custom(custom) => {
                let value = self.lower_component_value_token_list(&custom.value);

                self.lower_custom_property_declaration(&custom.name, value)
            }
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
        &mut self,
        property: &lightning::FontPaletteValuesProperty<'a>,
    ) -> Declaration {
        match property {
            lightning::FontPaletteValuesProperty::FontFamily(value) => {
                let value = self.lower_font_family_declaration_value(value);

                self.lower_raw_declaration("font-family", value)
            }
            lightning::FontPaletteValuesProperty::BasePalette(value) => {
                let value = self.lower_base_palette_declaration_value(value);

                self.lower_raw_declaration("base-palette", value)
            }
            lightning::FontPaletteValuesProperty::OverrideColors(value) => {
                let value = self.lower_serialized_declaration_value_list(value);

                self.lower_raw_declaration("override-colors", value)
            }
            lightning::FontPaletteValuesProperty::Custom(custom) => {
                let value = self.lower_component_value_token_list(&custom.value);

                self.lower_custom_property_declaration(&custom.name, value)
            }
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
        &mut self,
        property: &lightning::ViewTransitionProperty<'a>,
    ) -> Declaration {
        match property {
            lightning::ViewTransitionProperty::Navigation(value) => {
                let value = self.lower_navigation_declaration_value(value);

                self.lower_raw_declaration("navigation", value)
            }
            lightning::ViewTransitionProperty::Types(value) => {
                let value = self.lower_none_or_custom_ident_list(value);

                self.lower_raw_declaration("types", value)
            }
            lightning::ViewTransitionProperty::Custom(custom) => {
                let value = self.lower_component_value_token_list(&custom.value);

                self.lower_custom_property_declaration(&custom.name, value)
            }
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

    /// Lower one serialized CSS list into one declaration value.
    fn lower_serialized_declaration_value_list<T: lightning::ToCss>(
        &mut self,
        values: &[T],
    ) -> DeclarationValue {
        let mut components = Vec::new();

        for (index, value) in values.iter().enumerate() {
            if index > 0 {
                components.push(ComponentValue::Token(Token::Symbol(Symbol::Comma)));
                components.push(ComponentValue::Token(Token::WhiteSpace(" ".to_string())));
            }

            components.extend(
                self.lower_component_value_list_source(&self.serialize_value(value))
                    .values,
            );
        }

        DeclarationValue {
            components: ComponentValueList { values: components },
        }
    }

    /// Lower one `@font-face` source list into one declaration value.
    fn lower_font_source_list(&mut self, values: &[lightning::FontSource<'_>]) -> DeclarationValue {
        let mut components = Vec::new();

        for (index, value) in values.iter().enumerate() {
            if index > 0 {
                components.push(ComponentValue::Token(Token::Symbol(Symbol::Comma)));
                components.push(ComponentValue::Token(Token::WhiteSpace(" ".to_string())));
            }

            components.extend(self.lower_font_source(value));
        }

        DeclarationValue {
            components: ComponentValueList { values: components },
        }
    }

    /// Lower one `@font-face` source into component values.
    fn lower_font_source(&mut self, value: &lightning::FontSource<'_>) -> Vec<ComponentValue> {
        match value {
            lightning::FontSource::Url(value) => {
                let mut values = vec![ComponentValue::Function(
                    self.lower_url_function(value.url.url.as_ref()),
                )];

                if let Some(format) = &value.format {
                    values.push(ComponentValue::Token(Token::WhiteSpace(" ".to_string())));
                    values.push(ComponentValue::Function(Function {
                        name: self.intern_string("format"),
                        url_resource: None,
                        arguments: ComponentValueList {
                            values: vec![ComponentValue::Token(Token::String(
                                self.font_format_name(format).to_string(),
                            ))],
                        },
                    }));
                }

                if !value.tech.is_empty() {
                    let arguments = value
                        .tech
                        .iter()
                        .enumerate()
                        .flat_map(|(index, technology)| {
                            let mut values = Vec::new();

                            if index > 0 {
                                values.push(ComponentValue::Token(Token::Symbol(Symbol::Comma)));
                                values.push(ComponentValue::Token(Token::WhiteSpace(
                                    " ".to_string(),
                                )));
                            }

                            values.push(ComponentValue::Token(Token::Ident(
                                self.intern_string(self.font_technology_name(technology)),
                            )));

                            values
                        })
                        .collect();

                    values.push(ComponentValue::Token(Token::WhiteSpace(" ".to_string())));
                    values.push(ComponentValue::Function(Function {
                        name: self.intern_string("tech"),
                        url_resource: None,
                        arguments: ComponentValueList { values: arguments },
                    }));
                }

                values
            }
            lightning::FontSource::Local(value) => vec![ComponentValue::Function(Function {
                name: self.intern_string("local"),
                url_resource: None,
                arguments: self.lower_font_family_components(value),
            })],
        }
    }

    /// Lower one font family into one declaration value.
    fn lower_font_family_declaration_value(
        &mut self,
        value: &lightning::FontFamily<'_>,
    ) -> DeclarationValue {
        DeclarationValue {
            components: self.lower_font_family_components(value),
        }
    }

    /// Lower one font family into one component value list.
    fn lower_font_family_components(
        &mut self,
        value: &lightning::FontFamily<'_>,
    ) -> ComponentValueList {
        match value {
            lightning::FontFamily::Generic(value) => ComponentValueList {
                values: vec![ComponentValue::Token(Token::Ident(
                    self.intern_string(self.generic_font_family_name(value)),
                ))],
            },
            lightning::FontFamily::FamilyName(_) => {
                self.lower_serialized_component_value_list(value)
            }
        }
    }

    /// Lower one `@font-face` font style range into one declaration value.
    fn lower_font_face_style_range(
        &mut self,
        value: &lightning::FontFaceStyle,
    ) -> DeclarationValue {
        let values = match value {
            lightning::FontFaceStyle::Normal => {
                vec![ComponentValue::Token(Token::Ident(
                    self.intern_string("normal"),
                ))]
            }
            lightning::FontFaceStyle::Italic => {
                vec![ComponentValue::Token(Token::Ident(
                    self.intern_string("italic"),
                ))]
            }
            lightning::FontFaceStyle::Oblique(angles) => {
                let mut values = vec![ComponentValue::Token(Token::Ident(
                    self.intern_string("oblique"),
                ))];

                if angles.0 != lightning::Angle::Deg(14.0)
                    || angles.1 != lightning::Angle::Deg(14.0)
                {
                    values.push(ComponentValue::Token(Token::WhiteSpace(" ".to_string())));
                    values.push(self.lower_angle_value(&angles.0));

                    if angles.1 != angles.0 {
                        values.push(ComponentValue::Token(Token::WhiteSpace(" ".to_string())));
                        values.push(self.lower_angle_value(&angles.1));
                    }
                }

                values
            }
        };

        DeclarationValue {
            components: ComponentValueList { values },
        }
    }

    /// Lower one `@font-face` font weight range into one declaration value.
    fn lower_font_weight_range(
        &mut self,
        value: &lightning::Size2D<lightning::FontWeight>,
    ) -> DeclarationValue {
        let mut values = vec![self.lower_font_weight_value(&value.0)];

        if value.1 != value.0 {
            values.push(ComponentValue::Token(Token::WhiteSpace(" ".to_string())));
            values.push(self.lower_font_weight_value(&value.1));
        }

        DeclarationValue {
            components: ComponentValueList { values },
        }
    }

    /// Lower one font weight into one component value.
    fn lower_font_weight_value(&self, value: &lightning::FontWeight) -> ComponentValue {
        match value {
            lightning::FontWeight::Absolute(value) => self.lower_absolute_font_weight(value),
            lightning::FontWeight::Bolder => {
                ComponentValue::Token(Token::Ident(self.intern_string("bolder")))
            }
            lightning::FontWeight::Lighter => {
                ComponentValue::Token(Token::Ident(self.intern_string("lighter")))
            }
        }
    }

    /// Lower one absolute font weight into one component value.
    fn lower_absolute_font_weight(&self, value: &lightning::AbsoluteFontWeight) -> ComponentValue {
        match value {
            lightning::AbsoluteFontWeight::Weight(value) => {
                ComponentValue::Token(Token::Number(Number {
                    has_sign: value.is_sign_negative(),
                    value: *value,
                    integer_value: (value.fract() == 0.0).then_some(*value as i32),
                }))
            }
            lightning::AbsoluteFontWeight::Normal => {
                ComponentValue::Token(Token::Ident(self.intern_string("normal")))
            }
            lightning::AbsoluteFontWeight::Bold => {
                ComponentValue::Token(Token::Ident(self.intern_string("bold")))
            }
        }
    }

    /// Lower one `@font-face` font stretch range into one declaration value.
    fn lower_font_stretch_range(
        &mut self,
        value: &lightning::Size2D<lightning::FontStretch>,
    ) -> DeclarationValue {
        let mut values = vec![self.lower_font_stretch_value(&value.0)];

        if value.1 != value.0 {
            values.push(ComponentValue::Token(Token::WhiteSpace(" ".to_string())));
            values.push(self.lower_font_stretch_value(&value.1));
        }

        DeclarationValue {
            components: ComponentValueList { values },
        }
    }

    /// Lower one font stretch into one component value.
    fn lower_font_stretch_value(&self, value: &lightning::FontStretch) -> ComponentValue {
        match value {
            lightning::FontStretch::Keyword(value) => ComponentValue::Token(Token::Ident(
                self.intern_string(self.font_stretch_keyword_name(value)),
            )),
            lightning::FontStretch::Percentage(value) => self.lower_percentage_component(value),
        }
    }

    /// Lower one base palette into one declaration value.
    fn lower_base_palette_declaration_value(
        &self,
        value: &lightning::BasePalette,
    ) -> DeclarationValue {
        let value = match value {
            lightning::BasePalette::Light => {
                ComponentValue::Token(Token::Ident(self.intern_string("light")))
            }
            lightning::BasePalette::Dark => {
                ComponentValue::Token(Token::Ident(self.intern_string("dark")))
            }
            lightning::BasePalette::Integer(value) => {
                ComponentValue::Token(Token::Number(Number {
                    has_sign: false,
                    value: *value as f32,
                    integer_value: Some(*value as i32),
                }))
            }
        };

        DeclarationValue {
            components: ComponentValueList {
                values: vec![value],
            },
        }
    }

    /// Lower one navigation descriptor into one declaration value.
    fn lower_navigation_declaration_value(
        &self,
        value: &lightning::Navigation,
    ) -> DeclarationValue {
        let value = match value {
            lightning::Navigation::None => "none",
            lightning::Navigation::Auto => "auto",
        };

        DeclarationValue {
            components: ComponentValueList {
                values: vec![ComponentValue::Token(Token::Ident(
                    self.intern_string(value),
                ))],
            },
        }
    }

    /// Lower one unicode range list into one declaration value.
    fn lower_unicode_range_list(&self, values: &[lightning::UnicodeRange]) -> DeclarationValue {
        let mut components = Vec::new();

        for (index, value) in values.iter().enumerate() {
            if index > 0 {
                components.push(ComponentValue::Token(Token::Symbol(Symbol::Comma)));
                components.push(ComponentValue::Token(Token::WhiteSpace(" ".to_string())));
            }

            components.push(ComponentValue::Token(Token::Ident(
                self.intern_string(&self.unicode_range_source(value)),
            )));
        }

        DeclarationValue {
            components: ComponentValueList { values: components },
        }
    }

    /// Lower one percentage into one component value.
    fn lower_percentage_component(&self, value: &lightning::Percentage) -> ComponentValue {
        ComponentValue::Token(Token::Percentage(self.lower_percentage_number(value.0)))
    }

    /// Return the canonical name of one generic font family.
    fn generic_font_family_name(&self, value: &lightning::GenericFontFamily) -> &'static str {
        match value {
            lightning::GenericFontFamily::Serif => "serif",
            lightning::GenericFontFamily::SansSerif => "sans-serif",
            lightning::GenericFontFamily::Cursive => "cursive",
            lightning::GenericFontFamily::Fantasy => "fantasy",
            lightning::GenericFontFamily::Monospace => "monospace",
            lightning::GenericFontFamily::SystemUI => "system-ui",
            lightning::GenericFontFamily::Emoji => "emoji",
            lightning::GenericFontFamily::Math => "math",
            lightning::GenericFontFamily::FangSong => "fangsong",
            lightning::GenericFontFamily::UISerif => "ui-serif",
            lightning::GenericFontFamily::UISansSerif => "ui-sans-serif",
            lightning::GenericFontFamily::UIMonospace => "ui-monospace",
            lightning::GenericFontFamily::UIRounded => "ui-rounded",
            lightning::GenericFontFamily::Initial => "initial",
            lightning::GenericFontFamily::Inherit => "inherit",
            lightning::GenericFontFamily::Unset => "unset",
            lightning::GenericFontFamily::Default => "default",
            lightning::GenericFontFamily::Revert => "revert",
            lightning::GenericFontFamily::RevertLayer => "revert-layer",
        }
    }

    /// Return the canonical name of one font format.
    fn font_format_name(&self, value: &lightning::FontFormat<'_>) -> String {
        match value {
            lightning::FontFormat::WOFF => "woff".to_string(),
            lightning::FontFormat::WOFF2 => "woff2".to_string(),
            lightning::FontFormat::TrueType => "truetype".to_string(),
            lightning::FontFormat::OpenType => "opentype".to_string(),
            lightning::FontFormat::EmbeddedOpenType => "embedded-opentype".to_string(),
            lightning::FontFormat::Collection => "collection".to_string(),
            lightning::FontFormat::SVG => "svg".to_string(),
            lightning::FontFormat::String(value) => value.as_ref().to_string(),
        }
    }

    /// Return the canonical name of one font technology.
    fn font_technology_name(&self, value: &lightning::FontTechnology) -> &'static str {
        match value {
            lightning::FontTechnology::FeaturesOpentype => "features-opentype",
            lightning::FontTechnology::FeaturesAat => "features-aat",
            lightning::FontTechnology::FeaturesGraphite => "features-graphite",
            lightning::FontTechnology::ColorCOLRv0 => "color-colrv0",
            lightning::FontTechnology::ColorCOLRv1 => "color-colrv1",
            lightning::FontTechnology::ColorSVG => "color-svg",
            lightning::FontTechnology::ColorSbix => "color-sbix",
            lightning::FontTechnology::ColorCBDT => "color-cbdt",
            lightning::FontTechnology::Variations => "variations",
            lightning::FontTechnology::Palettes => "palettes",
            lightning::FontTechnology::Incremental => "incremental",
        }
    }

    /// Return the canonical name of one font stretch keyword.
    fn font_stretch_keyword_name(&self, value: &lightning::FontStretchKeyword) -> &'static str {
        match value {
            lightning::FontStretchKeyword::UltraCondensed => "ultra-condensed",
            lightning::FontStretchKeyword::ExtraCondensed => "extra-condensed",
            lightning::FontStretchKeyword::Condensed => "condensed",
            lightning::FontStretchKeyword::SemiCondensed => "semi-condensed",
            lightning::FontStretchKeyword::Normal => "normal",
            lightning::FontStretchKeyword::SemiExpanded => "semi-expanded",
            lightning::FontStretchKeyword::Expanded => "expanded",
            lightning::FontStretchKeyword::ExtraExpanded => "extra-expanded",
            lightning::FontStretchKeyword::UltraExpanded => "ultra-expanded",
        }
    }

    /// Return the canonical source of one unicode range.
    fn unicode_range_source(&self, value: &lightning::UnicodeRange) -> String {
        if value.start != value.end {
            let mut shift = 24;
            let mut mask = 0xf << shift;

            while shift > 0 {
                let start_digit = value.start & mask;
                let end_digit = value.end & mask;

                if start_digit != end_digit {
                    break;
                }

                mask >>= 4;
                shift -= 4;
            }

            shift += 4;

            let remainder_mask = (1 << shift) - 1;
            let start_remainder = value.start & remainder_mask;
            let end_remainder = value.end & remainder_mask;

            if start_remainder == 0 && end_remainder == remainder_mask {
                let prefix = (value.start & !remainder_mask) >> shift;
                let mut source = if prefix == 0 {
                    "U+".to_string()
                } else {
                    format!("U+{prefix:X}")
                };

                for _ in 0..(shift / 4) {
                    source.push('?');
                }

                return source;
            }
        }

        if value.start == value.end {
            return format!("U+{:X}", value.start);
        }

        format!("U+{:X}-{:X}", value.start, value.end)
    }

    /// Lower one `none | <custom-ident>+` list into one declaration value.
    fn lower_none_or_custom_ident_list(
        &mut self,
        values: &lightning::NoneOrCustomIdentList<'_>,
    ) -> DeclarationValue {
        let values = match values {
            lightning::NoneOrCustomIdentList::None => vec![ComponentValue::Token(Token::Ident(
                self.intern_string("none"),
            ))],
            lightning::NoneOrCustomIdentList::Idents(values) => {
                self.lower_custom_ident_list_argument(values, false).values
            }
        };

        DeclarationValue {
            components: ComponentValueList { values },
        }
    }

    /// Lower one authored `@property` initial value from source.
    fn lower_property_rule_initial_value(&mut self, rule_span: Span) -> Option<DeclarationValue> {
        let (body_start, body_end) = self.rule_block_body_range(rule_span)?;
        let body_source = &self.source[body_start..body_end];
        let declaration = Parser::parse_declaration_block(body_source)
            .into_iter()
            .find(|declaration| declaration.name.eq_ignore_ascii_case("initial-value"))?;
        let (value, _) = self.lower_declaration_value_authored_source(declaration.value);

        Some(value)
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

    /// Lower one authored keyframe selector list source.
    fn lower_keyframe_selector_list_source(&self, source: &str) -> KeyframeSelectorList {
        KeyframeSelectorList {
            selectors: source
                .split(',')
                .map(|selector| self.lower_keyframe_selector_source(selector.trim()))
                .collect(),
        }
    }

    /// Lower one authored keyframe selector source.
    fn lower_keyframe_selector_source(&self, source: &str) -> KeyframeSelector {
        if source.eq_ignore_ascii_case("from") {
            return KeyframeSelector::From;
        }

        if source.eq_ignore_ascii_case("to") {
            return KeyframeSelector::To;
        }

        if source.ends_with('%') && !source.contains(char::is_whitespace) {
            let value = source[..source.len() - 1]
                .parse::<f32>()
                .unwrap_or_else(|error| {
                    panic!("failed to parse keyframe percentage {source}: {error}")
                });

            return KeyframeSelector::Percentage(self.lower_percentage_number(value / 100.0));
        }

        KeyframeSelector::TimelineRangePercentage(
            self.lower_timeline_range_percentage_source(source),
        )
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
    pub(crate) fn lower_declaration_value_source(&mut self, source: &str) -> DeclarationValue {
        DeclarationValue {
            components: self.lower_component_value_list_source(source),
        }
    }

    /// Lower one authored declaration value source and split one trailing `!important`.
    fn lower_declaration_value_authored_source(
        &mut self,
        source: &str,
    ) -> (DeclarationValue, bool) {
        let (components, is_important) = Parser::parse_declaration_value_with_pool(
            &self.tree.strings,
            &mut self.next_resource_id,
            source,
        );

        (DeclarationValue { components }, is_important)
    }

    /// Lower one Lightning property value into one owned declaration value.
    pub(crate) fn lower_declaration_value_property(
        &mut self,
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
                values.push(ComponentValue::Token(Token::Ident(
                    self.tree.intern(local_name),
                )));
            }
            lightning::SelectorComponent::AttributeInNoNamespace {
                local_name,
                operator,
                value,
                case_sensitivity,
                ..
            } => {
                values.push(ComponentValue::Token(Token::Ident(
                    self.tree.intern(local_name),
                )));
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
                    self.tree.intern(attribute.local_name.as_ref()),
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
                values.push(ComponentValue::Token(Token::Ident(
                    self.tree.intern(prefix),
                )));
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
                values.push(ComponentValue::Token(Token::Ident(self.tree.intern("i"))));
            }
            parcel::ParsedCaseSensitivity::ExplicitCaseSensitive => {
                values.push(ComponentValue::Token(Token::WhiteSpace(" ".to_string())));
                values.push(ComponentValue::Token(Token::Ident(self.tree.intern("s"))));
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
            lightning::PseudoClass::CustomFunction { name, arguments } => {
                let arguments = self.lower_component_value_token_list(arguments);

                PseudoClass {
                    name: name.to_string(),
                    arguments: Some(self.lower_component_pseudo_argument(arguments)),
                }
            }
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
            lightning::PseudoElement::ViewTransitionGroup { part } => {
                let arguments = self.lower_view_transition_part_argument(part);

                PseudoElement {
                    name: "view-transition-group".to_string(),
                    arguments: Some(self.lower_view_transition_pseudo_argument(arguments)),
                }
            }
            lightning::PseudoElement::ViewTransitionImagePair { part } => {
                let arguments = self.lower_view_transition_part_argument(part);

                PseudoElement {
                    name: "view-transition-image-pair".to_string(),
                    arguments: Some(self.lower_view_transition_pseudo_argument(arguments)),
                }
            }
            lightning::PseudoElement::ViewTransitionOld { part } => {
                let arguments = self.lower_view_transition_part_argument(part);

                PseudoElement {
                    name: "view-transition-old".to_string(),
                    arguments: Some(self.lower_view_transition_pseudo_argument(arguments)),
                }
            }
            lightning::PseudoElement::ViewTransitionNew { part } => {
                let arguments = self.lower_view_transition_part_argument(part);

                PseudoElement {
                    name: "view-transition-new".to_string(),
                    arguments: Some(self.lower_view_transition_pseudo_argument(arguments)),
                }
            }
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
            lightning::PseudoElement::CustomFunction { name, arguments } => {
                let arguments = self.lower_component_value_token_list(arguments);

                PseudoElement {
                    name: name.to_string(),
                    arguments: Some(self.lower_component_pseudo_argument(arguments)),
                }
            }
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
            values: vec![ComponentValue::Token(Token::Ident(
                self.tree.intern(identifier),
            ))],
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
                self.tree.intern(identifier.as_ref()),
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
        &mut self,
        part: &lightning::ViewTransitionPartSelector<'_>,
    ) -> ViewTransitionPartArgument {
        let source = self.serialize_value(part);

        self.parse_view_transition_part_argument(&source)
    }

    /// Wrap one generic pseudo argument payload.
    fn lower_component_pseudo_argument(&self, arguments: ComponentValueList) -> PseudoArgument {
        PseudoArgument::Components(arguments)
    }

    /// Wrap one view-transition part selector argument.
    fn lower_view_transition_pseudo_argument(
        &self,
        argument: ViewTransitionPartArgument,
    ) -> PseudoArgument {
        PseudoArgument::ViewTransitionPart(argument)
    }

    /// Lower one selector argument into component values.
    fn lower_selector_argument(&mut self, selector: &lightning::Selector<'_>) -> PseudoArgument {
        PseudoArgument::Selector(self.lower_selector(selector))
    }

    /// Parse one serialized view-transition part selector argument.
    fn parse_view_transition_part_argument(&self, source: &str) -> ViewTransitionPartArgument {
        let bytes = source.as_bytes();
        let mut index = 0;

        let name = if index < bytes.len() && bytes[index] != b'.' {
            let start = index;

            while index < bytes.len() && bytes[index] != b'.' {
                index += 1;
            }

            Some(source[start..index].to_string())
        } else {
            None
        };

        let mut classes = Vec::new();

        while index < bytes.len() {
            if bytes[index] != b'.' {
                panic!("invalid view-transition part selector: {source}");
            }

            index += 1;
            let start = index;

            while index < bytes.len() && bytes[index] != b'.' {
                index += 1;
            }

            if start == index {
                panic!("empty view-transition class in {source}");
            }

            classes.push(source[start..index].to_string());
        }

        if name.is_none() && classes.is_empty() {
            panic!("empty view-transition part selector: {source}");
        }

        ViewTransitionPartArgument { name, classes }
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
    pub(crate) fn lower_percentage_number(&self, value: f32) -> Number {
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
        value
            .to_css_string(lightning::PrinterOptions::default())
            .unwrap_or_default()
    }

    /// Return the serialized form of one CSS rule.
    fn serialize_rule(&self, rule: &lightning::CssRule<'a>) -> String {
        self.serialize_value(rule)
    }

    /// Return the source span for one CSS rule.
    fn span_for_rule(&self, rule: &lightning::CssRule<'a>) -> Span {
        let location = match rule {
            lightning::CssRule::Media(rule) => Some(rule.loc),
            lightning::CssRule::Import(rule) => Some(rule.loc),
            lightning::CssRule::Style(rule) => Some(rule.loc),
            lightning::CssRule::Keyframes(rule) => Some(rule.loc),
            lightning::CssRule::FontFace(rule) => Some(rule.loc),
            lightning::CssRule::FontPaletteValues(rule) => Some(rule.loc),
            lightning::CssRule::FontFeatureValues(rule) => Some(rule.loc),
            lightning::CssRule::Page(rule) => Some(rule.loc),
            lightning::CssRule::Supports(rule) => Some(rule.loc),
            lightning::CssRule::CounterStyle(rule) => Some(rule.loc),
            lightning::CssRule::Namespace(rule) => Some(rule.loc),
            lightning::CssRule::MozDocument(rule) => Some(rule.loc),
            lightning::CssRule::Nesting(rule) => Some(rule.loc),
            lightning::CssRule::NestedDeclarations(rule) => Some(rule.loc),
            lightning::CssRule::Viewport(rule) => Some(rule.loc),
            lightning::CssRule::CustomMedia(rule) => Some(rule.loc),
            lightning::CssRule::LayerStatement(rule) => Some(rule.loc),
            lightning::CssRule::LayerBlock(rule) => Some(rule.loc),
            lightning::CssRule::Property(rule) => Some(rule.loc),
            lightning::CssRule::Container(rule) => Some(rule.loc),
            lightning::CssRule::Scope(rule) => Some(rule.loc),
            lightning::CssRule::StartingStyle(rule) => Some(rule.loc),
            lightning::CssRule::ViewTransition(rule) => Some(rule.loc),
            lightning::CssRule::Unknown(rule) => Some(rule.loc),
            lightning::CssRule::Custom(_) | lightning::CssRule::Ignored => None,
        };

        let Some(location) = location else {
            return Span::empty(self.file.id);
        };

        self.span_for_location(location)
    }

    /// Return the source span for one CSS rule that starts at one Lightning location.
    fn span_for_location(&self, location: lightning::Location) -> Span {
        let start = self.location_to_offset(location.line as usize, location.column as usize);
        let end = self.rule_end_offset(start as usize) as u32;

        Span::new(self.file.id, start, end)
    }

    /// Return the prelude span for one CSS rule when it has one.
    fn prelude_span_for_rule(&self, rule_span: Span) -> Option<Span> {
        if rule_span.is_empty() {
            return None;
        }

        let bytes = self.source.as_bytes();
        let start = rule_span.start as usize;
        let end = rule_span.end as usize;
        let mut index = start;
        let mut state = ScanState::default();

        while index < end {
            let next = state.skip(bytes, index);

            if next != index {
                index = next;
                continue;
            }

            // top level prelude boundary
            if state.is_top_level() {
                match bytes[index] {
                    b'{' | b';' => {
                        let prelude_end = Self::trim_ascii_whitespace_end(bytes, start, index);

                        return Some(Span::new(
                            rule_span.file,
                            rule_span.start,
                            prelude_end as u32,
                        ));
                    }
                    _ => {}
                }
            }

            state.advance(bytes[index]);
            index += 1;
        }

        let prelude_end = Self::trim_ascii_whitespace_end(bytes, start, end);

        Some(Span::new(
            rule_span.file,
            rule_span.start,
            prelude_end as u32,
        ))
    }

    /// Return the authored span for one import url within one rule.
    fn import_url_span_for_rule(&self, rule_span: Span, url: &str) -> Option<Span> {
        if rule_span.is_empty() || url.is_empty() {
            return None;
        }

        let start = rule_span.start as usize;
        let end = rule_span.end as usize;
        let rule_source = self.source.get(start..end)?;
        let offset = rule_source.find(url)?;

        Some(Span::new(
            rule_span.file,
            (start + offset) as u32,
            (start + offset + url.len()) as u32,
        ))
    }

    /// Return the authored block body byte range for one braced rule.
    fn rule_block_body_range(&self, rule_span: Span) -> Option<(usize, usize)> {
        if rule_span.is_empty() {
            return None;
        }

        let bytes = self.source.as_bytes();
        let start = rule_span.start as usize;
        let end = rule_span.end as usize;
        let mut index = start;
        let mut state = ScanState::default();

        while index < end {
            let next = state.skip(bytes, index);

            if next != index {
                index = next;
                continue;
            }

            // top level block start
            if state.is_top_level() && bytes[index] == b'{' {
                let body_start = index + 1;
                state.brace_depth = 1;
                index += 1;

                while index < end {
                    let next = state.skip(bytes, index);

                    if next != index {
                        index = next;
                        continue;
                    }

                    // matching block end
                    if bytes[index] == b'}' && state.brace_depth == 1 {
                        let body_end = Self::trim_ascii_whitespace_end(bytes, body_start, index);

                        return Some((body_start, body_end));
                    }

                    state.advance(bytes[index]);
                    index += 1;
                }

                return None;
            }

            state.advance(bytes[index]);
            index += 1;
        }

        None
    }

    /// Convert one Lightning line and column pair into one byte offset.
    fn location_to_offset(&self, line: usize, column: usize) -> u32 {
        File::byte_offset_from_position(self.source, line + 1, column)
    }

    /// Return the exclusive byte end offset for one rule that starts at `start`.
    fn rule_end_offset(&self, start: usize) -> usize {
        let bytes = self.source.as_bytes();
        let mut index = start;
        let mut state = ScanState::default();

        while index < bytes.len() {
            let next = state.skip(bytes, index);

            if next != index {
                index = next;
                continue;
            }

            // top level rule boundary
            if state.is_top_level() {
                if bytes[index] == b';' {
                    return index + 1;
                }

                if bytes[index] == b'{' {
                    state.brace_depth += 1;
                    index += 1;

                    break;
                }
            }

            state.advance(bytes[index]);
            index += 1;
        }

        while index < bytes.len() {
            let next = state.skip(bytes, index);

            if next != index {
                index = next;
                continue;
            }

            if bytes[index] == b'}' && state.brace_depth == 1 {
                return index + 1;
            }

            state.advance(bytes[index]);
            index += 1;
        }

        bytes.len()
    }

    /// Trim one trailing ASCII whitespace run.
    fn trim_ascii_whitespace_end(bytes: &[u8], start: usize, mut end: usize) -> usize {
        while end > start && bytes[end - 1].is_ascii_whitespace() {
            end -= 1;
        }

        end
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

/// One scan state for authored CSS source ranges.
#[derive(Debug, Default, Clone, Copy)]
struct ScanState {
    /// The parenthesis nesting depth.
    parenthesis_depth: usize,
    /// The bracket nesting depth.
    bracket_depth: usize,
    /// The brace nesting depth.
    brace_depth: usize,
    /// The current string delimiter.
    string_delimiter: Option<u8>,
}

impl ScanState {
    /// Return whether the scanner is at top level.
    fn is_top_level(&self) -> bool {
        self.parenthesis_depth == 0 && self.bracket_depth == 0 && self.brace_depth == 0
    }

    /// Advance this state by one ordinary byte.
    fn advance(&mut self, byte: u8) {
        match byte {
            b'(' => self.parenthesis_depth += 1,
            b')' => self.parenthesis_depth = self.parenthesis_depth.saturating_sub(1),
            b'[' => self.bracket_depth += 1,
            b']' => self.bracket_depth = self.bracket_depth.saturating_sub(1),
            b'{' => self.brace_depth += 1,
            b'}' => self.brace_depth = self.brace_depth.saturating_sub(1),
            b'\'' | b'"' => self.string_delimiter = Some(byte),
            _ => {}
        }
    }

    /// Skip comments and string bodies.
    fn skip(&mut self, bytes: &[u8], index: usize) -> usize {
        if let Some(delimiter) = self.string_delimiter {
            let mut index = index;

            while index < bytes.len() {
                if bytes[index] == b'\\' {
                    index += 2;
                    continue;
                }

                index += 1;

                if bytes[index - 1] == delimiter {
                    self.string_delimiter = None;
                    break;
                }
            }

            return index;
        }

        if bytes.get(index) == Some(&b'/') && bytes.get(index + 1) == Some(&b'*') {
            let mut index = index + 2;

            while index + 1 < bytes.len() {
                if bytes[index] == b'*' && bytes[index + 1] == b'/' {
                    return index + 2;
                }

                index += 1;
            }

            return bytes.len();
        }

        index
    }
}
