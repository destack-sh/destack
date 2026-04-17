use std::hash::{Hash, Hasher};

use rustc_hash::FxHasher;

use crate::Compiler;
use destack_dir::{
    FloatType, GlobalNodeIdAny, GlobalSymbolId, IntType, IntrinsicType, LocalNodeId, LocalTypeId,
    PrimitiveType, ScalarLiteral, StaticArgument, StaticExpression, SymbolType, Type,
    TypeExpression, TypeLiteral, TypeTable,
};
use destack_source::ModuleId;

/// Cache context for expression type evaluation.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DeclaredTypeResolutionContext {
    /// Whether static argument bounds must be validated.
    validate_static_argument_bounds: bool,
    /// Whether implicit managed semantics must be enforced.
    enforce_implicit_managed: bool,
    /// Whether static arguments should be resolved eagerly.
    resolve_static_arguments: bool,
}

impl DeclaredTypeResolutionContext {
    /// Create a cache context for expression evaluation.
    pub(crate) fn new(
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
        resolve_static_arguments: bool,
    ) -> Self {
        Self {
            validate_static_argument_bounds,
            enforce_implicit_managed,
            resolve_static_arguments,
        }
    }

    /// Build a cache key for expression evaluation.
    pub(crate) fn cache_key(self, node_id: GlobalNodeIdAny) -> u64 {
        let mut hasher = FxHasher::default();
        node_id.hash(&mut hasher);
        self.validate_static_argument_bounds.hash(&mut hasher);
        self.enforce_implicit_managed.hash(&mut hasher);
        self.resolve_static_arguments.hash(&mut hasher);
        hasher.finish()
    }

    /// Return whether one evaluated type is stable enough for the unkeyed declared-type slot.
    fn allows_declared_type_cache(self, ty: &Type) -> bool {
        let blocks_bound_sensitive_slot = self.validate_static_argument_bounds
            && matches!(
                ty,
                Type::Reference {
                    static_arguments: Some(arguments),
                    ..
                } if !arguments.is_empty()
            );
        if blocks_bound_sensitive_slot {
            return false;
        }

        let blocks_alias_argument_slot = self.resolve_static_arguments
            && matches!(
                ty,
                Type::Reference { symbol, .. }
                    if matches!(symbol.ty(), SymbolType::TypeAlias | SymbolType::Newtype)
            );
        if blocks_alias_argument_slot {
            return false;
        }

        true
    }

