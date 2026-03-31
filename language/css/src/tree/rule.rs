use crate::{
    ComponentValueList, ContainerCondition, ContainerName, DeclarationBlock, DeclarationValue,
    ImportLayer, KeyframeSelectorList, LayerNameList, LocalNodeId, MediaQueryList, Node, NodeType,
    PageMarginBox, PageSelectorList, SelectorList, SupportsCondition,
};
use serde::{Deserialize, Serialize};

/// One CSS stylesheet node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stylesheet {
    /// The source filenames attached to the stylesheet.
    pub sources: Vec<String>,
    /// The extracted license comments.
    pub license_comments: Vec<String>,
    /// The top level rules.
    pub rules: Vec<LocalNodeId<Rule>>,
}

impl Node for Stylesheet {
    const TYPE: NodeType = NodeType::Stylesheet;
}

/// One CSS rule node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rule {
    /// One `@import` rule.
    Import(ImportRule),
    /// One style rule.
    Style(StyleRule),
    /// One `@media` rule.
    Media(MediaRule),
    /// One `@supports` rule.
    Supports(SupportsRule),
    /// One `@layer` block rule.
    LayerBlock(LayerBlockRule),
    /// One `@layer` statement rule.
    LayerStatement(LayerStatementRule),
    /// One `@container` rule.
    Container(ContainerRule),
    /// One `@scope` rule.
    Scope(ScopeRule),
    /// One `@starting-style` rule.
    StartingStyle(StartingStyleRule),
    /// One `@page` rule.
    Page(PageRule),
    /// One `@font-face` rule.
    FontFace(FontFaceRule),
    /// One `@font-palette-values` rule.
    FontPaletteValues(FontPaletteValuesRule),
    /// One `@font-feature-values` rule.
    FontFeatureValues(FontFeatureValuesRule),
    /// One `@counter-style` rule.
    CounterStyle(CounterStyleRule),
    /// One `@namespace` rule.
    Namespace(NamespaceRule),
    /// One `@-moz-document` rule.
    MozDocument(MozDocumentRule),
    /// One nesting rule.
    Nesting(NestingRule),
    /// One nested declarations rule.
    NestedDeclarations(NestedDeclarationsRule),
    /// One `@viewport` rule.
    Viewport(ViewportRule),
    /// One `@custom-media` rule.
    CustomMedia(CustomMediaRule),
    /// One `@property` rule.
    Property(PropertyRule),
    /// One `@keyframes` rule.
    Keyframes(KeyframesRule),
    /// One keyframe block rule.
    Keyframe(KeyframeRule),
    /// One `@view-transition` rule.
    ViewTransition(ViewTransitionRule),
    /// One unknown at-rule.
    Unknown(UnknownRule),
    /// One custom rule.
    Custom(CustomRule),
    /// One ignored rule placeholder.
    Ignored(IgnoredRule),
}

impl Node for Rule {
    const TYPE: NodeType = NodeType::Rule;
}

/// One `@media` rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaRule {
    /// The media query list.
    pub query: LocalNodeId<MediaQueryList>,
    /// The nested rules.
    pub rules: Vec<LocalNodeId<Rule>>,
}

/// One `@supports` rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupportsRule {
    /// The supports condition.
    pub condition: LocalNodeId<SupportsCondition>,
    /// The nested rules.
    pub rules: Vec<LocalNodeId<Rule>>,
}

/// One `@layer` block rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerBlockRule {
    /// The optional layer name.
    pub name: Option<LayerNameList>,
    /// The nested rules.
    pub rules: Vec<LocalNodeId<Rule>>,
}

/// One `@container` rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerRule {
    /// The optional container name.
    pub name: Option<ContainerName>,
    /// The optional container condition.
    pub condition: Option<LocalNodeId<ContainerCondition>>,
    /// The nested rules.
    pub rules: Vec<LocalNodeId<Rule>>,
}

/// One `@scope` rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeRule {
    /// The optional scope start selector list.
    pub scope_start: Option<LocalNodeId<SelectorList>>,
    /// The optional scope end selector list.
    pub scope_end: Option<LocalNodeId<SelectorList>>,
    /// The nested rules.
    pub rules: Vec<LocalNodeId<Rule>>,
}

/// One `@starting-style` rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartingStyleRule {
    /// The nested rules.
    pub rules: Vec<LocalNodeId<Rule>>,
}

