//! destack.core

#![destack::partial(destack.core, file)]
#![allow(unused_imports)]
#![allow(unreachable_pub)]

pub use crate::core::builtin::{
    EnumType, EventStatus, ExtensionFlag, HandleType, Materialization, ModuleType, NodeType,
    ProcessFlag, RuntimeLanguage, RuntimePlatform, RuntimeType, StringCasing, StructType,
    TraitType, UniverseCategory, UniverseDomain,
};
pub use crate::core::common::{
    Aggregation, AggregationType, ChangeType, CollectionConstraint, Condition, ConditionalType,
    EditOperation, EditOperationType, Expression, ExpressionType, Icon, IconType, Join, JoinType,
    NamedValue, NodeIdentityReference, NodeSpatialReference, NodeTemporalReference,
    NumberConstraint, PrimitiveType, PropertyReference, PropertyZone, Query, QueryType,
    ReferenceType, ScalarType, Select, Sort, SortMode, SortType, StringConstraint, Text, TextSpan,
    TextSpanType, TextStyleFlag, Type, TypeCardinality, Value, ValueFactory,
};
pub use crate::core::definition::{
    ConstantDefinition, EnumDefinition, HandleDefinition, ModuleDefinition, NodeDefinition,
    OptionDefinition, PropertyDefinition, SchemaDefinition, StructDefinition,
};
pub use crate::core::encoding::{EncoderFlag, EncoderStability, Encoding};
pub use crate::core::space::{BranchType, FolderType, SnapshotStatus, SnapshotType};
pub use crate::core::universe::{
    ClientType, UniverseSignupRequest, UniverseSignupResponse, UniverseSpawnRequest,
    UniverseSpawnResponse,
};

pub mod builtin;
pub mod common;
pub mod definition;
pub mod encoding;
pub mod generation;
pub mod local;
pub mod persistence;
pub mod space;
pub mod universe;
