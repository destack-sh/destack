use super::printer::Printer;
use crate::{Declaration, DeclarationBlock, LocalNodeId};

impl<'a> Printer<'a> {
    /// Print one declaration block node.
    pub(crate) fn print_declaration_block_id(
        &mut self,
        declaration_block_id: LocalNodeId<DeclarationBlock>,
    ) {
        let declaration_block = self.tree.get(declaration_block_id);

        for (index, declaration_id) in declaration_block.declarations.iter().enumerate() {
            if index > 0 {
                self.source.push(';');
            }

            self.print_declaration_id(*declaration_id);
        }
    }

    /// Print one declaration node.
    pub(crate) fn print_declaration_id(&mut self, declaration_id: LocalNodeId<Declaration>) {
        let declaration = self.tree.get(declaration_id);

        self.source
            .push_str(Self::render_property_name(&declaration.name));
        self.source.push(':');
        self.source
            .push_str(&self.render_component_value_list(declaration.value.components()));

        if declaration.is_important {
            self.source.push_str("!important");
        }
    }
}
