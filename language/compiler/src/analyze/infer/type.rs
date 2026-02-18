use std::collections::{HashMap, HashSet};

use super::member::MemberLookupMode;
use super::{
    index_key_kind_for_member, index_key_kind_for_type, index_key_kinds_compatible_for_access,
};
use crate::analyze::common::{
    AnalyzeReadStage, CanonicalSymbolMode, ConstContext, REWRITER_TAG_LITERAL_WIDENING,
    ReadonlyMaterializer, RelationMode, TypeRewriteCache, TypeWalkContext, TypeWalkKey,
    WideningMode, rewrite_type_with_cache,
};
use crate::{AnalyzeError, AnalyzeOptions, AnalyzeResult, Assignability, Compiler, InferContext};
use destack_builtin::LanguageSymbol;
use destack_dir::{
    Asynchrony, BinaryOperator, Declaration, DependencyItem, EnumBackingType, Expression,
    Extension, ExtensionKind, FloatType, FunctionCardinality, GlobalSymbolId, InferTable, IntType,
    LocalNodeId, LocalNodeIdAny, LocalTypeId, ModuleTarget, Mutability, NodeTree, NodeType,
    NormalizationMode, PrimitiveType, Resolution, ResolutionCandidate, ScalarLiteral,
    StaticArgument, StaticExpression, StaticKey, StaticProperty, StringId, SymbolSpaceOrder,
    SymbolTable, SymbolType, Type, TypeBinaryOperator, TypeElement, TypeField, TypeIndexSignature,
    TypeLiteral, TypeMappedParameter, TypeRewriter, TypeRewriterOptions, TypeTable,
    TypeUnaryOperator, UnaryOperator, VarianceBound, WellKnownSymbol,
};
use destack_source::ModuleId;
use destack_workspace::{Module, ModuleSource, ProfileId};

/// Rewrite literal types for binding commits.
struct LiteralWideningRewriter<'a> {
    /// The compiler backing literal widening helpers.
    compiler: &'a Compiler,
    /// The module providing language-specific widening defaults.
    module: &'a Module,
    /// The context controlling widening policy.
    ctx: &'a InferContext,
    /// The rewriter options for caching.
    options: TypeRewriterOptions,
}

impl<'a> LiteralWideningRewriter<'a> {
    /// Create a literal widening rewriter.
    fn new(
        compiler: &'a Compiler,
        module: &'a Module,
        ctx: &'a InferContext,
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
pub(super) enum TypeGuardTarget {
    /// Guard against a concrete type id.
    TypeId(LocalTypeId),
    /// Guard against object like values, including null.
    ObjectLike,
    /// Guard against callable values.
    FunctionLike,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Commit a binding type based on const context and widening rules.
    pub(crate) fn commit_binding_type(
        &self,
        module: &Module,
        ctx: &InferContext,
        binding_ty_id: LocalTypeId,
        types: &mut TypeTable,
        is_const_asserted: bool,
    ) -> LocalTypeId {
        // preserve literal types for const contexts
        if matches!(
            ctx.const_context,
            ConstContext::Const | ConstContext::AsConst
        ) {
            return binding_ty_id;
        }

        // preserve literal types for const assertions
        if is_const_asserted {
            return binding_ty_id;
        }

        // avoid widening when the context requests literal preservation
        if matches!(ctx.widening_mode, WideningMode::Preserve) {
            return binding_ty_id;
        }

        // regularize fresh literals before widening
        let regularized_ctx = ctx.for_widening_commit();
        let walk_ctx = TypeWalkContext::new(TypeWalkKey::BASE)
            .with_rewriter_tag(REWRITER_TAG_LITERAL_WIDENING);
        let walk_ctx = walk_ctx.with_context_key(ctx.widening_cache_key());
        let options = walk_ctx.rewriter_options();
        let cache_key = options.cache_key();
        let mut cache = TypeRewriteCache::new();
        let mut rewriter = LiteralWideningRewriter::new(self, module, &regularized_ctx, options);
        rewrite_type_with_cache(&mut rewriter, types, &mut cache, cache_key, binding_ty_id)
    }

    /// Resolve a typeof guard target for a string literal.
    pub(super) fn type_guard_target_for_typeof_string(
        &self,
        string_id: StringId,
        source_id: LocalNodeIdAny,
        types: &mut TypeTable,
    ) -> Option<TypeGuardTarget> {
        match self.program.strings.get(string_id).as_ref() {
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
            Type::Unary {
                operator: TypeUnaryOperator::Type,
                right,
            } => {
                let right_ty = types.get_type(*right);
                let Type::Reference {
                    symbol,
                    static_arguments,
                } = right_ty
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

    /// Unwrap structural type aliases to their instance types when possible.
    pub(super) fn unwrap_type_alias_reference(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        let symbol = match types.get_type(type_id).symbol() {
            Some(symbol) => symbol,
            None => return Ok(type_id),
        };

        // only unwrap structural type aliases
        if symbol.ty() != SymbolType::TypeAlias {
            return Ok(type_id);
        }

        // skip remote aliases during local flow computation
        if symbol.module_id != module.id {
            return Ok(type_id);
        }

        // load the type alias declaration
        let symbol_entry = symbols.get_symbol(symbol.local_id);
        let Some(primary_declaration) = symbol_entry.primary_declaration else {
            return Ok(type_id);
        };
        let declaration_id = match primary_declaration.local_id.try_into_typed::<Declaration>() {
            Ok(declaration_id) => declaration_id,
            Err(_) => return Ok(type_id),
        };
        let Declaration::Type {
            static_parameters,
            value,
            ..
        } = tree.get(declaration_id)
        else {
            return Ok(type_id);
        };

        // avoid eager evaluation for generic aliases
        if static_parameters
            .as_ref()
            .is_some_and(|parameters| !parameters.is_empty())
        {
            return Ok(type_id);
        }

        // reuse any apparent instance type before evaluating the alias body
        let source_id = types.get_type_source(type_id);
        if let Some(instance_type_id) =
            self.apparent_instance_type(module, profile, source_id, symbol, symbols, types)
        {
            return Ok(instance_type_id);
        }

        // evaluate the alias value into an instance type
        let instance_type_id = self.try_evaluate_expression_to_type(
            module, profile, *value, tree, symbols, types, true, true,
        )?;
        types.set_instance_type(symbol, instance_type_id);

        Ok(instance_type_id)
    }

    /// Ensure instance types for any reference types inside a type.
    pub(super) fn ensure_reference_instance_types_for_type(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        ty_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        let mut visited = HashSet::new();
        self.ensure_reference_instance_types_for_type_inner(
            module,
            profile,
            node_id,
            ty_id,
            types,
            &mut visited,
        )
    }

    /// Ensure instance types for any reference types inside a type.
    fn ensure_reference_instance_types_for_type_inner(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        ty_id: LocalTypeId,
        types: &mut TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> AnalyzeResult<()> {
        // skip types we have already visited
        if !visited.insert(ty_id) {
            return Ok(());
        }

        // clone to avoid holding a borrow across recursion
        let ty = types.get_type(ty_id).clone();

        // ensure reference symbols have instance types
        if let Type::Reference { symbol, .. } = ty {
            let instance_id =
                self.resolve_instance_type_for_symbol(module, profile, node_id, symbol, types)?;
            if let Some(instance_id) = instance_id {
                self.ensure_reference_instance_types_for_type_inner(
                    module,
                    profile,
                    node_id,
                    instance_id,
                    types,
                    visited,
                )?;
            }
            return Ok(());
        }

        // walk nested types based on structure
        match ty {
            Type::Value { value } => self.ensure_reference_instance_types_for_type_inner(
                module, profile, node_id, value, types, visited,
            ),
            Type::Conditional {
                distributive_symbol: _,
                left,
                right,
                then_type,
                else_type,
            } => {
                self.ensure_reference_instance_types_for_type_inner(
                    module, profile, node_id, left, types, visited,
                )?;
                self.ensure_reference_instance_types_for_type_inner(
                    module, profile, node_id, right, types, visited,
                )?;
                self.ensure_reference_instance_types_for_type_inner(
                    module, profile, node_id, then_type, types, visited,
                )?;
                self.ensure_reference_instance_types_for_type_inner(
                    module, profile, node_id, else_type, types, visited,
                )?;
                Ok(())
            }
            Type::Mapped {
                parameter, value, ..
            } => {
                self.ensure_reference_instance_types_for_type_inner(
                    module,
                    profile,
                    node_id,
                    parameter.constraint,
                    types,
                    visited,
                )?;

                if let Some(key_remap) = parameter.key_remap {
                    self.ensure_reference_instance_types_for_type_inner(
                        module, profile, node_id, key_remap, types, visited,
                    )?;
                }

                self.ensure_reference_instance_types_for_type_inner(
                    module, profile, node_id, value, types, visited,
                )
            }
            Type::Index { left, index } => {
                self.ensure_reference_instance_types_for_type_inner(
                    module, profile, node_id, left, types, visited,
                )?;
                self.ensure_reference_instance_types_for_type_inner(
                    module, profile, node_id, index, types, visited,
                )
            }
            Type::TemplateLiteral { spans, .. } => {
                for span in spans {
                    self.ensure_reference_instance_types_for_type_inner(
                        module, profile, node_id, span, types, visited,
                    )?;
                }
                Ok(())
            }
            Type::Infer { constraint, .. } => {
                if let Some(constraint) = constraint {
                    self.ensure_reference_instance_types_for_type_inner(
                        module, profile, node_id, constraint, types, visited,
                    )?;
                }
                Ok(())
            }
            Type::Predicate { target, .. } => {
                if let Some(target) = target {
                    self.ensure_reference_instance_types_for_type_inner(
                        module, profile, node_id, target, types, visited,
                    )?;
                }
                Ok(())
            }
            Type::Unary { right, .. }
            | Type::ValueOf { right, .. }
            | Type::ReferenceOf { right, .. }
            | Type::PointerOf { right, .. } => self.ensure_reference_instance_types_for_type_inner(
                module, profile, node_id, right, types, visited,
            ),
            Type::Binary { left, right, .. } => {
                self.ensure_reference_instance_types_for_type_inner(
                    module, profile, node_id, left, types, visited,
                )?;
                self.ensure_reference_instance_types_for_type_inner(
                    module, profile, node_id, right, types, visited,
                )
            }
            Type::ArraySized { element, .. } => self
                .ensure_reference_instance_types_for_type_inner(
                    module, profile, node_id, element, types, visited,
                ),
            Type::Array { element, .. } => {
                if let Some(element) = element {
                    self.ensure_reference_instance_types_for_type_inner(
                        module, profile, node_id, element, types, visited,
                    )?;
                }
                Ok(())
            }
            Type::Tuple { elements, .. } => {
                for element in elements {
                    self.ensure_reference_instance_types_for_type_inner(
                        module, profile, node_id, element.ty, types, visited,
                    )?;
                }
                Ok(())
            }
            Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                for field in fields {
                    self.ensure_reference_instance_types_for_type_inner(
                        module, profile, node_id, field.ty, types, visited,
                    )?;
                }

                for signature in call_signatures {
                    self.ensure_reference_instance_types_for_type_inner(
                        module, profile, node_id, signature, types, visited,
                    )?;
                }

                for signature in construct_signatures {
                    self.ensure_reference_instance_types_for_type_inner(
                        module, profile, node_id, signature, types, visited,
                    )?;
                }

                for signature in index_signatures {
                    self.ensure_reference_instance_types_for_type_inner(
                        module,
                        profile,
                        node_id,
                        signature.key_type,
                        types,
                        visited,
                    )?;
                    self.ensure_reference_instance_types_for_type_inner(
                        module,
                        profile,
                        node_id,
                        signature.value_type,
                        types,
                        visited,
                    )?;
                }

                Ok(())
            }
            Type::Function {
                static_parameters,
                this_parameter,
                dynamic_parameters,
                return_type,
                ..
            } => {
                for parameter in static_parameters {
                    self.ensure_reference_instance_types_for_type_inner(
                        module, profile, node_id, parameter, types, visited,
                    )?;
                }

                if let Some(this_parameter) = this_parameter {
                    self.ensure_reference_instance_types_for_type_inner(
                        module,
                        profile,
                        node_id,
                        this_parameter,
                        types,
                        visited,
                    )?;
                }

                for parameter in dynamic_parameters {
                    self.ensure_reference_instance_types_for_type_inner(
                        module, profile, node_id, parameter, types, visited,
                    )?;
                }

                if let Some(return_type) = return_type {
                    self.ensure_reference_instance_types_for_type_inner(
                        module,
                        profile,
                        node_id,
                        return_type,
                        types,
                        visited,
                    )?;
                }

                Ok(())
            }
            Type::Union { elements } | Type::Intersection { elements } => {
                for element in elements {
                    self.ensure_reference_instance_types_for_type_inner(
                        module, profile, node_id, element, types, visited,
                    )?;
                }
                Ok(())
            }
            Type::TypeLiteral { .. }
            | Type::InferVar { .. }
            | Type::This
            | Type::Reference { .. }
            | Type::Unevaluated(_)
            | Type::Import { .. }
            | Type::Error => Ok(()),
        }
    }

    /// Resolve the instance type for a referenced symbol into the local type table.
    pub(crate) fn resolve_instance_type_for_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // load symbol metadata for merge group selection
        let (symbol_type, symbol_key, symbol_space) = {
            let symbol_module = self.program.modules.get(symbol.module_id);
            let symbol_module = symbol_module.read();
            let symbol_table = symbol_module.dir_base().symbols.read();
            let symbol_entry = symbol_table.get_symbol(symbol.local_id);
            (symbol_entry.ty, symbol_entry.key, symbol_entry.space)
        };

        // normalize the symbol id to the stored symbol type
        let symbol = GlobalSymbolId::new(symbol.module_id, symbol.local_id.with_type(symbol_type));

        // skip symbols that cannot have instance types
        if !self.is_instantiable_symbol(symbol) {
            return Ok(None);
        }

        // ensure the defining module is declared before reading its types
        if symbol.module_id != module.id {
            self.require_analyze_module_declare(symbol.module_id, profile)
                .map_err(AnalyzeError::from)?;
        }

        // ensure the global symbol table is available for this module
        self.require_resolve_module_prepare(module.id, profile)
            .map_err(AnalyzeError::from)?;

        // select the global merge group when the symbol participates
        let mut group_symbols = Vec::new();
        if !self.module_is_ambient_lib(module)
            && let Some(key) = symbol_key
        {
            if let Some(group) = self.get_global_symbol_group(module.id, profile, key, symbol_space)
            {
                group_symbols.extend(group);
            }
            if let Some(ambient_symbols) =
                self.get_ambient_lib_symbol_sources_for_merge(profile, key, symbol_space)
            {
                group_symbols.extend(ambient_symbols);
            }
        }
        if group_symbols.is_empty() {
            group_symbols.push(symbol);
        } else {
            let mut seen = HashSet::new();
            group_symbols.retain(|symbol| seen.insert(*symbol));
        }

        // normalize group symbols to the stored symbol types
        let mut normalized_group_symbols = Vec::with_capacity(group_symbols.len());
        for group_symbol in group_symbols {
            let group_module = self.program.modules.get(group_symbol.module_id);
            let group_module = group_module.read();
            let group_symbol_table = group_module.dir_base().symbols.read();
            let group_entry = group_symbol_table.get_symbol(group_symbol.local_id);
            let normalized = GlobalSymbolId::new(
                group_symbol.module_id,
                group_symbol.local_id.with_type(group_entry.ty),
            );
            normalized_group_symbols.push(normalized);
        }
        let group_symbols = normalized_group_symbols;

        // reuse cached instance types when no merge is needed
        if group_symbols.len() == 1
            && let Some(existing) = types.get_instance_type_id(symbol)
        {
            return Ok(Some(existing));
        }

        // reuse an already merged instance type when available
        if group_symbols.len() > 1 {
            let mut merged_id = None;
            let mut all_match = true;
            for group_symbol in &group_symbols {
                let Some(group_instance_id) = types.get_instance_type_id(*group_symbol) else {
                    all_match = false;
                    break;
                };
                if let Some(existing) = merged_id {
                    if existing != group_instance_id {
                        all_match = false;
                        break;
                    }
                } else {
                    merged_id = Some(group_instance_id);
                }
            }

            if all_match && let Some(merged_id) = merged_id {
                return Ok(Some(merged_id));
            }
        }

        // import instance types for each group symbol
        let mut fields = Vec::new();
        let mut call_signatures = Vec::new();
        let mut construct_signatures = Vec::new();
        let mut index_signatures = Vec::new();
        let mut fallback_instance_id = None;

        for group_symbol in group_symbols.iter().copied() {
            // load the instance type for the group symbol
            let local_instance_id = if let Some(existing) = types.get_instance_type_id(group_symbol)
            {
                existing
            } else if group_symbol.module_id == module.id {
                continue;
            } else {
                let Some(imported) =
                    self.import_instance_type_for_symbol(profile, node_id, group_symbol, types)?
                else {
                    continue;
                };
                imported
            };

            if group_symbol == symbol {
                fallback_instance_id = Some(local_instance_id);
            }

            // extract instance type members
            let local_instance_ty = types.get_type(local_instance_id);
            if let Type::Object {
                fields: instance_fields,
                call_signatures: instance_calls,
                construct_signatures: instance_constructs,
                index_signatures: instance_indexes,
            } = local_instance_ty
            {
                fields.extend_from_slice(instance_fields);
                call_signatures.extend_from_slice(instance_calls);
                construct_signatures.extend_from_slice(instance_constructs);
                index_signatures.extend_from_slice(instance_indexes);
            }
        }

        // handle non mergeable instances and empty merges
        if group_symbols.len() == 1
            || (fields.is_empty()
                && call_signatures.is_empty()
                && construct_signatures.is_empty()
                && index_signatures.is_empty())
        {
            if let Some(fallback_instance_id) = fallback_instance_id {
                types.set_instance_type(symbol, fallback_instance_id);
            }
            return Ok(fallback_instance_id);
        }

        // create a merged instance type for all group symbols
        let merged_ty = Type::Object {
            fields,
            call_signatures,
            construct_signatures,
            index_signatures,
        };
        let merged_id = types.insert_type_from_any(merged_ty, node_id);
        for group_symbol in group_symbols {
            types.set_instance_type(group_symbol, merged_id);
        }

        Ok(Some(merged_id))
    }

    /// Import a remote instance type into the local type table.
    pub(super) fn import_instance_type_for_symbol(
        &self,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        self.with_module_tree_symbols_by_id_for_stage(
            profile,
            symbol.module_id,
            AnalyzeReadStage::Declare,
            |remote_module, remote_tree, remote_symbols| {
                let remote_dir = remote_module.dir(profile);
                let mut remote_types = remote_dir.types.write();
                let Some(remote_instance_id) = remote_types.get_instance_type_id(symbol) else {
                    return Ok(None);
                };

                // materialize and import the remote type
                self.materialize_imported_type(
                    remote_module,
                    profile,
                    remote_instance_id,
                    remote_tree,
                    remote_symbols,
                    &mut remote_types,
                )?;
                let remote_instance_ty = remote_types.get_type(remote_instance_id);
                let local_instance_id = self.import_type_from_remote_for_node(
                    node_id,
                    remote_instance_ty,
                    &remote_types,
                    symbol,
                    types,
                );
                Ok(Some(local_instance_id))
            },
        )
        .map_err(AnalyzeError::from)?
    }

    /// Infer the result type of a scalar literal.
    /// Returns the literal type (e.g., `4` has type `4`), allowing assignability to check
    /// whether the literal fits the target type (int32, number, etc.).
    pub(super) fn infer_scalar_literal(&self, value: &ScalarLiteral) -> TypeLiteral {
        // return the literal type, not the widened primitive type
        // this allows `let x: int = 4` to work via assignability checking
        TypeLiteral::ScalarLiteral(value.clone())
    }

    /// Widen a scalar literal to its primitive type.
    pub(super) fn widen_scalar_literal_for_module(
        &self,
        module: &Module,
        value: &ScalarLiteral,
    ) -> TypeLiteral {
        let primitive = match value {
            ScalarLiteral::Boolean(_) => PrimitiveType::Boolean,
            ScalarLiteral::Integer(value) => {
                if module.language_type.is_destack()
                    && self.is_integer_literal_assignable(*value as i128, &IntType::Int32)
                {
                    PrimitiveType::Int(IntType::Int32)
                } else {
                    PrimitiveType::Number
                }
            }
            ScalarLiteral::Float(_) => {
                if module.language_type.is_destack() {
                    PrimitiveType::Float(FloatType::Float64)
                } else {
                    PrimitiveType::Number
                }
            }
            ScalarLiteral::Bigint(_) => PrimitiveType::Bigint,
            ScalarLiteral::Character(_)
            | ScalarLiteral::String(_)
            | ScalarLiteral::RegexString { .. } => PrimitiveType::String,
        };
        TypeLiteral::Primitive(primitive)
    }

    /// Infer the result type of a binary operation.
    pub(super) fn infer_binary_operation(
        &self,
        operator: &BinaryOperator,
        left: &Type,
        right: &Type,
        types: &TypeTable,
    ) -> Type {
        match operator {
            // comparison operators: try constant folding, else return boolean
            BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::EqualStrict
            | BinaryOperator::NotEqualStrict
            | BinaryOperator::LessThan
            | BinaryOperator::LessThanOrEqual
            | BinaryOperator::GreaterThan
            | BinaryOperator::GreaterThanOrEqual => self
                .try_fold_comparison(operator, left, right)
                .unwrap_or(Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::Boolean),
                }),

            // in/instanceof always return boolean (no constant folding)
            BinaryOperator::In | BinaryOperator::InstanceOf => Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean),
            },

            // logical operators: try constant folding, else return boolean
            BinaryOperator::And | BinaryOperator::Or => self
                .try_fold_logical(operator, left, right)
                .unwrap_or(Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::Boolean),
                }),

