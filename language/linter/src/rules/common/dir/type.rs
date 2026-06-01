use std::collections::{HashMap, HashSet};

use destack_core::StringPool;
use destack_dir as dir;

use crate::LintModuleContext;

/// Return the type id used for flow queries.
pub fn normalized_flow_type_id(
    _ctx: &LintModuleContext<'_>,
    type_id: dir::GlobalTypeId,
) -> dir::GlobalTypeId {
    type_id
}

/// Return true when two types are equivalent after flow normalization.
///
/// This also treats `any` and `unknown` as compatible escape hatches.
pub fn types_are_equivalent_or_any(
    ctx: &LintModuleContext<'_>,
    left_type_id: dir::GlobalTypeId,
    right_type_id: dir::GlobalTypeId,
) -> bool {
    let left_normalized = normalized_flow_type_id(ctx, left_type_id);
    let right_normalized = normalized_flow_type_id(ctx, right_type_id);

    if left_normalized == right_normalized {
        return true;
    }

    is_any_type(ctx, left_normalized) || is_any_type(ctx, right_normalized)
}

/// Shared traversal state for recursive type queries.
struct TypeQueryState {
    /// Type ids in the active recursion stack.
    active_type_ids: HashSet<dir::GlobalTypeId>,
}

impl TypeQueryState {
    /// Build an empty query state.
    fn new() -> Self {
        Self {
            active_type_ids: HashSet::new(),
        }
    }

    /// Enter one type id and return false on recursive cycles.
    fn enter_type_id(&mut self, type_id: dir::GlobalTypeId) -> bool {
        self.active_type_ids.insert(type_id)
    }

    /// Leave one type id after query evaluation.
    fn leave_type_id(&mut self, type_id: dir::GlobalTypeId) {
        let did_remove = self.active_type_ids.remove(&type_id);
        debug_assert!(did_remove);
    }
}

/// Resolve the next type id for value-like wrappers.
fn value_like_type_id(ty: &dir::Type) -> Option<dir::GlobalTypeId> {
    match ty {
        dir::Type::Form(value) => Some(value.value),
        _ => None,
    }
}

/// Resolve union or intersection element type ids.
fn union_or_intersection_elements(ty: &dir::Type) -> Option<&[dir::GlobalTypeId]> {
    match ty {
        dir::Type::Union(union) => Some(union.elements.as_slice()),
        dir::Type::Intersection(intersection) => Some(intersection.elements.as_slice()),
        _ => None,
    }
}

/// Resolve the effective type id for one reference symbol.
fn reference_symbol_type_id(
    ctx: &LintModuleContext<'_>,
    symbol: dir::GlobalSymbolId,
) -> Option<dir::GlobalTypeId> {
    ctx.symbol_type_id(symbol)
}

/// Visit the effective type id for one reference symbol.
fn for_each_reference_symbol_type_id(
    ctx: &LintModuleContext<'_>,
    symbol: dir::GlobalSymbolId,
    mut visitor: impl FnMut(dir::GlobalTypeId),
) {
    if let Some(type_id) = ctx.symbol_type_id(symbol) {
        visitor(type_id);
    }
}

/// Return whether one type is a template literal operation.
fn type_is_template_literal_operation(ty: &dir::Type) -> bool {
    matches!(
        ty,
        dir::Type::Operation(dir::TypeOperation::TemplateLiteral(_))
    )
}

/// Return whether one type operation still needs check reduction.
fn type_is_reducible_operation(ty: &dir::Type) -> bool {
    matches!(
        ty,
        dir::Type::Operation(
            dir::TypeOperation::StringMapping { .. }
                | dir::TypeOperation::Conditional(_)
                | dir::TypeOperation::Mapped(_)
                | dir::TypeOperation::Index(_)
                | dir::TypeOperation::TemplateLiteral(_)
                | dir::TypeOperation::Infer(_)
                | dir::TypeOperation::KeyOf(_)
        )
    )
}

/// Composition policy for union or intersection type queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TypeCompositionPolicy {
    /// Every element must satisfy the query.
    All,
    /// Any element may satisfy the query.
    Any,
}

/// Shared boolean type query shape.
#[derive(Debug, Clone, Copy)]
enum TypeBooleanQuery<'a> {
    /// Check strict boolean compatibility.
    StrictBoolean,
    /// Check array compatibility.
    Array {
        /// Optional array symbol for declared library references.
        array_symbol: Option<dir::GlobalSymbolId>,
    },
    /// Check string compatibility.
    String {
        /// Optional string symbol for declared library references.
        string_symbol: Option<dir::GlobalSymbolId>,
    },
    /// Check floating point compatibility.
    Float,
    /// Check function compatibility.
    Function,
    /// Check async function compatibility.
    AsyncFunction,
    /// Check `any` or `unknown` compatibility.
    Any,
    /// Check explicit `any` compatibility.
    ExplicitAny,
    /// Check Promise compatibility.
    Promise {
        /// Promise symbol for declared library references.
        promise_symbol: dir::GlobalSymbolId,
    },
    /// Check Promise or any-like compatibility.
    PromiseOrAny {
        /// Promise symbol for declared library references.
        promise_symbol: dir::GlobalSymbolId,
    },
    /// Check reference symbol type compatibility.
    ReferenceSymbolKind {
        /// Local symbol table for declaration kind reads.
        symbols: &'a dir::BindingTable<'a>,
        /// Required symbol type.
        symbol_kind: dir::SymbolKind,
    },
    /// Check Promise spread element compatibility.
    PromiseSpreadElementCompatible {
        /// Promise symbol for declared library references.
        promise_symbol: dir::GlobalSymbolId,
    },
    /// Check map references with empty value type arguments.
    MapWithEmptyValue {
        /// Map symbol for declared library references.
        map_symbol: dir::GlobalSymbolId,
    },
    /// Check whether one type declares a `this` parameter.
    HasThisParameter,
    /// Check whether one type has a useful `toString` representation.
    HasUsefulToString,
    /// Check `void` or `never` compatibility.
    VoidOrNever,
    /// Check template interpolation compatibility.
    TemplateInterpolation {
        /// Optional string symbol for declared library references.
        string_symbol: Option<dir::GlobalSymbolId>,
    },
    /// Check string-like property key compatibility.
    StringLikePropertyKey,
    /// Check numeric property key compatibility.
    NumericPropertyKey,
    /// Check symbol-like property key compatibility.
    SymbolLikePropertyKey,
    /// Check definite non error value compatibility.
    DefinitelyNonErrorValue {
        /// Optional `Error` symbol.
        error_symbol: Option<dir::GlobalSymbolId>,
        /// Optional `Result` symbol.
        result_symbol: Option<dir::GlobalSymbolId>,
    },
    /// Check potential nullishness compatibility.
    MaybeNullish,
    /// Check potential non nullish falsy compatibility.
    HasNonNullishFalsy {
        /// Program string table for string literal checks.
        strings: &'a StringPool,
    },
}

/// Shared traversal state for memoized boolean type queries.
struct TypeBooleanQueryState {
    /// Type ids in the active recursion stack.
    active_type_ids: HashSet<dir::GlobalTypeId>,
    /// Cached query results by local type id.
    cached_results: HashMap<dir::GlobalTypeId, bool>,
}

impl TypeBooleanQueryState {
    /// Build an empty boolean query state.
    fn new() -> Self {
        Self {
            active_type_ids: HashSet::new(),
            cached_results: HashMap::new(),
        }
    }

    /// Return one cached result for a type id.
    fn cached_result(&self, type_id: dir::GlobalTypeId) -> Option<bool> {
        self.cached_results.get(&type_id).copied()
    }

    /// Enter one type id and return false on recursive cycles.
    fn enter_type_id(&mut self, type_id: dir::GlobalTypeId) -> bool {
        self.active_type_ids.insert(type_id)
    }