/// One `@keyframes` rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyframesRule {
    /// The animation name.
    pub name: KeyframesName,
    /// The vendor prefix.
    pub vendor_prefix: VendorPrefix,
    /// The nested keyframe rules.
    pub rules: Vec<LocalNodeId<Rule>>,
}

/// One `@-moz-document` rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MozDocumentRule {
    /// The nested rules.
    pub rules: Vec<LocalNodeId<Rule>>,
}

/// One `@layer` statement rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerStatementRule {
    /// The declared layer names.
    pub names: Vec<LayerNameList>,
}

/// One `@font-feature-values` rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FontFeatureValuesRule {
    /// The font family names.
    pub families: Vec<FontFeatureFamilyName>,
    /// The nested font feature subrules.
    pub subrules: Vec<FontFeatureSubrule>,
}

/// One `@namespace` rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamespaceRule {
    /// The optional namespace prefix.
    pub prefix: Option<NamespacePrefix>,
    /// The namespace url.
    pub url: NamespaceUrl,
}

/// One `@custom-media` rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomMediaRule {
    /// The declared custom media name.
    pub name: CustomMediaName,
    /// The declared media query.
    pub query: LocalNodeId<MediaQueryList>,
}

/// One `@property` rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyRule {
    /// The custom property name.
    pub name: CustomPropertyName,
    /// The property syntax definition.
    pub syntax: PropertySyntax,
    /// Whether the custom property inherits.
    pub inherits: bool,
    /// The optional initial value.
    pub initial_value: Option<DeclarationValue>,
}

/// One unknown at-rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnknownRule {
    /// The at-rule name without the `@`.
    pub name: String,
    /// The authored prelude component values.
    pub prelude: ComponentValueList,
    /// The optional authored block component values.
    pub block: Option<ComponentValueList>,
}

/// One custom rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomRule {
    /// The full authored rule components.
    pub components: ComponentValueList,
}

/// One nesting rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NestingRule {
    /// The selector or nesting prelude before the block.
    pub prelude: LocalNodeId<SelectorList>,
    /// The declarations within the rule.
    pub declarations: Option<LocalNodeId<DeclarationBlock>>,
    /// The nested rules.
    pub rules: Vec<LocalNodeId<Rule>>,
}

/// One `@font-face` rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FontFaceRule {
    /// The declarations within the rule.
    pub declarations: Option<LocalNodeId<DeclarationBlock>>,
}

/// One `@font-palette-values` rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FontPaletteValuesRule {
    /// The declared palette name.
    pub name: FontPaletteName,
    /// The declarations within the rule.
    pub declarations: Option<LocalNodeId<DeclarationBlock>>,
}

/// One `@counter-style` rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CounterStyleRule {
    /// The declared counter style name.
    pub name: CounterStyleName,
    /// The declarations within the rule.
    pub declarations: Option<LocalNodeId<DeclarationBlock>>,
}

/// One `@viewport` rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewportRule {
    /// The vendor prefix for the rule.
    pub vendor_prefix: VendorPrefix,
    /// The declarations within the rule.
    pub declarations: Option<LocalNodeId<DeclarationBlock>>,
}

/// One `@view-transition` rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewTransitionRule {
    /// The declarations within the rule.
    pub declarations: Option<LocalNodeId<DeclarationBlock>>,
}

/// One authored CSS keyframes name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyframesName {
    /// The keyframes name.
    pub name: String,
}

/// One authored CSS vendor prefix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VendorPrefix {
    /// No prefix.
    None,
    /// One `-webkit-` prefix.
    Webkit,
    /// One `-moz-` prefix.
    Moz,
    /// One `-ms-` prefix.
    Ms,
    /// One `-o-` prefix.
    O,
    /// One non-standard prefix.
    Other(String),
}

/// One authored CSS namespace prefix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamespacePrefix {
    /// The namespace prefix.
    pub name: String,
}

/// One authored CSS namespace url.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NamespaceUrl {
    /// One string namespace URL.
    String(String),
    /// One `url(...)` namespace URL.
    Url(String),
}

/// One authored CSS custom media name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomMediaName {
    /// The custom media name.
    pub name: String,
}

/// One authored CSS custom property name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomPropertyName {
    /// The custom property name.
    pub name: String,
}

/// One authored CSS property syntax definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropertySyntax {
    /// The universal syntax definition.
    Universal,
    /// The authored syntax components.
    Components(Vec<PropertySyntaxComponent>),
}

impl PropertySyntax {}

/// One authored CSS property syntax component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertySyntaxComponent {
    /// The component kind.
    pub kind: PropertySyntaxComponentKind,
    /// The repetition multiplier.
    pub multiplier: PropertySyntaxMultiplier,
}

