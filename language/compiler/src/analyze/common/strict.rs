use std::collections::HashSet;

use destack_dir::{
    GlobalSymbolId, LocalTypeId, PrimitiveType, StaticArgument, StaticExpression, StaticProperty,
    Type, TypeElement, TypeField, TypeIndexSignature, TypeLiteral, TypeTable,
};
use destack_workspace::Module;

use crate::Compiler;

impl Compiler {
    /// Check whether a type contains the `any` literal.
    pub(crate) fn type_contains_any(
        &self,
        module: &Module,
        ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        self.type_contains_forbidden_literal(module, ty_id, types, literal_is_any)
    }

    /// Check whether a type contains the `unknown` literal.
    pub(crate) fn type_contains_unknown(
        &self,
        module: &Module,
        ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        self.type_contains_forbidden_literal(module, ty_id, types, literal_is_unknown)
    }

    /// Check whether a type contains imprecise primitive literals.
    pub(crate) fn type_contains_imprecise_primitive(
        &self,
        module: &Module,
        ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        self.type_contains_forbidden_literal(module, ty_id, types, literal_is_imprecise_primitive)
    }

    /// Check whether a type contains a forbidden literal.
    fn type_contains_forbidden_literal(
        &self,
        module: &Module,
        ty_id: LocalTypeId,
        types: &TypeTable,
        predicate: fn(&TypeLiteral) -> bool,
    ) -> bool {
        // track visited ids to prevent cycles
        let mut visited_types = HashSet::new();
        let mut visited_symbols = HashSet::new();

        // scan the type graph
        self.type_contains_forbidden_literal_inner(
            module,
            ty_id,
            types,
            predicate,
            &mut visited_types,
            &mut visited_symbols,
        )
    }

