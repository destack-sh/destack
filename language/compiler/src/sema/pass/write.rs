use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;
use destack_dir::TypeFold;
use destack_repository::ArtifactAttemptRecorder;
use destack_source::ModuleId;

use crate::sema::{CheckState, Origin};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Write one solved module into its checked DIR segments.
    pub(in crate::sema) fn write_module(&mut self, module: ModuleId) -> CompilerResult<()> {
        // resolve the literal value each symbol settled on
        let symbol_literals = self.static_symbol_literals(module)?;

        // write symbol values as final statics
        let state = self.module_mut(module);
        for (symbol, literal) in symbol_literals {
            let id = state
                .statics_tail
                .push_static(dir::StaticTerm::ScalarLiteral { value: literal });
            state
                .statics_tail
                .set_symbol_static(symbol, id.into_global(module));
        }

        // collect the identity values, like unique symbol keys
        let identities = self
            .module(module)
            .static_values
            .iter()
            .map(|(symbol, value)| (*symbol, *value))
            .collect::<Vec<_>>();

        // write each identity value as a type static
        for (symbol, value) in identities {
            let value = self.fully_resolve(value, &FxIndexSet::default())?;
            let state = self.module_mut(module);
            if state.statics_tail.get_symbol_static_id(symbol).is_some() {
                continue;
            }
            let id = state
                .statics_tail
                .push_static(dir::StaticTerm::Type { ty: value });
            state
                .statics_tail
                .set_symbol_static(symbol, id.into_global(module));
        }

        // expose each class declaration as a type static for its value uses
        let classes = self
            .module(module)
            .iter_definitions()
            .filter(|(_, definition)| matches!(definition, dir::Definition::Class(_)))
            .map(|(symbol, _)| symbol)
            .collect::<Vec<_>>();
        for symbol in classes {
            if self.symbol_static_id(symbol).is_some() {
                continue;
            }

            // push one type static naming the class
            let ty = self.intern_type(dir::Type::Reference(dir::TypeReference { symbol }))?;
            let state = self.module_mut(module);
            let id = state.statics_tail.push_static(dir::StaticTerm::Type { ty });
            state
                .statics_tail
                .set_symbol_static(symbol, id.into_global(module));
        }

        // store evaluable module constants beside the literal statics
        let constants = self.static_module_constants(module)?;
        let state = self.module_mut(module);

        for (symbol, term) in constants {
            if state.statics_tail.get_symbol_static_id(symbol).is_some() {
                continue;
            }
            let id = state.statics_tail.push_static(term);
            state
                .statics_tail
                .set_symbol_static(symbol, id.into_global(module));
        }

        // write closure capture frames and bindings
        self.write_captures(module)?;

        // settle every member site this pass recorded, then store what they select
        if self.is_declaration() {
            self.settle_member_subjects(module)?;
        } else {
            let recorder = self.recorder;
            ArtifactAttemptRecorder::breakdown_maybe(recorder, "write.members", || {
                self.write_member_bindings(module)
            })?;
            ArtifactAttemptRecorder::breakdown_maybe(recorder, "write.resolutions", || {
                self.write_path_segment_resolutions(module)?;
                self.write_member_type_resolutions(module)
            })?;
        }

        // write the symbol uses the resolutions and captures prove
        self.write_flows();

        Ok(())
    }

    /// Re-key every recorded member site on the subject inference settled on.
    pub(in crate::sema) fn settle_member_subjects(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<()> {
        for (site, recorded) in self.recorded_member_sites(module) {
            self.settle_member_site(module, site, recorded)?;
        }

        Ok(())
    }

    /// Return the member sites this pass recorded, in selection order.
    fn recorded_member_sites(
        &self,
        module: ModuleId,
    ) -> Vec<(dir::MemberSite, dir::MemberSubject)> {
        self.module(module)
            .iter_member_subjects()
            .collect::<Vec<_>>()
    }

    /// Re-key one recorded member site on the subject inference settled on.
    fn settle_member_site(
        &mut self,
        module: ModuleId,
        site: dir::MemberSite,
        recorded: dir::MemberSubject,
    ) -> CompilerResult<dir::MemberSubject> {
        let subject = self.settle_member_subject(recorded)?;
        if subject != recorded {
            self.module_mut(module)
                .members_tail
                .record_subject(site, subject);
        }

        Ok(subject)
    }

    /// Store the member bindings each subject this pass recorded selects.
    fn write_member_bindings(&mut self, module: ModuleId) -> CompilerResult<()> {
        // resolve each settled subject once, at the first site that selected it
        let mut first_recorded: FxIndexMap<dir::MemberSubject, dir::MemberSubject> =
            FxIndexMap::default();
        for (site, recorded) in self.recorded_member_sites(module) {
            // re-key the site, since inference solved its subject after selection
            let subject = self.settle_member_site(module, site, recorded)?;

            // store what this subject selects the first time it settles
            if self
                .module(module)
                .member_subject_bindings(&subject)
                .is_none()
            {
                let bindings = self.settled_member_bindings(module, site, subject)?;
                self.module_mut(module)
                    .members_tail
                    .set_bindings(subject, bindings);
                first_recorded.insert(subject, recorded);
            }
            // require a second recorded form settling on this subject to select the same members
            else if let Some(first) = first_recorded.get(&subject).copied()
                && first != recorded
            {
                let bindings = self.settled_member_bindings(module, site, subject)?;
                let stored = self.module(module).member_subject_bindings(&subject);
                if stored != Some(bindings.as_slice()) {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "member subjects {first:?} and {recorded:?} settle to {subject:?} with conflicting bindings"
                        ),
                    });
                }
            }
        }

        Ok(())
    }

    /// Resolve the member bindings one subject selects, over the types inference settled on.
    fn settled_member_bindings(
        &mut self,
        module: ModuleId,
        site: dir::MemberSite,
        subject: dir::MemberSubject,
    ) -> CompilerResult<Vec<dir::MemberBinding>> {
        // resolve the subject in its declared form and read what it selects
        let origin = Origin::Node(site.node(), subject.scope);
        let declared = self.declared_member_subject(subject)?;
        let mut bindings = self
            .body()
            .subject_member_bindings(origin, module, declared)?;

        // settle the access and callable types each selected binding carries
        let intact = FxIndexSet::default();
        for binding in &mut bindings {
            binding.map_types(&mut |ty| self.fully_resolve(ty, &intact))?;
        }

        Ok(bindings)
    }

    /// Write the declaration selected for each source path segment.
    fn write_path_segment_resolutions(&mut self, module: ModuleId) -> CompilerResult<()> {
        // collect exact path sites before mutating resolutions
        let sites = self
            .module(module)
            .iter_member_subjects()
            .filter_map(|(site, _)| match site {
                dir::MemberSite::Path { node, segment } => Some((node, segment)),
                dir::MemberSite::Node(_) => None,
            })
            .collect::<Vec<_>>();

        for (node, segment) in sites {
            let site = dir::MemberSite::Path { node, segment };

            // require the site to name a type expression
            if node.local_id.ty != dir::NodeType::TypeExpression {
                return Err(CompilerError::Internal {
                    message: format!("path member site {site:?} is not a type expression"),
                });
            }

            // read the key at this exact path site
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

            // commit the declaration the stored bindings select at this key
            let resolution = self.member_site_resolution(module, site, key)?;
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
            .iter_member_subjects()
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

            // commit the declaration the stored bindings select at this key
            let resolution = self.member_site_resolution(module, site, key)?;
            if let Some(resolution) = resolution {
                self.commit_name(node, resolution)?;
            }
        }

        Ok(())
    }

    /// Return the declaration one site's stored member bindings select at one key.
    fn member_site_resolution(
        &self,
        module: ModuleId,
        site: dir::MemberSite,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<dir::NameResolution>> {
        let Some(subject) = self.module(module).member_subject(site) else {
            return Ok(None);
        };
        let bindings = self
            .module(module)
            .member_subject_bindings(&subject)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("member site {site:?} stored no bindings for {subject:?}"),
            })?;

        Ok(bindings
            .iter()
            .find(|binding| binding.key == key)
            .and_then(dir::MemberBinding::declaration_resolution))
    }

    /// Commit one path segment resolution into its module's resolution segment.
    fn commit_path_resolution(
        &mut self,
        node: dir::GlobalNodeIdAny,
        segment: u16,
        resolution: dir::NameResolution,
    ) -> CompilerResult<()> {
        // read whatever an earlier derivation wrote at this segment
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

    /// Re-derive one member lookup subject over the types inference settled on.
    fn settle_member_subject(
        &mut self,
        subject: dir::MemberSubject,
    ) -> CompilerResult<dir::MemberSubject> {
        let intact = FxIndexSet::default();
        let receiver = self.fully_resolve(subject.receiver, &intact)?;
        let target = self.fully_resolve(subject.target, &intact)?;
        let key_type = self.fully_resolve(subject.key_type, &intact)?;

        Ok(dir::MemberSubject {
            receiver,
            target,
            key_type,
            ..subject
        })
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

        // name an application for generic declarations only
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
        // collect the values the walk recorded per symbol
        let static_values = self
            .module(module)
            .static_values
            .iter()
            .map(|(symbol, value)| (*symbol, *value))
            .collect::<Vec<_>>();

        // keep the symbols whose value settled on a scalar literal
        let mut literals = Vec::new();
        for (symbol, value) in static_values {
            let value = self.shallow_resolve(value)?;
            if let dir::Type::Literal(literal) = self.ty(value)? {
                literals.push((symbol, literal));
            }
        }

        Ok(literals)
    }

    /// Evaluate module-level const initializers into static values, skipping runtime ones.
    fn static_module_constants(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Vec<(dir::GlobalSymbolId, dir::StaticTerm)>> {
        // read the module's expanded tree
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
