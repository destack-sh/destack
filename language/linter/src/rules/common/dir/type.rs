use std::collections::{HashMap, HashSet};

use destack_core::StringPool;
use destack_dir as dir;

/// The default relation cache key used for flow normalization.
///
/// This matches the assign relation cache key used by compiler type normalization.
pub const DEFAULT_RELATION_CACHE_KEY: u64 = 0;

/// Normalize one type id with flow mode and the default relation cache key.
pub fn normalized_flow_type_id(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
) -> dir::LocalTypeId {
    types.get_normalized_type_id_or(
        dir::NormalizationMode::Flow,
        DEFAULT_RELATION_CACHE_KEY,
        type_id,
    )
}

/// Return true when two types are equivalent after flow normalization.
///
/// This also treats `any` and `unknown` as compatible escape hatches.
pub fn types_are_equivalent_or_any(
    types: &dir::TypeTable,
    left_type_id: dir::LocalTypeId,
    right_type_id: dir::LocalTypeId,
) -> bool {
    let left_normalized = normalized_flow_type_id(types, left_type_id);
    let right_normalized = normalized_flow_type_id(types, right_type_id);

    if left_normalized == right_normalized {
        return true;
    }

    is_any_type(types, left_normalized) || is_any_type(types, right_normalized)
}

/// Shared traversal state for recursive type queries.
struct TypeQueryState {
    /// Type ids in the active recursion stack.
    active_type_ids: HashSet<dir::LocalTypeId>,
}

impl TypeQueryState {
    /// Build an empty query state.
    fn new() -> Self {
        Self {
            active_type_ids: HashSet::new(),
        }
    }

    /// Enter one type id and return false on recursive cycles.
    fn enter_type_id(&mut self, type_id: dir::LocalTypeId) -> bool {
        self.active_type_ids.insert(type_id)
    }

    /// Leave one type id after query evaluation.
    fn leave_type_id(&mut self, type_id: dir::LocalTypeId) {
        let did_remove = self.active_type_ids.remove(&type_id);
        debug_assert!(did_remove);
    }
}

/// Shared traversal state for recursive symbol lineage queries.
struct SymbolQueryState {
    /// Symbol ids in the active recursion stack.
    active_symbol_ids: HashSet<dir::GlobalSymbolId>,
}

impl SymbolQueryState {
    /// Build an empty symbol query state.
    fn new() -> Self {
        Self {
            active_symbol_ids: HashSet::new(),
        }
    }

    /// Enter one symbol id and return false on recursive cycles.
    fn enter_symbol_id(&mut self, symbol_id: dir::GlobalSymbolId) -> bool {
        self.active_symbol_ids.insert(symbol_id)
    }

    /// Leave one symbol id after query evaluation.
    fn leave_symbol_id(&mut self, symbol_id: dir::GlobalSymbolId) {
        let did_remove = self.active_symbol_ids.remove(&symbol_id);
        debug_assert!(did_remove);
    }
}

/// Resolve the next type id for value-like wrappers.
fn value_like_type_id(ty: &dir::Type) -> Option<dir::LocalTypeId> {
    match ty {
        dir::Type::Value { value } => Some(*value),
        dir::Type::ValueOf { right, .. }
        | dir::Type::ReferenceOf { right, .. }
        | dir::Type::PointerOf { right, .. } => Some(*right),
        _ => None,
    }
}

/// Resolve union or intersection element type ids.
fn union_or_intersection_elements(ty: &dir::Type) -> Option<&[dir::LocalTypeId]> {
    match ty {
        dir::Type::Union { elements } | dir::Type::Intersection { elements } => {
            Some(elements.as_slice())
        }
        _ => None,
    }
}

/// Resolve the primary target type id for one reference symbol.
///
/// This follows instance types first and then value types for aliases.
fn primary_reference_target_type_id(
    types: &dir::TypeTable,
    symbol: dir::GlobalSymbolId,
) -> Option<dir::LocalTypeId> {
    types
        .get_instance_type_id(symbol)
        .or_else(|| types.get_value_type_id(symbol))
}

