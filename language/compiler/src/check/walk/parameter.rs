use destack_dir as dir;

use crate::check::{
    Expectation, GenericInductionParameter, GenericParameterId, GenericTemplateId, Origin,
    ValueUse, WalkState,
};
use crate::{CompilerError, CompilerResult};

impl WalkState<'_, '_> {
    /// Open one generic parameter header.
    ///
    /// Example:
    /// ```ds
    /// <T extends Serializable = string>
    /// ```
    pub(in crate::check) fn open_generic_parameter(
        &mut self,
        template: GenericTemplateId,
        id: dir::LocalNodeId<dir::GenericParameter>,
        generic_parameter: &dir::GenericParameter,
    ) -> CompilerResult<Option<GenericParameterId>> {
        if !self.decide_decorated_presence(id.into_any())? {
            return Ok(None);
        }
        let symbol = self.generic_parameter_symbol(id)?;

        if let Some(parameter) = self.check.generics.parameter_by_symbol(symbol) {
            return Ok(Some(parameter));
        }

        // shape the parameter binding by its declared kind
        let (variance, is_variadic, is_const, is_comptime) = match generic_parameter {
            // <T>
            dir::GenericParameter::Type {
                variance, is_const, ..
            } => (*variance, false, *is_const, false),
            // <...T>
            dir::GenericParameter::VariadicType {
                variance, is_const, ..
            } => (*variance, true, *is_const, false),
            // <comptime C: T>
            dir::GenericParameter::Value { .. } => (None, false, false, true),
            // <comptime ...C: T>
            dir::GenericParameter::VariadicValue { .. } => (None, true, false, true),
            // ignore damaged nodes
            dir::GenericParameter::Error => return Ok(None),
        };
        let binding = dir::GenericParameterBinding {
            template: template.local_id,
            key: dir::GenericParameterKey::Symbol(symbol),
            variance,
            constraint: None,
            default: None,
            origin: dir::GenericParameterOrigin::Explicit,
            is_variadic,
            is_const,
            is_comptime,
        };
        let parameter = self
            .check
            .push_generic_parameter(binding, template, Some(symbol))?;

        // the parameter name writes its own parameter type
        let ty = self.push_type(dir::Type::Parameter(parameter), id.into_any())?;
        self.bind_symbol_type(symbol, ty)?;

        Ok(Some(parameter))
    }

    /// Walk one generic parameter's bounds.
    ///
    /// Example:
    /// ```ds
    /// <T extends Serializable = string>
    /// ```
    pub(in crate::check) fn walk_generic_parameter(
        &mut self,
        template: GenericTemplateId,
        id: dir::LocalNodeId<dir::GenericParameter>,
        generic_parameter: &dir::GenericParameter,
    ) -> CompilerResult<()> {
        if !self.decide_decorated_presence(id.into_any())? {
            return Ok(());
        }
        let Some(parameter) = self.open_generic_parameter(template, id, generic_parameter)? else {
            return Ok(());
        };

        match generic_parameter {
            // <T extends U = V>, <...T extends U = V>
            dir::GenericParameter::Type {
                constraint,
                default,
                ..
            }
            | dir::GenericParameter::VariadicType {
                constraint,
                default,
                ..
            } => {
                let (constraint, default) = (*constraint, *default);
                let constraint = constraint
                    .map(|constraint| self.walk_type_expression(constraint))
                    .transpose()?;
                let default = default
                    .map(|default| self.walk_type_expression(default))
                    .transpose()?;

                self.check
                    .update_generic_parameter_bounds(parameter, constraint, default)?;
            }
            // <comptime C: T = N>, <comptime ...C: T = N>
            dir::GenericParameter::Value {
                declared_type,
                default,
                ..
            }
            | dir::GenericParameter::VariadicValue {
                declared_type,
                default,
                ..
            } => {
                let (declared_type, default) = (*declared_type, *default);

                // static parameters require an explicit value type
                if declared_type.is_none() {
                    self.check
                        .report_missing_type_annotation(self.module, id.into_any());
                }

                let constraint = declared_type
                    .map(|declared_type| self.walk_type_expression(declared_type))
                    .transpose()?;
                let default = default
                    .map(|default| {
                        // check generic defaults in declaration context
                        let before_default = self.fork_flow();
                        self.walk_expression(default, self.tree.get(default), None)?;
                        self.restore_flow(before_default);

                        self.walk_static_term(default)
                    })
                    .transpose()?;

                self.check
                    .update_generic_parameter_bounds(parameter, constraint, default)?;
            }
            // ignore damaged nodes
            dir::GenericParameter::Error => {}
        }

        Ok(())
    }

