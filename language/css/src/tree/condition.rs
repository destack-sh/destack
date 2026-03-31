use crate::{
    ComponentValueList, DeclarationValue, LocalNodeId, Node, NodeType, Number, PropertyName,
    SelectorList,
};
use serde::{Deserialize, Serialize};

/// One authored CSS media query list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaQueryList {
    /// The media queries in authored order.
    pub queries: Vec<LocalNodeId<MediaQuery>>,
}

impl Node for MediaQueryList {
    const TYPE: NodeType = NodeType::MediaQueryList;
}

/// One authored CSS media query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaQuery {
    /// The optional qualifier.
    pub qualifier: Option<MediaQualifier>,
    /// The media type.
    pub media_type: MediaType,
    /// The optional condition.
    pub condition: Option<LocalNodeId<MediaCondition>>,
}

impl Node for MediaQuery {
    const TYPE: NodeType = NodeType::MediaQuery;
}

/// One authored CSS media qualifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaQualifier {
    /// One `only` qualifier.
    Only,
    /// One `not` qualifier.
    Not,
}

/// One authored CSS media type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaType {
    /// The `all` media type.
    All,
    /// The `print` media type.
    Print,
    /// The `screen` media type.
    Screen,
    /// One custom media type.
    Custom(String),
}

/// One authored CSS media condition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaCondition {
    /// One media feature condition.
    Feature(LocalNodeId<QueryFeature>),
    /// One negated media condition.
    Not(LocalNodeId<MediaCondition>),
    /// One boolean operation over media conditions.
    Operation {
        /// The boolean operator.
        operator: ConditionOperator,
        /// The child conditions.
        conditions: Vec<LocalNodeId<MediaCondition>>,
    },
    /// One unknown media condition.
    Unknown(MediaUnknownCondition),
}

impl Node for MediaCondition {
    const TYPE: NodeType = NodeType::MediaCondition;
}

/// One authored CSS supports condition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupportsCondition {
    /// One negated supports condition.
    Not(LocalNodeId<SupportsCondition>),
    /// One conjunction of supports conditions.
    And(Vec<LocalNodeId<SupportsCondition>>),
    /// One disjunction of supports conditions.
    Or(Vec<LocalNodeId<SupportsCondition>>),
    /// One declaration supports condition.
    Declaration {
        /// The property name.
        property: PropertyName,
        /// The declaration value.
        value: DeclarationValue,
    },
    /// One selector supports condition.
    Selector(SupportsSelectorCondition),
    /// One unknown supports condition.
    Unknown(SupportsUnknownCondition),
}

impl Node for SupportsCondition {
    const TYPE: NodeType = NodeType::SupportsCondition;
}

/// One authored CSS layer name list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerNameList {
    /// The dotted layer name segments.
    pub names: Vec<String>,
}

/// One authored CSS import layer clause.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportLayer {
    /// The optional named layer.
    pub name: Option<LayerNameList>,
}

/// One authored CSS container name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerName {
    /// The container name.
    pub name: String,
}

/// One authored boolean operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConditionOperator {
    /// One `and` operator.
    And,
    /// One `or` operator.
    Or,
}

/// One authored CSS feature comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeatureComparison {
    /// One `=` comparison.
    Equal,
    /// One `>` comparison.
    GreaterThan,
    /// One `>=` comparison.
    GreaterThanEqual,
    /// One `<` comparison.
    LessThan,
    /// One `<=` comparison.
    LessThanEqual,
}

/// One authored CSS feature name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeatureName {
    /// One standard feature name.
    Standard(String),
    /// One custom feature name.
    Custom(String),
    /// One unknown feature name.
    Unknown(String),
}

impl Node for FeatureName {
    const TYPE: NodeType = NodeType::FeatureName;
}

/// One authored CSS feature value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeatureValue {
    /// One length value.
    Length(DeclarationValue),
    /// One number.
    Number(Number),
    /// One integer.
    Integer(i32),
    /// One boolean.
    Boolean(bool),
    /// One resolution value.
    Resolution(DeclarationValue),
    /// One ratio.
    Ratio(LocalNodeId<RatioValue>),
    /// One identifier.
    Ident(String),
    /// One environment variable reference.
    EnvironmentVariable(LocalNodeId<EnvironmentVariable>),
}

impl Node for FeatureValue {
    const TYPE: NodeType = NodeType::FeatureValue;
}

/// One authored CSS ratio value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RatioValue {
    /// The numerator.
    pub numerator: Number,
    /// The denominator.
    pub denominator: Number,
}

