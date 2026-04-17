use std::collections::HashMap;

use crate::analyze::StaticMemberSymbolKind;
use crate::analyze::common::{
    AnalyzeIndex, CanonicalSymbolMode, MaterializationMode, ModuleSymbolView,
    REWRITER_TAG_STATIC_ARGUMENT, TypeContext, TypeRewriteCache, TypeWalkContext,
    rewrite_type_with_cache,
};
use crate::timing::tags;
use crate::{AnalyzeResult, Compiler, CompilerContext};
use destack_dir::{
    Argument, Expression, GenericParameterKind, GlobalSymbolId, LocalNodeIdAny, LocalTypeId,
    MappedTypeParameter, NodeTree, StaticArgument, StaticExpression, StaticKey, StaticProperty,
    SymbolTable, SymbolType, Type, TypeElement, TypeField, TypeRewriter, TypeRewriterOptions,
    TypeTable, rewrite_type,
};
use destack_workspace::{Module, ProfileId};

/// Rewrite static arguments inside types for substitution.
struct StaticArgumentMaterializer<'a> {
    /// The compiler instance.
    compiler: &'a Compiler,
    /// The pinned compiler context.
    compiler_context: &'a CompilerContext<'a>,
    /// The module that owns the arguments.
    argument_module: &'a Module,
    /// The active profile.
    profile: ProfileId,
    /// The tree that owns the arguments.
    argument_tree: &'a NodeTree,
    /// The symbols that own the arguments.
    argument_symbols: &'a SymbolTable,
    /// The materialization mode.
    mode: MaterializationMode,
    /// The cached materializations.
    cache: TypeRewriteCache,
    /// The cache key for rewrites.
    cache_key: u64,
    /// The rewriter options.
    rewrite_options: TypeRewriterOptions,
}

impl<'a> StaticArgumentMaterializer<'a> {
    /// Create a materializer for static arguments.
    fn new(
        compiler: &'a Compiler,
        compiler_context: &'a CompilerContext<'a>,
        argument_module: &'a Module,
        profile: ProfileId,
        argument_tree: &'a NodeTree,
        argument_symbols: &'a SymbolTable,
        mode: MaterializationMode,
        cache: TypeRewriteCache,
    ) -> Self {
        // derive rewrite options from the materialization mode
        let walk_context = TypeWalkContext::for_materialization(mode)
            .with_rewriter_tag(REWRITER_TAG_STATIC_ARGUMENT);
        let context_key = generic_argument_context_key(argument_module, profile);
        let walk_context = walk_context.with_context_key(context_key);
        let rewrite_options = walk_context.rewriter_options();
        let cache_key = rewrite_options.cache_key();

        // seed the materializer state
        Self {
            compiler,
            compiler_context,
            argument_module,
            profile,
            argument_tree,
            argument_symbols,
            mode,
            cache,
            cache_key,
            rewrite_options,
        }
    }

    /// Return the internal cache.
    fn into_cache(self) -> TypeRewriteCache {
        self.cache
    }

    /// Rewrite a list of static arguments.
    fn rewrite_static_arguments(
        &mut self,
        types: &mut TypeTable,
        arguments: &[StaticArgument],
    ) -> (Vec<StaticArgument>, bool) {
        // rewrite each argument and track changes
        let mut changed = false;
        let mut mapped = Vec::with_capacity(arguments.len());
        for argument in arguments {
            let mapped_argument = self.rewrite_static_argument(types, argument);
            if mapped_argument != *argument {
                changed = true;
            }
            mapped.push(mapped_argument);
        }
        (mapped, changed)
    }
}

/// Return a cache key for static argument materialization.
fn generic_argument_context_key(argument_module: &Module, profile: ProfileId) -> u64 {
    // base module key
    // NOTE #Architecture: this key currently omits substitution context
    let module_id = argument_module.id;
    let module_key = module_id.package_id.raw() ^ ((module_id.local_id as u64) << 32);

    // profile key mix
    let profile_key = profile.raw().rotate_left(17);

    module_key ^ profile_key
}

