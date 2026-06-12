use destack_dir as dir;

use crate::check::{GenericParameterId, GenericTemplateId, Origin, Relation, WalkState};
use crate::{CompilerError, CompilerResult};

impl WalkState<'_, '_> {
    /// Declare one generic parameter header.
    ///
    /// Example:
    /// ```ds
    /// <T extends Serializable = string>
    /// ```
    pub(in crate::check) fn declare_generic_parameter(
        &mut self,
        template: GenericTemplateId,
        id: dir::LocalNodeId<dir::GenericParameter>,
        generic_parameter: &dir::GenericParameter,
    ) -> CompilerResult<Option<GenericParameterId>> {
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any())? else {
            return Ok(None);
        };
        let symbol = self.generic_parameter_symbol(id)?;

        if let Some(parameter) = self.check.generics.parameter_by_symbol(symbol) {
            return Ok(Some(parameter));
        }

        // shape the parameter binding by its declared kind
        let (variance, is_variadic, is_comptime) = match generic_parameter {
            // <T>
            dir::GenericParameter::Type { variance, .. } => (*variance, false, false),
            // <...T>
            dir::GenericParameter::VariadicType { variance, .. } => (*variance, true, false),
            // <comptime C: T>
            dir::GenericParameter::Value { .. } => (None, false, true),
            // <comptime ...C: T>
            dir::GenericParameter::VariadicValue { .. } => (None, true, true),
            // ignore damaged syntax
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
            is_comptime,
        };
        let parameter = self
            .check
            .declare_generic_parameter(binding, template, Some(symbol))?;

        // the parameter name writes its own parameter type
        let ty = self.push_type(dir::Type::Parameter(parameter), id.into_any())?;
        self.declare_symbol_type(symbol, ty)?;

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
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any())? else {
            return Ok(());
        };
        let Some(parameter) = self.declare_generic_parameter(template, id, generic_parameter)?
        else {
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
                        self.walk_expression(default, self.tree.get(default))?;
                        self.restore_flow(before_default);

                        self.lower_static_predicate(default)
                    })
                    .transpose()?;

                self.check
                    .update_generic_parameter_bounds(parameter, constraint, default)?;
            }
            // ignore damaged syntax
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
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any())? else {
            return Ok(());
        };

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
                if let Some(default) = default {
                    // check parameter defaults in declaration context
                    let before_default = self.fork_flow();
                    self.walk_expression(default, self.tree.get(default))?;
                    self.restore_flow(before_default);
                }

                let symbol = self
                    .check
                    .module(self.module)
                    .declaration_symbol(id.into_any());
                let parameter_type = self.parameter_type(id)?;

                // bind the parameter name to its type
                if let Some(symbol) = symbol {
                    if is_comptime {
                        self.induce_comptime_parameter(
                            template,
                            id,
                            symbol,
                            parameter_type,
                            default,
                            false,
                        )?;
                    } else if let Some(parameter_type) = parameter_type {
                        self.declare_symbol_type(symbol, parameter_type)?;
                    }
                }

                // defaults flow into the declared type
                if let (Some(default), Some(parameter_type)) = (default, parameter_type) {
                    self.expect_assignable(default, parameter_type)?;
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
                let parameter_type = self.parameter_type(id)?;

                // bind the variadic parameter name to its type
                if let Some(symbol) = symbol {
                    if is_comptime {
                        self.induce_comptime_parameter(
                            template,
                            id,
                            symbol,
                            parameter_type,
                            None,
                            true,
                        )?;
                    } else if let Some(parameter_type) = parameter_type {
                        self.declare_symbol_type(symbol, parameter_type)?;
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

                // walk pattern and optional children
                self.walk_pattern(pattern, self.tree.get(pattern))?;
                if let Some(default) = default {
                    let before_default = self.fork_flow();
                    self.walk_expression(default, self.tree.get(default))?;
                    self.restore_flow(before_default);
                }

                // flow the parameter type into the pattern holes
                if let Some(parameter_type) = self.parameter_type(id)? {
                    let origin = Origin::Node(pattern.into_global_any(self.module));
                    let pattern_type = self.node_type(pattern)?;
                    self.relate_type(origin, Relation::Assignable, parameter_type, pattern_type);

                    if let Some(default) = default {
                        self.expect_assignable(default, parameter_type)?;
                    }
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

                // walk pattern and flow the parameter type into its holes
                self.walk_pattern(pattern, self.tree.get(pattern))?;
                if let Some(parameter_type) = self.parameter_type(id)? {
                    let origin = Origin::Node(pattern.into_global_any(self.module));
                    let pattern_type = self.node_type(pattern)?;
                    self.relate_type(origin, Relation::Assignable, parameter_type, pattern_type);
                }
            }
            // ignore damaged syntax
            dir::Parameter::Error => {}
        }

        Ok(())
    }

    /// Induce one comptime runtime parameter as a static generic parameter.
    ///
    /// Example:
    /// ```ds
    /// function repeat(value: string, comptime count: uint): [string; count]
    /// ```
    fn induce_comptime_parameter(
        &mut self,
        template: Option<GenericTemplateId>,
        id: dir::LocalNodeId<dir::Parameter>,
        symbol: dir::GlobalSymbolId,
        parameter_type: Option<dir::GlobalTypeId>,
        default: Option<dir::LocalNodeId<dir::Expression>>,
        is_variadic: bool,
    ) -> CompilerResult<Option<GenericParameterId>> {
        let Some(template) = template else {
            return Err(CompilerError::Internal {
                message: format!(
                    "comptime parameter {:?} has no generic template",
                    id.into_global(self.module)
                ),
            });
        };

        // declare the hidden static parameter
        let default = default
            .map(|default| self.lower_static_predicate(default))
            .transpose()?;
        let binding = dir::GenericParameterBinding {
            template: template.local_id,
            key: dir::GenericParameterKey::Symbol(symbol),
            variance: None,
            constraint: parameter_type,
            default,
            origin: dir::GenericParameterOrigin::Induced(dir::GenericParameterInduction::Comptime),
            is_variadic,
            is_comptime: true,
        };
        let parameter = self
            .check
            .declare_generic_parameter(binding, template, Some(symbol))?;

        // the parameter name writes its own parameter type
        let ty = self.push_type(dir::Type::Parameter(parameter), id.into_any())?;
        self.declare_symbol_type(symbol, ty)?;

        Ok(Some(parameter))
    }
}
