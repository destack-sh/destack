use destack_dir::Expression;
use destack_workspace::Module;

use crate::Compiler;

impl Compiler {
    /// Desugar module syntactically: transforms that don't need type information:
    /// - `AssignBinary` → `Assign` + `Binary` (`x += 1` → `x = x + 1`)
    /// - `AwaitMaybe` → `Maybe` + `Await` (`await? x` → `(await x)?`)
    pub(super) fn bind_module_desugar(&self, module: &Module) {
        let mut tree = module.dir_base().tree.write();

        // desugar expressions
        for expression_id in tree.iter_node_ids_of_type::<Expression>() {
            self.desugar_expression(expression_id, &mut tree);
        }
    }
}
