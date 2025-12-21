use crate::{AnalyzeError, Compiler};
use destack_dir::{
    Declaration, DeclarationAbstraction, DeclarationKind, GlobalNodeIdAny, LocalNodeId, TypeTable,
};
use destack_workspace::Module;

impl Compiler {
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