    /// Leave one type id after query evaluation.
    fn leave_type_id(&mut self, type_id: dir::GlobalTypeId) {
        let did_remove = self.active_type_ids.remove(&type_id);
        debug_assert!(did_remove);
    }

    /// Cache one query result for a type id.
    fn cache_result(&mut self, type_id: dir::GlobalTypeId, result: bool) {
        self.cached_results.insert(type_id, result);
    }
}

/// Evaluate one boolean type query from one source type id.
fn evaluate_boolean_type_query(
    ctx: &LintModuleContext<'_>,
    type_id: dir::GlobalTypeId,
    query: TypeBooleanQuery<'_>,
) -> bool {
    evaluate_boolean_type_query_with_statics(ctx, None, type_id, query)
}

/// Evaluate one boolean type query with static values available.
fn evaluate_boolean_type_query_with_statics(
    ctx: &LintModuleContext<'_>,
    statics: Option<&dir::StaticTable<'_>>,
    type_id: dir::GlobalTypeId,
    query: TypeBooleanQuery<'_>,
) -> bool {
    let normalized_type_id = normalized_flow_type_id(ctx, type_id);
    let mut state = TypeBooleanQueryState::new();
    evaluate_boolean_type_query_inner(ctx, statics, normalized_type_id, query, &mut state)
}

/// Evaluate one boolean type query for one normalized type id.
fn evaluate_boolean_type_query_inner(
    ctx: &LintModuleContext<'_>,
    statics: Option<&dir::StaticTable<'_>>,
    type_id: dir::GlobalTypeId,
    query: TypeBooleanQuery<'_>,
    state: &mut TypeBooleanQueryState,
) -> bool {
    let normalized_type_id = normalized_flow_type_id(ctx, type_id);

    if let Some(cached_result) = state.cached_result(normalized_type_id) {
        return cached_result;
    }

    if !state.enter_type_id(normalized_type_id) {
        return false;
    }

    let ty = ctx
        .checked_type(normalized_type_id)
        .unwrap_or(dir::Type::Error);
    let result = if let Some(next_type_id) = value_like_type_id(&ty) {
        evaluate_boolean_type_query_inner(ctx, statics, next_type_id, query, state)
    } else if let dir::Type::Reference(reference) = ty {
        evaluate_reference_boolean_type_query(
            ctx,
            statics,
            reference.symbol,
            Some(reference.arguments.as_slice()),
            query,
            state,
        )
    } else if let Some(element_type_ids) = union_or_intersection_elements(&ty) {
        let composition_policy = type_query_composition_policy(query, &ty);
        aggregate_boolean_query_results(
            ctx,
            statics,
            element_type_ids,
            query,
            composition_policy,
            state,
        )
    } else {
        evaluate_terminal_boolean_type_query(ctx, statics, &ty, query, state)
    };

    state.leave_type_id(normalized_type_id);
    state.cache_result(normalized_type_id, result);
    result
}

/// Return the composition policy for one union or intersection query.
fn type_query_composition_policy(
    query: TypeBooleanQuery<'_>,
    ty: &dir::Type,
) -> TypeCompositionPolicy {
    match ty {
        dir::Type::Union(_) => match query {
            TypeBooleanQuery::Any
            | TypeBooleanQuery::ExplicitAny
            | TypeBooleanQuery::PromiseOrAny { .. }
            | TypeBooleanQuery::ReferenceSymbolKind { .. }
            | TypeBooleanQuery::PromiseSpreadElementCompatible { .. }
            | TypeBooleanQuery::MapWithEmptyValue { .. }
            | TypeBooleanQuery::HasThisParameter
            | TypeBooleanQuery::MaybeNullish
            | TypeBooleanQuery::HasNonNullishFalsy { .. } => TypeCompositionPolicy::Any,
            _ => TypeCompositionPolicy::All,
        },
        dir::Type::Intersection(_) => match query {
            TypeBooleanQuery::Promise { .. }
            | TypeBooleanQuery::PromiseOrAny { .. }
            | TypeBooleanQuery::ReferenceSymbolKind { .. }
            | TypeBooleanQuery::PromiseSpreadElementCompatible { .. }
            | TypeBooleanQuery::MapWithEmptyValue { .. }
            | TypeBooleanQuery::HasThisParameter
            | TypeBooleanQuery::HasUsefulToString
            | TypeBooleanQuery::HasNonNullishFalsy { .. } => TypeCompositionPolicy::Any,
            TypeBooleanQuery::MaybeNullish => TypeCompositionPolicy::All,
            TypeBooleanQuery::Any | TypeBooleanQuery::ExplicitAny => TypeCompositionPolicy::Any,
            _ => TypeCompositionPolicy::All,
        },
        _ => TypeCompositionPolicy::All,
    }
}

/// Aggregate boolean query results across one list of element types.
fn aggregate_boolean_query_results(
    ctx: &LintModuleContext<'_>,
    statics: Option<&dir::StaticTable<'_>>,
    element_type_ids: &[dir::GlobalTypeId],
    query: TypeBooleanQuery<'_>,
    composition_policy: TypeCompositionPolicy,
    state: &mut TypeBooleanQueryState,
) -> bool {
    match composition_policy {
        TypeCompositionPolicy::All => element_type_ids
            .iter()
            .all(|type_id| evaluate_boolean_type_query_inner(ctx, statics, *type_id, query, state)),
        TypeCompositionPolicy::Any => element_type_ids
            .iter()
            .any(|type_id| evaluate_boolean_type_query_inner(ctx, statics, *type_id, query, state)),
    }
}

/// Evaluate one boolean type query for one reference symbol.
fn evaluate_reference_boolean_type_query(
    ctx: &LintModuleContext<'_>,
    statics: Option<&dir::StaticTable<'_>>,
    symbol: dir::GlobalSymbolId,
    generic_arguments: Option<&[dir::StaticArgument]>,
    query: TypeBooleanQuery<'_>,
    state: &mut TypeBooleanQueryState,
) -> bool {
    match query {
        TypeBooleanQuery::Array { array_symbol } => {
            if array_symbol.is_some_and(|array_symbol| array_symbol == symbol) {
                return true;
            }
        }
        TypeBooleanQuery::String { string_symbol }
        | TypeBooleanQuery::TemplateInterpolation { string_symbol } => {
            if string_symbol.is_some_and(|string_symbol| string_symbol == symbol) {
                return true;
            }
        }
        TypeBooleanQuery::Promise { promise_symbol } => {
            if symbol == promise_symbol {
                return true;
            }
        }
        TypeBooleanQuery::PromiseOrAny { promise_symbol } => {
            if symbol == promise_symbol {
                return true;
            }
        }
        TypeBooleanQuery::ReferenceSymbolKind {
            symbols,
            symbol_kind,
        } => {
            if symbol.module_id == symbols.module_id
                && symbols.get_symbol(symbol.local_id).kind == symbol_kind
            {
                return true;
            }
        }
        TypeBooleanQuery::PromiseSpreadElementCompatible { promise_symbol } => {
            if symbol == promise_symbol {
                return true;
            }
        }
        TypeBooleanQuery::MapWithEmptyValue { map_symbol } => {
            if symbol == map_symbol
                && generic_arguments_contain_empty_map_value(ctx, statics, generic_arguments)
            {
                return true;
            }
        }
        TypeBooleanQuery::HasUsefulToString => {
            if symbol.module_id == ctx.module_id() {
                return true;
            }
        }
        TypeBooleanQuery::DefinitelyNonErrorValue {
            error_symbol,
            result_symbol,
        } => {
            if error_symbol.is_some_and(|error_symbol| {
                symbol_matches_relation_target(ctx, symbol, error_symbol)
            }) {
                return false;
            }

            if result_symbol.is_some_and(|result_symbol| {
                symbol_matches_relation_target(ctx, symbol, result_symbol)
            }) {
                return true;
            }
        }
        _ => {}
    }

    if let Some(next_type_id) = reference_symbol_type_id(ctx, symbol) {
        return evaluate_boolean_type_query_inner(ctx, statics, next_type_id, query, state);
    }

    matches!(query, TypeBooleanQuery::HasNonNullishFalsy { .. })
}

