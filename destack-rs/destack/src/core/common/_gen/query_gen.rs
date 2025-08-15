//! destack.core.common.query@2025.08.15.1

#![destack::generated(destack.core.common.query, file)]

use crate::Aggregation;
use crate::AggregationType;
use crate::Condition;
use crate::ConditionalType;
use crate::Expression;
use crate::ExpressionType;
use crate::Join;
use crate::JoinType;
use crate::Query;
use crate::QueryType;
use crate::Select;
use crate::Sort;
use crate::SortMode;
use crate::SortType;

#[destack::generated(Expression, Debug, block)]
impl std::fmt::Debug for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Expression")
    }
}

#[destack::generated(Join, Debug, block)]
impl std::fmt::Debug for Join {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Join")
    }
}

#[destack::generated(Aggregation, Debug, block)]
impl std::fmt::Debug for Aggregation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Aggregation")
    }
}

#[destack::generated(Condition, Debug, block)]
impl std::fmt::Debug for Condition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Condition")
    }
}

#[destack::generated(Sort, Debug, block)]
impl std::fmt::Debug for Sort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Sort")
    }
}

#[destack::generated(Select, Debug, block)]
impl std::fmt::Debug for Select {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Select")
    }
}

#[destack::generated(Query, Debug, block)]
impl std::fmt::Debug for Query {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Query")
    }
}

#[destack::generated(ConditionalType, Debug, block)]
impl std::fmt::Debug for ConditionalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConditionalType::Not => write!(f, "NOT"),
            ConditionalType::And => write!(f, "AND"),
            ConditionalType::Or => write!(f, "OR"),
            ConditionalType::Equals => write!(f, "EQUALS"),
            ConditionalType::NotEquals => write!(f, "NOT_EQUALS"),
            ConditionalType::GreaterThan => write!(f, "GREATER_THAN"),
            ConditionalType::GreaterThanOrEquals => write!(f, "GREATER_THAN_OR_EQUALS"),
            ConditionalType::LessThan => write!(f, "LESS_THAN"),
            ConditionalType::LessThanOrEquals => write!(f, "LESS_THAN_OR_EQUALS"),
            ConditionalType::Matches => write!(f, "MATCHES"),
            ConditionalType::StartsWith => write!(f, "STARTS_WITH"),
            ConditionalType::EndsWith => write!(f, "ENDS_WITH"),
            ConditionalType::In => write!(f, "IN"),
            ConditionalType::NotIn => write!(f, "NOT_IN"),
            ConditionalType::Exists => write!(f, "EXISTS"),
            ConditionalType::NotExists => write!(f, "NOT_EXISTS"),
        }
    }
}

#[destack::generated(AggregationType, Debug, block)]
impl std::fmt::Debug for AggregationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AggregationType::Exists => write!(f, "EXISTS"),
            AggregationType::Count => write!(f, "COUNT"),
            AggregationType::Sum => write!(f, "SUM"),
            AggregationType::Min => write!(f, "MIN"),
            AggregationType::Max => write!(f, "MAX"),
            AggregationType::Average => write!(f, "AVERAGE"),
        }
    }
}

#[destack::generated(SortMode, Debug, block)]
impl std::fmt::Debug for SortMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SortMode::Max => write!(f, "MAX"),
            SortMode::Min => write!(f, "MIN"),
            SortMode::Average => write!(f, "AVERAGE"),
            SortMode::Sum => write!(f, "SUM"),
            SortMode::Median => write!(f, "MEDIAN"),
        }
    }
}

#[destack::generated(SortType, Debug, block)]
impl std::fmt::Debug for SortType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SortType::Ascending => write!(f, "ASCENDING"),
            SortType::Descending => write!(f, "DESCENDING"),
        }
    }
}

#[destack::generated(JoinType, Debug, block)]
impl std::fmt::Debug for JoinType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JoinType::Left => write!(f, "LEFT"),
            JoinType::Parent => write!(f, "PARENT"),
            JoinType::Child => write!(f, "CHILD"),
        }
    }
}

#[destack::generated(ExpressionType, Debug, block)]
impl std::fmt::Debug for ExpressionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExpressionType::Literal => write!(f, "LITERAL"),
            ExpressionType::Attribute => write!(f, "ATTRIBUTE"),
            ExpressionType::Condition => write!(f, "CONDITION"),
            ExpressionType::Aggregation => write!(f, "AGGREGATION"),
        }
    }
}

#[destack::generated(QueryType, Debug, block)]
impl std::fmt::Debug for QueryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryType::Node => write!(f, "NODE"),
            QueryType::Scalar => write!(f, "SCALAR"),
            QueryType::GroupedNode => write!(f, "GROUPED_NODE"),
            QueryType::GroupedScalar => write!(f, "GROUPED_SCALAR"),
        }
    }
}