    /// Return the symbol declared by one generic parameter.
    fn generic_parameter_symbol(
        &self,
        id: dir::LocalNodeId<dir::GenericParameter>,
    ) -> CompilerResult<dir::GlobalSymbolId> {
        let source = id.into_any();
        let Some(symbol) = self.check.module(self.module).declaration_symbol(source) else {
            return Err(CompilerError::Internal {
                message: format!(
                    "generic parameter {:?} has no declaration symbol",
                    id.into_global(self.module)
                ),
            });
        };

        Ok(symbol)
    }

    /// Walk one runtime parameter.
    ///
    /// Example:
    /// ```ds
    /// (value: T = defaultValue)
    /// ```
    pub(in crate::check) fn walk_parameter(
        &mut self,
        template: Option<GenericTemplateId>,
        id: dir::LocalNodeId<dir::Parameter>,
        parameter: &dir::Parameter,
        is_annotation_required: bool,
    ) -> CompilerResult<()> {
        if !self.decide_decorated_presence(id.into_any())? {
            return Ok(());
        }

        match parameter {
            // (p: T), (p: ...T)
            dir::Parameter::Named {
                declared_type,
                default,
                is_comptime,
                ..
            } => {
                let (declared_type, default, is_comptime) =
                    (*declared_type, *default, *is_comptime);

                // report missing annotations
                if is_annotation_required && declared_type.is_none() {
                    self.check
                        .report_missing_type_annotation(self.module, id.into_any());
                }

                let symbol = self
                    .check
                    .module(self.module)
                    .declaration_symbol(id.into_any());
                let parameter_type = self.walk_parameter_type(id)?;

                // bind the parameter name to its type
                if let Some(symbol) = symbol {
                    if is_comptime {
                        self.induce_comptime_parameter(
                            template,
                            id.into_any(),
                            Some(symbol),
                            parameter_type,
                            default,
                            false,
                        )?;
                    } else if let Some(parameter_type) = parameter_type {
                        self.bind_symbol_type(symbol, parameter_type)?;
                    }
                }

                // check default after the parameter type is known
                if let Some(default) = default {
                    let before_default = self.fork_flow();
                    let expectation = parameter_type.map(|parameter_type| {
                        Expectation::assignable(
                            parameter_type,
                            Origin::Node(default.into_global_any(self.module)),
                            ValueUse::Store,
                        )
                    });
                    self.walk_expression(default, self.tree.get(default), expectation.as_ref())?;
                    self.restore_flow(before_default);
                }
            }
            // (...p: T)
            dir::Parameter::VariadicNamed {
                declared_type,
                is_comptime,
                ..
            } => {
                let (declared_type, is_comptime) = (*declared_type, *is_comptime);

                // report missing annotations
                if is_annotation_required && declared_type.is_none() {
                    self.check
                        .report_missing_type_annotation(self.module, id.into_any());
                }

                let symbol = self
                    .check
                    .module(self.module)
                    .declaration_symbol(id.into_any());
                let parameter_type = self.walk_parameter_type(id)?;

                // bind the variadic parameter name to its type
                if let Some(symbol) = symbol {
                    if is_comptime {
                        self.induce_comptime_parameter(
                            template,
                            id.into_any(),
                            Some(symbol),
                            parameter_type,
                            None,
                            true,
                        )?;
                    } else if let Some(parameter_type) = parameter_type {
                        self.bind_symbol_type(symbol, parameter_type)?;
                    }
                }
            }
            // ({ p }: T), (...{ p }: T)
            dir::Parameter::Pattern {
                pattern,
                declared_type,
                default,
                ..
            } => {
                let (pattern, declared_type, default) = (*pattern, *declared_type, *default);

                // report missing annotations
                if is_annotation_required && declared_type.is_none() {
                    self.check
                        .report_missing_type_annotation(self.module, id.into_any());
                }

                // constrain pattern type from the parameter type
                self.walk_pattern(pattern, self.tree.get(pattern))?;
                let parameter_type = self.walk_parameter_type(id)?;
                if let Some(parameter_type) = parameter_type {
                    self.write_node_type(pattern, parameter_type)?;
                    self.queue_node_task(pattern)?;
                }

                // check default after the parameter type is known
                if let Some(default) = default {
                    let before_default = self.fork_flow();
                    let expectation = parameter_type.map(|parameter_type| {
                        Expectation::assignable(
                            parameter_type,
                            Origin::Node(default.into_global_any(self.module)),
                            ValueUse::Store,
                        )
                    });
                    self.walk_expression(default, self.tree.get(default), expectation.as_ref())?;
                    self.restore_flow(before_default);
                }
            }
            dir::Parameter::VariadicPattern {
                pattern,
                declared_type,
                ..
            } => {
                let (pattern, declared_type) = (*pattern, *declared_type);

                // report missing annotations
                if is_annotation_required && declared_type.is_none() {
                    self.check
                        .report_missing_type_annotation(self.module, id.into_any());
                }

                // constrain pattern type from the parameter type
                self.walk_pattern(pattern, self.tree.get(pattern))?;
                if let Some(parameter_type) = self.walk_parameter_type(id)? {
                    self.write_node_type(pattern, parameter_type)?;
                    self.queue_node_task(pattern)?;
                }
            }
            // ignore damaged nodes
            dir::Parameter::Error => {}
        }

        Ok(())
    }

