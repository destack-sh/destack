use destack_dir::{
    Declaration, LocalNodeId, NodeVisitor, NodeVisitorOptions, SymbolOrigin, SymbolTable, Tree,
    walk_declaration,
};
use destack_workspace::Module;

use crate::Compiler;

struct GlobalAugmentationVisitor<'a> {
    symbols: &'a mut SymbolTable,
    in_global: bool,
    options: NodeVisitorOptions,
}

impl<'a> GlobalAugmentationVisitor<'a> {
    fn new(symbols: &'a mut SymbolTable) -> Self {
        Self {
            symbols,
            in_global: false,
            options: NodeVisitorOptions::default(),
        }
    }
}

impl NodeVisitor for GlobalAugmentationVisitor<'_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_declaration(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Declaration>,
        declaration: &Declaration,
    ) {
        // visit nested declarations inside global blocks
        if matches!(declaration, Declaration::Global { .. }) {
            let previous = self.in_global;
            self.in_global = true;

            walk_declaration(self, tree, id, declaration);

            self.in_global = previous;
            return;
        }

        // mark declarations that live inside global augmentations
        if self.in_global {
            let symbol_id = declaration.symbol();
            let symbol = self.symbols.get_symbol_mut(symbol_id);
            symbol.origin = SymbolOrigin::GlobalAugmentation;
        }

        walk_declaration(self, tree, id, declaration);
    }
}

impl Compiler {
    /// Mark symbols declared within global augmentation blocks.
    pub(super) fn mark_global_augmentation_symbols(
        &self,
        _module: &Module,
        tree: &Tree,
        symbols: &mut SymbolTable,
    ) {
        // visit each global declaration to mark nested symbols
        let mut visitor = GlobalAugmentationVisitor::new(symbols);
        for (declaration_id, declaration) in tree.iter_nodes_of_type::<Declaration>() {
            if matches!(declaration, Declaration::Global { .. }) {
                visitor.visit_declaration(tree, declaration_id, declaration);
            }
        }
    }
}
