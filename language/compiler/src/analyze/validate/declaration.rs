use crate::{AnalyzeError, Compiler};
use destack_dir::{
    BindingOperator, Declaration, DeclarationAbstraction, DeclarationKind, DependencyKind,
    DependencyMode, LocalNodeId, LocalNodeIdAny, Name, NodeTree, NodeType, TypeTable,
};
use destack_workspace::{Module, ProfileId};

/// The kind of declare namespace context for a node.
#[derive(Clone, Copy, PartialEq, Eq)]
enum DeclareNamespaceContext {
    /// The node is not inside a declare namespace.
    None,
    /// The node is inside a declared namespace with an identifier name.
    Namespace,
    /// The node is inside a declared module with a string name.
    Module,
}

impl Compiler {
    /// Validate a declaration.
    pub(super) fn validate_declaration(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        types: &TypeTable,
        id: LocalNodeId<Declaration>,
        declaration: &Declaration,
    ) {
        // reject typescript-only declarations in javascript modules
        if module.language_type.is_javascript()
            && matches!(
                declaration,
                Declaration::Interface { .. } | Declaration::Type { .. }
            )
        {
            let node = id.into_global_any(module.id).into_anchored(Some(profile));
            self.error(AnalyzeError::TypeScriptSyntaxInJavaScript { node });
        }

        // validate by declaration kind
        match declaration {
            Declaration::Interface {
                descriptor,
                heritage,
                ..
            } => {
                // resolve the interface node for diagnostics
                let node = id.into_global_any(module.id).into_anchored(Some(profile));

                // interfaces cannot be abstract
                if descriptor.abstraction == DeclarationAbstraction::Abstract {
                    self.error(AnalyzeError::InvalidInterface { node });
                }

                // default export interfaces must be named
                if descriptor.export == Some(DependencyMode::Default) && descriptor.name.is_none() {
                    self.error(AnalyzeError::InvalidInterface { node });
                }

                // reject empty extends clauses
                let has_empty_extends = heritage
                    .extends_types
                    .as_ref()
                    .is_some_and(|e| e.is_empty());
                if has_empty_extends {
                    self.error(AnalyzeError::InvalidLineage {
                        node,
                        extends_symbols: Vec::new(),
                        implements_symbols: Vec::new(),
                        embedded_symbols: Vec::new(),
                    });
                }
            }

            Declaration::Function {
                descriptor, body, ..
            } => {
                // resolve the function node for diagnostics
                let node = id.into_global_any(module.id).into_anchored(Some(profile));

                // declare functions cannot have a body
                let is_declare = descriptor.kind == DeclarationKind::Declaration
                    || self.is_in_declare_namespace(tree, id.into_any());
                if is_declare && body.is_some() {
                    self.error(AnalyzeError::InvalidFunction { node });
                }
            }

            Declaration::Type {
                static_parameters, ..
            } => {
                // reject invalid type parameter modifiers in type aliases
                if let Some(static_parameters) = static_parameters {
                    for parameter_id in static_parameters {
                        let parameter = tree.get(*parameter_id);
                        let modifiers = parameter.modifiers();
                        let has_const_modifier = modifiers.is_some_and(|modifiers| {
                            modifiers.operator == Some(BindingOperator::AsConst)
                        });
                        if has_const_modifier {
                            let node = parameter_id
                                .into_global_any(module.id)
                                .into_anchored(Some(profile));
                            self.error(AnalyzeError::InvalidTypeParameterModifier { node });
                        }
                    }
                }
            }

            Declaration::Struct { heritage, .. } => {
                // resolve the struct node for diagnostics
                let node = id.into_global_any(module.id).into_anchored(Some(profile));

                // structs cannot use extends
                let has_extends = heritage
                    .extends_types
                    .as_ref()
                    .is_some_and(|e| !e.is_empty());
                if has_extends {
                    let extends_symbols = types
                        .get_lineage_for_symbol(declaration.symbol().into_global(module.id))
                        .and_then(|lineage| lineage.extends)
                        .into_iter()
                        .collect();
                    self.error(AnalyzeError::InvalidLineage {
                        node,
                        extends_symbols,
                        implements_symbols: Vec::new(),
                        embedded_symbols: Vec::new(),
                    });
                }
            }

            Declaration::ImportAlias { kind, .. } => {
                // import aliases cannot use import type
                if *kind == DependencyKind::Type {
                    let node = id.into_global_any(module.id).into_anchored(Some(profile));
                    self.error(AnalyzeError::InvalidTypeOnlyImportAlias { node });
                }
            }

            Declaration::Class { heritage, .. } => {
                // resolve the class node for diagnostics
                let node = id.into_global_any(module.id).into_anchored(Some(profile));

                // classes can only extend one class
                let has_multiple_extends =
                    heritage.extends_types.as_ref().is_some_and(|e| e.len() > 1);
                if has_multiple_extends {
                    let lineage =
                        types.get_lineage_for_symbol(declaration.symbol().into_global(module.id));
                    let extends_symbols = lineage.and_then(|l| l.extends).into_iter().collect();
                    let implements_symbols =
                        lineage.map(|l| l.implements.clone()).unwrap_or_default();
                    let embedded_symbols = lineage.map(|l| l.embedded.clone()).unwrap_or_default();
                    self.error(AnalyzeError::InvalidLineage {
                        node,
                        extends_symbols,
                        implements_symbols,
                        embedded_symbols,
                    });
                }

                // reject empty extends or implements clauses
                let has_empty_extends = heritage
                    .extends_types
                    .as_ref()
                    .is_some_and(|e| e.is_empty());
                let has_empty_implements = heritage
                    .implements_types
                    .as_ref()
                    .is_some_and(|i| i.is_empty());
                if has_empty_extends || has_empty_implements {
                    self.error(AnalyzeError::InvalidLineage {
                        node,
                        extends_symbols: Vec::new(),
                        implements_symbols: Vec::new(),
                        embedded_symbols: Vec::new(),
                    });
                }
            }

            _ => {}
        }
    }

