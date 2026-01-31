use crate::{AnalyzeError, Compiler};
use destack_dir::{
    BindingModifier, Declaration, FunctionMode, GlobalNodeIdAny, LocalNodeId, Member, Mutability,
    NodeTree, NodeType, Parameter, Property,
};
use destack_workspace::{Module, ProfileId};

#[allow(clippy::collapsible_match)]
impl Compiler {
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
        id: LocalNodeId<Parameter>,
        parameter: &Parameter,
    ) {
        // normalize parameter property state
        let modifiers = parameter.modifiers();
        let is_parameter_property =
            modifiers.is_some_and(|modifiers| self.is_parameter_property(modifiers));

        // parameter properties only allowed in constructors
        if is_parameter_property && !self.is_in_constructor(tree, id) {
            let node = GlobalNodeIdAny::new(module.id, id.into_any()).into_anchored(Some(profile));
            self.error(AnalyzeError::InvalidParameterProperty { node });
        }

        // parameter properties cannot use binding patterns
        let is_binding_pattern = matches!(
            parameter,
            Parameter::Pattern { .. } | Parameter::Variadic { .. }
        );
        if is_parameter_property && is_binding_pattern {
            let node = GlobalNodeIdAny::new(module.id, id.into_any()).into_anchored(Some(profile));
            self.error(AnalyzeError::InvalidParameterProperty { node });
        }
    }
}
