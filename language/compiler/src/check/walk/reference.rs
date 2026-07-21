use destack_dir as dir;
use smallvec::smallvec;

use crate::check::{Decision, WalkState};
use crate::{CompilerError, CompilerResult};

impl WalkState<'_, '_> {
    /// Walk one identifier expression.
    ///
    /// Example:
    /// ```ds
    /// value
    /// ```
    pub(in crate::check) fn walk_identifier_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        name: dir::StringId,
    ) -> CompilerResult<()> {
        let source = id.into_global_any(self.module);
        let reference = self
            .check
            .module(self.module)
            .resolved
            .references
            .get(source)
            .cloned();
        match reference {
            // a bound name gives one declaration or a callable overload set
            Some(dir::Reference::Bound(symbols)) => {
                let symbols = self.check.present_symbols(&symbols);
                match symbols.as_slice() {
                    [symbol] => {
                        let symbol = *symbol;
                        self.capture_symbol_reference(symbol);
                        self.check.commit_decision(
                            source,
                            Decision::Name(dir::NameResolution::new(symbol)),
                        )?;
                        self.check_assigned_read(id.into_any(), symbol);
                    }

                    // overload sets resolve at their call sites
                    _ => {
                        for symbol in symbols.iter().copied() {
                            self.capture_symbol_reference(symbol);
                        }
                        self.check.commit_decision(
                            source,
                            Decision::Name(dir::NameResolution::from_symbols(symbols.to_vec())),
                        )?;
                    }
                }
            }

            // conflicting lexical names fail loudly
            Some(dir::Reference::Ambiguous(_)) => {
                let path = dir::Path {
                    segments: smallvec![name],
                };
                self.check
                    .report_ambiguous_reference(self.module, id.into_any(), &path);
                let error = self.intern_type(dir::Type::Error)?;
                self.commit_node_type(id, error)?;
            }

            // missing names fail loudly
            Some(dir::Reference::Missing) => {
                let path = dir::Path {
                    segments: smallvec![name],
                };
                self.check
                    .report_unresolved_reference(self.module, id.into_any(), &path);
                let error = self.intern_type(dir::Type::Error)?;
                self.commit_node_type(id, error)?;
            }

            // reject namespaces used directly as values
            Some(dir::Reference::Namespace(_)) => {
                let error = self.intern_type(dir::Type::Error)?;
                self.commit_node_type(id, error)?;
            }

            // require resolve to write the bare name reference
            Some(dir::Reference::Projected { .. }) | None => {
                return Err(CompilerError::Internal {
                    message: format!("identifier reference {source:?} has no resolved name"),
                });
            }
        }

        Ok(())
    }

    /// Walk one member access, deciding name paths and queuing value members.
    pub(in crate::check) fn walk_member_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        let source = id.into_global_any(self.module);
        let reference = self
            .check
            .module(self.module)
            .resolved
            .references
            .get(source)
            .cloned();

        match reference {
            // a name path resolves to its declaration like an identifier
            Some(dir::Reference::Bound(symbols)) => {
                let symbols = self.check.present_symbols(&symbols);
                for symbol in symbols.iter().copied() {
                    self.capture_symbol_reference(symbol);
                }
                match symbols.as_slice() {
                    [symbol] => {
                        let symbol = *symbol;
                        self.check.commit_decision(
                            source,
                            Decision::Name(dir::NameResolution::new(symbol)),
                        )?;
                        self.check_assigned_read(id.into_any(), symbol);
                    }
                    // overload sets resolve at their call sites
                    _ => {
                        self.check.commit_decision(
                            source,
                            Decision::Name(dir::NameResolution::from_symbols(symbols.to_vec())),
                        )?;
                    }
                }
            }

            // conflicting name targets fail loudly
            Some(dir::Reference::Ambiguous(_)) => {
                if let Some(path) = self.tree.tree().reference_path(id) {
                    self.check
                        .report_ambiguous_reference(self.module, id.into_any(), &path);
                }
                let error = self.intern_type(dir::Type::Error)?;
                self.commit_node_type(id, error)?;
            }

            // unresolved name paths fail loudly
            Some(dir::Reference::Missing) => {
                if let Some(path) = self.tree.tree().reference_path(id) {
                    self.check
                        .report_unresolved_reference(self.module, id.into_any(), &path);
                }
                let error = self.intern_type(dir::Type::Error)?;
                self.commit_node_type(id, error)?;
            }

            // reject namespaces used directly as values
            Some(dir::Reference::Namespace(_)) => {
                let error = self.intern_type(dir::Type::Error)?;
                self.commit_node_type(id, error)?;
            }

            // select value member access
            Some(dir::Reference::Projected { .. }) | None => {
                self.walk_expression(left, self.tree.get(left))?;
            }
        }

        Ok(())
    }

    /// Walk one explicit instantiation expression.
    ///
    /// Example:
    /// ```ds
    /// value<T>
    /// ```
    pub(in crate::check) fn walk_instantiation_expression(
        &mut self,
        _id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<()> {
        self.walk_expression(left, self.tree.get(left))?;

        // collect written argument types for instantiation selection
        self.walk_generic_arguments(generic_arguments)?;

        Ok(())
    }
}
