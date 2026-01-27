use std::collections::HashSet;

use destack_dir::{
    GlobalSymbolId, LocalNodeIdAny, LocalTypeId, NodeTree, StaticArgument, StaticExpression,
    StaticParameterKind, StaticProperty, SymbolTable, SymbolType, Type, TypeLiteral, TypeTable,
};
use destack_workspace::{Module, ProfileId};

use super::CanonicalSymbolMode;
use crate::{AnalyzeResult, Compiler};

#[allow(clippy::too_many_arguments)]
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
        let mut visited = HashSet::new();
        let mut current = symbol;

        loop {
            if !visited.insert(current) {
                return None;
            }

            // load the local alias target when the symbol is local
            if current.module_id == module.id {
                let symbol_entry = symbols.get_symbol(current.local_id);
                if matches!(symbol_entry.ty, SymbolType::TypeAlias | SymbolType::Newtype) {
                    let typed_symbol = GlobalSymbolId::new(
                        current.module_id,
                        current.local_id.with_type(symbol_entry.ty),
                    );
                    if let Some(target) = types.get_alias_target_type_id(typed_symbol) {
                        return Some(target);
                    }
                }

                // follow import targets for local alias references
                let target_symbol = symbol_entry
                    .target_symbol
                    .or(symbol_entry.canonical_symbol)?;
                current = target_symbol;
                continue;
            }

            // import the alias target when the symbol is remote
            if current.module_id != module.id {
                let _ = self.require_analyze_module_declare(current.module_id, profile);
            }
            let remote_module = self.program.modules.get(current.module_id);
            let remote_module = remote_module.read();
            let remote_dir = remote_module.dir(profile);
            let remote_tree = remote_dir.tree.read();
            let remote_symbols = remote_dir.symbols.read();
            let symbol_entry = remote_symbols.get_symbol(current.local_id);
            if !matches!(symbol_entry.ty, SymbolType::TypeAlias | SymbolType::Newtype) {
                // follow remote import targets when present
                let target_symbol = symbol_entry
                    .target_symbol
                    .or(symbol_entry.canonical_symbol)?;
                current = target_symbol;
                continue;
            }

            let typed_symbol = GlobalSymbolId::new(
                current.module_id,
                current.local_id.with_type(symbol_entry.ty),
            );

            // import remote alias targets from the export summary
            let mut remote_types = remote_dir.types.write();
            let remote_target_id = remote_types.get_alias_target_type_id(typed_symbol)?;

            // evaluate remote alias targets before importing
            if matches!(
                remote_types.get_type(remote_target_id),
                Type::Unevaluated(_)
            ) && let Err(error) = self.evaluate_type(
                &remote_module,
                profile,
                remote_target_id,
                &remote_tree,
                &remote_symbols,
                &mut remote_types,
            ) {
                self.error(error);
                return None;
            }

            // skip alias targets that still need value materialization
            let needs_materialization = self.type_contains_unevaluated_value_static_arguments(
                &remote_module,
                profile,
                remote_target_id,
                &remote_tree,
                &remote_symbols,
                &remote_types,
                &mut HashSet::new(),
            );
            if needs_materialization {
                return None;
            }

            let remote_target_ty = remote_types.get_type(remote_target_id);
            let local_alias_target_id = self.import_type_from_remote_for_node(
                source_id,
                remote_target_ty,
                &remote_types,
                typed_symbol,
                types,
            );
            types.set_alias_target_type_id(typed_symbol, local_alias_target_id);
            return Some(local_alias_target_id);
        }
    }

    /// Require an instance type for a symbol into the local type table.
    pub(crate) fn require_instance_type(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        // resolve through canonical import targets while preserving aliases
        let symbol = if symbol.ty() == SymbolType::Extension {
            symbol
        } else {
            self.canonical_symbol_id(
                module,
                symbols,
                profile,
                symbol,
                CanonicalSymbolMode::PreserveAliases,
            )
        };

        // reuse local instance types when already available
        if let Some(instance_id) = types.get_instance_type_id(symbol) {
            return Some(instance_id);
        }

        // load or import the instance type through the existing resolver
        match self.resolve_instance_type_for_symbol(module, profile, source_id, symbol, types) {
            Ok(instance_id) => instance_id,
            Err(error) => {
                self.error(error);
                None
            }
        }
    }

    /// Resolve the apparent instance type for shape queries like `keyof`.
    pub(crate) fn apparent_instance_type(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        // resolve through canonical import targets while preserving aliases
        let symbol = if symbol.ty() == SymbolType::Extension {
            symbol
        } else {
            self.canonical_symbol_id(
                module,
                symbols,
                profile,
                symbol,
                CanonicalSymbolMode::PreserveAliases,
            )
        };

        // prefer alias targets as the apparent type when available
        if let Some(alias_target_id) =
            self.alias_target_type_id_for_symbol(module, profile, symbol, source_id, symbols, types)
        {
            return Some(alias_target_id);
        }

        // otherwise fall back to the instance type
        self.require_instance_type(module, profile, source_id, symbol, symbols, types)
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

    pub(crate) fn type_contains_unevaluated_value_static_arguments(
        &self,
        module: &Module,
        profile: ProfileId,
        ty_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        // avoid infinite recursion on self referential types
        if !visited.insert(ty_id) {
            return false;
        }

        // check for unevaluated value static arguments in this type
        let has_unevaluated = match types.get_type(ty_id).clone() {
            Type::Reference {
                symbol,
                static_arguments,
            } => self.reference_contains_unevaluated_value_arguments(
                module,
                profile,
                symbol,
                static_arguments.as_deref(),
                tree,
                symbols,
                types,
                visited,
            ),
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
            Type::Value { value } => self.type_contains_unevaluated_value_static_arguments(
                module, profile, value, tree, symbols, types, visited,
            ),
            Type::Unary { right, .. }
            | Type::ValueOf { right, .. }
            | Type::ReferenceOf { right, .. }
            | Type::PointerOf { right, .. }
            | Type::Infer {
                constraint: Some(right),
                ..
            } => self.type_contains_unevaluated_value_static_arguments(
                module, profile, right, tree, symbols, types, visited,
            ),
            Type::Conditional {
                left,
                right,
                then_type,
                else_type,
                ..
            } => {
                self.type_contains_unevaluated_value_static_arguments(
                    module, profile, left, tree, symbols, types, visited,
                ) || self.type_contains_unevaluated_value_static_arguments(
                    module, profile, right, tree, symbols, types, visited,
                ) || self.type_contains_unevaluated_value_static_arguments(
                    module, profile, then_type, tree, symbols, types, visited,
                ) || self.type_contains_unevaluated_value_static_arguments(
                    module, profile, else_type, tree, symbols, types, visited,
                )
            }
            Type::Binary { left, right, .. } => {
                self.type_contains_unevaluated_value_static_arguments(
                    module, profile, left, tree, symbols, types, visited,
                ) || self.type_contains_unevaluated_value_static_arguments(
                    module, profile, right, tree, symbols, types, visited,
                )
            }
            Type::Mapped { value, .. } => self.type_contains_unevaluated_value_static_arguments(
                module, profile, value, tree, symbols, types, visited,
            ),
            Type::Index { left, index } => {
                self.type_contains_unevaluated_value_static_arguments(
                    module, profile, left, tree, symbols, types, visited,
                ) || self.type_contains_unevaluated_value_static_arguments(
                    module, profile, index, tree, symbols, types, visited,
                )
            }
            Type::TemplateLiteral { spans, .. } => spans.iter().any(|span| {
                self.type_contains_unevaluated_value_static_arguments(
                    module, profile, *span, tree, symbols, types, visited,
                )
            }),
            Type::ArraySized { element, .. }
            | Type::Array {
                element: Some(element),
                ..
            } => self.type_contains_unevaluated_value_static_arguments(
                module, profile, element, tree, symbols, types, visited,
            ),
            Type::Tuple { elements, .. } => elements.iter().any(|element| {
                self.type_contains_unevaluated_value_static_arguments(
                    module, profile, element.ty, tree, symbols, types, visited,
                )
            }),
            Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                fields.iter().any(|field| {
                    self.type_contains_unevaluated_value_static_arguments(
                        module, profile, field.ty, tree, symbols, types, visited,
                    )
                }) || call_signatures.iter().any(|signature| {
                    self.type_contains_unevaluated_value_static_arguments(
                        module, profile, *signature, tree, symbols, types, visited,
                    )
                }) || construct_signatures.iter().any(|signature| {
                    self.type_contains_unevaluated_value_static_arguments(
                        module, profile, *signature, tree, symbols, types, visited,
                    )
                }) || index_signatures.iter().any(|signature| {
                    self.type_contains_unevaluated_value_static_arguments(
                        module,
                        profile,
                        signature.key_type,
                        tree,
                        symbols,
                        types,
                        visited,
                    ) || self.type_contains_unevaluated_value_static_arguments(
                        module,
                        profile,
                        signature.value_type,
                        tree,
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
                    self.type_contains_unevaluated_value_static_arguments(
                        module, profile, *parameter, tree, symbols, types, visited,
                    )
                }) || this_parameter.is_some_and(|parameter| {
                    self.type_contains_unevaluated_value_static_arguments(
                        module, profile, parameter, tree, symbols, types, visited,
                    )
                }) || dynamic_parameters.iter().any(|parameter| {
                    self.type_contains_unevaluated_value_static_arguments(
                        module, profile, *parameter, tree, symbols, types, visited,
                    )
                }) || return_type.is_some_and(|return_type| {
                    self.type_contains_unevaluated_value_static_arguments(
                        module,
                        profile,
                        return_type,
                        tree,
                        symbols,
                        types,
                        visited,
                    )
                })
            }
            Type::Union { elements } | Type::Intersection { elements } => {
                elements.iter().any(|element| {
                    self.type_contains_unevaluated_value_static_arguments(
                        module, profile, *element, tree, symbols, types, visited,
                    )
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
        has_unevaluated
    }

    fn reference_contains_unevaluated_value_arguments(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        arguments: Option<&[StaticArgument]>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let Some(arguments) = arguments else {
            return false;
        };

        // no arguments means no value materialization is needed
        if arguments.is_empty() {
            return false;
        }

        // resolve parameter kinds for the referenced declaration
        let Some(parameter_symbols) =
            self.collect_static_parameter_symbols(module, symbol, profile, tree, symbols)
        else {
            return false;
        };

        for (index, argument) in arguments.iter().enumerate() {
            let kind = parameter_symbols
                .get(index)
                .map(|parameter_symbol| {
                    if parameter_symbol.module_id == module.id {
                        self.static_parameter_kind_for_symbol_in_module(
                            *parameter_symbol,
                            tree,
                            symbols,
                        )
                    } else {
                        let remote_module = self.program.modules.get(parameter_symbol.module_id);
                        let remote_module = remote_module.read();
                        let remote_tree = remote_module.dir(profile).tree.read();
                        let remote_symbols = remote_module.dir(profile).symbols.read();
                        self.static_parameter_kind_for_symbol_in_module(
                            *parameter_symbol,
                            &remote_tree,
                            &remote_symbols,
                        )
                    }
                })
                .unwrap_or(StaticParameterKind::Type);

            // only value parameters require unevaluated materialization
            if kind != StaticParameterKind::Value {
                continue;
            }

            let has_unevaluated = match argument {
                StaticArgument::Unevaluated { .. } => true,
                StaticArgument::Evaluated { value, .. } => self
                    .static_expression_contains_unevaluated_static_arguments(value, types, visited),
            };
            if has_unevaluated {
                return true;
            }
        }

        false
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
