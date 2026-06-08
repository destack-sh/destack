use crate::CompilerResult;
use crate::check::WalkState;
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
        _decorator: &dir::Decorator,
    ) -> CompilerResult<()> {
        let invocation = self.check.decorator_invocation(self.module, id);

        // bind ordinary decorator target names
        if self
            .check
            .static_if_decorator_from_invocation(self.module, &invocation)
            .is_none()
        {
            self.bind_decorator_target_name(invocation.target)?;
        }

        // walk annotation arguments as static metadata
        for argument in invocation.arguments {
            let Some(value) = self.tree.get(argument).value() else {
                continue;
            };

            self.walk_static_expression(value)?;
        }

        Ok(())
    }

    /// Bind one ordinary decorator target name.
    ///
    /// Example:
    /// ```ds
    /// @repr("C")
    /// struct Header {}
    /// ```
    fn bind_decorator_target_name(
        &mut self,
        target: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        let source = target.into_global_any(self.module);
        let guard = self.active_static_guard();

        match self.tree.get(target) {
            // bind bare decorator target
            dir::Expression::Identifier { name } => {
                if let Some(symbol) = self.check.symbol_by_name_under(
                    self.module,
                    target.into_any(),
                    *name,
                    dir::SymbolSpace::Value,
                    &guard,
                ) {
                    self.check
                        .inference
                        .select_name(source, dir::NameResolution::new(symbol))?;
                }
            }

            // bind qualified decorator target
            dir::Expression::QualifiedReference { path, .. } => {
                if let Some(symbol) = self.check.symbol_by_path_under(
                    self.module,
                    target.into_any(),
                    path,
                    dir::SymbolSpace::Value,
                    &guard,
                ) {
                    self.check
                        .inference
                        .select_name(source, dir::NameResolution::new(symbol))?;
                }
            }

            // leave non-reference decorator targets unresolved
            _ => {}
        }

        Ok(())
    }
}
