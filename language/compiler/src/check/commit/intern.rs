use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::CheckState;

use super::CheckModuleOutput;

impl CheckState<'_> {
    /// Intern one checked type into the output type segment.
    pub(super) fn intern_type(
        &self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        ty: dir::Type,
        source: dir::LocalNodeIdAny,
    ) -> dir::LocalTypeId {
        let types = self.module(module).type_table();

        types.intern_type(&mut output.types, ty, source)
    }

    /// Intern one checked static value into the output static segment.
    pub(super) fn intern_static(
        &self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        term: dir::StaticTerm,
    ) -> dir::LocalStaticId {
        let statics = self.module(module).static_table();

        statics.intern_static(&mut output.statics, term)
    }
}
