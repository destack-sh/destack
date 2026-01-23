use std::collections::HashSet;

use destack_dir::{
    Expression, GlobalSymbolId, LocalNodeId, LocalNodeIdAny, LocalTypeId, NodeTree, StaticArgument,
    StaticExpression, StaticProperty, SymbolTable, SymbolType, Type, TypeLiteral, TypeTable,
};
use destack_workspace::{Module, ModuleDir, ProfileId};

use crate::{AnalyzeResult, Compiler};

impl Compiler {
    /// Return true when a type is wrapped in explicit ownership modifiers.
    pub(crate) fn type_is_explicit_ownership_wrapper(
        &self,
        types: &TypeTable,
        type_id: LocalTypeId,
    ) -> bool {
        match types.get_type(type_id) {
            Type::ValueOf { .. } | Type::ReferenceOf { .. } | Type::PointerOf { .. } => true,
            Type::Value { value } => self.type_is_explicit_ownership_wrapper(types, *value),
            _ => false,
        }
    }

    /// Evaluate a type id in place when it is unevaluated.
    pub(crate) fn evaluate_unevaluated_type(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        if matches!(types.get_type(type_id), Type::Unevaluated(_)) {
            self.evaluate_type(module, profile, type_id, tree, symbols, types)?;
        }
        Ok(type_id)
    }

    /// Resolve a type symbol from a type reference or type-as-value.
    pub(crate) fn unwrap_type_value_symbol(
        &self,
        types: &TypeTable,
        type_id: LocalTypeId,
    ) -> Option<GlobalSymbolId> {
        // unwrap direct references
        if let Type::Reference { symbol, .. } = types.get_type(type_id) {
            return Some(*symbol);
        }

        // unwrap references stored in type-as-value wrappers
        if let Type::Value { value } = types.get_type(type_id)
            && let Type::Reference { symbol, .. } = types.get_type(*value)
        {
            return Some(*symbol);
        }

        None
    }

    /// Align a symbol id with the stored symbol type.
    pub(crate) fn typed_symbol_id(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        symbols: &SymbolTable,
    ) -> GlobalSymbolId {
        if symbol.module_id == module.id {
            let symbol_entry = symbols.get_symbol(symbol.local_id);
            return GlobalSymbolId::new(
                symbol.module_id,
                symbol.local_id.with_type(symbol_entry.ty),
            );
        }

        let remote_module = self.program.modules.get(symbol.module_id);
        let remote_module = remote_module.read();
        let remote_symbols = remote_module.dir(profile).symbols.read();
        let symbol_entry = remote_symbols.get_symbol(symbol.local_id);
        GlobalSymbolId::new(symbol.module_id, symbol.local_id.with_type(symbol_entry.ty))
    }

    /// Import the alias target type for a symbol when available.
    pub(crate) fn alias_target_type_id_for_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        source_id: LocalNodeIdAny,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        // load the local alias target when the symbol is local
        if symbol.module_id == module.id {
            let symbol_entry = symbols.get_symbol(symbol.local_id);
            if !matches!(symbol_entry.ty, SymbolType::TypeAlias | SymbolType::Newtype) {
                return None;
            }

            let typed_symbol =
                GlobalSymbolId::new(symbol.module_id, symbol.local_id.with_type(symbol_entry.ty));
            return types.get_alias_target_type_id(typed_symbol);
        }

        // import the alias target when the symbol is remote
        let remote_module = self.program.modules.get(symbol.module_id);
        let remote_module = remote_module.read();
        let remote_symbols = remote_module.dir(profile).symbols.read();
        let symbol_entry = remote_symbols.get_symbol(symbol.local_id);
        if !matches!(symbol_entry.ty, SymbolType::TypeAlias | SymbolType::Newtype) {
            return None;
        }

        let typed_symbol =
            GlobalSymbolId::new(symbol.module_id, symbol.local_id.with_type(symbol_entry.ty));