    /// Walk a type tree for forbidden literal usage.
    fn type_contains_forbidden_literal_inner(
        &self,
        module: &Module,
        ty_id: LocalTypeId,
        types: &TypeTable,
        predicate: fn(&TypeLiteral) -> bool,
        visited_types: &mut HashSet<LocalTypeId>,
        visited_symbols: &mut HashSet<GlobalSymbolId>,
    ) -> bool {
        // stop on recursion
        if !visited_types.insert(ty_id) {
            return false;
        }

        // read the type for matching
        let ty = types.get_type(ty_id);

        // walk the type structure
        match ty {
            Type::TypeLiteral { value } => predicate(value),
            Type::Value { value }
            | Type::Unary { right: value, .. }
            | Type::ValueOf { right: value, .. }
            | Type::ReferenceOf { right: value, .. }
            | Type::PointerOf { right: value, .. } => self.type_contains_forbidden_literal_inner(
                module,
                *value,
                types,
                predicate,
                visited_types,
                visited_symbols,
            ),
            Type::Reference { symbol, .. } => self.type_reference_contains_forbidden_literal(
                module,
                *symbol,
                types,
                predicate,
                visited_types,
                visited_symbols,
            ),
            Type::Conditional {
                distributive: _,
                left,
                right,
                then_type,
                else_type,
            } => {
                self.type_contains_forbidden_literal_inner(
                    module,
                    *left,
                    types,
                    predicate,
                    visited_types,
                    visited_symbols,
                ) || self.type_contains_forbidden_literal_inner(
                    module,
                    *right,
                    types,
                    predicate,
                    visited_types,
                    visited_symbols,
                ) || self.type_contains_forbidden_literal_inner(
                    module,
                    *then_type,
                    types,
                    predicate,
                    visited_types,
                    visited_symbols,
                ) || self.type_contains_forbidden_literal_inner(
                    module,
                    *else_type,
                    types,
                    predicate,
                    visited_types,
                    visited_symbols,
                )
            }
            Type::Mapped {
                parameter, value, ..
            } => {
                self.type_contains_forbidden_literal_inner(
                    module,
                    parameter.constraint,
                    types,
                    predicate,
                    visited_types,
                    visited_symbols,
                ) || parameter.key_remap.is_some_and(|key_remap| {
                    self.type_contains_forbidden_literal_inner(
                        module,
                        key_remap,
                        types,
                        predicate,
                        visited_types,
                        visited_symbols,
                    )
                }) || self.type_contains_forbidden_literal_inner(
                    module,
                    *value,
                    types,
                    predicate,
                    visited_types,
                    visited_symbols,
                )
            }
            Type::Index { left, index } => {
                self.type_contains_forbidden_literal_inner(
                    module,
                    *left,
                    types,
                    predicate,
                    visited_types,
                    visited_symbols,
                ) || self.type_contains_forbidden_literal_inner(
                    module,
                    *index,
                    types,
                    predicate,
                    visited_types,
                    visited_symbols,
                )
            }
            Type::TemplateLiteral { spans, .. } => spans.iter().any(|span| {
                self.type_contains_forbidden_literal_inner(
                    module,
                    *span,
                    types,
                    predicate,
                    visited_types,
                    visited_symbols,
                )
            }),
            Type::Import {
                static_arguments, ..
            } => static_arguments
                .as_ref()
                .map(|arguments| {
                    arguments.iter().any(|argument| {
                        self.static_argument_contains_forbidden_literal(
                            module,
                            argument,
                            types,
                            predicate,
                            visited_types,
                            visited_symbols,
                        )
                    })
                })
                .unwrap_or(false),
            Type::Infer { constraint, .. } => constraint.is_some_and(|constraint| {
                self.type_contains_forbidden_literal_inner(
                    module,
                    constraint,
                    types,
                    predicate,
                    visited_types,
                    visited_symbols,
                )
            }),
            Type::Predicate { target, .. } => target.is_some_and(|target| {
                self.type_contains_forbidden_literal_inner(
                    module,
                    target,
                    types,
                    predicate,
                    visited_types,
                    visited_symbols,
                )
            }),
            Type::Binary { left, right, .. } => {
                self.type_contains_forbidden_literal_inner(
                    module,
                    *left,
                    types,
                    predicate,
                    visited_types,
                    visited_symbols,
                ) || self.type_contains_forbidden_literal_inner(
                    module,
                    *right,
                    types,
                    predicate,
                    visited_types,
                    visited_symbols,
                )
            }
            Type::ArraySized { element, .. } => self.type_contains_forbidden_literal_inner(
                module,
                *element,
                types,
                predicate,
                visited_types,
                visited_symbols,
            ),
            Type::Array { element, .. } => element.is_some_and(|element| {
                self.type_contains_forbidden_literal_inner(
                    module,
                    element,
                    types,
                    predicate,
                    visited_types,
                    visited_symbols,
                )
            }),
            Type::Tuple { elements, .. } => elements.iter().any(|element| {
                self.type_element_contains_forbidden_literal(
                    module,
                    element,
                    types,
                    predicate,
                    visited_types,
                    visited_symbols,
                )
            }),
            Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                fields.iter().any(|field| {
                    self.type_field_contains_forbidden_literal(
                        module,
                        field,
                        types,
                        predicate,
                        visited_types,
                        visited_symbols,
                    )
                }) || call_signatures.iter().any(|signature| {
                    self.type_contains_forbidden_literal_inner(
                        module,
                        *signature,
                        types,
                        predicate,
                        visited_types,
                        visited_symbols,
                    )
                }) || construct_signatures.iter().any(|signature| {
                    self.type_contains_forbidden_literal_inner(
                        module,
                        *signature,
                        types,
                        predicate,
                        visited_types,
                        visited_symbols,
                    )
                }) || index_signatures.iter().any(|signature| {
                    self.type_index_signature_contains_forbidden_literal(
                        module,
                        signature,
                        types,
                        predicate,
                        visited_types,
                        visited_symbols,
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
                    self.type_contains_forbidden_literal_inner(
                        module,
                        *parameter,
                        types,
                        predicate,
                        visited_types,
                        visited_symbols,
                    )
                }) || this_parameter.is_some_and(|parameter| {
                    self.type_contains_forbidden_literal_inner(
                        module,
                        parameter,
                        types,
                        predicate,
                        visited_types,
                        visited_symbols,
                    )
                }) || dynamic_parameters.iter().any(|parameter| {
                    self.type_contains_forbidden_literal_inner(
                        module,
                        *parameter,
                        types,
                        predicate,
                        visited_types,
                        visited_symbols,
                    )
                }) || return_type.is_some_and(|return_type| {
                    self.type_contains_forbidden_literal_inner(
                        module,
                        return_type,
                        types,
                        predicate,
                        visited_types,
                        visited_symbols,
                    )
                })
            }
            Type::Union { elements } | Type::Intersection { elements } => {
                elements.iter().any(|element| {
                    self.type_contains_forbidden_literal_inner(
                        module,
                        *element,
                        types,
                        predicate,
                        visited_types,
                        visited_symbols,
                    )
                })
            }
            Type::InferVar { .. } | Type::Unevaluated(_) | Type::This | Type::Error => false,
        }
    }

