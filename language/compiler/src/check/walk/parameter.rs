use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    GenericParameterBinding, GenericParameterId, Origin, PatternRelation, StaticTerm, TypeOperand,
    TypeRelation, TypeTerm, WalkState,
};

impl WalkState<'_, '_> {
    /// Walk one generic parameter.
    ///
    /// Example:
    /// ```ds
    /// <T extends Serializable = string>
    /// ```
    pub(in crate::check) fn walk_generic_parameter(
        &mut self,
        id: dir::LocalNodeId<dir::GenericParameter>,
        generic_parameter: &dir::GenericParameter,
    ) -> CompilerResult<()> {
        // enter static owner guard
        let Some(_guard) = self.enter_static_guard_for(id.into_any(), None)? else {
            return Ok(());
        };

        match generic_parameter {
            // <T>
            dir::GenericParameter::Type {
                constraint,
                default,
                ..
            } => {
                // walk optional bounds
                if let Some(constraint) = constraint {
                    self.walk_type_expression(*constraint, self.tree.get(*constraint))?;
                }
                if let Some(default) = default {
                    self.walk_type_expression(*default, self.tree.get(*default))?;
                }
                self.bind_generic_parameter(self.module, id, generic_parameter)?;
            }
            // <...T>
            dir::GenericParameter::VariadicType {
                constraint,
                default,
                ..
            } => {
                // walk optional bounds
                if let Some(constraint) = constraint {
                    self.walk_type_expression(*constraint, self.tree.get(*constraint))?;
                }
                if let Some(default) = default {
                    self.walk_type_expression(*default, self.tree.get(*default))?;
                }
                self.bind_generic_parameter(self.module, id, generic_parameter)?;
            }
            // <comptime C: T>
            dir::GenericParameter::Value {
                declared_type,
                default,
                ..
            } => {
                // static parameters require an explicit value type
                if declared_type.is_none() {
                    self.check
                        .report_missing_type_annotation(self.module, id.into_any());
                }

                // walk optional bounds
                if let Some(declared_type) = declared_type {
                    self.walk_type_expression(*declared_type, self.tree.get(*declared_type))?;
                }
                if let Some(default) = default {
                    // check generic default in declaration context
                    let before_default = self.fork_flow();

                    self.walk_expression(*default, self.tree.get(*default))?;
                    self.restore_flow(before_default);
                }
                self.bind_generic_parameter(self.module, id, generic_parameter)?;
            }
            // <comptime ...C: T>
            dir::GenericParameter::VariadicValue {
                declared_type,
                default,
                ..
            } => {
                // static parameters require an explicit value type
                if declared_type.is_none() {
                    self.check
                        .report_missing_type_annotation(self.module, id.into_any());
                }

                // walk optional bounds
                if let Some(declared_type) = declared_type {
                    self.walk_type_expression(*declared_type, self.tree.get(*declared_type))?;
                }
                if let Some(default) = default {
                    // check generic default in declaration context
                    let before_default = self.fork_flow();

                    self.walk_expression(*default, self.tree.get(*default))?;
                    self.restore_flow(before_default);
                }
                self.bind_generic_parameter(self.module, id, generic_parameter)?;
            }
            // ignore damaged syntax
            dir::GenericParameter::Error => {}
        }

        Ok(())
    }

