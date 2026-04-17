use super::*;

/// Rewrite literal types for binding commits.
struct LiteralWideningRewriter<'a> {
    /// The compiler backing literal widening helpers.
    compiler: &'a Compiler,
    /// The module providing language-specific widening defaults.
    module: &'a Module,
    /// The context controlling widening policy.
    ctx: &'a InferState,
    /// The rewriter options for caching.
    options: TypeRewriterOptions,
}

impl<'a> LiteralWideningRewriter<'a> {
    /// Create a literal widening rewriter.
    fn new(
        compiler: &'a Compiler,
        module: &'a Module,
        ctx: &'a InferState,
        options: TypeRewriterOptions,
    ) -> Self {
        Self {
            compiler,
            module,
            ctx,
            options,
        }
    }
}

impl TypeRewriter for LiteralWideningRewriter<'_> {
    fn options(&self) -> &TypeRewriterOptions {
        &self.options
    }

    /// Preserve static arguments during literal widening.
    fn rewrite_static_argument(
        &mut self,
        _types: &mut TypeTable,
        argument: &StaticArgument,
    ) -> StaticArgument {
        argument.clone()
    }

    fn rewrite_any(
        &mut self,
        types: &mut TypeTable,
        id: LocalTypeId,
        ty: &Type,
    ) -> Option<LocalTypeId> {
        let Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(literal),
        } = ty
        else {
            return None;
        };

        if !types.is_fresh_type(id) {
            return None;
        }

        if !self.compiler.should_widen_scalar_literal(self.ctx) {
            return None;
        }

        let widened = Type::TypeLiteral {
            value: self
                .compiler
                .widen_scalar_literal_for_module(self.module, literal),
        };
        Some(types.insert_type_from_type(widened, id))
    }
}

/// A TypeGuardTarget describes the target for a typeof or runtime type guard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TypeGuardTarget {
    /// Guard against a concrete type id.
    TypeId(LocalTypeId),
    /// Guard against object like values, including null.
    ObjectLike,
    /// Guard against callable values.
    FunctionLike,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Build the initializer inference context for one binding.
    pub(crate) fn binding_initializer_context(
        &self,
        state: &InferState,
        mutability: Option<Mutability>,
    ) -> InferState {
        state.fork().with_binding_initializer(mutability)
    }

    pub(crate) fn materialize_binding_type(
        &self,
        module: &Module,
        state: &InferState,
        binding_ty_id: LocalTypeId,
        types: &mut TypeTable,
        is_const_asserted: bool,
    ) -> LocalTypeId {
        // preserve literal types for const contexts
        if matches!(
            state.const_context,
            ConstContext::Const | ConstContext::AsConst
        ) {
            return binding_ty_id;
        }

        // preserve literal types for const assertions
        if is_const_asserted {
            return binding_ty_id;
        }

        // avoid widening when the context requests literal preservation
        if matches!(state.widening_mode, WideningMode::Preserve) {
            return binding_ty_id;
        }

        // regularize fresh literals before widening
        let regularized_ctx = state.for_widening_commit();
        let walk_ctx = TypeWalkContext::new(TypeWalkKey::BASE)
            .with_rewriter_tag(REWRITER_TAG_LITERAL_WIDENING);
        let walk_ctx = walk_ctx.with_context_key(state.widening_cache_key());
        let options = walk_ctx.rewriter_options();
        let cache_key = options.cache_key();
        let mut cache = TypeRewriteCache::new();
        let mut rewriter = LiteralWideningRewriter::new(self, module, &regularized_ctx, options);
        rewrite_type_with_cache(&mut rewriter, types, &mut cache, cache_key, binding_ty_id)
    }

    /// Commit one declarator initializer type using binding commitment rules.
    pub(crate) fn materialize_declarator_initializer_type(
        &self,
        ctx: &mut TypeContext<'_>,
        declarator_id: LocalNodeId<Declarator>,
        initializer_ty_id: LocalTypeId,
        state: &InferState,
    ) -> LocalTypeId {
        // preserve literal precision when the declarator uses const assertion
        let is_const_asserted = self.declarator_is_const_assertion(declarator_id, ctx.tree);
        self.materialize_binding_type(
            ctx.module,
            state,
            initializer_ty_id,
            ctx.types,
            is_const_asserted,
        )
    }

    /// Resolve a typeof guard target for a string literal.
    pub(crate) fn type_guard_target_for_typeof_string(
        &self,
        string_id: StringId,
        source_id: LocalNodeIdAny,
        types: &mut TypeTable,
    ) -> Option<TypeGuardTarget> {
        match self.repository.strings.get(string_id).as_ref() {
            "string" => Some(TypeGuardTarget::TypeId(types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::String),
                },
                source_id,
            ))),
            "number" => Some(TypeGuardTarget::TypeId(types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::Number),
                },
                source_id,
            ))),
            "boolean" => Some(TypeGuardTarget::TypeId(types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::Boolean),
                },
                source_id,
            ))),
            "bigint" => Some(TypeGuardTarget::TypeId(types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::Bigint),
                },
                source_id,
            ))),
            "symbol" => Some(TypeGuardTarget::TypeId(types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::Symbol),
                },
                source_id,
            ))),
            "undefined" => Some(TypeGuardTarget::TypeId(types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::Undefined,
                },
                source_id,
            ))),
            "object" => Some(TypeGuardTarget::ObjectLike),
            "function" => Some(TypeGuardTarget::FunctionLike),
            _ => None,
        }
    }

    /// Unwrap `type` operator annotations to reach the underlying reference.
    pub(crate) fn unwrap_type_symbol(
        &self,
        types: &TypeTable,
        ty_id: LocalTypeId,
    ) -> Option<(GlobalSymbolId, Option<Vec<StaticArgument>>, LocalNodeIdAny)> {
        let ty = types.get_type(ty_id);

        // unwrap type operator annotations to reach the underlying reference
        let (symbol, static_arguments) = match ty {
            Type::Reference {
                symbol,
                static_arguments,
            } => (*symbol, static_arguments.clone()),
            Type::Value { value } => {
                let value_ty = types.get_type(*value);
                let Type::Reference {
                    symbol,
                    static_arguments,
                } = value_ty
                else {
                    return None;
                };
                (*symbol, static_arguments.clone())
            }
            _ => return None,
        };

        // keep the outer type source id for node registration
        Some((symbol, static_arguments, types.get_type_source(ty_id)))
    }
}