    /// Return whether one evaluated type is stable enough for the keyed expression cache.
    pub(crate) fn permits_expression_cache(self, ty: &Type, is_reference_expression: bool) -> bool {
        if ty.is_unevaluated() {
            return false;
        }

        if is_reference_expression && ty.is_unknown() {
            return false;
        }

        true
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Cache one declared type result when its value is stable across resolution modes.
    pub(crate) fn cache_declared_type_maybe(
        &self,
        global_node_id: GlobalNodeIdAny,
        cache_context: DeclaredTypeResolutionContext,
        type_id: LocalTypeId,
        ty: &Type,
        types: &mut TypeTable,
    ) {
        if cache_context.allows_declared_type_cache(ty) {
            types.set_declared_type(global_node_id, type_id);
        }
    }

    /// Cache a type-expression type id and optional value for a cache context.
    pub(crate) fn cache_expression_type_maybe(
        &self,
        module_id: ModuleId,
        expression_id: LocalNodeId<TypeExpression>,
        cache_context: DeclaredTypeResolutionContext,
        cache_type_id: Option<LocalTypeId>,
        ty: Option<&Type>,
        is_reference_expression: bool,
        types: &mut TypeTable,
    ) {
        let Some(ty) = ty else {
            return;
        };
        if !cache_context.permits_expression_cache(ty, is_reference_expression) {
            return;
        }

        let cache_key = cache_context.cache_key(expression_id.into_global_any(module_id));
        if let Some(cache_type_id) = cache_type_id {
            types.set_expression_type_id_cache(cache_key, cache_type_id);
        }
        types.set_expression_type_value_cache(cache_key, ty.clone());
    }

    /// Cache a resolved type reference when the key and type are stable.
    pub(crate) fn cache_type_reference_maybe(
        &self,
        cache_key: Option<u64>,
        ty: &Type,
        types: &mut TypeTable,
    ) {
        let Some(cache_key) = cache_key else {
            return;
        };
        if !ty.is_unevaluated() && !ty.is_error_or_unknown() {
            types.set_type_reference_cache(cache_key, ty.clone());
        }
    }

    /// Build a cache key for a type reference when arguments are hashable.
    pub(crate) fn type_reference_cache_key(
        &self,
        symbol: GlobalSymbolId,
        static_arguments: Option<&[StaticArgument]>,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
        resolve_static_arguments: bool,
    ) -> Option<u64> {
        // NOTE #Architecture: cache keys currently omit instantiation context
        let mut hasher = FxHasher::default();
        symbol.hash(&mut hasher);
        validate_static_argument_bounds.hash(&mut hasher);
        enforce_implicit_managed.hash(&mut hasher);
        resolve_static_arguments.hash(&mut hasher);
        self.hash_static_arguments_for_cache(static_arguments, &mut hasher)?;
        Some(hasher.finish())
    }

    /// Build a cache key for resolved static arguments when inputs are hashable.
    pub(crate) fn static_argument_resolution_cache_key(
        &self,
        symbol: GlobalSymbolId,
        static_arguments: Option<&[StaticArgument]>,
        validate_static_argument_bounds: bool,
        treat_type_arguments_as_types: bool,
        options_cache_key: u64,
    ) -> Option<u64> {
        let mut hasher = FxHasher::default();
        symbol.hash(&mut hasher);
        validate_static_argument_bounds.hash(&mut hasher);
        treat_type_arguments_as_types.hash(&mut hasher);
        options_cache_key.hash(&mut hasher);
        self.hash_static_arguments_for_cache(static_arguments, &mut hasher)?;
        Some(hasher.finish())
    }

    /// Hash static arguments for cache keys when they are safe to memoize.
    pub(super) fn hash_static_arguments_for_cache(
        &self,
        static_arguments: Option<&[StaticArgument]>,
        hasher: &mut FxHasher,
    ) -> Option<()> {
        match static_arguments {
            None => {
                0u8.hash(hasher);
            }
            Some(arguments) => {
                1u8.hash(hasher);
                for argument in arguments {
                    self.hash_static_argument_for_cache(argument, hasher)?;
                }
            }
        }
        Some(())
    }

    /// Hash a static argument for cache keys when it is safe to memoize.
    pub(super) fn hash_static_argument_for_cache(
        &self,
        argument: &StaticArgument,
        hasher: &mut FxHasher,
    ) -> Option<()> {
        match argument {
            StaticArgument::Unevaluated { .. } => None,
            StaticArgument::Evaluated { name, value } => {
                0u8.hash(hasher);
                name.hash(hasher);
                self.hash_static_expression_for_cache(value, hasher)
            }
        }
    }

    /// Hash a static expression for cache keys when it is safe to memoize.
    pub(super) fn hash_static_expression_for_cache(
        &self,
        value: &StaticExpression,
        hasher: &mut FxHasher,
    ) -> Option<()> {
        match value {
            StaticExpression::Unevaluated { .. } => None,
            StaticExpression::ScalarLiteral { value } => {
                0u8.hash(hasher);
                self.hash_scalar_literal_for_cache(value, hasher);
                Some(())
            }
            StaticExpression::TypeLiteral { value } => {
                1u8.hash(hasher);
                self.hash_type_literal_for_cache(value, hasher);
                Some(())
            }
            StaticExpression::Type { ty } => {
                2u8.hash(hasher);
                ty.hash(hasher);
                Some(())
            }
            StaticExpression::Declaration { .. }
            | StaticExpression::ArrayExpression { .. }
            | StaticExpression::TupleExpression { .. }
            | StaticExpression::ObjectExpression { .. } => None,
        }
    }

    /// Hash a scalar literal for cache keys.
    pub(super) fn hash_scalar_literal_for_cache(
        &self,
        value: &ScalarLiteral,
        hasher: &mut FxHasher,
    ) {
        match value {
            ScalarLiteral::Null => {
                0u8.hash(hasher);
            }
            ScalarLiteral::Boolean(value) => {
                1u8.hash(hasher);
                value.hash(hasher);
            }
            ScalarLiteral::Integer(value) => {
                2u8.hash(hasher);
                value.hash(hasher);
            }
            ScalarLiteral::Bigint(value) => {
                3u8.hash(hasher);
                value.hash(hasher);
            }
            ScalarLiteral::Float(value) => {
                4u8.hash(hasher);
                value.to_bits().hash(hasher);
            }
            ScalarLiteral::Character(value) => {
                5u8.hash(hasher);
                value.hash(hasher);
            }
            ScalarLiteral::String(value) => {
                6u8.hash(hasher);
                value.hash(hasher);
            }
            ScalarLiteral::RegexString { content, flags } => {
                7u8.hash(hasher);
                content.hash(hasher);
                flags.hash(hasher);
            }
        }
    }

    /// Hash a type literal for cache keys.
    pub(super) fn hash_type_literal_for_cache(&self, value: &TypeLiteral, hasher: &mut FxHasher) {
        match value {
            TypeLiteral::Never => 0u8.hash(hasher),
            TypeLiteral::Any => 1u8.hash(hasher),
            TypeLiteral::Infer => 2u8.hash(hasher),
            TypeLiteral::Undefined => 3u8.hash(hasher),
            TypeLiteral::Unknown => 4u8.hash(hasher),
            TypeLiteral::Object => 5u8.hash(hasher),
            TypeLiteral::Void => 6u8.hash(hasher),
            TypeLiteral::Null => 7u8.hash(hasher),
            TypeLiteral::Primitive(value) => {
                8u8.hash(hasher);
                self.hash_primitive_type_for_cache(*value, hasher);
            }
            TypeLiteral::Intrinsic(value) => {
                9u8.hash(hasher);
                match value {
                    IntrinsicType::Uppercase => 0u8.hash(hasher),
                    IntrinsicType::Lowercase => 1u8.hash(hasher),
                    IntrinsicType::Capitalize => 2u8.hash(hasher),
                    IntrinsicType::Uncapitalize => 3u8.hash(hasher),
                    IntrinsicType::NoInfer => 4u8.hash(hasher),
                    IntrinsicType::BuiltinIteratorReturn => 5u8.hash(hasher),
                }
            }
            TypeLiteral::ScalarLiteral(value) => {
                10u8.hash(hasher);
                self.hash_scalar_literal_for_cache(value, hasher);
            }
        }
    }

    /// Hash a primitive type for cache keys.
    pub(super) fn hash_primitive_type_for_cache(
        &self,
        value: PrimitiveType,
        hasher: &mut FxHasher,
    ) {
        match value {
            PrimitiveType::Boolean => 0u8.hash(hasher),
            PrimitiveType::Character => 1u8.hash(hasher),
            PrimitiveType::String => 2u8.hash(hasher),
            PrimitiveType::Bigint => 3u8.hash(hasher),
            PrimitiveType::Number => 4u8.hash(hasher),
            PrimitiveType::Int(int_type) => {
                5u8.hash(hasher);
                self.hash_int_type_for_cache(int_type, hasher);
            }
            PrimitiveType::Float(float_type) => {
                6u8.hash(hasher);
                self.hash_float_type_for_cache(float_type, hasher);
            }
            PrimitiveType::Symbol => 7u8.hash(hasher),
            PrimitiveType::UniqueSymbol => 8u8.hash(hasher),
        }
    }

    /// Hash an integer type for cache keys.
    pub(super) fn hash_int_type_for_cache(&self, value: IntType, hasher: &mut FxHasher) {
        match value {
            IntType::Int8 => 0u8.hash(hasher),
            IntType::Int16 => 1u8.hash(hasher),
            IntType::Int32 => 2u8.hash(hasher),
            IntType::Int64 => 3u8.hash(hasher),
            IntType::Int128 => 4u8.hash(hasher),
            IntType::Int256 => 5u8.hash(hasher),
            IntType::Isize => 6u8.hash(hasher),
            IntType::Uint8 => 7u8.hash(hasher),
            IntType::Uint16 => 8u8.hash(hasher),
            IntType::Uint32 => 9u8.hash(hasher),
            IntType::Uint64 => 10u8.hash(hasher),
            IntType::Uint128 => 11u8.hash(hasher),
            IntType::Uint256 => 12u8.hash(hasher),
            IntType::Usize => 13u8.hash(hasher),
            IntType::Arbitrary { width, is_signed } => {
                14u8.hash(hasher);
                width.hash(hasher);
                is_signed.hash(hasher);
            }
        }
    }

    /// Hash a float type for cache keys.
    pub(super) fn hash_float_type_for_cache(&self, value: FloatType, hasher: &mut FxHasher) {
        match value {
            FloatType::Float32 => 0u8.hash(hasher),
            FloatType::Float64 => 1u8.hash(hasher),
            FloatType::Arbitrary { width } => {
                2u8.hash(hasher);
                width.hash(hasher);
            }
        }
    }
}
