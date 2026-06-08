use crate::CompilerResult;
use crate::check::CheckState;

impl CheckState<'_> {
    /// Build tables that solve reads as stable input.
    pub(in crate::check) fn build(&mut self) -> CompilerResult<()> {
        self.build_definition_table()
    }
}