        // load the remote alias target, evaluating when needed
        let remote_dir = remote_module.dir(profile);
        let mut remote_types = remote_dir.types.write();
        let alias_target_id = remote_types.get_alias_target_type_id(typed_symbol)?;
        if let Type::Unevaluated(expression_id) = *remote_types.get_type(alias_target_id) {
            self.try_evaluate_remote_alias_target(
                &remote_module,
                profile,
                alias_target_id,
                expression_id,
                remote_dir,
                &mut remote_types,
            );
        }
        let alias_target_ty = remote_types.get_type(alias_target_id);
        Some(self.import_type_from_remote_for_node(
            source_id,
            alias_target_ty,
            &remote_types,
            typed_symbol,
            types,
        ))
    }

    /// Try to evaluate a remote alias target expression in place.
    fn try_evaluate_remote_alias_target(
        &self,
        module: &Module,
        profile: ProfileId,
        alias_target_id: LocalTypeId,
        expression_id: LocalNodeId<Expression>,
        dir: &ModuleDir,
        types: &mut TypeTable,
    ) {
        // evaluate the alias target when possible
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();
        match self.try_evaluate_expression_to_type_value(
            module,
            profile,
            expression_id,
            &tree,
            &symbols,
            types,
            false,
        ) {
            Ok(evaluated_ty) => {
                // cache the evaluated target for normalization
                let ty = types.get_type_mut(alias_target_id);
                *ty = evaluated_ty;
                types.invalidate_normalization_cache();
            }
            Err(_) => {
                // ignore failures to avoid cascading errors in callers
            }
        }
    }

    /// Unwrap a type-as-value wrapper to the underlying type id.
    pub(crate) fn unwrap_type_value(&self, type_id: LocalTypeId, types: &TypeTable) -> LocalTypeId {
        match types.get_type(type_id) {
            Type::Value { value } => *value,
            _ => type_id,
        }
    }

    /// Resolve an enum symbol from a type when possible.
    pub(crate) fn enum_symbol_for_type(
        &self,
        ty: &Type,
        types: &TypeTable,
    ) -> Option<GlobalSymbolId> {
        match ty {
            Type::Reference { symbol, .. } if symbol.ty() == SymbolType::Enum => Some(*symbol),
            Type::Value { value } => {
                let inner = types.get_type(*value);
                self.enum_symbol_for_type(inner, types)
            }
            Type::Intersection { elements } => elements.iter().find_map(|element_id| {
                let element_ty = types.get_type(*element_id);
                self.enum_symbol_for_type(element_ty, types)
            }),
            _ => None,
        }
    }

    /// Build a union type from two type ids.
    pub(crate) fn union_type(
        &self,
        left: LocalTypeId,
        right: LocalTypeId,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // reuse the left source for the combined union
        self.union_type_from_list(vec![left, right], left, types)
    }

    /// Build a union type from a list of elements.
    pub(crate) fn union_type_from_list(
        &self,
        elements: Vec<LocalTypeId>,
        source_type_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // flatten nested unions and keep elements unique
        let mut flattened = Vec::new();
        for element_id in elements {
            match types.get_type(element_id) {
                Type::Union { elements: union } => {
                    for element_id in union {
                        if !flattened.contains(element_id) {
                            flattened.push(*element_id);
                        }
                    }
                }
                _ => {
                    if !flattened.contains(&element_id) {
                        flattened.push(element_id);
                    }
                }
            }
        }

        // collapse any or unknown and remove never
        let mut any_type = None;
        let mut unknown_type = None;
        let mut never_type = None;
        let mut filtered = Vec::new();
        for element_id in flattened {
            match types.get_type(element_id) {
                Type::TypeLiteral {
                    value: TypeLiteral::Any,
                } => any_type = Some(element_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                } => unknown_type = Some(element_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Never,
                } => never_type = Some(element_id),
                _ => filtered.push(element_id),
            }
        }

        // honor dominating any or unknown
        if let Some(any_type) = any_type {
            return any_type;
        }
        if let Some(unknown_type) = unknown_type {
            return unknown_type;
        }

        // fall back to never when the union is empty
        if filtered.is_empty() {
            return never_type.unwrap_or_else(|| {
                types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::Never,
                    },
                    types.get_type_source(source_type_id),
                )
            });
        }

        // avoid rebuilding when a single element remains
        if filtered.len() == 1 {
            return filtered[0];
        }

        // construct the union type
        let union = Type::Union { elements: filtered };
        types.insert_type_from_any(union, types.get_type_source(source_type_id))
    }

    /// Build an intersection type from a list of elements.
    pub(crate) fn intersection_type_from_list(
        &self,
        elements: Vec<LocalTypeId>,
        source_type_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // flatten nested intersections and keep elements unique
        let mut flattened = Vec::new();
        for element_id in elements {
            match types.get_type(element_id) {
                Type::Intersection { elements } => {
                    for element_id in elements {
                        if !flattened.contains(element_id) {
                            flattened.push(*element_id);
                        }
                    }
                }
                _ => {
                    if !flattened.contains(&element_id) {
                        flattened.push(element_id);
                    }
                }
            }
        }

        // collapse any or unknown and handle never
        let mut any_type = None;
        let mut unknown_type = None;
        let mut never_type = None;
        let mut filtered = Vec::new();
        for element_id in flattened {
            match types.get_type(element_id) {
                Type::TypeLiteral {
                    value: TypeLiteral::Any,
                } => any_type = Some(element_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                } => unknown_type = Some(element_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Never,
                } => never_type = Some(element_id),
                _ => filtered.push(element_id),
            }
        }

        // honor dominating never or any
        if let Some(never_type) = never_type {
            return never_type;
        }
        if let Some(any_type) = any_type {
            return any_type;
        }

        // fall back to unknown when the intersection is empty
        if filtered.is_empty() {
            return unknown_type.unwrap_or_else(|| {
                types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    },
                    types.get_type_source(source_type_id),
                )
            });
        }

        // avoid rebuilding when a single element remains
        if filtered.len() == 1 {
            return filtered[0];
        }

        // construct the intersection type
        let intersection = Type::Intersection { elements: filtered };
        types.insert_type_from_any(intersection, types.get_type_source(source_type_id))
    }

    /// Check whether a type contains an error type.
    pub(crate) fn type_contains_error(
        &self,
        ty_id: LocalTypeId,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        // avoid infinite recursion on self referential types
        if !visited.insert(ty_id) {
            return false;
        }

        let ty = types.get_type(ty_id).clone();
        let contains_error = match ty {
            Type::Error => true,
            Type::Reference {
                static_arguments, ..
            } => static_arguments.as_ref().is_some_and(|arguments| {
                arguments
                    .iter()
                    .any(|argument| self.static_argument_contains_error(argument, types, visited))
            }),
            Type::Value { value } => self.type_contains_error(value, types, visited),
            Type::Unary { right, .. }
            | Type::ValueOf { right, .. }
            | Type::ReferenceOf { right, .. }
            | Type::PointerOf { right, .. }
            | Type::Infer {
                constraint: Some(right),
                ..
            } => self.type_contains_error(right, types, visited),
            Type::Conditional {
                left,
                right,
                then_type,
                else_type,
                ..
            } => {
                self.type_contains_error(left, types, visited)
                    || self.type_contains_error(right, types, visited)
                    || self.type_contains_error(then_type, types, visited)
                    || self.type_contains_error(else_type, types, visited)
            }
            Type::Binary { left, right, .. } => {
                self.type_contains_error(left, types, visited)
                    || self.type_contains_error(right, types, visited)
            }
            Type::Mapped { value, .. } => self.type_contains_error(value, types, visited),
            Type::Index { left, index } => {
                self.type_contains_error(left, types, visited)
                    || self.type_contains_error(index, types, visited)
            }
            Type::TemplateLiteral { spans, .. } => spans
                .iter()
                .any(|span| self.type_contains_error(*span, types, visited)),
            Type::Import {
                static_arguments, ..
            } => static_arguments.as_ref().is_some_and(|arguments| {
                arguments
                    .iter()
                    .any(|argument| self.static_argument_contains_error(argument, types, visited))
            }),
            Type::ArraySized { element, .. }
            | Type::Array {
                element: Some(element),
                ..
            } => self.type_contains_error(element, types, visited),
            Type::Tuple { elements, .. } => elements
                .iter()
                .any(|element| self.type_contains_error(element.ty, types, visited)),
            Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                fields
                    .iter()
                    .any(|field| self.type_contains_error(field.ty, types, visited))
                    || call_signatures
                        .iter()
                        .any(|signature| self.type_contains_error(*signature, types, visited))
                    || construct_signatures
                        .iter()
                        .any(|signature| self.type_contains_error(*signature, types, visited))
                    || index_signatures.iter().any(|signature| {
                        self.type_contains_error(signature.value_type, types, visited)
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
                    .any(|parameter| self.type_contains_error(*parameter, types, visited))
                    || this_parameter.is_some_and(|parameter| {
                        self.type_contains_error(parameter, types, visited)
                    })
                    || dynamic_parameters
                        .iter()
                        .any(|parameter| self.type_contains_error(*parameter, types, visited))
                    || return_type.is_some_and(|return_type| {
                        self.type_contains_error(return_type, types, visited)
                    })
            }
            Type::Union { elements } | Type::Intersection { elements } => elements
                .iter()
                .any(|element| self.type_contains_error(*element, types, visited)),
            Type::Array { element: None, .. }
            | Type::TypeLiteral { .. }
            | Type::InferVar { .. }
            | Type::Unevaluated(_)
            | Type::Infer {
                constraint: None, ..
            }
            | Type::Predicate { .. }
            | Type::This => false,
        };

        visited.remove(&ty_id);
        contains_error
    }

    /// Check whether a type contains unevaluated static arguments.
    pub(crate) fn type_contains_unevaluated_static_arguments(
        &self,
        ty_id: LocalTypeId,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        // avoid infinite recursion on self referential types
        if !visited.insert(ty_id) {
            return false;
        }

        let ty = types.get_type(ty_id).clone();
        let contains_unevaluated = match ty {
            Type::Reference {
                static_arguments, ..
            } => static_arguments.as_ref().is_some_and(|arguments| {
                arguments.iter().any(|argument| match argument {
                    StaticArgument::Unevaluated { .. } => true,
                    StaticArgument::Evaluated { value, .. } => self
                        .static_expression_contains_unevaluated_static_arguments(
                            value, types, visited,
                        ),
                })
            }),
            Type::Value { value } => {
                self.type_contains_unevaluated_static_arguments(value, types, visited)
            }
            Type::Unary { right, .. }
            | Type::ValueOf { right, .. }
            | Type::ReferenceOf { right, .. }
            | Type::PointerOf { right, .. }
            | Type::Infer {
                constraint: Some(right),
                ..
            } => self.type_contains_unevaluated_static_arguments(right, types, visited),
            Type::Conditional {
                left,
                right,
                then_type,
                else_type,
                ..
            } => {
                self.type_contains_unevaluated_static_arguments(left, types, visited)
                    || self.type_contains_unevaluated_static_arguments(right, types, visited)
                    || self.type_contains_unevaluated_static_arguments(then_type, types, visited)
                    || self.type_contains_unevaluated_static_arguments(else_type, types, visited)
            }
            Type::Binary { left, right, .. } => {
                self.type_contains_unevaluated_static_arguments(left, types, visited)
                    || self.type_contains_unevaluated_static_arguments(right, types, visited)
            }
            Type::Mapped { value, .. } => {
                self.type_contains_unevaluated_static_arguments(value, types, visited)
            }
            Type::Index { left, index } => {
                self.type_contains_unevaluated_static_arguments(left, types, visited)
                    || self.type_contains_unevaluated_static_arguments(index, types, visited)
            }
            Type::TemplateLiteral { spans, .. } => spans
                .iter()
                .any(|span| self.type_contains_unevaluated_static_arguments(*span, types, visited)),
            Type::Import {
                static_arguments, ..
            } => static_arguments.as_ref().is_some_and(|arguments| {
                arguments.iter().any(|argument| match argument {
                    StaticArgument::Unevaluated { .. } => true,
                    StaticArgument::Evaluated { value, .. } => self
                        .static_expression_contains_unevaluated_static_arguments(
                            value, types, visited,
                        ),
                })
            }),
            Type::ArraySized { element, .. }
            | Type::Array {
                element: Some(element),
                ..
            } => self.type_contains_unevaluated_static_arguments(element, types, visited),
            Type::Tuple { elements, .. } => elements.iter().any(|element| {
                self.type_contains_unevaluated_static_arguments(element.ty, types, visited)
            }),
            Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                fields.iter().any(|field| {
                    self.type_contains_unevaluated_static_arguments(field.ty, types, visited)
                }) || call_signatures.iter().any(|signature| {
                    self.type_contains_unevaluated_static_arguments(*signature, types, visited)
                }) || construct_signatures.iter().any(|signature| {
                    self.type_contains_unevaluated_static_arguments(*signature, types, visited)
                }) || index_signatures.iter().any(|signature| {
                    self.type_contains_unevaluated_static_arguments(
                        signature.key_type,
                        types,
                        visited,
                    ) || self.type_contains_unevaluated_static_arguments(
                        signature.value_type,
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
                    self.type_contains_unevaluated_static_arguments(*parameter, types, visited)
                }) || this_parameter.is_some_and(|parameter| {
                    self.type_contains_unevaluated_static_arguments(parameter, types, visited)
                }) || dynamic_parameters.iter().any(|parameter| {
                    self.type_contains_unevaluated_static_arguments(*parameter, types, visited)
                }) || return_type.is_some_and(|return_type| {
                    self.type_contains_unevaluated_static_arguments(return_type, types, visited)
                })
            }
            Type::Union { elements } | Type::Intersection { elements } => {
                elements.iter().any(|element| {
                    self.type_contains_unevaluated_static_arguments(*element, types, visited)
                })
            }
            Type::Array { element: None, .. }
            | Type::TypeLiteral { .. }
            | Type::InferVar { .. }
            | Type::Unevaluated(_)
            | Type::Infer {
                constraint: None, ..
            }
            | Type::Predicate { .. }
            | Type::This
            | Type::Error => false,
        };

        visited.remove(&ty_id);
        contains_unevaluated
    }

    /// Check whether a static expression contains unevaluated static arguments.
    fn static_expression_contains_unevaluated_static_arguments(
        &self,
        expression: &StaticExpression,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        match expression {
            StaticExpression::Unevaluated { .. } => true,
            StaticExpression::ScalarLiteral { .. } => false,
            StaticExpression::TypeLiteral { .. } => false,
            StaticExpression::Type { ty } => {
                self.type_contains_unevaluated_static_arguments(*ty, types, visited)
            }
            StaticExpression::Declaration {
                static_arguments, ..
            } => static_arguments.as_ref().is_some_and(|arguments| {
                arguments.iter().any(|argument| match argument {
                    StaticArgument::Unevaluated { .. } => true,
                    StaticArgument::Evaluated { value, .. } => self
                        .static_expression_contains_unevaluated_static_arguments(
                            value, types, visited,
                        ),
                })
            }),
            StaticExpression::RangeExpression { start, end, .. } => {
                self.static_expression_contains_unevaluated_static_arguments(start, types, visited)
                    || self.static_expression_contains_unevaluated_static_arguments(
                        end, types, visited,
                    )
            }
            StaticExpression::ArrayExpression { elements }
            | StaticExpression::TupleExpression { elements } => elements.iter().any(|element| {
                self.static_expression_contains_unevaluated_static_arguments(
                    element, types, visited,
                )
            }),
            StaticExpression::ObjectExpression { properties } => {
                properties.iter().any(|property| match property {
                    StaticProperty::Unevaluated { .. } => false,
                    StaticProperty::Field { value, default, .. } => {
                        self.static_expression_contains_unevaluated_static_arguments(
                            value, types, visited,
                        ) || default.as_ref().is_some_and(|default| {
                            self.static_expression_contains_unevaluated_static_arguments(
                                default, types, visited,
                            )
                        })
                    }
                    StaticProperty::Method { body, .. } => self
                        .static_expression_contains_unevaluated_static_arguments(
                            body, types, visited,
                        ),
                })
            }
        }
    }

    /// Check whether a static argument contains an error type.
    pub(crate) fn static_argument_contains_error(
        &self,
        argument: &StaticArgument,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        match argument {
            StaticArgument::Unevaluated { .. } => false,
            StaticArgument::Evaluated { value, .. } => {
                self.static_expression_contains_error(value, types, visited)
            }
        }
    }

    /// Check whether a static expression contains an error type.
    pub(crate) fn static_expression_contains_error(
        &self,
        expression: &StaticExpression,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        match expression {
            StaticExpression::Type { ty } => self.type_contains_error(*ty, types, visited),
            StaticExpression::Declaration {
                static_arguments, ..
            } => static_arguments.as_ref().is_some_and(|arguments| {
                arguments
                    .iter()
                    .any(|argument| self.static_argument_contains_error(argument, types, visited))
            }),
            StaticExpression::ArrayExpression { elements }
            | StaticExpression::TupleExpression { elements } => elements
                .iter()
                .any(|element| self.static_expression_contains_error(element, types, visited)),
            StaticExpression::ObjectExpression { properties } => {
                properties.iter().any(|property| match property {
                    StaticProperty::Unevaluated { .. } => false,
                    StaticProperty::Field { value, default, .. } => {
                        self.static_expression_contains_error(value, types, visited)
                            || default.as_ref().is_some_and(|default| {
                                self.static_expression_contains_error(default, types, visited)
                            })
                    }
                    StaticProperty::Method { body, .. } => {
                        self.static_expression_contains_error(body, types, visited)
                    }
                })
            }
            StaticExpression::Unevaluated { .. }
            | StaticExpression::ScalarLiteral { .. }
            | StaticExpression::TypeLiteral { .. }
            | StaticExpression::RangeExpression { .. } => false,
        }
    }
}
