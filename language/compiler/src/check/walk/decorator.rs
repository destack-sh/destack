use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Decision, WalkState};

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
    /// A decorator names a single declaration, either by a lexical name or
    /// through a namespace path. Decorators resolve eagerly during the walk
    /// and cannot defer to selection, so anything else is an error.
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
        let symbol = match self.tree.get(target) {
            dir::Expression::Identifier { name } => self.decorator_identifier_symbol(target, *name),
            dir::Expression::Member { .. } => self.decorator_path_symbol(target),
            // any other computed form is not a name and cannot resolve here
            _ => {
                self.check
                    .report_invalid_decorator_target(self.module, target.into_any());

                None
            }
        };

        if let Some(symbol) = symbol {
            self.check.commit_decision(
                target.into_global_any(self.module),
                Decision::Name(dir::NameResolution::new(symbol)),
            )?;
        }

        Ok(())
    }

    /// Resolve one lexical decorator name to its single declaration.
    fn decorator_identifier_symbol(
        &mut self,
        target: dir::LocalNodeId<dir::Expression>,
        name: dir::StringId,
    ) -> Option<dir::GlobalSymbolId> {
        let reference = self
            .check
            .module(self.module)
            .resolved
            .references
            .get(target.into_global_any(self.module))
            .cloned();

        match reference {
            Some(dir::Reference::Bound(symbols)) => {
                let symbols = self.check.present_symbols(&symbols);
                match symbols.as_slice() {
                    [symbol] => Some(*symbol),
                    // a namespace is not itself a decorator
                    _ => {
                        self.check
                            .report_invalid_decorator_target(self.module, target.into_any());

                        None
                    }
                }
            }
            Some(dir::Reference::Missing) => {
                let path = dir::Path {
                    segments: smallvec::smallvec![name],
                };
                self.check
                    .report_unresolved_reference(self.module, target.into_any(), &path);

                None
            }
            Some(dir::Reference::Ambiguous(_)) => {
                let path = dir::Path {
                    segments: smallvec::smallvec![name],
                };
                self.check
                    .report_ambiguous_reference(self.module, target.into_any(), &path);

                None
            }
            Some(dir::Reference::Namespace(_)) | Some(dir::Reference::Projected { .. }) | None => {
                self.check
                    .report_invalid_decorator_target(self.module, target.into_any());

                None
            }
        }
    }

    /// Resolve one namespace-path decorator to its single declaration.
    fn decorator_path_symbol(
        &mut self,
        target: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::GlobalSymbolId> {
        let reference = self
            .check
            .module(self.module)
            .resolved
            .references
            .get(target.into_global_any(self.module))
            .cloned();

        match reference {
            // a namespace path naming a single declaration
            Some(dir::Reference::Bound(symbols)) => {
                let symbols = self.check.present_symbols(&symbols);
                match symbols.as_slice() {
                    [symbol] => Some(*symbol),
                    // an overload set is not a single decorator
                    _ => {
                        self.check
                            .report_invalid_decorator_target(self.module, target.into_any());

                        None
                    }
                }
            }
            Some(dir::Reference::Ambiguous(_)) => {
                if let Some(path) = self.tree.tree().reference_path(target) {
                    self.check
                        .report_ambiguous_reference(self.module, target.into_any(), &path);
                }

                None
            }
            Some(dir::Reference::Missing) => {
                if let Some(path) = self.tree.tree().reference_path(target) {
                    self.check
                        .report_unresolved_reference(self.module, target.into_any(), &path);
                }

                None
            }
            // a namespace, a value projection, or an untracked target is not a decorator
            Some(dir::Reference::Namespace(_)) | Some(dir::Reference::Projected { .. }) | None => {
                self.check
                    .report_invalid_decorator_target(self.module, target.into_any());

                None
            }
        }
    }
}
