//! destack.core.common.query@2025.08.15.1

#![destack::partial(destack.core.common.query, file)]

use crate::ObjectDefinitionReference;
use crate::PropertyReference;
use crate::Uuid;
use crate::Value;

#[destack::generated(Expression, , block)]
/// Wrapper to unify any scalar / boolean / aggregate sub-tree.
pub struct Expression {
    r#type: ExpressionType,
    literal: Option<Value>,
    attribute: Option<PropertyReference>,
}

#[destack::generated(Join, , block)]
/// Join a Query with another Query.
pub struct Join {
    r#type: JoinType,
    recursive: bool,
    on: Option<Condition>,
}

#[destack::generated(Aggregation, , block)]
/// Aggregation.
pub struct Aggregation {
    r#type: AggregationType,
    expression: Option<Expression>,
}

#[destack::generated(Condition, , block)]
/// Boolean predicate (AND, =, <, etc.).
pub struct Condition {
    r#type: ConditionalType,
    left: Expression,
    right: Option<Expression>,
}

#[destack::generated(Sort, , block)]
/// ORDER BY specification.
pub struct Sort {
    r#type: SortType,
    by: Expression,
    mode: Option<SortMode>,
}

#[destack::generated(Select, , block)]
/// Select specific Attributes.
pub struct Select {
    attributes: Vec<PropertyReference>,
}

#[destack::generated(Query, , block)]
/// A Query into the supergraph about Nodes (node or scalar and potentially grouped).
/// Queries may either be about Entities or Events.
pub struct Query {
    id: Uuid,
    r#type: QueryType,
    name: String,
    definition: ObjectDefinitionReference,
    subqueries: Vec<Query>,
    join: Option<Join>,
    select: Option<Select>,
    r#where: Option<Condition>,
    having: Option<Condition>,
    group_by: Option<Vec<Expression>>,
    aggregation: Option<Aggregation>,
    sort: Option<Vec<Sort>>,
    limit: Option<u32>,
    offset: Option<u32>,
}

#[destack::generated(ConditionalType, , block)]
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

#[destack::generated(AggregationType, , block)]
/// AggregationType
pub enum AggregationType {
    Exists = 1,
    Count = 2,
    Sum = 3,
    Min = 4,
    Max = 5,
    Average = 6,
}

#[destack::generated(SortMode, , block)]
/// SortMode
pub enum SortMode {
    Max = 1,
    Min = 2,
    Average = 3,
    Sum = 4,
    Median = 5,
}

#[destack::generated(SortType, , block)]
/// SortType
pub enum SortType {
    Ascending = 1,
    Descending = 2,
}

#[destack::generated(JoinType, , block)]
/// JoinType
pub enum JoinType {
    Left = 1,
    Parent = 10,
    Child = 11,
}

#[destack::generated(ExpressionType, , block)]
/// ExpressionType
pub enum ExpressionType {
    Literal = 1,
    Attribute = 2,
    Condition = 3,
    Aggregation = 4,
}

#[destack::generated(QueryType, , block)]
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