impl Node for RatioValue {
    const TYPE: NodeType = NodeType::RatioValue;
}

/// One authored CSS environment variable reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvironmentVariable {
    /// The environment variable name.
    pub name: EnvironmentVariableName,
    /// The optional dimension indices.
    pub indices: Vec<i32>,
    /// The optional fallback value.
    pub fallback: Option<ComponentValueList>,
}

impl Node for EnvironmentVariable {
    const TYPE: NodeType = NodeType::EnvironmentVariable;
}

/// One authored CSS environment variable name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnvironmentVariableName {
    /// One UA defined environment variable.
    Ua(String),
    /// One custom environment variable.
    Custom(String),
    /// One unknown environment variable.
    Unknown(String),
}

/// One generic authored CSS query feature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryFeature {
    /// One plain feature query.
    Plain {
        /// The feature name.
        name: LocalNodeId<FeatureName>,
        /// The feature value.
        value: LocalNodeId<FeatureValue>,
    },
    /// One boolean feature query.
    Boolean {
        /// The feature name.
        name: LocalNodeId<FeatureName>,
    },
    /// One range feature query.
    Range {
        /// The feature name.
        name: LocalNodeId<FeatureName>,
        /// The feature comparison.
        operator: FeatureComparison,
        /// The feature value.
        value: LocalNodeId<FeatureValue>,
    },
    /// One interval feature query.
    Interval {
        /// The feature name.
        name: LocalNodeId<FeatureName>,
        /// The interval start value.
        start: LocalNodeId<FeatureValue>,
        /// The interval start comparison.
        start_operator: FeatureComparison,
        /// The interval end value.
        end: LocalNodeId<FeatureValue>,
        /// The interval end comparison.
        end_operator: FeatureComparison,
    },
}

impl Node for QueryFeature {
    const TYPE: NodeType = NodeType::QueryFeature;
}

/// One authored unknown media condition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaUnknownCondition {
    /// The unknown condition component values.
    pub components: ComponentValueList,
}

/// One authored selector supports condition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupportsSelectorCondition {
    /// The selector list to evaluate.
    pub selectors: LocalNodeId<SelectorList>,
}

/// One authored unknown supports condition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupportsUnknownCondition {
    /// The unknown condition component values.
    pub components: ComponentValueList,
}

/// One authored CSS container condition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerCondition {
    /// One container feature condition.
    Feature(LocalNodeId<QueryFeature>),
    /// One negated container condition.
    Not(LocalNodeId<ContainerCondition>),
    /// One boolean operation over container conditions.
    Operation {
        /// The boolean operator.
        operator: ConditionOperator,
        /// The child conditions.
        conditions: Vec<LocalNodeId<ContainerCondition>>,
    },
    /// One style query.
    Style(LocalNodeId<ContainerStyleQuery>),
    /// One scroll state query.
    ScrollState(LocalNodeId<ContainerScrollStateQuery>),
    /// One unknown container condition.
    Unknown(ContainerUnknownCondition),
}

impl Node for ContainerCondition {
    const TYPE: NodeType = NodeType::ContainerCondition;
}

/// One authored CSS container style query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerStyleQuery {
    /// One property declaration query.
    Declaration {
        /// The property name.
        property: PropertyName,
        /// The declaration value.
        value: DeclarationValue,
    },
    /// One property existence query.
    Property(PropertyName),
    /// One negated style query.
    Not(LocalNodeId<ContainerStyleQuery>),
    /// One boolean operation over style queries.
    Operation {
        /// The boolean operator.
        operator: ConditionOperator,
        /// The child queries.
        conditions: Vec<LocalNodeId<ContainerStyleQuery>>,
    },
}

impl Node for ContainerStyleQuery {
    const TYPE: NodeType = NodeType::ContainerStyleQuery;
}

/// One authored CSS scroll state query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerScrollStateQuery {
    /// One scroll state feature query.
    Feature(LocalNodeId<QueryFeature>),
    /// One negated scroll state query.
    Not(LocalNodeId<ContainerScrollStateQuery>),
    /// One boolean operation over scroll state queries.
    Operation {
        /// The boolean operator.
        operator: ConditionOperator,
        /// The child queries.
        conditions: Vec<LocalNodeId<ContainerScrollStateQuery>>,
    },
}

impl Node for ContainerScrollStateQuery {
    const TYPE: NodeType = NodeType::ContainerScrollStateQuery;
}

/// One authored unknown container condition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerUnknownCondition {
    /// The unknown condition component values.
    pub components: ComponentValueList,
}
