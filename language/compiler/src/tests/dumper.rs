use std::sync::Arc;

use destack_dir::{Dumper, NodeVisitor};
use destack_parser::colorize_source;
use destack_workspace::Module;
use parking_lot::RwLock;

use crate::TestProgram;

impl TestProgram {
    /// Dump all representations (source, nodes, symbols) for all modules.
    pub fn dump(&self) {
        for module in self.program.modules.iter() {
            self.dump_module_file(&module);
            self.dump_module_nodes(&module);
            self.dump_module_symbols(&module);
        }
    }

    /// Dump file representations for all modules.
    pub fn dump_files(&self) {
        for module in self.program.modules.iter() {
            self.dump_module_file(&module);
        }
    }

    /// Dump node representations for all modules.
    pub fn dump_nodes(&self) {
        for module in self.program.modules.iter() {
            self.dump_module_nodes(&module);
        }
    }

    /// Dump symbol representations for all modules.
    pub fn dump_symbols(&self) {
        for module in self.program.modules.iter() {
            self.dump_module_symbols(&module);
        }
    }

    /// Dump the file representation of a single module.
    pub fn dump_module_file(&self, module: &Arc<RwLock<Module>>) {
        let module = module.read();
        let file = self.program.files.get(module.file_id);
        println!("{}", "=".repeat(80));
        println!("{} [FILE]", module.uri);
        println!("{}", "=".repeat(80));
        println!("{}", colorize_source(&file));
    }

    /// Dump the node representation of a single module.
    pub fn dump_module_nodes(&self, module: &Arc<RwLock<Module>>) {
        let strings = (*self.program.strings).clone().into_immutable();
        let module = module.read();
        let tree = module.dir.tree.read();
        let mut dumper = Dumper::new(&strings, &tree, self.dumper_options);
        println!("{}", "=".repeat(80));
        println!("{} [NODE]", module.uri);
        println!("{}", "=".repeat(80));
        for expression_id in &module.dir.roots {
            let expression = tree.get(*expression_id);
            dumper.visit_expression(&tree, *expression_id, expression);
        }
        println!("{}", dumper.finish());
    }

    /// Dump the symbol representation of a single module.
    pub fn dump_module_symbols(&self, module: &Arc<RwLock<Module>>) {
        let strings = (*self.program.strings).clone().into_immutable();
        let module = module.read();
        let tree = module.dir.tree.read();
        let symbols = module.dir.symbols.read();
        let mut dumper = Dumper::new(&strings, &tree, self.dumper_options);
        println!("{}", "=".repeat(80));
        println!("{} [SYMBOL]", module.uri);
        println!("{}", "=".repeat(80));
        let scope = symbols.get_scope_by_id(module.dir.namespace_scope);
        dumper.visit_scope(&tree, &symbols, module.dir.namespace_scope, scope);
        println!("{}", dumper.finish());
    }
}