    /// Induce one comptime parameter as a static generic parameter.
    ///
    /// Example:
    /// ```ds
    /// function repeat(value: string, comptime count: uint): [string; count]
    /// ```
    pub(in crate::check) fn induce_comptime_parameter(
        &mut self,
        template: Option<GenericTemplateId>,
        source: dir::LocalNodeIdAny,
        symbol: Option<dir::GlobalSymbolId>,
        parameter_type: Option<dir::GlobalTypeId>,
        default: Option<dir::LocalNodeId<dir::Expression>>,
        is_variadic: bool,
    ) -> CompilerResult<Option<GenericParameterId>> {
        let Some(template) = template else {
            return Err(CompilerError::Internal {
                message: format!(
                    "comptime parameter {:?} has no generic template",
                    source.into_global(self.module)
                ),
            });
        };

        // open a generated static parameter when the callable type has no label
        let Some(symbol) = symbol else {
            let parameter = GenericInductionParameter {
                name_prefix: "C",
                constraint: parameter_type,
                is_comptime: true,
                induction: dir::GenericParameterInduction::Comptime,
            };
            let parameter = self
                .check
                .push_induced_generic_parameter(template, parameter)?;

            return Ok(Some(parameter));
        };

        // open the named static parameter
        let default = default
            .map(|default| self.walk_static_term(default))
            .transpose()?;
        let binding = dir::GenericParameterBinding {
            template: template.local_id,
            key: dir::GenericParameterKey::Symbol(symbol),
            variance: None,
            constraint: parameter_type,
            default,
            origin: dir::GenericParameterOrigin::Induced(dir::GenericParameterInduction::Comptime),
            is_variadic,
            is_const: false,
            is_comptime: true,
        };
        let parameter = self
            .check
            .push_generic_parameter(binding, template, Some(symbol))?;

        // write the parameter name as its own parameter type
        let ty = self.push_type(dir::Type::Parameter(parameter), source)?;
        self.bind_symbol_type(symbol, ty)?;

        Ok(Some(parameter))
    }
}