    /// Bind the generic parameter introduced by one generic parameter.
    ///
    /// Example:
    /// ```ds
    /// <comptime Size: number = 4>
    /// ```
    pub(in crate::check) fn bind_generic_parameter(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::GenericParameter>,
        generic_parameter: &dir::GenericParameter,
    ) -> CompilerResult<Option<GenericParameterId>> {
        let source = id.into_any();
        let Some(owner) = self.check.module(module).scope_owner_symbol(source) else {
            return Ok(None);
        };
        let Some(symbol) = self.check.module(module).declaration_symbol(source) else {
            return Ok(None);
        };

        match generic_parameter {
            // <T>
            dir::GenericParameter::Type {
                variance,
                constraint,
                default,
                ..
            } => {
                let parameter = self
                    .check
                    .allocate_explicit_generic_parameter(owner, symbol);
                let parameter_id = parameter.id();
                let constraint = constraint
                    .map(|id| self.allocate_node_type_operand(id))
                    .transpose()?
                    .map(Into::into);
                let default = default
                    .map(|id| self.allocate_node_type_operand(id))
                    .transpose()?;
                let generic = GenericParameterBinding::Type {
                    identity: parameter,
                    variance: *variance,
                    constraint,
                    default,
                };

                self.check.insert_generic_parameter(generic);
                let condition = self.active_static_guard();

                self.bind_symbol_type(symbol, TypeTerm::Parameter(parameter_id), condition)?;

                Ok(Some(parameter_id))
            }
            // <...T>
            dir::GenericParameter::VariadicType {
                variance,
                constraint,
                default,
                ..
            } => {
                let parameter = self
                    .check
                    .allocate_explicit_generic_parameter(owner, symbol);
                let parameter_id = parameter.id();
                let constraint = constraint
                    .map(|id| self.allocate_node_type_operand(id))
                    .transpose()?
                    .map(Into::into);
                let default = default
                    .map(|id| self.allocate_node_type_operand(id))
                    .transpose()?;
                let generic = GenericParameterBinding::VariadicType {
                    identity: parameter,
                    variance: *variance,
                    constraint,
                    default,
                };

                self.check.insert_generic_parameter(generic);
                let condition = self.active_static_guard();

                self.bind_symbol_type(symbol, TypeTerm::Parameter(parameter_id), condition)?;

                Ok(Some(parameter_id))
            }
            // <comptime C: T>
            dir::GenericParameter::Value {
                declared_type,
                default,
                ..
            } => {
                let parameter = self
                    .check
                    .allocate_explicit_generic_parameter(owner, symbol);
                let parameter_id = parameter.id();
                let constraint = declared_type
                    .map(|id| self.allocate_node_type_operand(id))
                    .transpose()?
                    .map(Into::into);
                let default = default
                    .map(|id| {
                        let condition = self.active_static_guard();

                        self.allocate_static_expression_variable(id, condition)
                    })
                    .transpose()?
                    .map(Into::into);
                let generic = GenericParameterBinding::Static {
                    identity: parameter,
                    constraint,
                    default,
                };

                self.check.insert_generic_parameter(generic);
                let condition = self.active_static_guard();

                self.bind_symbol_static(symbol, StaticTerm::Parameter(parameter_id), condition)?;

                Ok(Some(parameter_id))
            }
            // <comptime ...C: T>
            dir::GenericParameter::VariadicValue {
                declared_type,
                default,
                ..
            } => {
                let parameter = self
                    .check
                    .allocate_explicit_generic_parameter(owner, symbol);
                let parameter_id = parameter.id();
                let constraint = declared_type
                    .map(|id| self.allocate_node_type_operand(id))
                    .transpose()?
                    .map(Into::into);
                let default = default
                    .map(|id| {
                        let condition = self.active_static_guard();

                        self.allocate_static_expression_variable(id, condition)
                    })
                    .transpose()?
                    .map(Into::into);
                let generic = GenericParameterBinding::VariadicStatic {
                    identity: parameter,
                    constraint,
                    default,
                };

                self.check.insert_generic_parameter(generic);
                let condition = self.active_static_guard();

                self.bind_symbol_static(symbol, StaticTerm::Parameter(parameter_id), condition)?;

                Ok(Some(parameter_id))
            }
            // ignore damaged syntax
            dir::GenericParameter::Error => Ok(None),
        }
    }