/// Evaluate one boolean type query for one terminal type node.
fn evaluate_terminal_boolean_type_query(
    ctx: &LintModuleContext<'_>,
    statics: Option<&dir::StaticTable<'_>>,
    ty: &dir::Type,
    query: TypeBooleanQuery<'_>,
    state: &mut TypeBooleanQueryState,
) -> bool {
    match query {
        TypeBooleanQuery::StrictBoolean => matches!(
            ty,
            dir::Type::Primitive(dir::PrimitiveType::Boolean)
                | dir::Type::Literal(dir::ScalarLiteral::Boolean(_))
        ),
        TypeBooleanQuery::Array { .. } => matches!(
            ty,
            dir::Type::Slice(_) | dir::Type::FixedArray(_) | dir::Type::Tuple(_)
        ),
        TypeBooleanQuery::String { .. } => {
            type_is_template_literal_operation(ty) || type_is_string_like(ty)
        }
        TypeBooleanQuery::Float => matches!(ty, dir::Type::Primitive(dir::PrimitiveType::Float(_))),
        TypeBooleanQuery::Function => match ty {
            dir::Type::Function(_) => true,
            dir::Type::Shape(object) => !object.call_signatures.is_empty(),
            _ => false,
        },
        TypeBooleanQuery::HasThisParameter => match ty {
            dir::Type::Function(function) => function.this_parameter.is_some(),
            _ => false,
        },
        TypeBooleanQuery::ReferenceSymbolKind { .. } => false,
        TypeBooleanQuery::PromiseSpreadElementCompatible { promise_symbol } => match ty {
            dir::Type::Any | dir::Type::Unknown => true,
            dir::Type::Slice(slice) => evaluate_boolean_type_query_inner(
                ctx,
                statics,
                slice.element,
                TypeBooleanQuery::PromiseOrAny { promise_symbol },
                state,
            ),
            dir::Type::FixedArray(array) => evaluate_boolean_type_query_inner(
                ctx,
                statics,
                array.element,
                TypeBooleanQuery::PromiseOrAny { promise_symbol },
                state,
            ),
            dir::Type::Tuple(tuple) => tuple.elements.iter().any(|element| {
                evaluate_boolean_type_query_inner(
                    ctx,
                    statics,
                    element.ty,
                    TypeBooleanQuery::PromiseOrAny { promise_symbol },
                    state,
                )
            }),
            _ => false,
        },
        TypeBooleanQuery::MapWithEmptyValue { .. } => false,
        TypeBooleanQuery::HasUsefulToString => match ty {
            dir::Type::Primitive(
                dir::PrimitiveType::String
                | dir::PrimitiveType::Boolean
                | dir::PrimitiveType::Integer(_)
                | dir::PrimitiveType::Float(_)
                | dir::PrimitiveType::Bigint,
            )
            | dir::Type::Literal(_) => true,
            dir::Type::Slice(slice) => evaluate_boolean_type_query_inner(
                ctx,
                statics,
                slice.element,
                TypeBooleanQuery::HasUsefulToString,
                state,
            ),
            dir::Type::FixedArray(array) => evaluate_boolean_type_query_inner(
                ctx,
                statics,
                array.element,
                TypeBooleanQuery::HasUsefulToString,
                state,
            ),
            dir::Type::Tuple(tuple) => tuple.elements.iter().all(|element| {
                evaluate_boolean_type_query_inner(
                    ctx,
                    statics,
                    element.ty,
                    TypeBooleanQuery::HasUsefulToString,
                    state,
                )
            }),
            dir::Type::Function(_) => true,
            dir::Type::Shape(_) => false,
            dir::Type::Error => true,
            _ => false,
        },
        TypeBooleanQuery::AsyncFunction => match ty {
            dir::Type::Function(function) => function.asynchrony == dir::Asynchrony::Async,
            dir::Type::Shape(object) => object.call_signatures.iter().any(|type_id| {
                evaluate_boolean_type_query_inner(
                    ctx,
                    statics,
                    *type_id,
                    TypeBooleanQuery::AsyncFunction,
                    state,
                )
            }),
            _ => false,
        },
        TypeBooleanQuery::Any => matches!(ty, dir::Type::Any | dir::Type::Unknown),
        TypeBooleanQuery::PromiseOrAny { .. } => matches!(ty, dir::Type::Any | dir::Type::Unknown),
        TypeBooleanQuery::ExplicitAny => matches!(ty, dir::Type::Any),
        TypeBooleanQuery::Promise { .. } => false,
        TypeBooleanQuery::VoidOrNever => matches!(ty, dir::Type::Void | dir::Type::Never),
        TypeBooleanQuery::TemplateInterpolation { .. } => match ty {
            ty if type_is_template_literal_operation(ty) => true,
            dir::Type::Primitive(
                dir::PrimitiveType::Bigint
                | dir::PrimitiveType::Integer(_)
                | dir::PrimitiveType::Float(_),
            )
            | dir::Type::Literal(
                dir::ScalarLiteral::Integer(_)
                | dir::ScalarLiteral::Bigint(_)
                | dir::ScalarLiteral::Float(_),
            ) => true,
            _ => type_is_string_like(ty),
        },
        TypeBooleanQuery::StringLikePropertyKey => type_is_string_like_property_key(ty),
        TypeBooleanQuery::NumericPropertyKey => type_is_numeric_property_key(ty),
        TypeBooleanQuery::SymbolLikePropertyKey => type_is_symbol_like_property_key(ty),
        TypeBooleanQuery::DefinitelyNonErrorValue { .. } => matches!(
            ty,
            dir::Type::Never
                | dir::Type::Undefined
                | dir::Type::Void
                | dir::Type::Null
                | dir::Type::Primitive(_)
                | dir::Type::Range(_)
                | dir::Type::Literal(_)
        ),
        TypeBooleanQuery::MaybeNullish => match ty {
            dir::Type::Null
            | dir::Type::Undefined
            | dir::Type::Void
            | dir::Type::Any
            | dir::Type::Unknown
            | dir::Type::Error => true,
            ty if type_is_reducible_operation(ty) => true,
            _ => false,
        },
        TypeBooleanQuery::HasNonNullishFalsy { strings } => match ty {
            dir::Type::Null | dir::Type::Undefined | dir::Type::Void | dir::Type::Never => false,
            dir::Type::Any | dir::Type::Unknown => true,
            dir::Type::Object => false,
            dir::Type::Primitive(primitive) => matches!(
                primitive,
                dir::PrimitiveType::Boolean
                    | dir::PrimitiveType::Bigint
                    | dir::PrimitiveType::String
                    | dir::PrimitiveType::Integer(_)
                    | dir::PrimitiveType::Float(_)
            ),
            dir::Type::Literal(literal) => match literal {
                dir::ScalarLiteral::Null => true,
                dir::ScalarLiteral::Boolean(false) => true,
                dir::ScalarLiteral::Boolean(true) => false,
                dir::ScalarLiteral::Integer(value) => *value == 0,
                dir::ScalarLiteral::Bigint(value) => *value == 0,
                dir::ScalarLiteral::Float(value) => *value == 0.0,
                dir::ScalarLiteral::String(value) => strings.get(*value).is_empty(),
                dir::ScalarLiteral::Character(_) | dir::ScalarLiteral::RegexString { .. } => true,
            },
            dir::Type::Operation(_) => true,
            dir::Type::Array(_)
            | dir::Type::Slice(_)
            | dir::Type::FixedArray(_)
            | dir::Type::Tuple(_)
            | dir::Type::Shape(_)
            | dir::Type::Closure(_)
            | dir::Type::Function(_) => false,
            dir::Type::Parameter(_)
            | dir::Type::Reference(_)
            | dir::Type::Member(_)
            | dir::Type::This
            | dir::Type::Union(_)
            | dir::Type::Intersection(_)
            | dir::Type::Form(_)
            | dir::Type::Dynamic(_)
            | dir::Type::Range(_)
            | dir::Type::Error => true,
        },
    }
}

