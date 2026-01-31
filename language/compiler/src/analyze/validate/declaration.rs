use crate::{AnalyzeError, Compiler};
use destack_dir::{
    Declaration, DeclarationAbstraction, DeclarationKind, DependencyMode, GlobalNodeIdAny,
    LocalNodeId, TypeTable,
};
use destack_workspace::{Module, ProfileId};

impl Compiler {
    /// Validate a declaration.
    pub(super) fn validate_declaration(
        &self,
        module: &Module,
        profile: ProfileId,
        types: &TypeTable,
        id: LocalNodeId<Declaration>,
        declaration: &Declaration,
    ) {
        // validate by declaration kind
        match declaration {
            Declaration::Interface {
                descriptor,
                heritage,
                ..
            } => {
                // resolve the interface node for diagnostics
                let node =
                    GlobalNodeIdAny::new(module.id, id.into_any()).into_anchored(Some(profile));

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
                let node =
                    GlobalNodeIdAny::new(module.id, id.into_any()).into_anchored(Some(profile));

                // declare functions cannot have a body
                let is_declare = descriptor.kind == DeclarationKind::Declaration;
                if is_declare && body.is_some() {
                    self.error(AnalyzeError::InvalidFunction { node });
                }
            }

            Declaration::Struct { heritage, .. } => {
                // resolve the struct node for diagnostics
                let node =
                    GlobalNodeIdAny::new(module.id, id.into_any()).into_anchored(Some(profile));

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

            Declaration::Class { heritage, .. } => {
                // resolve the class node for diagnostics
                let node =
                    GlobalNodeIdAny::new(module.id, id.into_any()).into_anchored(Some(profile));

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
}