/// Visit all available reference target type ids for one symbol.
fn for_each_reference_target_type_id(
    types: &dir::TypeTable,
    symbol: dir::GlobalSymbolId,
    mut visitor: impl FnMut(dir::LocalTypeId),
) {
    if let Some(instance_type_id) = types.get_instance_type_id(symbol) {
        visitor(instance_type_id);
    }

    if let Some(value_type_id) = types.get_value_type_id(symbol) {
        visitor(value_type_id);
    }
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
    ReferenceSymbolType {
        /// Required symbol type.
        symbol_type: dir::SymbolType,
    },
    /// Check infer variable compatibility.
    InferVar,
    /// Check Promise spread element compatibility.
    PromiseSpreadElementCompatible {
        /// Promise symbol for declared library references.
        promise_symbol: dir::GlobalSymbolId,
    },
    /// Check map references with empty value type arguments.
    MapWithEmptyValue {
        /// Candidate map symbols.
        map_symbols: &'a [dir::GlobalSymbolId],
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
    active_type_ids: HashSet<dir::LocalTypeId>,
    /// Cached query results by local type id.
    cached_results: HashMap<u32, bool>,
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
    fn cached_result(&self, type_id: dir::LocalTypeId) -> Option<bool> {
        self.cached_results.get(&type_id.0).copied()
    }

    /// Enter one type id and return false on recursive cycles.
    fn enter_type_id(&mut self, type_id: dir::LocalTypeId) -> bool {
        self.active_type_ids.insert(type_id)
    }

    /// Leave one type id after query evaluation.
    fn leave_type_id(&mut self, type_id: dir::LocalTypeId) {
        let did_remove = self.active_type_ids.remove(&type_id);
        debug_assert!(did_remove);
    }

    /// Cache one query result for a type id.
    fn cache_result(&mut self, type_id: dir::LocalTypeId, result: bool) {
        self.cached_results.insert(type_id.0, result);
    }
}

/// Evaluate one boolean type query from one source type id.
fn evaluate_boolean_type_query(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    query: TypeBooleanQuery<'_>,
) -> bool {
    let normalized_type_id = normalized_flow_type_id(types, type_id);
    let mut state = TypeBooleanQueryState::new();
    evaluate_boolean_type_query_inner(types, normalized_type_id, query, &mut state)
}

/// Evaluate one boolean type query for one normalized type id.
fn evaluate_boolean_type_query_inner(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    query: TypeBooleanQuery<'_>,
    state: &mut TypeBooleanQueryState,
) -> bool {
    let normalized_type_id = normalized_flow_type_id(types, type_id);

    if let Some(cached_result) = state.cached_result(normalized_type_id) {
        return cached_result;
    }

    if !state.enter_type_id(normalized_type_id) {
        return false;
    }

    let ty = types.get_type(normalized_type_id);
    let result = if let Some(next_type_id) = value_like_type_id(ty) {
        evaluate_boolean_type_query_inner(types, next_type_id, query, state)
    } else if let dir::Type::Reference {
        symbol,
        static_arguments,
    } = ty
    {
        evaluate_reference_boolean_type_query(
            types,
            *symbol,
            static_arguments.as_deref(),
            query,
            state,
        )
    } else if let Some(element_type_ids) = union_or_intersection_elements(ty) {
        let composition_policy = type_query_composition_policy(query, ty);
        aggregate_boolean_query_results(types, element_type_ids, query, composition_policy, state)
    } else {
        evaluate_terminal_boolean_type_query(types, ty, query, state)
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
        dir::Type::Union { .. } => match query {
            TypeBooleanQuery::Any
            | TypeBooleanQuery::ExplicitAny
            | TypeBooleanQuery::PromiseOrAny { .. }
            | TypeBooleanQuery::ReferenceSymbolType { .. }
            | TypeBooleanQuery::InferVar
            | TypeBooleanQuery::PromiseSpreadElementCompatible { .. }
            | TypeBooleanQuery::MapWithEmptyValue { .. }
            | TypeBooleanQuery::HasThisParameter
            | TypeBooleanQuery::MaybeNullish
            | TypeBooleanQuery::HasNonNullishFalsy { .. } => TypeCompositionPolicy::Any,
            _ => TypeCompositionPolicy::All,
        },
        dir::Type::Intersection { .. } => match query {
            TypeBooleanQuery::Promise { .. }
            | TypeBooleanQuery::PromiseOrAny { .. }
            | TypeBooleanQuery::ReferenceSymbolType { .. }
            | TypeBooleanQuery::InferVar
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
    types: &dir::TypeTable,
    element_type_ids: &[dir::LocalTypeId],
    query: TypeBooleanQuery<'_>,
    composition_policy: TypeCompositionPolicy,
    state: &mut TypeBooleanQueryState,
) -> bool {
    match composition_policy {
        TypeCompositionPolicy::All => element_type_ids
            .iter()
            .all(|type_id| evaluate_boolean_type_query_inner(types, *type_id, query, state)),
        TypeCompositionPolicy::Any => element_type_ids
            .iter()
            .any(|type_id| evaluate_boolean_type_query_inner(types, *type_id, query, state)),
    }
}

/// Evaluate one boolean type query for one reference symbol.
fn evaluate_reference_boolean_type_query(
    types: &dir::TypeTable,
    symbol: dir::GlobalSymbolId,
    static_arguments: Option<&[dir::StaticArgument]>,
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
        TypeBooleanQuery::ReferenceSymbolType { symbol_type } => {
            if symbol.ty() == symbol_type {
                return true;
            }
        }
        TypeBooleanQuery::PromiseSpreadElementCompatible { promise_symbol } => {
            if symbol == promise_symbol {
                return true;
            }
        }
        TypeBooleanQuery::MapWithEmptyValue { map_symbols } => {
            if map_symbols.contains(&symbol)
                && static_arguments_contain_empty_map_value(types, static_arguments)
            {
                return true;
            }
        }
        TypeBooleanQuery::HasUsefulToString => {
            if !matches!(symbol.ty(), dir::SymbolType::Void) {
                return true;
            }
        }
        TypeBooleanQuery::DefinitelyNonErrorValue {
            error_symbol,
            result_symbol,
        } => {
            if error_symbol
                .is_some_and(|error_symbol| symbol_lineage_contains(types, symbol, error_symbol))
            {
                return false;
            }

            if result_symbol
                .is_some_and(|result_symbol| symbol_lineage_contains(types, symbol, result_symbol))
            {
                return true;
            }
        }
        _ => {}
    }

    if let Some(next_type_id) = primary_reference_target_type_id(types, symbol) {
        return evaluate_boolean_type_query_inner(types, next_type_id, query, state);
    }

    matches!(query, TypeBooleanQuery::HasNonNullishFalsy { .. })
}

/// Evaluate one boolean type query for one terminal type node.
fn evaluate_terminal_boolean_type_query(
    types: &dir::TypeTable,
    ty: &dir::Type,
    query: TypeBooleanQuery<'_>,
    state: &mut TypeBooleanQueryState,
) -> bool {
    match query {
        TypeBooleanQuery::StrictBoolean => matches!(
            ty,
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Primitive(dir::PrimitiveType::Boolean)
                    | dir::TypeLiteral::ScalarLiteral(dir::ScalarLiteral::Boolean(_))
            }
        ),
        TypeBooleanQuery::Array { .. } => matches!(
            ty,
            dir::Type::Array { .. } | dir::Type::ArraySized { .. } | dir::Type::Tuple { .. }
        ),
        TypeBooleanQuery::String { .. } => match ty {
            dir::Type::TemplateLiteral { .. } => true,
            dir::Type::TypeLiteral { value } => type_literal_is_string_like(value),
            _ => false,
        },
        TypeBooleanQuery::Float => matches!(
            ty,
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Primitive(dir::PrimitiveType::Float(_))
                    | dir::TypeLiteral::Primitive(dir::PrimitiveType::Number)
            }
        ),
        TypeBooleanQuery::Function => match ty {
            dir::Type::Function { .. } => true,
            dir::Type::Object {
                call_signatures, ..
            } => !call_signatures.is_empty(),
            _ => false,
        },
        TypeBooleanQuery::HasThisParameter => match ty {
            dir::Type::Function { this_parameter, .. } => this_parameter.is_some(),
            _ => false,
        },
        TypeBooleanQuery::ReferenceSymbolType { .. } => false,
        TypeBooleanQuery::InferVar => matches!(ty, dir::Type::InferVar { .. }),
        TypeBooleanQuery::PromiseSpreadElementCompatible { promise_symbol } => match ty {
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Any | dir::TypeLiteral::Unknown,
            } => true,
            dir::Type::Array { element, .. } => element.is_some_and(|element_type_id| {
                evaluate_boolean_type_query_inner(
                    types,
                    element_type_id,
                    TypeBooleanQuery::PromiseOrAny { promise_symbol },
                    state,
                )
            }),
            dir::Type::ArraySized { element, .. } => evaluate_boolean_type_query_inner(
                types,
                *element,
                TypeBooleanQuery::PromiseOrAny { promise_symbol },
                state,
            ),
            dir::Type::Tuple { elements, .. } => elements.iter().any(|element| {
                evaluate_boolean_type_query_inner(
                    types,
                    element.ty,
                    TypeBooleanQuery::PromiseOrAny { promise_symbol },
                    state,
                )
            }),
            _ => false,
        },
        TypeBooleanQuery::MapWithEmptyValue { .. } => false,
        TypeBooleanQuery::HasUsefulToString => match ty {
            dir::Type::TypeLiteral { value } => {
                matches!(
                    value,
                    dir::TypeLiteral::Primitive(dir::PrimitiveType::String)
                        | dir::TypeLiteral::Primitive(dir::PrimitiveType::Number)
                        | dir::TypeLiteral::Primitive(dir::PrimitiveType::Boolean)
                        | dir::TypeLiteral::Primitive(dir::PrimitiveType::Int(_))
                        | dir::TypeLiteral::Primitive(dir::PrimitiveType::Float(_))
                        | dir::TypeLiteral::Primitive(dir::PrimitiveType::Bigint)
                        | dir::TypeLiteral::ScalarLiteral(_)
                )
            }
            dir::Type::Array { element, .. } => element.is_none_or(|element_type_id| {
                evaluate_boolean_type_query_inner(
                    types,
                    element_type_id,
                    TypeBooleanQuery::HasUsefulToString,
                    state,
                )
            }),
            dir::Type::ArraySized { element, .. } => evaluate_boolean_type_query_inner(
                types,
                *element,
                TypeBooleanQuery::HasUsefulToString,
                state,
            ),
            dir::Type::Tuple { elements, .. } => elements.iter().all(|element| {
                evaluate_boolean_type_query_inner(
                    types,
                    element.ty,
                    TypeBooleanQuery::HasUsefulToString,
                    state,
                )
            }),
            dir::Type::Function { .. } => true,
            dir::Type::Object { .. } => false,
            dir::Type::Error => true,
            _ => false,
        },
        TypeBooleanQuery::AsyncFunction => match ty {
            dir::Type::Function { asynchrony, .. } => *asynchrony == dir::Asynchrony::Async,
            dir::Type::Object {
                call_signatures, ..
            } => call_signatures.iter().any(|type_id| {
                evaluate_boolean_type_query_inner(
                    types,
                    *type_id,
                    TypeBooleanQuery::AsyncFunction,
                    state,
                )
            }),
            _ => false,
        },
        TypeBooleanQuery::Any => matches!(
            ty,
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Any | dir::TypeLiteral::Unknown
            }
        ),
        TypeBooleanQuery::PromiseOrAny { .. } => matches!(
            ty,
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Any | dir::TypeLiteral::Unknown
            }
        ),
        TypeBooleanQuery::ExplicitAny => matches!(
            ty,
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Any
            }
        ),
        TypeBooleanQuery::Promise { .. } => false,
        TypeBooleanQuery::VoidOrNever => matches!(
            ty,
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Void | dir::TypeLiteral::Never
            }
        ),
        TypeBooleanQuery::TemplateInterpolation { .. } => match ty {
            dir::Type::TemplateLiteral { .. } => true,
            dir::Type::TypeLiteral { value } => {
                type_literal_is_string_like(value)
                    || matches!(
                        value,
                        dir::TypeLiteral::Primitive(
                            dir::PrimitiveType::Bigint
                                | dir::PrimitiveType::Number
                                | dir::PrimitiveType::Int(_)
                                | dir::PrimitiveType::Float(_),
                        ) | dir::TypeLiteral::ScalarLiteral(
                            dir::ScalarLiteral::Integer(_)
                                | dir::ScalarLiteral::Bigint(_)
                                | dir::ScalarLiteral::Float(_),
                        )
                    )
            }
            _ => false,
        },
        TypeBooleanQuery::StringLikePropertyKey => match ty {
            dir::Type::TypeLiteral { value } => type_literal_is_string_like_property_key(value),
            _ => false,
        },
        TypeBooleanQuery::NumericPropertyKey => match ty {
            dir::Type::TypeLiteral { value } => type_literal_is_numeric_property_key(value),
            _ => false,
        },
        TypeBooleanQuery::SymbolLikePropertyKey => match ty {
            dir::Type::TypeLiteral { value } => type_literal_is_symbol_like_property_key(value),
            _ => false,
        },
        TypeBooleanQuery::DefinitelyNonErrorValue { .. } => matches!(
            ty,
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Never
                    | dir::TypeLiteral::Undefined
                    | dir::TypeLiteral::Void
                    | dir::TypeLiteral::Null
                    | dir::TypeLiteral::Primitive(_)
                    | dir::TypeLiteral::ScalarLiteral(_)
            }
        ),
        TypeBooleanQuery::MaybeNullish => matches!(
            ty,
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Null
                    | dir::TypeLiteral::Undefined
                    | dir::TypeLiteral::Void
                    | dir::TypeLiteral::Any
                    | dir::TypeLiteral::Infer
                    | dir::TypeLiteral::Unknown
            } | dir::Type::InferVar { .. }
                | dir::Type::Conditional { .. }
                | dir::Type::Mapped { .. }
                | dir::Type::Index { .. }
                | dir::Type::TemplateLiteral { .. }
                | dir::Type::Import { .. }
                | dir::Type::Infer { .. }
                | dir::Type::Predicate { .. }
                | dir::Type::Readonly { .. }
                | dir::Type::KeyOf { .. }
                | dir::Type::Must { .. }
                | dir::Type::AsComptime { .. }
                | dir::Type::Not { .. }
                | dir::Type::In { .. }
                | dir::Type::Extends { .. }
                | dir::Type::Implements { .. }
                | dir::Type::Error
                | dir::Type::Unevaluated(_)
        ),
        TypeBooleanQuery::HasNonNullishFalsy { strings } => match ty {
            dir::Type::TypeLiteral { value } => match value {
                dir::TypeLiteral::Null | dir::TypeLiteral::Undefined | dir::TypeLiteral::Void => {
                    false
                }
                dir::TypeLiteral::Never => false,
                dir::TypeLiteral::Any | dir::TypeLiteral::Infer | dir::TypeLiteral::Unknown => true,
                dir::TypeLiteral::Object => false,
                dir::TypeLiteral::Primitive(primitive) => matches!(
                    primitive,
                    dir::PrimitiveType::Boolean
                        | dir::PrimitiveType::Number
                        | dir::PrimitiveType::Bigint
                        | dir::PrimitiveType::String
                        | dir::PrimitiveType::Int(_)
                        | dir::PrimitiveType::Float(_)
                ),
                dir::TypeLiteral::ScalarLiteral(literal) => match literal {
                    dir::ScalarLiteral::Null => true,
                    dir::ScalarLiteral::Boolean(false) => true,
                    dir::ScalarLiteral::Boolean(true) => false,
                    dir::ScalarLiteral::Integer(value) => *value == 0,
                    dir::ScalarLiteral::Bigint(value) => *value == 0,
                    dir::ScalarLiteral::Float(value) => *value == 0.0,
                    dir::ScalarLiteral::String(value) => strings.get(*value).is_empty(),
                    dir::ScalarLiteral::Character(_) | dir::ScalarLiteral::RegexString { .. } => {
                        true
                    }
                },
                dir::TypeLiteral::Intrinsic(_) => true,
            },
            dir::Type::Array { .. }
            | dir::Type::ArraySized { .. }
            | dir::Type::Tuple { .. }
            | dir::Type::Object { .. }
            | dir::Type::Function { .. } => false,
            dir::Type::InferVar { .. }
            | dir::Type::This
            | dir::Type::Unevaluated(_)
            | dir::Type::Conditional { .. }
            | dir::Type::Mapped { .. }
            | dir::Type::Index { .. }
            | dir::Type::TemplateLiteral { .. }
            | dir::Type::Import { .. }
            | dir::Type::Infer { .. }
            | dir::Type::Predicate { .. }
            | dir::Type::Readonly { .. }
            | dir::Type::KeyOf { .. }
            | dir::Type::Must { .. }
            | dir::Type::AsComptime { .. }
            | dir::Type::Not { .. }
            | dir::Type::In { .. }
            | dir::Type::Extends { .. }
            | dir::Type::Implements { .. }
            | dir::Type::Error => true,
            _ => false,
        },
    }
}

