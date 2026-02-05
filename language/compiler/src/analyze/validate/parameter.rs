use crate::{AnalyzeError, Compiler};
use destack_dir::{
    BindingKind, BindingModifier, Declaration, FunctionMode, LocalNodeId, Member, Mutability,
    NodeTree, NodeType, Parameter, Property, SymbolSpace, SymbolTable,
};
use destack_workspace::{Module, ProfileId};

#[allow(clippy::collapsible_match)]
impl Compiler {
    /// Return true when variance is allowed for this parameter.
    fn is_parameter_variable(
        &self,
        tree: &NodeTree,
        parameter_id: LocalNodeId<Parameter>,
    ) -> bool {
        let Some(parent) = tree.get_parent(parameter_id.id) else {
            return false;
        };
        if parent.ty != NodeType::Declaration {
            return false;
        }
        let declaration = tree.get(parent.into_typed::<Declaration>());
        matches!(
            declaration,
            Declaration::Class { .. } | Declaration::Interface { .. } | Declaration::Type { .. }
        )
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
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        id: LocalNodeId<Parameter>,
        parameter: &Parameter,
    ) {
        // normalize parameter property state
        let modifiers = parameter.modifiers();
        let is_parameter_property =
            modifiers.is_some_and(|modifiers| self.is_parameter_property(modifiers));

        // parameter properties only allowed in constructors
        if is_parameter_property && !self.is_in_constructor(tree, id) {
            let node = id.into_global_any(module.id).into_anchored(Some(profile));
            self.error(AnalyzeError::InvalidParameterProperty { node });
        }

        // parameter properties cannot use binding patterns
        let is_binding_pattern = matches!(
            parameter,
            Parameter::Pattern { .. } | Parameter::Variadic { .. }
        );
        if is_parameter_property && is_binding_pattern {
            let node = id.into_global_any(module.id).into_anchored(Some(profile));
            self.error(AnalyzeError::InvalidParameterProperty { node });
        }

        // optional pattern or rest parameters are not valid in TS/JS
        if !module.language_type.is_destack() {
            let is_optional =
                modifiers.is_some_and(|modifiers| modifiers.kind == Some(BindingKind::Maybe));
            if is_optional {
                let node = id.into_global_any(module.id).into_anchored(Some(profile));
                match parameter {
                    Parameter::Pattern { .. } => {
                        self.error(AnalyzeError::InvalidOptionalPatternParameter { node });
                    }
                    Parameter::Variadic { .. } => {
                        self.error(AnalyzeError::InvalidOptionalRestParameter { node });
                    }
                    Parameter::Named { .. } => {}
                }
            }
        }

        // variance modifiers are restricted
        if let Some(modifiers) = modifiers
            && (modifiers.variance.is_some() || modifiers.visibility.is_some())
        {
            let symbol = symbols.get_symbol(parameter.symbol());
            let is_type_parameter = symbol.space == SymbolSpace::Type;

            if modifiers.visibility.is_some() && is_type_parameter {
                let node = id.into_global_any(module.id).into_anchored(Some(profile));
                self.error(AnalyzeError::InvalidTypeParameterModifier { node });
            }

            if modifiers.variance.is_some() {
                let is_allowed = is_type_parameter && self.is_parameter_variable(tree, id);
                if !is_allowed {
                    let node = id.into_global_any(module.id).into_anchored(Some(profile));
                    self.error(AnalyzeError::InvalidTypeParameterModifier { node });
                }
            }
        }
    }
}
