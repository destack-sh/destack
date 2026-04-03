use crate::{ComponentValueList, LocalNodeId, Node, NodeType, Number, VendorPrefix};
use serde::{Deserialize, Serialize};

/// One authored CSS selector list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectorList {
    /// The selectors in authored order.
    pub selectors: Vec<LocalNodeId<Selector>>,
}

impl Node for SelectorList {
    const TYPE: NodeType = NodeType::SelectorList;
}

/// One authored CSS selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selector {
    /// The selector components in parse order.
    pub components: Vec<LocalNodeId<SelectorComponent>>,
}

impl Node for Selector {
    const TYPE: NodeType = NodeType::Selector;
}

/// One authored CSS selector component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectorComponent {
    /// One combinator between selector segments.
    Combinator(Combinator),
    /// One simple selector component.
    Simple(LocalNodeId<SimpleSelector>),
}

impl Node for SelectorComponent {
    const TYPE: NodeType = NodeType::SelectorComponent;
}

/// One CSS selector combinator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Combinator {
    /// One child combinator.
    Child,
    /// One descendant combinator.
    Descendant,
    /// One next sibling combinator.
    NextSibling,
    /// One later sibling combinator.
    LaterSibling,
    /// One pseudo-element boundary combinator.
    PseudoElement,
    /// One slot assignment combinator.
    SlotAssignment,
    /// One part boundary combinator.
    Part,
    /// One deep descendant combinator.
    DeepDescendant,
    /// One deep combinator.
    Deep,
}

impl Combinator {}

/// One CSS simple selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimpleSelector {
    /// One explicit any namespace marker.
    ExplicitAnyNamespace,
    /// One explicit no namespace marker.
    ExplicitNoNamespace,
    /// One default namespace marker.
    DefaultNamespace,
    /// One named namespace marker.
    Namespace(String),
    /// One explicit universal type selector.
    ExplicitUniversalType,
    /// One local name selector.
    LocalName(LocalName),
    /// One id selector.
    Id(String),
    /// One class selector.
    Class(String),
    /// One attribute selector.
    Attribute(LocalNodeId<AttributeSelector>),
    /// One `:not(...)` selector.
    Negation(LocalNodeId<SelectorList>),
    /// One `:root` selector.
    Root,
    /// One `:empty` selector.
    Empty,
    /// One `:scope` selector.
    Scope,
    /// One `:nth-*` selector without `of`.
    Nth(LocalNodeId<NthSelector>),
    /// One `:nth-*` selector with `of`.
    NthOf(LocalNodeId<NthOfSelector>),
    /// One non-TS pseudo class.
    PseudoClass(LocalNodeId<PseudoClass>),
    /// One `::slotted(...)` selector.
    Slotted(LocalNodeId<Selector>),
    /// One `::part(...)` selector.
    Part(Vec<String>),
    /// One `:host(...)` selector.
    Host(Option<LocalNodeId<Selector>>),
    /// One `:where(...)` selector.
    Where(LocalNodeId<SelectorList>),
    /// One `:is(...)` selector.
    Is(LocalNodeId<SelectorList>),
    /// One vendor `:any(...)` selector.
    Any(LocalNodeId<AnySelector>),
    /// One `:has(...)` selector.
    Has(LocalNodeId<SelectorList>),
    /// One pseudo element.
    PseudoElement(LocalNodeId<PseudoElement>),
    /// One nesting selector.
    Nesting,
}

impl Node for SimpleSelector {
    const TYPE: NodeType = NodeType::SimpleSelector;
}

/// One authored local-name selector payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalName {
    /// The authored name.
    pub name: String,
    /// The lowercase matching name.
    pub lower_name: String,
}

/// One authored attribute selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttributeSelector {
    /// The attribute selector contents without brackets.
    pub components: ComponentValueList,
}

impl Node for AttributeSelector {
    const TYPE: NodeType = NodeType::AttributeSelector;
}

/// One authored CSS nth selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NthSelector {
    /// The nth selector kind.
    pub kind: NthSelectorKind,
    /// Whether this is an authored function form.
    pub is_function: bool,
    /// The affine `a` coefficient.
    pub a: i32,
    /// The affine `b` constant.
    pub b: i32,
}

impl Node for NthSelector {
    const TYPE: NodeType = NodeType::NthSelector;
}

/// One authored CSS nth selector with `of`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NthOfSelector {
    /// The nth selector data.
    pub nth: LocalNodeId<NthSelector>,
    /// The trailing selector list after `of`.
    pub selectors: LocalNodeId<SelectorList>,
}

impl Node for NthOfSelector {
    const TYPE: NodeType = NodeType::NthOfSelector;
}