    /// Check whether a node is nested inside a declare namespace.
    pub(super) fn is_in_declare_namespace(&self, tree: &NodeTree, node_id: LocalNodeIdAny) -> bool {
        matches!(
            self.declare_namespace_context(tree, node_id),
            DeclareNamespaceContext::Namespace | DeclareNamespaceContext::Module
        )
    }

    /// Check whether a node is nested inside a declared module with a string name.
    pub(super) fn is_in_declare_module(&self, tree: &NodeTree, node_id: LocalNodeIdAny) -> bool {
        matches!(
            self.declare_namespace_context(tree, node_id),
            DeclareNamespaceContext::Module
        )
    }

    /// Classify the nearest declare namespace context for a node.
    /// NOTE #Performance: store "declaredness" on Scope/Symbol directly (from import/bind)?
    fn declare_namespace_context(
        &self,
        tree: &NodeTree,
        node_id: LocalNodeIdAny,
    ) -> DeclareNamespaceContext {
        // walk up the parent chain
        let mut saw_namespace = false;
        let mut current = tree.get_parent(node_id.id);
        while let Some(parent) = current {
            if parent.ty == NodeType::Declaration {
                let declaration = tree.get(parent.into_typed::<Declaration>());
                if let Declaration::Namespace { descriptor, .. } = declaration
                    && descriptor.kind == DeclarationKind::Declaration
                {
                    if matches!(descriptor.name, Some(Name::String(_))) {
                        return DeclareNamespaceContext::Module;
                    }
                    saw_namespace = true;
                }
            }
            current = tree.get_parent(parent.id);
        }
        if saw_namespace {
            DeclareNamespaceContext::Namespace
        } else {
            DeclareNamespaceContext::None
        }
    }
}