/// Return true when static arguments contain an empty map value argument.
fn generic_arguments_contain_empty_map_value(
    ctx: &LintModuleContext<'_>,
    _statics: Option<&dir::StaticTable<'_>>,
    generic_arguments: Option<&[dir::StaticArgument]>,
) -> bool {
    let Some(generic_arguments) = generic_arguments else {
        return false;
    };
    if generic_arguments.len() != 2 {
        return false;
    }

    generic_argument_is_void_or_never_type(ctx, &generic_arguments[1])
}

/// Return true when one static argument resolves to `void` or `never`.
fn generic_argument_is_void_or_never_type(
    ctx: &LintModuleContext<'_>,
    generic_argument: &dir::StaticArgument,
) -> bool {
    let Some(term) = ctx.checked_static(generic_argument.value) else {
        return false;
    };

    match &term {
        dir::StaticTerm::Type { ty } => is_void_or_never_type(ctx, *ty),
        dir::StaticTerm::TypeLiteral {
            value: dir::TypeLiteral::Void | dir::TypeLiteral::Never,
        } => true,
        _ => false,
    }
}

/// Return true when one type is string-like.
fn type_is_string_like(ty: &dir::Type) -> bool {
    matches!(
        ty,
        dir::Type::Primitive(dir::PrimitiveType::String)
            | dir::Type::Literal(dir::ScalarLiteral::String(_))
            | dir::Type::Operation(dir::TypeOperation::StringMapping { .. })
    )
}

/// Return true when one type is string-like for object property keys.
fn type_is_string_like_property_key(ty: &dir::Type) -> bool {
    matches!(
        ty,
        dir::Type::Primitive(
            dir::PrimitiveType::String
                | dir::PrimitiveType::Boolean
                | dir::PrimitiveType::Character,
        ) | dir::Type::Literal(
            dir::ScalarLiteral::String(_)
                | dir::ScalarLiteral::Boolean(_)
                | dir::ScalarLiteral::Character(_)
                | dir::ScalarLiteral::RegexString { .. },
        ) | dir::Type::Null
            | dir::Type::Undefined
    )
}

/// Return true when one type is numeric for object property keys.
fn type_is_numeric_property_key(ty: &dir::Type) -> bool {
    matches!(
        ty,
        dir::Type::Primitive(
            dir::PrimitiveType::Bigint
                | dir::PrimitiveType::Integer(_)
                | dir::PrimitiveType::Float(_),
        ) | dir::Type::Literal(
            dir::ScalarLiteral::Integer(_)
                | dir::ScalarLiteral::Float(_)
                | dir::ScalarLiteral::Bigint(_),
        )
    )
}

/// Return true when one type is symbol-like for object property keys.
fn type_is_symbol_like_property_key(ty: &dir::Type) -> bool {
    matches!(
        ty,
        dir::Type::Primitive(dir::PrimitiveType::Symbol | dir::PrimitiveType::UniqueSymbol)
    )
}

/// Return true when the type is strictly boolean.
pub fn is_strict_boolean_type(ctx: &LintModuleContext<'_>, type_id: dir::GlobalTypeId) -> bool {
    evaluate_boolean_type_query(ctx, type_id, TypeBooleanQuery::StrictBoolean)
}

/// Return true when the type is an array type.
pub fn is_array_type(
    ctx: &LintModuleContext<'_>,
    type_id: dir::GlobalTypeId,
    array_symbol: Option<dir::GlobalSymbolId>,
) -> bool {
    evaluate_boolean_type_query(ctx, type_id, TypeBooleanQuery::Array { array_symbol })
}

/// Return the fixed arity when one type resolves to a tuple.
pub fn tuple_type_arity(ctx: &LintModuleContext<'_>, type_id: dir::GlobalTypeId) -> Option<usize> {
    let mut current_type_id = normalized_flow_type_id(ctx, type_id);
    let mut visited_type_ids = HashSet::new();

    loop {
        if !visited_type_ids.insert(current_type_id) {
            return None;
        }

        let current_type = ctx
            .checked_type(current_type_id)
            .unwrap_or(dir::Type::Error);
        match current_type {
            dir::Type::Tuple(tuple) => return Some(tuple.elements.len()),
            dir::Type::Form(value) => {
                current_type_id = value.value;
            }
            dir::Type::Reference(reference) => {
                current_type_id = reference_symbol_type_id(ctx, reference.symbol)?;
            }
            _ => return None,
        }
    }
}

/// Return true when the type is an array or tuple whose elements are strings.
pub fn is_string_array_type(
    ctx: &LintModuleContext<'_>,
    type_id: dir::GlobalTypeId,
    array_symbol: Option<dir::GlobalSymbolId>,
    string_symbol: Option<dir::GlobalSymbolId>,
) -> bool {
    let mut state = TypeQueryState::new();
    is_string_array_type_inner(ctx, type_id, array_symbol, string_symbol, &mut state)
}

/// Evaluate string-array compatibility recursively.
fn is_string_array_type_inner(
    ctx: &LintModuleContext<'_>,
    type_id: dir::GlobalTypeId,
    array_symbol: Option<dir::GlobalSymbolId>,
    string_symbol: Option<dir::GlobalSymbolId>,
    state: &mut TypeQueryState,
) -> bool {
    let normalized_type_id = normalized_flow_type_id(ctx, type_id);
    if !state.enter_type_id(normalized_type_id) {
        return false;
    }

    let ty = ctx
        .checked_type(normalized_type_id)
        .unwrap_or(dir::Type::Error);
    let result = match ty {
        dir::Type::Slice(slice) => is_string_type(ctx, slice.element, string_symbol),
        dir::Type::FixedArray(array) => is_string_type(ctx, array.element, string_symbol),
        dir::Type::Tuple(tuple) => tuple
            .elements
            .iter()
            .all(|element| is_string_type(ctx, element.ty, string_symbol)),
        dir::Type::Reference(reference) => {
            if array_symbol.is_none_or(|array_symbol| reference.symbol != array_symbol) {
                false
            } else {
                reference.arguments.first().is_some_and(|generic_argument| {
                    generic_argument_type_id(ctx, generic_argument).is_some_and(|element_type_id| {
                        is_string_type(ctx, element_type_id, string_symbol)
                    })
                })
            }
        }
        dir::Type::Union(union) => union.elements.iter().all(|element_type_id| {
            is_string_array_type_inner(ctx, *element_type_id, array_symbol, string_symbol, state)
        }),
        dir::Type::Intersection(intersection) => {
            intersection.elements.iter().any(|element_type_id| {
                is_string_array_type_inner(
                    ctx,
                    *element_type_id,
                    array_symbol,
                    string_symbol,
                    state,
                )
            })
        }
        dir::Type::Form(value) => {
            is_string_array_type_inner(ctx, value.value, array_symbol, string_symbol, state)
        }
        _ => false,
    };

    state.leave_type_id(normalized_type_id);
    result
}

/// Resolve one static argument into a concrete type id when available.
fn generic_argument_type_id(
    ctx: &LintModuleContext<'_>,
    generic_argument: &dir::StaticArgument,
) -> Option<dir::GlobalTypeId> {
    match ctx.checked_static(generic_argument.value)? {
        dir::StaticTerm::Type { ty } => Some(ty),
        _ => None,
    }
}

/// Return true when the type may behave as array-like in `for-in` iteration.
///
/// This returns true for direct arrays, tuples, and any union or intersection
/// branch that resolves to an array-like structure.
pub fn is_array_like_iteration_type(
    ctx: &LintModuleContext<'_>,
    strings: &StringPool,
    type_id: dir::GlobalTypeId,
    array_symbol: Option<dir::GlobalSymbolId>,
) -> bool {
    let mut visited_type_ids = HashSet::new();
    is_array_like_iteration_type_inner(ctx, strings, type_id, array_symbol, &mut visited_type_ids)
}