/// One authored CSS nth selector kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NthSelectorKind {
    /// The `:nth-child` family.
    Child,
    /// The `:nth-last-child` family.
    LastChild,
    /// The `:nth-of-type` family.
    OfType,
    /// The `:nth-last-of-type` family.
    LastOfType,
    /// The `:only-child` family.
    OnlyChild,
    /// The `:only-of-type` family.
    OnlyOfType,
    /// The `:nth-col` family.
    Column,
    /// The `:nth-last-col` family.
    LastColumn,
}

/// One authored CSS pseudo class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PseudoClass {
    /// The pseudo class name without `:`.
    pub name: String,
    /// The optional pseudo class arguments.
    pub arguments: Option<PseudoArgument>,
}

impl Node for PseudoClass {
    const TYPE: NodeType = NodeType::PseudoClass;
}

/// One authored CSS vendor `:any(...)` selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnySelector {
    /// The vendor prefix for the selector.
    pub vendor_prefix: VendorPrefix,
    /// The selector list arguments.
    pub selectors: LocalNodeId<SelectorList>,
}

impl Node for AnySelector {
    const TYPE: NodeType = NodeType::AnySelector;
}

/// One authored CSS pseudo element.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PseudoElement {
    /// The pseudo element name without `::`.
    pub name: String,
    /// The optional pseudo element arguments.
    pub arguments: Option<PseudoArgument>,
}

impl Node for PseudoElement {
    const TYPE: NodeType = NodeType::PseudoElement;
}

/// One authored pseudo selector argument payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PseudoArgument {
    /// One generic component value argument list.
    Components(ComponentValueList),
    /// One nested selector argument.
    Selector(LocalNodeId<Selector>),
    /// One view-transition part selector argument.
    ViewTransitionPart(ViewTransitionPartArgument),
}

/// One authored view-transition part selector argument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewTransitionPartArgument {
    /// The optional part name.
    pub name: Option<String>,
    /// The class names in authored order.
    pub classes: Vec<String>,
}

/// One authored CSS keyframe selector list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyframeSelectorList {
    /// The selectors in authored order.
    pub selectors: Vec<KeyframeSelector>,
}

impl KeyframeSelectorList {}

/// One authored CSS keyframe selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyframeSelector {
    /// One percentage selector.
    Percentage(Number),
    /// The `from` selector.
    From,
    /// The `to` selector.
    To,
    /// One timeline range percentage selector.
    TimelineRangePercentage(TimelineRangePercentage),
}

/// One authored CSS timeline range percentage selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineRangePercentage {
    /// The timeline range name.
    pub name: TimelineRangeName,
    /// The timeline range percentage.
    pub percentage: Number,
}

/// One authored CSS timeline range name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimelineRangeName {
    /// The `cover` range.
    Cover,
    /// The `contain` range.
    Contain,
    /// The `entry` range.
    Entry,
    /// The `exit` range.
    Exit,
    /// The `entry-crossing` range.
    EntryCrossing,
    /// The `exit-crossing` range.
    ExitCrossing,
}

/// One authored CSS page selector list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageSelectorList {
    /// The selectors in authored order.
    pub selectors: Vec<PageSelector>,
}

impl PageSelectorList {}

/// One authored CSS page selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageSelector {
    /// The optional named page.
    pub name: Option<String>,
    /// The page pseudo classes.
    pub pseudo_classes: Vec<PagePseudoClass>,
}

/// One authored CSS page pseudo class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PagePseudoClass {
    /// The `:left` pseudo class.
    Left,
    /// The `:right` pseudo class.
    Right,
    /// The `:first` pseudo class.
    First,
    /// The `:last` pseudo class.
    Last,
    /// The `:blank` pseudo class.
    Blank,
}

/// One authored CSS page margin box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PageMarginBox {
    /// The `@top-left-corner` margin box.
    TopLeftCorner,
    /// The `@top-left` margin box.
    TopLeft,
    /// The `@top-center` margin box.
    TopCenter,
    /// The `@top-right` margin box.
    TopRight,
    /// The `@top-right-corner` margin box.
    TopRightCorner,
    /// The `@left-top` margin box.
    LeftTop,
    /// The `@left-middle` margin box.
    LeftMiddle,
    /// The `@left-bottom` margin box.
    LeftBottom,
    /// The `@right-top` margin box.
    RightTop,
    /// The `@right-middle` margin box.
    RightMiddle,
    /// The `@right-bottom` margin box.
    RightBottom,
    /// The `@bottom-left-corner` margin box.
    BottomLeftCorner,
    /// The `@bottom-left` margin box.
    BottomLeft,
    /// The `@bottom-center` margin box.
    BottomCenter,
    /// The `@bottom-right` margin box.
    BottomRight,
    /// The `@bottom-right-corner` margin box.
    BottomRightCorner,
}
