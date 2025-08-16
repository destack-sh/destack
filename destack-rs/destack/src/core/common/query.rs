//! destack.core.common.query@2025.08.15.1

#![destack::partial(destack.core.common.query, file)]

use crate::{ObjectDefinitionReference, PropertyReference, Uuid, Value};

#[destack::generated(Expression, -, block)]
/// Wrapper to unify any scalar / boolean / aggregate sub-tree.
pub struct Expression {
    pub r#type: ExpressionType,
    pub literal: Option<Value>,
    pub attribute: Option<PropertyReference>,
}

#[destack::generated(Join, -, block)]
/// Join a Query with another Query.
pub struct Join {
    pub r#type: JoinType,
    pub recursive: bool,
    pub on: Option<Condition>,
}

#[destack::generated(Aggregation, -, block)]
/// Aggregation.
pub struct Aggregation {
    pub r#type: AggregationType,
    pub expression: Option<Expression>,
}

#[destack::generated(Condition, -, block)]
/// Boolean predicate (AND, =, <, etc.).
pub struct Condition {
    pub r#type: ConditionalType,
    pub left: Expression,
    pub right: Option<Expression>,
}

#[destack::generated(Sort, -, block)]
/// ORDER BY specification.
pub struct Sort {
    pub r#type: SortType,
    pub by: Expression,
    pub mode: Option<SortMode>,
}

#[destack::generated(Select, -, block)]
/// Select specific Attributes.
pub struct Select {
    pub attributes: Vec<PropertyReference>,
}

#[destack::generated(Query, -, block)]
/// A Query into the supergraph about Nodes (node or scalar and potentially grouped).
/// Queries may either be about Entities or Events.
pub struct Query {
    pub id: Uuid,
    pub r#type: QueryType,
    pub name: String,
    pub definition: ObjectDefinitionReference,
    pub subqueries: Vec<Query>,
    pub join: Option<Join>,
    pub select: Option<Select>,
    pub r#where: Option<Condition>,
    pub having: Option<Condition>,
    pub group_by: Option<Vec<Expression>>,
    pub aggregation: Option<Aggregation>,
    pub sort: Option<Vec<Sort>>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[destack::generated(ConditionalType, -, block)]
/// ConditionalType
pub enum ConditionalType {
    Not = 1,
    And = 2,
    Or = 3,
    Equals = 10,
    NotEquals = 11,
    GreaterThan = 12,
    GreaterThanOrEquals = 13,
    LessThan = 14,
    LessThanOrEquals = 15,
    Matches = 20,
    StartsWith = 21,
    EndsWith = 22,
    In = 30,
    NotIn = 31,
    Exists = 40,
    NotExists = 41,
}

#[destack::generated(AggregationType, -, block)]
/// AggregationType
pub enum AggregationType {
    Exists = 1,
    Count = 2,
    Sum = 3,
    Min = 4,
    Max = 5,
    Average = 6,
}

#[destack::generated(SortMode, -, block)]
/// SortMode
pub enum SortMode {
    Max = 1,
    Min = 2,
    Average = 3,
    Sum = 4,
    Median = 5,
}

#[destack::generated(SortType, -, block)]
/// SortType
pub enum SortType {
    Ascending = 1,
    Descending = 2,
}

#[destack::generated(JoinType, -, block)]
/// JoinType
pub enum JoinType {
    Left = 1,
    Parent = 10,
    Child = 11,
}

#[destack::generated(ExpressionType, -, block)]
/// ExpressionType
pub enum ExpressionType {
    Literal = 1,
    Attribute = 2,
    Condition = 3,
    Aggregation = 4,
}

#[destack::generated(QueryType, -, block)]
/// QueryType
pub enum QueryType {
    /// Flat list of Nodes
    Node = 1,
    /// Single scalar Value
    Scalar = 5,
    /// Grouped list of Nodes
    GroupedNode = 10,
    /// Grouped list of scalar Values
    GroupedScalar = 15,
}