            // arithmetic operators: try constant folding, else widen types
            BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::Remainder
            | BinaryOperator::Exponent => self
                .try_infer_string_concatenation(operator, left, right, types)
                .or_else(|| self.try_fold_arithmetic(operator, left, right))
                .unwrap_or_else(|| self.widen_numeric_types(left, right)),

            _ => left.clone(),
        }
    }

    /// Try to constant fold a comparison operation on literal types.
    fn try_fold_comparison(
        &self,
        operator: &BinaryOperator,
        left: &Type,
        right: &Type,
    ) -> Option<Type> {
        // extract scalar literals from both sides
        let (left_lit, right_lit) = Self::extract_scalar_literals(left, right)?;

        // compare integers
        if let (ScalarLiteral::Integer(left_value), ScalarLiteral::Integer(right_value)) =
            (left_lit, right_lit)
        {
            let result = match operator {
                BinaryOperator::Equal | BinaryOperator::EqualStrict => left_value == right_value,
                BinaryOperator::NotEqual | BinaryOperator::NotEqualStrict => {
                    left_value != right_value
                }
                BinaryOperator::LessThan => left_value < right_value,
                BinaryOperator::LessThanOrEqual => left_value <= right_value,
                BinaryOperator::GreaterThan => left_value > right_value,
                BinaryOperator::GreaterThanOrEqual => left_value >= right_value,
                _ => return None,
            };
            return Some(Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(result)),
            });
        }

        // compare floats (or mixed int/float)
        let (left_value, right_value) = Self::to_f64_pair(left_lit, right_lit)?;
        let result = match operator {
            BinaryOperator::Equal | BinaryOperator::EqualStrict => left_value == right_value,
            BinaryOperator::NotEqual | BinaryOperator::NotEqualStrict => left_value != right_value,
            BinaryOperator::LessThan => left_value < right_value,
            BinaryOperator::LessThanOrEqual => left_value <= right_value,
            BinaryOperator::GreaterThan => left_value > right_value,
            BinaryOperator::GreaterThanOrEqual => left_value >= right_value,
            _ => return None,
        };
        Some(Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(result)),
        })
    }

    /// Try to constant fold a logical operation on literal types.
    fn try_fold_logical(
        &self,
        operator: &BinaryOperator,
        left: &Type,
        right: &Type,
    ) -> Option<Type> {
        // extract boolean literals from both sides
        let (left_lit, right_lit) = Self::extract_scalar_literals(left, right)?;

        if let (ScalarLiteral::Boolean(left_value), ScalarLiteral::Boolean(right_value)) =
            (left_lit, right_lit)
        {
            let result = match operator {
                BinaryOperator::And => *left_value && *right_value,
                BinaryOperator::Or => *left_value || *right_value,
                _ => return None,
            };
            return Some(Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(result)),
            });
        }

        None
    }

    /// Try to constant fold an arithmetic operation on literal types.
    fn try_fold_arithmetic(
        &self,
        operator: &BinaryOperator,
        left: &Type,
        right: &Type,
    ) -> Option<Type> {
        // extract scalar literals from both sides
        let (left_lit, right_lit) = Self::extract_scalar_literals(left, right)?;

        // try to fold integer operations (preserves integer type)
        if let (ScalarLiteral::Integer(left_value), ScalarLiteral::Integer(right_value)) =
            (left_lit, right_lit)
        {
            let result = match operator {
                BinaryOperator::Add => left_value.checked_add(*right_value),
                BinaryOperator::Subtract => left_value.checked_sub(*right_value),
                BinaryOperator::Multiply => left_value.checked_mul(*right_value),
                BinaryOperator::Divide => {
                    if *right_value != 0 {
                        left_value.checked_div(*right_value)
                    } else {
                        None
                    }
                }
                BinaryOperator::Remainder => {
                    if *right_value != 0 {
                        left_value.checked_rem(*right_value)
                    } else {
                        None
                    }
                }
                BinaryOperator::Exponent => {
                    if *right_value >= 0 && *right_value <= u32::MAX as i64 {
                        left_value.checked_pow(*right_value as u32)
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some(result) = result {
                return Some(Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(result)),
                });
            }
        }

        // try to fold float operations (if at least one operand is float)
        if matches!(left_lit, ScalarLiteral::Float(_))
            || matches!(right_lit, ScalarLiteral::Float(_))
        {
            let (left_value, right_value) = Self::to_f64_pair(left_lit, right_lit)?;
            let result = match operator {
                BinaryOperator::Add => left_value + right_value,
                BinaryOperator::Subtract => left_value - right_value,
                BinaryOperator::Multiply => left_value * right_value,
                BinaryOperator::Divide => left_value / right_value,
                BinaryOperator::Remainder => left_value % right_value,
                BinaryOperator::Exponent => left_value.powf(right_value),
                _ => return None,
            };
            return Some(Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Float(result)),
            });
        }

        None
    }

    /// Try to infer string concatenation for add.
    fn try_infer_string_concatenation(
        &self,
        operator: &BinaryOperator,
        left: &Type,
        right: &Type,
        types: &TypeTable,
    ) -> Option<Type> {
        if !matches!(operator, BinaryOperator::Add) {
            return None;
        }

        if self.is_string_like_type(left, types) || self.is_string_like_type(right, types) {
            return Some(Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String),
            });
        }

        None
    }

    /// Check whether a type behaves like a string type.
    pub(super) fn is_string_like_type(&self, ty: &Type, types: &TypeTable) -> bool {
        match ty {
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String),
            } => true,
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_)),
            } => true,
            Type::Union { elements } => elements
                .iter()
                .all(|element_id| self.is_string_like_type(types.get_type(*element_id), types)),
            _ => false,
        }
    }

    /// Extract scalar literals from two types.
    fn extract_scalar_literals<'a>(
        left: &'a Type,
        right: &'a Type,
    ) -> Option<(&'a ScalarLiteral, &'a ScalarLiteral)> {
        match (left, right) {
            (
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(left_literal),
                },
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(right_literal),
                },
            ) => Some((left_literal, right_literal)),
            _ => None,
        }
    }

    /// Convert two scalar literals to f64 values (for numeric operations).
    fn to_f64_pair(left: &ScalarLiteral, right: &ScalarLiteral) -> Option<(f64, f64)> {
        let left_value = match left {
            ScalarLiteral::Integer(i) => *i as f64,
            ScalarLiteral::Float(f) => *f,
            _ => return None,
        };
        let right_value = match right {
            ScalarLiteral::Integer(i) => *i as f64,
            ScalarLiteral::Float(f) => *f,
            _ => return None,
        };
        Some((left_value, right_value))
    }

    /// Widen two numeric types to a common type.
    /// Used when constant folding fails (e.g., `x + 1` where x is a variable).
    pub(super) fn widen_numeric_types(&self, left: &Type, right: &Type) -> Type {
        let left_prim = Self::to_numeric_primitive(left);
        let right_prim = Self::to_numeric_primitive(right);
        match (left_prim, right_prim) {
            // if both are known primitives, return the wider one
            (Some(left_primitive), Some(right_primitive)) => Type::TypeLiteral {
                value: TypeLiteral::Primitive(Self::wider_numeric_primitive(
                    &left_primitive,
                    &right_primitive,
                )),
            },
            // if one side is a primitive, use it
            (Some(p), None) | (None, Some(p)) => Type::TypeLiteral {
                value: TypeLiteral::Primitive(p),
            },
            // fallback to number
            (None, None) => Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number),
            },
        }
    }

    /// Extract the numeric primitive type from a type.
    fn to_numeric_primitive(ty: &Type) -> Option<PrimitiveType> {
        match ty {
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(p),
            } if Self::is_numeric_primitive(p) => Some(*p),
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(_)),
            } => None,
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Float(_)),
            } => Some(PrimitiveType::Number),
            _ => None,
        }
    }

    /// Check if a primitive type is numeric.
    fn is_numeric_primitive(p: &PrimitiveType) -> bool {
        matches!(
            p,
            PrimitiveType::Number
                | PrimitiveType::Int(_)
                | PrimitiveType::Float(_)
                | PrimitiveType::Bigint
        )
    }

    /// Return the wider of two numeric primitive types.
    fn wider_numeric_primitive(left: &PrimitiveType, right: &PrimitiveType) -> PrimitiveType {
        // number is the widest
        if matches!(left, PrimitiveType::Number) || matches!(right, PrimitiveType::Number) {
            return PrimitiveType::Number;
        }
        // float is wider than int; pick the wider float
        match (left, right) {
            (PrimitiveType::Float(left_float), PrimitiveType::Float(right_float)) => {
                let wider = if left_float.width() >= right_float.width() {
                    *left_float
                } else {
                    *right_float
                };
                return PrimitiveType::Float(wider);
            }
            (PrimitiveType::Float(f), _) | (_, PrimitiveType::Float(f)) => {
                return PrimitiveType::Float(*f);
            }
            _ => {}
        }
        // bigint stays bigint
        if matches!(left, PrimitiveType::Bigint) || matches!(right, PrimitiveType::Bigint) {
            return PrimitiveType::Bigint;
        }
        // compare int widths and return the wider one
        match (left, right) {
            (PrimitiveType::Int(left_int), PrimitiveType::Int(right_int)) => {
                // if either is signed, result should be signed
                let is_signed = left_int.is_signed() || right_int.is_signed();
                match (left_int.width(), right_int.width()) {
                    (Some(left_width), Some(right_width)) => {
                        let width = left_width.max(right_width);
                        PrimitiveType::Int(IntType::Arbitrary { width, is_signed })
                    }
                    // pointer sized ints: cannot determine width at compile time
                    _ => PrimitiveType::Number,
                }
            }
            (PrimitiveType::Int(i), _) | (_, PrimitiveType::Int(i)) => PrimitiveType::Int(*i),
            _ => PrimitiveType::Number,
        }
    }

    /// Infer the result type of a unary operation.
    pub(super) fn infer_unary_operation(&self, operator: &UnaryOperator, right: &Type) -> Type {
        match operator {
            // constant folding for logical not
            UnaryOperator::Not => self.try_fold_not(right).unwrap_or(Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean),
            }),
            // constant folding for numeric negation
            UnaryOperator::Negate => self.try_fold_negate(right),
            UnaryOperator::Typeof => Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String),
            },
            UnaryOperator::Void => Type::TypeLiteral {
                value: TypeLiteral::Undefined,
            },
            _ => right.clone(),
        }
    }

    /// Try to constant fold logical not on a boolean literal.
    fn try_fold_not(&self, right: &Type) -> Option<Type> {
        if let Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(b)),
        } = right
        {
            return Some(Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(!b)),
            });
        }
        None
    }

    /// Try to constant fold unary negation on a literal type.
    /// Returns the negated literal type, or the original type if folding is not possible.
    fn try_fold_negate(&self, right: &Type) -> Type {
        if let Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(scalar),
        } = right
        {
            match scalar {
                ScalarLiteral::Integer(i) => {
                    if let Some(negated) = i.checked_neg() {
                        return Type::TypeLiteral {
                            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(negated)),
                        };
                    }
                }
                ScalarLiteral::Bigint(i) => {
                    if let Some(negated) = i.checked_neg() {
                        return Type::TypeLiteral {
                            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Bigint(negated)),
                        };
                    }
                }
                ScalarLiteral::Float(f) => {
                    return Type::TypeLiteral {
                        value: TypeLiteral::ScalarLiteral(ScalarLiteral::Float(-f)),
                    };
                }
                _ => {}
            }
        }
        right.clone()
    }

    /// Infer the result type of a type unary operation.
    pub(super) fn infer_type_unary_operation(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        operator: &TypeUnaryOperator,
        right_ty_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Type {
        match operator {
            TypeUnaryOperator::Not => {
                // invert boolean literals and otherwise return boolean
                let right_ty_id = self.unwrap_type_value(right_ty_id, types);
                let right_ty = types.get_type(right_ty_id).clone();
                self.try_fold_not(&right_ty).unwrap_or(Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::Boolean),
                })
            }
            TypeUnaryOperator::Must => {
                // strip nullish types for must
                let right_ty_id = self.unwrap_type_value(right_ty_id, types);
                let (non_nullish, _) = self.strip_nullish_from_union(right_ty_id, types);
                let Some(non_nullish) = non_nullish else {
                    return Type::TypeLiteral {
                        value: TypeLiteral::Never,
                    };
                };
                types.get_type(non_nullish).clone()
            }
            TypeUnaryOperator::Type => {
                // normalize to a type descriptor for `type`
                let right_ty = types.get_type(right_ty_id).clone();
                if let Type::Value { value } = right_ty {
                    return Type::Value { value };
                }
                Type::Value { value: right_ty_id }
            }
            TypeUnaryOperator::Readonly => {
                // normalize readonly modifiers
                let right_ty_id = self.unwrap_type_value(right_ty_id, types);
                let deep_readonly = self
                    .analyze_context_options_for_module(module.id)
                    .deep_readonly;
                let readonly_id = self.materialize_readonly_type(
                    expression_id.into_any(),
                    right_ty_id,
                    types,
                    deep_readonly,
                );
                types.get_type(readonly_id).clone()
            }
            TypeUnaryOperator::AsConst => {
                // normalize const modifiers with deep readonly
                let right_ty_id = self.unwrap_type_value(right_ty_id, types);
                let readonly_id = self.materialize_readonly_type(
                    expression_id.into_any(),
                    right_ty_id,
                    types,
                    true,
                );
                types.get_type(readonly_id).clone()
            }
            TypeUnaryOperator::AsComptime => {
                // as comptime only changes type-index interpretation
                let right_ty_id = self.unwrap_type_value(right_ty_id, types);
                types.get_type(right_ty_id).clone()
            }
            TypeUnaryOperator::Keyof => {
                // resolve keys for keyof expressions
                let right_ty_id = self.unwrap_type_value(right_ty_id, types);
                let mut visited = Vec::new();
                let key_type_id = self.normalize_keyof_type(
                    module,
                    profile,
                    expression_id.into_any(),
                    None,
                    right_ty_id,
                    symbols,
                    types,
                    NormalizationMode::Assign,
                    RelationMode::TYPE_OPS,
                    &mut visited,
                );
                types.get_type(key_type_id).clone()
            }
            TypeUnaryOperator::Typeof => {
                // typeof expressions evaluate to strings in value contexts
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::String),
                }
            }
            TypeUnaryOperator::Newtype => {
                // newtype is a no-op in expression contexts
                let right_ty_id = self.unwrap_type_value(right_ty_id, types);
                types.get_type(right_ty_id).clone()
            }
        }
    }

    /// Infer the result type of a type binary operation.
    pub(super) fn infer_type_binary_operation(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        operator: &TypeBinaryOperator,
        left_ty_id: LocalTypeId,
        right_ty_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &InferTable,
        options: &AnalyzeOptions,
    ) -> Type {
        match operator {
            TypeBinaryOperator::Cast => {
                // type assertion: `x as T`
                // allow explicit raw pointer casts
                let allow_pointer_cast = {
                    let left_ty = types.get_type(left_ty_id);
                    let right_ty = types.get_type(right_ty_id);

                    matches!(
                        (left_ty, right_ty),
                        (Type::PointerOf { .. }, Type::PointerOf { .. })
                    ) || matches!(
                        (left_ty, right_ty),
                        (
                            Type::TypeLiteral {
                                value: TypeLiteral::Null | TypeLiteral::Undefined,
                            },
                            Type::PointerOf { .. }
                        )
                    )
                };

                // allow explicit enum backing casts
                let allow_enum_cast = {
                    let left_ty = types.get_type(left_ty_id).clone();
                    let right_ty = types.get_type(right_ty_id).clone();

                    self.is_enum_backing_cast(module, profile, &left_ty, &right_ty, types)
                };
                let allow_record_cast =
                    self.allow_record_like_cast(profile, left_ty_id, right_ty_id, types);

                // check if cast is valid (types overlap: at least one direction is assignable)
                let left_to_right = self.is_type_assignable(
                    module,
                    profile,
                    symbols,
                    right_ty_id,
                    left_ty_id,
                    types,
                    options,
                );
                let right_to_left = self.is_type_assignable(
                    module,
                    profile,
                    symbols,
                    left_ty_id,
                    right_ty_id,
                    types,
                    options,
                );

                // reject unsafe type assertions when configured
                if options.no_unsafe_type_assertions && matches!(module.source, ModuleSource::User)
                {
                    // read the source type
                    let left_ty = types.get_type(left_ty_id);

                    // check for any or unknown assertions
                    let is_any_or_unknown = matches!(
                        left_ty,
                        Type::TypeLiteral {
                            value: TypeLiteral::Any | TypeLiteral::Unknown,
                        }
                    );
                    let is_unsafe_cast =
                        is_any_or_unknown || left_to_right == Assignability::NotAssignable;

                    if is_unsafe_cast {
                        self.error(AnalyzeError::UnsafeTypeAssertionDisabled {
                            node: expression_id
                                .into_global_any(module.id)
                                .into_anchored(Some(profile)),
                        });
                    }
                }

                // report invalid casts when types do not overlap
                if !allow_pointer_cast
                    && !allow_enum_cast
                    && !allow_record_cast
                    && left_to_right == Assignability::NotAssignable
                    && right_to_left == Assignability::NotAssignable
                {
                    // neither direction works: illegal cast
                    self.error(AnalyzeError::InvalidCast {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(profile)),
                        from_ty: left_ty_id.into_global(module.id),
                        to_ty: right_ty_id.into_global(module.id),
                    });
                }
                // cast returns the target (right) type
                types.get_type(right_ty_id).clone()
            }
            TypeBinaryOperator::Satisfies => {
                // unwrap Type::Value when comparing against type expressions
                let target_ty_id = match types.get_type(right_ty_id) {
                    Type::Value { value } => *value,
                    _ => right_ty_id,
                };
                let actual_ty_id = match types.get_type(left_ty_id) {
                    Type::Value { value } => *value,
                    _ => left_ty_id,
                };
                let resolved_target = self.materialize_infer_type_for_check(
                    module,
                    profile,
                    symbols,
                    target_ty_id,
                    infer,
                    types,
                    options,
                );
                let resolved_actual = self.materialize_infer_type_for_check(
                    module,
                    profile,
                    symbols,
                    actual_ty_id,
                    infer,
                    types,
                    options,
                );

                // check if left type satisfies (is assignable to) right type
                if self.is_type_assignable(
                    module,
                    profile,
                    symbols,
                    resolved_target,
                    resolved_actual,
                    types,
                    options,
                ) == Assignability::NotAssignable
                {
                    self.error(AnalyzeError::UnsatisfiedType {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(profile)),
                        expected_ty: resolved_target.into_global(module.id),
                        actual_ty: resolved_actual.into_global(module.id),
                    });
                }
                // satisfies returns the original (left) type, not the asserted type
                types.get_type(left_ty_id).clone()
            }
            TypeBinaryOperator::Is | TypeBinaryOperator::InstanceOf => {
                // unwrap type descriptor values to the underlying type
                let target_ty_id = match types.get_type(right_ty_id) {
                    Type::Value { value } => *value,
                    _ => right_ty_id,
                };

                // enforce class-only instanceof targets
                if matches!(operator, TypeBinaryOperator::InstanceOf) {
                    let is_class_target = types
                        .get_type(target_ty_id)
                        .symbol()
                        .is_some_and(|symbol| symbol.local_id.ty == SymbolType::Class);
                    if !is_class_target {
                        self.error(AnalyzeError::InvalidInstanceOfTarget {
                            node: expression_id
                                .into_global_any(module.id)
                                .into_anchored(Some(profile)),
                        });
                    }
                }

                // record runtime check kind for guard expressions
                let value_type_id = self.unwrap_type_value(left_ty_id, types);
                let runtime_check_kind = self.runtime_check_kind_for_relation(
                    module,
                    profile,
                    symbols,
                    value_type_id,
                    target_ty_id,
                    types,
                    options,
                );
                if let Some(kind) = runtime_check_kind {
                    types.set_runtime_check_kind(expression_id.into_global_any(module.id), kind);
                }

                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::Boolean),
                }
            }
            TypeBinaryOperator::In => {
                // check if the left type is a member of the right type keys
                let left_ty_id = self.unwrap_type_value(left_ty_id, types);
                let right_ty_id = self.unwrap_type_value(right_ty_id, types);

                // compute the key space for the right type
                let mut visited = Vec::new();
                let key_type_id = self.normalize_keyof_type(
                    module,
                    profile,
                    expression_id.into_any(),
                    None,
                    right_ty_id,
                    symbols,
                    types,
                    NormalizationMode::Assign,
                    RelationMode::TYPE_OPS,
                    &mut visited,
                );

                // compare the left type against the key space
                let assignability = self.is_type_assignable(
                    module,
                    profile,
                    symbols,
                    key_type_id,
                    left_ty_id,
                    types,
                    options,
                );

                // only emit boolean literals when the relation is static
                let is_decidable = self.type_operator_is_decidable(
                    module,
                    profile,
                    left_ty_id,
                    right_ty_id,
                    symbols,
                    types,
                );
                self.boolean_type_for_assignability(assignability, is_decidable)
            }
            TypeBinaryOperator::Extends | TypeBinaryOperator::Implements => {
                // check assignability for extends/implements
                let left_ty_id = self.unwrap_type_value(left_ty_id, types);
                let right_ty_id = self.unwrap_type_value(right_ty_id, types);

                // compare the left type against the right type
                let assignability = self.is_type_assignable(
                    module,
                    profile,
                    symbols,
                    right_ty_id,
                    left_ty_id,
                    types,
                    options,
                );

                // only emit boolean literals when the relation is static
                let is_decidable = self.type_operator_is_decidable(
                    module,
                    profile,
                    left_ty_id,
                    right_ty_id,
                    symbols,
                    types,
                );
                self.boolean_type_for_assignability(assignability, is_decidable)
            }
        }
    }

    /// Decide whether a type relation can be reduced to a boolean literal.
    fn type_operator_is_decidable(
        &self,
        module: &Module,
        profile: ProfileId,
        left_ty_id: LocalTypeId,
        right_ty_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> bool {
        // static parameters make assignability depend on runtime values
        let mut static_visited = HashSet::new();
        let left_contains_static = self.type_contains_static_parameters(
            module,
            profile,
            left_ty_id,
            symbols,
            types,
            &mut static_visited,
        );
        let mut static_visited = HashSet::new();
        let right_contains_static = self.type_contains_static_parameters(
            module,
            profile,
            right_ty_id,
            symbols,
            types,
            &mut static_visited,
        );
        !(left_contains_static || right_contains_static)
    }

    /// Build a boolean type literal from assignability results.
    fn boolean_type_for_assignability(
        &self,
        assignability: Assignability,
        is_decidable: bool,
    ) -> Type {
        if !is_decidable {
            return Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean),
            };
        }

        let value = matches!(assignability, Assignability::Assignable);
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(value)),
        }
    }

    /// Materialize readonly modifiers for object-like type expressions.
    pub(crate) fn materialize_readonly_type(
        &self,
        source_id: LocalNodeIdAny,
        ty_id: LocalTypeId,
        types: &mut TypeTable,
        deep_readonly: bool,
    ) -> LocalTypeId {
        let mut rewriter = ReadonlyMaterializer::new(source_id);
        if deep_readonly {
            return rewriter.rewrite_type_id(types, ty_id);
        }

        rewriter.apply_shallow(types, ty_id)
    }

    /// Infer the result type of a value of operation.
    pub(super) fn infer_value_of_operation(
        &self,
        mutability: Option<Mutability>,
        variance: Option<VarianceBound>,
        right_ty_id: LocalTypeId,
    ) -> Type {
        // wrap the owned value with explicit ownership
        Type::ValueOf {
            mutability,
            variance,
            right: right_ty_id,
        }
    }

    /// Infer the result type of a reference of operation.
    pub(super) fn infer_reference_of_operation(
        &self,
        mutability: Option<Mutability>,
        variance: Option<VarianceBound>,
        right_ty_id: LocalTypeId,
    ) -> Type {
        // wrap the reference with explicit ownership
        Type::ReferenceOf {
            mutability,
            variance,
            right: right_ty_id,
        }
    }

    /// Infer the type of a member field on a type by key.
    pub(super) fn infer_member_of_type(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        symbols: &SymbolTable,
        receiver_ty: &Type,
        member_key: &StaticKey,
        lookup_mode: MemberLookupMode,
        types: &mut TypeTable,
        visited: &mut Vec<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        match receiver_ty {
            // object type: look up field directly
            Type::Object {
                fields,
                call_signatures,
                ..
            } => {
                // check explicit object fields first
                if let Some(field_ty) =
                    self.member_type_from_fields(fields, member_key, node_id, types)
                {
                    return Ok(Some(field_ty));
                }

                // override bind/call/apply signatures based on strictness policy
                let options = self.analyze_context_options_for_module(module.id);
                if !call_signatures.is_empty()
                    && let Some(synthetic) = self.bind_call_apply_member_type(
                        node_id,
                        receiver_ty,
                        member_key,
                        options.strict_bind_call_apply,
                        types,
                    )
                {
                    return Ok(Some(synthetic));
                }

                // fall back to implicit Object members
                let Some(reference_ty) = self.well_known_type(profile, receiver_ty, types) else {
                    return Ok(None);
                };
                self.infer_member_of_type(
                    module,
                    profile,
                    node_id,
                    symbols,
                    &reference_ty,
                    member_key,
                    lookup_mode,
                    types,
                    visited,
                )
            }

            // value type: unwrap to the underlying type
            Type::Value { .. } => {
                let Some(reference_ty) = self.well_known_type(profile, receiver_ty, types) else {
                    return Ok(None);
                };
                self.infer_member_of_type(
                    module,
                    profile,
                    node_id,
                    symbols,
                    &reference_ty,
                    member_key,
                    lookup_mode,
                    types,
                    visited,
                )
            }

            // reference to a nominal type: expand alias arguments before member lookup
            Type::Reference {
                symbol,
                static_arguments,
            } => {
                if static_arguments.is_some() && matches!(symbol.ty(), SymbolType::TypeAlias) {
                    let mut normalize_visited = Vec::new();
                    if let Some(expanded_id) = self.normalize_type_alias_reference_with_arguments(
                        module,
                        profile,
                        node_id,
                        *symbol,
                        static_arguments.as_deref().unwrap_or(&[]),
                        symbols,
                        types,
                        NormalizationMode::Assign,
                        RelationMode::ASSIGN,
                        &mut normalize_visited,
                    ) {
                        let expanded_ty = types.get_type(expanded_id).clone();
                        return self.infer_member_of_type(
                            module,
                            profile,
                            node_id,
                            symbols,
                            &expanded_ty,
                            member_key,
                            lookup_mode,
                            types,
                            visited,
                        );
                    }
                }

                match lookup_mode {
                    MemberLookupMode::Instance | MemberLookupMode::Any => self
                        .infer_member_of_symbol(
                            module,
                            profile,
                            node_id,
                            symbols,
                            *symbol,
                            member_key,
                            lookup_mode,
                            types,
                            visited,
                        ),
                    MemberLookupMode::Value => {
                        let Some(value_ty_id) = types.get_value_type_id(*symbol) else {
                            return Ok(None);
                        };
                        let value_ty = types.get_type(value_ty_id).clone();
                        self.infer_member_of_type(
                            module,
                            profile,
                            node_id,
                            symbols,
                            &value_ty,
                            member_key,
                            lookup_mode,
                            types,
                            visited,
                        )
                    }
                }
            }

            // array like types: fall back to well known Array members
            Type::Array { .. } | Type::ArraySized { .. } | Type::Tuple { .. } => {
                let Some(reference_ty) = self.well_known_type(profile, receiver_ty, types) else {
                    return Ok(None);
                };
                self.infer_member_of_type(
                    module,
                    profile,
                    node_id,
                    symbols,
                    &reference_ty,
                    member_key,
                    lookup_mode,
                    types,
                    visited,
                )
            }

            // unary wrappers: unwrap before resolving members
            Type::Unary { right, .. }
            | Type::ValueOf { right, .. }
            | Type::ReferenceOf { right, .. }
            | Type::PointerOf { right, .. } => {
                let inner_ty = types.get_type(*right).clone();
                self.infer_member_of_type(
                    module,
                    profile,
                    node_id,
                    symbols,
                    &inner_ty,
                    member_key,
                    lookup_mode,
                    types,
                    visited,
                )
            }

            // union type: require all elements to have the field, return union of field types
            Type::Union { elements } => {
                let element_ids = elements.clone();
                let mut field_types: Vec<LocalTypeId> = Vec::new();
                for element_id in element_ids {
                    let element_ty = types.get_type(element_id).clone();
                    if let Some(field_ty) = self.infer_member_of_type(
                        module,
                        profile,
                        node_id,
                        symbols,
                        &element_ty,
                        member_key,
                        lookup_mode,
                        types,
                        visited,
                    )? {
                        field_types.push(field_ty);
                    } else {
                        return Ok(None);
                    }
                }
                // if all field types are the same, return that type
                // otherwise, return a union of the field types
                if field_types.is_empty() {
                    Ok(None)
                } else if field_types.len() == 1 {
                    Ok(Some(field_types[0]))
                } else {
                    // check if all types are identical
                    let first = field_types[0];
                    if field_types.iter().all(|&t| t == first) {
                        Ok(Some(first))
                    } else {
                        Ok(Some(types.insert_type_from_any(
                            Type::Union {
                                elements: field_types,
                            },
                            node_id,
                        )))
                    }
                }
            }

            // intersection type: first match wins
            Type::Intersection { elements } => {
                let element_ids = elements.clone();
                for element_id in element_ids {
                    let element_ty = types.get_type(element_id).clone();
                    if let Some(field_ty) = self.infer_member_of_type(
                        module,
                        profile,
                        node_id,
                        symbols,
                        &element_ty,
                        member_key,
                        lookup_mode,
                        types,
                        visited,
                    )? {
                        return Ok(Some(field_ty));
                    }
                }
                Ok(None)
            }

            Type::Function { .. } => {
                let options = self.analyze_context_options_for_module(module.id);

                // override bind/call/apply signatures based on strictness policy
                if let Some(synthetic) = self.bind_call_apply_member_type(
                    node_id,
                    receiver_ty,
                    member_key,
                    options.strict_bind_call_apply,
                    types,
                ) {
                    return Ok(Some(synthetic));
                }

                let Some(reference_ty) = self.well_known_type(profile, receiver_ty, types) else {
                    return Ok(None);
                };
                self.infer_member_of_type(
                    module,
                    profile,
                    node_id,
                    symbols,
                    &reference_ty,
                    member_key,
                    lookup_mode,
                    types,
                    visited,
                )
            }

            Type::TypeLiteral { .. } => {
                let Some(reference_ty) = self.well_known_type(profile, receiver_ty, types) else {
                    return Ok(None);
                };
                self.infer_member_of_type(
                    module,
                    profile,
                    node_id,
                    symbols,
                    &reference_ty,
                    member_key,
                    lookup_mode,
                    types,
                    visited,
                )
            }

            _ => Ok(None),
        }
    }

    /// Build a member type from fields that share the same key.
    fn member_type_from_fields(
        &self,
        fields: &[TypeField],
        member_key: &StaticKey,
        node_id: LocalNodeIdAny,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        let mut matching = Vec::new();
        for field in fields {
            if field.key.matches(member_key) {
                matching.push(field.ty);
            }
        }

        if matching.is_empty() {
            return None;
        }

        if matching.len() == 1 {
            return Some(matching[0]);
        }

        let all_functions = matching
            .iter()
            .all(|ty_id| matches!(types.get_type(*ty_id), Type::Function { .. }));
        if all_functions {
            let overload_set = Type::Object {
                fields: Vec::new(),
                call_signatures: matching,
                construct_signatures: Vec::new(),
                index_signatures: Vec::new(),
            };
            return Some(types.insert_type_from_any(overload_set, node_id));
        }

        let source_type_id = matching[0];
        Some(self.union_type_from_list(matching, source_type_id, types))
    }

    /// Build bind/call/apply member types for callable receivers.
    fn bind_call_apply_member_type(
        &self,
        node_id: LocalNodeIdAny,
        receiver_ty: &Type,
        member_key: &StaticKey,
        is_strict_bind_call_apply: bool,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        let member_name = member_key.name()?;

        // only override call/apply/bind with strict signatures
        let member_name = self.program.strings.get(member_name);
        let member_name = member_name.as_ref();
        if member_name != "call" && member_name != "apply" && member_name != "bind" {
            return None;
        }

        let mut member_signatures = Vec::new();
        match receiver_ty {
            Type::Function { .. } => {
                if let Some(signature_id) = self.bind_call_apply_signature_from_type(
                    node_id,
                    receiver_ty,
                    member_name,
                    is_strict_bind_call_apply,
                    types,
                ) {
                    member_signatures.push(signature_id);
                }
            }
            Type::Object {
                call_signatures, ..
            } => {
                for signature_id in call_signatures {
                    let signature_ty = types.get_type(*signature_id).clone();
                    if let Some(member_signature_id) = self.bind_call_apply_signature_from_type(
                        node_id,
                        &signature_ty,
                        member_name,
                        is_strict_bind_call_apply,
                        types,
                    ) {
                        member_signatures.push(member_signature_id);
                    }
                }
            }
            _ => return None,
        }

        if member_signatures.is_empty() {
            return None;
        }

        if member_signatures.len() == 1 {
            return Some(member_signatures[0]);
        }

        let overload_set = Type::Object {
            fields: Vec::new(),
            call_signatures: member_signatures,
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        };
        Some(types.insert_type_from_any(overload_set, node_id))
    }

    fn bind_call_apply_signature_from_type(
        &self,
        node_id: LocalNodeIdAny,
        receiver_ty: &Type,
        member_name: &str,
        is_strict_bind_call_apply: bool,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        let Type::Function {
            static_parameters,
            this_parameter,
            dynamic_parameters,
            return_type,
            ..
        } = receiver_ty
        else {
            return None;
        };

        // resolve the strict this argument type
        let strict_this_arg = if let Some(this_parameter) = this_parameter {
            *this_parameter
        } else {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            types.insert_type_from_any(ty, node_id)
        };
        let non_strict_arg = {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            types.insert_type_from_any(ty, node_id)
        };

        // use strict or permissive bind/call/apply argument typing
        let this_arg = if is_strict_bind_call_apply {
            strict_this_arg
        } else {
            non_strict_arg
        };
        let call_parameters = if is_strict_bind_call_apply {
            dynamic_parameters.clone()
        } else {
            vec![non_strict_arg]
        };

        // build shared function metadata
        let asynchrony = Asynchrony::Sync;
        let cardinality = FunctionCardinality::Scalar;

        // build the strict member signature
        let member_ty = match member_name {
            "call" => {
                // call(thisArg, ...args) -> return_type
                let mut params = Vec::with_capacity(call_parameters.len() + 1);
                params.push(this_arg);
                params.extend_from_slice(&call_parameters);

                Type::Function {
                    asynchrony,
                    cardinality,
                    static_parameters: static_parameters.clone(),
                    this_parameter: None,
                    dynamic_parameters: params,
                    return_type: *return_type,
                }
            }
            "apply" => {
                // apply(thisArg, argsTuple) -> return_type
                let tuple_elements = call_parameters
                    .iter()
                    .map(|ty| TypeElement::new(*ty))
                    .collect();
                let tuple_ty = Type::Tuple {
                    elements: tuple_elements,
                    is_readonly: false,
                };
                let tuple_ty_id = types.insert_type_from_any(tuple_ty, node_id);

                Type::Function {
                    asynchrony,
                    cardinality,
                    static_parameters: static_parameters.clone(),
                    this_parameter: None,
                    dynamic_parameters: vec![this_arg, tuple_ty_id],
                    return_type: *return_type,
                }
            }
            "bind" => {
                // bind(thisArg, ...args) -> bound function
                let mut params = Vec::with_capacity(call_parameters.len() + 1);
                params.push(this_arg);
                params.extend_from_slice(&call_parameters);

                let bound_function = Type::Function {
                    asynchrony,
                    cardinality,
                    static_parameters: static_parameters.clone(),
                    this_parameter: None,
                    dynamic_parameters: call_parameters,
                    return_type: *return_type,
                };
                let bound_function_id = types.insert_type_from_any(bound_function, node_id);

                Type::Function {
                    asynchrony,
                    cardinality,
                    static_parameters: static_parameters.clone(),
                    this_parameter: None,
                    dynamic_parameters: params,
                    return_type: Some(bound_function_id),
                }
            }
            _ => return None,
        };

        Some(types.insert_type_from_any(member_ty, node_id))
    }

    /// Infer the index signature value type for a member key.
    pub(super) fn infer_index_signature_value_type_for_key(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        symbols: &SymbolTable,
        receiver_ty: &Type,
        member_key: &StaticKey,
        types: &mut TypeTable,
        visited: &mut Vec<GlobalSymbolId>,
    ) -> Option<LocalTypeId> {
        match receiver_ty {
            Type::Object {
                index_signatures, ..
            } => self.index_signature_value_type_for_key(index_signatures, member_key, types),
            Type::Value { value } => {
                let value_ty = types.get_type(*value).clone();
                self.infer_index_signature_value_type_for_key(
                    module, profile, node_id, symbols, &value_ty, member_key, types, visited,
                )
            }
            Type::Reference { symbol, .. } => self.infer_index_signature_value_type_for_symbol(
                module, profile, node_id, symbols, *symbol, member_key, types, visited,
            ),
            Type::Unary { right, .. }
            | Type::ValueOf { right, .. }
            | Type::ReferenceOf { right, .. }
            | Type::PointerOf { right, .. } => {
                let inner_ty = types.get_type(*right).clone();
                self.infer_index_signature_value_type_for_key(
                    module, profile, node_id, symbols, &inner_ty, member_key, types, visited,
                )
            }
            Type::Union { elements } => {
                let mut value_types = Vec::new();
                for element_id in elements.clone() {
                    let element_ty = types.get_type(element_id).clone();
                    if let Some(value_ty) = self.infer_index_signature_value_type_for_key(
                        module,
                        profile,
                        node_id,
                        symbols,
                        &element_ty,
                        member_key,
                        types,
                        visited,
                    ) {
                        value_types.push(value_ty);
                    } else {
                        return None;
                    }
                }
                match value_types.len() {
                    0 => None,
                    1 => Some(value_types[0]),
                    _ => {
                        let source_type_id = value_types[0];
                        Some(self.union_type_from_list(value_types, source_type_id, types))
                    }
                }
            }
            Type::Intersection { elements } => {
                for element_id in elements.clone() {
                    let element_ty = types.get_type(element_id).clone();
                    if let Some(value_ty) = self.infer_index_signature_value_type_for_key(
                        module,
                        profile,
                        node_id,
                        symbols,
                        &element_ty,
                        member_key,
                        types,
                        visited,
                    ) {
                        return Some(value_ty);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Infer member type for a nominal type symbol, traversing lineage and extensions.
    fn infer_member_of_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        symbols: &SymbolTable,
        symbol: GlobalSymbolId,
        member_key: &StaticKey,
        lookup_mode: MemberLookupMode,
        types: &mut TypeTable,
        visited: &mut Vec<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // cycle detection: if we've already visited this symbol, stop
        // NOTE #Suspicious: should we really just return None for already visited symbol types?
        if visited.contains(&symbol) {
            return Ok(None);
        }
        visited.push(symbol);

        // resolve instance types before walking members and extensions
        self.resolve_instance_type_for_symbol(module, profile, node_id, symbol, types)?;

        // step 1: look up in the type's own instance type
        if let Some(ty_id) =
            self.apparent_instance_type(module, profile, node_id, symbol, symbols, types)
        {
            let ty = types.get_type(ty_id).clone();
            if let Some(member_ty) = self.infer_member_of_type(
                module,
                profile,
                node_id,
                symbols,
                &ty,
                member_key,
                lookup_mode,
                types,
                visited,
            )? {
                return Ok(Some(member_ty));
            }
        }

        // step 2: traverse lineage (extends, implements, embedded)
        let lineage = types.get_lineage_for_symbol(symbol).cloned();
        if let Some(lineage) = lineage {
            // check parent type (extends)
            if let Some(extends) = lineage.extends
                && let Some(member_ty) = self.infer_member_of_symbol(
                    module,
                    profile,
                    node_id,
                    symbols,
                    extends,
                    member_key,
                    lookup_mode,
                    types,
                    visited,
                )?
            {
                return Ok(Some(member_ty));
            }

            // check implemented interfaces
            for implements in &lineage.implements {
                if let Some(member_ty) = self.infer_member_of_symbol(
                    module,
                    profile,
                    node_id,
                    symbols,
                    *implements,
                    member_key,
                    lookup_mode,
                    types,
                    visited,
                )? {
                    return Ok(Some(member_ty));
                }
            }

            // check embedded types
            for embedded in &lineage.embedded {
                if let Some(member_ty) = self.infer_member_of_symbol(
                    module,
                    profile,
                    node_id,
                    symbols,
                    *embedded,
                    member_key,
                    lookup_mode,
                    types,
                    visited,
                )? {
                    return Ok(Some(member_ty));
                }
            }
        }

        // step 3: check visible extensions
        let extension_symbols =
            self.visible_extension_symbols_for_target(module, profile, symbols, types, symbol)?;
        for extension_symbol in extension_symbols {
            let Some(extension) =
                self.extension_for_symbol_in_module(module, profile, extension_symbol, types)?
            else {
                continue;
            };
            if !self.is_extension_visible(module, &extension) {
                continue;
            }

            // ensure the extension instance type is available
            if let Some(ty_id) = self.apparent_instance_type(
                module,
                profile,
                node_id,
                extension_symbol,
                symbols,
                types,
            ) {
                let ty = types.get_type(ty_id).clone();
                if let Type::Object { fields, .. } = ty
                    && let Some(field_ty) =
                        self.member_type_from_fields(&fields, member_key, node_id, types)
                {
                    return Ok(Some(field_ty));
                }
            }
        }

        Ok(None)
    }

    /// Infer the index signature value type for a symbol.
    fn infer_index_signature_value_type_for_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        symbols: &SymbolTable,
        symbol: GlobalSymbolId,
        member_key: &StaticKey,
        types: &mut TypeTable,
        visited: &mut Vec<GlobalSymbolId>,
    ) -> Option<LocalTypeId> {
        if visited.contains(&symbol) {
            return None;
        }
        visited.push(symbol);

        // step 1: look up in the type's own instance type
        if let Some(ty_id) =
            self.apparent_instance_type(module, profile, node_id, symbol, symbols, types)
        {
            let ty = types.get_type(ty_id).clone();
            if let Some(value_ty) = self.infer_index_signature_value_type_for_key(
                module, profile, node_id, symbols, &ty, member_key, types, visited,
            ) {
                return Some(value_ty);
            }
        }

        // step 2: traverse lineage (extends, implements, embedded)
        if let Some(lineage) = types.get_lineage_for_symbol(symbol).cloned() {
            if let Some(extends) = lineage.extends
                && let Some(value_ty) = self.infer_index_signature_value_type_for_symbol(
                    module, profile, node_id, symbols, extends, member_key, types, visited,
                )
            {
                return Some(value_ty);
            }

            for implements in &lineage.implements {
                if let Some(value_ty) = self.infer_index_signature_value_type_for_symbol(
                    module,
                    profile,
                    node_id,
                    symbols,
                    *implements,
                    member_key,
                    types,
                    visited,
                ) {
                    return Some(value_ty);
                }
            }

            for embedded in &lineage.embedded {
                if let Some(value_ty) = self.infer_index_signature_value_type_for_symbol(
                    module, profile, node_id, symbols, *embedded, member_key, types, visited,
                ) {
                    return Some(value_ty);
                }
            }
        }

        // step 3: check visible extensions
        let extension_symbols = match self
            .visible_extension_symbols_for_target(module, profile, symbols, types, symbol)
        {
            Ok(symbols) => symbols,
            Err(AnalyzeError::Yield { .. }) => return None,
            Err(error) => {
                self.error(error);
                return None;
            }
        };
        for extension_symbol in extension_symbols {
            let extension =
                match self.extension_for_symbol_in_module(module, profile, extension_symbol, types)
                {
                    Ok(extension) => extension,
                    Err(AnalyzeError::Yield { .. }) => return None,
                    Err(error) => {
                        self.error(error);
                        return None;
                    }
                };
            let Some(extension) = extension else {
                continue;
            };
            if !self.is_extension_visible(module, &extension) {
                continue;
            }
            if let Some(ty_id) = self.apparent_instance_type(
                module,
                profile,
                node_id,
                extension_symbol,
                symbols,
                types,
            ) {
                let ty = types.get_type(ty_id).clone();
                if let Some(value_ty) = self.infer_index_signature_value_type_for_key(
                    module, profile, node_id, symbols, &ty, member_key, types, visited,
                ) {
                    return Some(value_ty);
                }
            }
        }

        None
    }

    /// Get the idnex signature value type for a member key.
    fn index_signature_value_type_for_key(
        &self,
        index_signatures: &[TypeIndexSignature],
        member_key: &StaticKey,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        let key_kind = index_key_kind_for_member(member_key);
        let mut value_types = Vec::new();

        for signature in index_signatures {
            let signature_kind = index_key_kind_for_type(signature.key_type, types);
            if index_key_kinds_compatible_for_access(signature_kind, key_kind) {
                value_types.push(signature.value_type);
            }
        }

        match value_types.len() {
            0 => None,
            1 => Some(value_types[0]),
            _ => {
                let source_type_id = value_types[0];
                Some(self.union_type_from_list(value_types, source_type_id, types))
            }
        }
    }

    /// Check if an extension is visible from the given module:
    /// Native: Extension in same module as target type, always visible wherever type is used.
    /// Anonymous: Extension on foreign type, only visible in the file where it is declared.
    /// Named: Extension on foreign type, must be explicitly imported to use.
    pub(super) fn is_extension_visible(&self, module: &Module, extension: &Extension) -> bool {
        match extension.kind {
            ExtensionKind::Inherent => true,
            ExtensionKind::Local => extension.symbol.module_id == module.id,
            ExtensionKind::Nominal => true,
        }
    }

    /// Resolve a remote symbol's value type by ensuring its module is analyzed
    /// and copying the type into the current module's TypeTable.
    pub(crate) fn resolve_remote_symbol_value_type(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        target_symbol: GlobalSymbolId,
        is_surface_inference: bool,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        let remote_module_id = target_symbol.module_id;
        let error_node = node_id.into_global(module.id).into_anchored(Some(profile));

        // reject export inference cycles that lack explicit annotations
        if is_surface_inference
            && self.export_inference_has_cycle(module.id, profile, remote_module_id)?
            && !self.remote_symbol_has_declared_value_type(profile, target_symbol)
        {
            return Err(AnalyzeError::ExportInferenceRequiresAnnotation { node: error_node });
        }

        // reject type-only symbols in value resolution
        if !self.symbol_is_value_capable(profile, target_symbol) {
            if module.language_type.is_declaration() {
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                return Ok(types.insert_type_from_any(ty, node_id));
            }

            self.error(AnalyzeError::TypeOnlyValue { node: error_node });
            let ty = Type::Error;
            return Ok(types.insert_type_from_any(ty, node_id));
        }

        self.with_module_tree_symbols_for_stage(
            module,
            profile,
            remote_module_id,
            AnalyzeReadStage::Infer,
            |remote_module, remote_tree, remote_symbols| {
                let remote_dir = remote_module.dir(profile);
                let mut remote_types = remote_dir.types.write();
                if let Some(remote_ty_id) = remote_types.get_value_type_id(target_symbol) {
                    // materialize and import the remote type
                    self.materialize_imported_type(
                        remote_module,
                        profile,
                        remote_ty_id,
                        remote_tree,
                        remote_symbols,
                        &mut remote_types,
                    )?;
                    let remote_ty = remote_types.get_type(remote_ty_id);
                    let local_ty = self.import_type_from_remote_for_node(
                        node_id,
                        remote_ty,
                        &remote_types,
                        target_symbol,
                        types,
                    );
                    Ok(local_ty)
                } else if let Some(primary_declaration) = remote_symbols
                    .get_symbol(target_symbol.local_id)
                    .primary_declaration
                    && primary_declaration.module_id == remote_module.id
                    && let Some(signature_ty_id) =
                        remote_types.get_signature_type_for_node(primary_declaration)
                {
                    // materialize and import declared member signatures
                    self.materialize_imported_type(
                        remote_module,
                        profile,
                        signature_ty_id,
                        remote_tree,
                        remote_symbols,
                        &mut remote_types,
                    )?;
                    let remote_ty = remote_types.get_type(signature_ty_id);
                    let local_ty = self.import_type_from_remote_for_node(
                        node_id,
                        remote_ty,
                        &remote_types,
                        target_symbol,
                        types,
                    );
                    Ok(local_ty)
                } else {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    Ok(types.insert_type_from_any(ty, node_id))
                }
            },
        )
        .map_err(|error| {
            // only reject when we have an explicit export inference cycle
            if is_surface_inference
                && !self.remote_symbol_has_declared_value_type(profile, target_symbol)
                && let Ok(has_cycle) =
                    self.export_inference_has_cycle(module.id, profile, remote_module_id)
                && has_cycle
            {
                return AnalyzeError::ExportInferenceRequiresAnnotation { node: error_node };
            }
            AnalyzeError::from(error)
        })?
    }

    /// Check whether a remote symbol has an explicit value type annotation.
    pub(crate) fn remote_symbol_has_declared_value_type(
        &self,
        profile: ProfileId,
        symbol: GlobalSymbolId,
    ) -> bool {
        self.with_module_tree_symbols_by_id(
            profile,
            symbol.module_id,
            |remote_module, tree, symbols| {
                let types = remote_module.dir(profile).types.read();

                // check for explicit declarator annotations
                if let Some(declarator_id) =
                    self.direct_binding_declarator_for_symbol(remote_module, symbol, tree, symbols)
                {
                    let node_id = declarator_id.into_global_any(remote_module.id);
                    if types.get_declared_type_id(node_id).is_some() {
                        return true;
                    }
                }

                // check for annotated function declarations
                let symbol_entry = symbols.get_symbol(symbol.local_id);
                let Some(primary_declaration) = symbol_entry.primary_declaration else {
                    return false;
                };
                if primary_declaration.module_id != remote_module.id {
                    return false;
                }
                let Some(declaration_id) =
                    self.declaration_id_from_primary(tree, primary_declaration.local_id)
                else {
                    return false;
                };
                let Declaration::Function { signature, .. } = tree.get(declaration_id) else {
                    return false;
                };

                signature.return_type.is_some()
            },
        )
    }

    /// Resolve a declaration id from a primary declaration node.
    fn declaration_id_from_primary(
        &self,
        tree: &NodeTree,
        primary_declaration: LocalNodeIdAny,
    ) -> Option<LocalNodeId<Declaration>> {
        // lift declaration nodes out of primary declaration wrappers
        match primary_declaration.ty {
            NodeType::Declaration => Some(primary_declaration.into_typed()),
            NodeType::Expression => {
                let expression_id = primary_declaration.into_typed::<Expression>();
                match tree.get(expression_id) {
                    Expression::Declaration { declaration } => Some(*declaration),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Check if an export inference cycle is detected between two modules.
    pub(crate) fn export_inference_has_cycle(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        remote_module_id: ModuleId,
    ) -> AnalyzeResult<bool> {
        // short circuit direct self references
        if module_id == remote_module_id {
            return Ok(true);
        }

        // walk resolved dependency edges for the remote module
        let mut visited = HashSet::new();
        let mut queue = vec![remote_module_id];
        while let Some(current) = queue.pop() {
            // skip modules we have already visited
            if !visited.insert(current) {
                continue;
            }

            // stop when we reach the source module
            if current == module_id {
                return Ok(true);
            }

            // ensure imports are resolved before inspecting module dependencies
            self.require_resolve_module_direct(current, profile)?;

            // collect direct module dependencies for cycle checks
            let dependencies = self.module_dependency_ids_for_cycle_detection(current, profile)?;
            for dependency in dependencies {
                queue.push(dependency);
            }
        }

        Ok(false)
    }

    /// Collect module dependency ids for export inference cycle detection.
    fn module_dependency_ids_for_cycle_detection(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> AnalyzeResult<Vec<ModuleId>> {
        // load the module dir for dependency discovery
        let module = self.program.modules.get(module_id);
        let module = module.read();

        // stop when the module has no dir for this profile
        let Some(dir) = module.dir_maybe(profile) else {
            return Ok(Vec::new());
        };

        // collect remote dependency targets from resolved dependency items
        let tree = dir.tree.read();
        let mut dependencies = Vec::new();
        for item_id in tree.iter_node_ids_of_type::<DependencyItem>() {
            let DependencyItem::Remote { target_module, .. } = tree.get(item_id) else {
                continue;
            };

            for target in [target_module.value, target_module.ty] {
                if let Some(ModuleTarget::Module(target_id)) = target {
                    dependencies.push(target_id);
                }
            }
        }

        Ok(dependencies)
    }

    /// Build a stable local expression id for imported fixed-array counts.
    fn imported_array_sized_count_expression_id(
        &self,
        node_id: LocalNodeIdAny,
        remote_count: LocalNodeId<Expression>,
        target_symbol: GlobalSymbolId,
    ) -> LocalNodeId<Expression> {
        // synthesize an expression id in a high range to avoid colliding with real tree nodes
        let symbol_bits = target_symbol.local_id.id;
        let mixed = node_id.id
            ^ remote_count.id.rotate_left(11)
            ^ symbol_bits.rotate_left(21)
            ^ 0x6a09_e667;
        let synthetic = 0x8000_0000 | (mixed & 0x7fff_fffe);
        LocalNodeId::new(synthetic)
    }

    /// Import a type from a remote module into the current module's TypeTable.
    /// For structural types (arrays, objects, ..): recursively copy the type structure.
    /// For nominal types (Type::Reference): keep them as references to the original symbol.
    pub(crate) fn import_type_from_remote_for_node(
        &self,
        node_id: LocalNodeIdAny,
        remote_ty: &Type,
        remote_types: &TypeTable,
        target_symbol: GlobalSymbolId,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        match remote_ty {
            // leaf types: copy directly
            Type::TypeLiteral { value } => types.insert_imported_type_from_any(
                Type::TypeLiteral {
                    value: value.clone(),
                },
                node_id,
            ),
            Type::InferVar { .. } => types.insert_imported_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                },
                node_id,
            ),
            Type::Error => types.insert_imported_type_from_any(Type::Error, node_id),
            Type::This => types.insert_imported_type_from_any(Type::This, node_id),
            Type::Conditional {
                distributive_symbol,
                left,
                right,
                then_type,
                else_type,
            } => {
                let local_left = self.import_type_from_remote_for_node(
                    node_id,
                    remote_types.get_type(*left),
                    remote_types,
                    target_symbol,
                    types,
                );
                let local_right = self.import_type_from_remote_for_node(
                    node_id,
                    remote_types.get_type(*right),
                    remote_types,
                    target_symbol,
                    types,
                );
                let local_then = self.import_type_from_remote_for_node(
                    node_id,
                    remote_types.get_type(*then_type),
                    remote_types,
                    target_symbol,
                    types,
                );
                let local_else = self.import_type_from_remote_for_node(
                    node_id,
                    remote_types.get_type(*else_type),
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_imported_type_from_any(
                    Type::Conditional {
                        distributive_symbol: *distributive_symbol,
                        left: local_left,
                        right: local_right,
                        then_type: local_then,
                        else_type: local_else,
                    },
                    node_id,
                )
            }
            Type::Mapped {
                parameter,
                modifiers,
                value,
            } => {
                let local_constraint = self.import_type_from_remote_for_node(
                    node_id,
                    remote_types.get_type(parameter.constraint),
                    remote_types,
                    target_symbol,
                    types,
                );
                let local_key_remap = parameter.key_remap.map(|key_remap| {
                    self.import_type_from_remote_for_node(
                        node_id,
                        remote_types.get_type(key_remap),
                        remote_types,
                        target_symbol,
                        types,
                    )
                });
                let local_value = self.import_type_from_remote_for_node(
                    node_id,
                    remote_types.get_type(*value),
                    remote_types,
                    target_symbol,
                    types,
                );
                let parameter = TypeMappedParameter {
                    name: parameter.name,
                    symbol: parameter.symbol,
                    constraint: local_constraint,
                    key_remap: local_key_remap,
                };
                types.insert_imported_type_from_any(
                    Type::Mapped {
                        parameter,
                        modifiers: *modifiers,
                        value: local_value,
                    },
                    node_id,
                )
            }
            Type::Index { left, index } => {
                let local_left = self.import_type_from_remote_for_node(
                    node_id,
                    remote_types.get_type(*left),
                    remote_types,
                    target_symbol,
                    types,
                );
                let local_index = self.import_type_from_remote_for_node(
                    node_id,
                    remote_types.get_type(*index),
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_imported_type_from_any(
                    Type::Index {
                        left: local_left,
                        index: local_index,
                    },
                    node_id,
                )
            }
            Type::TemplateLiteral { strings, spans } => {
                let local_spans = spans
                    .iter()
                    .map(|span| {
                        self.import_type_from_remote_for_node(
                            node_id,
                            remote_types.get_type(*span),
                            remote_types,
                            target_symbol,
                            types,
                        )
                    })
                    .collect();
                types.insert_imported_type_from_any(
                    Type::TemplateLiteral {
                        strings: strings.clone(),
                        spans: local_spans,
                    },
                    node_id,
                )
            }
            Type::Import {
                target,
                qualifier,
                static_arguments,
            } => {
                // map embedded type ids inside static arguments
                let local_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.import_static_argument_from_remote_for_node(
                                node_id,
                                argument,
                                remote_types,
                                target_symbol,
                                types,
                            )
                        })
                        .collect::<Vec<_>>()
                });
                types.insert_imported_type_from_any(
                    Type::Import {
                        target: *target,
                        qualifier: qualifier.clone(),
                        static_arguments: local_arguments,
                    },
                    node_id,
                )
            }
            Type::Infer { name, constraint } => {
                let local_constraint = constraint.map(|constraint| {
                    self.import_type_from_remote_for_node(
                        node_id,
                        remote_types.get_type(constraint),
                        remote_types,
                        target_symbol,
                        types,
                    )
                });
                types.insert_imported_type_from_any(
                    Type::Infer {
                        name: *name,
                        constraint: local_constraint,
                    },
                    node_id,
                )
            }
            Type::Predicate {
                asserts,
                subject,
                target,
            } => {
                let local_target = target.map(|target| {
                    self.import_type_from_remote_for_node(
                        node_id,
                        remote_types.get_type(target),
                        remote_types,
                        target_symbol,
                        types,
                    )
                });
                types.insert_imported_type_from_any(
                    Type::Predicate {
                        asserts: *asserts,
                        subject: *subject,
                        target: local_target,
                    },
                    node_id,
                )
            }

            // array types
            Type::Array {
                element: None,
                is_readonly,
            } => types.insert_imported_type_from_any(
                Type::Array {
                    element: None,
                    is_readonly: *is_readonly,
                },
                node_id,
            ),
            Type::Array {
                element: Some(element_id),
                is_readonly,
            } => {
                let element_ty = remote_types.get_type(*element_id);
                let local_elem = self.import_type_from_remote_for_node(
                    node_id,
                    element_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_imported_type_from_any(
                    Type::Array {
                        element: Some(local_elem),
                        is_readonly: *is_readonly,
                    },
                    node_id,
                )
            }

            // tuple types
            Type::Tuple {
                elements,
                is_readonly,
            } => {
                let local_elements: Vec<_> = elements
                    .iter()
                    .map(|element| {
                        let ty = remote_types.get_type(element.ty);
                        let local_ty = self.import_type_from_remote_for_node(
                            node_id,
                            ty,
                            remote_types,
                            target_symbol,
                            types,
                        );
                        let mut element = element.clone();
                        element.ty = local_ty;
                        element
                    })
                    .collect();
                types.insert_imported_type_from_any(
                    Type::Tuple {
                        elements: local_elements,
                        is_readonly: *is_readonly,
                    },
                    node_id,
                )
            }

            // object types
            Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                let local_fields: Vec<_> = fields
                    .iter()
                    .map(|field| {
                        let ty = remote_types.get_type(field.ty);
                        let local_ty = self.import_type_from_remote_for_node(
                            node_id,
                            ty,
                            remote_types,
                            target_symbol,
                            types,
                        );
                        TypeField {
                            key: field.key,
                            ty: local_ty,
                            is_optional: field.is_optional,
                            is_readonly: field.is_readonly,
                        }
                    })
                    .collect();
                let local_call_signatures: Vec<_> = call_signatures
                    .iter()
                    .map(|signature| {
                        let ty = remote_types.get_type(*signature);
                        self.import_type_from_remote_for_node(
                            node_id,
                            ty,
                            remote_types,
                            target_symbol,
                            types,
                        )
                    })
                    .collect();
                let local_construct_signatures: Vec<_> = construct_signatures
                    .iter()
                    .map(|signature| {
                        let ty = remote_types.get_type(*signature);
                        self.import_type_from_remote_for_node(
                            node_id,
                            ty,
                            remote_types,
                            target_symbol,
                            types,
                        )
                    })
                    .collect();
                let local_index_signatures: Vec<_> = index_signatures
                    .iter()
                    .map(|signature| {
                        let key_type = remote_types.get_type(signature.key_type);
                        let value_type = remote_types.get_type(signature.value_type);
                        TypeIndexSignature {
                            name: signature.name,
                            key_type: self.import_type_from_remote_for_node(
                                node_id,
                                key_type,
                                remote_types,
                                target_symbol,
                                types,
                            ),
                            value_type: self.import_type_from_remote_for_node(
                                node_id,
                                value_type,
                                remote_types,
                                target_symbol,
                                types,
                            ),
                            is_readonly: signature.is_readonly,
                        }
                    })
                    .collect();
                types.insert_imported_type_from_any(
                    Type::Object {
                        fields: local_fields,
                        call_signatures: local_call_signatures,
                        construct_signatures: local_construct_signatures,
                        index_signatures: local_index_signatures,
                    },
                    node_id,
                )
            }

            // function types
            Type::Function {
                asynchrony,
                cardinality,
                static_parameters,
                this_parameter,
                dynamic_parameters,
                return_type,
            } => {
                let local_static_params: Vec<_> = static_parameters
                    .iter()
                    .map(|id| {
                        let ty = remote_types.get_type(*id);
                        self.import_type_from_remote_for_node(
                            node_id,
                            ty,
                            remote_types,
                            target_symbol,
                            types,
                        )
                    })
                    .collect();
                let local_this = this_parameter.map(|this_parameter| {
                    let ty = remote_types.get_type(this_parameter);
                    self.import_type_from_remote_for_node(
                        node_id,
                        ty,
                        remote_types,
                        target_symbol,
                        types,
                    )
                });
                let local_dynamic_params: Vec<_> = dynamic_parameters
                    .iter()
                    .map(|id| {
                        let ty = remote_types.get_type(*id);
                        self.import_type_from_remote_for_node(
                            node_id,
                            ty,
                            remote_types,
                            target_symbol,
                            types,
                        )
                    })
                    .collect();
                let local_return = return_type.map(|id| {
                    let ty = remote_types.get_type(id);
                    self.import_type_from_remote_for_node(
                        node_id,
                        ty,
                        remote_types,
                        target_symbol,
                        types,
                    )
                });
                types.insert_imported_type_from_any(
                    Type::Function {
                        asynchrony: *asynchrony,
                        cardinality: *cardinality,
                        static_parameters: local_static_params,
                        this_parameter: local_this,
                        dynamic_parameters: local_dynamic_params,
                        return_type: local_return,
                    },
                    node_id,
                )
            }

            // union and intersection types
            Type::Union { elements } => {
                let local_elements: Vec<_> = elements
                    .iter()
                    .map(|id| {
                        let ty = remote_types.get_type(*id);
                        self.import_type_from_remote_for_node(
                            node_id,
                            ty,
                            remote_types,
                            target_symbol,
                            types,
                        )
                    })
                    .collect();
                types.insert_imported_type_from_any(
                    Type::Union {
                        elements: local_elements,
                    },
                    node_id,
                )
            }
            Type::Intersection { elements } => {
                let local_elements: Vec<_> = elements
                    .iter()
                    .map(|id| {
                        let ty = remote_types.get_type(*id);
                        self.import_type_from_remote_for_node(
                            node_id,
                            ty,
                            remote_types,
                            target_symbol,
                            types,
                        )
                    })
                    .collect();
                types.insert_imported_type_from_any(
                    Type::Intersection {
                        elements: local_elements,
                    },
                    node_id,
                )
            }

            // type modifiers: recurse into inner type
            Type::Value { value } => {
                let inner_ty = remote_types.get_type(*value);
                let local_inner = self.import_type_from_remote_for_node(
                    node_id,
                    inner_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_imported_type_from_any(Type::Value { value: local_inner }, node_id)
            }
            Type::ValueOf {
                mutability,
                variance,
                right,
            } => {
                let inner_ty = remote_types.get_type(*right);
                let local_inner = self.import_type_from_remote_for_node(
                    node_id,
                    inner_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_imported_type_from_any(
                    Type::ValueOf {
                        mutability: *mutability,
                        variance: *variance,
                        right: local_inner,
                    },
                    node_id,
                )
            }
            Type::ReferenceOf {
                mutability,
                variance,
                right,
            } => {
                let inner_ty = remote_types.get_type(*right);
                let local_inner = self.import_type_from_remote_for_node(
                    node_id,
                    inner_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_imported_type_from_any(
                    Type::ReferenceOf {
                        mutability: *mutability,
                        variance: *variance,
                        right: local_inner,
                    },
                    node_id,
                )
            }
            Type::PointerOf { mutability, right } => {
                let inner_ty = remote_types.get_type(*right);
                let local_inner = self.import_type_from_remote_for_node(
                    node_id,
                    inner_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_imported_type_from_any(
                    Type::PointerOf {
                        mutability: *mutability,
                        right: local_inner,
                    },
                    node_id,
                )
            }
            Type::Unary { operator, right } => {
                let inner_ty = remote_types.get_type(*right);
                let local_inner = self.import_type_from_remote_for_node(
                    node_id,
                    inner_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_imported_type_from_any(
                    Type::Unary {
                        operator: *operator,
                        right: local_inner,
                    },
                    node_id,
                )
            }
            Type::Binary {
                left,
                operator,
                right,
            } => {
                let left_ty = remote_types.get_type(*left);
                let right_ty = remote_types.get_type(*right);
                let local_left = self.import_type_from_remote_for_node(
                    node_id,
                    left_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                let local_right = self.import_type_from_remote_for_node(
                    node_id,
                    right_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_imported_type_from_any(
                    Type::Binary {
                        left: local_left,
                        operator: *operator,
                        right: local_right,
                    },
                    node_id,
                )
            }

            // nominal/reference types: keep as Type::Reference to the original symbol
            Type::Reference {
                symbol,
                static_arguments,
            } => {
                // map embedded type ids inside static arguments
                let local_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.import_static_argument_from_remote_for_node(
                                node_id,
                                argument,
                                remote_types,
                                target_symbol,
                                types,
                            )
                        })
                        .collect::<Vec<_>>()
                });
                types.insert_imported_type_from_any(
                    Type::Reference {
                        symbol: *symbol,
                        static_arguments: local_arguments,
                    },
                    node_id,
                )
            }

            // unevaluated remote types can't be imported reliably, fall back to a reference
            Type::Unevaluated(_) => types.insert_imported_type_from_any(
                Type::Reference {
                    symbol: target_symbol,
                    static_arguments: None,
                },
                node_id,
            ),
            // import fixed arrays with a local synthetic count id and inferred count type
            Type::ArraySized {
                element,
                count,
                is_readonly,
            } => {
                let local_element = self.import_type_from_remote_for_node(
                    node_id,
                    remote_types.get_type(*element),
                    remote_types,
                    target_symbol,
                    types,
                );
                let local_count =
                    self.imported_array_sized_count_expression_id(node_id, *count, target_symbol);

                let remote_count_global = count.into_global_any(remote_types.module_id);
                if let Some(remote_count_type_id) = remote_types
                    .get_declared_type_id(remote_count_global)
                    .or_else(|| remote_types.get_inferred_type_id(remote_count_global))
                {
                    let local_count_type_id = self.import_type_from_remote_for_node(
                        node_id,
                        remote_types.get_type(remote_count_type_id),
                        remote_types,
                        target_symbol,
                        types,
                    );
                    let local_count_global = local_count.into_global_any(types.module_id);
                    types.set_inferred_type(local_count_global, local_count_type_id);
                    if let Some(remote_resolution_id) =
                        remote_types.get_resolution_for_node(remote_count_global)
                        && let Resolution::Static { candidate, .. } =
                            remote_types.get_resolution(remote_resolution_id)
                    {
                        let local_resolution = Resolution::Static {
                            receiver: None,
                            candidate: ResolutionCandidate {
                                key: None,
                                target_symbol: candidate.target_symbol,
                                instance: None,
                                resolved_signature: None,
                            },
                        };
                        let local_resolution_id = types.insert_resolution(local_resolution);
                        types.set_resolution_for_node(local_count_global, local_resolution_id);
                    }
                }

                types.insert_imported_type_from_any(
                    Type::ArraySized {
                        element: local_element,
                        count: local_count,
                        is_readonly: *is_readonly,
                    },
                    node_id,
                )
            }
        }
    }

    /// Import a static argument from a remote module into the local type table.
    fn import_static_argument_from_remote_for_node(
        &self,
        node_id: LocalNodeIdAny,
        argument: &StaticArgument,
        remote_types: &TypeTable,
        target_symbol: GlobalSymbolId,
        types: &mut TypeTable,
    ) -> StaticArgument {
        match argument {
            StaticArgument::Unevaluated { .. } => argument.clone(),
            StaticArgument::Evaluated { name, value } => {
                let mapped_value = self.import_static_expression_from_remote_for_node(
                    node_id,
                    value,
                    remote_types,
                    target_symbol,
                    types,
                );
                StaticArgument::Evaluated {
                    name: *name,
                    value: mapped_value,
                }
            }
        }
    }

    /// Import a static expression from a remote module into the local type table.
    fn import_static_expression_from_remote_for_node(
        &self,
        node_id: LocalNodeIdAny,
        expression: &StaticExpression,
        remote_types: &TypeTable,
        target_symbol: GlobalSymbolId,
        types: &mut TypeTable,
    ) -> StaticExpression {
        match expression {
            StaticExpression::Unevaluated { .. } => expression.clone(),
            StaticExpression::ScalarLiteral { .. } => expression.clone(),
            StaticExpression::TypeLiteral { .. } => expression.clone(),
            StaticExpression::Type { ty } => {
                let remote_ty = remote_types.get_type(*ty);
                let local_ty = self.import_type_from_remote_for_node(
                    node_id,
                    remote_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                StaticExpression::Type { ty: local_ty }
            }
            StaticExpression::Declaration {
                declaration,
                static_arguments,
            } => {
                // remap static arguments for declarations
                let local_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.import_static_argument_from_remote_for_node(
                                node_id,
                                argument,
                                remote_types,
                                target_symbol,
                                types,
                            )
                        })
                        .collect::<Vec<_>>()
                });
                StaticExpression::Declaration {
                    declaration: *declaration,
                    static_arguments: local_arguments,
                }
            }
            StaticExpression::ArrayExpression { elements } => {
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        self.import_static_expression_from_remote_for_node(
                            node_id,
                            element,
                            remote_types,
                            target_symbol,
                            types,
                        )
                    })
                    .collect();
                StaticExpression::ArrayExpression {
                    elements: mapped_elements,
                }
            }
            StaticExpression::TupleExpression { elements } => {
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        self.import_static_expression_from_remote_for_node(
                            node_id,
                            element,
                            remote_types,
                            target_symbol,
                            types,
                        )
                    })
                    .collect();
                StaticExpression::TupleExpression {
                    elements: mapped_elements,
                }
            }
            StaticExpression::ObjectExpression { properties } => {
                let mapped_properties = properties
                    .iter()
                    .map(|property| {
                        self.import_static_property_from_remote_for_node(
                            node_id,
                            property,
                            remote_types,
                            target_symbol,
                            types,
                        )
                    })
                    .collect();
                StaticExpression::ObjectExpression {
                    properties: mapped_properties,
                }
            }
        }
    }

    /// Import a static property from a remote module into the local type table.
    fn import_static_property_from_remote_for_node(
        &self,
        node_id: LocalNodeIdAny,
        property: &StaticProperty,
        remote_types: &TypeTable,
        target_symbol: GlobalSymbolId,
        types: &mut TypeTable,
    ) -> StaticProperty {
        match property {
            StaticProperty::Unevaluated { .. } => property.clone(),
            StaticProperty::Field {
                modifiers,
                key,
                value,
                default,
                symbol,
            } => {
                let mapped_value = self.import_static_expression_from_remote_for_node(
                    node_id,
                    value,
                    remote_types,
                    target_symbol,
                    types,
                );
                let mapped_default = default.as_ref().map(|default| {
                    self.import_static_expression_from_remote_for_node(
                        node_id,
                        default,
                        remote_types,
                        target_symbol,
                        types,
                    )
                });
                StaticProperty::Field {
                    modifiers: *modifiers,
                    key: *key,
                    value: mapped_value,
                    default: mapped_default,
                    symbol: *symbol,
                }
            }
            StaticProperty::Method {
                modifiers,
                key,
                signature,
                body,
                symbol,
            } => {
                let mapped_body = self.import_static_expression_from_remote_for_node(
                    node_id,
                    body,
                    remote_types,
                    target_symbol,
                    types,
                );
                StaticProperty::Method {
                    modifiers: *modifiers,
                    key: *key,
                    signature: signature.clone(),
                    body: mapped_body,
                    symbol: *symbol,
                }
            }
        }
    }

    /// Check whether the receiver explicitly implements a language item interface.
    pub(super) fn is_interface_implemented(
        &self,
        module: &Module,
        profile: ProfileId,
        ty: &Type,
        interface_item: LanguageSymbol,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> bool {
        let interface_symbol = self.language_symbol(profile, interface_item);
        match ty {
            Type::Value { value } => {
                let inner_ty = types.get_type(*value);
                self.is_interface_implemented(
                    module,
                    profile,
                    inner_ty,
                    interface_item,
                    symbols,
                    types,
                )
            }
            Type::Reference { symbol, .. } => {
                let canonical_symbol = self.canonical_symbol_id(
                    module,
                    symbols,
                    profile,
                    *symbol,
                    CanonicalSymbolMode::FollowAliases,
                );
                self.is_type_lineage_assignable(
                    module,
                    profile,
                    canonical_symbol,
                    interface_symbol,
                    symbols,
                    types,
                )
            }
            Type::Union { elements } => elements.iter().all(|element_id| {
                let element_ty = types.get_type(*element_id);
                self.is_interface_implemented(
                    module,
                    profile,
                    element_ty,
                    interface_item,
                    symbols,
                    types,
                )
            }),
            _ => false,
        }
    }

    /// Check whether a type is definitely a struct type.
    pub(super) fn is_definitely_struct_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Reference { symbol, .. } => symbol.ty() == SymbolType::Struct,
            _ => false,
        }
    }

    /// Check whether a type is unresolved for operator resolution.
    pub(super) fn is_unresolved_operator_type(&self, ty: &Type, types: &TypeTable) -> bool {
        match ty {
            Type::InferVar { .. } => true,
            Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            } => true,
            Type::Union { elements } => elements.iter().any(|element_id| {
                self.is_unresolved_operator_type(types.get_type(*element_id), types)
            }),
            _ => false,
        }
    }

    /// Check whether a type behaves like a numeric type.
    pub(super) fn is_numeric_like_type(&self, ty: &Type, types: &TypeTable) -> bool {
        match ty {
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(primitive),
            } => matches!(
                primitive,
                PrimitiveType::Number
                    | PrimitiveType::Int(_)
                    | PrimitiveType::Float(_)
                    | PrimitiveType::Bigint
            ),
            Type::TypeLiteral {
                value:
                    TypeLiteral::ScalarLiteral(
                        ScalarLiteral::Integer(_)
                        | ScalarLiteral::Float(_)
                        | ScalarLiteral::Bigint(_),
                    ),
            } => true,
            Type::Union { elements } => elements
                .iter()
                .all(|element_id| self.is_numeric_like_type(types.get_type(*element_id), types)),
            _ => false,
        }
    }

    /// Check whether a type is a primitive or scalar literal for builtin operators.
    pub(super) fn is_primitive_literal_type(&self, ty: &Type, types: &TypeTable) -> bool {
        match ty {
            Type::TypeLiteral {
                value:
                    TypeLiteral::Primitive(_)
                    | TypeLiteral::ScalarLiteral(_)
                    | TypeLiteral::Null
                    | TypeLiteral::Undefined,
            } => true,
            Type::Union { elements } => elements.iter().all(|element_id| {
                self.is_primitive_literal_type(types.get_type(*element_id), types)
            }),
            _ => false,
        }
    }

    /// Check whether a type is a literal value.
    pub(super) fn is_literal_value_type(&self, ty: &Type) -> bool {
        matches!(
            ty,
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(_) | TypeLiteral::Null | TypeLiteral::Undefined,
            }
        )
    }

    /// Extract the return type from a function type.
    pub(super) fn function_return_type(
        &self,
        fn_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        match types.get_type(fn_ty_id) {
            Type::Function { return_type, .. } => *return_type,
            _ => None,
        }
    }

    /// Decide whether a return type allows implicit fallthrough.
    pub(super) fn return_type_allows_fallthrough_infer(
        &self,
        ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        match types.get_type(ty_id) {
            Type::TypeLiteral {
                value:
                    TypeLiteral::Void
                    | TypeLiteral::Undefined
                    | TypeLiteral::Any
                    | TypeLiteral::Unknown
                    | TypeLiteral::Infer,
            } => true,
            Type::Predicate { asserts: true, .. } => true,
            Type::Union { elements } => elements
                .iter()
                .any(|element| self.return_type_allows_fallthrough_infer(*element, types)),
            Type::InferVar { .. } => true,
            Type::Error => true,
            _ => false,
        }
    }

    /// Substitute `this` types with a concrete receiver type.
    pub(super) fn substitute_this_type(
        &self,
        ty_id: LocalTypeId,
        this_ty_id: LocalTypeId,
        types: &mut TypeTable,
        cache: &mut HashMap<LocalTypeId, LocalTypeId>,
    ) -> LocalTypeId {
        if let Some(mapped) = cache.get(&ty_id).copied() {
            return mapped;
        }

        let ty = types.get_type(ty_id).clone();
        let mapped = match ty {
            Type::This => this_ty_id,
            Type::Reference {
                symbol,
                static_arguments,
            } => {
                if let Some(static_arguments) = static_arguments {
                    let mut changed = false;
                    let mapped_arguments = static_arguments
                        .iter()
                        .map(|argument| {
                            let mapped = self.substitute_this_static_argument(
                                argument, this_ty_id, types, cache,
                            );
                            if mapped != *argument {
                                changed = true;
                            }
                            mapped
                        })
                        .collect::<Vec<_>>();

                    if changed {
                        if !mapped_arguments.is_empty() {
                            self.register_instance_for_symbol(
                                symbol,
                                mapped_arguments.clone(),
                                types,
                            );
                        }

                        types.insert_type_from_type(
                            Type::Reference {
                                symbol,
                                static_arguments: Some(mapped_arguments),
                            },
                            ty_id,
                        )
                    } else {
                        ty_id
                    }
                } else {
                    ty_id
                }
            }
            Type::TypeLiteral { .. }
            | Type::InferVar { .. }
            | Type::Unevaluated(_)
            | Type::Import { .. }
            | Type::Error => ty_id,
            Type::Value { value } => {
                let mapped_value = self.substitute_this_type(value, this_ty_id, types, cache);
                if mapped_value == value {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Value {
                            value: mapped_value,
                        },
                        ty_id,
                    )
                }
            }
            Type::Unary { operator, right } => {
                let mapped_right = self.substitute_this_type(right, this_ty_id, types, cache);
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Unary {
                            operator,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::Binary {
                left,
                operator,
                right,
            } => {
                let mapped_left = self.substitute_this_type(left, this_ty_id, types, cache);
                let mapped_right = self.substitute_this_type(right, this_ty_id, types, cache);
                if mapped_left == left && mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Binary {
                            left: mapped_left,
                            operator,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::Conditional {
                distributive_symbol,
                left,
                right,
                then_type,
                else_type,
            } => {
                let mapped_left = self.substitute_this_type(left, this_ty_id, types, cache);
                let mapped_right = self.substitute_this_type(right, this_ty_id, types, cache);
                let mapped_then = self.substitute_this_type(then_type, this_ty_id, types, cache);
                let mapped_else = self.substitute_this_type(else_type, this_ty_id, types, cache);
                if mapped_left == left
                    && mapped_right == right
                    && mapped_then == then_type
                    && mapped_else == else_type
                {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Conditional {
                            distributive_symbol,
                            left: mapped_left,
                            right: mapped_right,
                            then_type: mapped_then,
                            else_type: mapped_else,
                        },
                        ty_id,
                    )
                }
            }
            Type::Mapped {
                parameter,
                modifiers,
                value,
            } => {
                let mapped_constraint =
                    self.substitute_this_type(parameter.constraint, this_ty_id, types, cache);
                let mapped_key_remap = parameter.key_remap.map(|key_remap| {
                    self.substitute_this_type(key_remap, this_ty_id, types, cache)
                });
                let mapped_value = self.substitute_this_type(value, this_ty_id, types, cache);
                if mapped_constraint == parameter.constraint
                    && mapped_key_remap == parameter.key_remap
                    && mapped_value == value
                {
                    ty_id
                } else {
                    let parameter = TypeMappedParameter {
                        name: parameter.name,
                        symbol: parameter.symbol,
                        constraint: mapped_constraint,
                        key_remap: mapped_key_remap,
                    };
                    types.insert_type_from_type(
                        Type::Mapped {
                            parameter,
                            modifiers,
                            value: mapped_value,
                        },
                        ty_id,
                    )
                }
            }
            Type::Index { left, index } => {
                let mapped_left = self.substitute_this_type(left, this_ty_id, types, cache);
                let mapped_index = self.substitute_this_type(index, this_ty_id, types, cache);
                if mapped_left == left && mapped_index == index {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Index {
                            left: mapped_left,
                            index: mapped_index,
                        },
                        ty_id,
                    )
                }
            }
            Type::TemplateLiteral { strings, spans } => {
                let mut changed = false;
                let mapped_spans = spans
                    .iter()
                    .map(|span| {
                        let mapped = self.substitute_this_type(*span, this_ty_id, types, cache);
                        if mapped != *span {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                if changed {
                    types.insert_type_from_type(
                        Type::TemplateLiteral {
                            strings,
                            spans: mapped_spans,
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::Infer { name, constraint } => {
                let mapped_constraint = constraint.map(|constraint| {
                    self.substitute_this_type(constraint, this_ty_id, types, cache)
                });
                if mapped_constraint == constraint {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Infer {
                            name,
                            constraint: mapped_constraint,
                        },
                        ty_id,
                    )
                }
            }
            Type::Predicate {
                asserts,
                subject,
                target,
            } => {
                let mapped_target = target
                    .map(|target| self.substitute_this_type(target, this_ty_id, types, cache));
                if mapped_target == target {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Predicate {
                            asserts,
                            subject,
                            target: mapped_target,
                        },
                        ty_id,
                    )
                }
            }
            Type::ValueOf {
                mutability,
                variance,
                right,
            } => {
                let mapped_right = self.substitute_this_type(right, this_ty_id, types, cache);
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::ValueOf {
                            mutability,
                            variance,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::ReferenceOf {
                mutability,
                variance,
                right,
            } => {
                let mapped_right = self.substitute_this_type(right, this_ty_id, types, cache);
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::ReferenceOf {
                            mutability,
                            variance,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::PointerOf { mutability, right } => {
                let mapped_right = self.substitute_this_type(right, this_ty_id, types, cache);
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::PointerOf {
                            mutability,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::ArraySized {
                element,
                count,
                is_readonly,
            } => {
                let mapped_element = self.substitute_this_type(element, this_ty_id, types, cache);
                if mapped_element == element {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::ArraySized {
                            element: mapped_element,
                            count,
                            is_readonly,
                        },
                        ty_id,
                    )
                }
            }
            Type::Array {
                element,
                is_readonly,
            } => {
                let mapped_element = element
                    .map(|element| self.substitute_this_type(element, this_ty_id, types, cache));
                if mapped_element == element {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Array {
                            element: mapped_element,
                            is_readonly,
                        },
                        ty_id,
                    )
                }
            }
            Type::Tuple {
                elements,
                is_readonly,
            } => {
                let mut changed = false;
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        let mapped =
                            self.substitute_this_type(element.ty, this_ty_id, types, cache);
                        if mapped != element.ty {
                            changed = true;
                        }
                        let mut element = element.clone();
                        element.ty = mapped;
                        element
                    })
                    .collect::<Vec<_>>();
                if changed {
                    types.insert_type_from_type(
                        Type::Tuple {
                            elements: mapped_elements,
                            is_readonly,
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                let mut changed = false;
                let mapped_fields = fields
                    .iter()
                    .map(|field| {
                        let mapped = self.substitute_this_type(field.ty, this_ty_id, types, cache);
                        if mapped != field.ty {
                            changed = true;
                        }
                        TypeField {
                            key: field.key,
                            ty: mapped,
                            is_optional: field.is_optional,
                            is_readonly: field.is_readonly,
                        }
                    })
                    .collect::<Vec<_>>();
                let mapped_call_signatures = call_signatures
                    .iter()
                    .map(|signature| {
                        let mapped =
                            self.substitute_this_type(*signature, this_ty_id, types, cache);
                        if mapped != *signature {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                let mapped_construct_signatures = construct_signatures
                    .iter()
                    .map(|signature| {
                        let mapped =
                            self.substitute_this_type(*signature, this_ty_id, types, cache);
                        if mapped != *signature {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                let mapped_index_signatures = index_signatures
                    .iter()
                    .map(|signature| {
                        let mapped_key =
                            self.substitute_this_type(signature.key_type, this_ty_id, types, cache);
                        let mapped_value = self.substitute_this_type(
                            signature.value_type,
                            this_ty_id,
                            types,
                            cache,
                        );
                        if mapped_key != signature.key_type || mapped_value != signature.value_type
                        {
                            changed = true;
                        }
                        let mut signature = signature.clone();
                        signature.key_type = mapped_key;
                        signature.value_type = mapped_value;
                        signature
                    })
                    .collect::<Vec<_>>();
                if changed {
                    types.insert_type_from_type(
                        Type::Object {
                            fields: mapped_fields,
                            call_signatures: mapped_call_signatures,
                            construct_signatures: mapped_construct_signatures,
                            index_signatures: mapped_index_signatures,
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::Function {
                asynchrony,
                cardinality,
                static_parameters,
                this_parameter,
                dynamic_parameters,
                return_type,
            } => {
                let mut changed = false;
                let mapped_static_parameters = static_parameters
                    .iter()
                    .map(|parameter| {
                        let mapped =
                            self.substitute_this_type(*parameter, this_ty_id, types, cache);
                        if mapped != *parameter {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                let mapped_this = this_parameter.map(|this_parameter| {
                    let mapped =
                        self.substitute_this_type(this_parameter, this_ty_id, types, cache);
                    if mapped != this_parameter {
                        changed = true;
                    }
                    mapped
                });
                let mapped_parameters = dynamic_parameters
                    .iter()
                    .map(|parameter| {
                        let mapped =
                            self.substitute_this_type(*parameter, this_ty_id, types, cache);
                        if mapped != *parameter {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                let mapped_return = return_type.map(|return_type| {
                    let mapped = self.substitute_this_type(return_type, this_ty_id, types, cache);
                    if mapped != return_type {
                        changed = true;
                    }
                    mapped
                });
                if changed {
                    types.insert_type_from_type(
                        Type::Function {
                            asynchrony,
                            cardinality,
                            static_parameters: mapped_static_parameters,
                            this_parameter: mapped_this,
                            dynamic_parameters: mapped_parameters,
                            return_type: mapped_return,
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::Union { elements } => {
                let mut changed = false;
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        let mapped = self.substitute_this_type(*element, this_ty_id, types, cache);
                        if mapped != *element {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                if changed {
                    types.insert_type_from_type(
                        Type::Union {
                            elements: mapped_elements,
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::Intersection { elements } => {
                let mut changed = false;
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        let mapped = self.substitute_this_type(*element, this_ty_id, types, cache);
                        if mapped != *element {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                if changed {
                    types.insert_type_from_type(
                        Type::Intersection {
                            elements: mapped_elements,
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
        };

        cache.insert(ty_id, mapped);
        mapped
    }

    /// Substitute `this` types in a static argument.
    pub(super) fn substitute_this_static_argument(
        &self,
        argument: &StaticArgument,
        this_ty_id: LocalTypeId,
        types: &mut TypeTable,
        cache: &mut HashMap<LocalTypeId, LocalTypeId>,
    ) -> StaticArgument {
        match argument {
            StaticArgument::Unevaluated { .. } => argument.clone(),
            StaticArgument::Evaluated { name, value } => {
                let mapped_value =
                    self.substitute_this_static_expression(value, this_ty_id, types, cache);
                StaticArgument::Evaluated {
                    name: *name,
                    value: mapped_value,
                }
            }
        }
    }

    /// Substitute `this` types in a static expression.
    pub(super) fn substitute_this_static_expression(
        &self,
        expression: &StaticExpression,
        this_ty_id: LocalTypeId,
        types: &mut TypeTable,
        cache: &mut HashMap<LocalTypeId, LocalTypeId>,
    ) -> StaticExpression {
        match expression {
            StaticExpression::Unevaluated { .. } => expression.clone(),
            StaticExpression::ScalarLiteral { .. } => expression.clone(),
            StaticExpression::TypeLiteral { .. } => expression.clone(),
            StaticExpression::Type { ty } => StaticExpression::Type {
                ty: self.substitute_this_type(*ty, this_ty_id, types, cache),
            },
            StaticExpression::Declaration {
                declaration,
                static_arguments,
            } => {
                let mapped_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.substitute_this_static_argument(argument, this_ty_id, types, cache)
                        })
                        .collect::<Vec<_>>()
                });
                StaticExpression::Declaration {
                    declaration: *declaration,
                    static_arguments: mapped_arguments,
                }
            }
            StaticExpression::ArrayExpression { elements } => {
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        self.substitute_this_static_expression(element, this_ty_id, types, cache)
                    })
                    .collect::<Vec<_>>();
                StaticExpression::ArrayExpression {
                    elements: mapped_elements,
                }
            }
            StaticExpression::TupleExpression { elements } => {
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        self.substitute_this_static_expression(element, this_ty_id, types, cache)
                    })
                    .collect::<Vec<_>>();
                StaticExpression::TupleExpression {
                    elements: mapped_elements,
                }
            }
            StaticExpression::ObjectExpression { properties } => {
                let mapped_properties = properties
                    .iter()
                    .map(|property| {
                        self.substitute_this_static_property(property, this_ty_id, types, cache)
                    })
                    .collect::<Vec<_>>();
                StaticExpression::ObjectExpression {
                    properties: mapped_properties,
                }
            }
        }
    }

    /// Substitute `this` types in a static property.
    pub(super) fn substitute_this_static_property(
        &self,
        property: &StaticProperty,
        this_ty_id: LocalTypeId,
        types: &mut TypeTable,
        cache: &mut HashMap<LocalTypeId, LocalTypeId>,
    ) -> StaticProperty {
        match property {
            StaticProperty::Unevaluated { .. } => property.clone(),
            StaticProperty::Field {
                modifiers,
                key,
                value,
                default,
                symbol,
            } => {
                let mapped_value =
                    self.substitute_this_static_expression(value, this_ty_id, types, cache);
                let mapped_default = default.as_ref().map(|default| {
                    self.substitute_this_static_expression(default, this_ty_id, types, cache)
                });
                StaticProperty::Field {
                    modifiers: *modifiers,
                    key: *key,
                    value: mapped_value,
                    default: mapped_default,
                    symbol: *symbol,
                }
            }
            StaticProperty::Method {
                modifiers,
                key,
                signature,
                body,
                symbol,
            } => {
                let mapped_body =
                    self.substitute_this_static_expression(body, this_ty_id, types, cache);
                StaticProperty::Method {
                    modifiers: *modifiers,
                    key: *key,
                    signature: signature.clone(),
                    body: mapped_body,
                    symbol: *symbol,
                }
            }
        }
    }

    /// Strip nullish types from a type id.
    pub(super) fn strip_nullish_from_union(
        &self,
        ty_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> (Option<LocalTypeId>, bool) {
        let ty = types.get_type(ty_id);

        match ty {
            Type::Union { elements } => {
                let mut filtered = Vec::new();
                let mut has_nullish = false;

                for element_id in elements {
                    let element_ty = types.get_type(*element_id);
                    if self.is_nullish_type(element_ty) {
                        has_nullish = true;
                    } else {
                        filtered.push(*element_id);
                    }
                }

                if !has_nullish {
                    return (Some(ty_id), false);
                }

                let non_nullish_ty_id = match filtered.len() {
                    0 => None,
                    1 => Some(filtered[0]),
                    _ => {
                        Some(types.insert_type_from_type(Type::Union { elements: filtered }, ty_id))
                    }
                };

                (non_nullish_ty_id, true)
            }
            _ if self.is_nullish_type(ty) => (None, true),
            _ => (Some(ty_id), false),
        }
    }

    /// Check whether a type has the given property key.
    pub(super) fn type_has_property(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        key: &StaticKey,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> bool {
        // anchor apparent type resolution to the current type source
        let source_id = types.get_type_source(type_id);

        // walk through shapes that can carry fields
        let ty = types.get_type(type_id).clone();
        match ty {
            Type::Object { fields, .. } => fields.iter().any(|field| field.key.matches(key)),
            Type::Reference { symbol, .. } => {
                // follow apparent instance types for nominal references
                self.apparent_instance_type(module, profile, source_id, symbol, symbols, types)
                    .is_some_and(|instance_id| {
                        self.type_has_property(module, profile, instance_id, key, symbols, types)
                    })
            }
            Type::Intersection { elements } => {
                // accept any intersection member that matches
                elements.iter().any(|element_id| {
                    self.type_has_property(module, profile, *element_id, key, symbols, types)
                })
            }
            _ => false,
        }
    }

    /// Resolve the type for a field with the given key.
    pub(super) fn type_field_type_for_key(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        key: &StaticKey,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<(LocalTypeId, bool)>> {
        // unwrap aliases before walking fields
        let type_id =
            self.unwrap_type_alias_reference(module, profile, type_id, tree, symbols, types)?;
        let mut field_types = Vec::new();
        let mut is_optional = true;

        // collect matching field types for the key
        let mut pending_type_ids = vec![type_id];
        let mut visited_type_ids = Vec::new();
        while let Some(current_type_id) = pending_type_ids.pop() {
            if visited_type_ids.contains(&current_type_id) {
                continue;
            }
            visited_type_ids.push(current_type_id);
            match types.get_type(current_type_id) {
                Type::Object { fields, .. } => {
                    // collect all matching fields from the object
                    for field in fields {
                        if field.key.matches(key) {
                            field_types.push(field.ty);
                            is_optional = is_optional && field.is_optional;
                        }
                    }
                }
                Type::Reference { symbol, .. } => {
                    // prefer apparent instance types for nominal references
                    let source_id = types.get_type_source(current_type_id);
                    if let Some(instance_id) = self
                        .apparent_instance_type(module, profile, source_id, *symbol, symbols, types)
                    {
                        pending_type_ids.push(instance_id);
                    }
                }
                Type::Intersection { elements } => {
                    // gather fields from every element
                    for element_id in elements {
                        pending_type_ids.push(*element_id);
                    }
                }
                _ => {}
            }
        }
        if field_types.is_empty() {
            return Ok(None);
        }

        // combine multiple field types with intersection
        let field_type_id = match field_types.len() {
            1 => field_types[0],
            _ => {
                let source_type_id = field_types[0];
                types.insert_type_from_type(
                    Type::Intersection {
                        elements: field_types,
                    },
                    source_type_id,
                )
            }
        };

        Ok(Some((field_type_id, is_optional)))
    }

    /// Check whether a type id is any or unknown.
    pub(super) fn type_is_any_or_unknown(&self, type_id: LocalTypeId, types: &TypeTable) -> bool {
        let ty = types.get_type(type_id);
        matches!(
            ty,
            Type::TypeLiteral {
                value: TypeLiteral::Any | TypeLiteral::Unknown,
            }
        )
    }

    /// Build a Promise reference type id with an optional value type.
    pub(super) fn promise_type(
        &self,
        profile: ProfileId,
        value_type: Option<LocalTypeId>,
        source_id: LocalNodeIdAny,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        let promise_symbol = self.get_well_known_type_symbol(profile, WellKnownSymbol::Promise)?;

        // default missing type arguments to unknown
        let value_type = value_type.unwrap_or_else(|| {
            types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                },
                source_id,
            )
        });

        // make promise type
        let static_arguments = vec![StaticArgument::Evaluated {
            name: None,
            value: StaticExpression::Type { ty: value_type },
        }];
        Some(types.insert_type_from_any(
            Type::Reference {
                symbol: promise_symbol,
                static_arguments: Some(static_arguments),
            },
            source_id,
        ))
    }

    /// Resolve generator context types from a declared return type.
    pub(super) fn generator_context_types(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        return_type: Option<LocalTypeId>,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> (LocalTypeId, LocalTypeId, LocalTypeId) {
        let unknown_ty_id = types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            },
            source_id,
        );
        let Some(return_type_id) = return_type else {
            return (unknown_ty_id, unknown_ty_id, unknown_ty_id);
        };

        if let Some((yield_ty_id, return_ty_id, next_ty_id)) =
            self.generator_type_arguments(module, profile, return_type_id, symbols, types)
        {
            return (yield_ty_id, return_ty_id, next_ty_id);
        }

        (unknown_ty_id, return_type_id, unknown_ty_id)
    }

    /// Unwrap a Promise reference into its value type when possible.
    pub(super) fn unwrap_promise_type(
        &self,
        module: &Module,
        symbols: &SymbolTable,
        profile: ProfileId,
        type_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        let ty = types.get_type(type_id);
        let symbol = ty.symbol()?;
        let static_arguments = match ty {
            Type::Reference {
                static_arguments, ..
            } => static_arguments.clone(),
            _ => return None,
        };

        // compare canonical symbols to avoid alias mismatches
        let canonical_symbol = self.canonical_symbol_id(
            module,
            symbols,
            profile,
            symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        let is_promise_symbol =
            self.is_well_known_symbol(profile, canonical_symbol, WellKnownSymbol::Promise);
        if !is_promise_symbol {
            return None;
        }

        // resolve unevaluated promise arguments when possible
        let static_arguments = if let Some(static_arguments) = static_arguments.as_ref()
            && static_arguments
                .iter()
                .any(|arg| matches!(arg, StaticArgument::Unevaluated { .. }))
        {
            let options = self.analyze_context_options_for_module(module.id);
            let tree = module.dir(profile).tree.read();
            self.resolve_type_reference_static_arguments(
                module,
                profile,
                types.get_type_source(type_id),
                symbol,
                Some(static_arguments),
                true,
                &options,
                &tree,
                symbols,
                types,
            )
            .ok()
            .flatten()
            .or_else(|| Some(static_arguments.clone()))
        } else {
            static_arguments
        };

        let Some(first_argument) = static_arguments
            .as_ref()
            .and_then(|arguments| arguments.first())
        else {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            return Some(types.insert_type_from_type(ty, type_id));
        };

        // evaluate remaining unevaluated arguments as types when possible
        let mut argument = first_argument.clone();
        if let StaticArgument::Unevaluated { node } = argument {
            let tree = module.dir(profile).tree.read();
            let symbols = module.dir(profile).symbols.read();
            if let Ok(Some(evaluated)) =
                self.evaluate_static_argument_as_type(module, profile, node, &tree, &symbols, types)
            {
                argument = evaluated;
            }
        }

        Some(self.convert_static_argument_type(&argument, types.get_type_source(type_id), types))
    }

    /// Extract generator type arguments from a reference when possible.
    pub(super) fn generator_type_arguments(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<(LocalTypeId, LocalTypeId, LocalTypeId)> {
        let (symbol, static_arguments) = {
            let Type::Reference {
                symbol,
                static_arguments,
            } = types.get_type(type_id)
            else {
                return None;
            };

            (*symbol, static_arguments.clone())
        };
        let source_id = types.get_type_source(type_id);

        let canonical_symbol = self.canonical_symbol_id(
            module,
            symbols,
            profile,
            symbol,
            CanonicalSymbolMode::FollowAliases,
        );

        let generator_name = self.program.strings.intern("Generator");
        let generator_symbol = self.get_declared_lib_symbol_from(
            profile,
            generator_name,
            SymbolSpaceOrder::TypeThenValue,
        );
        let iterator_symbol = self.get_well_known_type_symbol(profile, WellKnownSymbol::Iterator);

        let is_generator = generator_symbol.is_some_and(|symbol| symbol == canonical_symbol)
            || iterator_symbol.is_some_and(|symbol| symbol == canonical_symbol);
        if !is_generator {
            return None;
        }

        let options = self.analyze_context_options_for_module(module.id);
        let tree = module.dir(profile).tree.read();
        let resolved_arguments = self
            .resolve_type_reference_static_arguments(
                module,
                profile,
                source_id,
                symbol,
                static_arguments.as_deref(),
                true,
                &options,
                &tree,
                symbols,
                types,
            )
            .ok()
            .flatten();
        let arguments = resolved_arguments
            .as_ref()
            .or(static_arguments.as_ref())
            .map(|arguments| arguments.as_slice())
            .unwrap_or(&[]);

        let unknown_ty_id = types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            },
            source_id,
        );

        let yield_ty_id = arguments
            .first()
            .map(|argument| self.convert_static_argument_type(argument, source_id, types))
            .unwrap_or(unknown_ty_id);
        let return_ty_id = arguments
            .get(1)
            .map(|argument| self.convert_static_argument_type(argument, source_id, types))
            .unwrap_or(unknown_ty_id);
        let next_ty_id = arguments
            .get(2)
            .map(|argument| self.convert_static_argument_type(argument, source_id, types))
            .unwrap_or(unknown_ty_id);

        Some((yield_ty_id, return_ty_id, next_ty_id))
    }

    /// Resolve the awaited type for a value.
    pub(super) fn unwrap_awaited_type(
        &self,
        module: &Module,
        symbols: &SymbolTable,
        profile: ProfileId,
        type_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        let mut visited = Vec::new();
        self.unwrap_awaited_type_inner(module, symbols, profile, type_id, types, &mut visited)
    }

    /// Resolve the awaited type for a value with cycle detection.
    fn unwrap_awaited_type_inner(
        &self,
        module: &Module,
        symbols: &SymbolTable,
        profile: ProfileId,
        type_id: LocalTypeId,
        types: &mut TypeTable,
        visited: &mut Vec<LocalTypeId>,
    ) -> LocalTypeId {
        // avoid infinite recursion in cyclic types
        if visited.contains(&type_id) {
            return type_id;
        }
        visited.push(type_id);

        // expand alias references so await sees concrete promise targets
        let mut expanded_id = type_id;
        let mut visited_aliases = HashSet::new();
        loop {
            let Type::Reference {
                symbol,
                static_arguments,
            } = types.get_type(expanded_id).clone()
            else {
                break;
            };
            let symbol = self.normalize_reference_symbol_id(module, profile, symbol);
            if symbol.ty() != SymbolType::TypeAlias {
                break;
            }
            if !visited_aliases.insert(symbol) {
                break;
            }

            let arguments = static_arguments.as_deref().unwrap_or(&[]);
            let mut normalize_visited = Vec::new();
            let Some(next_id) = self.normalize_type_alias_reference_with_arguments(
                module,
                profile,
                types.get_type_source(expanded_id),
                symbol,
                arguments,
                symbols,
                types,
                NormalizationMode::Assign,
                RelationMode::TYPE_OPS,
                &mut normalize_visited,
            ) else {
                break;
            };
            if next_id == expanded_id {
                break;
            }
            expanded_id = next_id;
        }
        let type_id = expanded_id;

        // materialize static arguments before awaiting promise targets
        let tree = module.dir(profile).tree.read();
        let mut materialize_cache = TypeRewriteCache::new();
        let type_id = self.materialize_static_arguments_in_type(
            module,
            profile,
            type_id,
            &tree,
            symbols,
            types,
            &mut materialize_cache,
        );

        // normalize after alias expansion to avoid cached alias results
        let normalized_id = self.normalize_type_with_relation(
            module,
            profile,
            type_id,
            symbols,
            types,
            NormalizationMode::Assign,
            RelationMode::TYPE_OPS,
        );
        let type_id = if normalized_id != type_id {
            normalized_id
        } else {
            type_id
        };
        if !visited.contains(&type_id) {
            visited.push(type_id);
        }

        // keep any/unknown as-is
        if self.type_is_any_or_unknown(type_id, types) {
            return type_id;
        }

        // distribute await across unions
        if let Type::Union { elements } = types.get_type(type_id).clone() {
            let mut awaited_elements = Vec::new();

            // evaluate each union element independently
            for element_id in elements {
                let awaited_id = self.unwrap_awaited_type_inner(
                    module, symbols, profile, element_id, types, visited,
                );
                awaited_elements.push(awaited_id);
            }

            return self.union_type_from_list(awaited_elements, type_id, types);
        }

        // unwrap promises when possible
        if let Some(inner_id) = self.unwrap_promise_type(module, symbols, profile, type_id, types) {
            return self
                .unwrap_awaited_type_inner(module, symbols, profile, inner_id, types, visited);
        }

        type_id
    }

    /// Check whether a type is object like for typeof guards.
    pub(super) fn type_is_object_like(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> bool {
        // anchor apparent type resolution to the current type source
        let source_id = types.get_type_source(type_id);

        // match shapes that would produce typeof object
        let ty = types.get_type(type_id).clone();
        match ty {
            Type::TypeLiteral {
                value: TypeLiteral::Null,
            } => true,
            Type::Object { .. }
            | Type::Array { .. }
            | Type::ArraySized { .. }
            | Type::Tuple { .. }
            | Type::Value { .. } => true,
            Type::Reference { symbol, .. } => {
                // prefer apparent instance types when available
                if let Some(instance_id) =
                    self.apparent_instance_type(module, profile, source_id, symbol, symbols, types)
                {
                    return self.type_is_object_like(module, profile, instance_id, symbols, types);
                }

                matches!(
                    symbol.local_id.ty,
                    SymbolType::Class
                        | SymbolType::Struct
                        | SymbolType::Interface
                        | SymbolType::Extension
                        | SymbolType::Enum
                )
            }
            Type::Intersection { elements } => elements.iter().any(|element_id| {
                self.type_is_object_like(module, profile, *element_id, symbols, types)
            }),
            _ => false,
        }
    }

    /// Check whether a type is function like for typeof guards.
    pub(super) fn type_is_function_like(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> bool {
        // anchor apparent type resolution to the current type source
        let source_id = types.get_type_source(type_id);

        // match callable shapes for typeof function
        let ty = types.get_type(type_id).clone();
        match ty {
            Type::Function { .. } => true,
            Type::Object {
                call_signatures,
                construct_signatures,
                ..
            } => !call_signatures.is_empty() || !construct_signatures.is_empty(),
            Type::Reference { symbol, .. } => {
                // prefer apparent instance types when available
                if let Some(instance_id) =
                    self.apparent_instance_type(module, profile, source_id, symbol, symbols, types)
                {
                    return self.type_is_function_like(
                        module,
                        profile,
                        instance_id,
                        symbols,
                        types,
                    );
                }

                symbol.local_id.ty == SymbolType::Function
            }
            Type::Intersection { elements } => elements.iter().any(|element_id| {
                self.type_is_function_like(module, profile, *element_id, symbols, types)
            }),
            _ => false,
        }
    }

    /// Return the enum backing type for a reference type.
    fn enum_backing_type_for_type(
        &self,
        module: &Module,
        profile: ProfileId,
        ty: &Type,
        types: &mut TypeTable,
    ) -> Option<EnumBackingType> {
        match ty {
            // read backing types directly from enum references
            Type::Reference { symbol, .. } => {
                self.enum_backing_type_for_symbol_best_effort(module, profile, *symbol, types)
            }
            // unwrap value containers to reach enum references
            Type::Value { value } => {
                let inner_ty = types.get_type(*value).clone();
                self.enum_backing_type_for_type(module, profile, &inner_ty, types)
            }
            // scan intersections for a matching enum reference
            Type::Intersection { elements } => elements.iter().find_map(|element_id| {
                let element_ty = types.get_type(*element_id).clone();
                self.enum_backing_type_for_type(module, profile, &element_ty, types)
            }),
            _ => None,
        }
    }

    /// Return true when the target type matches the enum backing type.
    fn enum_backing_type_matches(&self, backing: EnumBackingType, target: &Type) -> bool {
        // match integer or string targets
        match backing {
            EnumBackingType::Int(_) => matches!(
                target,
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::Int(_))
                } | Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(_))
                }
            ),
            EnumBackingType::String => matches!(
                target,
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::String)
                } | Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_))
                }
            ),
        }
    }

    /// Return true when an enum cast targets its backing type.
    fn is_enum_backing_cast(
        &self,
        module: &Module,
        profile: ProfileId,
        left_ty: &Type,
        right_ty: &Type,
        types: &mut TypeTable,
    ) -> bool {
        // read backing types
        let left_backing = self.enum_backing_type_for_type(module, profile, left_ty, types);
        let right_backing = self.enum_backing_type_for_type(module, profile, right_ty, types);

        // compare backing types against the other side
        match (left_backing, right_backing) {
            (Some(backing), None) => self.enum_backing_type_matches(backing, right_ty),
            (None, Some(backing)) => self.enum_backing_type_matches(backing, left_ty),
            _ => false,
        }
    }

    /// Return true when an explicit cast targets a record like map alias.
    fn allow_record_like_cast(
        &self,
        profile: ProfileId,
        left_ty_id: LocalTypeId,
        right_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        // require an object source type
        let left_ty_id = self.unwrap_type_value(left_ty_id, types);
        if !matches!(types.get_type(left_ty_id), Type::Object { .. }) {
            return false;
        }

        // locate the map and record symbols
        let map_symbol = self.get_well_known_type_symbol(profile, WellKnownSymbol::Map);
        let record_symbol = self.get_well_known_type_symbol(profile, WellKnownSymbol::Record);
        let Some(map_symbol) = map_symbol else {
            return false;
        };

        // walk aliases to find map and record references
        let mut current = self.unwrap_type_value(right_ty_id, types);
        let mut visited = HashSet::new();
        loop {
            if !visited.insert(current) {
                return false;
            }

            match types.get_type(current) {
                // accept direct map and record references
                Type::Reference { symbol, .. } => {
                    if *symbol == map_symbol
                        || record_symbol.is_some_and(|record_symbol| record_symbol == *symbol)
                    {
                        return true;
                    }

                    // follow alias targets when available
                    if let Some(alias_target) = types.get_alias_target_type_id(*symbol) {
                        current = alias_target;
                        continue;
                    }

                    return false;
                }
                // unwrap value wrapper types
                Type::Value { value } => {
                    current = *value;
                }
                _ => return false,
            }
        }
    }

    /// Check whether a type is null or undefined.
    pub(super) fn is_nullish_type(&self, ty: &Type) -> bool {
        matches!(
            ty,
            Type::TypeLiteral {
                value: TypeLiteral::Null | TypeLiteral::Undefined,
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::TestProgram;
    use destack_dir::{Expression, StaticKey, SymbolSpace};

    /// Detect export inference cycles from module dependencies.
    #[test]
    fn test_export_inference_cycle_detected_from_imports() {
        let test = TestProgram::memory_sequential();
        let a_module_id = test.add_module(
            "a.ts",
            r#"
import { y } from "./b";

export const x = y;
"#,
        );
        let b_module_id = test.add_module(
            "b.ts",
            r#"
import { x } from "./a";

export const y = x;
"#,
        );

        test.resolve_module(a_module_id);
        test.resolve_module(b_module_id);
        test.compile();

        let profile = test.default_profile_id(a_module_id);
        let module = test.program.modules.get(a_module_id);
        let module = module.read();
        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();
        let exported_symbols = dir.exported_symbols.read();

        let x_name = test.program.strings.intern("x");
        let x_key = StaticKey::Name(x_name);
        assert!(
            exported_symbols.contains_key(&(SymbolSpace::Value, x_key)),
            "expected export table to include x as a value export"
        );

        let declarator_id = crate::expect_let_declarator_by_name(&dir.roots, &tree, x_name);
        let declarator = tree.get(declarator_id);
        let value_id = declarator.value.expect("expected initializer for export x");
        let Expression::ModuleReference { target_symbol, .. } = tree.get(value_id) else {
            panic!("expected module reference for export initializer");
        };
        let symbol_entry = symbols.get_symbol(target_symbol.local_id);
        assert!(
            symbol_entry.target_symbol.is_some(),
            "expected import binding to resolve to a target symbol"
        );

        let y_symbol = test
            .resolve_to_symbol("b.ts", "y")
            .expect("expected y symbol in b.ts");
        let has_declared_type = test
            .compiler
            .remote_symbol_has_declared_value_type(profile, y_symbol);
        assert!(
            !has_declared_type,
            "expected unannotated export y to have no declared value type"
        );

        let has_cycle = test
            .compiler
            .export_inference_has_cycle(a_module_id, profile, b_module_id)
            .expect("cycle detection should not error");

        assert!(
            has_cycle,
            "expected export inference cycle for a.ts <-> b.ts"
        );
    }
}
