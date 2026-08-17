use destack_dir as dir;

use crate::sema::{CauseKind, GenericParameterId, GenericTemplateId, Origin, ValueUse, WalkState};
use crate::{CompilerError, CompilerResult};

impl WalkState<'_, '_> {
    /// Walk decorators on one declared callable's parameters.
    pub(in crate::sema) fn walk_parameter_decorators(
        &mut self,
        this_parameter: Option<dir::LocalNodeId<dir::Parameter>>,
        parameters: &[dir::LocalNodeId<dir::Parameter>],
    ) -> CompilerResult<()> {
        // walk the explicit receiver
        if let Some(parameter) = this_parameter {
            self.walk_decorators(parameter.into_any())?;
        }

        // walk ordinary parameters
        for parameter in parameters {
            self.walk_decorators(parameter.into_any())?;
        }

        Ok(())
    }

    /// Open one generic parameter header.
    ///
    /// Example:
    /// ```ds
    /// <T: Serializable = string>
    /// ```
    pub(in crate::sema) fn open_generic_parameter(
        &mut self,
        template: GenericTemplateId,
        id: dir::LocalNodeId<dir::GenericParameter>,
        generic_parameter: &dir::GenericParameter,
    ) -> CompilerResult<Option<GenericParameterId>> {
        if !self.decide_static_presence(id.into_any())? {
            return Ok(None);
        }
        let symbol = self.generic_parameter_symbol(id)?;

        if let Some(parameter) = self.check.parameter_by_symbol(symbol) {
            return Ok(Some(parameter));
        }

        // shape the parameter binding by its declared kind
        let (variance, kind, is_variadic, is_const) = match generic_parameter {
            // <T>
            dir::GenericParameter::Type {
                variance, is_const, ..
            } => (*variance, dir::GenericParameterKind::Type, false, *is_const),
            // <...T>
            dir::GenericParameter::VariadicType {
                variance, is_const, ..
            } => (*variance, dir::GenericParameterKind::Type, true, *is_const),
            // <'a>
            dir::GenericParameter::Lifetime { .. } => (
                None,
                dir::GenericParameterKind::Memory(dir::MemoryParameter::Lifetime),
                false,
                false,
            ),
            // ignore damaged nodes
            dir::GenericParameter::Error => return Ok(None),
        };
        let parameter = self.check.push_generic_parameter(
            template,
            id.into_global_any(self.module),
            Some(symbol),
            dir::GenericParameterKey::Symbol(symbol),
            variance,
            None,
            None,
            dir::GenericParameterOrigin::Explicit,
            kind,
            is_variadic,
            is_const,
        )?;

        // the parameter name writes its own parameter type
        let ty = self.check.generic_parameter_type(parameter)?;
        self.bind_symbol_type(symbol, ty)?;

        Ok(Some(parameter))
    }

    /// Walk one generic parameter's bounds.
    ///
    /// Example:
    /// ```ds
    /// <T: Serializable = string>
    /// ```
    pub(in crate::sema) fn walk_generic_parameter(
        &mut self,
        template: GenericTemplateId,
        id: dir::LocalNodeId<dir::GenericParameter>,
        generic_parameter: &dir::GenericParameter,
    ) -> CompilerResult<()> {
        if !self.walk_decorators(id.into_any())? {
            return Ok(());
        }
        let Some(parameter) = self.open_generic_parameter(template, id, generic_parameter)? else {
            return Ok(());
        };

        match generic_parameter {
            // <T: U = V>, <...T: U = V>
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
            // <'a>
            dir::GenericParameter::Lifetime { .. } => {
                let constraint = self.language_type_reference(dir::LanguageItem::Lifetime, &[])?;

                self.check
                    .update_generic_parameter_bounds(parameter, Some(constraint), None)?;
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
    pub(in crate::sema) fn walk_parameter(
        &mut self,
        id: dir::LocalNodeId<dir::Parameter>,
        parameter: &dir::Parameter,
        is_annotation_required: bool,
        represents_open_type: bool,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        if !self.walk_decorators(id.into_any())? {
            return Ok(None);
        }

        let mut result = None;
        match parameter {
            // (p: T), (p: ...T)
            dir::Parameter::Named {
                declared_type,
                default,
                ..
            } => {
                let (declared_type, default) = (*declared_type, *default);

                // report missing annotations
                if is_annotation_required && declared_type.is_none() {
                    self.check
                        .report_missing_type_annotation(self.module, id.into_any());
                }

                let parameter_type = self.walk_parameter_type(id, represents_open_type)?;

                // validate defaults while checking, declaring transcribes them
                if let Some(default) = default.filter(|_| !self.check.is_declaration()) {
                    let before_default = self.fork_flow();
                    self.walk_expression(default, self.tree.get(default))?;
                    if let Some(parameter_type) = parameter_type {
                        // check the default against the narrowed body binding
                        let origin = Origin::Node(
                            id.into_global_any(self.module),
                            self.flow().template_scope(),
                        );
                        let target = self.defaulted_value_type(origin, parameter_type)?;
                        let annotation =
                            declared_type.map(|annotation| annotation.into_global_any(self.module));
                        self.check_assignable(
                            default,
                            target,
                            CauseKind::Initializer { annotation },
                            ValueUse::Store,
                        )?;
                    }
                    self.restore_flow(before_default);
                }

                result = parameter_type;
            }
            // (...p: T)
            dir::Parameter::VariadicNamed { declared_type, .. } => {
                let declared_type = *declared_type;

                // report missing annotations
                if is_annotation_required && declared_type.is_none() {
                    self.check
                        .report_missing_type_annotation(self.module, id.into_any());
                }

                let parameter_type = self.walk_parameter_type(id, represents_open_type)?;

                result = parameter_type;
            }
            // ({ p }: T), (...{ p }: T)
            dir::Parameter::Pattern { declared_type, .. }
            | dir::Parameter::VariadicPattern { declared_type, .. } => {
                // report missing annotations
                if is_annotation_required && declared_type.is_none() {
                    self.check
                        .report_missing_type_annotation(self.module, id.into_any());
                }

                // build the declared parameter type only, the body pass
                //  destructures the pattern against it later
                result = self.walk_parameter_type(id, represents_open_type)?;
            }
            // ignore damaged nodes
            dir::Parameter::Error => {}
        }

        Ok(result)
    }
}
