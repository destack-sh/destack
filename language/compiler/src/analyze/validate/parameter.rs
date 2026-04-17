use crate::analyze::common::TypeContext;
use crate::{AnalyzeError, Compiler};
use destack_dir::{
    Declaration, FunctionMode, LocalNodeId, Member, NodeTree, NodeType, Parameter, Property,
};

#[allow(clippy::collapsible_match, clippy::too_many_arguments)]
impl Compiler {
    /// Return whether one parameter is a parameter property.
    fn parameter_is_property(&self, parameter: &Parameter) -> bool {
        match parameter {
            Parameter::Named {
                visibility,
                is_readonly,
                ..
            }
            | Parameter::VariadicNamed {
                visibility,
                is_readonly,
                ..
            } => visibility.is_some() || *is_readonly,
            Parameter::Pattern { .. }
            | Parameter::VariadicPattern { .. }
            | Parameter::Error { .. } => false,
        }
    }

    /// Return whether one parameter has an authored type annotation.
    fn parameter_has_declared_type(&self, parameter: &Parameter) -> bool {
        match parameter {
            Parameter::Named { declared_type, .. }
            | Parameter::Pattern { declared_type, .. }
            | Parameter::VariadicNamed { declared_type, .. }
            | Parameter::VariadicPattern { declared_type, .. } => declared_type.is_some(),
            Parameter::Error { .. } => false,
        }
    }

    /// Return whether one parameter is optional in syntax.
    fn parameter_is_optional(&self, parameter: &Parameter) -> bool {
        match parameter {
            Parameter::Named { is_optional, .. } | Parameter::Pattern { is_optional, .. } => {
                *is_optional
            }
            Parameter::VariadicNamed { .. }
            | Parameter::VariadicPattern { .. }
            | Parameter::Error { .. } => false,
        }
    }

    /// Check whether a parameter belongs to a constructor.
    fn is_in_constructor(&self, tree: &NodeTree, parameter_id: LocalNodeId<Parameter>) -> bool {
        // read the parent node
        let Some(parent) = tree.get_parent(parameter_id.id) else {
            return false;
        };

        // validate constructor ownership
        match parent.ty {
            NodeType::Member => {
                let member = tree.get(LocalNodeId::<Member>::new(parent.id));
                matches!(
                    member,
                    Member::Method { signature, .. }
                        if signature.mode == Some(FunctionMode::Constructor)
                )
            }
            NodeType::Property => {
                let property = tree.get(LocalNodeId::<Property>::new(parent.id));
                matches!(
                    property,
                    Property::Method { signature, .. }
                        if signature.mode == Some(FunctionMode::Constructor)
                )
            }
            NodeType::Declaration => {
                let declaration = tree.get(LocalNodeId::<Declaration>::new(parent.id));
                matches!(
                    declaration,
                    Declaration::Function(declaration)
                        if declaration.signature.mode == Some(FunctionMode::Constructor)
                )
            }
            _ => false,
        }
    }

    /// Validate a parameter.
    pub(super) fn validate_parameter(
        &self,
        ctx: &mut TypeContext<'_>,
        id: LocalNodeId<Parameter>,
        parameter: &Parameter,
    ) {
        // normalize parameter property state
        let is_parameter_property = self.parameter_is_property(parameter);

        // classify explicit type annotations on parameters
        let has_declared_type = self.parameter_has_declared_type(parameter);

        // reject parameter properties in javascript modules
        if is_parameter_property && ctx.module.language_type.is_javascript() {
            let node = id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
            self.error(AnalyzeError::TypeScriptSyntaxInJavaScript { node });
        }

        // reject parameter type annotations in javascript modules
        if ctx.module.language_type.is_javascript() && has_declared_type {
            let node = id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
            self.error(AnalyzeError::TypeScriptSyntaxInJavaScript { node });
        }

        // reject optional parameters in javascript modules
        if ctx.module.language_type.is_javascript() && self.parameter_is_optional(parameter) {
            let node = id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
            self.error(AnalyzeError::TypeScriptSyntaxInJavaScript { node });
        }

        // parameter properties only allowed in constructors
        if is_parameter_property && !self.is_in_constructor(ctx.tree, id) {
            let node = id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
            self.error(AnalyzeError::InvalidParameterProperty { node });
        }

        // parameter properties cannot use binding patterns
        let is_binding_pattern = matches!(
            parameter,
            Parameter::Pattern { .. } | Parameter::VariadicPattern { .. }
        );
        if is_parameter_property && is_binding_pattern {
            let node = id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
            self.error(AnalyzeError::InvalidParameterProperty { node });
        }

        // parameter properties cannot use rest bindings
        let is_rest_parameter = matches!(
            parameter,
            Parameter::VariadicNamed { .. } | Parameter::VariadicPattern { .. }
        );
        if is_parameter_property && is_rest_parameter {
            let node = id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
            self.error(AnalyzeError::InvalidParameterProperty { node });
        }

        // optional pattern or rest parameters are not valid
        if self.parameter_is_optional(parameter) {
            let node = id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
            match parameter {
                Parameter::Pattern { .. } => {
                    self.error(AnalyzeError::InvalidOptionalPatternParameter { node });
                }
                Parameter::VariadicNamed { .. } | Parameter::VariadicPattern { .. } => {
                    self.error(AnalyzeError::InvalidOptionalRestParameter { node });
                }
                Parameter::Named { .. } | Parameter::Error { .. } => {}
            }
        }

        // JS/TS parameter patterns must only contain assignment targets
        if ctx.module.language_type.is_javascript() || ctx.module.language_type.is_typescript() {
            match parameter {
                Parameter::Pattern { pattern, .. } | Parameter::VariadicPattern { pattern, .. } => {
                    self.validate_for_each_assignment_pattern(&mut ctx.reborrow(), *pattern, true);
                }
                Parameter::Named { .. }
                | Parameter::VariadicNamed { .. }
                | Parameter::Error { .. } => {}
            }
        }
        // variadic destructuring bindings must contain assignment targets
        else if let Parameter::VariadicPattern { pattern, .. } = parameter {
            self.validate_for_each_assignment_pattern(&mut ctx.reborrow(), *pattern, true);
        }
    }
}
