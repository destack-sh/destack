use std::collections::{HashMap, HashSet};

use destack_dir::{
    GlobalSymbolId, LocalNodeIdAny, LocalTypeId, NodeType, NormalizationMode, PrimitiveType,
    ScalarLiteral, StaticArgument, StaticExpression, StaticProperty, StringId, SymbolKind,
    SymbolSpace, SymbolTable, SymbolType, Type, TypeElement, TypeField, TypeIndexSignature,
    TypeLiteral, TypeMappedParameter, TypeTable,
};
use destack_workspace::{Module, ProfileId};

use crate::Compiler;

/// Substitutions captured from conditional infer patterns.
#[derive(Debug, Default, Clone)]
pub(crate) struct InferSubstitutions {
    /// Inferred bindings keyed by infer name.
    by_name: HashMap<StringId, LocalTypeId>,
}

/// Merge mode for inferred bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InferMergeMode {
    /// Union inferred candidates (covariant positions).
    Union,
    /// Intersect inferred candidates (contravariant positions).
    Intersection,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Check whether a symbol is a static parameter.
    pub(crate) fn symbol_is_static_parameter(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> bool {
        // honor cached constraints for mapped parameters
        if types.get_static_parameter_constraint_type(symbol).is_some() {
            return true;
        }

        // rely on the declared parameter metadata
        if symbol.module_id == module.id {
            let symbol = symbols.get_symbol(symbol.local_id);
            if symbol.is_static_parameter() {
                return true;
            }
            return symbol
                .primary_declaration
                .is_some_and(|declaration| declaration.local_id.ty == NodeType::Parameter);
        }

        let remote_module = self.program.modules.get(symbol.module_id);
        let remote_module = remote_module.read();
        let remote_symbols = remote_module.dir(profile).symbols.read();
        let symbol = remote_symbols.get_symbol(symbol.local_id);
        if symbol.is_static_parameter() {
            return true;
        }
        symbol
            .primary_declaration
            .is_some_and(|declaration| declaration.local_id.ty == NodeType::Parameter)
    }

    /// Return the distributive static parameter symbol for a conditional left side.
    pub(crate) fn conditional_left_distributive_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> Option<GlobalSymbolId> {
        // only naked static parameters distribute
        match types.get_type(type_id) {
            Type::Reference {
                symbol,
                static_arguments,
            } => {
                if static_arguments.is_none()
                    && self.symbol_is_static_parameter(module, profile, *symbol, symbols, types)
                {
                    Some(*symbol)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Check whether a type contains a static parameter reference.
    pub(crate) fn type_contains_static_parameters(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        // avoid recursion cycles
        if !visited.insert(type_id) {
            return false;
        }

        // inspect the type shape
        let ty = types.get_type(type_id);

        // walk nested types for static parameters
        match ty {
            Type::Reference {
                symbol,
                static_arguments,
            } => {
                if self.symbol_is_static_parameter(module, profile, *symbol, symbols, types) {
                    return true;
                }
                static_arguments.as_ref().is_some_and(|arguments| {
                    arguments.iter().any(|argument| {
                        self.static_argument_contains_infer(argument, types, visited)
                            || self.static_argument_contains_static_parameters(
                                module, profile, argument, symbols, types, visited,
                            )
                    })
                })
            }
            Type::Import {
                static_arguments, ..
            } => static_arguments.as_ref().is_some_and(|arguments| {
                arguments.iter().any(|argument| {
                    self.static_argument_contains_infer(argument, types, visited)
                        || self.static_argument_contains_static_parameters(
                            module, profile, argument, symbols, types, visited,
                        )
                })
            }),
            Type::Conditional {
                distributive_symbol: _,
                left,
                right,
                then_type,
                else_type,
            } => {
                self.type_contains_static_parameters(
                    module, profile, *left, symbols, types, visited,
                ) || self.type_contains_static_parameters(
                    module, profile, *right, symbols, types, visited,
                ) || self.type_contains_static_parameters(
                    module, profile, *then_type, symbols, types, visited,
                ) || self.type_contains_static_parameters(
                    module, profile, *else_type, symbols, types, visited,
                )
            }
            Type::Mapped {
                parameter,
                modifiers: _,
                value,
            } => {
                self.type_contains_static_parameters(
                    module,
                    profile,
                    parameter.constraint,
                    symbols,
                    types,
                    visited,
                ) || parameter.key_remap.is_some_and(|key_remap| {
                    self.type_contains_static_parameters(
                        module, profile, key_remap, symbols, types, visited,
                    )
                }) || self.type_contains_static_parameters(
                    module, profile, *value, symbols, types, visited,
                )
            }
            Type::Index { left, index } => {
                self.type_contains_static_parameters(
                    module, profile, *left, symbols, types, visited,
                ) || self.type_contains_static_parameters(
                    module, profile, *index, symbols, types, visited,
                )
            }
            Type::TemplateLiteral { spans, .. } => spans.iter().any(|span| {
                self.type_contains_static_parameters(
                    module, profile, *span, symbols, types, visited,
                )
            }),
            Type::Array { element, .. } => element.is_some_and(|element| {
                self.type_contains_static_parameters(
                    module, profile, element, symbols, types, visited,
                )
            }),
            Type::ArraySized { element, .. } => self.type_contains_static_parameters(
                module, profile, *element, symbols, types, visited,
            ),
            Type::Tuple { elements, .. } => elements.iter().any(|element| {
                self.type_contains_static_parameters(
                    module, profile, element.ty, symbols, types, visited,
                )
            }),
            Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                fields.iter().any(|field| {
                    self.type_contains_static_parameters(
                        module, profile, field.ty, symbols, types, visited,
                    )
                }) || call_signatures.iter().any(|signature| {
                    self.type_contains_static_parameters(
                        module, profile, *signature, symbols, types, visited,
                    )
                }) || construct_signatures.iter().any(|signature| {
                    self.type_contains_static_parameters(
                        module, profile, *signature, symbols, types, visited,
                    )
                }) || index_signatures.iter().any(|signature| {
                    self.type_contains_static_parameters(
                        module,
                        profile,
                        signature.key_type,
                        symbols,
                        types,
                        visited,
                    ) || self.type_contains_static_parameters(
                        module,
                        profile,
                        signature.value_type,
                        symbols,
                        types,
                        visited,
                    )
                })
            }
            Type::Function {
                static_parameters,
                this_parameter,
                dynamic_parameters,
                return_type,
                ..
            } => {
                static_parameters.iter().any(|parameter| {
                    self.type_contains_static_parameters(
                        module, profile, *parameter, symbols, types, visited,
                    )
                }) || this_parameter.is_some_and(|parameter| {
                    self.type_contains_static_parameters(
                        module, profile, parameter, symbols, types, visited,
                    )
                }) || dynamic_parameters.iter().any(|parameter| {
                    self.type_contains_static_parameters(
                        module, profile, *parameter, symbols, types, visited,
                    )
                }) || return_type.is_some_and(|return_type| {
                    self.type_contains_static_parameters(
                        module,
                        profile,
                        return_type,
                        symbols,
                        types,
                        visited,
                    )
                })
            }
            Type::Unary { right, .. }
            | Type::ValueOf { right, .. }
            | Type::ReferenceOf { right, .. }
            | Type::PointerOf { right, .. } => self
                .type_contains_static_parameters(module, profile, *right, symbols, types, visited),
            Type::Binary { left, right, .. } => {
                self.type_contains_static_parameters(
                    module, profile, *left, symbols, types, visited,
                ) || self.type_contains_static_parameters(
                    module, profile, *right, symbols, types, visited,
                )
            }
            Type::Predicate { target, .. } => target.is_some_and(|target| {
                self.type_contains_static_parameters(
                    module, profile, target, symbols, types, visited,
                )
            }),
            Type::Union { elements } | Type::Intersection { elements } => {
                elements.iter().any(|element| {
                    self.type_contains_static_parameters(
                        module, profile, *element, symbols, types, visited,
                    )
                })
            }
            Type::Value { value } => self
                .type_contains_static_parameters(module, profile, *value, symbols, types, visited),
            Type::TypeLiteral { .. }
            | Type::InferVar { .. }
            | Type::Infer { .. }
            | Type::Unevaluated(_)
            | Type::This
            | Type::Error => false,
        }
    }

    /// Check whether a type contains an infer binding.
    pub(crate) fn type_contains_infer(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        // avoid recursion cycles
        if !visited.insert(type_id) {
            return false;
        }

        // inspect the type shape
        let ty = types.get_type(type_id);

        // walk nested types for infer bindings
        match ty {
            Type::Infer { .. } => true,
            Type::Reference {
                static_arguments, ..
            }
            | Type::Import {
                static_arguments, ..
            } => static_arguments.as_ref().is_some_and(|arguments| {
                arguments
                    .iter()
                    .any(|argument| self.static_argument_contains_infer(argument, types, visited))
            }),
            Type::Conditional {
                distributive_symbol: _,
                left,
                right,
                then_type,
                else_type,
            } => {
                self.type_contains_infer(*left, types, visited)
                    || self.type_contains_infer(*right, types, visited)
                    || self.type_contains_infer(*then_type, types, visited)
                    || self.type_contains_infer(*else_type, types, visited)
            }
            Type::Mapped {
                parameter,
                modifiers: _,
                value,
            } => {
                self.type_contains_infer(parameter.constraint, types, visited)
                    || parameter.key_remap.is_some_and(|key_remap| {
                        self.type_contains_infer(key_remap, types, visited)
                    })
                    || self.type_contains_infer(*value, types, visited)
            }
            Type::Index { left, index } => {
                self.type_contains_infer(*left, types, visited)
                    || self.type_contains_infer(*index, types, visited)
            }
            Type::TemplateLiteral { spans, .. } => spans
                .iter()
                .any(|span| self.type_contains_infer(*span, types, visited)),
            Type::Array { element, .. } => {
                element.is_some_and(|element| self.type_contains_infer(element, types, visited))
            }
            Type::ArraySized { element, .. } => self.type_contains_infer(*element, types, visited),
            Type::Tuple { elements, .. } => elements
                .iter()
                .any(|element| self.type_contains_infer(element.ty, types, visited)),
            Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                fields
                    .iter()
                    .any(|field| self.type_contains_infer(field.ty, types, visited))
                    || call_signatures
                        .iter()
                        .any(|signature| self.type_contains_infer(*signature, types, visited))
                    || construct_signatures
                        .iter()
                        .any(|signature| self.type_contains_infer(*signature, types, visited))
                    || index_signatures.iter().any(|signature| {
                        self.type_contains_infer(signature.key_type, types, visited)
                            || self.type_contains_infer(signature.value_type, types, visited)
                    })
            }
            Type::Function {
                static_parameters,
                this_parameter,
                dynamic_parameters,
                return_type,
                ..
            } => {
                static_parameters
                    .iter()
                    .any(|parameter| self.type_contains_infer(*parameter, types, visited))
                    || this_parameter.is_some_and(|parameter| {
                        self.type_contains_infer(parameter, types, visited)
                    })
                    || dynamic_parameters
                        .iter()
                        .any(|parameter| self.type_contains_infer(*parameter, types, visited))
                    || return_type.is_some_and(|return_type| {
                        self.type_contains_infer(return_type, types, visited)
                    })
            }
            Type::Unary { right, .. }
            | Type::ValueOf { right, .. }
            | Type::ReferenceOf { right, .. }
            | Type::PointerOf { right, .. } => self.type_contains_infer(*right, types, visited),
            Type::Binary { left, right, .. } => {
                self.type_contains_infer(*left, types, visited)
                    || self.type_contains_infer(*right, types, visited)
            }
            Type::Predicate { target, .. } => {
                target.is_some_and(|target| self.type_contains_infer(target, types, visited))
            }
            Type::Union { elements } | Type::Intersection { elements } => elements
                .iter()
                .any(|element| self.type_contains_infer(*element, types, visited)),
            Type::Value { value } => self.type_contains_infer(*value, types, visited),
            Type::TypeLiteral { .. }
            | Type::InferVar { .. }
            | Type::Unevaluated(_)
            | Type::This
            | Type::Error => false,
        }
    }

    /// Check whether a static argument contains a static parameter.
    pub(crate) fn static_argument_contains_static_parameters(
        &self,
        module: &Module,
        profile: ProfileId,
        argument: &StaticArgument,
        symbols: &SymbolTable,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        match argument {
            // treat unresolved arguments as static parameter dependent
            StaticArgument::Unevaluated { .. } => true,
            StaticArgument::Evaluated { value, .. } => self
                .static_expression_contains_static_parameters(
                    module, profile, value, symbols, types, visited,
                ),
        }
    }

    /// Check whether a static expression contains a static parameter.
    fn static_expression_contains_static_parameters(
        &self,
        module: &Module,
        profile: ProfileId,
        value: &StaticExpression,
        symbols: &SymbolTable,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        match value {
            StaticExpression::Type { ty } => {
                self.type_contains_static_parameters(module, profile, *ty, symbols, types, visited)
            }
            StaticExpression::Declaration {
                static_arguments, ..
            } => static_arguments.as_ref().is_some_and(|arguments| {
                arguments.iter().any(|argument| {
                    self.static_argument_contains_static_parameters(
                        module, profile, argument, symbols, types, visited,
                    )
                })
            }),
            StaticExpression::ArrayExpression { elements }
            | StaticExpression::TupleExpression { elements } => elements.iter().any(|element| {
                self.static_expression_contains_static_parameters(
                    module, profile, element, symbols, types, visited,
                )
            }),
            StaticExpression::ObjectExpression { properties } => {
                properties.iter().any(|property| match property {
                    StaticProperty::Unevaluated { .. } => false,
                    StaticProperty::Field { value, default, .. } => {
                        self.static_expression_contains_static_parameters(
                            module, profile, value, symbols, types, visited,
                        ) || default.as_ref().is_some_and(|default| {
                            self.static_expression_contains_static_parameters(
                                module, profile, default, symbols, types, visited,
                            )
                        })
                    }
                    StaticProperty::Method { body, .. } => self
                        .static_expression_contains_static_parameters(
                            module, profile, body, symbols, types, visited,
                        ),
                })
            }
            StaticExpression::RangeExpression { start, end, .. } => {
                self.static_expression_contains_static_parameters(
                    module, profile, start, symbols, types, visited,
                ) || self.static_expression_contains_static_parameters(
                    module, profile, end, symbols, types, visited,
                )
            }
            StaticExpression::Unevaluated { .. }
            | StaticExpression::ScalarLiteral { .. }
            | StaticExpression::TypeLiteral { .. } => false,
        }
    }

    /// Check whether a static argument contains an infer binding.
    fn static_argument_contains_infer(
        &self,
        argument: &StaticArgument,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        match argument {
            StaticArgument::Unevaluated { .. } => false,
            StaticArgument::Evaluated { value, .. } => {
                self.static_expression_contains_infer(value, types, visited)
            }
        }
    }

    /// Check whether a static expression contains an infer binding.
    fn static_expression_contains_infer(
        &self,
        value: &StaticExpression,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        match value {
            StaticExpression::Type { ty } => self.type_contains_infer(*ty, types, visited),
            StaticExpression::Declaration {
                static_arguments, ..
            } => static_arguments.as_ref().is_some_and(|arguments| {
                arguments
                    .iter()
                    .any(|argument| self.static_argument_contains_infer(argument, types, visited))
            }),
            StaticExpression::ArrayExpression { elements } => elements
                .iter()
                .any(|element| self.static_expression_contains_infer(element, types, visited)),
            StaticExpression::TupleExpression { elements } => elements
                .iter()
                .any(|element| self.static_expression_contains_infer(element, types, visited)),
            StaticExpression::ObjectExpression { properties } => {
                properties.iter().any(|property| match property {
                    StaticProperty::Unevaluated { .. } => false,
                    StaticProperty::Field { value, default, .. } => {
                        self.static_expression_contains_infer(value, types, visited)
                            || default.as_ref().is_some_and(|default| {
                                self.static_expression_contains_infer(default, types, visited)
                            })
                    }
                    StaticProperty::Method { body, .. } => {
                        self.static_expression_contains_infer(body, types, visited)
                    }
                })
            }
            StaticExpression::RangeExpression { start, end, .. } => {
                self.static_expression_contains_infer(start, types, visited)
                    || self.static_expression_contains_infer(end, types, visited)
            }
            StaticExpression::Unevaluated { .. }
            | StaticExpression::ScalarLiteral { .. }
            | StaticExpression::TypeLiteral { .. } => false,
        }
    }

    /// Infer substitutions for conditional types with `infer` bindings.
    pub(crate) fn infer_conditional_type_substitutions(
        &self,
        module: &Module,
        profile: ProfileId,
        distributive: bool,
        left: LocalTypeId,
        right: LocalTypeId,
        source_id: LocalNodeIdAny,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<InferSubstitutions> {
        // collect substitutions for each matching branch
        let mut visited = HashSet::new();
        self.infer_conditional_type_substitutions_inner(
            module,
            profile,
            distributive,
            left,
            right,
            source_id,
            symbols,
            types,
            &mut visited,
        )
    }

    /// Infer substitutions for conditional type matching with recursion guard.
    fn infer_conditional_type_substitutions_inner(
        &self,
        module: &Module,
        profile: ProfileId,
        distributive: bool,
        left: LocalTypeId,
        right: LocalTypeId,
        source_id: LocalNodeIdAny,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        visited: &mut HashSet<(LocalTypeId, LocalTypeId)>,
    ) -> Option<InferSubstitutions> {
        // stop on recursion cycles
        if !visited.insert((left, right)) {
            return Some(InferSubstitutions::default());
        }

        // infer from matching reference arguments before normalization
        let left_type = types.get_type(left).clone();
        let right_type = types.get_type(right).clone();
        if let (
            Type::Reference {
                symbol,
                static_arguments: left_arguments,
            },
            Type::Reference {
                symbol: right_symbol,
                static_arguments: right_arguments,
            },
        ) = (&left_type, &right_type)
            && symbol == right_symbol
            && let (Some(left_arguments), Some(right_arguments)) =
                (left_arguments.as_deref(), right_arguments.as_deref())
            && let Some(inferred) = self
                .infer_conditional_type_substitutions_for_reference_arguments(
                    module,
                    profile,
                    left_arguments,
                    right_arguments,
                    distributive,
                    source_id,
                    symbols,
                    types,
                    visited,
                )
        {
            return Some(inferred);
        }

        // unwrap alias references before matching
        let left = self.unwrap_normalization_alias_reference(left, types);
        let right = self.unwrap_normalization_alias_reference(right, types);

        // normalize alias references with concrete arguments before matching
        let left = self.normalize_type(
            module,
            profile,
            left,
            symbols,
            types,
            NormalizationMode::Assign,
        );
        let right = self.normalize_type(
            module,
            profile,
            right,
            symbols,
            types,
            NormalizationMode::Assign,
        );

        // read the current type shapes
        let left_type = types.get_type(left).clone();
        let right_type = types.get_type(right).clone();

        // handle infer bindings and trivial matches early
        match (&left_type, &right_type) {
            (_, Type::Infer { name, .. }) => {
                let mut substitutions = InferSubstitutions::default();
                substitutions.by_name.insert(*name, left);
                return Some(substitutions);
            }
            (
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                },
                _,
            ) => {
                return None;
            }
            (
                Type::TypeLiteral {
                    value: TypeLiteral::Any,
                },
                _,
            ) => {
                return self.infer_substitutions_for_any_left(
                    module,
                    profile,
                    distributive,
                    left,
                    right,
                    source_id,
                    symbols,
                    types,
                );
            }
            _ => {}
        }

        // infer defaults for never inputs
        if matches!(
            left_type,
            Type::TypeLiteral {
                value: TypeLiteral::Never,
            }
        ) {
            return self.infer_substitutions_for_never_left(
                module, profile, right, source_id, symbols, types,
            );
        }

        // resolve static parameter constraints for the left side
        if let Type::Reference { symbol, .. } = &left_type
            && self.symbol_is_static_parameter(module, profile, *symbol, symbols, types)
        {
            let constraint_id = self.static_parameter_constraint_type(
                module, profile, *symbol, source_id, symbols, types,
            );
            if let Some(constraint_id) = constraint_id {
                return self.infer_conditional_type_substitutions_inner(
                    module,
                    profile,
                    distributive,
                    constraint_id,
                    right,
                    source_id,
                    symbols,
                    types,
                    visited,
                );
            }
        }

        // resolve static parameter constraints for the right side
        if let Type::Reference { symbol, .. } = &right_type
            && self.symbol_is_static_parameter(module, profile, *symbol, symbols, types)
        {
            let constraint_id = self.static_parameter_constraint_type(
                module, profile, *symbol, source_id, symbols, types,
            );
            if let Some(constraint_id) = constraint_id {
                return self.infer_conditional_type_substitutions_inner(
                    module,
                    profile,
                    distributive,
                    left,
                    constraint_id,
                    source_id,
                    symbols,
                    types,
                    visited,
                );
            }
        }

        // short-circuit identical type aliases
        if let (
            Type::Reference { symbol, .. },
            Type::Reference {
                symbol: right_symbol,
                ..
            },
        ) = (&left_type, &right_type)
            && symbol.ty() == SymbolType::TypeAlias
            && right_symbol.ty() == SymbolType::TypeAlias
            && symbol == right_symbol
        {
            return Some(InferSubstitutions::default());
        }

        // compare instance types for identical references
        if let (
            Type::Reference {
                symbol,
                static_arguments,
            },
            Type::Reference {
                symbol: right_symbol,
                static_arguments: right_arguments,
            },
        ) = (&left_type, &right_type)
            && symbol == right_symbol
            && static_arguments
                .as_ref()
                .is_none_or(|arguments| arguments.is_empty())
            && right_arguments
                .as_ref()
                .is_none_or(|arguments| arguments.is_empty())
            && let Some(left_instance_id) = types.get_instance_type_id(*symbol)
        {
            return self.infer_conditional_type_substitutions_inner(
                module,
                profile,
                distributive,
                left_instance_id,
                right,
                source_id,
                symbols,
                types,
                visited,
            );
        }

        // expand left instance types when available
        if let (
            Type::Reference {
                symbol,
                static_arguments,
            },
            _,
        ) = (&left_type, &right_type)
            && static_arguments
                .as_ref()
                .is_none_or(|arguments| arguments.is_empty())
            && let Some(left_instance_id) = types.get_instance_type_id(*symbol)
        {
            return self.infer_conditional_type_substitutions_inner(
                module,
                profile,
                distributive,
                left_instance_id,
                right,
                source_id,
                symbols,
                types,
                visited,
            );
        }

        // expand right instance types when available
        if let (
            _,
            Type::Reference {
                symbol,
                static_arguments,
            },
        ) = (&left_type, &right_type)
            && static_arguments
                .as_ref()
                .is_none_or(|arguments| arguments.is_empty())
            && let Some(right_instance_id) = types.get_instance_type_id(*symbol)
        {
            return self.infer_conditional_type_substitutions_inner(
                module,
                profile,
                distributive,
                left,
                right_instance_id,
                source_id,
                symbols,
                types,
                visited,
            );
        }

        // normalize alias references before structural matching
        if let (Type::Reference { symbol, .. }, _) = (&left_type, &right_type)
            && symbol.ty() == SymbolType::TypeAlias
        {
            let normalized_left = self.normalize_type(
                module,
                profile,
                left,
                symbols,
                types,
                NormalizationMode::Flow,
            );
            if normalized_left != left {
                return self.infer_conditional_type_substitutions_inner(
                    module,
                    profile,
                    distributive,
                    normalized_left,
                    right,
                    source_id,
                    symbols,
                    types,
                    visited,
                );
            }
        }

        // normalize alias references before structural matching
        if let (_, Type::Reference { symbol, .. }) = (&left_type, &right_type)
            && symbol.ty() == SymbolType::TypeAlias
        {
            let normalized_right = self.normalize_type(
                module,
                profile,
                right,
                symbols,
                types,
                NormalizationMode::Flow,
            );
            if normalized_right != right {
                return self.infer_conditional_type_substitutions_inner(
                    module,
                    profile,
                    distributive,
                    left,
                    normalized_right,
                    source_id,
                    symbols,
                    types,
                    visited,
                );
            }
        }

        // expose callable value shapes for matching
        if let (Type::Reference { symbol, .. }, _) = (&left_type, &right_type)
            && let Some(function_id) = types.get_value_type_id(*symbol)
        {
            return self.infer_conditional_type_substitutions_inner(
                module,
                profile,
                distributive,
                function_id,
                right,
                source_id,
                symbols,
                types,
                visited,
            );
        }

        // expose callable value shapes for matching
        if let (_, Type::Reference { symbol, .. }) = (&left_type, &right_type)
            && let Some(function_id) = types.get_value_type_id(*symbol)
        {
            return self.infer_conditional_type_substitutions_inner(
                module,
                profile,
                distributive,
                left,
                function_id,
                source_id,
                symbols,
                types,
                visited,
            );
        }

        // structural matching across type shapes
        match (left_type, right_type) {
            (Type::Conditional { .. }, _) | (_, Type::Conditional { .. }) => {
                Some(InferSubstitutions::default())
            }
            (Type::Union { elements }, _) => {
                let mut combined = InferSubstitutions::default();
                for element_id in elements {
                    let inferred = self.infer_conditional_type_substitutions_inner(
                        module,
                        profile,
                        distributive,
                        element_id,
                        right,
                        source_id,
                        symbols,
                        types,
                        visited,
                    );
                    match inferred {
                        Some(inferred) => {
                            self.merge_infer_substitutions(
                                module,
                                profile,
                                &mut combined,
                                inferred,
                                symbols,
                                types,
                                InferMergeMode::Union,
                            );
                        }
                        None => {
                            if !distributive {
                                return None;
                            }
                        }
                    }
                }
                Some(combined)
            }
            (_, Type::Union { elements }) => {
                let mut combined = InferSubstitutions::default();
                // require at least one matching union branch
                let mut matched_any = false;
                for element_id in elements {
                    if let Some(inferred) = self.infer_conditional_type_substitutions_inner(
                        module,
                        profile,
                        distributive,
                        left,
                        element_id,
                        source_id,
                        symbols,
                        types,
                        visited,
                    ) {
                        self.merge_infer_substitutions(
                            module,
                            profile,
                            &mut combined,
                            inferred,
                            symbols,
                            types,
                            InferMergeMode::Union,
                        );
                        matched_any = true;
                    }
                }
                if !matched_any {
                    return None;
                }
                Some(combined)
            }
            (Type::Intersection { elements }, _) => {
                let mut combined = InferSubstitutions::default();
                for element_id in elements {
                    if let Some(inferred) = self.infer_conditional_type_substitutions_inner(
                        module,
                        profile,
                        distributive,
                        element_id,
                        right,
                        source_id,
                        symbols,
                        types,
                        visited,
                    ) {
                        self.merge_infer_substitutions(
                            module,
                            profile,
                            &mut combined,
                            inferred,
                            symbols,
                            types,
                            InferMergeMode::Union,
                        );
                    }
                }
                Some(combined)
            }
            (
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(string_id)),
                },
                Type::TemplateLiteral { strings, spans },
            ) => {
                let value = self.program.strings.get(string_id).to_string();
                self.infer_template_literal_substitutions_from_string(
                    module, profile, &value, &strings, &spans, source_id, symbols, types,
                )
            }
            (
                Type::TemplateLiteral {
                    strings: left_strings,
                    spans: left_spans,
                },
                Type::TemplateLiteral {
                    strings: right_strings,
                    spans: right_spans,
                },
            ) if left_spans.is_empty() => {
                // treat spanless templates as string literals for matching
                let mut value = String::new();
                for string_id in left_strings {
                    let fragment = self.program.strings.get(string_id);
                    value.push_str(fragment.as_ref());
                }
                self.infer_template_literal_substitutions_from_string(
                    module,
                    profile,
                    &value,
                    &right_strings,
                    &right_spans,
                    source_id,
                    symbols,
                    types,
                )
            }
            (
                Type::TemplateLiteral {
                    strings: left_strings,
                    spans: left_spans,
                },
                Type::TemplateLiteral {
                    strings: right_strings,
                    spans: right_spans,
                },
            ) => self.infer_template_literal_substitutions_from_template(
                module,
                profile,
                distributive,
                &left_strings,
                &left_spans,
                &right_strings,
                &right_spans,
                source_id,
                symbols,
                types,
                visited,
            ),
            (_, Type::Intersection { elements }) => {
                let mut combined = InferSubstitutions::default();
                for element_id in elements {
                    let inferred = self.infer_conditional_type_substitutions_inner(
                        module,
                        profile,
                        distributive,
                        left,
                        element_id,
                        source_id,
                        symbols,
                        types,
                        visited,
                    )?;
                    self.merge_infer_substitutions(
                        module,
                        profile,
                        &mut combined,
                        inferred,
                        symbols,
                        types,
                        InferMergeMode::Union,
                    );
                }
                Some(combined)
            }
            (
                Type::Array { element, .. },
                Type::Array {
                    element: right_element,
                    ..
                },
            ) => {
                let Some(left_element) = element else {
                    return Some(InferSubstitutions::default());
                };
                let Some(right_element) = right_element else {
                    return Some(InferSubstitutions::default());
                };
                self.infer_conditional_type_substitutions_inner(
                    module,
                    profile,
                    distributive,
                    left_element,
                    right_element,
                    source_id,
                    symbols,
                    types,
                    visited,
                )
            }
            (
                Type::ArraySized { element, .. },
                Type::ArraySized {
                    element: right_element,
                    ..
                },
            ) => self.infer_conditional_type_substitutions_inner(
                module,
                profile,
                distributive,
                element,
                right_element,
                source_id,
                symbols,
                types,
                visited,
            ),
            (
                Type::Tuple {
                    elements,
                    is_readonly,
                },
                Type::Tuple {
                    elements: right_elements,
                    is_readonly: right_is_readonly,
                },
            ) => {
                if is_readonly != right_is_readonly {
                    return None;
                }
                if elements.len() != right_elements.len() {
                    return None;
                }
                let mut combined = InferSubstitutions::default();
                for (left_element, right_element) in elements.iter().zip(right_elements.iter()) {
                    let inferred = self.infer_conditional_type_substitutions_inner(
                        module,
                        profile,
                        distributive,
                        left_element.ty,
                        right_element.ty,
                        source_id,
                        symbols,
                        types,
                        visited,
                    )?;
                    self.merge_infer_substitutions(
                        module,
                        profile,
                        &mut combined,
                        inferred,
                        symbols,
                        types,
                        InferMergeMode::Union,
                    );
                }
                Some(combined)
            }
            (
                Type::Function {
                    static_parameters,
                    this_parameter,
                    dynamic_parameters,
                    return_type,
                    ..
                },
                Type::Function {
                    static_parameters: right_static_parameters,
                    this_parameter: right_this_parameter,
                    dynamic_parameters: right_dynamic_parameters,
                    return_type: right_return_type,
                    ..
                },
            ) => {
                // ensure static parameter counts match
                if static_parameters.len() != right_static_parameters.len() {
                    return None;
                }

                // infer static parameter substitutions
                let mut combined = InferSubstitutions::default();
                for (left_parameter, right_parameter) in
                    static_parameters.iter().zip(right_static_parameters.iter())
                {
                    let inferred = self.infer_conditional_type_substitutions_inner(
                        module,
                        profile,
                        distributive,
                        *left_parameter,
                        *right_parameter,
                        source_id,
                        symbols,
                        types,
                        visited,
                    )?;
                    self.merge_infer_substitutions(
                        module,
                        profile,
                        &mut combined,
                        inferred,
                        symbols,
                        types,
                        InferMergeMode::Union,
                    );
                }
                // infer this-parameter substitutions
                let inferred = match (this_parameter, right_this_parameter) {
                    (Some(left_parameter), Some(right_parameter)) => self
                        .infer_conditional_type_substitutions_inner(
                            module,
                            profile,
                            distributive,
                            left_parameter,
                            right_parameter,
                            source_id,
                            symbols,
                            types,
                            visited,
                        ),
                    (None, None) => Some(InferSubstitutions::default()),
                    _ => None,
                }?;
                self.merge_infer_substitutions(
                    module,
                    profile,
                    &mut combined,
                    inferred,
                    symbols,
                    types,
                    InferMergeMode::Intersection,
                );

                // check dynamic parameter matching
                let mut matched_dynamic = false;
                if dynamic_parameters.len() == right_dynamic_parameters.len() {
                    for (left_parameter, right_parameter) in dynamic_parameters
                        .iter()
                        .zip(right_dynamic_parameters.iter())
                    {
                        let inferred = self.infer_conditional_type_substitutions_inner(
                            module,
                            profile,
                            distributive,
                            *left_parameter,
                            *right_parameter,
                            source_id,
                            symbols,
                            types,
                            visited,
                        )?;
                        self.merge_infer_substitutions(
                            module,
                            profile,
                            &mut combined,
                            inferred,
                            symbols,
                            types,
                            InferMergeMode::Intersection,
                        );
                    }
                    matched_dynamic = true;
                } else if right_dynamic_parameters.len() == 1 {
                    let only = right_dynamic_parameters[0];
                    let tuple_type = types.insert_type_from_any(
                        Type::Tuple {
                            elements: dynamic_parameters
                                .iter()
                                .map(|parameter| TypeElement::new(*parameter))
                                .collect(),
                            is_readonly: false,
                        },
                        source_id,
                    );

                    if let Some(inferred) = self.infer_conditional_type_substitutions_inner(
                        module,
                        profile,
                        distributive,
                        tuple_type,
                        only,
                        source_id,
                        symbols,
                        types,
                        visited,
                    ) {
                        self.merge_infer_substitutions(
                            module,
                            profile,
                            &mut combined,
                            inferred,
                            symbols,
                            types,
                            InferMergeMode::Intersection,
                        );
                        matched_dynamic = true;
                    } else {
                        // fall back to assignability for variadic patterns
                        let options = self.analyze_context_options_for_module(module.id);
                        matched_dynamic = self
                            .is_type_assignable(
                                module, profile, symbols, only, tuple_type, types, &options,
                            )
                            .is_assignable();
                    }
                }

                if !matched_dynamic {
                    return None;
                }

                // infer return type substitutions
                if let (Some(left_return), Some(right_return)) = (return_type, right_return_type)
                    && self.type_contains_infer(right_return, types, &mut HashSet::new())
                {
                    let inferred = self.infer_conditional_type_substitutions_inner(
                        module,
                        profile,
                        distributive,
                        left_return,
                        right_return,
                        source_id,
                        symbols,
                        types,
                        visited,
                    )?;
                    self.merge_infer_substitutions(
                        module,
                        profile,
                        &mut combined,
                        inferred,
                        symbols,
                        types,
                        InferMergeMode::Union,
                    );
                }
                Some(combined)
            }
            (
                Type::Function { .. },
                Type::Object {
                    call_signatures,
                    construct_signatures,
                    ..
                },
            ) => {
                // match function types against callable object patterns
                let mut combined = InferSubstitutions::default();
                for right_signature in call_signatures
                    .iter()
                    .chain(construct_signatures.iter())
                    .copied()
                {
                    let inferred = self.infer_conditional_type_substitutions_inner(
                        module,
                        profile,
                        distributive,
                        left,
                        right_signature,
                        source_id,
                        symbols,
                        types,
                        visited,
                    )?;
                    self.merge_infer_substitutions(
                        module,
                        profile,
                        &mut combined,
                        inferred,
                        symbols,
                        types,
                        InferMergeMode::Union,
                    );
                }
                Some(combined)
            }
            (
                Type::Object {
                    call_signatures,
                    construct_signatures,
                    ..
                },
                Type::Function { .. },
            ) => {
                // match callable objects against function patterns
                let mut combined = InferSubstitutions::default();
                for left_signature in call_signatures
                    .iter()
                    .chain(construct_signatures.iter())
                    .copied()
                {
                    let inferred = self.infer_conditional_type_substitutions_inner(
                        module,
                        profile,
                        distributive,
                        left_signature,
                        right,
                        source_id,
                        symbols,
                        types,
                        visited,
                    )?;
                    self.merge_infer_substitutions(
                        module,
                        profile,
                        &mut combined,
                        inferred,
                        symbols,
                        types,
                        InferMergeMode::Union,
                    );
                }
                Some(combined)
            }
            (
                Type::Object {
                    fields,
                    call_signatures,
                    construct_signatures,
                    index_signatures,
                },
                Type::Object {
                    fields: right_fields,
                    call_signatures: right_calls,
                    construct_signatures: right_constructs,
                    index_signatures: right_indexes,
                },
            ) => {
                let mut combined = InferSubstitutions::default();

                // match required fields in the pattern
                for right_field in right_fields {
                    if let Some(left_field) =
                        fields.iter().find(|field| field.key == right_field.key)
                    {
                        let inferred = self.infer_conditional_type_substitutions_inner(
                            module,
                            profile,
                            distributive,
                            left_field.ty,
                            right_field.ty,
                            source_id,
                            symbols,
                            types,
                            visited,
                        )?;
                        self.merge_infer_substitutions(
                            module,
                            profile,
                            &mut combined,
                            inferred,
                            symbols,
                            types,
                            InferMergeMode::Union,
                        );
                    } else if !right_field.is_optional {
                        return None;
                    }
                }

                // match callable signatures in the pattern
                if !right_calls.is_empty() && call_signatures.is_empty() {
                    return None;
                }

                for right_signature in right_calls {
                    let mut matched_signature = false;
                    for left_signature in call_signatures.iter() {
                        if let Some(inferred) = self.infer_conditional_type_substitutions_inner(
                            module,
                            profile,
                            distributive,
                            *left_signature,
                            right_signature,
                            source_id,
                            symbols,
                            types,
                            visited,
                        ) {
                            self.merge_infer_substitutions(
                                module,
                                profile,
                                &mut combined,
                                inferred,
                                symbols,
                                types,
                                InferMergeMode::Union,
                            );
                            matched_signature = true;
                        }
                    }
                    if !matched_signature {
                        return None;
                    }
                }

                // match constructor signatures in the pattern
                if !right_constructs.is_empty() && construct_signatures.is_empty() {
                    return None;
                }

                for right_signature in right_constructs {
                    let mut matched_signature = false;
                    for left_signature in construct_signatures.iter() {
                        if let Some(inferred) = self.infer_conditional_type_substitutions_inner(
                            module,
                            profile,
                            distributive,
                            *left_signature,
                            right_signature,
                            source_id,
                            symbols,
                            types,
                            visited,
                        ) {
                            self.merge_infer_substitutions(
                                module,
                                profile,
                                &mut combined,
                                inferred,
                                symbols,
                                types,
                                InferMergeMode::Union,
                            );
                            matched_signature = true;
                        }
                    }
                    if !matched_signature {
                        return None;
                    }
                }

                // match index signatures in the pattern
                if !right_indexes.is_empty() && index_signatures.is_empty() {
                    return None;
                }

                for right_signature in right_indexes {
                    let mut matched_signature = false;
                    for left_signature in index_signatures.iter() {
                        let inferred_key = self.infer_conditional_type_substitutions_inner(
                            module,
                            profile,
                            distributive,
                            left_signature.key_type,
                            right_signature.key_type,
                            source_id,
                            symbols,
                            types,
                            visited,
                        );
                        let inferred_value = self.infer_conditional_type_substitutions_inner(
                            module,
                            profile,
                            distributive,
                            left_signature.value_type,
                            right_signature.value_type,
                            source_id,
                            symbols,
                            types,
                            visited,
                        );

                        if let (Some(inferred_key), Some(inferred_value)) =
                            (inferred_key, inferred_value)
                        {
                            self.merge_infer_substitutions(
                                module,
                                profile,
                                &mut combined,
                                inferred_key,
                                symbols,
                                types,
                                InferMergeMode::Union,
                            );
                            self.merge_infer_substitutions(
                                module,
                                profile,
                                &mut combined,
                                inferred_value,
                                symbols,
                                types,
                                InferMergeMode::Union,
                            );
                            matched_signature = true;
                        }
                    }
                    if !matched_signature {
                        return None;
                    }
                }

                Some(combined)
            }
            (
                Type::TypeLiteral {
                    value:
                        TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(_))
                        | TypeLiteral::ScalarLiteral(ScalarLiteral::Float(_)),
                },
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(destack_dir::PrimitiveType::Number),
                },
            )
            | (
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_)),
                },
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(destack_dir::PrimitiveType::String),
                },
            ) => Some(InferSubstitutions::default()),
            (
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(_)),
                },
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(destack_dir::PrimitiveType::Boolean),
                },
            ) => Some(InferSubstitutions::default()),
            (Type::TypeLiteral { value }, Type::TypeLiteral { value: right_value }) => {
                if value == right_value {
                    return Some(InferSubstitutions::default());
                }
                None
            }
            (
                Type::TypeLiteral {
                    value: TypeLiteral::Any,
                },
                _,
            ) => Some(InferSubstitutions::default()),
            _ => None,
        }
    }

    /// Infer substitutions for conditional patterns when the left side is `any`.
    fn infer_substitutions_for_any_left(
        &self,
        module: &Module,
        profile: ProfileId,
        distributive: bool,
        left: LocalTypeId,
        right: LocalTypeId,
        source_id: LocalNodeIdAny,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<InferSubstitutions> {
        // handle union patterns by merging successful branches
        if let Type::Union { elements } = types.get_type(right).clone() {
            let mut combined = InferSubstitutions::default();
            let mut matched = false;

            for element_id in elements {
                let mut branch_visited = HashSet::new();
                if let Some(inferred) = self.infer_conditional_type_substitutions_inner(
                    module,
                    profile,
                    distributive,
                    left,
                    element_id,
                    source_id,
                    symbols,
                    types,
                    &mut branch_visited,
                ) {
                    matched = true;
                    self.merge_infer_substitutions(
                        module,
                        profile,
                        &mut combined,
                        inferred,
                        symbols,
                        types,
                        InferMergeMode::Union,
                    );
                }
            }

            if matched {
                return Some(combined);
            }
            return None;
        }

        // infer template literal spans as string-compatible for `any`
        if let Type::TemplateLiteral { spans, .. } = types.get_type(right) {
            let spans = spans.clone();
            let substitutions = self.infer_template_literal_substitutions_for_never(
                module, profile, &spans, source_id, symbols, types,
            );
            return Some(substitutions);
        }

        // collect infer names from the pattern
        let mut names = HashSet::new();
        let mut visited = HashSet::new();
        self.collect_infer_names_from_type(right, types, &mut visited, &mut names);

        // default to an empty substitution when there are no infer names
        if names.is_empty() {
            return Some(InferSubstitutions::default());
        }

        // map infer bindings to unknown for `any` patterns
        let unknown_type = types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            },
            source_id,
        );
        let mut substitutions = InferSubstitutions::default();
        for name in names {
            substitutions.by_name.insert(name, unknown_type);
        }

        Some(substitutions)
    }

    /// Infer substitutions from matching reference static arguments.
    fn infer_conditional_type_substitutions_for_reference_arguments(
        &self,
        module: &Module,
        profile: ProfileId,
        left_arguments: &[StaticArgument],
        right_arguments: &[StaticArgument],
        distributive: bool,
        source_id: LocalNodeIdAny,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        visited: &mut HashSet<(LocalTypeId, LocalTypeId)>,
    ) -> Option<InferSubstitutions> {
        // infer substitutions for each aligned argument
        let mut combined = InferSubstitutions::default();
        if left_arguments.len() != right_arguments.len() {
            return None;
        }

        for (left_argument, right_argument) in left_arguments.iter().zip(right_arguments.iter()) {
            let left_ty = self.convert_static_argument_type(left_argument, source_id, types);
            let right_ty = self.convert_static_argument_type(right_argument, source_id, types);
            let inferred = self.infer_conditional_type_substitutions_inner(
                module,
                profile,
                distributive,
                left_ty,
                right_ty,
                source_id,
                symbols,
                types,
                visited,
            )?;
            self.merge_infer_substitutions(
                module,
                profile,
                &mut combined,
                inferred,
                symbols,
                types,
                InferMergeMode::Union,
            );
        }

        Some(combined)
    }

    /// Collect infer bindings from a pattern type.
    fn collect_infer_names_from_type(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
        names: &mut HashSet<StringId>,
    ) {
        // stop on recursion cycles
        if !visited.insert(type_id) {
            return;
        }

        // walk infer-bearing type shapes
        match types.get_type(type_id) {
            Type::Infer { name, .. } => {
                names.insert(*name);
            }
            Type::Reference {
                static_arguments, ..
            }
            | Type::Import {
                static_arguments, ..
            } => {
                if let Some(arguments) = static_arguments {
                    for argument in arguments {
                        self.collect_infer_names_from_static_argument(
                            argument, types, visited, names,
                        );
                    }
                }
            }
            Type::Conditional {
                distributive_symbol: _,
                left,
                right,
                then_type,
                else_type,
            } => {
                self.collect_infer_names_from_type(*left, types, visited, names);
                self.collect_infer_names_from_type(*right, types, visited, names);
                self.collect_infer_names_from_type(*then_type, types, visited, names);
                self.collect_infer_names_from_type(*else_type, types, visited, names);
            }
            Type::Mapped {
                parameter, value, ..
            } => {
                self.collect_infer_names_from_type(parameter.constraint, types, visited, names);
                if let Some(key_remap) = parameter.key_remap {
                    self.collect_infer_names_from_type(key_remap, types, visited, names);
                }
                self.collect_infer_names_from_type(*value, types, visited, names);
            }
            Type::Index { left, index } => {
                self.collect_infer_names_from_type(*left, types, visited, names);
                self.collect_infer_names_from_type(*index, types, visited, names);
            }
            Type::TemplateLiteral { spans, .. } => {
                for span in spans {
                    self.collect_infer_names_from_type(*span, types, visited, names);
                }
            }
            Type::Array { element, .. } => {
                if let Some(element) = element {
                    self.collect_infer_names_from_type(*element, types, visited, names);
                }
            }
            Type::ArraySized { element, .. } => {
                self.collect_infer_names_from_type(*element, types, visited, names);
            }
            Type::Tuple { elements, .. } => {
                for element in elements {
                    self.collect_infer_names_from_type(element.ty, types, visited, names);
                }
            }
            Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                for field in fields {
                    self.collect_infer_names_from_type(field.ty, types, visited, names);
                }
                for signature in call_signatures {
                    self.collect_infer_names_from_type(*signature, types, visited, names);
                }
                for signature in construct_signatures {
                    self.collect_infer_names_from_type(*signature, types, visited, names);
                }
                for signature in index_signatures {
                    self.collect_infer_names_from_type(signature.key_type, types, visited, names);
                    self.collect_infer_names_from_type(signature.value_type, types, visited, names);
                }
            }
            Type::Function {
                static_parameters,
                this_parameter,
                dynamic_parameters,
                return_type,
                ..
            } => {
                for parameter in static_parameters {
                    self.collect_infer_names_from_type(*parameter, types, visited, names);
                }
                if let Some(parameter) = this_parameter {
                    self.collect_infer_names_from_type(*parameter, types, visited, names);
                }
                for parameter in dynamic_parameters {
                    self.collect_infer_names_from_type(*parameter, types, visited, names);
                }
                if let Some(return_type) = return_type {
                    self.collect_infer_names_from_type(*return_type, types, visited, names);
                }
            }
            Type::Unary { right, .. }
            | Type::ValueOf { right, .. }
            | Type::ReferenceOf { right, .. }
            | Type::PointerOf { right, .. } => {
                self.collect_infer_names_from_type(*right, types, visited, names);
            }
            Type::Binary { left, right, .. } => {
                self.collect_infer_names_from_type(*left, types, visited, names);
                self.collect_infer_names_from_type(*right, types, visited, names);
            }
            Type::Predicate { target, .. } => {
                if let Some(target) = target {
                    self.collect_infer_names_from_type(*target, types, visited, names);
                }
            }
            Type::Union { elements } | Type::Intersection { elements } => {
                for element in elements {
                    self.collect_infer_names_from_type(*element, types, visited, names);
                }
            }
            Type::Value { value } => {
                self.collect_infer_names_from_type(*value, types, visited, names);
            }
            Type::TypeLiteral { .. }
            | Type::InferVar { .. }
            | Type::Unevaluated(_)
            | Type::This
            | Type::Error => {}
        }
    }

    /// Collect infer bindings from a static argument.
    fn collect_infer_names_from_static_argument(
        &self,
        argument: &StaticArgument,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
        names: &mut HashSet<StringId>,
    ) {
        // inspect static argument expressions
        match argument {
            StaticArgument::Unevaluated { .. } => {}
            StaticArgument::Evaluated { value, .. } => {
                self.collect_infer_names_from_static_expression(value, types, visited, names);
            }
        }
    }

    /// Collect infer bindings from a static expression.
    fn collect_infer_names_from_static_expression(
        &self,
        expression: &StaticExpression,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
        names: &mut HashSet<StringId>,
    ) {
        // walk static expression shapes
        match expression {
            StaticExpression::Type { ty } => {
                self.collect_infer_names_from_type(*ty, types, visited, names);
            }
            StaticExpression::Declaration {
                static_arguments, ..
            } => {
                if let Some(arguments) = static_arguments {
                    for argument in arguments {
                        self.collect_infer_names_from_static_argument(
                            argument, types, visited, names,
                        );
                    }
                }
            }
            StaticExpression::ArrayExpression { elements }
            | StaticExpression::TupleExpression { elements } => {
                for element in elements {
                    self.collect_infer_names_from_static_expression(element, types, visited, names);
                }
            }
            StaticExpression::ObjectExpression { properties } => {
                for property in properties {
                    match property {
                        StaticProperty::Unevaluated { .. } => {}
                        StaticProperty::Field { value, default, .. } => {
                            self.collect_infer_names_from_static_expression(
                                value, types, visited, names,
                            );
                            if let Some(default) = default {
                                self.collect_infer_names_from_static_expression(
                                    default, types, visited, names,
                                );
                            }
                        }
                        StaticProperty::Method { body, .. } => {
                            self.collect_infer_names_from_static_expression(
                                body, types, visited, names,
                            );
                        }
                    }
                }
            }
            StaticExpression::RangeExpression { start, end, .. } => {
                self.collect_infer_names_from_static_expression(start, types, visited, names);
                self.collect_infer_names_from_static_expression(end, types, visited, names);
            }
            StaticExpression::ScalarLiteral { .. }
            | StaticExpression::TypeLiteral { .. }
            | StaticExpression::Unevaluated { .. } => {}
        }
    }

    /// Infer substitutions for `never` inputs.
    fn infer_substitutions_for_never_left(
        &self,
        module: &Module,
        profile: ProfileId,
        right: LocalTypeId,
        source_id: LocalNodeIdAny,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<InferSubstitutions> {
        // read the target pattern
        let right_type = types.get_type(right).clone();

        // infer defaults for template literal spans
        if let Type::TemplateLiteral { spans, .. } = right_type {
            let substitutions = self.infer_template_literal_substitutions_for_never(
                module, profile, &spans, source_id, symbols, types,
            );
            return Some(substitutions);
        }

        // merge inferred defaults across union branches
        if let Type::Union { elements } = right_type {
            let mut combined = InferSubstitutions::default();
            for element_id in elements {
                if let Some(inferred) = self.infer_substitutions_for_never_left(
                    module, profile, element_id, source_id, symbols, types,
                ) {
                    self.merge_infer_substitutions(
                        module,
                        profile,
                        &mut combined,
                        inferred,
                        symbols,
                        types,
                        InferMergeMode::Union,
                    );
                }
            }
            return Some(combined);
        }

        // merge inferred defaults across intersection branches
        if let Type::Intersection { elements } = right_type {
            let mut combined = InferSubstitutions::default();
            for element_id in elements {
                if let Some(inferred) = self.infer_substitutions_for_never_left(
                    module, profile, element_id, source_id, symbols, types,
                ) {
                    self.merge_infer_substitutions(
                        module,
                        profile,
                        &mut combined,
                        inferred,
                        symbols,
                        types,
                        InferMergeMode::Intersection,
                    );
                }
            }
            return Some(combined);
        }

        // default infer bindings to never
        let mut infer_names = HashSet::new();
        self.collect_infer_names_from_type(right, types, &mut HashSet::new(), &mut infer_names);
        let mut substitutions = InferSubstitutions::default();
        let never_id = types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Never,
            },
            source_id,
        );

        for name in infer_names {
            substitutions.by_name.insert(name, never_id);
        }

        Some(substitutions)
    }

    /// Infer substitutions by matching a template literal type against a string literal.
    pub(crate) fn infer_template_literal_substitutions_from_string(
        &self,
        module: &Module,
        profile: ProfileId,
        value: &str,
        strings: &[StringId],
        spans: &[LocalTypeId],
        source_id: LocalNodeIdAny,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<InferSubstitutions> {
        // match template literal parts to the string
        let span_values = self.match_template_literal_to_string(strings, value)?;

        // require span counts to match
        if span_values.len() != spans.len() {
            return None;
        }

        // collect inferred substitutions
        let mut substitutions = InferSubstitutions::default();
        let mut visited = HashSet::new();

        // apply each span constraint
        for (span_ty_id, span_value) in spans.iter().zip(span_values.iter()) {
            // resolve span type
            let span_ty = types.get_type(*span_ty_id).clone();
            match span_ty {
                Type::Infer { name, constraint } => {
                    // enforce span constraints for constrained inference
                    if let Some(constraint_id) = constraint
                        && !self.template_span_matches_string(
                            module,
                            profile,
                            constraint_id,
                            span_value,
                            symbols,
                            types,
                            &mut visited,
                        )
                    {
                        return None;
                    }

                    // infer literal type for the span
                    let inferred_ty =
                        self.template_infer_literal_type(constraint, span_value, source_id, types)?;
                    if let Some(existing) = substitutions.by_name.get(&name).copied()
                        && !self.inferred_type_ids_equivalent(
                            module,
                            profile,
                            existing,
                            inferred_ty,
                            symbols,
                            types,
                        )
                    {
                        return None;
                    }

                    substitutions.by_name.insert(name, inferred_ty);
                }
                _ => {
                    // enforce non infer span constraints
                    if !self.template_span_matches_string(
                        module,
                        profile,
                        *span_ty_id,
                        span_value,
                        symbols,
                        types,
                        &mut visited,
                    ) {
                        return None;
                    }
                }
            }
        }

        Some(substitutions)
    }

    /// Infer substitutions for template literals when the input is never.
    fn infer_template_literal_substitutions_for_never(
        &self,
        module: &Module,
        profile: ProfileId,
        spans: &[LocalTypeId],
        source_id: LocalNodeIdAny,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> InferSubstitutions {
        // default to string spans when unconstrained
        let string_id = types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String),
            },
            source_id,
        );

        // collect inferred substitutions
        let mut substitutions = InferSubstitutions::default();

        // collect inferred bindings from each span
        for span_ty_id in spans {
            let span_ty = types.get_type(*span_ty_id).clone();
            let Type::Infer { name, constraint } = span_ty else {
                continue;
            };

            let inferred_ty = constraint.unwrap_or(string_id);
            if let Some(existing) = substitutions.by_name.get(&name).copied() {
                if !self.inferred_type_ids_equivalent(
                    module,
                    profile,
                    existing,
                    inferred_ty,
                    symbols,
                    types,
                ) {
                    let union_id =
                        self.union_type_from_list(vec![existing, inferred_ty], existing, types);
                    substitutions.by_name.insert(name, union_id);
                }
            } else {
                substitutions.by_name.insert(name, inferred_ty);
            }
        }

        substitutions
    }

    /// Infer substitutions by matching a template literal type against another template literal.
    pub(crate) fn infer_template_literal_substitutions_from_template(
        &self,
        module: &Module,
        profile: ProfileId,
        distributive: bool,
        left_strings: &[StringId],
        left_spans: &[LocalTypeId],
        right_strings: &[StringId],
        right_spans: &[LocalTypeId],
        source_id: LocalNodeIdAny,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        visited: &mut HashSet<(LocalTypeId, LocalTypeId)>,
    ) -> Option<InferSubstitutions> {
        // require aligned literal parts
        if left_strings.len() != right_strings.len() || left_spans.len() != right_spans.len() {
            return None;
        }

        // ensure literal parts match exactly
        for (left_string, right_string) in left_strings.iter().zip(right_strings.iter()) {
            if left_string != right_string {
                return None;
            }
        }

        // collect inferred substitutions
        let mut substitutions = InferSubstitutions::default();

        // apply each span mapping
        for (left_span, right_span) in left_spans.iter().zip(right_spans.iter()) {
            // resolve right span type
            let right_ty = types.get_type(*right_span).clone();
            match right_ty {
                Type::Infer { name, .. } => {
                    // map inferred spans directly
                    if let Some(existing) = substitutions.by_name.get(&name).copied()
                        && !self.inferred_type_ids_equivalent(
                            module,
                            profile,
                            existing,
                            *left_span,
                            symbols,
                            types,
                        )
                    {
                        return None;
                    }

                    substitutions.by_name.insert(name, *left_span);
                }
                _ => {
                    // recursively infer nested substitutions
                    let inferred = self.infer_conditional_type_substitutions_inner(
                        module,
                        profile,
                        distributive,
                        *left_span,
                        *right_span,
                        source_id,
                        symbols,
                        types,
                        visited,
                    )?;
                    self.merge_infer_substitutions(
                        module,
                        profile,
                        &mut substitutions,
                        inferred,
                        symbols,
                        types,
                        InferMergeMode::Union,
                    );
                }
            }
        }

        Some(substitutions)
    }

    /// Merge inferred substitutions by unioning divergent bindings.
    fn merge_infer_substitutions(
        &self,
        module: &Module,
        profile: ProfileId,
        base: &mut InferSubstitutions,
        other: InferSubstitutions,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        mode: InferMergeMode,
    ) {
        // merge inferred bindings across branches
        for (name, ty) in other.by_name {
            let entry = base.by_name.entry(name);
            match entry {
                std::collections::hash_map::Entry::Vacant(vacant) => {
                    vacant.insert(ty);
                }
                std::collections::hash_map::Entry::Occupied(mut occupied) => {
                    let existing = *occupied.get();
                    if self
                        .inferred_type_ids_equivalent(module, profile, existing, ty, symbols, types)
                    {
                        continue;
                    }

                    let merged = match mode {
                        InferMergeMode::Union => {
                            self.union_type_from_list(vec![existing, ty], existing, types)
                        }
                        InferMergeMode::Intersection => {
                            self.intersection_type_from_list(vec![existing, ty], existing, types)
                        }
                    };
                    occupied.insert(merged);
                }
            }
        }
    }

    /// Check whether two inferred type ids are equivalent for merging.
    fn inferred_type_ids_equivalent(
        &self,
        module: &Module,
        profile: ProfileId,
        left: LocalTypeId,
        right: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> bool {
        // compare assignability in both directions
        let options = self.analyze_context_options_for_module(module.id);
        let left_to_right = self
            .is_type_assignable(module, profile, symbols, right, left, types, &options)
            .is_assignable();
        let right_to_left = self
            .is_type_assignable(module, profile, symbols, left, right, types, &options)
            .is_assignable();

        left_to_right && right_to_left
    }

    /// Apply inferred bindings to a type id.
    pub(crate) fn substitute_infer_types(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        substitutions: &InferSubstitutions,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        let mut cache = HashMap::new();
        self.substitute_infer_types_inner(
            module,
            profile,
            type_id,
            substitutions,
            symbols,
            types,
            &mut cache,
        )
    }

    /// Substitute inferred bindings with a cache to avoid recursion cycles.
    fn substitute_infer_types_inner(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        substitutions: &InferSubstitutions,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        cache: &mut HashMap<LocalTypeId, LocalTypeId>,
    ) -> LocalTypeId {
        if let Some(mapped) = cache.get(&type_id) {
            return *mapped;
        }

        let ty = types.get_type(type_id).clone();
        let mapped_id = match ty {
            Type::Infer { name, .. } => {
                substitutions.by_name.get(&name).copied().unwrap_or(type_id)
            }
            Type::Reference {
                symbol,
                static_arguments,
            } => {
                // replace local infer bindings by name
                if static_arguments.is_none()
                    && let Some(mapped) =
                        self.infer_binding_substitution(module, symbol, substitutions, symbols)
                {
                    mapped
                } else {
                    let static_arguments = static_arguments.as_ref().map(|arguments| {
                        arguments
                            .iter()
                            .map(|argument| {
                                self.substitute_infer_static_argument(
                                    module,
                                    profile,
                                    argument,
                                    substitutions,
                                    symbols,
                                    types,
                                    cache,
                                )
                            })
                            .collect()
                    });
                    types.insert_type_from_type(
                        Type::Reference {
                            symbol,
                            static_arguments,
                        },
                        type_id,
                    )
                }
            }
            Type::Import {
                target,
                qualifier,
                static_arguments,
            } => {
                let static_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.substitute_infer_static_argument(
                                module,
                                profile,
                                argument,
                                substitutions,
                                symbols,
                                types,
                                cache,
                            )
                        })
                        .collect()
                });
                types.insert_type_from_type(
                    Type::Import {
                        target,
                        qualifier,
                        static_arguments,
                    },
                    type_id,
                )
            }
            Type::Conditional {
                distributive_symbol,
                left,
                right,
                then_type,
                else_type,
            } => {
                let mapped_left = self.substitute_infer_types_inner(
                    module,
                    profile,
                    left,
                    substitutions,
                    symbols,
                    types,
                    cache,
                );
                let mapped_right = self.substitute_infer_types_inner(
                    module,
                    profile,
                    right,
                    substitutions,
                    symbols,
                    types,
                    cache,
                );
                let mapped_then = self.substitute_infer_types_inner(
                    module,
                    profile,
                    then_type,
                    substitutions,
                    symbols,
                    types,
                    cache,
                );
                let mapped_else = self.substitute_infer_types_inner(
                    module,
                    profile,
                    else_type,
                    substitutions,
                    symbols,
                    types,
                    cache,
                );
                if mapped_left == left
                    && mapped_right == right
                    && mapped_then == then_type
                    && mapped_else == else_type
                {
                    type_id
                } else {
                    types.insert_type_from_type(
                        Type::Conditional {
                            distributive_symbol,
                            left: mapped_left,
                            right: mapped_right,
                            then_type: mapped_then,
                            else_type: mapped_else,
                        },
                        type_id,
                    )
                }
            }
            Type::Mapped {
                parameter,
                modifiers,
                value,
            } => {
                let mapped_constraint = self.substitute_infer_types_inner(
                    module,
                    profile,
                    parameter.constraint,
                    substitutions,
                    symbols,
                    types,
                    cache,
                );
                let mapped_key_remap = parameter.key_remap.map(|key_remap| {
                    self.substitute_infer_types_inner(
                        module,
                        profile,
                        key_remap,
                        substitutions,
                        symbols,
                        types,
                        cache,
                    )
                });
                let mapped_value = self.substitute_infer_types_inner(
                    module,
                    profile,
                    value,
                    substitutions,
                    symbols,
                    types,
                    cache,
                );
                if mapped_constraint == parameter.constraint
                    && mapped_key_remap == parameter.key_remap
                    && mapped_value == value
                {
                    type_id
                } else {
                    let mapped_parameter = TypeMappedParameter {
                        name: parameter.name,
                        symbol: parameter.symbol,
                        constraint: mapped_constraint,
                        key_remap: mapped_key_remap,
                    };
                    types.insert_type_from_type(
                        Type::Mapped {
                            parameter: mapped_parameter,
                            modifiers,
                            value: mapped_value,
                        },
                        type_id,
                    )
                }
            }
            Type::Index { left, index } => {
                let mapped_left = self.substitute_infer_types_inner(
                    module,
                    profile,
                    left,
                    substitutions,
                    symbols,
                    types,
                    cache,
                );
                let mapped_index = self.substitute_infer_types_inner(
                    module,
                    profile,
                    index,
                    substitutions,
                    symbols,
                    types,
                    cache,
                );
                if mapped_left == left && mapped_index == index {
                    type_id
                } else {
                    types.insert_type_from_type(
                        Type::Index {
                            left: mapped_left,
                            index: mapped_index,
                        },
                        type_id,
                    )
                }
            }
            Type::TemplateLiteral { strings, spans } => {
                let mapped_spans = spans
                    .iter()
                    .map(|span| {
                        self.substitute_infer_types_inner(
                            module,
                            profile,
                            *span,
                            substitutions,
                            symbols,
                            types,
                            cache,
                        )
                    })
                    .collect();
                types.insert_type_from_type(
                    Type::TemplateLiteral {
                        strings,
                        spans: mapped_spans,
                    },
                    type_id,
                )
            }
            Type::Array {
                element,
                is_readonly,
            } => {
                let mapped_element = element.map(|element| {
                    self.substitute_infer_types_inner(
                        module,
                        profile,
                        element,
                        substitutions,
                        symbols,
                        types,
                        cache,
                    )
                });
                if mapped_element == element {
                    type_id
                } else {
                    types.insert_type_from_type(
                        Type::Array {
                            element: mapped_element,
                            is_readonly,
                        },
                        type_id,
                    )
                }
            }
            Type::ArraySized {
                element,
                count,
                is_readonly,
            } => {
                let mapped_element = self.substitute_infer_types_inner(
                    module,
                    profile,
                    element,
                    substitutions,
                    symbols,
                    types,
                    cache,
                );
                if mapped_element == element {
                    type_id
                } else {
                    types.insert_type_from_type(
                        Type::ArraySized {
                            element: mapped_element,
                            count,
                            is_readonly,
                        },
                        type_id,
                    )
                }
            }
            Type::Tuple {
                elements,
                is_readonly,
            } => {
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        let mapped = self.substitute_infer_types_inner(
                            module,
                            profile,
                            element.ty,
                            substitutions,
                            symbols,
                            types,
                            cache,
                        );
                        if mapped == element.ty {
                            element.clone()
                        } else {
                            TypeElement {
                                ty: mapped,
                                ..element.clone()
                            }
                        }
                    })
                    .collect();
                types.insert_type_from_type(
                    Type::Tuple {
                        elements: mapped_elements,
                        is_readonly,
                    },
                    type_id,
                )
            }
            Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                let mapped_fields = fields
                    .iter()
                    .map(|field| {
                        let mapped = self.substitute_infer_types_inner(
                            module,
                            profile,
                            field.ty,
                            substitutions,
                            symbols,
                            types,
                            cache,
                        );
                        if mapped == field.ty {
                            field.clone()
                        } else {
                            TypeField {
                                ty: mapped,
                                ..field.clone()
                            }
                        }
                    })
                    .collect();
                let mapped_call_signatures = call_signatures
                    .iter()
                    .map(|signature| {
                        self.substitute_infer_types_inner(
                            module,
                            profile,
                            *signature,
                            substitutions,
                            symbols,
                            types,
                            cache,
                        )
                    })
                    .collect();
                let mapped_construct_signatures = construct_signatures
                    .iter()
                    .map(|signature| {
                        self.substitute_infer_types_inner(
                            module,
                            profile,
                            *signature,
                            substitutions,
                            symbols,
                            types,
                            cache,
                        )
                    })
                    .collect();
                let mapped_index_signatures = index_signatures
                    .iter()
                    .map(|signature| {
                        let mapped_key = self.substitute_infer_types_inner(
                            module,
                            profile,
                            signature.key_type,
                            substitutions,
                            symbols,
                            types,
                            cache,
                        );
                        let mapped_value = self.substitute_infer_types_inner(
                            module,
                            profile,
                            signature.value_type,
                            substitutions,
                            symbols,
                            types,
                            cache,
                        );
                        if mapped_key == signature.key_type && mapped_value == signature.value_type
                        {
                            signature.clone()
                        } else {
                            TypeIndexSignature {
                                key_type: mapped_key,
                                value_type: mapped_value,
                                ..signature.clone()
                            }
                        }
                    })
                    .collect();
                types.insert_type_from_type(
                    Type::Object {
                        fields: mapped_fields,
                        call_signatures: mapped_call_signatures,
                        construct_signatures: mapped_construct_signatures,
                        index_signatures: mapped_index_signatures,
                    },
                    type_id,
                )
            }
            Type::Function {
                asynchrony,
                cardinality,
                static_parameters,
                this_parameter,
                dynamic_parameters,
                return_type,
            } => {
                let mapped_static_parameters = static_parameters
                    .iter()
                    .map(|parameter| {
                        self.substitute_infer_types_inner(
                            module,
                            profile,
                            *parameter,
                            substitutions,
                            symbols,
                            types,
                            cache,
                        )
                    })
                    .collect();
                let mapped_this_parameter = this_parameter.map(|parameter| {
                    self.substitute_infer_types_inner(
                        module,
                        profile,
                        parameter,
                        substitutions,
                        symbols,
                        types,
                        cache,
                    )
                });
                let mapped_dynamic_parameters = dynamic_parameters
                    .iter()
                    .map(|parameter| {
                        self.substitute_infer_types_inner(
                            module,
                            profile,
                            *parameter,
                            substitutions,
                            symbols,
                            types,
                            cache,
                        )
                    })
                    .collect();
                let mapped_return_type = return_type.map(|return_type| {
                    self.substitute_infer_types_inner(
                        module,
                        profile,
                        return_type,
                        substitutions,
                        symbols,
                        types,
                        cache,
                    )
                });
                types.insert_type_from_type(
                    Type::Function {
                        asynchrony,
                        cardinality,
                        static_parameters: mapped_static_parameters,
                        this_parameter: mapped_this_parameter,
                        dynamic_parameters: mapped_dynamic_parameters,
                        return_type: mapped_return_type,
                    },
                    type_id,
                )
            }
            Type::ValueOf {
                mutability,
                variance,
                right,
            } => {
                let mapped_right = self.substitute_infer_types_inner(
                    module,
                    profile,
                    right,
                    substitutions,
                    symbols,
                    types,
                    cache,
                );
                if mapped_right == right {
                    type_id
                } else {
                    types.insert_type_from_type(
                        Type::ValueOf {
                            mutability,
                            variance,
                            right: mapped_right,
                        },
                        type_id,
                    )
                }
            }
            Type::ReferenceOf {
                mutability,
                variance,
                right,
            } => {
                let mapped_right = self.substitute_infer_types_inner(
                    module,
                    profile,
                    right,
                    substitutions,
                    symbols,
                    types,
                    cache,
                );
                if mapped_right == right {
                    type_id
                } else {
                    types.insert_type_from_type(
                        Type::ReferenceOf {
                            mutability,
                            variance,
                            right: mapped_right,
                        },
                        type_id,
                    )
                }
            }
            Type::PointerOf { mutability, right } => {
                let mapped_right = self.substitute_infer_types_inner(
                    module,
                    profile,
                    right,
                    substitutions,
                    symbols,
                    types,
                    cache,
                );
                if mapped_right == right {
                    type_id
                } else {
                    types.insert_type_from_type(
                        Type::PointerOf {
                            mutability,
                            right: mapped_right,
                        },
                        type_id,
                    )
                }
            }
            Type::Binary {
                left,
                operator,
                right,
            } => {
                let mapped_left = self.substitute_infer_types_inner(
                    module,
                    profile,
                    left,
                    substitutions,
                    symbols,
                    types,
                    cache,
                );
                let mapped_right = self.substitute_infer_types_inner(
                    module,
                    profile,
                    right,
                    substitutions,
                    symbols,
                    types,
                    cache,
                );
                if mapped_left == left && mapped_right == right {
                    type_id
                } else {
                    types.insert_type_from_type(
                        Type::Binary {
                            left: mapped_left,
                            operator,
                            right: mapped_right,
                        },
                        type_id,
                    )
                }
            }
            Type::Unary { operator, right } => {
                let mapped_right = self.substitute_infer_types_inner(
                    module,
                    profile,
                    right,
                    substitutions,
                    symbols,
                    types,
                    cache,
                );
                if mapped_right == right {
                    type_id
                } else {
                    types.insert_type_from_type(
                        Type::Unary {
                            operator,
                            right: mapped_right,
                        },
                        type_id,
                    )
                }
            }
            Type::Union { elements } => {
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        self.substitute_infer_types_inner(
                            module,
                            profile,
                            *element,
                            substitutions,
                            symbols,
                            types,
                            cache,
                        )
                    })
                    .collect::<Vec<_>>();
                if mapped_elements == elements {
                    type_id
                } else {
                    types.insert_type_from_type(
                        Type::Union {
                            elements: mapped_elements,
                        },
                        type_id,
                    )
                }
            }
            Type::Intersection { elements } => {
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        self.substitute_infer_types_inner(
                            module,
                            profile,
                            *element,
                            substitutions,
                            symbols,
                            types,
                            cache,
                        )
                    })
                    .collect::<Vec<_>>();
                if mapped_elements == elements {
                    type_id
                } else {
                    types.insert_type_from_type(
                        Type::Intersection {
                            elements: mapped_elements,
                        },
                        type_id,
                    )
                }
            }
            Type::Predicate {
                asserts,
                subject,
                target,
            } => {
                let mapped_target = target.map(|target| {
                    self.substitute_infer_types_inner(
                        module,
                        profile,
                        target,
                        substitutions,
                        symbols,
                        types,
                        cache,
                    )
                });
                if mapped_target == target {
                    type_id
                } else {
                    types.insert_type_from_type(
                        Type::Predicate {
                            asserts,
                            subject,
                            target: mapped_target,
                        },
                        type_id,
                    )
                }
            }
            Type::Value { value } => {
                let mapped_value = self.substitute_infer_types_inner(
                    module,
                    profile,
                    value,
                    substitutions,
                    symbols,
                    types,
                    cache,
                );
                if mapped_value == value {
                    type_id
                } else {
                    types.insert_type_from_type(
                        Type::Value {
                            value: mapped_value,
                        },
                        type_id,
                    )
                }
            }
            Type::TypeLiteral { .. }
            | Type::InferVar { .. }
            | Type::Unevaluated(_)
            | Type::This
            | Type::Error => type_id,
        };

        cache.insert(type_id, mapped_id);
        mapped_id
    }

    /// Resolve inferred types for a local infer binding symbol.
    fn infer_binding_substitution(
        &self,
        module: &Module,
        symbol: GlobalSymbolId,
        substitutions: &InferSubstitutions,
        symbols: &SymbolTable,
    ) -> Option<LocalTypeId> {
        // only local symbols can be conditional infer bindings
        if symbol.module_id != module.id {
            return None;
        }

        // only local type symbols without declarations are infer bindings
        let symbol_entry = symbols.get_symbol(symbol.local_id);
        if symbol_entry.space != SymbolSpace::Type || symbol_entry.kind != SymbolKind::Local {
            return None;
        }

        // skip declared symbols that shadow infer names
        if symbol_entry.primary_declaration.is_some() {
            return None;
        }

        // resolve the inferred binding by name
        let name = symbol_entry.name()?;
        substitutions.by_name.get(&name).copied()
    }

    /// Substitute inferred bindings into a static argument.
    fn substitute_infer_static_argument(
        &self,
        module: &Module,
        profile: ProfileId,
        argument: &StaticArgument,
        substitutions: &InferSubstitutions,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        cache: &mut HashMap<LocalTypeId, LocalTypeId>,
    ) -> StaticArgument {
        match argument {
            StaticArgument::Unevaluated { .. } => argument.clone(),
            StaticArgument::Evaluated { name, value } => {
                let mapped = self.substitute_infer_static_expression(
                    module,
                    profile,
                    value,
                    substitutions,
                    symbols,
                    types,
                    cache,
                );
                StaticArgument::Evaluated {
                    name: *name,
                    value: mapped,
                }
            }
        }
    }

    /// Substitute inferred bindings into a static expression.
    fn substitute_infer_static_expression(
        &self,
        module: &Module,
        profile: ProfileId,
        value: &StaticExpression,
        substitutions: &InferSubstitutions,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        cache: &mut HashMap<LocalTypeId, LocalTypeId>,
    ) -> StaticExpression {
        match value {
            StaticExpression::Type { ty } => {
                let mapped = self.substitute_infer_types_inner(
                    module,
                    profile,
                    *ty,
                    substitutions,
                    symbols,
                    types,
                    cache,
                );
                StaticExpression::Type { ty: mapped }
            }
            StaticExpression::Declaration {
                declaration,
                static_arguments,
            } => {
                let mapped_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.substitute_infer_static_argument(
                                module,
                                profile,
                                argument,
                                substitutions,
                                symbols,
                                types,
                                cache,
                            )
                        })
                        .collect()
                });
                StaticExpression::Declaration {
                    declaration: *declaration,
                    static_arguments: mapped_arguments,
                }
            }
            StaticExpression::RangeExpression {
                start,
                end,
                is_inclusive,
            } => {
                let mapped_start = self.substitute_infer_static_expression(
                    module,
                    profile,
                    start,
                    substitutions,
                    symbols,
                    types,
                    cache,
                );
                let mapped_end = self.substitute_infer_static_expression(
                    module,
                    profile,
                    end,
                    substitutions,
                    symbols,
                    types,
                    cache,
                );
                StaticExpression::RangeExpression {
                    start: Box::new(mapped_start),
                    end: Box::new(mapped_end),
                    is_inclusive: *is_inclusive,
                }
            }
            StaticExpression::ArrayExpression { elements } => {
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        self.substitute_infer_static_expression(
                            module,
                            profile,
                            element,
                            substitutions,
                            symbols,
                            types,
                            cache,
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
                        self.substitute_infer_static_expression(
                            module,
                            profile,
                            element,
                            substitutions,
                            symbols,
                            types,
                            cache,
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
                        self.substitute_infer_static_property(
                            module,
                            profile,
                            property,
                            substitutions,
                            symbols,
                            types,
                            cache,
                        )
                    })
                    .collect();
                StaticExpression::ObjectExpression {
                    properties: mapped_properties,
                }
            }
            StaticExpression::Unevaluated { .. }
            | StaticExpression::ScalarLiteral { .. }
            | StaticExpression::TypeLiteral { .. } => value.clone(),
        }
    }

    /// Substitute inferred bindings into a static property.
    fn substitute_infer_static_property(
        &self,
        module: &Module,
        profile: ProfileId,
        property: &StaticProperty,
        substitutions: &InferSubstitutions,
        symbols: &SymbolTable,
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
                let mapped_value = self.substitute_infer_static_expression(
                    module,
                    profile,
                    value,
                    substitutions,
                    symbols,
                    types,
                    cache,
                );
                let mapped_default = default.as_ref().map(|default| {
                    self.substitute_infer_static_expression(
                        module,
                        profile,
                        default,
                        substitutions,
                        symbols,
                        types,
                        cache,
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
                let mapped_body = self.substitute_infer_static_expression(
                    module,
                    profile,
                    body,
                    substitutions,
                    symbols,
                    types,
                    cache,
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
}
