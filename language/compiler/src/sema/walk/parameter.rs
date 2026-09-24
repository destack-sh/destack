use destack_dir as dir;

use crate::sema::{
    CauseKind, ElisionSite, GenericParameterId, GenericTemplateId, Origin, ValueUse, WalkState,
};
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

        if let Some(parameter) = self.check.parameter_by_symbol(symbol)? {
            return Ok(Some(parameter));
        }

        // shape the parameter binding by its declaration and the item its constraint names
        let (variance, constraint, is_variadic, is_const) = match generic_parameter {
            // <T>
            dir::GenericParameter::Type {
                variance,
                constraint,
                is_const,
                ..
            } => (*variance, *constraint, false, *is_const),
            // <...T>
            dir::GenericParameter::VariadicType {
                variance,
                constraint,
                is_const,
                ..
            } => (*variance, *constraint, true, *is_const),
            // <'a>
            dir::GenericParameter::Lifetime { .. } => (None, None, false, false),
            // ignore damaged nodes
            dir::GenericParameter::Error => return Ok(None),
        };
        let resolved = self.check.module.resolved.clone();
        let kind = self.check.declared_parameter_kind(
            self.module,
            &self.tree,
            &resolved,
            generic_parameter,
            constraint,
        )?;

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

        // write the parameter type under the parameter name
        let ty = self.check.generic_parameter_type(parameter)?;
        self.commit_symbol_type(symbol, ty)?;

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

                // induce elided bound borrows on the declaring template
                let (constraint, default) = self.with_template_owner(template, |walk| {
                    let constraint = constraint
                        .map(|constraint| {
                            walk.walk_type_expression_in(constraint, ElisionSite::Signature)
                        })
                        .transpose()?;
                    let default = default
                        .map(|default| {
                            walk.walk_type_expression_in(default, ElisionSite::Signature)
                        })
                        .transpose()?;

                    Ok((constraint, default))
                })?;

                self.check
                    .update_generic_parameter_bounds(parameter, constraint, default)?;
            }
            // <'a>
            dir::GenericParameter::Lifetime { .. } => {
                let constraint = self.language_type(dir::LanguageItem::Region, &[])?;

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
        is_ambient: bool,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        if !self.walk_decorators(id.into_any())? {
            return Ok(None);
        }

        // build the declared type for the parameter form
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

                let parameter_type = self.walk_parameter_type(id)?;

                // reject a default on an ambient signature
                if let Some(default) = default.filter(|_| is_ambient) {
                    self.check
                        .report_parameter_initializer_outside_implementation(
                            self.module,
                            default.into_any(),
                        );
                }
                // validate defaults while checking, the declaring pass copies them as written
                else if let Some(default) = default.filter(|_| !self.check.is_declaring()) {
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

                let parameter_type = self.walk_parameter_type(id)?;

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

                // build the declared parameter type only
                result = self.walk_parameter_type(id)?;
            }
            // ignore damaged nodes
            dir::Parameter::Error => {}
        }

        Ok(result)
    }
}
