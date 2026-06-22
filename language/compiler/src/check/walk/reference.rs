use destack_dir as dir;

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
                let symbols = self.check.available_symbols(&symbols);
                match symbols.as_slice() {
                    [symbol] => {
                        let symbol = *symbol;
                        self.capture_symbol_reference(symbol);
                        self.check.record_decision(
                            source,
                            Decision::Name(dir::NameResolution::new(symbol)),
                        )?;
                        let ty = self.symbol_type(symbol)?;
                        // read the active flow narrowing when one exists
                        let ty = self.flow_path_narrowing(id).unwrap_or(ty);
                        self.declare_node_type(id, ty)?;
                    }

                    // overload sets resolve at their call sites
                    _ => {
                        for symbol in symbols.iter().copied() {
                            self.capture_symbol_reference(symbol);
                        }
                        self.check.record_decision(
                            source,
                            Decision::Name(dir::NameResolution::from_symbols(symbols.to_vec())),
                        )?;

                        // open the node type for call selection
                        self.node_type(id)?;
                    }
                }
            }

            // conflicting lexical names fail loudly
            Some(dir::Reference::Ambiguous(_)) => {
                let path = dir::Path {
                    segments: smallvec::smallvec![name],
                };
                self.check
                    .report_ambiguous_reference(self.module, id.into_any(), &path);
                let error = self.push_type(dir::Type::Error, id.into_any())?;
                self.declare_node_type(id, error)?;
            }

            // missing names fail loudly
            Some(dir::Reference::Missing) => {
                let path = dir::Path {
                    segments: smallvec::smallvec![name],
                };
                self.check
                    .report_unresolved_reference(self.module, id.into_any(), &path);
                let error = self.push_type(dir::Type::Error, id.into_any())?;
                self.declare_node_type(id, error)?;
            }

            // reject namespaces used directly as values
            Some(dir::Reference::Namespace(_)) => {
                let error = self.push_type(dir::Type::Error, id.into_any())?;
                self.declare_node_type(id, error)?;
            }

            // identifiers must have a bare name reference row
            Some(dir::Reference::Projected { .. }) | None => {
                return Err(CompilerError::Internal {
                    message: format!("identifier reference {source:?} has no resolved name"),
                });
            }
        }

        Ok(())
    }

    /// Walk one member access, deciding static name paths and queuing value members.
    ///
    /// A member chain that names a declaration decides immediately.
    /// A value member projection queues selection after the receiver has been walked.
    pub(in crate::check) fn walk_member_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // resolve the receiver chain first
        self.walk_expression(left, self.tree.get(left))?;

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
                let symbols = self.check.available_symbols(&symbols);
                for symbol in symbols.iter().copied() {
                    self.capture_symbol_reference(symbol);
                }
                match symbols.as_slice() {
                    [symbol] => {
                        let symbol = *symbol;
                        self.check.record_decision(
                            source,
                            Decision::Name(dir::NameResolution::new(symbol)),
                        )?;
                        let ty = self.symbol_type(symbol)?;
                        self.declare_node_type(id, ty)?;
                    }
                    // overload sets resolve at their call sites
                    _ => {
                        self.check.record_decision(
                            source,
                            Decision::Name(dir::NameResolution::from_symbols(symbols.to_vec())),
                        )?;
                        self.node_type(id)?;
                    }
                }
            }

            // conflicting name targets fail loudly
            Some(dir::Reference::Ambiguous(_)) => {
                if let Some(path) = self.tree.tree().reference_path(id) {
                    self.check
                        .report_ambiguous_reference(self.module, id.into_any(), &path);
                }
                let error = self.push_type(dir::Type::Error, id.into_any())?;
                self.declare_node_type(id, error)?;
            }

            // unresolved name paths fail loudly
            Some(dir::Reference::Missing) => {
                if let Some(path) = self.tree.tree().reference_path(id) {
                    self.check
                        .report_unresolved_reference(self.module, id.into_any(), &path);
                }
                let error = self.push_type(dir::Type::Error, id.into_any())?;
                self.declare_node_type(id, error)?;
            }

            // reject namespaces used directly as values
            Some(dir::Reference::Namespace(_)) => {
                let error = self.push_type(dir::Type::Error, id.into_any())?;
                self.declare_node_type(id, error)?;
            }

            // queue selection for value member access
            Some(dir::Reference::Projected { .. }) | None => {
                self.node_type(id)?;
                self.queue_select(id.into_global_any(self.module));
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
        id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<()> {
        self.walk_expression(left, self.tree.get(left))?;

        // walk written argument types before queuing instantiation
        for argument in generic_arguments {
            self.walk_generic_arguments(std::slice::from_ref(argument))?;
        }
        self.node_type(id)?;
        self.queue_select(id.into_global_any(self.module));

        Ok(())
    }
}