/// Evaluate array-like iteration compatibility recursively.
fn is_array_like_iteration_type_inner(
    ctx: &LintModuleContext<'_>,
    strings: &StringPool,
    type_id: dir::GlobalTypeId,
    array_symbol: Option<dir::GlobalSymbolId>,
    visited_type_ids: &mut HashSet<dir::GlobalTypeId>,
) -> bool {
    if !visited_type_ids.insert(type_id) {
        return false;
    }

    if is_array_type(ctx, type_id, array_symbol) {
        return true;
    }

    let type_node = ctx.checked_type(type_id).unwrap_or(dir::Type::Error);
    match type_node {
        dir::Type::Union(union) => union.elements.iter().any(|element_type_id| {
            is_array_like_iteration_type_inner(
                ctx,
                strings,
                *element_type_id,
                array_symbol,
                visited_type_ids,
            )
        }),
        dir::Type::Intersection(intersection) => {
            intersection.elements.iter().any(|element_type_id| {
                is_array_like_iteration_type_inner(
                    ctx,
                    strings,
                    *element_type_id,
                    array_symbol,
                    visited_type_ids,
                )
            })
        }
        dir::Type::Form(value) => is_array_like_iteration_type_inner(
            ctx,
            strings,
            value.value,
            array_symbol,
            visited_type_ids,
        ),
        dir::Type::Shape(object) => {
            object_has_numeric_index_signature(ctx, &object.index_signatures)
                && object_has_array_like_length_field(ctx, strings, &object.fields)
        }
        _ => false,
    }
}

/// Return true when one object declares a numeric index signature.
fn object_has_numeric_index_signature(
    ctx: &LintModuleContext<'_>,
    index_signatures: &[dir::TypeIndexSignature],
) -> bool {
    index_signatures
        .iter()
        .any(|signature| is_numeric_property_key_type(ctx, signature.key_type))
}

/// Return true when one object has a numeric `length` field.
fn object_has_array_like_length_field(
    ctx: &LintModuleContext<'_>,
    strings: &StringPool,
    fields: &[dir::TypeField],
) -> bool {
    fields.iter().any(|field| {
        let Some(field_name_id) = field.key.name() else {
            return false;
        };
        if strings.get(field_name_id) != "length" {
            return false;
        }

        is_numeric_property_key_type(ctx, field.ty)
    })
}

/// Return true when the type is a string type.
pub fn is_string_type(
    ctx: &LintModuleContext<'_>,
    type_id: dir::GlobalTypeId,
    string_symbol: Option<dir::GlobalSymbolId>,
) -> bool {
    evaluate_boolean_type_query(ctx, type_id, TypeBooleanQuery::String { string_symbol })
}

/// Return true when the type is a floating point type.
pub fn is_float_type(ctx: &LintModuleContext<'_>, type_id: dir::GlobalTypeId) -> bool {
    evaluate_boolean_type_query(ctx, type_id, TypeBooleanQuery::Float)
}

/// Return true when the type is a function type.
pub fn is_function_type(ctx: &LintModuleContext<'_>, type_id: dir::GlobalTypeId) -> bool {
    evaluate_boolean_type_query(ctx, type_id, TypeBooleanQuery::Function)
}

/// Return true when one type may resolve to one symbol type.
pub fn is_reference_symbol_kind(
    ctx: &LintModuleContext<'_>,
    symbols: &dir::BindingTable<'_>,
    type_id: dir::GlobalTypeId,
    symbol_kind: dir::SymbolKind,
) -> bool {
    evaluate_boolean_type_query(
        ctx,
        type_id,
        TypeBooleanQuery::ReferenceSymbolKind {
            symbols,
            symbol_kind,
        },
    )
}

/// Return true when one type declares a `this` parameter.
pub fn has_this_parameter_type(ctx: &LintModuleContext<'_>, type_id: dir::GlobalTypeId) -> bool {
    evaluate_boolean_type_query(ctx, type_id, TypeBooleanQuery::HasThisParameter)
}

/// Return true when one type declares a non-void `this` parameter.
pub fn has_non_void_this_parameter_type(
    ctx: &LintModuleContext<'_>,
    type_id: dir::GlobalTypeId,
) -> bool {
    let mut visited_type_ids = HashSet::new();
    has_non_void_this_parameter_type_inner(ctx, type_id, &mut visited_type_ids)
}

/// Evaluate non-void `this` parameter compatibility recursively.
fn has_non_void_this_parameter_type_inner(
    ctx: &LintModuleContext<'_>,
    type_id: dir::GlobalTypeId,
    visited_type_ids: &mut HashSet<dir::GlobalTypeId>,
) -> bool {
    let normalized_type_id = normalized_flow_type_id(ctx, type_id);
    if !visited_type_ids.insert(normalized_type_id) {
        return false;
    }

    let ty = ctx
        .checked_type(normalized_type_id)
        .unwrap_or(dir::Type::Error);
    match ty {
        dir::Type::Function(function) => {
            function
                .this_parameter
                .is_some_and(|this_parameter_type_id| {
                    !is_void_or_never_type(ctx, this_parameter_type_id)
                })
        }
        dir::Type::Shape(object) => object.call_signatures.iter().any(|signature_type_id| {
            has_non_void_this_parameter_type_inner(ctx, *signature_type_id, visited_type_ids)
        }),
        dir::Type::Union(union) => union.elements.iter().any(|element_type_id| {
            has_non_void_this_parameter_type_inner(ctx, *element_type_id, visited_type_ids)
        }),
        dir::Type::Intersection(intersection) => {
            intersection.elements.iter().any(|element_type_id| {
                has_non_void_this_parameter_type_inner(ctx, *element_type_id, visited_type_ids)
            })
        }
        dir::Type::Form(value) => {
            has_non_void_this_parameter_type_inner(ctx, value.value, visited_type_ids)
        }
        dir::Type::Reference(reference) => reference_symbol_type_id(ctx, reference.symbol)
            .is_some_and(|target_type_id| {
                has_non_void_this_parameter_type_inner(ctx, target_type_id, visited_type_ids)
            }),
        _ => false,
    }
}

/// Return true when one type has a useful `toString` representation.
pub fn has_useful_to_string_type(ctx: &LintModuleContext<'_>, type_id: dir::GlobalTypeId) -> bool {
    evaluate_boolean_type_query(ctx, type_id, TypeBooleanQuery::HasUsefulToString)
}

/// Return true when the type is an async function.
pub fn is_async_function_type(ctx: &LintModuleContext<'_>, type_id: dir::GlobalTypeId) -> bool {
    evaluate_boolean_type_query(ctx, type_id, TypeBooleanQuery::AsyncFunction)
}

/// Return true when the type is `any`.
pub fn is_any_type(ctx: &LintModuleContext<'_>, type_id: dir::GlobalTypeId) -> bool {
    evaluate_boolean_type_query(ctx, type_id, TypeBooleanQuery::Any)
}

/// Return true when the type is `Error`.
pub fn is_error_type(ctx: &LintModuleContext<'_>, type_id: dir::GlobalTypeId) -> bool {
    matches!(
        ctx.checked_type(normalized_flow_type_id(ctx, type_id))
            .unwrap_or(dir::Type::Error),
        dir::Type::Error
    )
}

/// Return true when the type tree contains explicit `any` (but not `unknown`).
pub fn is_explicit_any_type(ctx: &LintModuleContext<'_>, type_id: dir::GlobalTypeId) -> bool {
    evaluate_boolean_type_query(ctx, type_id, TypeBooleanQuery::ExplicitAny)
}