    /// Check whether a reference symbol uses a forbidden literal.
    fn type_reference_contains_forbidden_literal(
        &self,
        module: &Module,
        symbol_id: GlobalSymbolId,
        types: &TypeTable,
        predicate: fn(&TypeLiteral) -> bool,
        visited_types: &mut HashSet<LocalTypeId>,
        visited_symbols: &mut HashSet<GlobalSymbolId>,
    ) -> bool {
        // skip remote symbols to avoid scanning other modules
        if symbol_id.module_id != module.id {
            return false;
        }

        // guard against cycles
        if !visited_symbols.insert(symbol_id) {
            return false;
        }

        // follow alias targets when present
        if let Some(alias_target) = types.get_alias_target_type_id(symbol_id) {
            return self.type_contains_forbidden_literal_inner(
                module,
                alias_target,
                types,
                predicate,
                visited_types,
                visited_symbols,
            );
        }

        // follow instance types when present
        let Some(instance_id) = types.get_instance_type_id(symbol_id) else {
            return false;
        };

        // scan the instance type
        self.type_contains_forbidden_literal_inner(
            module,
            instance_id,
            types,
            predicate,
            visited_types,
            visited_symbols,
        )
    }

    /// Check whether a tuple element uses a forbidden literal.
    fn type_element_contains_forbidden_literal(
        &self,
        module: &Module,
        element: &TypeElement,
        types: &TypeTable,
        predicate: fn(&TypeLiteral) -> bool,
        visited_types: &mut HashSet<LocalTypeId>,
        visited_symbols: &mut HashSet<GlobalSymbolId>,
    ) -> bool {
        // scan the element type
        self.type_contains_forbidden_literal_inner(
            module,
            element.ty,
            types,
            predicate,
            visited_types,
            visited_symbols,
        )
    }

    /// Check whether a field type uses a forbidden literal.
    fn type_field_contains_forbidden_literal(
        &self,
        module: &Module,
        field: &TypeField,
        types: &TypeTable,
        predicate: fn(&TypeLiteral) -> bool,
        visited_types: &mut HashSet<LocalTypeId>,
        visited_symbols: &mut HashSet<GlobalSymbolId>,
    ) -> bool {
        // scan the field type
        self.type_contains_forbidden_literal_inner(
            module,
            field.ty,
            types,
            predicate,
            visited_types,
            visited_symbols,
        )
    }

    /// Check whether an index signature uses a forbidden literal.
    fn type_index_signature_contains_forbidden_literal(
        &self,
        module: &Module,
        signature: &TypeIndexSignature,
        types: &TypeTable,
        predicate: fn(&TypeLiteral) -> bool,
        visited_types: &mut HashSet<LocalTypeId>,
        visited_symbols: &mut HashSet<GlobalSymbolId>,
    ) -> bool {
        // scan key and value types
        self.type_contains_forbidden_literal_inner(
            module,
            signature.key_type,
            types,
            predicate,
            visited_types,
            visited_symbols,
        ) || self.type_contains_forbidden_literal_inner(
            module,
            signature.value_type,
            types,
            predicate,
            visited_types,
            visited_symbols,
        )
    }

    /// Check whether a static argument uses a forbidden literal.
    fn static_argument_contains_forbidden_literal(
        &self,
        module: &Module,
        argument: &StaticArgument,
        types: &TypeTable,
        predicate: fn(&TypeLiteral) -> bool,
        visited_types: &mut HashSet<LocalTypeId>,
        visited_symbols: &mut HashSet<GlobalSymbolId>,
    ) -> bool {
        // ignore unevaluated static arguments
        let StaticArgument::Evaluated { value, .. } = argument else {
            return false;
        };

        // scan the evaluated argument value
        self.static_expression_contains_forbidden_literal(
            module,
            value,
            types,
            predicate,
            visited_types,
            visited_symbols,
        )
    }