impl PropertySyntaxComponent {}

/// One authored CSS property syntax component kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropertySyntaxComponentKind {
    /// One `<length>` component.
    Length,
    /// One `<number>` component.
    Number,
    /// One `<percentage>` component.
    Percentage,
    /// One `<length-percentage>` component.
    LengthPercentage,
    /// One `<string>` component.
    String,
    /// One `<color>` component.
    Color,
    /// One `<image>` component.
    Image,
    /// One `<url>` component.
    Url,
    /// One `<integer>` component.
    Integer,
    /// One `<angle>` component.
    Angle,
    /// One `<time>` component.
    Time,
    /// One `<resolution>` component.
    Resolution,
    /// One `<transform-function>` component.
    TransformFunction,
    /// One `<transform-list>` component.
    TransformList,
    /// One `<custom-ident>` component.
    CustomIdent,
    /// One literal component.
    Literal(String),
}

impl PropertySyntaxComponentKind {}

/// One authored CSS property syntax multiplier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropertySyntaxMultiplier {
    /// One non-repeated component.
    None,
    /// One space-separated repeated component.
    Space,
    /// One comma-separated repeated component.
    Comma,
}

impl PropertySyntaxMultiplier {}

/// One authored CSS font palette name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FontPaletteName {
    /// The palette name.
    pub name: String,
}

/// One authored CSS counter style name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CounterStyleName {
    /// The counter style name.
    pub name: String,
}

/// One authored `@font-feature-values` family name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FontFeatureFamilyName {
    /// The family name.
    pub name: String,
}

/// One authored `@font-feature-values` subrule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FontFeatureSubrule {
    /// The subrule kind.
    pub kind: FontFeatureSubruleKind,
    /// The declarations within the subrule.
    pub declarations: Vec<FontFeatureDeclaration>,
}

/// One authored `@font-feature-values` subrule kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontFeatureSubruleKind {
    /// One `@stylistic` subrule.
    Stylistic,
    /// One `@historical-forms` subrule.
    HistoricalForms,
    /// One `@styleset` subrule.
    Styleset,
    /// One `@character-variant` subrule.
    CharacterVariant,
    /// One `@swash` subrule.
    Swash,
    /// One `@ornaments` subrule.
    Ornaments,
    /// One `@annotation` subrule.
    Annotation,
}

/// One authored `@font-feature-values` declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FontFeatureDeclaration {
    /// The declaration name.
    pub name: String,
    /// The declared feature indices.
    pub values: Vec<i32>,
}

/// One CSS page margin rule node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageMarginRule {
    /// The page margin box identifier.
    pub margin_box: PageMarginBox,
    /// The declarations within the rule.
    pub declarations: Option<LocalNodeId<DeclarationBlock>>,
}

impl Node for PageMarginRule {
    const TYPE: NodeType = NodeType::PageMarginRule;
}

/// One ignored rule placeholder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IgnoredRule {}

/// One `@import` rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportRule {
    /// The authored import url without surrounding quotes.
    pub url: String,
    /// The optional layer clause.
    pub layer: Option<ImportLayer>,
    /// The optional supports clause.
    pub supports: Option<LocalNodeId<SupportsCondition>>,
    /// The optional media query list.
    pub media: Option<LocalNodeId<MediaQueryList>>,
}

/// One style like rule with declarations and nested rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyleRule {
    /// The selector or nesting prelude before the block.
    pub prelude: LocalNodeId<SelectorList>,
    /// The declarations within the rule.
    pub declarations: Option<LocalNodeId<DeclarationBlock>>,
    /// The nested rules.
    pub rules: Vec<LocalNodeId<Rule>>,
}

/// One keyframe block rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyframeRule {
    /// The keyframe selectors.
    pub selectors: KeyframeSelectorList,
    /// The declarations within the keyframe block.
    pub declarations: Option<LocalNodeId<DeclarationBlock>>,
}

/// One nested declarations rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NestedDeclarationsRule {
    /// The declarations within the nested declarations rule.
    pub declarations: Option<LocalNodeId<DeclarationBlock>>,
}

/// One `@page` rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageRule {
    /// The page selectors.
    pub selectors: PageSelectorList,
    /// The declarations within the rule.
    pub declarations: Option<LocalNodeId<DeclarationBlock>>,
    /// The nested page margin rules.
    pub page_margin_rules: Vec<LocalNodeId<PageMarginRule>>,
}
