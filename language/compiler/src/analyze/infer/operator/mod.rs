use super::call::ResolvedMemberFunction;
use super::member::{MemberLookupMode, MemberResolution};
use super::obligation::relation::UnassignableRelationFailureMode;
use super::{
    index_key_kind_for_index, index_key_kind_for_type, index_key_kinds_compatible_for_access,
};
use crate::analyze::common::{CanonicalSymbolMode, InferContext, RelationMode};
use crate::timing::tags;
use crate::{
    AnalyzeError, AnalyzeResult, AnalyzeWarning, Assignability, Compiler, InferState,
    OperatorLanguageSymbolExt,
};
use destack_builtin::LanguageSymbol;
use destack_dir::{
    AssignOperator, BinaryOperator, Constraint, Expression, GlobalSymbolId, Key, LocalInstanceId,
    LocalNodeId, LocalNodeIdAny, LocalTypeId, Mutability, NodeTree, NormalizationMode,
    PrimitiveType, ResolvedSignature, ScalarLiteral, StaticKey, SymbolTable, SymbolType, Type,
    TypeLiteral, TypeTable, UnaryOperator,
};
use destack_workspace::{Module, ModuleSource};
use std::collections::HashMap;

mod assign;
mod binary;
mod core;
mod index;
mod r#try;
mod unary;
