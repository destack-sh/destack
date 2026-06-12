use crate::CompilerResult;
use crate::check::{Decision, NameLookup, WalkState};
use destack_dir as dir;

impl WalkState<'_, '_> {
    /// Walk one annotation invocation.
    ///
    /// Example:
    /// ```ds
    /// @inline
    /// function f() {}
    /// ```
    pub(in crate::check) fn walk_decorator(
        &mut self,
        id: dir::LocalNodeId<dir::Decorator>,
    ) -> CompilerResult<()> {
        let invocation = self.check.decorator_invocation(self.module, id);

        // walk non-if decorator target names
        if self
            .check
            .static_if_decorator_from_invocation(self.module, &invocation)
            .is_none()
        {
            self.walk_decorator_target_name(invocation.target)?;
        }

        // annotation arguments are read by their decorator consumers
        Ok(())
    }

    /// Walk one non-if decorator target name.
    ///
    /// Example:
    /// ```ds
    /// @repr("C")
    /// struct Header {}
    /// ```
    fn walk_decorator_target_name(
        &mut self,
        target: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        let source = target.into_global_any(self.module);

        // select bare decorator targets
        let name = match self.tree.get(target) {
            dir::Expression::Identifier { name } => *name,
            // leave non-reference decorator targets unresolved
            _ => return Ok(()),
        };
        let lookup = self.check.lookup_name(
            self.module,
            target.into_any(),
            name,
            dir::SymbolSpace::Value,
        );
        if let NameLookup::Found(candidate) = lookup {
            if let Some(symbol) = candidate.symbol() {
                let resolution = dir::NameResolution::new(symbol);

                self.check
                    .record_decision(source, Decision::Name(resolution))?;
            }
        }

        Ok(())
    }
}
