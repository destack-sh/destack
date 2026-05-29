use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{CheckState, TypeTerm};

impl CheckState<'_> {
    /// Define one node output type.
    pub(in crate::check) fn define_node_type<T: dir::Node + Clone>(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<T>,
        term: TypeTerm,
    ) {
        let variable = self.intern_local_node_type_variable(module, id);
        let condition = self.active_static_condition(module);

        self.add_type_definition(variable, term, condition);
    }
}
