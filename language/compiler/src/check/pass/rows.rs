use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{CheckState, Origin};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Write one solved module into its checked DIR segments.
    pub(in crate::check) fn write_module(&mut self, module: ModuleId) -> CompilerResult<()> {
        // every type settled at its statement close; only derived statics remain
        let symbol_literals = self.static_symbol_literals(module)?;

        // write symbol values as final statics
        let state = self.module_mut(module);
        for (symbol, literal) in symbol_literals {
            let id = state
                .statics
                .push_static(dir::StaticTerm::ScalarLiteral { value: literal });
            state
                .statics
                .set_symbol_static(symbol, id.into_global(module));
        }

        // write identity values, like unique symbol keys, as type statics
        let identities = self
            .module(module)
            .static_values
            .iter()
            .map(|(symbol, value)| (*symbol, *value))
            .collect::<Vec<_>>();
        for (symbol, value) in identities {
            let value = self.resolve_committed_type(value, &FxIndexSet::default())?;
            let state = self.module_mut(module);
            if state.statics.get_symbol_static_id(symbol).is_some() {
                continue;
            }
            let id = state
                .statics
                .push_static(dir::StaticTerm::Type { ty: value });
            state
                .statics
                .set_symbol_static(symbol, id.into_global(module));
        }

        // store evaluable module constants beside the literal statics
        let constants = self.static_module_constants(module)?;
        let state = self.module_mut(module);
        for (symbol, term) in constants {
            if state.statics.get_symbol_static_id(symbol).is_some() {
                continue;
            }
            let id = state.statics.push_static(term);
            state
                .statics
                .set_symbol_static(symbol, id.into_global(module));
        }

        // write closure capture frames and bindings
        self.write_captures(module)?;

        // record source resolutions
        if !self.is_declaration() {
            self.write_path_segment_resolutions(module)?;
            self.write_member_type_resolutions(module)?;
        }

        Ok(())
    }

    /// Write the declaration selected for each source path segment.
    fn write_path_segment_resolutions(&mut self, module: ModuleId) -> CompilerResult<()> {
        // collect exact path sites before mutating resolutions
        let sites = self
            .module(module)
            .members
            .iter_subjects()
            .filter_map(|(site, _)| match site {
                dir::MemberSite::Path { node, segment } => Some((node, segment)),
                dir::MemberSite::Node(_) => None,
            })
            .collect::<Vec<_>>();

        for (node, segment) in sites {
            let site = dir::MemberSite::Path { node, segment };

            // read the key at this exact path site
            if node.local_id.ty != dir::NodeType::TypeExpression {
                return Err(CompilerError::Internal {
                    message: format!("path member site {site:?} is not a type expression"),
                });
            }
            let type_expression = node.local_id.into_typed::<dir::TypeExpression>();
            let dir::TypeExpression::Reference { path, .. } =
                self.module(module).view().get(type_expression)
            else {
                return Err(CompilerError::Internal {
                    message: format!("path member site {site:?} is not a reference path"),
                });
            };
            let name = path
                .segments
                .get(segment as usize)
                .copied()
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("path member site {site:?} has no source segment"),
                })?;
            let key = dir::StaticKey::Name(name);

            // copy the declaration selected by member lookup
            let resolution = self
                .module(module)
                .members
                .binding(site, key)
                .and_then(dir::MemberBinding::declaration_resolution);
            if let Some(resolution) = resolution {
                self.commit_path_resolution(node, segment, resolution)?;
            }
        }

        Ok(())
    }

    /// Resolve each source member type expression to its selected symbol.
    fn write_member_type_resolutions(&mut self, module: ModuleId) -> CompilerResult<()> {
        // collect the member type expressions the walk recorded subjects for
        let sites = self
            .module(module)
            .members
            .iter_subjects()
            .filter_map(|(site, _)| match site {
                dir::MemberSite::Node(node)
                    if node.local_id.ty == dir::NodeType::TypeExpression =>
                {
                    Some(node)
                }
                dir::MemberSite::Node(_) | dir::MemberSite::Path { .. } => None,
            })
            .collect::<Vec<_>>();

        for node in sites {
            let site = dir::MemberSite::Node(node);

            // read the member key at this exact node
            let type_expression = node.local_id.into_typed::<dir::TypeExpression>();
            let dir::TypeExpression::Member { name, .. } =
                self.module(module).view().get(type_expression)
            else {
                return Err(CompilerError::Internal {
                    message: format!("member type site {site:?} is not a member expression"),
                });
            };
            let key = dir::StaticKey::Name(*name);

            // resolve the declaration selected by member lookup
            let Some(subject) = self.module(module).members.subject(site) else {
                continue;
            };
            let origin = Origin::Node(node, subject.scope);
            let subject = self.declared_member_subject(subject)?;
            let bindings = self
                .body()
                .resolve_member_bindings(origin, module, subject)?;
            let resolution = bindings
                .iter()
                .find(|binding| binding.key == key)
                .and_then(dir::MemberBinding::declaration_resolution);
            if let Some(resolution) = resolution {
                self.commit_name(node, resolution)?;
            }
        }

        Ok(())
    }

    /// Commit one path segment resolution into its module's resolution segment.
    fn commit_path_resolution(
        &mut self,
        node: dir::GlobalNodeIdAny,
        segment: u16,
        resolution: dir::NameResolution,
    ) -> CompilerResult<()> {
        let previous = self
            .module(node.module_id)
            .resolutions
            .path_resolution(node, segment);

        // collapse identical re-derivations, reject conflicting ones
        if let Some(previous) = previous {
            if previous == &resolution {
                return Ok(());
            }

            return Err(CompilerError::Internal {
                message: format!(
                    "check node {} selected conflicting resolutions {previous:?} and {resolution:?} for segment {segment}",
                    self.node_label(node),
                ),
            });
        }

        self.module_mut(node.module_id)
            .resolutions
            .set_path_resolution(node, segment, resolution);

        Ok(())
    }

    /// Bind bare generic subject references through their declared applications.
    fn declared_member_subject(
        &mut self,
        subject: dir::MemberSubject,
    ) -> CompilerResult<dir::MemberSubject> {
        let receiver = self.declared_subject_type(subject.receiver)?;
        let target = self.declared_subject_type(subject.target)?;
        let key_type = self.declared_subject_type(subject.key_type)?;

        Ok(dir::MemberSubject {
            receiver,
            target,
            key_type,
            ..subject
        })
    }

    /// Return one bare generic reference as its own declared application.
    fn declared_subject_type(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let dir::Type::Reference(reference) = self.ty(ty)? else {
            return Ok(ty);
        };
        let symbol = self.resolve_symbol_alias(reference.symbol)?;
        if self.symbol_template(symbol)?.is_none() {
            return Ok(ty);
        }
        let instance = self.declaration_instance(symbol)?;

        self.intern_type(dir::Type::Application(instance))
    }

    /// Resolve one module's literal symbol values.
    fn static_symbol_literals(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Vec<(dir::GlobalSymbolId, dir::ScalarLiteral)>> {
        let static_values = self
            .module(module)
            .static_values
            .iter()
            .map(|(symbol, value)| (*symbol, *value))
            .collect::<Vec<_>>();
        let mut literals = Vec::new();
        for (symbol, value) in static_values {
            let value = self.shallow_resolve(value)?;
            if let dir::Type::Literal(literal) = self.ty(value)? {
                literals.push((symbol, literal));
            }
        }

        Ok(literals)
    }

    /// Evaluate module-level const initializers into static values.
    ///
    /// Initializers that stay runtime store no value.
    fn static_module_constants(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Vec<(dir::GlobalSymbolId, dir::StaticTerm)>> {
        let input = self.module(module);
        let parsed = input.parsed.clone();
        let expanded = input.expanded.clone();
        let tree = dir::View::with_patches(&parsed.tree, std::slice::from_ref(&expanded.patch));

        // collect the constant declarator bindings first
        let mut bindings = Vec::new();
        for root in &expanded.roots {
            let dir::Expression::Let {
                mutability: dir::Mutability::Immutable,
                ref declarators,
                ..
            } = *tree.get(*root)
            else {
                continue;
            };
            for declarator in declarators {
                let declarator = tree.get(*declarator);
                let Some(value) = declarator.value else {
                    continue;
                };
                let pattern = declarator.pattern.into_any();
                let Some(symbol) = self.module(module).declaration_symbol(pattern) else {
                    continue;
                };
                bindings.push((symbol, value));
            }
        }

        // evaluate each initializer, keeping only the static ones
        let mut constants = Vec::new();
        for (symbol, value) in bindings {
            if let Ok(term) = self.evaluate_static_expression(module, value)? {
                constants.push((symbol, term));
            }
        }

        Ok(constants)
    }
}
