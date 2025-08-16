//! destack.core.common@2025.08.15.1

#![destack::partial(destack.core.common, file)]
#![allow(unused_imports)]
#![allow(unreachable_pub)]

pub use crate::core::common::change::{ChangeType, EditOperation, EditOperationType};
pub use crate::core::common::icon::{
    Icon, IconType, PrimitiveType, PropertyZone, ReferenceType, ScalarType, TypeCardinality,
    ValueFactory,
};
pub use crate::core::common::query::{
    Aggregation, AggregationType, Condition, ConditionalType, Expression, ExpressionType, Join,
    JoinType, Query, QueryType, Select, Sort, SortMode, SortType,
};
pub use crate::core::common::relation::{
    NodeIdentityReference, NodeSpatialReference, NodeTemporalReference, ObjectDefinitionReference,
    PropertyReference,
};
pub use crate::core::common::text::{Text, TextSpan, TextSpanType, TextStyleFlag};
pub use crate::core::common::r#type::{
    CollectionConstraint, NumberConstraint, StringConstraint, Type,
};
pub use crate::core::common::value::{NamedValue, Value};

pub mod _gen;
pub mod change;
pub mod icon;
pub mod query;
pub mod relation;
pub mod text;
pub mod r#type;
pub mod value;