/// Return true when the type is a Promise.
pub fn is_promise_type(
    ctx: &LintModuleContext<'_>,
    type_id: dir::GlobalTypeId,
    promise_symbol: Option<dir::GlobalSymbolId>,
) -> bool {
    let Some(promise_symbol) = promise_symbol else {
        return false;
    };

    evaluate_boolean_type_query(ctx, type_id, TypeBooleanQuery::Promise { promise_symbol })
}

/// Return true when the type is Promise or any-like.
pub fn is_promise_or_any_type(
    ctx: &LintModuleContext<'_>,
    type_id: dir::GlobalTypeId,
    promise_symbol: Option<dir::GlobalSymbolId>,
) -> bool {
    let Some(promise_symbol) = promise_symbol else {
        return false;
    };

    evaluate_boolean_type_query(
        ctx,
        type_id,
        TypeBooleanQuery::PromiseOrAny { promise_symbol },
    )
}

/// Return true when one type supports Promise spread elements.
pub fn supports_promise_spread_elements(
    ctx: &LintModuleContext<'_>,
    type_id: dir::GlobalTypeId,
    promise_symbol: Option<dir::GlobalSymbolId>,
) -> bool {
    let Some(promise_symbol) = promise_symbol else {
        return false;
    };

    evaluate_boolean_type_query(
        ctx,
        type_id,
        TypeBooleanQuery::PromiseSpreadElementCompatible { promise_symbol },
    )
}

/// Return true when one type contains `Map<_, void | never>`.
pub fn contains_map_with_empty_value_type(
    ctx: &LintModuleContext<'_>,
    type_id: dir::GlobalTypeId,
    map_symbol: dir::GlobalSymbolId,
) -> bool {
    evaluate_boolean_type_query_with_statics(
        ctx,
        Some(ctx.statics),
        type_id,
        TypeBooleanQuery::MapWithEmptyValue { map_symbol },
    )
}

/// Return true when the type is `void` or `never`.
pub fn is_void_or_never_type(ctx: &LintModuleContext<'_>, type_id: dir::GlobalTypeId) -> bool {
    evaluate_boolean_type_query(ctx, type_id, TypeBooleanQuery::VoidOrNever)
}

/// Return true when the type can be interpolated into a template string.
pub fn is_template_interpolation_type(
    ctx: &LintModuleContext<'_>,
    type_id: dir::GlobalTypeId,
    string_symbol: Option<dir::GlobalSymbolId>,
) -> bool {
    evaluate_boolean_type_query(
        ctx,
        type_id,
        TypeBooleanQuery::TemplateInterpolation { string_symbol },
    )
}

/// Return true when the type is string-like for object property keys.
pub fn is_string_like_property_key_type(
    ctx: &LintModuleContext<'_>,
    type_id: dir::GlobalTypeId,
) -> bool {
    evaluate_boolean_type_query(ctx, type_id, TypeBooleanQuery::StringLikePropertyKey)
}

/// Return true when the type is numeric for object property keys.
pub fn is_numeric_property_key_type(
    ctx: &LintModuleContext<'_>,
    type_id: dir::GlobalTypeId,
) -> bool {
    evaluate_boolean_type_query(ctx, type_id, TypeBooleanQuery::NumericPropertyKey)
}

/// Return true when the type is symbol-like for object property keys.
pub fn is_symbol_like_property_key_type(
    ctx: &LintModuleContext<'_>,
    type_id: dir::GlobalTypeId,
) -> bool {
    evaluate_boolean_type_query(ctx, type_id, TypeBooleanQuery::SymbolLikePropertyKey)
}

/// Return true when the type is definitely a non error runtime value.
pub fn is_definitely_non_error_value_type(
    ctx: &LintModuleContext<'_>,
    type_id: dir::GlobalTypeId,
    error_symbol: Option<dir::GlobalSymbolId>,
    result_symbol: Option<dir::GlobalSymbolId>,
) -> bool {
    if type_may_be_nominal_symbol(ctx, type_id, error_symbol) {
        return false;
    }

    evaluate_boolean_type_query(
        ctx,
        type_id,
        TypeBooleanQuery::DefinitelyNonErrorValue {
            error_symbol,
            result_symbol,
        },
    )
}

/// Return true when the type can evaluate to a nullish value.
pub fn is_maybe_nullish_type(ctx: &LintModuleContext<'_>, type_id: dir::GlobalTypeId) -> bool {
    evaluate_boolean_type_query(ctx, type_id, TypeBooleanQuery::MaybeNullish)
}

/// Return true when the type can evaluate to a non nullish falsy value.
pub fn has_non_nullish_falsy_type(
    ctx: &LintModuleContext<'_>,
    strings: &StringPool,
    type_id: dir::GlobalTypeId,
) -> bool {
    evaluate_boolean_type_query(
        ctx,
        type_id,
        TypeBooleanQuery::HasNonNullishFalsy { strings },
    )
}

/// Truthiness certainty for one type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeTruthiness {
    /// The type is always truthy.
    AlwaysTruthy,
    /// The type is always falsy.
    AlwaysFalsy,
    /// The type may be either truthy or falsy.
    Unknown,
}

/// Nullish certainty for one type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeNullishness {
    /// The type is never nullish.
    Never,
    /// The type is always nullish.
    Always,
    /// The type may be nullish or non nullish.
    Maybe,
}

/// Shared traversal state for truthiness queries.
struct TypeTruthinessState {
    /// Type ids in the active recursion stack.
    active_type_ids: HashSet<dir::GlobalTypeId>,
    /// Cached truthiness values by local type id.
    cached_values: HashMap<dir::GlobalTypeId, TypeTruthiness>,
}

impl TypeTruthinessState {
    /// Build an empty truthiness query state.
    fn new() -> Self {
        Self {
            active_type_ids: HashSet::new(),
            cached_values: HashMap::new(),
        }
    }
}

/// Shared traversal state for nullishness queries.
struct TypeNullishnessState {
    /// Type ids in the active recursion stack.
    active_type_ids: HashSet<dir::GlobalTypeId>,
    /// Cached nullishness values by local type id.
    cached_values: HashMap<dir::GlobalTypeId, TypeNullishness>,
}

impl TypeNullishnessState {
    /// Build an empty nullishness query state.
    fn new() -> Self {
        Self {
            active_type_ids: HashSet::new(),
            cached_values: HashMap::new(),
        }
    }
}

/// Return truthiness certainty for one type.
pub fn type_truthiness(
    ctx: &LintModuleContext<'_>,
    strings: &StringPool,
    type_id: dir::GlobalTypeId,
) -> TypeTruthiness {
    let mut state = TypeTruthinessState::new();
    type_truthiness_inner(ctx, strings, type_id, &mut state)
}

/// Return nullishness certainty for one type.
pub fn type_nullishness(
    ctx: &LintModuleContext<'_>,
    type_id: dir::GlobalTypeId,
) -> TypeNullishness {
    let mut state = TypeNullishnessState::new();
    type_nullishness_inner(ctx, type_id, &mut state)
}

/// Unwrap nested `Type::Form` wrappers to one underlying type.
pub fn unwrap_form_payload_type_id(
    ctx: &LintModuleContext<'_>,
    mut type_id: dir::GlobalTypeId,
) -> dir::GlobalTypeId {
    let mut active_type_ids = HashSet::new();

    loop {
        if !active_type_ids.insert(type_id) {
            return type_id;
        }

        let ty = ctx.checked_type(type_id).unwrap_or(dir::Type::Error);
        let dir::Type::Form(value) = ty else {
            return type_id;
        };
        type_id = value.value;
    }
}

