use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Decision, NameLookup, NameTarget, WalkState};

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
        let lookup =
            self.check
                .lookup_name(self.module, id.into_any(), name, dir::SymbolSpace::Value);

        match lookup {
            NameLookup::Found(candidate) => match candidate.target {
                NameTarget::Symbol(symbol) => {
                    self.capture_symbol_reference(symbol);
                    self.check.record_decision(
                        source,
                        Decision::Name(dir::NameResolution::new(symbol)),
                    )?;
                    let ty = self.reference_symbol_type(symbol)?;
                    // active flow narrowing refines the reference read
                    let ty = self.flow_path_narrowing(id).unwrap_or(ty);
                    self.declare_node_type(id, ty)?;
                }
                // namespaces only carry further member selection
                NameTarget::Namespace(_) => {
                    let error = self.push_type(dir::Type::Error, id.into_any())?;
                    self.declare_node_type(id, error)?;
                }
            },
            NameLookup::Missing => {
                let path = dir::Path {
                    segments: smallvec::smallvec![name],
                };
                self.check
                    .report_unresolved_reference(self.module, id.into_any(), &path);
                let error = self.push_type(dir::Type::Error, id.into_any())?;
                self.declare_node_type(id, error)?;
            }
            // overload sets resolve at their call sites
            NameLookup::Ambiguous(candidates) => {
                let symbols = candidates
                    .iter()
                    .filter_map(|candidate| candidate.symbol())
                    .collect::<Vec<_>>();
                for symbol in &symbols {
                    self.capture_symbol_reference(*symbol);
                }
                self.check.record_decision(
                    source,
                    Decision::Name(dir::NameResolution::from_symbols(symbols)),
                )?;

                // the node value stays open until the call selects
                self.node_type(id)?;
            }
        }

        Ok(())
    }

    /// Walk one qualified reference expression.
    ///
    /// Example:
    /// ```ds
    /// namespace.value<T>
    /// ```
    pub(in crate::check) fn walk_qualified_reference_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        path: &dir::Path,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<()> {
        let source = id.into_global_any(self.module);

        // resolve the path through the resolve-phase table
        let key = dir::PathKey::new(source, path.segments.len() as u32);
        let resolution = self
            .check
            .module(self.module)
            .resolved
            .paths
            .get(key)
            .cloned();
        let symbol = match resolution {
            Some(dir::PathResolution::Found(dir::PathTarget::Symbol(symbol))) => Some(symbol),
            Some(dir::PathResolution::Found(dir::PathTarget::Namespace(_))) => None,
            Some(dir::PathResolution::Ambiguous(_)) => {
                self.check
                    .report_ambiguous_reference(self.module, id.into_any(), path);

                None
            }
            Some(dir::PathResolution::Missing) | None => {
                // single-segment paths resolve lexically
                if let [name] = path.segments.as_slice() {
                    return self.walk_identifier_reference(id, *name, generic_arguments);
                }
                self.check
                    .report_unresolved_reference(self.module, id.into_any(), path);

                None
            }
        };

        let Some(symbol) = symbol else {
            let error = self.push_type(dir::Type::Error, id.into_any())?;
            self.declare_node_type(id, error)?;

            return Ok(());
        };

        self.capture_symbol_reference(symbol);
        self.check
            .record_decision(source, Decision::Name(dir::NameResolution::new(symbol)))?;
        let ty = self.applied_reference_type(id, symbol, generic_arguments)?;
        self.declare_node_type(id, ty)?;

        Ok(())
    }

    /// Walk one identifier with applied generic arguments.
    fn walk_identifier_reference(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        name: dir::StringId,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<()> {
        if generic_arguments.is_empty() {
            return self.walk_identifier_expression(id, name);
        }

        let source = id.into_global_any(self.module);
        let lookup =
            self.check
                .lookup_name(self.module, id.into_any(), name, dir::SymbolSpace::Value);
        let symbol = match lookup {
            NameLookup::Found(candidate) => candidate.symbol(),
            NameLookup::Missing | NameLookup::Ambiguous(_) => None,
        };
        let Some(symbol) = symbol else {
            let path = dir::Path {
                segments: smallvec::smallvec![name],
            };
            self.check
                .report_unresolved_reference(self.module, id.into_any(), &path);
            let error = self.push_type(dir::Type::Error, id.into_any())?;
            self.declare_node_type(id, error)?;

            return Ok(());
        };

        self.capture_symbol_reference(symbol);
        self.check
            .record_decision(source, Decision::Name(dir::NameResolution::new(symbol)))?;
        let ty = self.applied_reference_type(id, symbol, generic_arguments)?;
        self.declare_node_type(id, ty)?;

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

        // apply explicit arguments to the selected declaration
        let symbol = match self.check.decisions.get(left.into_global_any(self.module)) {
            Some(Decision::Name(resolution)) => match resolution.symbols() {
                [symbol] => Some(*symbol),
                _ => None,
            },
            _ => None,
        };

        match symbol {
            Some(symbol) => {
                let source = id.into_global_any(self.module);
                self.check
                    .record_decision(source, Decision::Name(dir::NameResolution::new(symbol)))?;
                let ty = self.applied_reference_type(id, symbol, generic_arguments)?;
                self.declare_node_type(id, ty)?;
            }
            // unresolved or overloaded instantiations resolve at selection
            None => {
                for argument in generic_arguments {
                    self.walk_generic_arguments(std::slice::from_ref(argument))?;
                }
                self.node_type(id)?;
                self.queue_select(id.into_global_any(self.module));
            }
        }

        Ok(())
    }

    /// Return one symbol's reference value type at a use site.
    /// Type declarations write their applied reference, values keep
    /// their declared type.
    pub(in crate::check) fn reference_symbol_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.symbol_type(symbol)
    }

    /// Return one declaration applied to written generic arguments.
    ///
    /// Example:
    /// ```ds
    /// Box<T>
    /// ```
    fn applied_reference_type(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        symbol: dir::GlobalSymbolId,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<dir::GlobalTypeId> {
        if generic_arguments.is_empty() {
            return self.reference_symbol_type(symbol);
        }

        // canonicalize written arguments onto declared positions
        let applied = self.walk_generic_arguments(generic_arguments)?;
        let arguments = self.canonical_generic_arguments(id.into_any(), symbol, &applied)?;
        let reference = dir::Type::Reference(dir::GenericInstance { symbol, arguments });

        self.push_type(reference, id.into_any())
    }

    /// Map written generic arguments onto declared parameter positions.
    /// Missing positions open inference holes.
    pub(in crate::check) fn canonical_generic_arguments(
        &mut self,
        source: dir::LocalNodeIdAny,
        symbol: dir::GlobalSymbolId,
        applied: &[(Option<dir::StringId>, dir::GlobalTypeId)],
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        let Some(template) = self.check.generics.template_by_symbol(symbol) else {
            // non-generic targets keep their written order
            return Ok(applied.iter().map(|(_, argument)| *argument).collect());
        };
        let parameters = self.check.generic_template_parameters(template);

        // reject applications with more arguments than parameters
        if applied.len() > parameters.len() {
            let name = self.check.format_symbol(symbol);
            self.check.report_wrong_generic_arity(
                self.module,
                source,
                name,
                parameters.len(),
                applied.len(),
            );
        }

        // fill declared positions from written arguments in order
        // TODO(check): match named associated arguments onto their
        // declared parameter names instead of written order.
        let mut arguments = Vec::with_capacity(parameters.len());
        let mut written = applied.iter().map(|(_, argument)| *argument);
        for _parameter in parameters {
            let argument = match written.next() {
                Some(argument) => argument,
                // missing positions stay open for inference
                None => self.open_type(source)?,
            };
            arguments.push(argument);
        }

        Ok(arguments)
    }
}
