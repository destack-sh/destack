use std::collections::{HashMap, HashSet};

use super::{index_key_kind_for_member, index_key_kind_for_type, index_key_kinds_compatible};
use crate::{
    AnalyzeError, AnalyzeOptions, AnalyzeResult, Assignability, Compiler, TaskDependencyError,
};
use destack_builtin::LanguageItem;
use destack_dir::{
    BinaryOperator, Declaration, DeclarationType, Expression, Extension, ExtensionKind,
    GlobalSymbolId, IntType, LocalNodeId, LocalNodeIdAny, LocalTypeId, Mutability, NodeTree,
    PrimitiveType, ScalarLiteral, StaticArgument, StaticExpression, StaticKey, StaticProperty,
    StringId, SymbolTable, SymbolType, Type, TypeBinaryOperator, TypeField, TypeIndexSignature,
    TypeLiteral, TypeMappedParameter, TypeTable, TypeUnaryOperator, UnaryOperator, VarianceBound,
    WellKnownSymbol,
};
use destack_workspace::{Module, ProfileId};

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
    /// Resolve a typeof guard target for a string literal.
    pub(super) fn type_guard_target_for_typeof_string(
        &self,
        string_id: StringId,
        types: &mut TypeTable,
    ) -> Option<TypeGuardTarget> {
        match self.program.strings.get(string_id).as_ref() {
            "string" => Some(TypeGuardTarget::TypeId(types.insert_type(
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::String),
                },
            ))),
            "number" => Some(TypeGuardTarget::TypeId(types.insert_type(
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::Number),
                },
            ))),
            "boolean" => Some(TypeGuardTarget::TypeId(types.insert_type(
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::Boolean),
                },
            ))),
            "bigint" => Some(TypeGuardTarget::TypeId(types.insert_type(
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::Bigint),
                },
            ))),
            "symbol" => Some(TypeGuardTarget::TypeId(types.insert_type(
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::Symbol),
                },
            ))),
            "undefined" => Some(TypeGuardTarget::TypeId(types.insert_type(
                Type::TypeLiteral {
                    value: TypeLiteral::Undefined,
                },
            ))),
            "object" => Some(TypeGuardTarget::ObjectLike),
            "function" => Some(TypeGuardTarget::FunctionLike),
            _ => None,
        }
    }

    /// Unwrap `type` operator annotations to reach the underlying reference.
    pub(super) fn unwrap_type_symbol(
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

        // reuse cached instance types when available
        if let Some(instance_type_id) = types.get_instance_type_id(symbol) {
            return Ok(instance_type_id);
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

        // evaluate the alias value into an instance type
        let instance_type_id =
            self.try_evaluate_expression_to_type(module, profile, *value, tree, symbols, types)?;
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
            let _ =
                self.resolve_instance_type_for_symbol(module, profile, node_id, symbol, types)?;
            return Ok(());
        }

        // walk nested types based on structure
        match ty {
            Type::Value { value } => self.ensure_reference_instance_types_for_type_inner(
                module, profile, node_id, value, types, visited,
            ),
            Type::Conditional {
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
            | Type::Mutable { right, .. }
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
            Type::Array { element } => {
                if let Some(element) = element {
                    self.ensure_reference_instance_types_for_type_inner(
                        module, profile, node_id, element, types, visited,
                    )?;
                }
                Ok(())
            }
            Type::Tuple { elements } => {
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
    pub(super) fn resolve_instance_type_for_symbol(
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

        // ensure the defining module is analyzed before reading its types #RemoteAnalyze
        if symbol.module_id != module.id {
            self.require_analyze_module(symbol.module_id, profile)
                .map_err(AnalyzeError::from)?;
        }

        // ensure the global symbol table is available for this module
        self.require_resolve_module_prepare(module.id, profile)
            .map_err(AnalyzeError::from)?;

        // select the global merge group when the symbol participates
        let mut group_symbols = Vec::new();
        if let Some(key) = symbol_key
            && let Some(group) = self.get_global_symbol_group(module.id, profile, key, symbol_space)
        {
            group_symbols = group;
        }
        if group_symbols.is_empty() {
            group_symbols.push(symbol);
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
    /// FUGU: infer remote module without requiring it be analyzed first? #RemoteAnalyze
    fn import_instance_type_for_symbol(
        &self,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // ensure the remote module is analyzed before reading its types
        self.require_analyze_module(symbol.module_id, profile)
            .map_err(|error| match error {
                TaskDependencyError::NotReady { dependency } => AnalyzeError::Yield { dependency },
                TaskDependencyError::Failed { dependency } => {
                    AnalyzeError::UnsatisfiedDependency { dependency }
                }
            })?;

        // load the remote instance type from its module
        let remote_module = self.program.modules.get(symbol.module_id);
        let remote_module = remote_module.read();
        let remote_types = remote_module.dir(profile).types.read();
        let Some(remote_instance_id) = remote_types.get_instance_type_id(symbol) else {
            return Ok(None);
        };

        // import the remote instance type into the local table
        let remote_instance_ty = remote_types.get_type(remote_instance_id);
        let local_instance_id = self.import_type_from_remote_for_node(
            node_id,
            remote_instance_ty,
            &remote_types,
            symbol,
            types,
        );
        Ok(Some(local_instance_id))
    }

    /// Infer the result type of a scalar literal.
    /// Returns the literal type (e.g., `4` has type `4`), allowing assignability to check
    /// whether the literal fits the target type (int32, number, etc.).
    pub(super) fn infer_scalar_literal(&self, value: &ScalarLiteral) -> TypeLiteral {
        // return the literal type, not the widened primitive type
        // this allows `let x: int = 4` to work via assignability checking
        TypeLiteral::ScalarLiteral(value.clone())
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
    fn widen_numeric_types(&self, left: &Type, right: &Type) -> Type {
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
            } => Some(PrimitiveType::Number),
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
        _operator: &TypeUnaryOperator,
        _right_ty_id: LocalTypeId,
        _types: &TypeTable,
    ) -> Type {
        // NOTE #Incomplete: type level unary operation
        Type::TypeLiteral {
            value: TypeLiteral::Unknown,
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
        options: &AnalyzeOptions,
    ) -> Type {
        match operator {
            TypeBinaryOperator::Cast => {
                // type assertion: `x as T`
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

                if left_to_right == Assignability::NotAssignable
                    && right_to_left == Assignability::NotAssignable
                {
                    // neither direction works: illegal cast
                    self.error(AnalyzeError::InvalidCast {
                        node: expression_id.into_global_any(module.id),
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

                // check if left type satisfies (is assignable to) right type
                if self.is_type_assignable(
                    module,
                    profile,
                    symbols,
                    target_ty_id,
                    actual_ty_id,
                    types,
                    options,
                ) == Assignability::NotAssignable
                {
                    self.error(AnalyzeError::UnsatisfiedType {
                        node: expression_id.into_global_any(module.id),
                        expected_ty: target_ty_id.into_global(module.id),
                        actual_ty: actual_ty_id.into_global(module.id),
                    });
                }
                // satisfies returns the original (left) type, not the asserted type
                types.get_type(left_ty_id).clone()
            }
            _ => {
                // NOTE #Incomplete: other type level binary operations (is, instanceof, extends, etc.)
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                }
            }
        }
    }

    /// Infer the result type of a value of operation.
    pub(super) fn infer_value_of_operation(
        &self,
        _mutability: Option<Mutability>,
        _variance: Option<VarianceBound>,
        right: &Type,
    ) -> Type {
        // NOTE #Incomplete: resolve value of operation type
        right.clone()
    }

    /// Infer the result type of a reference of operation.
    pub(super) fn infer_reference_of_operation(
        &self,
        _mutability: Option<Mutability>,
        _variance: Option<VarianceBound>,
        right: &Type,
    ) -> Type {
        // NOTE #Incomplete: resolve reference of operation type
        right.clone()
    }

    /// Infer the type of a member field on a type by key.
    pub(super) fn infer_member_of_type(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        receiver_ty: &Type,
        member_key: &StaticKey,
        types: &mut TypeTable,
        visited: &mut Vec<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        match receiver_ty {
            // object type: look up field directly
            Type::Object { fields, .. } => Ok(fields
                .iter()
                .find(|f| f.key.matches(member_key))
                .map(|f| f.ty)),

            // value type: unwrap to the underlying type
            Type::Value { value } => {
                let value_ty = types.get_type(*value).clone();
                self.infer_member_of_type(
                    module, profile, node_id, &value_ty, member_key, types, visited,
                )
            }

            // reference to a nominal type: look up in the declaration instance type and extensions
            Type::Reference { symbol, .. } => self.infer_member_of_symbol(
                module, profile, node_id, *symbol, member_key, types, visited,
            ),

            // unary wrappers: unwrap before resolving members
            Type::Unary { right, .. }
            | Type::Mutable { right, .. }
            | Type::ValueOf { right, .. }
            | Type::ReferenceOf { right, .. }
            | Type::PointerOf { right, .. } => {
                let inner_ty = types.get_type(*right).clone();
                self.infer_member_of_type(
                    module, profile, node_id, &inner_ty, member_key, types, visited,
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
                        &element_ty,
                        member_key,
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
                        Ok(Some(types.insert_type(Type::Union {
                            elements: field_types,
                        })))
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
                        &element_ty,
                        member_key,
                        types,
                        visited,
                    )? {
                        return Ok(Some(field_ty));
                    }
                }
                Ok(None)
            }

            _ => Ok(None),
        }
    }

    /// Infer the index signature value type for a member key.
    pub(super) fn infer_index_signature_value_type_for_key(
        &self,
        module: &Module,
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
                    module, &value_ty, member_key, types, visited,
                )
            }
            Type::Reference { symbol, .. } => self.infer_index_signature_value_type_for_symbol(
                module, *symbol, member_key, types, visited,
            ),
            Type::Unary { right, .. }
            | Type::Mutable { right, .. }
            | Type::ValueOf { right, .. }
            | Type::ReferenceOf { right, .. }
            | Type::PointerOf { right, .. } => {
                let inner_ty = types.get_type(*right).clone();
                self.infer_index_signature_value_type_for_key(
                    module, &inner_ty, member_key, types, visited,
                )
            }
            Type::Union { elements } => {
                let mut value_types = Vec::new();
                for element_id in elements.clone() {
                    let element_ty = types.get_type(element_id).clone();
                    if let Some(value_ty) = self.infer_index_signature_value_type_for_key(
                        module,
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
                    _ => Some(self.union_types_from_list(value_types, types)),
                }
            }
            Type::Intersection { elements } => {
                for element_id in elements.clone() {
                    let element_ty = types.get_type(element_id).clone();
                    if let Some(value_ty) = self.infer_index_signature_value_type_for_key(
                        module,
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
        symbol: GlobalSymbolId,
        member_key: &StaticKey,
        types: &mut TypeTable,
        visited: &mut Vec<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // cycle detection: if we've already visited this symbol, stop
        // NOTE #Suspicious: should we really just return None for already visited symbol types?
        if visited.contains(&symbol) {
            return Ok(None);
        }
        visited.push(symbol);

        // ensure instance types are resolved for this symbol
        let _ = self.resolve_instance_type_for_symbol(module, profile, node_id, symbol, types)?;

        // step 1: look up in the type's own instance type
        if let Some(ty_id) = types.get_instance_type_id(symbol) {
            let ty = types.get_type(ty_id).clone();
            if let Some(member_ty) = self
                .infer_member_of_type(module, profile, node_id, &ty, member_key, types, visited)?
            {
                return Ok(Some(member_ty));
            }
        }

        // step 2: traverse lineage (extends, implements, embedded)
        if let Some(lineage) = types.get_lineage_for_symbol(symbol).cloned() {
            // check parent type (extends)
            if let Some(extends) = lineage.extends
                && let Some(member_ty) = self.infer_member_of_symbol(
                    module, profile, node_id, extends, member_key, types, visited,
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
                    *implements,
                    member_key,
                    types,
                    visited,
                )? {
                    return Ok(Some(member_ty));
                }
            }

            // check embedded types
            for embedded in &lineage.embedded {
                if let Some(member_ty) = self.infer_member_of_symbol(
                    module, profile, node_id, *embedded, member_key, types, visited,
                )? {
                    return Ok(Some(member_ty));
                }
            }
        }

        // step 3: check visible extensions
        let Some(extension_ids) = types.get_extensions_for_target(symbol) else {
            return Ok(None);
        };
        let extension_ids = extension_ids.clone();
        for extension_id in extension_ids {
            let extension = types.get_extension(extension_id);
            if !self.is_extension_visible(module, extension) {
                continue;
            }
            if let Some(ty_id) = types.get_instance_type_id(extension.symbol) {
                let ty = types.get_type(ty_id);
                if let Type::Object { fields, .. } = ty
                    && let Some(field) = fields.iter().find(|f| f.key.matches(member_key))
                {
                    return Ok(Some(field.ty));
                }
            }
        }

        Ok(None)
    }

    /// Infer the index signature value type for a symbol.
    fn infer_index_signature_value_type_for_symbol(
        &self,
        module: &Module,
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
        if let Some(ty_id) = types.get_instance_type_id(symbol) {
            let ty = types.get_type(ty_id).clone();
            if let Some(value_ty) = self
                .infer_index_signature_value_type_for_key(module, &ty, member_key, types, visited)
            {
                return Some(value_ty);
            }
        }

        // step 2: traverse lineage (extends, implements, embedded)
        if let Some(lineage) = types.get_lineage_for_symbol(symbol).cloned() {
            if let Some(extends) = lineage.extends
                && let Some(value_ty) = self.infer_index_signature_value_type_for_symbol(
                    module, extends, member_key, types, visited,
                )
            {
                return Some(value_ty);
            }

            for implements in &lineage.implements {
                if let Some(value_ty) = self.infer_index_signature_value_type_for_symbol(
                    module,
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
                    module, *embedded, member_key, types, visited,
                ) {
                    return Some(value_ty);
                }
            }
        }

        // step 3: check visible extensions
        let extension_ids = types.get_extensions_for_target(symbol)?.clone();
        for extension_id in extension_ids {
            let extension = types.get_extension(extension_id);
            if !self.is_extension_visible(module, extension) {
                continue;
            }
            if let Some(ty_id) = types.get_instance_type_id(extension.symbol) {
                let ty = types.get_type(ty_id).clone();
                if let Some(value_ty) = self.infer_index_signature_value_type_for_key(
                    module, &ty, member_key, types, visited,
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
            if index_key_kinds_compatible(signature_kind, key_kind) {
                value_types.push(signature.value_type);
            }
        }

        match value_types.len() {
            0 => None,
            1 => Some(value_types[0]),
            _ => Some(self.union_types_from_list(value_types, types)),
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
            ExtensionKind::Nominal => {
                if extension.symbol.module_id == module.id {
                    return true;
                }
                false // TODO #Incomplete: local/named extensions #Extensions
            }
        }
    }

    /// Resolve a remote symbol's value type by ensuring its module is analyzed
    /// and copying the type into the current module's TypeTable.
    pub(super) fn resolve_remote_symbol_value_type(
        &self,
        _module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        target_symbol: GlobalSymbolId,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        let remote_module_id = target_symbol.module_id;

        // ensure the remote module is analyzed (may yield)
        self.require_analyze_module(remote_module_id, profile)?;

        // look up the type in the remote module's TypeTable
        let remote_module = self.program.modules.get(remote_module_id);
        let remote_module = remote_module.read();
        let remote_types = remote_module.dir(profile).types.read();

        // copy the type into our local TypeTable
        if let Some(remote_ty_id) = remote_types.get_value_type_id(target_symbol) {
            let remote_ty = remote_types.get_type(remote_ty_id);
            let local_ty = self.import_type_from_remote_for_node(
                expression_id.into_any(),
                remote_ty,
                &remote_types,
                target_symbol,
                types,
            );
            Ok(local_ty)
        }
        // remote symbol doesn't have a value type, return unknown
        else {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            Ok(types.insert_type_from(ty, expression_id))
        }
    }

    /// Import a type from a remote module into the current module's TypeTable.
    /// For structural types (arrays, objects, ..): recursively copy the type structure.
    /// For nominal types (Type::Reference): keep them as references to the original symbol.
    pub(super) fn import_type_from_remote_for_node(
        &self,
        node_id: LocalNodeIdAny,
        remote_ty: &Type,
        remote_types: &TypeTable,
        target_symbol: GlobalSymbolId,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        match remote_ty {
            // leaf types: copy directly
            Type::TypeLiteral { value } => types.insert_type_from_any(
                Type::TypeLiteral {
                    value: value.clone(),
                },
                node_id,
            ),
            Type::InferVar { .. } => types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                },
                node_id,
            ),
            Type::Error => types.insert_type_from_any(Type::Error, node_id),
            Type::This => types.insert_type_from_any(Type::This, node_id),
            Type::Conditional {
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
                types.insert_type_from_any(
                    Type::Conditional {
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
                    constraint: local_constraint,
                    key_remap: local_key_remap,
                };
                types.insert_type_from_any(
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
                types.insert_type_from_any(
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
                types.insert_type_from_any(
                    Type::TemplateLiteral {
                        strings: strings.clone(),
                        spans: local_spans,
                    },
                    node_id,
                )
            }
            Type::Import { target, qualifier } => types.insert_type_from_any(
                Type::Import {
                    target: *target,
                    qualifier: qualifier.clone(),
                },
                node_id,
            ),
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
                types.insert_type_from_any(
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
                types.insert_type_from_any(
                    Type::Predicate {
                        asserts: *asserts,
                        subject: *subject,
                        target: local_target,
                    },
                    node_id,
                )
            }

            // array types
            Type::Array { element: None } => {
                types.insert_type_from_any(Type::Array { element: None }, node_id)
            }
            Type::Array {
                element: Some(element_id),
            } => {
                let element_ty = remote_types.get_type(*element_id);
                let local_elem = self.import_type_from_remote_for_node(
                    node_id,
                    element_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_type_from_any(
                    Type::Array {
                        element: Some(local_elem),
                    },
                    node_id,
                )
            }

            // tuple types
            Type::Tuple { elements } => {
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
                types.insert_type_from_any(
                    Type::Tuple {
                        elements: local_elements,
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
                types.insert_type_from_any(
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
                types.insert_type_from_any(
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
                types.insert_type_from_any(
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
                types.insert_type_from_any(
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
                types.insert_type_from_any(Type::Value { value: local_inner }, node_id)
            }
            Type::Mutable { mutability, right } => {
                let inner_ty = remote_types.get_type(*right);
                let local_inner = self.import_type_from_remote_for_node(
                    node_id,
                    inner_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_type_from_any(
                    Type::Mutable {
                        mutability: *mutability,
                        right: local_inner,
                    },
                    node_id,
                )
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
                types.insert_type_from_any(
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
                types.insert_type_from_any(
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
                types.insert_type_from_any(
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
                types.insert_type_from_any(
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
                types.insert_type_from_any(
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
            } => types.insert_type_from_any(
                Type::Reference {
                    symbol: *symbol,
                    static_arguments: static_arguments.clone(),
                },
                node_id,
            ),

            // types that can't be meaningfully copied: fall back to reference
            Type::Unevaluated(_) | Type::ArraySized { .. } => types.insert_type_from_any(
                Type::Reference {
                    symbol: target_symbol,
                    static_arguments: None,
                },
                node_id,
            ),
        }
    }

    /// Check whether the receiver explicitly implements a language item interface.
    pub(super) fn is_interface_implemented(
        &self,
        ty: &Type,
        interface_item: LanguageItem,
        types: &TypeTable,
    ) -> bool {
        let interface_symbol = self.language_item(interface_item);
        match ty {
            Type::Reference { symbol, .. } => {
                self.is_type_lineage_assignable(*symbol, interface_symbol, types)
            }
            Type::Union { elements } => elements.iter().all(|element_id| {
                let element_ty = types.get_type(*element_id);
                self.is_interface_implemented(element_ty, interface_item, types)
            }),
            _ => false,
        }
    }

    /// Check whether a type is definitely a struct type.
    pub(super) fn is_definitely_struct_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Reference { symbol, .. } => symbol.ty() == SymbolType::Struct,
            Type::TypeLiteral {
                value: TypeLiteral::Composite(DeclarationType::Struct),
            } => true,
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

                        types.insert_type(Type::Reference {
                            symbol,
                            static_arguments: Some(mapped_arguments),
                        })
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
                    types.insert_type(Type::Value {
                        value: mapped_value,
                    })
                }
            }
            Type::Unary { operator, right } => {
                let mapped_right = self.substitute_this_type(right, this_ty_id, types, cache);
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type(Type::Unary {
                        operator,
                        right: mapped_right,
                    })
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
                    types.insert_type(Type::Binary {
                        left: mapped_left,
                        operator,
                        right: mapped_right,
                    })
                }
            }
            Type::Conditional {
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
                    types.insert_type(Type::Conditional {
                        left: mapped_left,
                        right: mapped_right,
                        then_type: mapped_then,
                        else_type: mapped_else,
                    })
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
                        constraint: mapped_constraint,
                        key_remap: mapped_key_remap,
                    };
                    types.insert_type(Type::Mapped {
                        parameter,
                        modifiers,
                        value: mapped_value,
                    })
                }
            }
            Type::Index { left, index } => {
                let mapped_left = self.substitute_this_type(left, this_ty_id, types, cache);
                let mapped_index = self.substitute_this_type(index, this_ty_id, types, cache);
                if mapped_left == left && mapped_index == index {
                    ty_id
                } else {
                    types.insert_type(Type::Index {
                        left: mapped_left,
                        index: mapped_index,
                    })
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
                    types.insert_type(Type::TemplateLiteral {
                        strings,
                        spans: mapped_spans,
                    })
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
                    types.insert_type(Type::Infer {
                        name,
                        constraint: mapped_constraint,
                    })
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
                    types.insert_type(Type::Predicate {
                        asserts,
                        subject,
                        target: mapped_target,
                    })
                }
            }
            Type::Mutable { mutability, right } => {
                let mapped_right = self.substitute_this_type(right, this_ty_id, types, cache);
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type(Type::Mutable {
                        mutability,
                        right: mapped_right,
                    })
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
                    types.insert_type(Type::ValueOf {
                        mutability,
                        variance,
                        right: mapped_right,
                    })
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
                    types.insert_type(Type::ReferenceOf {
                        mutability,
                        variance,
                        right: mapped_right,
                    })
                }
            }
            Type::PointerOf { mutability, right } => {
                let mapped_right = self.substitute_this_type(right, this_ty_id, types, cache);
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type(Type::PointerOf {
                        mutability,
                        right: mapped_right,
                    })
                }
            }
            Type::ArraySized { element, count } => {
                let mapped_element = self.substitute_this_type(element, this_ty_id, types, cache);
                if mapped_element == element {
                    ty_id
                } else {
                    types.insert_type(Type::ArraySized {
                        element: mapped_element,
                        count,
                    })
                }
            }
            Type::Array { element } => {
                let mapped_element = element
                    .map(|element| self.substitute_this_type(element, this_ty_id, types, cache));
                if mapped_element == element {
                    ty_id
                } else {
                    types.insert_type(Type::Array {
                        element: mapped_element,
                    })
                }
            }
            Type::Tuple { elements } => {
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
                    types.insert_type(Type::Tuple {
                        elements: mapped_elements,
                    })
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
                    types.insert_type(Type::Object {
                        fields: mapped_fields,
                        call_signatures: mapped_call_signatures,
                        construct_signatures: mapped_construct_signatures,
                        index_signatures: mapped_index_signatures,
                    })
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
                    types.insert_type(Type::Function {
                        asynchrony,
                        cardinality,
                        static_parameters: mapped_static_parameters,
                        this_parameter: mapped_this,
                        dynamic_parameters: mapped_parameters,
                        return_type: mapped_return,
                    })
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
                    types.insert_type(Type::Union {
                        elements: mapped_elements,
                    })
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
                    types.insert_type(Type::Intersection {
                        elements: mapped_elements,
                    })
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
            StaticExpression::RangeExpression {
                start,
                end,
                is_inclusive,
            } => {
                let mapped_start =
                    self.substitute_this_static_expression(start, this_ty_id, types, cache);
                let mapped_end =
                    self.substitute_this_static_expression(end, this_ty_id, types, cache);
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
                    _ => Some(types.insert_type(Type::Union { elements: filtered })),
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
        type_id: LocalTypeId,
        key: &StaticKey,
        types: &TypeTable,
    ) -> bool {
        // walk through shapes that can carry fields
        match types.get_type(type_id) {
            Type::Object { fields, .. } => fields.iter().any(|field| field.key.matches(key)),
            Type::Reference { symbol, .. } => {
                // follow instance types for declared references
                types
                    .get_instance_type_id(*symbol)
                    .map(|instance_id| self.type_has_property(instance_id, key, types))
                    .unwrap_or(false)
            }
            Type::Intersection { elements } => {
                // accept any intersection member that matches
                elements
                    .iter()
                    .any(|element_id| self.type_has_property(*element_id, key, types))
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
                    // prefer instance types when available
                    if let Some(instance_id) = types.get_instance_type_id(*symbol) {
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
            _ => types.insert_type(Type::Intersection {
                elements: field_types,
            }),
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
        let canonical_symbol = self.canonical_symbol_id(module, symbols, profile, symbol);
        let is_promise_symbol = self
            .get_well_known_symbol(profile, WellKnownSymbol::Promise)
            .is_some_and(|promise_symbol| promise_symbol == canonical_symbol);
        if !is_promise_symbol {
            return None;
        }

        let Some(first_argument) = static_arguments
            .as_ref()
            .and_then(|arguments| arguments.first())
        else {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            return Some(types.insert_type(ty));
        };

        Some(self.convert_static_argument_type(first_argument, types))
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

            return self.union_types_from_list(awaited_elements, types);
        }

        // unwrap promises when possible
        if let Some(inner_id) = self.unwrap_promise_type(module, symbols, profile, type_id, types) {
            return self
                .unwrap_awaited_type_inner(module, symbols, profile, inner_id, types, visited);
        }

        type_id
    }

    /// Check whether a type is object like for typeof guards.
    pub(super) fn type_is_object_like(&self, type_id: LocalTypeId, types: &TypeTable) -> bool {
        // match shapes that would produce typeof object
        match types.get_type(type_id) {
            Type::TypeLiteral {
                value: TypeLiteral::Null,
            } => true,
            Type::Object { .. }
            | Type::Array { .. }
            | Type::ArraySized { .. }
            | Type::Tuple { .. }
            | Type::Value { .. } => true,
            Type::Reference { symbol, .. } => {
                // prefer instance types when available
                if let Some(instance_id) = types.get_instance_type_id(*symbol) {
                    return self.type_is_object_like(instance_id, types);
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
            Type::Intersection { elements } => elements
                .iter()
                .any(|element_id| self.type_is_object_like(*element_id, types)),
            _ => false,
        }
    }

    /// Check whether a type is function like for typeof guards.
    pub(super) fn type_is_function_like(&self, type_id: LocalTypeId, types: &TypeTable) -> bool {
        // match callable shapes for typeof function
        match types.get_type(type_id) {
            Type::Function { .. } => true,
            Type::Object {
                call_signatures,
                construct_signatures,
                ..
            } => !call_signatures.is_empty() || !construct_signatures.is_empty(),
            Type::Reference { symbol, .. } => {
                // prefer instance types when available
                if let Some(instance_id) = types.get_instance_type_id(*symbol) {
                    return self.type_is_function_like(instance_id, types);
                }

                symbol.local_id.ty == SymbolType::Function
            }
            Type::Intersection { elements } => elements
                .iter()
                .any(|element_id| self.type_is_function_like(*element_id, types)),
            _ => false,
        }
    }

    /// Build a union type from two type ids.
    pub(super) fn union_types(
        &self,
        left_ty_id: LocalTypeId,
        right_ty_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        if left_ty_id == right_ty_id {
            return left_ty_id;
        }

        let mut elements = Vec::new();
        self.append_union_elements(left_ty_id, &mut elements, types);
        self.append_union_elements(right_ty_id, &mut elements, types);

        if elements.len() == 1 {
            elements[0]
        } else {
            types.insert_type(Type::Union { elements })
        }
    }

    /// Build a union type from a list of type ids.
    pub(super) fn union_types_from_list(
        &self,
        type_ids: Vec<LocalTypeId>,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        let mut elements = Vec::new();
        for ty_id in type_ids {
            self.append_union_elements(ty_id, &mut elements, types);
        }

        if elements.len() == 1 {
            elements[0]
        } else {
            types.insert_type(Type::Union { elements })
        }
    }

    /// Append union elements for a type id to a list.
    fn append_union_elements(
        &self,
        ty_id: LocalTypeId,
        elements: &mut Vec<LocalTypeId>,
        types: &TypeTable,
    ) {
        match types.get_type(ty_id) {
            Type::Union { elements: union } => {
                for element_id in union {
                    if !elements.contains(element_id) {
                        elements.push(*element_id);
                    }
                }
            }
            _ => {
                if !elements.contains(&ty_id) {
                    elements.push(ty_id);
                }
            }
        }
    }

    /// Check whether a type is null or undefined.
    fn is_nullish_type(&self, ty: &Type) -> bool {
        matches!(
            ty,
            Type::TypeLiteral {
                value: TypeLiteral::Null | TypeLiteral::Undefined,
            }
        )
    }
}