/// Return truthiness certainty for one type with recursion protection and caching.
fn type_truthiness_inner(
    ctx: &LintModuleContext<'_>,
    strings: &StringPool,
    type_id: dir::GlobalTypeId,
    state: &mut TypeTruthinessState,
) -> TypeTruthiness {
    let normalized_type_id = normalized_flow_type_id(ctx, type_id);
    if let Some(cached_value) = state.cached_values.get(&normalized_type_id).copied() {
        return cached_value;
    }

    if !state.active_type_ids.insert(normalized_type_id) {
        return TypeTruthiness::Unknown;
    }

    let ty = ctx
        .checked_type(normalized_type_id)
        .unwrap_or(dir::Type::Error);
    let truthiness = if let Some(next_type_id) = value_like_type_id(&ty) {
        type_truthiness_inner(ctx, strings, next_type_id, state)
    } else if let dir::Type::Reference(reference) = ty {
        if let Some(next_type_id) = reference_symbol_type_id(ctx, reference.symbol) {
            type_truthiness_inner(ctx, strings, next_type_id, state)
        } else {
            TypeTruthiness::AlwaysTruthy
        }
    } else if let Some(element_type_ids) = union_or_intersection_elements(&ty) {
        combine_truthiness(
            element_type_ids
                .iter()
                .map(|type_id| type_truthiness_inner(ctx, strings, *type_id, state)),
        )
    } else {
        match ty {
            dir::Type::Never | dir::Type::Any | dir::Type::Unknown => TypeTruthiness::Unknown,
            dir::Type::Void | dir::Type::Null | dir::Type::Undefined => TypeTruthiness::AlwaysFalsy,
            dir::Type::Object => TypeTruthiness::AlwaysTruthy,
            dir::Type::Primitive(primitive) => match primitive {
                dir::PrimitiveType::Symbol | dir::PrimitiveType::UniqueSymbol => {
                    TypeTruthiness::AlwaysTruthy
                }
                _ => TypeTruthiness::Unknown,
            },
            dir::Type::Literal(literal) => match literal {
                dir::ScalarLiteral::Null => TypeTruthiness::AlwaysFalsy,
                dir::ScalarLiteral::Boolean(value) => {
                    if value {
                        TypeTruthiness::AlwaysTruthy
                    } else {
                        TypeTruthiness::AlwaysFalsy
                    }
                }
                dir::ScalarLiteral::Integer(value) => {
                    if value == 0 {
                        TypeTruthiness::AlwaysFalsy
                    } else {
                        TypeTruthiness::AlwaysTruthy
                    }
                }
                dir::ScalarLiteral::Bigint(value) => {
                    if value == 0 {
                        TypeTruthiness::AlwaysFalsy
                    } else {
                        TypeTruthiness::AlwaysTruthy
                    }
                }
                dir::ScalarLiteral::Float(value) => {
                    if value == 0.0 {
                        TypeTruthiness::AlwaysFalsy
                    } else {
                        TypeTruthiness::AlwaysTruthy
                    }
                }
                dir::ScalarLiteral::String(value) => {
                    if strings.get(value).is_empty() {
                        TypeTruthiness::AlwaysFalsy
                    } else {
                        TypeTruthiness::AlwaysTruthy
                    }
                }
                dir::ScalarLiteral::Character(_) | dir::ScalarLiteral::RegexString { .. } => {
                    TypeTruthiness::Unknown
                }
            },
            dir::Type::Operation(_) => TypeTruthiness::Unknown,
            dir::Type::Array(_)
            | dir::Type::Slice(_)
            | dir::Type::FixedArray(_)
            | dir::Type::Tuple(_)
            | dir::Type::Shape(_)
            | dir::Type::Closure(_)
            | dir::Type::Function(_) => TypeTruthiness::AlwaysTruthy,
            dir::Type::Parameter(_)
            | dir::Type::This
            | dir::Type::Dynamic(_)
            | dir::Type::Range(_)
            | dir::Type::Error => TypeTruthiness::Unknown,
            dir::Type::Reference(_)
            | dir::Type::Member(_)
            | dir::Type::Form(_)
            | dir::Type::Union(_)
            | dir::Type::Intersection(_) => TypeTruthiness::Unknown,
        }
    };

    state.active_type_ids.remove(&normalized_type_id);
    state.cached_values.insert(normalized_type_id, truthiness);
    truthiness
}

/// Return nullishness certainty for one type with recursion protection and caching.
fn type_nullishness_inner(
    ctx: &LintModuleContext<'_>,
    type_id: dir::GlobalTypeId,
    state: &mut TypeNullishnessState,
) -> TypeNullishness {
    let normalized_type_id = normalized_flow_type_id(ctx, type_id);
    if let Some(cached_value) = state.cached_values.get(&normalized_type_id).copied() {
        return cached_value;
    }

    if !state.active_type_ids.insert(normalized_type_id) {
        return TypeNullishness::Maybe;
    }

    let ty = ctx
        .checked_type(normalized_type_id)
        .unwrap_or(dir::Type::Error);
    let nullishness = if let Some(next_type_id) = value_like_type_id(&ty) {
        type_nullishness_inner(ctx, next_type_id, state)
    } else if let dir::Type::Reference(reference) = ty {
        if let Some(next_type_id) = reference_symbol_type_id(ctx, reference.symbol) {
            type_nullishness_inner(ctx, next_type_id, state)
        } else {
            TypeNullishness::Never
        }
    } else if let Some(element_type_ids) = union_or_intersection_elements(&ty) {
        combine_nullishness(
            element_type_ids
                .iter()
                .map(|type_id| type_nullishness_inner(ctx, *type_id, state)),
        )
    } else {
        match ty {
            dir::Type::Null | dir::Type::Undefined | dir::Type::Void => TypeNullishness::Always,
            dir::Type::Never | dir::Type::Any | dir::Type::Unknown => TypeNullishness::Maybe,
            dir::Type::Object
            | dir::Type::Primitive(_)
            | dir::Type::Range(_)
            | dir::Type::Literal(_) => TypeNullishness::Never,
            dir::Type::Slice(_)
            | dir::Type::Array(_)
            | dir::Type::FixedArray(_)
            | dir::Type::Tuple(_)
            | dir::Type::Shape(_)
            | dir::Type::Closure(_)
            | dir::Type::Function(_) => TypeNullishness::Never,
            dir::Type::Parameter(_)
            | dir::Type::This
            | dir::Type::Dynamic(_)
            | dir::Type::Operation(_)
            | dir::Type::Error => TypeNullishness::Maybe,
            dir::Type::Reference(_)
            | dir::Type::Member(_)
            | dir::Type::Form(_)
            | dir::Type::Union(_)
            | dir::Type::Intersection(_) => TypeNullishness::Maybe,
        }
    };

    state.active_type_ids.remove(&normalized_type_id);
    state.cached_values.insert(normalized_type_id, nullishness);
    nullishness
}

/// Combine truthiness values from one type element set.
fn combine_truthiness(values: impl Iterator<Item = TypeTruthiness>) -> TypeTruthiness {
    let mut saw_truthy = false;
    let mut saw_falsy = false;

    for value in values {
        match value {
            TypeTruthiness::AlwaysTruthy => saw_truthy = true,
            TypeTruthiness::AlwaysFalsy => saw_falsy = true,
            TypeTruthiness::Unknown => return TypeTruthiness::Unknown,
        }
    }

    match (saw_truthy, saw_falsy) {
        (true, false) => TypeTruthiness::AlwaysTruthy,
        (false, true) => TypeTruthiness::AlwaysFalsy,
        _ => TypeTruthiness::Unknown,
    }
}

/// Combine nullishness values from one type element set.
fn combine_nullishness(values: impl Iterator<Item = TypeNullishness>) -> TypeNullishness {
    let mut saw_nullish = false;
    let mut saw_non_nullish = false;

    for value in values {
        match value {
            TypeNullishness::Always => saw_nullish = true,
            TypeNullishness::Never => saw_non_nullish = true,
            TypeNullishness::Maybe => return TypeNullishness::Maybe,
        }
    }

    match (saw_nullish, saw_non_nullish) {
        (true, false) => TypeNullishness::Always,
        (false, true) => TypeNullishness::Never,
        _ => TypeNullishness::Maybe,
    }
}

