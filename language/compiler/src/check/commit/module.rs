use std::sync::Arc;

use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::CheckState;

use super::CheckModuleOutput;

impl CheckState<'_> {
    /// Commit one module's solved checker state into output DIR tables.
    pub(in crate::check) fn commit_module(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<CheckModuleOutput> {
        let environment = Arc::clone(&self.environment);
        let mut output = CheckModuleOutput::new(module, self.module(module));

        output.annotations = self.commit_annotation_table(module, &mut output, &environment)?;
        output.generics = self.commit_generic_parameter_table(module, &mut output, &environment);
        output.resolutions = self.commit_resolution_table(module, &mut output, &environment)?;
        self.commit_type_table(module, &mut output, &environment)?;
        self.commit_static_table(module, &mut output, &environment)?;
        output.nominals = self.commit_nominal_table(module, &mut output, &environment);
        output.extensions = self.commit_extension_table(module, &mut output, &environment)?;
        output.layouts = self.commit_layout_table(module, &mut output, &environment)?;
        output.captures = self.commit_capture_table(module, &mut output, &environment)?;

        Ok(output)
    }
}
