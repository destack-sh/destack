use crate::{AnalyzeError, Compiler};
use destack_dir::{
    BindingModifier, Declaration, DeclarationAbstraction, DeclarationKind, FunctionAbstraction,
    FunctionMode, GlobalNodeIdAny, LocalNodeId, Member, Mutability, NodeTree, NodeType, Parameter,
    Property, TypeTable,
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

    /// Validate a declaration.
    pub(super) fn validate_declaration(
        &self,
        module: &Module,
        types: &TypeTable,
        id: LocalNodeId<Declaration>,
        declaration: &Declaration,
    ) {
        match declaration {
            Declaration::Interface { descriptor, .. } => {
                // interfaces cannot be abstract
                if descriptor.abstraction == DeclarationAbstraction::Abstract {
                    self.error(AnalyzeError::InvalidInterface {
                        node: GlobalNodeIdAny::new(module.id, id.into_any()),
                    });
                }
            }

            Declaration::Function {
                descriptor, body, ..
            } => {
                // declare functions cannot have a body
                if descriptor.kind == DeclarationKind::Declaration && body.is_some() {
                    self.error(AnalyzeError::InvalidFunction {
                        node: GlobalNodeIdAny::new(module.id, id.into_any()),
                    });
                }
            }

            Declaration::Struct { heritage, .. } => {
                // structs cannot use extends
                if heritage
                    .extends_types
                    .as_ref()
                    .is_some_and(|e| !e.is_empty())
                {
                    let extends_symbols = types
                        .get_lineage_for_symbol(declaration.symbol().into_global(module.id))
                        .and_then(|lineage| lineage.extends)
                        .into_iter()
                        .collect();
                    self.error(AnalyzeError::InvalidLineage {
                        node: GlobalNodeIdAny::new(module.id, id.into_any()),
                        extends_symbols,
                        implements_symbols: Vec::new(),
                        embedded_symbols: Vec::new(),
                    });
                }
            }

            Declaration::Class { heritage, .. } => {
                // classes can only extend one class
                if heritage.extends_types.as_ref().is_some_and(|e| e.len() > 1) {
                    let lineage =
                        types.get_lineage_for_symbol(declaration.symbol().into_global(module.id));
                    let extends_symbols = lineage.and_then(|l| l.extends).into_iter().collect();
                    let implements_symbols =
                        lineage.map(|l| l.implements.clone()).unwrap_or_default();
                    let embedded_symbols = lineage.map(|l| l.embedded.clone()).unwrap_or_default();
                    self.error(AnalyzeError::InvalidLineage {
                        node: GlobalNodeIdAny::new(module.id, id.into_any()),
                        extends_symbols,
                        implements_symbols,
                        embedded_symbols,
                    });
                }
            }

            _ => {}
        }
    }
}
