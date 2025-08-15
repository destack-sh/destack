//! destack.core.common.query@2025.08.14.0

#![destack::partial(destack.core.common.query, file)]

#[destack::generated(Expression, struct, block)]
/// Wrapper to unify any scalar / boolean / aggregate sub-tree.
pub struct Expression {

}

#[destack::generated(Join, struct, block)]
/// Join a Query with another Query.
pub struct Join {

}

#[destack::generated(Aggregation, struct, block)]
/// Aggregation.
pub struct Aggregation {

}

#[destack::generated(Condition, struct, block)]
/// Boolean predicate (AND, =, <, etc.).
pub struct Condition {

}

#[destack::generated(Sort, struct, block)]
/// ORDER BY specification.
pub struct Sort {

}

#[destack::generated(Select, struct, block)]
/// Select specific Attributes.
pub struct Select {

}

#[destack::generated(Query, struct, block)]
/// A Query into the supergraph about Nodes (node or scalar and potentially grouped).
/// Queries may either be about Entities or Events.
pub struct Query {

}

#[destack::generated(ConditionalType, enum, block)]
/// ConditionalType
pub enum ConditionalType {
    NOT = 1,
    AND = 2,
    OR = 3,
    EQUALS = 10,
    NOT_EQUALS = 11,
    GREATER_THAN = 12,
    GREATER_THAN_OR_EQUALS = 13,
    LESS_THAN = 14,
    LESS_THAN_OR_EQUALS = 15,
    MATCHES = 20,
    STARTS_WITH = 21,
    ENDS_WITH = 22,
    IN = 30,
    NOT_IN = 31,
    EXISTS = 40,
    NOT_EXISTS = 41
}

#[destack::generated(AggregationType, enum, block)]
/// AggregationType
pub enum AggregationType {
    EXISTS = 1,
    COUNT = 2,
    SUM = 3,
    MIN = 4,
    MAX = 5,
    AVERAGE = 6
}

#[destack::generated(SortMode, enum, block)]
/// SortMode
pub enum SortMode {
    MAX = 1,
    MIN = 2,
    AVERAGE = 3,
    SUM = 4,
    MEDIAN = 5
}

#[destack::generated(SortType, enum, block)]
/// SortType
pub enum SortType {
    ASCENDING = 1,
    DESCENDING = 2
}

#[destack::generated(JoinType, enum, block)]
/// JoinType
pub enum JoinType {
    LEFT = 1,
    PARENT = 10,
    CHILD = 11
}

#[destack::generated(ExpressionType, enum, block)]
/// ExpressionType
pub enum ExpressionType {
    LITERAL = 1,
    ATTRIBUTE = 2,
    CONDITION = 3,
    FUNCTION = 4,
    AGGREGATION = 5
}

#[destack::generated(QueryType, enum, block)]
/// QueryType
pub enum QueryType {
    /// Flat list of Nodes
    NODE = 1,
    /// Single scalar Value
    SCALAR = 5,
    /// Grouped list of Nodes
    GROUPED_NODE = 10,
    /// Grouped list of scalar Values
    GROUPED_SCALAR = 15
}