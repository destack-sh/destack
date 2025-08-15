//! destack.core.common.query@2025.08.15.1

#![destack::partial(destack.core.common.query, file)]

use crate::{ObjectDefinitionReference, PropertyReference, Uuid, Value};

#[destack::generated(Expression, struct, block)]
/// Wrapper to unify any scalar / boolean / aggregate sub-tree.
pub struct Expression {
    r#type: ExpressionType,
    literal: Value,
    attribute: PropertyReference,
    condition: Condition,
    aggregation: Aggregation,
}

#[destack::generated(Join, struct, block)]
/// Join a Query with another Query.
pub struct Join {
    r#type: JoinType,
    recursive: bool,
    on: Condition,
}

#[destack::generated(Aggregation, struct, block)]
/// Aggregation.
pub struct Aggregation {
    r#type: AggregationType,
    expression: Expression,
}

#[destack::generated(Condition, struct, block)]
/// Boolean predicate (AND, =, <, etc.).
pub struct Condition {
    r#type: ConditionalType,
    left: Expression,
    right: Expression,
}

#[destack::generated(Sort, struct, block)]
/// ORDER BY specification.
pub struct Sort {
    r#type: SortType,
    by: Expression,
    mode: SortMode,
}

#[destack::generated(Select, struct, block)]
/// Select specific Attributes.
pub struct Select {
    attributes: Vec<PropertyReference>,
}

#[destack::generated(Query, struct, block)]
/// A Query into the supergraph about Nodes (node or scalar and potentially grouped).
/// Queries may either be about Entities or Events.
pub struct Query {
    id: Uuid,
    r#type: QueryType,
    name: String,
    definition: ObjectDefinitionReference,
    subqueries: Vec<Query>,
    join: Join,
    select: Select,
    r#where: Condition,
    having: Condition,
    group_by: Vec<Expression>,
    aggregation: Aggregation,
    sort: Vec<Sort>,
    limit: u32,
    offset: u32,
}

#[destack::generated(ConditionalType, enum, block)]
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

#[destack::generated(AggregationType, enum, block)]
/// AggregationType
pub enum AggregationType {
    Exists = 1,
    Count = 2,
    Sum = 3,
    Min = 4,
    Max = 5,
    Average = 6,
}

#[destack::generated(SortMode, enum, block)]
/// SortMode
pub enum SortMode {
    Max = 1,
    Min = 2,
    Average = 3,
    Sum = 4,
    Median = 5,
}

#[destack::generated(SortType, enum, block)]
/// SortType
pub enum SortType {
    Ascending = 1,
    Descending = 2,
}

#[destack::generated(JoinType, enum, block)]
/// JoinType
pub enum JoinType {
    Left = 1,
    Parent = 10,
    Child = 11,
}

#[destack::generated(ExpressionType, enum, block)]
/// ExpressionType
pub enum ExpressionType {
    Literal = 1,
    Attribute = 2,
    Condition = 3,
    Function = 4,
    Aggregation = 5,
}

#[destack::generated(QueryType, enum, block)]
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