/// Resolve the parameter type at an index for a function-like type.
pub fn function_parameter_type_at(
    ctx: &LintModuleContext<'_>,
    type_id: dir::GlobalTypeId,
    index: usize,
) -> Option<dir::GlobalTypeId> {
    let normalized_type_id = normalized_flow_type_id(ctx, type_id);
    let mut state = TypeQueryState::new();
    function_parameter_type_at_inner(ctx, normalized_type_id, index, &mut state)
}

/// Resolve all parameter types at an index for a function-like type.
pub fn function_parameter_types_at(
    ctx: &LintModuleContext<'_>,
    type_id: dir::GlobalTypeId,
    index: usize,
) -> Vec<dir::GlobalTypeId> {
    // prefer flow normalized types when available
    let normalized_type_id = normalized_flow_type_id(ctx, type_id);

    let mut state = TypeQueryState::new();
    let mut results = Vec::new();
    function_parameter_types_at_inner(ctx, normalized_type_id, index, &mut state, &mut results);

    results
}

/// Resolve the return type for a function-like type.
pub fn function_return_type(
    ctx: &LintModuleContext<'_>,
    type_id: dir::GlobalTypeId,
) -> Option<dir::GlobalTypeId> {
    let normalized_type_id = normalized_flow_type_id(ctx, type_id);
    let mut state = TypeQueryState::new();
    function_return_type_inner(ctx, normalized_type_id, &mut state)
}

/// Return true when the type may include one nominal symbol.
fn type_may_be_nominal_symbol(
    ctx: &LintModuleContext<'_>,
    type_id: dir::GlobalTypeId,
    symbol: Option<dir::GlobalSymbolId>,
) -> bool {
    let Some(symbol) = symbol else {
        return false;
    };

    // prefer flow normalized types when available
    let normalized_type_id = normalized_flow_type_id(ctx, type_id);

    // track visited type ids to avoid recursion cycles
    let mut state = TypeQueryState::new();

    // resolve whether the type may include the symbol
    type_may_be_nominal_symbol_inner(ctx, normalized_type_id, symbol, &mut state)
}

/// Return true when the type may include one nominal symbol with cycle protection.
fn type_may_be_nominal_symbol_inner(
    ctx: &LintModuleContext<'_>,
    type_id: dir::GlobalTypeId,
    symbol: dir::GlobalSymbolId,
    state: &mut TypeQueryState,
) -> bool {
    // stop recursive cycles
    if !state.enter_type_id(type_id) {
        return false;
    }

    // inspect the type node
    let ty = ctx.checked_type(type_id).unwrap_or(dir::Type::Error);
    let result = match ty {
        dir::Type::Reference(reference) => {
            if reference.symbol == symbol
                || symbol_matches_relation_target(ctx, reference.symbol, symbol)
            {
                true
            } else if let Some(type_id) = ctx.symbol_type_id(reference.symbol) {
                type_may_be_nominal_symbol_inner(ctx, type_id, symbol, state)
            } else {
                false
            }
        }
        dir::Type::Form(value) => type_may_be_nominal_symbol_inner(ctx, value.value, symbol, state),
        dir::Type::Union(union) => union
            .elements
            .iter()
            .any(|element| type_may_be_nominal_symbol_inner(ctx, *element, symbol, state)),
        dir::Type::Intersection(intersection) => intersection
            .elements
            .iter()
            .any(|element| type_may_be_nominal_symbol_inner(ctx, *element, symbol, state)),
        _ => false,
    };

    state.leave_type_id(type_id);
    result
}

/// Return true when one symbol matches a relation target symbol.
fn symbol_matches_relation_target(
    _ctx: &LintModuleContext<'_>,
    symbol: dir::GlobalSymbolId,
    target_symbol: dir::GlobalSymbolId,
) -> bool {
    symbol == target_symbol
}

/// Resolve a parameter type for a function type with cycle protection.
fn function_parameter_type_at_inner(
    ctx: &LintModuleContext<'_>,
    type_id: dir::GlobalTypeId,
    index: usize,
    state: &mut TypeQueryState,
) -> Option<dir::GlobalTypeId> {
    // stop recursive cycles
    if !state.enter_type_id(type_id) {
        return None;
    }

    // inspect the type node
    let ty = ctx.checked_type(type_id).unwrap_or(dir::Type::Error);
    let result = match ty {
        dir::Type::Function(function) => {
            function.parameters.get(index).map(|parameter| parameter.ty)
        }
        dir::Type::Shape(object) => {
            object
                .call_signatures
                .first()
                .copied()
                .and_then(|first_signature| {
                    function_parameter_type_at_inner(ctx, first_signature, index, state)
                })
        }
        dir::Type::Form(value) => function_parameter_type_at_inner(ctx, value.value, index, state),
        dir::Type::Reference(reference) => {
            if let Some(next_type_id) = reference_symbol_type_id(ctx, reference.symbol) {
                function_parameter_type_at_inner(ctx, next_type_id, index, state)
            } else {
                None
            }
        }
        _ => None,
    };

    state.leave_type_id(type_id);
    result
}

/// Resolve all parameter types for a function type with cycle protection.
fn function_parameter_types_at_inner(
    ctx: &LintModuleContext<'_>,
    type_id: dir::GlobalTypeId,
    index: usize,
    state: &mut TypeQueryState,
    results: &mut Vec<dir::GlobalTypeId>,
) {
    // stop recursive cycles
    if !state.enter_type_id(type_id) {
        return;
    }

    // inspect the type node
    let ty = ctx.checked_type(type_id).unwrap_or(dir::Type::Error);
    match ty {
        dir::Type::Function(function) => {
            if let Some(parameter_type_id) =
                function.parameters.get(index).map(|parameter| parameter.ty)
                && !results.contains(&parameter_type_id)
            {
                results.push(parameter_type_id);
            }
        }
        dir::Type::Shape(object) => {
            for signature_id in &object.call_signatures {
                function_parameter_types_at_inner(ctx, *signature_id, index, state, results);
            }
        }
        dir::Type::Union(union) => {
            for element_type_id in &union.elements {
                function_parameter_types_at_inner(ctx, *element_type_id, index, state, results);
            }
        }
        dir::Type::Intersection(intersection) => {
            for element_type_id in &intersection.elements {
                function_parameter_types_at_inner(ctx, *element_type_id, index, state, results);
            }
        }
        dir::Type::Form(value) => {
            function_parameter_types_at_inner(ctx, value.value, index, state, results);
        }
        dir::Type::Reference(reference) => {
            for_each_reference_symbol_type_id(ctx, reference.symbol, |next_type_id| {
                function_parameter_types_at_inner(ctx, next_type_id, index, state, results);
            });
        }
        _ => {}
    }

    state.leave_type_id(type_id);
}

/// Resolve a return type for a function type with cycle protection.
fn function_return_type_inner(
    ctx: &LintModuleContext<'_>,
    type_id: dir::GlobalTypeId,
    state: &mut TypeQueryState,
) -> Option<dir::GlobalTypeId> {
    // stop recursive cycles
    if !state.enter_type_id(type_id) {
        return None;
    }

    // inspect the type node
    let ty = ctx.checked_type(type_id).unwrap_or(dir::Type::Error);
    let result = match ty {
        dir::Type::Function(function) => function.return_type,
        dir::Type::Shape(object) => object
            .call_signatures
            .first()
            .copied()
            .and_then(|first_signature| function_return_type_inner(ctx, first_signature, state)),
        dir::Type::Form(value) => function_return_type_inner(ctx, value.value, state),
        dir::Type::Reference(reference) => {
            if let Some(next_type_id) = reference_symbol_type_id(ctx, reference.symbol) {
                function_return_type_inner(ctx, next_type_id, state)
            } else {
                None
            }
        }
        _ => None,
    };

    state.leave_type_id(type_id);
    result
}