/// Return true when static arguments contain an empty map value argument.
fn static_arguments_contain_empty_map_value(
    types: &dir::TypeTable,
    static_arguments: Option<&[dir::StaticArgument]>,
) -> bool {
    let Some(static_arguments) = static_arguments else {
        return false;
    };
    if static_arguments.len() != 2 {
        return false;
    }

    static_argument_is_void_or_never_type(types, &static_arguments[1])
}

/// Return true when one static argument resolves to `void` or `never`.
fn static_argument_is_void_or_never_type(
    types: &dir::TypeTable,
    static_argument: &dir::StaticArgument,
) -> bool {
    match static_argument {
        dir::StaticArgument::Evaluated { value, .. } => match value {
            dir::StaticExpression::Type { ty } => is_void_or_never_type(types, *ty),
            dir::StaticExpression::TypeLiteral {
                value: dir::TypeLiteral::Void | dir::TypeLiteral::Never,
            } => true,
            _ => false,
        },
        dir::StaticArgument::Unevaluated { node } => types
            .get_declared_or_inferred_type_id(*node)
            .is_some_and(|type_id| is_void_or_never_type(types, type_id)),
    }
}

/// Return true when one type literal is string-like.
fn type_literal_is_string_like(value: &dir::TypeLiteral) -> bool {
    matches!(
        value,
        dir::TypeLiteral::Primitive(dir::PrimitiveType::String)
            | dir::TypeLiteral::ScalarLiteral(dir::ScalarLiteral::String(_))
            | dir::TypeLiteral::Intrinsic(
                dir::IntrinsicType::Uppercase
                    | dir::IntrinsicType::Lowercase
                    | dir::IntrinsicType::Capitalize
                    | dir::IntrinsicType::Uncapitalize
            )
    )
}

