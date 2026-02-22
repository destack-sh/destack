use super::call::ResolvedMemberFunction;
use super::member::{MemberLookupMode, MemberResolution};
use super::obligation::relation::UnassignableRelationFailureMode;
use super::{
    index_key_kind_for_index, index_key_kind_for_type, index_key_kinds_compatible_for_access,
};
use crate::analyze::common::{CanonicalSymbolMode, RelationMode};
use crate::timing::tags;
use crate::{
    AnalyzeError, AnalyzeOptions, AnalyzeResult, AnalyzeWarning, Assignability, Compiler,
    InferContext, OperatorLanguageSymbolExt,
};
use destack_builtin::LanguageSymbol;
use destack_dir::{
    AssignOperator, BinaryOperator, Constraint, DynamicKey, Expression, GlobalSymbolId, InferTable,
    LocalInstanceId, LocalNodeId, LocalNodeIdAny, LocalTypeId, Mutability, NodeTree,
    NormalizationMode, PrimitiveType, ResolvedSignature, ScalarLiteral, StaticKey, SymbolTable,
    SymbolType, Type, TypeLiteral, TypeTable, UnaryOperator,
};
use destack_workspace::{Module, ModuleSource, ProfileId};
use std::collections::HashMap;

mod assign;
mod binary;
mod core;
mod index;
mod r#try;
mod unary;
