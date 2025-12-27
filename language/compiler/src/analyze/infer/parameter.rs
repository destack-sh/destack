use std::collections::HashSet;

use crate::Compiler;
use destack_dir::{
    Declaration, Expression, GlobalNodeId, GlobalSymbolId, LocalTypeId, NodeTree, Parameter,
    StaticArgument, StaticExpression, StaticProperty, StringId, SymbolTable, Type, TypeLiteral,
    TypeTable,
};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};

/// Describe how a static parameter is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StaticParameterKind {
    /// Use the parameter as a type argument.
    Type,
    /// Use the parameter as a value argument.
    Value,
}

/// Parameter metadata needed to resolve and validate static parameters.
/// #Cleanup: maybe move StaticParameter into DIR (next to StaticArgument)?
#[derive(Debug, Clone)]
pub(super) struct StaticParameter {
    /// Whether this is a type or value parameter.
    pub(super) kind: StaticParameterKind,
    /// Identify the static parameter symbol.
    pub(super) symbol: GlobalSymbolId,
    /// Parameter name for mapping and diagnostics.
    pub(super) name: Option<StringId>,
    /// Declared type for validation.
    pub(super) declared_type_id: LocalTypeId,
    /// Default expression for missing arguments.
    pub(super) default_expression: Option<GlobalNodeId<Expression>>,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Collect static parameter symbols for a declaration symbol.
    pub(super) fn collect_static_parameter_symbols(
        &self,
        module: &Module,
        symbol: GlobalSymbolId,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<Vec<GlobalSymbolId>> {
        // select the module that owns the symbol
        if symbol.module_id == module.id {
            self.collect_static_parameter_symbols_in_module(module.id, symbol, tree, symbols)
        } else {
            let remote_module = self.program.modules.get(symbol.module_id);
            let remote_module = remote_module.read();
            let remote_tree = remote_module.dir(profile).tree.read();
            let remote_symbols = remote_module.dir(profile).symbols.read();
            self.collect_static_parameter_symbols_in_module(
                remote_module.id,
                symbol,
                &remote_tree,
                &remote_symbols,
            )
        }
    }

    /// Collect static parameter symbols for a declaration in a module.
    fn collect_static_parameter_symbols_in_module(
        &self,
        module_id: ModuleId,
        symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<Vec<GlobalSymbolId>> {
        // extract the declaration for the symbol
        let symbol_entry = symbols.get_symbol(symbol.local_id);
        let primary_declaration = symbol_entry.primary_declaration?;
        let declaration_id = match primary_declaration.try_into_local_typed::<Declaration>() {
            Ok(declaration_id) => declaration_id,
            Err(_) => return None,
        };
        let declaration = tree.get(declaration_id);

        // locate static parameters on the declaration shape
        let parameters = match declaration {
            Declaration::Type {
                static_parameters, ..
            } => static_parameters.as_ref(),
            Declaration::Struct { generics, .. }
            | Declaration::Class { generics, .. }
            | Declaration::Enum { generics, .. }
            | Declaration::Interface { generics, .. }
            | Declaration::Extension { generics, .. }
            | Declaration::Namespace { generics, .. } => generics.static_parameters.as_ref(),
            _ => None,
        };

        let Some(parameters) = parameters else {
            return Some(Vec::new());
        };

        // map parameter nodes to global symbols
        let symbols = parameters
            .iter()
            .map(|parameter_id| {
                let symbol = tree.get(*parameter_id).symbol();
                symbol.into_global(module_id)
            })
            .collect();

        Some(symbols)
    }

    /// Collect static parameter metadata for a symbol (in this or another module).
    pub(super) fn collect_static_parameter(
        &self,
        module: &Module,
        symbol_id: GlobalSymbolId,
        kind: StaticParameterKind,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> StaticParameter {
        let parameter = if symbol_id.module_id == module.id {
            self.collect_static_parameter_in_module(
                module.id, symbol_id, kind, tree, symbols, types,
            )
        } else {
            let remote_module = self.program.modules.get(symbol_id.module_id);
            let remote_module = remote_module.read();
            let remote_tree = remote_module.dir(profile).tree.read();
            let remote_symbols = remote_module.dir(profile).symbols.read();

            self.collect_static_parameter_in_module(
                remote_module.id,
                symbol_id,
                kind,
                &remote_tree,
                &remote_symbols,
                types,
            )
        };

        parameter.unwrap_or_else(|| self.fallback_static_parameter(symbol_id, kind, types))
    }

    /// Collect placeholder types for static parameters on a signature.
    pub(super) fn collect_static_parameter_placeholders(
        &self,
        module: &Module,
        signature: &destack_dir::FunctionSignature,
        tree: &NodeTree,
        types: &mut TypeTable,
    ) -> Vec<LocalTypeId> {
        // extract static parameters from the signature
        let mut static_parameters = Vec::new();
        let Some(generics) = &signature.generics else {
            return static_parameters;
        };
        let Some(parameters) = &generics.static_parameters else {
            return static_parameters;
        };

        // register each static parameter as a reference placeholder
        for parameter_id in parameters {
            let parameter = tree.get(*parameter_id);
            let symbol = parameter.symbol().into_global(module.id);
            let ty = Type::Reference {
                symbol,
                static_arguments: None,
            };
            let ty_id = types.insert_type_from(ty, *parameter_id);
            static_parameters.push(ty_id);
        }

        static_parameters
    }

    /// Build a fallback static parameter when metadata cannot be resolved.
    pub(super) fn fallback_static_parameter(
        &self,
        symbol: GlobalSymbolId,
        kind: StaticParameterKind,
        types: &mut TypeTable,
    ) -> StaticParameter {
        let unknown_ty_id = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Unknown,
        });

        StaticParameter {
            symbol,
            name: None,
            declared_type_id: unknown_ty_id,
            default_expression: None,
            kind,
        }
    }

    /// Collect static parameter metadata from a module.
    fn collect_static_parameter_in_module(
        &self,
        module_id: ModuleId,
        symbol_id: GlobalSymbolId,
        kind: StaticParameterKind,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<StaticParameter> {
        // read the parameter declaration
        let symbol = symbols.get_symbol(symbol_id.local_id);
        let primary_declaration = symbol.primary_declaration?;
        let parameter_id = primary_declaration
            .try_into_local_typed::<Parameter>()
            .ok()?;
        let parameter = tree.get(parameter_id);

        // resolve the declared type for the parameter
        let declared_type_id = if module_id == types.module_id {
            types
                .get_declared_type_id(primary_declaration)
                .unwrap_or_else(|| {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    types.insert_type(ty)
                })
        } else {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            types.insert_type(ty)
        };

        // derive the parameter name for mapping
        let name = match parameter {
            Parameter::Named { name, .. } => Some(*name),
            Parameter::Variadic { name, .. } => Some(*name),
            Parameter::Pattern { .. } => None,
        };

        // resolve the default expression for the parameter
        let default_expression = match parameter {
            Parameter::Named { default, .. } => {
                default.map(|expression_id| expression_id.into_global(module_id))
            }
            Parameter::Pattern { default, .. } => {
                default.map(|expression_id| expression_id.into_global(module_id))
            }
            Parameter::Variadic { .. } => None,
        };

        Some(StaticParameter {
            symbol: symbol_id,
            name,
            declared_type_id,
            default_expression,
            kind,
        })
    }

    /// Collect static parameter symbols referenced in a type.
    pub(super) fn collect_type_reference_symbols(
        &self,
        ty_id: LocalTypeId,
        types: &TypeTable,
        symbols: &mut HashSet<GlobalSymbolId>,
        visited: &mut HashSet<LocalTypeId>,
    ) {
        if !visited.insert(ty_id) {
            return;
        }
        match types.get_type(ty_id) {
            Type::TypeLiteral { .. } | Type::InferVar { .. } | Type::Error => {}
            Type::Value { value } => {
                self.collect_type_reference_symbols(*value, types, symbols, visited);
            }
            Type::Reference {
                symbol,
                static_arguments,
            } => {
                symbols.insert(*symbol);
                if let Some(static_arguments) = static_arguments {
                    for argument in static_arguments {
                        self.collect_type_reference_symbols_in_static_argument(
                            argument, types, symbols, visited,
                        );
                    }
                }
            }
            Type::Unevaluated(_) => {}
            Type::Unary { right, .. } => {
                self.collect_type_reference_symbols(*right, types, symbols, visited);
            }
            Type::Binary { left, right, .. } => {
                self.collect_type_reference_symbols(*left, types, symbols, visited);
                self.collect_type_reference_symbols(*right, types, symbols, visited);
            }
            Type::Mutable { right, .. }
            | Type::ValueOf { right, .. }
            | Type::ReferenceOf { right, .. } => {
                self.collect_type_reference_symbols(*right, types, symbols, visited);
            }
            Type::ArraySized { element, .. } => {
                self.collect_type_reference_symbols(*element, types, symbols, visited);
            }
            Type::Array { element } => {
                if let Some(element) = element {
                    self.collect_type_reference_symbols(*element, types, symbols, visited);
                }
            }
            Type::Tuple { elements } => {
                for element in elements {
                    self.collect_type_reference_symbols(*element, types, symbols, visited);
                }
            }
            Type::Object { fields } => {
                for field in fields {
                    self.collect_type_reference_symbols(field.ty, types, symbols, visited);
                }
            }
            Type::Function {
                dynamic_parameters,
                return_type,
                ..
            } => {
                for parameter in dynamic_parameters {
                    self.collect_type_reference_symbols(*parameter, types, symbols, visited);
                }
                if let Some(return_type) = return_type {
                    self.collect_type_reference_symbols(*return_type, types, symbols, visited);
                }
            }
            Type::Union { elements } | Type::Intersection { elements } => {
                for element in elements {
                    self.collect_type_reference_symbols(*element, types, symbols, visited);
                }
            }
        }
    }

    /// Collect static parameter symbols referenced in a static argument.
    pub(super) fn collect_type_reference_symbols_in_static_argument(
        &self,
        argument: &StaticArgument,
        types: &TypeTable,
        symbols: &mut HashSet<GlobalSymbolId>,
        visited: &mut HashSet<LocalTypeId>,
    ) {
        match argument {
            StaticArgument::Unevaluated { .. } => {}
            StaticArgument::Evaluated { value, .. } => {
                self.collect_type_reference_symbols_in_static_expression(
                    value, types, symbols, visited,
                );
            }
        }
    }

    /// Collect static parameter symbols referenced in a static expression.
    pub(super) fn collect_type_reference_symbols_in_static_expression(
        &self,
        expression: &StaticExpression,
        types: &TypeTable,
        symbols: &mut HashSet<GlobalSymbolId>,
        visited: &mut HashSet<LocalTypeId>,
    ) {
        match expression {
            StaticExpression::Unevaluated { .. }
            | StaticExpression::ScalarLiteral { .. }
            | StaticExpression::TypeLiteral { .. } => {}
            StaticExpression::Type { ty } => {
                self.collect_type_reference_symbols(*ty, types, symbols, visited);
            }
            StaticExpression::Declaration {
                static_arguments, ..
            } => {
                if let Some(static_arguments) = static_arguments {
                    for argument in static_arguments {
                        self.collect_type_reference_symbols_in_static_argument(
                            argument, types, symbols, visited,
                        );
                    }
                }
            }
            StaticExpression::RangeExpression { start, end, .. } => {
                self.collect_type_reference_symbols_in_static_expression(
                    start, types, symbols, visited,
                );
                self.collect_type_reference_symbols_in_static_expression(
                    end, types, symbols, visited,
                );
            }
            StaticExpression::ArrayExpression { elements }
            | StaticExpression::TupleExpression { elements } => {
                for element in elements {
                    self.collect_type_reference_symbols_in_static_expression(
                        element, types, symbols, visited,
                    );
                }
            }
            StaticExpression::ObjectExpression { properties } => {
                for property in properties {
                    self.collect_type_reference_symbols_in_static_property(
                        property, types, symbols, visited,
                    );
                }
            }
        }
    }

    /// Collect static parameter symbols referenced in a static property.
    pub(super) fn collect_type_reference_symbols_in_static_property(
        &self,
        property: &StaticProperty,
        types: &TypeTable,
        symbols: &mut HashSet<GlobalSymbolId>,
        visited: &mut HashSet<LocalTypeId>,
    ) {
        match property {
            StaticProperty::Unevaluated { .. } => {}
            StaticProperty::Field { value, default, .. } => {
                self.collect_type_reference_symbols_in_static_expression(
                    value, types, symbols, visited,
                );
                if let Some(default) = default {
                    self.collect_type_reference_symbols_in_static_expression(
                        default, types, symbols, visited,
                    );
                }
            }
            StaticProperty::Method { body, .. } => {
                self.collect_type_reference_symbols_in_static_expression(
                    body, types, symbols, visited,
                );
            }
        }
    }
}
