use crate::check::{
    GenericParameter, GenericParameterBinding, GenericParameterId, GenericTemplateId, Origin,
    PatternRelation, StaticTerm, TypeOperand, TypeRelation, TypeTerm, WalkState,
};
use crate::{CompilerError, CompilerResult};
use destack_dir as dir;

impl WalkState<'_, '_> {
    /// Walk one generic parameter.
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
        // enter static decorator guard
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any(), None)? else {
            return Ok(());
        };

        match generic_parameter {
            // <T>
            dir::GenericParameter::Type {
                variance,
                constraint,
                default,
                ..
            } => {
                let parameter = self.bind_type_generic_parameter(template, id)?;

                // walk optional bounds
                if let Some(constraint) = constraint {
                    self.walk_type_expression(*constraint, self.tree.get(*constraint))?;
                }
                if let Some(default) = default {
                    self.walk_type_expression(*default, self.tree.get(*default))?;
                }

                // insert checked parameter
                {
                    let constraint = constraint
                        .map(|id| self.node_type_operand(id))
                        .transpose()?
                        .map(Into::into);
                    let default = default.map(|id| self.node_type_operand(id)).transpose()?;

                    self.check
                        .inference
                        .insert_generic_parameter(GenericParameterBinding::r#type(
                            parameter, *variance, constraint, default,
                        ));
                }
            }
            // <...T>
            dir::GenericParameter::VariadicType {
                variance,
                constraint,
                default,
                ..
            } => {
                let parameter = self.bind_type_generic_parameter(template, id)?;

                // walk optional bounds
                if let Some(constraint) = constraint {
                    self.walk_type_expression(*constraint, self.tree.get(*constraint))?;
                }
                if let Some(default) = default {
                    self.walk_type_expression(*default, self.tree.get(*default))?;
                }

                // insert checked parameter
                {
                    let constraint = constraint
                        .map(|id| self.node_type_operand(id))
                        .transpose()?
                        .map(Into::into);
                    let default = default.map(|id| self.node_type_operand(id)).transpose()?;

                    self.check.inference.insert_generic_parameter(
                        GenericParameterBinding::variadic_type(
                            parameter, *variance, constraint, default,
                        ),
                    );
                }
            }
            // <comptime C: T>
            dir::GenericParameter::Value {
                declared_type,
                default,
                ..
            } => {
                let parameter = self.bind_static_generic_parameter(template, id)?;

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

                // insert checked parameter
                {
                    let constraint = declared_type
                        .map(|id| self.node_type_operand(id))
                        .transpose()?
                        .map(Into::into);
                    let default = default
                        .map(|id| {
                            let condition = self.active_static_guard();

                            self.static_expression_variable(id, condition)
                        })
                        .transpose()?
                        .map(Into::into);

                    self.check.inference.insert_generic_parameter(
                        GenericParameterBinding::r#static(parameter, constraint, default),
                    );
                }
            }
            // <comptime ...C: T>
            dir::GenericParameter::VariadicValue {
                declared_type,
                default,
                ..
            } => {
                let parameter = self.bind_static_generic_parameter(template, id)?;

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

                // insert checked parameter
                {
                    let constraint = declared_type
                        .map(|id| self.node_type_operand(id))
                        .transpose()?
                        .map(Into::into);
                    let default = default
                        .map(|id| {
                            let condition = self.active_static_guard();

                            self.static_expression_variable(id, condition)
                        })
                        .transpose()?
                        .map(Into::into);

                    self.check.inference.insert_generic_parameter(
                        GenericParameterBinding::variadic_static(parameter, constraint, default),
                    );
                }
            }
            // ignore damaged syntax
            dir::GenericParameter::Error => {}
        }

        Ok(())
    }

    /// Allocate the type parameter introduced by one generic parameter.
    ///
    /// Example:
    /// ```ds
    /// <T extends Serializable = string>
    /// ```
    fn bind_type_generic_parameter(
        &mut self,
        template: GenericTemplateId,
        id: dir::LocalNodeId<dir::GenericParameter>,
    ) -> CompilerResult<GenericParameter> {
        let symbol = self.generic_parameter_symbol(id)?;
        let parameter = self.check.allocate_symbol_generic_parameter(
            template,
            symbol,
            dir::GenericParameterOrigin::Explicit,
        );
        let parameter_id = parameter.id();
        let condition = self.active_static_guard();

        self.bind_symbol_type(symbol, TypeTerm::Parameter(parameter_id), condition)?;

        Ok(parameter)
    }

    /// Allocate the static parameter introduced by one generic parameter.
    ///
    /// Example:
    /// ```ds
    /// <comptime Size: number = 4>
    /// ```
    fn bind_static_generic_parameter(
        &mut self,
        template: GenericTemplateId,
        id: dir::LocalNodeId<dir::GenericParameter>,
    ) -> CompilerResult<GenericParameter> {
        let symbol = self.generic_parameter_symbol(id)?;
        let parameter = self.check.allocate_symbol_generic_parameter(
            template,
            symbol,
            dir::GenericParameterOrigin::Explicit,
        );
        let parameter_id = parameter.id();
        let condition = self.active_static_guard();

        self.bind_symbol_static(symbol, StaticTerm::Parameter(parameter_id), condition)?;

        Ok(parameter)
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

    /// Walk one parameter.
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
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any(), None)? else {
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
                            template,
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
                    self.constrain_parameter_default(*default, parameter_type)?;
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
                            template,
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
                    if let Some(term) = self.pattern_term(self.module, *pattern)? {
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
                        self.constrain_parameter_default(*default, parameter_type)?;
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
                    if let Some(term) = self.pattern_term(self.module, *pattern)? {
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
        template: Option<GenericTemplateId>,
        id: dir::LocalNodeId<dir::Parameter>,
        symbol: dir::GlobalSymbolId,
        parameter_type: Option<TypeOperand>,
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
        let parameter = self.check.allocate_symbol_generic_parameter(
            template,
            symbol,
            dir::GenericParameterOrigin::Induced(dir::GenericParameterInduction::Comptime),
        );
        let parameter_id = parameter.id();
        let default = default
            .map(|id| {
                let condition = self.active_static_guard();

                self.static_expression_variable(id, condition)
            })
            .transpose()?
            .map(Into::into);
        let generic = if is_variadic {
            GenericParameterBinding::variadic_static(parameter, parameter_type, default)
        } else {
            GenericParameterBinding::r#static(parameter, parameter_type, default)
        };

        self.check.inference.insert_generic_parameter(generic);
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
        default: dir::LocalNodeId<dir::Expression>,
        declared_type: TypeOperand,
    ) -> CompilerResult<()> {
        let value = self.node_type_operand(default)?;
        let origin = Origin::Node(default.into_global_any(self.module));
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
