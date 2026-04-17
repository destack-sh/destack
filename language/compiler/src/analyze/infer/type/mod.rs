use std::collections::HashSet;

use super::member::MemberLookupMode;
use super::{
    index_key_kind_for_member, index_key_kind_for_type, index_key_kinds_compatible_for_access,
};
use crate::analyze::common::{
    ConstContext, REWRITER_TAG_LITERAL_WIDENING, ReadonlyMaterializer, RelationMode, TypeContext,
    TypeRewriteCache, TypeWalkContext, TypeWalkKey, WideningMode, rewrite_type_with_cache,
};
use crate::{AnalyzeError, AnalyzeResult, Compiler, InferState};
use destack_dir::{
    Asynchrony, BinaryOperator, Declaration, Declarator, Expression, Extension, ExtensionKind,
    FloatType, FunctionCardinality, GlobalSymbolId, IntType, LocalNodeId, LocalNodeIdAny,
    LocalTypeId, Mutability, NodeTree, NormalizationMode, PrimitiveType, ScalarLiteral,
    StaticArgument, StaticExpression, StaticKey, StaticProperty, StringId, SymbolTable, SymbolType,
    Type, TypeElement, TypeField, TypeIndexSignature, TypeLiteral, TypeRewriter,
    TypeRewriterOptions, TypeTable, UnaryOperator, VarianceBound,
};
use destack_workspace::{Module, ProfileId};

mod binding;
mod import;
mod instance;
mod member;
mod operator;
mod remote;

pub(super) use binding::TypeGuardTarget;
pub(crate) use remote::RemoteValueTypeReadDomain;

#[cfg(test)]
mod tests;