    /// Check whether a static expression uses a forbidden literal.
    fn static_expression_contains_forbidden_literal(
        &self,
        module: &Module,
        expression: &StaticExpression,
        types: &TypeTable,
        predicate: fn(&TypeLiteral) -> bool,
        visited_types: &mut HashSet<LocalTypeId>,
        visited_symbols: &mut HashSet<GlobalSymbolId>,
    ) -> bool {
        // walk the static expression tree
        match expression {
            StaticExpression::TypeLiteral { value } => predicate(value),
            StaticExpression::Type { ty } => self.type_contains_forbidden_literal_inner(
                module,
                *ty,
                types,
                predicate,
                visited_types,
                visited_symbols,
            ),
            StaticExpression::RangeExpression { start, end, .. } => {
                self.static_expression_contains_forbidden_literal(
                    module,
                    start,
                    types,
                    predicate,
                    visited_types,
                    visited_symbols,
                ) || self.static_expression_contains_forbidden_literal(
                    module,
                    end,
                    types,
                    predicate,
                    visited_types,
                    visited_symbols,
                )
            }
            StaticExpression::ArrayExpression { elements } => elements.iter().any(|element| {
                self.static_expression_contains_forbidden_literal(
                    module,
                    element,
                    types,
                    predicate,
                    visited_types,
                    visited_symbols,
                )
            }),
            StaticExpression::TupleExpression { elements } => elements.iter().any(|element| {
                self.static_expression_contains_forbidden_literal(
                    module,
                    element,
                    types,
                    predicate,
                    visited_types,
                    visited_symbols,
                )
            }),
            StaticExpression::ObjectExpression { properties } => {
                properties.iter().any(|property| {
                    self.static_property_contains_forbidden_literal(
                        module,
                        property,
                        types,
                        predicate,
                        visited_types,
                        visited_symbols,
                    )
                })
            }
            StaticExpression::Declaration {
                static_arguments, ..
            } => static_arguments
                .as_ref()
                .map(|arguments| {
                    arguments.iter().any(|argument| {
                        self.static_argument_contains_forbidden_literal(
                            module,
                            argument,
                            types,
                            predicate,
                            visited_types,
                            visited_symbols,
                        )
                    })
                })
                .unwrap_or(false),
            StaticExpression::ScalarLiteral { .. } | StaticExpression::Unevaluated { .. } => false,
        }
    }

    /// Check whether a static property uses a forbidden literal.
    fn static_property_contains_forbidden_literal(
        &self,
        module: &Module,
        property: &StaticProperty,
        types: &TypeTable,
        predicate: fn(&TypeLiteral) -> bool,
        visited_types: &mut HashSet<LocalTypeId>,
        visited_symbols: &mut HashSet<GlobalSymbolId>,
    ) -> bool {
        // scan the static property payload
        match property {
            StaticProperty::Field { value, default, .. } => {
                self.static_expression_contains_forbidden_literal(
                    module,
                    value,
                    types,
                    predicate,
                    visited_types,
                    visited_symbols,
                ) || default.as_ref().is_some_and(|default| {
                    self.static_expression_contains_forbidden_literal(
                        module,
                        default,
                        types,
                        predicate,
                        visited_types,
                        visited_symbols,
                    )
                })
            }
            StaticProperty::Method { body, .. } => self
                .static_expression_contains_forbidden_literal(
                    module,
                    body,
                    types,
                    predicate,
                    visited_types,
                    visited_symbols,
                ),
            StaticProperty::Unevaluated { .. } => false,
        }
    }
}

/// Return true when the literal is `any`.
fn literal_is_any(value: &TypeLiteral) -> bool {
    matches!(value, TypeLiteral::Any)
}

/// Return true when the literal is `unknown`.
fn literal_is_unknown(value: &TypeLiteral) -> bool {
    matches!(value, TypeLiteral::Unknown)
}

/// Return true when the literal is an imprecise primitive.
fn literal_is_imprecise_primitive(value: &TypeLiteral) -> bool {
    matches!(value, TypeLiteral::Primitive(PrimitiveType::Number))
}