impl TypeRewriter for StaticArgumentMaterializer<'_> {
    fn options(&self) -> &TypeRewriterOptions {
        &self.rewrite_options
    }

    fn rewrite_type_id(&mut self, types: &mut TypeTable, id: LocalTypeId) -> LocalTypeId {
        // return cached rewrites when available
        if let Some(mapped) = self.cache.get(&(self.cache_key, id)).copied() {
            return mapped;
        }

        // evaluate unevaluated types before rewriting
        if matches!(types.get_type(id), Type::Unevaluated(_)) {
            let options = self
                .compiler_context
                .analyze_context_options_for_module(self.argument_module.id);
            let mut ctx = TypeContext::new(
                self.compiler_context,
                self.argument_module,
                self.profile,
                &options,
                self.argument_tree,
                self.argument_symbols,
                types,
                AnalyzeIndex::default(),
            );
            let _ = self.compiler.resolve_declared_type(&mut ctx.reborrow(), id);
        }

        // stop when evaluation still yields an unevaluated type
        if matches!(types.get_type(id), Type::Unevaluated(_)) {
            self.cache.insert((self.cache_key, id), id);
            return id;
        }

        // rewrite using the cached walker
        let mut cache = std::mem::take(&mut self.cache);
        let mapped = rewrite_type_with_cache(self, types, &mut cache, self.cache_key, id);
        self.cache = cache;
        mapped
    }

    fn rewrite_type(&mut self, types: &mut TypeTable, id: LocalTypeId, ty: &Type) -> LocalTypeId {
        // allow non surface modes to rewrite immediately
        match self.mode {
            MaterializationMode::Surface => {}
            MaterializationMode::Shape | MaterializationMode::Validation => {
                return rewrite_type(self, types, id, ty);
            }
        }

        // only materialize references with static arguments
        let Type::Reference {
            symbol,
            generic_arguments,
        } = ty
        else {
            return rewrite_type(self, types, id, ty);
        };

        // normalize to the type space symbol for the reference
        let symbol = self.compiler.normalize_reference_symbol_id(
            ModuleSymbolView::new(
                self.compiler_context,
                self.argument_module,
                self.profile,
                self.argument_symbols,
            ),
            *symbol,
        );

        // skip when no static arguments exist
        let Some(generic_arguments) = generic_arguments.as_ref() else {
            return id;
        };

        // resolve static arguments in the reference owner module
        let source_id = types.get_type_source(id);
        let resolved_arguments = if symbol.module_id == self.argument_module.id {
            let options = self
                .compiler_context
                .analyze_context_options_for_module(self.argument_module.id);
            let mut ctx = TypeContext::new(
                self.compiler_context,
                self.argument_module,
                self.profile,
                &options,
                self.argument_tree,
                self.argument_symbols,
                types,
                AnalyzeIndex::default(),
            );
            self.compiler.materialize_static_arguments_for_reference(
                &mut ctx.reborrow(),
                symbol,
                source_id,
                generic_arguments,
            )
        } else {
            let reference_module = self.compiler_context.module(symbol.module_id);
            let reference_module = reference_module.as_ref();
            let reference_snapshot = self.compiler.require_artifact_dir_resolved(
                self.compiler_context.revision(),
                symbol.module_id,
                self.profile,
            );
            let Ok(reference_snapshot) = reference_snapshot else {
                return rewrite_type(self, types, id, ty);
            };
            let reference_options = self
                .compiler_context
                .analyze_context_options_for_module(reference_module.id);
            let mut ctx = TypeContext::new(
                self.compiler_context,
                reference_module,
                self.profile,
                &reference_options,
                &reference_snapshot.tree,
                &reference_snapshot.symbols,
                types,
                AnalyzeIndex::default(),
            );
            self.compiler.materialize_static_arguments_for_reference(
                &mut ctx.reborrow(),
                symbol,
                source_id,
                generic_arguments,
            )
        };

        // rewrite nested static arguments
        let (mapped_arguments, nested_changed) =
            self.rewrite_static_arguments(types, &resolved_arguments);
        let changed = nested_changed || resolved_arguments != *generic_arguments;

        // return the original type when nothing changed
        if !changed {
            return id;
        }

        // return a rewritten reference type
        types.insert_type_from_type(
            Type::Reference {
                symbol,
                generic_arguments: Some(mapped_arguments),
            },
            id,
        )
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Substitute static parameter references in a type.
    pub(crate) fn substitute_static_parameters(
        &self,
        ty_id: LocalTypeId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        types: &mut TypeTable,
        cache: &mut HashMap<LocalTypeId, LocalTypeId>,
    ) -> LocalTypeId {
        let _timing = self.timing_scope(tags::ANALYZE_INFER_TYPE_SUBSTITUTE);

        if let Some(mapped) = cache.get(&ty_id).copied() {
            return mapped;
        }
        cache.insert(ty_id, ty_id);

        let ty = types.get_type(ty_id).clone();
        let mapped = match ty {
            Type::Reference {
                symbol,
                generic_arguments,
            } => {
                if let Some(mapped) = self.substitution_type_id_for_symbol(symbol, substitutions) {
                    mapped
                } else if let Some(generic_arguments) = generic_arguments {
                    let mut changed = false;
                    let mapped_arguments = generic_arguments
                        .iter()
                        .map(|argument| {
                            let mapped = self.substitute_static_argument(
                                argument,
                                substitutions,
                                types,
                                cache,
                            );
                            if mapped != *argument {
                                changed = true;
                            }
                            mapped
                        })
                        .collect::<Vec<_>>();

                    if changed {
                        types.insert_type_from_type(
                            Type::Reference {
                                symbol,
                                generic_arguments: Some(mapped_arguments),
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
            Type::This => ty_id,
            Type::Value { value } => {
                let mapped_value =
                    self.substitute_static_parameters(value, substitutions, types, cache);
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
            Type::Readonly { target_type: right } => {
                let mapped_right =
                    self.substitute_static_parameters(right, substitutions, types, cache);
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Readonly {
                            target_type: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::KeyOf { target_type: right } => {
                let mapped_right =
                    self.substitute_static_parameters(right, substitutions, types, cache);
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::KeyOf {
                            target_type: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::Must { target_type: right } => {
                let mapped_right =
                    self.substitute_static_parameters(right, substitutions, types, cache);
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Must {
                            target_type: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::AsComptime { target_type: right } => {
                let mapped_right =
                    self.substitute_static_parameters(right, substitutions, types, cache);
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::AsComptime {
                            target_type: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::Not { target_type: right } => {
                let mapped_right =
                    self.substitute_static_parameters(right, substitutions, types, cache);
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Not {
                            target_type: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::In { left, right } => {
                let mapped_left =
                    self.substitute_static_parameters(left, substitutions, types, cache);
                let mapped_right =
                    self.substitute_static_parameters(right, substitutions, types, cache);
                if mapped_left == left && mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::In {
                            left: mapped_left,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::Extends { left, right } => {
                let mapped_left =
                    self.substitute_static_parameters(left, substitutions, types, cache);
                let mapped_right =
                    self.substitute_static_parameters(right, substitutions, types, cache);
                if mapped_left == left && mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Extends {
                            left: mapped_left,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::Implements { left, right } => {
                let mapped_left =
                    self.substitute_static_parameters(left, substitutions, types, cache);
                let mapped_right =
                    self.substitute_static_parameters(right, substitutions, types, cache);
                if mapped_left == left && mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Implements {
                            left: mapped_left,
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
                let distributive_union = match distributive_symbol {
                    Some(symbol) => match types.get_type(left).clone() {
                        Type::Reference {
                            symbol: reference_symbol,
                            generic_arguments: None,
                        } if reference_symbol == symbol => {
                            if let Some(substitution) = substitutions.get(&symbol) {
                                let mut union_source = *substitution;
                                if let Type::Reference {
                                    symbol: union_symbol,
                                    generic_arguments: None,
                                } = types.get_type(union_source)
                                    && union_symbol.ty() == SymbolType::TypeAlias
                                    && let Some(instance_id) =
                                        types.get_instance_type_id(*union_symbol)
                                {
                                    union_source = instance_id;
                                }

                                match types.get_type(union_source).clone() {
                                    Type::Union { elements } => Some((symbol, elements)),
                                    _ => None,
                                }
                            } else {
                                None
                            }
                        }
                        _ => None,
                    },
                    None => None,
                };

                if let Some((symbol, elements)) = distributive_union {
                    let mut branches = Vec::with_capacity(elements.len());
                    for element in elements {
                        let mut branch_substitutions = substitutions.clone();
                        branch_substitutions.insert(symbol, element);
                        let mut branch_cache = HashMap::new();
                        let mapped_left = self.substitute_static_parameters(
                            left,
                            &branch_substitutions,
                            types,
                            &mut branch_cache,
                        );
                        let mapped_right = self.substitute_static_parameters(
                            right,
                            &branch_substitutions,
                            types,
                            &mut branch_cache,
                        );
                        let mapped_then = self.substitute_static_parameters(
                            then_type,
                            &branch_substitutions,
                            types,
                            &mut branch_cache,
                        );
                        let mapped_else = self.substitute_static_parameters(
                            else_type,
                            &branch_substitutions,
                            types,
                            &mut branch_cache,
                        );
                        let branch_id = types.insert_type_from_type(
                            Type::Conditional {
                                distributive_symbol: None,
                                left: mapped_left,
                                right: mapped_right,
                                then_type: mapped_then,
                                else_type: mapped_else,
                            },
                            ty_id,
                        );
                        branches.push(branch_id);
                    }

                    types.insert_type_from_type(Type::Union { elements: branches }, ty_id)
                } else {
                    let mapped_left =
                        self.substitute_static_parameters(left, substitutions, types, cache);
                    let mut branch_substitutions = substitutions.clone();
                    if let Some(symbol) = distributive_symbol {
                        branch_substitutions.remove(&symbol);
                    }
                    let mapped_right = self.substitute_static_parameters(
                        right,
                        &branch_substitutions,
                        types,
                        cache,
                    );
                    let mapped_then = self.substitute_static_parameters(
                        then_type,
                        &branch_substitutions,
                        types,
                        cache,
                    );
                    let mapped_else = self.substitute_static_parameters(
                        else_type,
                        &branch_substitutions,
                        types,
                        cache,
                    );
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
            }
            Type::Mapped {
                parameter,
                modifiers,
                value,
            } => {
                let mapped_constraint = self.substitute_static_parameters(
                    parameter.constraint,
                    substitutions,
                    types,
                    cache,
                );
                let mapped_key_remap = parameter.key_remap.map(|key_remap| {
                    self.substitute_static_parameters(key_remap, substitutions, types, cache)
                });
                let mapped_value =
                    self.substitute_static_parameters(value, substitutions, types, cache);
                if mapped_constraint == parameter.constraint
                    && mapped_key_remap == parameter.key_remap
                    && mapped_value == value
                {
                    ty_id
                } else {
                    let parameter = MappedTypeParameter {
                        name: parameter.name,
                        symbol: parameter.symbol.into(),
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
                let mapped_left =
                    self.substitute_static_parameters(left, substitutions, types, cache);
                let mapped_index =
                    self.substitute_static_parameters(index, substitutions, types, cache);
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
                        let mapped =
                            self.substitute_static_parameters(*span, substitutions, types, cache);
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
            Type::Import {
                target,
                qualifier,
                generic_arguments,
            } => {
                let Some(generic_arguments) = generic_arguments else {
                    return ty_id;
                };

                let mut changed = false;
                let mapped_arguments = generic_arguments
                    .iter()
                    .map(|argument| {
                        let mapped =
                            self.substitute_static_argument(argument, substitutions, types, cache);
                        if mapped != *argument {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();

                if changed {
                    types.insert_type_from_type(
                        Type::Import {
                            target,
                            qualifier,
                            generic_arguments: Some(mapped_arguments),
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::Infer { name, constraint } => {
                let mapped_constraint = constraint.map(|constraint| {
                    self.substitute_static_parameters(constraint, substitutions, types, cache)
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
                let mapped_target = target.map(|target| {
                    self.substitute_static_parameters(target, substitutions, types, cache)
                });
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
                let mapped_right =
                    self.substitute_static_parameters(right, substitutions, types, cache);
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
                let mapped_right =
                    self.substitute_static_parameters(right, substitutions, types, cache);
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
                let mapped_right =
                    self.substitute_static_parameters(right, substitutions, types, cache);
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
                // substitute the array element type
                let mapped_element =
                    self.substitute_static_parameters(element, substitutions, types, cache);
                let mapped_count =
                    self.substitute_static_parameters(count, substitutions, types, cache);

                // reuse the existing type if substitutions were no-ops
                if mapped_element == element && mapped_count == count {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::ArraySized {
                            element: mapped_element,
                            count: mapped_count,
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
                let mapped_element = element.map(|element| {
                    self.substitute_static_parameters(element, substitutions, types, cache)
                });
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
                let mut did_change = false;
                let mapped_elements = elements
                    .into_iter()
                    .map(|element| {
                        let TypeElement {
                            label,
                            ty,
                            is_optional,
                            is_readonly,
                            is_rest,
                        } = element;
                        let mapped_ty =
                            self.substitute_static_parameters(ty, substitutions, types, cache);
                        if mapped_ty != ty {
                            did_change = true;
                        }
                        TypeElement {
                            label,
                            ty: mapped_ty,
                            is_optional,
                            is_readonly,
                            is_rest,
                        }
                    })
                    .collect::<Vec<_>>();
                if did_change {
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
                        let mapped = self.substitute_static_parameters(
                            field.ty,
                            substitutions,
                            types,
                            cache,
                        );
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
                        let mapped = self.substitute_static_parameters(
                            *signature,
                            substitutions,
                            types,
                            cache,
                        );
                        if mapped != *signature {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                let mapped_construct_signatures = construct_signatures
                    .iter()
                    .map(|signature| {
                        let mapped = self.substitute_static_parameters(
                            *signature,
                            substitutions,
                            types,
                            cache,
                        );
                        if mapped != *signature {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                let mapped_index_signatures = index_signatures
                    .iter()
                    .map(|signature| {
                        let mapped_key = self.substitute_static_parameters(
                            signature.key_type,
                            substitutions,
                            types,
                            cache,
                        );
                        let mapped_value = self.substitute_static_parameters(
                            signature.value_type,
                            substitutions,
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
                generic_parameters,
                this_parameter,
                parameters,
                return_type,
            } => {
                let mut changed = false;
                let mapped_this = this_parameter.map(|this_parameter| {
                    let mapped = self.substitute_static_parameters(
                        this_parameter,
                        substitutions,
                        types,
                        cache,
                    );
                    if mapped != this_parameter {
                        changed = true;
                    }
                    mapped
                });
                let mapped_parameters = parameters
                    .iter()
                    .map(|parameter| {
                        let mapped = self.substitute_static_parameters(
                            *parameter,
                            substitutions,
                            types,
                            cache,
                        );
                        if mapped != *parameter {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                let mapped_return = return_type.map(|return_type| {
                    let mapped =
                        self.substitute_static_parameters(return_type, substitutions, types, cache);
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
                            generic_parameters,
                            this_parameter: mapped_this,
                            parameters: mapped_parameters,
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
                        let mapped = self.substitute_static_parameters(
                            *element,
                            substitutions,
                            types,
                            cache,
                        );
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
                        let mapped = self.substitute_static_parameters(
                            *element,
                            substitutions,
                            types,
                            cache,
                        );
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
            Type::TypeLiteral { .. }
            | Type::InferVar { .. }
            | Type::Unevaluated(_)
            | Type::Error => ty_id,
        };

        cache.insert(ty_id, mapped);
        mapped
    }

    /// Look up one substitution for a symbol id, tolerating placeholder symbol kinds.
    fn substitution_type_id_for_symbol(
        &self,
        symbol: GlobalSymbolId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
    ) -> Option<LocalTypeId> {
        substitutions.get(&symbol).copied()
    }

    /// Resolve one projection receiver reference from the projection source expression.
    fn projection_receiver_reference_from_type_source(
        &self,
        ctx: &mut TypeContext<'_>,
        projection_source_id: LocalNodeIdAny,
    ) -> AnalyzeResult<Option<(GlobalSymbolId, Vec<StaticArgument>)>> {
        let Ok(mut projection_expression_id) = projection_source_id.try_into_typed::<Expression>()
        else {
            return Ok(None);
        };
        if !ctx.tree.has_node_id(projection_expression_id.id) {
            return Ok(None);
        }

        if let Expression::Instantiation { left, .. } = ctx.tree.get(projection_expression_id) {
            projection_expression_id = *left;
        }
        let Expression::Member { left, .. } = ctx.tree.get(projection_expression_id) else {
            return Ok(None);
        };

        let receiver_expression_id = self.unwrap_parenthesized_expression(*left, ctx.tree);
        match ctx.tree.get(receiver_expression_id) {
            Expression::Instantiation {
                left,
                generic_arguments,
            } => {
                let receiver_expression_id = self.unwrap_parenthesized_expression(*left, ctx.tree);
                let receiver_symbol = self
                    .reference_symbol_for_expression(ctx.tree_symbol_view(), receiver_expression_id)
                    .or_else(|| ctx.tree.get(receiver_expression_id).target_symbol());
                let Some(receiver_symbol) = receiver_symbol else {
                    return Ok(None);
                };

                let receiver_arguments = self
                    .evaluate_generic_arguments(
                        &mut ctx.reborrow(),
                        Some(generic_arguments.as_slice()),
                    )?
                    .unwrap_or_default();
                Ok(Some((receiver_symbol, receiver_arguments)))
            }
            _ => {
                let receiver_symbol = self
                    .reference_symbol_for_expression(ctx.tree_symbol_view(), receiver_expression_id)
                    .or_else(|| ctx.tree.get(receiver_expression_id).target_symbol());
                Ok(receiver_symbol.map(|receiver_symbol| (receiver_symbol, Vec::new())))
            }
        }
    }

    /// Resolve one projection receiver reference from static-parameter substitutions and owner constraints.
    fn projection_receiver_reference_from_substitutions(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        owner_symbol: GlobalSymbolId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
    ) -> Option<(GlobalSymbolId, Vec<StaticArgument>)> {
        let mut candidates = Vec::<(GlobalSymbolId, Vec<StaticArgument>)>::new();

        for (parameter_symbol, substitution_type_id) in substitutions {
            if !self.symbol_is_static_parameter(ctx.symbol_type_view(), *parameter_symbol) {
                continue;
            }

            let Some(constraint_type_id) = self.generic_parameter_constraint_type(
                &mut ctx.reborrow(),
                *parameter_symbol,
                source_id,
            ) else {
                continue;
            };
            let constraint_symbol = self
                .unwrap_type_symbol(ctx.types, constraint_type_id)
                .map(|(symbol, _, _)| symbol)
                .or_else(|| match ctx.types.get_type(constraint_type_id) {
                    Type::Intersection { elements } | Type::Union { elements } => {
                        elements.iter().find_map(|element_id| {
                            self.unwrap_type_symbol(ctx.types, *element_id)
                                .map(|(symbol, _, _)| symbol)
                        })
                    }
                    _ => None,
                });
            let Some(constraint_symbol) = constraint_symbol else {
                continue;
            };
            let constraint_symbol = self
                .declaration_symbol_id(ctx.module_symbol_view(), constraint_symbol)
                .unwrap_or(constraint_symbol);
            if constraint_symbol != owner_symbol {
                continue;
            }

            let substitution_type_id = ctx.types.unwrap_value_type_id(*substitution_type_id);
            let Some((receiver_symbol, receiver_arguments, _)) =
                self.unwrap_type_symbol(ctx.types, substitution_type_id)
            else {
                continue;
            };
            let candidate = (receiver_symbol, receiver_arguments.unwrap_or_default());
            if !candidates.contains(&candidate) {
                candidates.push(candidate);
            }
        }

        if candidates.len() == 1 {
            return candidates.pop();
        }

        None
    }

    /// Materialize static arguments inside type references for substitution.
    pub(crate) fn materialize_static_arguments_in_type(
        &self,
        ctx: &mut TypeContext<'_>,
        ty_id: LocalTypeId,
        cache: &mut TypeRewriteCache,
    ) -> LocalTypeId {
        let _timing = self.timing_scope(tags::ANALYZE_INFER_STATIC_MATERIALIZE);

        let local_cache = std::mem::take(cache);
        let mut materializer = StaticArgumentMaterializer::new(
            self,
            ctx.compiler_context,
            ctx.module,
            ctx.profile,
            ctx.tree,
            ctx.symbols,
            MaterializationMode::Surface,
            local_cache,
        );
        let mapped = materializer.rewrite_type_id(ctx.types, ty_id);
        *cache = materializer.into_cache();
        mapped
    }

    /// Instantiate one type with a substitution environment, then normalize projections.
    pub(crate) fn instantiate_type_with_substitutions(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        owner_symbol: Option<GlobalSymbolId>,
        ty_id: LocalTypeId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        materialize_cache: &mut TypeRewriteCache,
        substitute_cache: &mut HashMap<LocalTypeId, LocalTypeId>,
    ) -> LocalTypeId {
        // materialize source-level static arguments before substitution
        let materialized = self.materialize_static_arguments_in_type(
            &mut ctx.reborrow(),
            ty_id,
            materialize_cache,
        );

        // apply static substitutions
        let substituted = if substitutions.is_empty() {
            materialized
        } else {
            self.substitute_static_parameters(
                materialized,
                substitutions,
                ctx.types,
                substitute_cache,
            )
        };

        // normalize substituted static arguments
        let mut normalized = self.materialize_static_arguments_in_type(
            &mut ctx.reborrow(),
            substituted,
            materialize_cache,
        );

        // resolve associated projections after substitution
        if let Some(projected) = self.instantiate_substituted_projection_type(
            &mut ctx.reborrow(),
            normalized,
            substitutions,
        ) {
            normalized = projected;
        }

        // rewrite owner-scoped associated aliases
        if let Some(owner_symbol) = owner_symbol {
            let owner_receiver_arguments = self.owner_projection_receiver_arguments(
                &mut ctx.reborrow(),
                owner_symbol,
                source_id,
                substitutions,
            );
            normalized = self.rewrite_associated_aliases_for_owner(
                &mut ctx.reborrow(),
                source_id,
                owner_symbol,
                Some(owner_symbol),
                &owner_receiver_arguments,
                substitutions,
                normalized,
            );
        }

        // substitute again after owner alias rewrites: alias materialization can expose static parameters
        if !substitutions.is_empty() {
            normalized = self.substitute_static_parameters(
                normalized,
                substitutions,
                ctx.types,
                substitute_cache,
            );
        }

        // rematerialize projections exposed by the second substitution pass
        if let Some(projected) = self.instantiate_substituted_projection_type(
            &mut ctx.reborrow(),
            normalized,
            substitutions,
        ) {
            normalized = projected;
        }

        self.materialize_static_arguments_in_type(
            &mut ctx.reborrow(),
            normalized,
            materialize_cache,
        )
    }

    /// Materialize one substituted associated projection from its source member expression.
    pub(crate) fn instantiate_substituted_projection_type(
        &self,
        ctx: &mut TypeContext<'_>,
        ty_id: LocalTypeId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
    ) -> Option<LocalTypeId> {
        // only projection-like references can be concretized in this pass
        let (projected_symbol, projected_arguments, projection_source_id) =
            self.unwrap_type_symbol(ctx.types, ty_id)?;
        if self
            .query_static_member_symbol_kind_for_symbol(ctx.tree_symbol_view(), projected_symbol)
            .ok()?
            != Some(StaticMemberSymbolKind::AssociatedType)
        {
            return None;
        }
        let projected_symbol = self
            .declaration_symbol_id(ctx.module_symbol_view(), projected_symbol)
            .unwrap_or(projected_symbol);
        let owner_symbol = self
            .query_owner_symbol_for_member_symbol(ctx.module_symbol_view(), projected_symbol)
            .ok()
            .flatten();
        let owner_symbol = owner_symbol.map(|owner_symbol| {
            self.declaration_symbol_id(ctx.module_symbol_view(), owner_symbol)
                .unwrap_or(owner_symbol)
        });
        let projected_member_key = self
            .symbol_name_for_global_in(ctx.module_symbol_view(), projected_symbol)
            .map(StaticKey::Name)?;

        // use projected reference arguments directly
        let source_id = projection_source_id;
        let explicit_member_arguments = projected_arguments.clone();
        let projected_member_type = Type::Reference {
            symbol: projected_symbol,
            generic_arguments: explicit_member_arguments.clone(),
        };

        // first materialize the projected member directly if it already resolves to a concrete alias
        if let Ok(direct_materialized_type) = self.materialize_associated_member_projection(
            &mut ctx.reborrow(),
            source_id,
            projected_symbol,
            None,
            &[],
            explicit_member_arguments.as_deref(),
            projected_member_type.clone(),
        ) && direct_materialized_type != projected_member_type
        {
            return Some(
                ctx.types
                    .insert_type_from_any(direct_materialized_type, source_id),
            );
        }

        // resolve one projection receiver from source syntax and substitutions
        let receiver_from_source = self
            .projection_receiver_reference_from_type_source(
                &mut ctx.reborrow(),
                projection_source_id,
            )
            .ok()
            .flatten();
        let receiver_from_owner = owner_symbol.and_then(|owner_symbol| {
            self.projection_receiver_reference_from_substitutions(
                &mut ctx.reborrow(),
                source_id,
                owner_symbol,
                substitutions,
            )
        });
        let (projection_receiver_symbol, projection_receiver_arguments) =
            receiver_from_source.or(receiver_from_owner)?;
        let receiver_substitution =
            self.substitution_type_id_for_symbol(projection_receiver_symbol, substitutions);
        let (mut receiver_symbol, receiver_arguments) =
            if let Some(receiver_substitution) = receiver_substitution {
                let receiver_substitution = ctx.types.unwrap_value_type_id(receiver_substitution);
                let (receiver_symbol, receiver_arguments, _) =
                    self.unwrap_type_symbol(ctx.types, receiver_substitution)?;
                (receiver_symbol, receiver_arguments.unwrap_or_default())
            } else {
                (projection_receiver_symbol, projection_receiver_arguments)
            };
        let receiver_arguments = if substitutions.is_empty() {
            receiver_arguments
        } else {
            let mut substitution_cache = HashMap::new();
            receiver_arguments
                .iter()
                .map(|argument| {
                    self.substitute_static_argument(
                        argument,
                        substitutions,
                        ctx.types,
                        &mut substitution_cache,
                    )
                })
                .collect()
        };

        receiver_symbol = self.canonical_symbol_id(
            ctx.module_symbol_view(),
            receiver_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        receiver_symbol = self
            .declaration_symbol_id(ctx.module_symbol_view(), receiver_symbol)
            .unwrap_or(receiver_symbol);
        let target_symbol = self
            .resolve_associated_member_symbol_for_receiver(
                &mut ctx.reborrow(),
                receiver_symbol,
                projected_member_key,
                StaticMemberSymbolKind::AssociatedType,
            )
            .ok()??;
        let member_type = Type::Reference {
            symbol: target_symbol,
            generic_arguments: explicit_member_arguments.clone(),
        };
        let projected_type = self
            .materialize_associated_member_projection(
                &mut ctx.reborrow(),
                source_id,
                target_symbol,
                Some(receiver_symbol),
                &receiver_arguments,
                explicit_member_arguments.as_deref(),
                member_type,
            )
            .ok()?;

        Some(ctx.types.insert_type_from_any(projected_type, source_id))
    }

    pub(crate) fn materialize_static_arguments_for_reference(
        &self,
        ctx: &mut TypeContext<'_>,
        symbol: GlobalSymbolId,
        source_id: LocalNodeIdAny,
        generic_arguments: &[StaticArgument],
    ) -> Vec<StaticArgument> {
        // collect parameter symbols for the reference
        let Some(parameter_symbols) =
            self.collect_static_parameter_symbols(ctx.type_view(), symbol)
        else {
            return generic_arguments.to_vec();
        };
        if parameter_symbols.is_empty() {
            return generic_arguments.to_vec();
        }

        // select a source node for parameter inference
        let source_id = generic_arguments
            .iter()
            .find_map(|argument| match argument {
                StaticArgument::Unevaluated { node }
                    if node.module_id == ctx.module.id
                        && ctx.tree.has_node_id(node.local_id.id) =>
                {
                    Some(node.local_id)
                }
                _ => None,
            })
            .unwrap_or(source_id);

        // map parameter names to their resolved kinds
        let mut parameter_kinds = Vec::with_capacity(parameter_symbols.len());
        let mut parameter_name_kinds = HashMap::new();
        for parameter_symbol in parameter_symbols {
            let parameter =
                self.resolve_static_parameter(&mut ctx.reborrow(), parameter_symbol, source_id);
            let kind = parameter.kind;
            if let Some(name) = parameter.name {
                parameter_name_kinds.insert(name, kind);
            }
            parameter_kinds.push(kind);
        }

        // evaluate arguments based on the referenced parameter kinds
        let mut resolved_arguments = Vec::with_capacity(generic_arguments.len());
        for (index, argument) in generic_arguments.iter().enumerate() {
            let (argument_name, argument_node) = match argument {
                StaticArgument::Evaluated { name, .. } => (*name, None),
                StaticArgument::Unevaluated { node } => {
                    let mut name = None;
                    let mut ctx = ctx.reborrow();
                    let _ = self.with_static_argument_owner(&mut ctx, *node, |ctx, argument_id| {
                        let argument_node = ctx.tree.get(argument_id);
                        name = match argument_node {
                            Argument::Named { name, .. } => Some(name.string()),
                            _ => None,
                        };
                        Ok(())
                    });
                    (name, Some(*node))
                }
            };

            let parameter_kind = argument_name
                .and_then(|name| parameter_name_kinds.get(&name).copied())
                .or_else(|| parameter_kinds.get(index).copied());

            let Some(parameter_kind) = parameter_kind else {
                resolved_arguments.push(argument.clone());
                continue;
            };

            let Some(argument_node) = argument_node else {
                let resolved = if parameter_kind == GenericParameterKind::Value {
                    self.normalize_value_static_argument(argument.clone(), ctx.types)
                } else {
                    argument.clone()
                };
                resolved_arguments.push(resolved);
                continue;
            };

            let mut evaluated = None;
            let mut evaluated_name = argument_name;
            let mut ctx = ctx.reborrow();
            let _ = self.with_static_argument_owner(&mut ctx, argument_node, |ctx, argument_id| {
                let argument = ctx.tree.get(argument_id);
                evaluated_name = match argument {
                    Argument::Named { name, .. } => Some(name.string()),
                    _ => None,
                };
                let expression_id = argument.value();
                evaluated = match parameter_kind {
                    GenericParameterKind::Type => {
                        // preserve static parameter references during materialization
                        if let Some(parameter_symbol) = self
                            .generic_parameter_symbol_for_reference(
                                ctx.type_view(),
                                expression_id,
                            )?
                        {
                            let reference_ty = Type::Reference {
                                symbol: parameter_symbol,
                                generic_arguments: None,
                            };
                            let ty_id = ctx
                                .types
                                .insert_type_from_any(reference_ty, expression_id.into_any());
                            Some(StaticExpression::Type { ty: ty_id })
                        } else if let Expression::Type { value, .. } = ctx.tree.get(expression_id)
                            && let Ok(ty_id) = self.resolve_declared_type_expression(
                                &mut ctx.reborrow(),
                                *value,
                                false,
                                true,
                            )
                            && !matches!(ctx.types.get_type(ty_id), Type::Unevaluated(_))
                        {
                            Some(StaticExpression::Type { ty: ty_id })
                        } else {
                            None
                        }
                    }
                    GenericParameterKind::Value => self
                        .evaluate_static_expression_value(&mut ctx.reborrow(), expression_id, None)
                        .ok()
                        .flatten(),
                };
                Ok(())
            });

            let resolved = if let Some(value) = evaluated {
                StaticArgument::Evaluated {
                    name: evaluated_name,
                    value,
                }
            } else {
                StaticArgument::Unevaluated {
                    node: argument_node,
                }
            };
            let resolved = if parameter_kind == GenericParameterKind::Value {
                self.normalize_value_static_argument(resolved, ctx.types)
            } else {
                resolved
            };
            resolved_arguments.push(resolved);
        }

        resolved_arguments
    }

    /// Substitute static parameters in a static argument.
    pub(crate) fn substitute_static_argument(
        &self,
        argument: &StaticArgument,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        types: &mut TypeTable,
        cache: &mut HashMap<LocalTypeId, LocalTypeId>,
    ) -> StaticArgument {
        match argument {
            StaticArgument::Unevaluated { .. } => argument.clone(),
            StaticArgument::Evaluated { name, value } => {
                let mapped_value =
                    self.substitute_static_expression(value, substitutions, types, cache);
                StaticArgument::Evaluated {
                    name: *name,
                    value: mapped_value,
                }
            }
        }
    }

    /// Substitute static parameters in a static expression.
    pub(crate) fn substitute_static_expression(
        &self,
        expression: &StaticExpression,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        types: &mut TypeTable,
        cache: &mut HashMap<LocalTypeId, LocalTypeId>,
    ) -> StaticExpression {
        match expression {
            StaticExpression::Unevaluated { .. } => expression.clone(),
            StaticExpression::ScalarLiteral { .. } => expression.clone(),
            StaticExpression::TypeLiteral { .. } => expression.clone(),
            StaticExpression::Type { ty } => StaticExpression::Type {
                ty: self.substitute_static_parameters(*ty, substitutions, types, cache),
            },
            StaticExpression::Declaration {
                declaration,
                generic_arguments,
            } => {
                let mapped_arguments = generic_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.substitute_static_argument(argument, substitutions, types, cache)
                        })
                        .collect::<Vec<_>>()
                });
                StaticExpression::Declaration {
                    declaration: *declaration,
                    generic_arguments: mapped_arguments,
                }
            }
            StaticExpression::ArrayExpression { elements } => {
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        self.substitute_static_expression(element, substitutions, types, cache)
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
                        self.substitute_static_expression(element, substitutions, types, cache)
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
                        self.substitute_static_property(property, substitutions, types, cache)
                    })
                    .collect::<Vec<_>>();
                StaticExpression::ObjectExpression {
                    properties: mapped_properties,
                }
            }
        }
    }

    /// Substitute static parameters in a static property.
    pub(crate) fn substitute_static_property(
        &self,
        property: &StaticProperty,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        types: &mut TypeTable,
        cache: &mut HashMap<LocalTypeId, LocalTypeId>,
    ) -> StaticProperty {
        match property {
            StaticProperty::Unevaluated { .. } => property.clone(),
            StaticProperty::Field { key, value, symbol } => {
                let mapped_value =
                    self.substitute_static_expression(value, substitutions, types, cache);
                StaticProperty::Field {
                    key: *key,
                    value: mapped_value,
                    symbol: *symbol,
                }
            }
            StaticProperty::Method {
                key,
                signature,
                body,
                symbol,
            } => {
                let mapped_body =
                    self.substitute_static_expression(body, substitutions, types, cache);
                StaticProperty::Method {
                    key: *key,
                    signature: signature.clone(),
                    body: mapped_body,
                    symbol: *symbol,
                }
            }
            StaticProperty::Spread { value, symbol } => {
                let mapped_value =
                    self.substitute_static_expression(value, substitutions, types, cache);
                StaticProperty::Spread {
                    value: mapped_value,
                    symbol: *symbol,
                }
            }
        }
    }
}
