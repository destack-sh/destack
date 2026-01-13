use destack_dir::{Argument, Expression, LocalNodeId, NodeTree, ScalarLiteral};
use destack_workspace::Loader;

use crate::Compiler;

impl Compiler {
    /// Extract loader override from import attributes.
    ///
    /// Supports the `type` attribute to override the default loader:
    /// ```text
    /// import data from "./file" with { type: "json" }
    /// import raw from "./file" with { type: "text" }
    /// ```
    pub(crate) fn loader_from_import_attributes(
        &self,
        arguments: Option<&Vec<LocalNodeId<Argument>>>,
        tree: &NodeTree,
    ) -> Option<Loader> {
        let arguments = arguments?;
        let type_key = self.program.strings.intern("type");

        for arg_id in arguments {
            let Argument::Named { name, value } = tree.get(*arg_id) else {
                continue;
            };
            if *name != type_key {
                continue;
            }

            // get string value from expression
            let Expression::ScalarLiteral {
                value: ScalarLiteral::String(s),
            } = tree.get(*value)
            else {
                continue;
            };
            let value_str = self.program.strings.get(*s);

            return Loader::from_type_attribute(value_str.as_ref());
        }
        None
    }
}