/// Return true when one type literal is string-like for object property keys.
fn type_literal_is_string_like_property_key(value: &dir::TypeLiteral) -> bool {
    matches!(
        value,
        dir::TypeLiteral::Primitive(dir::PrimitiveType::String)
            | dir::TypeLiteral::Primitive(dir::PrimitiveType::Boolean)
            | dir::TypeLiteral::Primitive(dir::PrimitiveType::Character)
            | dir::TypeLiteral::ScalarLiteral(dir::ScalarLiteral::String(_))
            | dir::TypeLiteral::ScalarLiteral(dir::ScalarLiteral::Boolean(_))
            | dir::TypeLiteral::ScalarLiteral(dir::ScalarLiteral::Character(_))
            | dir::TypeLiteral::ScalarLiteral(dir::ScalarLiteral::RegexString { .. })
            | dir::TypeLiteral::Null
            | dir::TypeLiteral::Undefined
    )
}

/// Return true when one type literal is numeric for object property keys.
fn type_literal_is_numeric_property_key(value: &dir::TypeLiteral) -> bool {
    matches!(
        value,
        dir::TypeLiteral::Primitive(dir::PrimitiveType::Number)
            | dir::TypeLiteral::Primitive(dir::PrimitiveType::Bigint)
            | dir::TypeLiteral::Primitive(dir::PrimitiveType::Int(_))
            | dir::TypeLiteral::Primitive(dir::PrimitiveType::Float(_))
            | dir::TypeLiteral::ScalarLiteral(dir::ScalarLiteral::Integer(_))
            | dir::TypeLiteral::ScalarLiteral(dir::ScalarLiteral::Float(_))
            | dir::TypeLiteral::ScalarLiteral(dir::ScalarLiteral::Bigint(_))
    )
}

/// Return true when one type literal is symbol-like for object property keys.
fn type_literal_is_symbol_like_property_key(value: &dir::TypeLiteral) -> bool {
    matches!(
        value,
        dir::TypeLiteral::Primitive(dir::PrimitiveType::Symbol)
            | dir::TypeLiteral::Primitive(dir::PrimitiveType::UniqueSymbol)
    )
}

