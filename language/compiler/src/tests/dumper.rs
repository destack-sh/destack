use dyst_dir::{Dumper, NodeVisitor};

use crate::TestProgram;

impl TestProgram {
    /// Dump the program.
    pub fn dump(&self) {
        self.dump_nodes();
        self.dump_symbols();
    }

    /// Dump the node representation of the program.
    pub fn dump_nodes(&self) {
        let strings = self.program.strings.clone().into_immutable();
        for module in self.program.modules.iter() {
            let module = module.read();
            let tree = module.tree.read();
            let mut dumper = Dumper::new(&strings, &tree, self.dumper_options);
            println!("{}", "=".repeat(80));
            println!("{} [NODE]", module.uri);
            println!("{}", "=".repeat(80));
            for expression_id in &module.roots {
                let expression = tree.get(*expression_id);
                dumper.visit_expression(&tree, *expression_id, expression);
            }
            println!("{}", dumper.finish());
        }
    }

    /// Dump the symbol representation of the program.
    pub fn dump_symbols(&self) {
        let strings = self.program.strings.clone().into_immutable();
        for module in self.program.modules.iter() {
            let module = module.read();
            let tree = module.tree.read();
            let symbols = module.symbols.read();
            let mut dumper = Dumper::new(&strings, &tree, self.dumper_options);
            println!("{}", "=".repeat(80));
            println!("{} [SYMBOL]", module.uri);
            println!("{}", "=".repeat(80));
            let scope = symbols.get_scope_by_id(module.namespace_scope);
            dumper.visit_scope(&tree, &symbols, module.namespace_scope, scope);
            println!("{}", dumper.finish());
        }
    }
}
