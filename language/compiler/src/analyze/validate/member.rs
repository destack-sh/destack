use crate::{AnalyzeError, Compiler};
use destack_dir::{
    Declaration, DeclarationAbstraction, FunctionAbstraction, FunctionMode, GlobalNodeIdAny,
    LocalNodeId, Member, NodeTree, NodeType,
};
use destack_workspace::Module;

impl Compiler {
    /// Validate a member.
    pub(super) fn validate_member(
        &self,
        module: &Module,
        tree: &NodeTree,
        id: LocalNodeId<Member>,
        member: &Member,
    ) {
        match member {
            // method validation
            Member::Method {
                signature, body, ..
            } => {
                // constructor cannot have static parameters
                if signature
                    .generics
                    .as_ref()
                    .is_some_and(|g| g.static_parameters.is_some())
                    && signature
                        .mode
                        .is_some_and(|mode| mode == FunctionMode::Constructor)
                {
                    self.error(AnalyzeError::InvalidConstructor {
                        node: GlobalNodeIdAny::new(module.id, id.into_any()),
                    });
                }

                // abstractness checks
                let is_abstract = matches!(
                    signature.abstraction,
                    FunctionAbstraction::Abstract | FunctionAbstraction::AbstractOverride
                );

                // abstract methods cannot have a body
                if is_abstract && body.is_some() {
                    self.error(AnalyzeError::InvalidMethod {
                        node: GlobalNodeIdAny::new(module.id, id.into_any()),
                        abstraction: signature.abstraction,
                    });
                }

                // abstract methods can only appear in abstract classes
                if is_abstract && !self.is_in_abstract_class(tree, id) {
                    self.error(AnalyzeError::InvalidMethod {
                        node: GlobalNodeIdAny::new(module.id, id.into_any()),
                        abstraction: signature.abstraction,
                    });
                }
            }

            // static block validation
            Member::StaticBlock { modifiers, .. } => {
                if let Some(m) = modifiers {
                    let has_invalid_modifier = m.visibility.is_some()
                        || m.mutability.is_some()
                        || m.kind.is_some()
                        || m.operator.is_some()
                        || m.accessor.is_some();
                    if has_invalid_modifier {
                        self.error(AnalyzeError::InvalidStaticBlockModifier {
                            node: GlobalNodeIdAny::new(module.id, id.into_any()),
                        });
                    }
                }
            }

            _ => {}
        }
    }

    /// Check if a member is inside an abstract class.
    fn is_in_abstract_class(&self, tree: &NodeTree, member_id: LocalNodeId<Member>) -> bool {
        // walk up to find the parent declaration
        let Some(parent) = tree.get_parent(member_id.id) else {
            return false;
        };

        if parent.ty == NodeType::Declaration {
            let declaration = tree.get(LocalNodeId::<Declaration>::new(parent.id));
            if let Declaration::Class { descriptor, .. } = declaration {
                return descriptor.abstraction == DeclarationAbstraction::Abstract;
            }
        }

        false
    }
}