/// Return true when the type is strictly boolean.
pub fn is_strict_boolean_type(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> bool {
    evaluate_boolean_type_query(types, type_id, TypeBooleanQuery::StrictBoolean)
}

/// Return true when the type is an array type.
pub fn is_array_type(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    array_symbol: Option<dir::GlobalSymbolId>,
) -> bool {
    evaluate_boolean_type_query(types, type_id, TypeBooleanQuery::Array { array_symbol })
}

/// Return the fixed arity when one type resolves to a tuple.
pub fn tuple_type_arity(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> Option<usize> {
    let mut current_type_id = normalized_flow_type_id(types, type_id);
    let mut visited_type_ids = HashSet::new();

    loop {
        if !visited_type_ids.insert(current_type_id) {
            return None;
        }

        let current_type = types.get_type(current_type_id);
        match current_type {
            dir::Type::Tuple { elements, .. } => return Some(elements.len()),
            dir::Type::Value { value } => {
                current_type_id = *value;
            }
            dir::Type::ValueOf { right, .. }
            | dir::Type::ReferenceOf { right, .. }
            | dir::Type::PointerOf { right, .. } => {
                current_type_id = *right;
            }
            dir::Type::Reference { symbol, .. } => {
                current_type_id = primary_reference_target_type_id(types, *symbol)?;
            }
            _ => return None,
        }
    }
}

/// Return true when the type is an array or tuple whose elements are strings.
pub fn is_string_array_type(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    array_symbol: Option<dir::GlobalSymbolId>,
    string_symbol: Option<dir::GlobalSymbolId>,
) -> bool {
    let mut state = TypeQueryState::new();
    is_string_array_type_inner(types, type_id, array_symbol, string_symbol, &mut state)
}

/// Evaluate string-array compatibility recursively.
fn is_string_array_type_inner(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    array_symbol: Option<dir::GlobalSymbolId>,
    string_symbol: Option<dir::GlobalSymbolId>,
    state: &mut TypeQueryState,
) -> bool {
    let normalized_type_id = normalized_flow_type_id(types, type_id);
    if !state.enter_type_id(normalized_type_id) {
        return false;
    }

    let ty = types.get_type(normalized_type_id);
    let result = match ty {
        dir::Type::Array { element, .. } => element
            .is_some_and(|element_type_id| is_string_type(types, element_type_id, string_symbol)),
        dir::Type::ArraySized { element, .. } => is_string_type(types, *element, string_symbol),
        dir::Type::Tuple { elements, .. } => elements
            .iter()
            .all(|element| is_string_type(types, element.ty, string_symbol)),
        dir::Type::Reference {
            symbol,
            static_arguments,
        } => {
            if array_symbol.is_none_or(|array_symbol| *symbol != array_symbol) {
                false
            } else {
                static_arguments.as_ref().is_some_and(|static_arguments| {
                    static_arguments.first().is_some_and(|static_argument| {
                        static_argument_type_id(types, static_argument).is_some_and(
                            |element_type_id| is_string_type(types, element_type_id, string_symbol),
                        )
                    })
                })
            }
        }
        dir::Type::Union { elements } => elements.iter().all(|element_type_id| {
            is_string_array_type_inner(types, *element_type_id, array_symbol, string_symbol, state)
        }),
        dir::Type::Intersection { elements } => elements.iter().any(|element_type_id| {
            is_string_array_type_inner(types, *element_type_id, array_symbol, string_symbol, state)
        }),
        dir::Type::Value { value } => {
            is_string_array_type_inner(types, *value, array_symbol, string_symbol, state)
        }
        dir::Type::ValueOf { right, .. }
        | dir::Type::ReferenceOf { right, .. }
        | dir::Type::PointerOf { right, .. } => {
            is_string_array_type_inner(types, *right, array_symbol, string_symbol, state)
        }
        _ => false,
    };

    state.leave_type_id(normalized_type_id);
    result
}

/// Resolve one static argument into a concrete type id when available.
fn static_argument_type_id(
    types: &dir::TypeTable,
    static_argument: &dir::StaticArgument,
) -> Option<dir::LocalTypeId> {
    match static_argument {
        dir::StaticArgument::Evaluated { value, .. } => match value {
            dir::StaticExpression::Type { ty } => Some(*ty),
            _ => None,
        },
        dir::StaticArgument::Unevaluated { node } => types.get_declared_or_inferred_type_id(*node),
    }
}

/// Return true when the type may behave as array-like in `for-in` iteration.
///
/// This returns true for direct arrays, tuples, and any union or intersection
/// branch that resolves to an array-like structure.
pub fn is_array_like_iteration_type(
    types: &dir::TypeTable,
    strings: &StringPool,
    type_id: dir::LocalTypeId,
    array_symbol: Option<dir::GlobalSymbolId>,
) -> bool {
    let mut visited_type_ids = HashSet::new();
    is_array_like_iteration_type_inner(types, strings, type_id, array_symbol, &mut visited_type_ids)
}

/// Evaluate array-like iteration compatibility recursively.
fn is_array_like_iteration_type_inner(
    types: &dir::TypeTable,
    strings: &StringPool,
    type_id: dir::LocalTypeId,
    array_symbol: Option<dir::GlobalSymbolId>,
    visited_type_ids: &mut HashSet<dir::LocalTypeId>,
) -> bool {
    if !visited_type_ids.insert(type_id) {
        return false;
    }

    if is_array_type(types, type_id, array_symbol) {
        return true;
    }

    let type_node = types.get_type(type_id);
    match type_node {
        dir::Type::Union { elements } | dir::Type::Intersection { elements } => {
            elements.iter().any(|element_type_id| {
                is_array_like_iteration_type_inner(
                    types,
                    strings,
                    *element_type_id,
                    array_symbol,
                    visited_type_ids,
                )
            })
        }
        dir::Type::Value { value } => is_array_like_iteration_type_inner(
            types,
            strings,
            *value,
            array_symbol,
            visited_type_ids,
        ),
        dir::Type::ValueOf { right, .. }
        | dir::Type::ReferenceOf { right, .. }
        | dir::Type::PointerOf { right, .. } => is_array_like_iteration_type_inner(
            types,
            strings,
            *right,
            array_symbol,
            visited_type_ids,
        ),
        dir::Type::Object {
            fields,
            index_signatures,
            ..
        } => {
            object_has_numeric_index_signature(types, index_signatures)
                && object_has_array_like_length_field(types, strings, fields)
        }
        _ => false,
    }
}

/// Return true when one object declares a numeric index signature.
fn object_has_numeric_index_signature(
    types: &dir::TypeTable,
    index_signatures: &[dir::TypeIndexSignature],
) -> bool {
    index_signatures
        .iter()
        .any(|signature| is_numeric_property_key_type(types, signature.key_type))
}

/// Return true when one object has a numeric `length` field.
fn object_has_array_like_length_field(
    types: &dir::TypeTable,
    strings: &StringPool,
    fields: &[dir::TypeField],
) -> bool {
    fields.iter().any(|field| {
        let Some(field_name_id) = field.key.name() else {
            return false;
        };
        if strings.get(field_name_id).as_ref() != "length" {
            return false;
        }

        is_numeric_property_key_type(types, field.ty)
    })
}

/// Return true when the type is a string type.
pub fn is_string_type(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    string_symbol: Option<dir::GlobalSymbolId>,
) -> bool {
    evaluate_boolean_type_query(types, type_id, TypeBooleanQuery::String { string_symbol })
}

/// Return true when the type is a floating point type.
pub fn is_float_type(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> bool {
    evaluate_boolean_type_query(types, type_id, TypeBooleanQuery::Float)
}

/// Return true when the type is a function type.
pub fn is_function_type(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> bool {
    evaluate_boolean_type_query(types, type_id, TypeBooleanQuery::Function)
}

/// Return true when one type may resolve to one symbol type.
pub fn is_reference_symbol_type(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    symbol_type: dir::SymbolType,
) -> bool {
    evaluate_boolean_type_query(
        types,
        type_id,
        TypeBooleanQuery::ReferenceSymbolType { symbol_type },
    )
}

/// Return true when one type is an infer variable.
pub fn is_infer_var_type(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> bool {
    evaluate_boolean_type_query(types, type_id, TypeBooleanQuery::InferVar)
}

/// Return true when one type declares a `this` parameter.
pub fn has_this_parameter_type(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> bool {
    evaluate_boolean_type_query(types, type_id, TypeBooleanQuery::HasThisParameter)
}

/// Return true when one type declares a non-void `this` parameter.
pub fn has_non_void_this_parameter_type(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> bool {
    let mut visited_type_ids = HashSet::new();
    has_non_void_this_parameter_type_inner(types, type_id, &mut visited_type_ids)
}

/// Evaluate non-void `this` parameter compatibility recursively.
fn has_non_void_this_parameter_type_inner(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    visited_type_ids: &mut HashSet<dir::LocalTypeId>,
) -> bool {
    let normalized_type_id = normalized_flow_type_id(types, type_id);
    if !visited_type_ids.insert(normalized_type_id) {
        return false;
    }

    let ty = types.get_type(normalized_type_id);
    match ty {
        dir::Type::Function { this_parameter, .. } => {
            this_parameter.is_some_and(|this_parameter_type_id| {
                !is_void_or_never_type(types, this_parameter_type_id)
            })
        }
        dir::Type::Object {
            call_signatures, ..
        } => call_signatures.iter().any(|signature_type_id| {
            has_non_void_this_parameter_type_inner(types, *signature_type_id, visited_type_ids)
        }),
        dir::Type::Union { elements } | dir::Type::Intersection { elements } => {
            elements.iter().any(|element_type_id| {
                has_non_void_this_parameter_type_inner(types, *element_type_id, visited_type_ids)
            })
        }
        dir::Type::Value { value } => {
            has_non_void_this_parameter_type_inner(types, *value, visited_type_ids)
        }
        dir::Type::ValueOf { right, .. }
        | dir::Type::ReferenceOf { right, .. }
        | dir::Type::PointerOf { right, .. } => {
            has_non_void_this_parameter_type_inner(types, *right, visited_type_ids)
        }
        dir::Type::Reference { symbol, .. } => primary_reference_target_type_id(types, *symbol)
            .is_some_and(|target_type_id| {
                has_non_void_this_parameter_type_inner(types, target_type_id, visited_type_ids)
            }),
        _ => false,
    }
}

/// Return true when one type has a useful `toString` representation.
pub fn has_useful_to_string_type(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> bool {
    evaluate_boolean_type_query(types, type_id, TypeBooleanQuery::HasUsefulToString)
}

/// Return true when the type is an async function.
pub fn is_async_function_type(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> bool {
    evaluate_boolean_type_query(types, type_id, TypeBooleanQuery::AsyncFunction)
}

/// Return true when the type is `any`.
pub fn is_any_type(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> bool {
    evaluate_boolean_type_query(types, type_id, TypeBooleanQuery::Any)
}

/// Return true when the type is `Error`.
pub fn is_error_type(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> bool {
    matches!(
        types.get_type(normalized_flow_type_id(types, type_id)),
        dir::Type::Error
    )
}

/// Return true when the type tree contains explicit `any` (but not `unknown`).
pub fn is_explicit_any_type(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> bool {
    evaluate_boolean_type_query(types, type_id, TypeBooleanQuery::ExplicitAny)
}

/// Return true when the type is a Promise.
pub fn is_promise_type(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    promise_symbol: Option<dir::GlobalSymbolId>,
) -> bool {
    let Some(promise_symbol) = promise_symbol else {
        return false;
    };

    evaluate_boolean_type_query(types, type_id, TypeBooleanQuery::Promise { promise_symbol })
}

/// Return true when one type resolves to Promise for any candidate symbol.
pub fn is_promise_type_with_candidates(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    promise_symbols: &[dir::GlobalSymbolId],
) -> bool {
    promise_symbols
        .iter()
        .copied()
        .any(|symbol| is_promise_type(types, type_id, Some(symbol)))
}

/// Return true when the type is Promise or any-like.
pub fn is_promise_or_any_type(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    promise_symbol: Option<dir::GlobalSymbolId>,
) -> bool {
    let Some(promise_symbol) = promise_symbol else {
        return false;
    };

    evaluate_boolean_type_query(
        types,
        type_id,
        TypeBooleanQuery::PromiseOrAny { promise_symbol },
    )
}

/// Return true when one type supports Promise spread elements.
pub fn supports_promise_spread_elements(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    promise_symbol: Option<dir::GlobalSymbolId>,
) -> bool {
    let Some(promise_symbol) = promise_symbol else {
        return false;
    };

    evaluate_boolean_type_query(
        types,
        type_id,
        TypeBooleanQuery::PromiseSpreadElementCompatible { promise_symbol },
    )
}

/// Return true when one type contains `Map<_, void | never>`.
pub fn contains_map_with_empty_value_type(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    map_symbols: &[dir::GlobalSymbolId],
) -> bool {
    evaluate_boolean_type_query(
        types,
        type_id,
        TypeBooleanQuery::MapWithEmptyValue { map_symbols },
    )
}

/// Return true when the type is `void` or `never`.
pub fn is_void_or_never_type(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> bool {
    evaluate_boolean_type_query(types, type_id, TypeBooleanQuery::VoidOrNever)
}

/// Return true when the type can be interpolated into a template string.
pub fn is_template_interpolation_type(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    string_symbol: Option<dir::GlobalSymbolId>,
) -> bool {
    evaluate_boolean_type_query(
        types,
        type_id,
        TypeBooleanQuery::TemplateInterpolation { string_symbol },
    )
}

/// Return true when the type is string-like for object property keys.
pub fn is_string_like_property_key_type(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> bool {
    evaluate_boolean_type_query(types, type_id, TypeBooleanQuery::StringLikePropertyKey)
}

/// Return true when the type is numeric for object property keys.
pub fn is_numeric_property_key_type(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> bool {
    evaluate_boolean_type_query(types, type_id, TypeBooleanQuery::NumericPropertyKey)
}

/// Return true when the type is symbol-like for object property keys.
pub fn is_symbol_like_property_key_type(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> bool {
    evaluate_boolean_type_query(types, type_id, TypeBooleanQuery::SymbolLikePropertyKey)
}

/// Return true when the type is definitely a non error runtime value.
pub fn is_definitely_non_error_value_type(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    error_symbol: Option<dir::GlobalSymbolId>,
    result_symbol: Option<dir::GlobalSymbolId>,
) -> bool {
    if type_may_be_nominal_symbol(types, type_id, error_symbol) {
        return false;
    }

    evaluate_boolean_type_query(
        types,
        type_id,
        TypeBooleanQuery::DefinitelyNonErrorValue {
            error_symbol,
            result_symbol,
        },
    )
}

/// Return true when the type can evaluate to a nullish value.
pub fn is_maybe_nullish_type(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> bool {
    evaluate_boolean_type_query(types, type_id, TypeBooleanQuery::MaybeNullish)
}

/// Return true when the type can evaluate to a non nullish falsy value.
pub fn has_non_nullish_falsy_type(
    types: &dir::TypeTable,
    strings: &StringPool,
    type_id: dir::LocalTypeId,
) -> bool {
    evaluate_boolean_type_query(
        types,
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
    active_type_ids: HashSet<dir::LocalTypeId>,
    /// Cached truthiness values by local type id.
    cached_values: HashMap<u32, TypeTruthiness>,
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
    active_type_ids: HashSet<dir::LocalTypeId>,
    /// Cached nullishness values by local type id.
    cached_values: HashMap<u32, TypeNullishness>,
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
    types: &dir::TypeTable,
    strings: &StringPool,
    type_id: dir::LocalTypeId,
) -> TypeTruthiness {
    let mut state = TypeTruthinessState::new();
    type_truthiness_inner(types, strings, type_id, &mut state)
}

/// Return nullishness certainty for one type.
pub fn type_nullishness(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> TypeNullishness {
    let mut state = TypeNullishnessState::new();
    type_nullishness_inner(types, type_id, &mut state)
}

/// Unwrap nested `Type::Value` wrappers to one underlying type.
pub fn unwrap_value_type_id(
    types: &dir::TypeTable,
    mut type_id: dir::LocalTypeId,
) -> dir::LocalTypeId {
    let mut active_type_ids = HashSet::new();

    loop {
        if !active_type_ids.insert(type_id) {
            return type_id;
        }

        let ty = types.get_type(type_id);
        let dir::Type::Value { value } = ty else {
            return type_id;
        };
        type_id = *value;
    }
}

/// Return truthiness certainty for one type with recursion protection and caching.
fn type_truthiness_inner(
    types: &dir::TypeTable,
    strings: &StringPool,
    type_id: dir::LocalTypeId,
    state: &mut TypeTruthinessState,
) -> TypeTruthiness {
    let normalized_type_id = normalized_flow_type_id(types, type_id);
    if let Some(cached_value) = state.cached_values.get(&normalized_type_id.0).copied() {
        return cached_value;
    }

    if !state.active_type_ids.insert(normalized_type_id) {
        return TypeTruthiness::Unknown;
    }

    let ty = types.get_type(normalized_type_id);
    let truthiness = if let Some(next_type_id) = value_like_type_id(ty) {
        type_truthiness_inner(types, strings, next_type_id, state)
    } else if let dir::Type::Reference { symbol, .. } = ty {
        if let Some(next_type_id) = primary_reference_target_type_id(types, *symbol) {
            type_truthiness_inner(types, strings, next_type_id, state)
        } else {
            TypeTruthiness::AlwaysTruthy
        }
    } else if let Some(element_type_ids) = union_or_intersection_elements(ty) {
        combine_truthiness(
            element_type_ids
                .iter()
                .map(|type_id| type_truthiness_inner(types, strings, *type_id, state)),
        )
    } else {
        match ty {
            dir::Type::TypeLiteral { value } => match value {
                dir::TypeLiteral::Never => TypeTruthiness::Unknown,
                dir::TypeLiteral::Any | dir::TypeLiteral::Infer | dir::TypeLiteral::Unknown => {
                    TypeTruthiness::Unknown
                }
                dir::TypeLiteral::Void | dir::TypeLiteral::Null | dir::TypeLiteral::Undefined => {
                    TypeTruthiness::AlwaysFalsy
                }
                dir::TypeLiteral::Object => TypeTruthiness::AlwaysTruthy,
                dir::TypeLiteral::Primitive(primitive) => match primitive {
                    dir::PrimitiveType::Symbol | dir::PrimitiveType::UniqueSymbol => {
                        TypeTruthiness::AlwaysTruthy
                    }
                    _ => TypeTruthiness::Unknown,
                },
                dir::TypeLiteral::ScalarLiteral(literal) => match literal {
                    dir::ScalarLiteral::Null => TypeTruthiness::AlwaysFalsy,
                    dir::ScalarLiteral::Boolean(value) => {
                        if *value {
                            TypeTruthiness::AlwaysTruthy
                        } else {
                            TypeTruthiness::AlwaysFalsy
                        }
                    }
                    dir::ScalarLiteral::Integer(value) => {
                        if *value == 0 {
                            TypeTruthiness::AlwaysFalsy
                        } else {
                            TypeTruthiness::AlwaysTruthy
                        }
                    }
                    dir::ScalarLiteral::Bigint(value) => {
                        if *value == 0 {
                            TypeTruthiness::AlwaysFalsy
                        } else {
                            TypeTruthiness::AlwaysTruthy
                        }
                    }
                    dir::ScalarLiteral::Float(value) => {
                        if *value == 0.0 {
                            TypeTruthiness::AlwaysFalsy
                        } else {
                            TypeTruthiness::AlwaysTruthy
                        }
                    }
                    dir::ScalarLiteral::String(value) => {
                        if strings.get(*value).is_empty() {
                            TypeTruthiness::AlwaysFalsy
                        } else {
                            TypeTruthiness::AlwaysTruthy
                        }
                    }
                    dir::ScalarLiteral::Character(_) | dir::ScalarLiteral::RegexString { .. } => {
                        TypeTruthiness::Unknown
                    }
                },
                dir::TypeLiteral::Intrinsic(_) => TypeTruthiness::Unknown,
            },
            dir::Type::Array { .. }
            | dir::Type::ArraySized { .. }
            | dir::Type::Tuple { .. }
            | dir::Type::Object { .. }
            | dir::Type::Function { .. } => TypeTruthiness::AlwaysTruthy,
            dir::Type::InferVar { .. }
            | dir::Type::This
            | dir::Type::Unevaluated(_)
            | dir::Type::Conditional { .. }
            | dir::Type::Mapped { .. }
            | dir::Type::Index { .. }
            | dir::Type::TemplateLiteral { .. }
            | dir::Type::Import { .. }
            | dir::Type::Infer { .. }
            | dir::Type::Predicate { .. }
            | dir::Type::Readonly { .. }
            | dir::Type::KeyOf { .. }
            | dir::Type::Must { .. }
            | dir::Type::AsComptime { .. }
            | dir::Type::Not { .. }
            | dir::Type::In { .. }
            | dir::Type::Extends { .. }
            | dir::Type::Implements { .. }
            | dir::Type::Error => TypeTruthiness::Unknown,
            _ => TypeTruthiness::Unknown,
        }
    };

    state.active_type_ids.remove(&normalized_type_id);
    state.cached_values.insert(normalized_type_id.0, truthiness);
    truthiness
}

/// Return nullishness certainty for one type with recursion protection and caching.
fn type_nullishness_inner(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    state: &mut TypeNullishnessState,
) -> TypeNullishness {
    let normalized_type_id = normalized_flow_type_id(types, type_id);
    if let Some(cached_value) = state.cached_values.get(&normalized_type_id.0).copied() {
        return cached_value;
    }

    if !state.active_type_ids.insert(normalized_type_id) {
        return TypeNullishness::Maybe;
    }

    let ty = types.get_type(normalized_type_id);
    let nullishness = if let Some(next_type_id) = value_like_type_id(ty) {
        type_nullishness_inner(types, next_type_id, state)
    } else if let dir::Type::Reference { symbol, .. } = ty {
        if let Some(next_type_id) = primary_reference_target_type_id(types, *symbol) {
            type_nullishness_inner(types, next_type_id, state)
        } else {
            TypeNullishness::Never
        }
    } else if let Some(element_type_ids) = union_or_intersection_elements(ty) {
        combine_nullishness(
            element_type_ids
                .iter()
                .map(|type_id| type_nullishness_inner(types, *type_id, state)),
        )
    } else {
        match ty {
            dir::Type::TypeLiteral { value } => match value {
                dir::TypeLiteral::Null | dir::TypeLiteral::Undefined | dir::TypeLiteral::Void => {
                    TypeNullishness::Always
                }
                dir::TypeLiteral::Never => TypeNullishness::Maybe,
                dir::TypeLiteral::Any | dir::TypeLiteral::Infer | dir::TypeLiteral::Unknown => {
                    TypeNullishness::Maybe
                }
                dir::TypeLiteral::Object
                | dir::TypeLiteral::Primitive(_)
                | dir::TypeLiteral::Intrinsic(_)
                | dir::TypeLiteral::ScalarLiteral(_) => TypeNullishness::Never,
            },
            dir::Type::Array { .. }
            | dir::Type::ArraySized { .. }
            | dir::Type::Tuple { .. }
            | dir::Type::Object { .. }
            | dir::Type::Function { .. } => TypeNullishness::Never,
            dir::Type::InferVar { .. }
            | dir::Type::This
            | dir::Type::Unevaluated(_)
            | dir::Type::Conditional { .. }
            | dir::Type::Mapped { .. }
            | dir::Type::Index { .. }
            | dir::Type::TemplateLiteral { .. }
            | dir::Type::Import { .. }
            | dir::Type::Infer { .. }
            | dir::Type::Predicate { .. }
            | dir::Type::Readonly { .. }
            | dir::Type::KeyOf { .. }
            | dir::Type::Must { .. }
            | dir::Type::AsComptime { .. }
            | dir::Type::Not { .. }
            | dir::Type::In { .. }
            | dir::Type::Extends { .. }
            | dir::Type::Implements { .. }
            | dir::Type::Error => TypeNullishness::Maybe,
            _ => TypeNullishness::Maybe,
        }
    };

    state.active_type_ids.remove(&normalized_type_id);
    state
        .cached_values
        .insert(normalized_type_id.0, nullishness);
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
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    index: usize,
) -> Option<dir::LocalTypeId> {
    let normalized_type_id = normalized_flow_type_id(types, type_id);
    let mut state = TypeQueryState::new();
    function_parameter_type_at_inner(types, normalized_type_id, index, &mut state)
}

/// Resolve all parameter types at an index for a function-like type.
pub fn function_parameter_types_at(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    index: usize,
) -> Vec<dir::LocalTypeId> {
    // prefer flow normalized types when available
    let normalized_type_id = normalized_flow_type_id(types, type_id);

    let mut state = TypeQueryState::new();
    let mut results = Vec::new();
    function_parameter_types_at_inner(types, normalized_type_id, index, &mut state, &mut results);

    results
}

/// Resolve the return type for a function-like type.
pub fn function_return_type(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
) -> Option<dir::LocalTypeId> {
    let normalized_type_id = normalized_flow_type_id(types, type_id);
    let mut state = TypeQueryState::new();
    function_return_type_inner(types, normalized_type_id, &mut state)
}

/// Return true when the type may include one nominal symbol.
fn type_may_be_nominal_symbol(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    symbol: Option<dir::GlobalSymbolId>,
) -> bool {
    let Some(symbol) = symbol else {
        return false;
    };

    // prefer flow normalized types when available
    let normalized_type_id = normalized_flow_type_id(types, type_id);

    // track visited type ids to avoid recursion cycles
    let mut state = TypeQueryState::new();

    // resolve whether the type may include the symbol
    type_may_be_nominal_symbol_inner(types, normalized_type_id, symbol, &mut state)
}

/// Return true when the type may include one nominal symbol with cycle protection.
fn type_may_be_nominal_symbol_inner(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    symbol: dir::GlobalSymbolId,
    state: &mut TypeQueryState,
) -> bool {
    // stop recursive cycles
    if !state.enter_type_id(type_id) {
        return false;
    }

    // inspect the type node
    let ty = types.get_type(type_id);
    let result = match ty {
        dir::Type::Reference {
            symbol: candidate, ..
        } => {
            if *candidate == symbol || symbol_lineage_contains(types, *candidate, symbol) {
                true
            } else if let Some(instance_type_id) = types.get_instance_type_id(*candidate) {
                type_may_be_nominal_symbol_inner(types, instance_type_id, symbol, state)
            } else if let Some(value_type_id) = types.get_value_type_id(*candidate) {
                type_may_be_nominal_symbol_inner(types, value_type_id, symbol, state)
            } else {
                false
            }
        }
        dir::Type::Value { value } => {
            type_may_be_nominal_symbol_inner(types, *value, symbol, state)
        }
        dir::Type::ValueOf { right, .. }
        | dir::Type::ReferenceOf { right, .. }
        | dir::Type::PointerOf { right, .. } => {
            type_may_be_nominal_symbol_inner(types, *right, symbol, state)
        }
        dir::Type::Union { elements } | dir::Type::Intersection { elements } => elements
            .iter()
            .any(|element| type_may_be_nominal_symbol_inner(types, *element, symbol, state)),
        _ => false,
    };

    state.leave_type_id(type_id);
    result
}

/// Return true when one symbol lineage reaches a target symbol.
fn symbol_lineage_contains(
    types: &dir::TypeTable,
    symbol: dir::GlobalSymbolId,
    target_symbol: dir::GlobalSymbolId,
) -> bool {
    let mut state = SymbolQueryState::new();
    symbol_lineage_contains_inner(types, symbol, target_symbol, &mut state)
}

/// Return true when one symbol lineage reaches a target symbol with cycle protection.
fn symbol_lineage_contains_inner(
    types: &dir::TypeTable,
    symbol: dir::GlobalSymbolId,
    target_symbol: dir::GlobalSymbolId,
    state: &mut SymbolQueryState,
) -> bool {
    // direct symbol matches are always valid
    if symbol == target_symbol {
        return true;
    }

    // guard against cycles
    if !state.enter_symbol_id(symbol) {
        return false;
    }

    // resolve the lineage edges
    let result = if let Some(lineage) = types.get_lineage_for_symbol(symbol) {
        // check the extends edge first
        if lineage.extends.is_some_and(|extends_symbol| {
            symbol_lineage_contains_inner(types, extends_symbol, target_symbol, state)
        }) {
            true
        } else {
            // check implemented and embedded edges
            let mut found_match = false;
            for related_symbol in lineage
                .implements
                .iter()
                .chain(lineage.embedded.iter())
                .copied()
            {
                if symbol_lineage_contains_inner(types, related_symbol, target_symbol, state) {
                    found_match = true;
                    break;
                }
            }
            found_match
        }
    } else {
        false
    };

    state.leave_symbol_id(symbol);
    result
}

/// Resolve a parameter type for a function type with cycle protection.
fn function_parameter_type_at_inner(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    index: usize,
    state: &mut TypeQueryState,
) -> Option<dir::LocalTypeId> {
    // stop recursive cycles
    if !state.enter_type_id(type_id) {
        return None;
    }

    // inspect the type node
    let ty = types.get_type(type_id);
    let result = match ty {
        dir::Type::Function {
            dynamic_parameters, ..
        } => dynamic_parameters.get(index).copied(),
        dir::Type::Object {
            call_signatures, ..
        } => call_signatures
            .first()
            .copied()
            .and_then(|first_signature| {
                function_parameter_type_at_inner(types, first_signature, index, state)
            }),
        dir::Type::Value { value } => function_parameter_type_at_inner(types, *value, index, state),
        dir::Type::ValueOf { right, .. }
        | dir::Type::ReferenceOf { right, .. }
        | dir::Type::PointerOf { right, .. } => {
            function_parameter_type_at_inner(types, *right, index, state)
        }
        dir::Type::Reference { symbol, .. } => {
            if let Some(next_type_id) = primary_reference_target_type_id(types, *symbol) {
                function_parameter_type_at_inner(types, next_type_id, index, state)
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
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    index: usize,
    state: &mut TypeQueryState,
    results: &mut Vec<dir::LocalTypeId>,
) {
    // stop recursive cycles
    if !state.enter_type_id(type_id) {
        return;
    }

    // inspect the type node
    let ty = types.get_type(type_id);
    match ty {
        dir::Type::Function {
            dynamic_parameters, ..
        } => {
            if let Some(parameter_type_id) = dynamic_parameters.get(index).copied()
                && !results.contains(&parameter_type_id)
            {
                results.push(parameter_type_id);
            }
        }
        dir::Type::Object {
            call_signatures, ..
        } => {
            for signature_id in call_signatures {
                function_parameter_types_at_inner(types, *signature_id, index, state, results);
            }
        }
        dir::Type::Union { elements } | dir::Type::Intersection { elements } => {
            for element_type_id in elements {
                function_parameter_types_at_inner(types, *element_type_id, index, state, results);
            }
        }
        dir::Type::Value { value } => {
            function_parameter_types_at_inner(types, *value, index, state, results);
        }
        dir::Type::ValueOf { right, .. }
        | dir::Type::ReferenceOf { right, .. }
        | dir::Type::PointerOf { right, .. } => {
            function_parameter_types_at_inner(types, *right, index, state, results);
        }
        dir::Type::Reference { symbol, .. } => {
            for_each_reference_target_type_id(types, *symbol, |next_type_id| {
                function_parameter_types_at_inner(types, next_type_id, index, state, results);
            });
        }
        _ => {}
    }

    state.leave_type_id(type_id);
}

/// Resolve a return type for a function type with cycle protection.
fn function_return_type_inner(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    state: &mut TypeQueryState,
) -> Option<dir::LocalTypeId> {
    // stop recursive cycles
    if !state.enter_type_id(type_id) {
        return None;
    }

    // inspect the type node
    let ty = types.get_type(type_id);
    let result = match ty {
        dir::Type::Function { return_type, .. } => *return_type,
        dir::Type::Object {
            call_signatures, ..
        } => call_signatures
            .first()
            .copied()
            .and_then(|first_signature| function_return_type_inner(types, first_signature, state)),
        dir::Type::Value { value } => function_return_type_inner(types, *value, state),
        dir::Type::ValueOf { right, .. }
        | dir::Type::ReferenceOf { right, .. }
        | dir::Type::PointerOf { right, .. } => function_return_type_inner(types, *right, state),
        dir::Type::Reference { symbol, .. } => {
            if let Some(next_type_id) = primary_reference_target_type_id(types, *symbol) {
                function_return_type_inner(types, next_type_id, state)
            } else {
                None
            }
        }
        _ => None,
    };

    state.leave_type_id(type_id);
    result
}
