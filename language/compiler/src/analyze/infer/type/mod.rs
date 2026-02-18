use std::collections::HashSet;

use super::member::MemberLookupMode;
use super::{
    index_key_kind_for_member, index_key_kind_for_type, index_key_kinds_compatible_for_access,
};
use crate::analyze::common::{
    AnalyzeReadStage, ConstContext, REWRITER_TAG_LITERAL_WIDENING, ReadonlyMaterializer,
    RelationMode, TypeRewriteCache, TypeWalkContext, TypeWalkKey, WideningMode,
    rewrite_type_with_cache,
};
use crate::{AnalyzeError, AnalyzeOptions, AnalyzeResult, Assignability, Compiler, InferContext};
use destack_dir::{
    Asynchrony, BinaryOperator, Declaration, DependencyItem, Expression, Extension, ExtensionKind,
    FloatType, FunctionCardinality, GlobalSymbolId, InferTable, IntType, LocalNodeId,
    LocalNodeIdAny, LocalTypeId, ModuleTarget, Mutability, NodeTree, NodeType, NormalizationMode,
    PrimitiveType, ScalarLiteral, StaticArgument, StaticExpression, StaticKey, StaticProperty,
    StringId, SymbolTable, SymbolType, Type, TypeBinaryOperator, TypeElement, TypeField,
    TypeIndexSignature, TypeLiteral, TypeMappedParameter, TypeRewriter, TypeRewriterOptions,
    TypeTable, TypeUnaryOperator, UnaryOperator, VarianceBound,
};
use destack_source::ModuleId;
use destack_workspace::{Module, ModuleSource, ProfileId};

mod binding;
mod instance;
mod member;
mod operator;
mod remote;

pub(super) use binding::TypeGuardTarget;

#[cfg(test)]
mod tests;
