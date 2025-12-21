use crate::{AnalyzeError, Compiler};
use destack_dir::{
    BindingModifier, Declaration, FunctionMode, GlobalNodeIdAny, LocalNodeId, Member, Mutability,
    NodeTree, NodeType, Parameter, Property,
};
use destack_workspace::Module;

#[allow(clippy::collapsible_match)]
impl Compiler {
    /// Check if a binding modifier indicates a parameter property (visibility or readonly).
    fn is_parameter_property(&self, modifiers: &BindingModifier) -> bool {
        modifiers.visibility.is_some() || modifiers.mutability == Some(Mutability::Immutable)
    }

    /// Check if a parameter's parent is a constructor.
    fn is_in_constructor(&self, tree: &NodeTree, parameter_id: LocalNodeId<Parameter>) -> bool {
        let Some(parent) = tree.get_parent(parameter_id.id) else {
            return false;
        };
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
        tree: &NodeTree,
        id: LocalNodeId<Parameter>,
        parameter: &Parameter,
    ) {
        // parameter properties only allowed in constructors
        if let Some(m) = parameter.modifiers()
            && self.is_parameter_property(m)
            && !self.is_in_constructor(tree, id)
        {
            self.error(AnalyzeError::InvalidParameterProperty {
                node: GlobalNodeIdAny::new(module.id, id.into_any()),
            });
        }
    }
}
