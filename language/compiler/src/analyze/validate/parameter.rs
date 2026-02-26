use crate::analyze::common::TypeContext;
use crate::{AnalyzeError, Compiler};
use destack_dir::{
    BindingKind, BindingModifier, Declaration, FunctionMode, LocalNodeId, Member, Mutability,
    NodeTree, NodeType, Parameter, Property, SymbolSpace,
};

#[allow(clippy::collapsible_match, clippy::too_many_arguments)]
impl Compiler {
    /// Return true when variance is allowed for this parameter.
    fn is_parameter_variable(&self, tree: &NodeTree, parameter_id: LocalNodeId<Parameter>) -> bool {
        let Some(parent) = tree.get_parent(parameter_id.id) else {
            return false;
        };

        // declaration parameters
        if parent.ty == NodeType::Declaration {
            let declaration = tree.get(parent.into_typed::<Declaration>());
            return matches!(
                declaration,
                Declaration::Class { .. }
                    | Declaration::Interface { .. }
                    | Declaration::Type { .. }
                    | Declaration::Function { .. }
            );
        }

        // method parameters
        if parent.ty == NodeType::Member {
            let member = tree.get(parent.into_typed::<Member>());
            return matches!(member, Member::Method { .. });
        }

        // object method parameters
        if parent.ty == NodeType::Property {
            let property = tree.get(parent.into_typed::<Property>());
            return matches!(property, Property::Method { .. });
        }

        false
    }

    /// Check whether a binding modifier indicates a parameter property.
    fn is_parameter_property(&self, modifiers: &BindingModifier) -> bool {
        modifiers.visibility.is_some() || modifiers.mutability == Some(Mutability::Immutable)
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
                    Declaration::Function { signature, .. }
                        if signature.mode == Some(FunctionMode::Constructor)
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
        let modifiers = parameter.modifiers();
        let is_parameter_property =
            modifiers.is_some_and(|modifiers| self.is_parameter_property(modifiers));

        // classify explicit type annotations on parameters
        let has_declared_type = ctx
            .types
            .get_declared_type_id(id.into_global_any(ctx.module.id))
            .is_some();

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
        if ctx.module.language_type.is_javascript()
            && modifiers.is_some_and(|modifiers| modifiers.kind == Some(BindingKind::Maybe))
        {
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
        let is_optional =
            modifiers.is_some_and(|modifiers| modifiers.kind == Some(BindingKind::Maybe));
        if is_optional {
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
                Parameter::Named { .. } => {}
            }
        }

        // JS/TS parameter patterns must only contain assignment targets
        if ctx.module.language_type.is_javascript() || ctx.module.language_type.is_typescript() {
            match parameter {
                Parameter::Pattern { pattern, .. } | Parameter::VariadicPattern { pattern, .. } => {
                    self.validate_for_each_assignment_pattern(&mut ctx.reborrow(), *pattern, true);
                }
                Parameter::Named { .. } | Parameter::VariadicNamed { .. } => {}
            }
        }
        // variadic destructuring bindings must contain assignment targets
        else if let Parameter::VariadicPattern { pattern, .. } = parameter {
            self.validate_for_each_assignment_pattern(&mut ctx.reborrow(), *pattern, true);
        }

        // variance modifiers are restricted
        if let Some(modifiers) = modifiers
            && (modifiers.variance.is_some() || modifiers.visibility.is_some())
        {
            let symbol = ctx.symbols.get_symbol(parameter.symbol());
            let is_type_parameter = symbol.space == SymbolSpace::Type;

            if modifiers.visibility.is_some() && is_type_parameter {
                let node = id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile));
                self.error(AnalyzeError::InvalidTypeParameterModifier { node });
            }

            if modifiers.variance.is_some() {
                let is_allowed = is_type_parameter && self.is_parameter_variable(ctx.tree, id);
                if !is_allowed {
                    let node = id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile));
                    self.error(AnalyzeError::InvalidTypeParameterModifier { node });
                }
            }
        }
    }
}