    /// Walk one parameter.
    ///
    /// Example:
    /// ```ds
    /// (value: T = defaultValue)
    /// ```
    pub(in crate::check) fn walk_parameter(
        &mut self,
        id: dir::LocalNodeId<dir::Parameter>,
        parameter: &dir::Parameter,
        is_annotation_required: bool,
    ) -> CompilerResult<()> {
        let Some(_guard) = self.enter_static_guard_for(id.into_any(), None)? else {
            return Ok(());
        };

        match parameter {
            // (p: T)
            dir::Parameter::Named {
                declared_type,
                default,
                is_comptime,
                ..
            } => {
                // report missing annotations
                if is_annotation_required && declared_type.is_none() {
                    self.check
                        .report_missing_type_annotation(self.module, id.into_any());
                }

                // walk optional children
                if let Some(declared_type) = declared_type {
                    self.walk_type_expression(*declared_type, self.tree.get(*declared_type))?;
                }
                if let Some(default) = default {
                    let before_default = self.fork_flow();

                    self.walk_expression(*default, self.tree.get(*default))?;
                    self.restore_flow(before_default);
                }

                let symbol = self
                    .check
                    .module(self.module)
                    .declaration_symbol(id.into_any());
                let parameter_type = self.parameter_type(id)?;

                // bind named parameter output
                if let Some(symbol) = symbol {
                    if *is_comptime {
                        self.bind_comptime_parameter(
                            self.module,
                            id,
                            symbol,
                            parameter_type,
                            *default,
                            false,
                        )?;
                    } else if let Some(parameter_type) = parameter_type {
                        let condition = self.active_static_guard();

                        self.bind_symbol_type_operand(symbol, parameter_type, condition)?;
                    }
                }

                // constrain default value
                if let (Some(default), Some(parameter_type)) = (default, parameter_type) {
                    self.constrain_parameter_default(self.module, *default, parameter_type)?;
                }
            }
            // (p: ...T)
            dir::Parameter::VariadicNamed {
                declared_type,
                is_comptime,
                ..
            } => {
                // report missing annotations
                if is_annotation_required && declared_type.is_none() {
                    self.check
                        .report_missing_type_annotation(self.module, id.into_any());
                }

                // walk optional element type
                if let Some(declared_type) = declared_type {
                    self.walk_type_expression(*declared_type, self.tree.get(*declared_type))?;
                }

                let symbol = self
                    .check
                    .module(self.module)
                    .declaration_symbol(id.into_any());
                let parameter_type = self.parameter_type(id)?;

                // bind variadic parameter output
                if let Some(symbol) = symbol {
                    if *is_comptime {
                        self.bind_comptime_parameter(
                            self.module,
                            id,
                            symbol,
                            parameter_type,
                            None,
                            true,
                        )?;
                    } else if let Some(parameter_type) = parameter_type {
                        let condition = self.active_static_guard();

                        self.bind_symbol_type_operand(symbol, parameter_type, condition)?;
                    }
                }
            }
            // ({ p }: T)
            dir::Parameter::Pattern {
                pattern,
                declared_type,
                default,
                ..
            } => {
                // report missing annotations
                if is_annotation_required && declared_type.is_none() {
                    self.check
                        .report_missing_type_annotation(self.module, id.into_any());
                }

                // walk pattern and optional children
                self.walk_pattern(*pattern, self.tree.get(*pattern))?;
                if let Some(declared_type) = declared_type {
                    self.walk_type_expression(*declared_type, self.tree.get(*declared_type))?;
                }
                if let Some(default) = default {
                    let before_default = self.fork_flow();

                    self.walk_expression(*default, self.tree.get(*default))?;
                    self.restore_flow(before_default);
                }

                let parameter_type = self.parameter_type(id)?;

                // constrain the pattern against the parameter type
                if let Some(parameter_type) = parameter_type {
                    if let Some(term) = self.lower_pattern_term(self.module, *pattern)? {
                        let condition = self.active_static_guard();

                        self.check.relate_pattern(
                            self.module,
                            PatternRelation::Match(term),
                            pattern.into_any(),
                            parameter_type,
                            condition,
                        );
                    }

                    if let Some(default) = default {
                        self.constrain_parameter_default(self.module, *default, parameter_type)?;
                    }
                }
            }
            // ({ p }: ...T)
            dir::Parameter::VariadicPattern {
                pattern,
                declared_type,
                ..
            } => {
                // report missing annotations
                if is_annotation_required && declared_type.is_none() {
                    self.check
                        .report_missing_type_annotation(self.module, id.into_any());
                }

                // walk pattern and optional element type
                self.walk_pattern(*pattern, self.tree.get(*pattern))?;
                if let Some(declared_type) = declared_type {
                    self.walk_type_expression(*declared_type, self.tree.get(*declared_type))?;
                }

                // constrain the pattern against the parameter type
                if let Some(parameter_type) = self.parameter_type(id)? {
                    if let Some(term) = self.lower_pattern_term(self.module, *pattern)? {
                        let condition = self.active_static_guard();

                        self.check.relate_pattern(
                            self.module,
                            PatternRelation::Match(term),
                            pattern.into_any(),
                            parameter_type,
                            condition,
                        );
                    }
                }
            }
            // ignore damaged syntax
            dir::Parameter::Error => {}
        }

        Ok(())
    }

    /// Bind one comptime runtime parameter as an induced static generic parameter.
    ///
    /// Example:
    /// ```ds
    /// function repeat(value: string, comptime count: uint): [string; count]
    /// ```
    fn bind_comptime_parameter(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::Parameter>,
        symbol: dir::GlobalSymbolId,
        parameter_type: Option<TypeOperand>,
        default: Option<dir::LocalNodeId<dir::Expression>>,
        is_variadic: bool,
    ) -> CompilerResult<Option<GenericParameterId>> {
        let source = id.into_any();
        let Some(owner) = self.check.module(module).scope_owner_symbol(source) else {
            return Ok(None);
        };
        let parameter = self
            .check
            .allocate_induced_symbol_generic_parameter(owner, symbol);
        let parameter_id = parameter.id();
        let default = default
            .map(|id| {
                let condition = self.active_static_guard();

                self.allocate_static_expression_variable(id, condition)
            })
            .transpose()?
            .map(Into::into);
        let generic = if is_variadic {
            GenericParameterBinding::VariadicStatic {
                identity: parameter,
                constraint: parameter_type,
                default,
            }
        } else {
            GenericParameterBinding::Static {
                identity: parameter,
                constraint: parameter_type,
                default,
            }
        };

        self.check.insert_generic_parameter(generic);
        let condition = self.active_static_guard();

        self.bind_symbol_static(symbol, StaticTerm::Parameter(parameter_id), condition)?;

        Ok(Some(parameter_id))
    }

    /// Constrain one parameter default value to its declared type.
    ///
    /// Example:
    /// ```ds
    /// (value: number = 1)
    /// ```
    fn constrain_parameter_default(
        &mut self,
        module: ModuleId,
        default: dir::LocalNodeId<dir::Expression>,
        declared_type: TypeOperand,
    ) -> CompilerResult<()> {
        let value = self.allocate_node_type_operand(default)?;
        let origin = Origin::Node(default.into_global_any(module));
        let condition = self.active_static_guard();

        self.check.relate_type(
            origin,
            TypeRelation::Assignable,
            value,
            declared_type,
            condition,
        );

        Ok(())
    }
}
