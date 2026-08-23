use destack_dir as dir;
use destack_dir::TypeFold;
use destack_repository::ArtifactAttemptRecorder;
use destack_source::ModuleId;

use crate::sema::{CheckState, Origin, VariableRole};
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
                .push_static(dir::StaticTerm::Literal { value: literal });
            state
                .statics_tail
                .set_symbol_static(symbol, id.into_global(module));
        }

        // collect the identity values
        let identities = self
            .module(module)
            .static_values
            .iter()
            .map(|(symbol, value)| (*symbol, *value))
            .collect::<Vec<_>>();

        // write each identity value as a type static
        for (symbol, value) in identities {
            let value = self.fully_resolve(value)?;
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

        // evaluate module constants the check phase left undecided
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

        Ok(())
    }

    /// Evaluate module const initializers into static terms after solving.
    ///
    /// Constants a same-module const generic argument forced commit at check
    /// time; this pass evaluates the rest over the solved types.
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

    /// Store the membership each settled subject selects.
    fn write_member_bindings(&mut self, module: ModuleId) -> CompilerResult<()> {
        for (site, recorded) in self.recorded_member_sites(module) {
            // re-key the site against the subject inference solved
            let subject = self.settle_member_site(module, site, recorded)?;

            // membership is a function of the settled subject, one projection stands for all sites
            if self.module(module).membership(&subject).is_some() {
                continue;
            }

            // store the membership the first settling site projects
            let membership = self.settled_membership(site, module, subject)?;
            self.module_mut(module)
                .members_tail
                .set_membership(subject, membership);
        }

        Ok(())
    }

    /// Project one settled subject's membership, settling the types its bindings carry.
    fn settled_membership(
        &mut self,
        site: dir::MemberSite,
        module: ModuleId,
        subject: dir::MemberSubject,
    ) -> CompilerResult<dir::Membership> {
        // project the membership under the settling flag
        let origin = Origin::Node(site.node(), subject.scope);
        self.is_settling = true;
        let membership = self.project_membership(origin, module, subject);
        self.is_settling = false;
        let mut membership = membership?;

        // settle the open types a structural binding still carries
        for binding in &mut membership.structural {
            binding.map_types(&mut |ty| {
                if !self.type_flags(ty)?.has_variable() {
                    return Ok(ty);
                }
                let ty = self.erase_instantiations(module, ty)?;

                self.fully_resolve(ty)
            })?;
        }

        Ok(membership)
    }

    /// Replace instantiation variables in one type with erased parameter holes.
    pub(in crate::sema) fn erase_instantiations(
        &mut self,
        module: ModuleId,
        mut id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        for variable in self.type_variables(id)? {
            let VariableRole::Instantiation { parameter } = self.infer.variable_role(variable)?
            else {
                continue;
            };
            let from = self.intern_type(dir::Type::Variable(variable))?;
            let to = self.intern_type(dir::Type::Erased(parameter))?;
            id = self.replace_type(module, id, from, to)?;
        }

        Ok(id)
    }

    /// Project the membership one settled subject selects.
    ///
    /// NOTE #Incomplete: extension members compose into the structural bindings
    /// until candidates carry their deduced arguments, where stage two references
    /// them as sources beside the nominal owner.
    fn project_membership(
        &mut self,
        origin: Origin,
        module: ModuleId,
        subject: dir::MemberSubject,
    ) -> CompilerResult<dir::Membership> {
        // reduce projection heads to their identities first
        let subject = match self.ty(subject.key_type)? {
            dir::Type::Member(_) | dir::Type::Operation(_) => {
                let reduced = self.normalize_computation(origin, subject.key_type)?;

                subject.with_key_type(reduced)
            }
            _ => subject,
        };

        // strip the receiver's form once for every projection below
        let receiver = self.strip_form(origin, subject.receiver)?;

        // reference declaration-backed subjects by their owner and arguments
        let instance = self.apparent_instance(subject.key_type)?;
        let is_newtype = match &instance {
            Some(instance) => matches!(
                self.definition(instance.symbol)?,
                Some(dir::Definition::Newtype(_))
            ),
            None => false,
        };
        if let Some(instance) = instance
            && !is_newtype
            && self
                .body()
                .member_bindings(instance.symbol, subject.space)?
                .is_some()
        {
            let arguments = self.intern_type_ids(&instance.arguments)?;
            let mut membership = dir::Membership {
                receiver,
                sources: vec![dir::MemberSource {
                    owner: instance.symbol,
                    arguments,
                }],
                structural: Vec::new(),
            };

            // reference the matching extensions as sources beside the owner
            let core = self.strip_form(origin, subject.key_type)?;
            let sources = self.body().decided_extension_sources(
                origin,
                module,
                subject.receiver,
                core,
                dir::TypeRoot::Declaration(instance.symbol),
            )?;
            for source in sources {
                let arguments = self.intern_type_ids(&source.arguments)?;
                membership.sources.push(dir::MemberSource {
                    owner: source.extension,
                    arguments,
                });
            }

            return Ok(membership);
        }

        // merge generic parameter subjects over their bounds' sources
        if let dir::Type::Parameter(parameter) = self.ty(subject.key_type)? {
            let bounds = self.body().parameter_bounds(origin, parameter)?;
            let mut membership = dir::Membership {
                receiver,
                sources: Vec::new(),
                structural: Vec::new(),
            };
            for bound in bounds {
                let projected =
                    self.project_membership(origin, module, subject.with_key_type(bound))?;

                // keep each owner and structural key the bounds contribute once
                for source in projected.sources {
                    if !membership.sources.contains(&source) {
                        membership.sources.push(source);
                    }
                }
                for binding in projected.structural {
                    if membership
                        .structural
                        .iter()
                        .all(|existing| existing.key != binding.key)
                    {
                        membership.structural.push(binding);
                    }
                }
            }

            return Ok(membership);
        }

        // fall back to keyed lookups for the remaining subject heads
        let keys = self.body().subject_member_keys(origin, module, &subject)?;
        let mut structural = Vec::with_capacity(keys.len());
        for key in keys {
            let lookup = self.body().lookup_member(origin, module, subject, key)?;
            if let Some(binding) = self.body().member_binding(key, &lookup)? {
                structural.push(binding);
            }
        }

        Ok(dir::Membership {
            receiver,
            sources: Vec::new(),
            structural,
        })
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

    /// Return the declaration one site's stored member membership selects at one key.
    fn member_site_resolution(
        &mut self,
        module: ModuleId,
        site: dir::MemberSite,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<dir::NameResolution>> {
        // read the membership the site's settled subject stored
        let Some(subject) = self.module(module).member_subject(site) else {
            return Ok(None);
        };
        let membership = self
            .module(module)
            .membership(&subject)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!("member site {site:?} stored no membership for {subject:?}"),
            })?;

        // answer from the structural bindings the subject projected
        if let Some(binding) = membership
            .structural
            .iter()
            .find(|binding| binding.key == key)
        {
            return Ok(binding.declaration_resolution());
        }

        // answer from each source owner's declared bindings
        for source in membership.sources {
            let Some(bindings) = self.body().member_bindings(source.owner, subject.space)? else {
                continue;
            };
            if let Some(binding) = bindings.iter().find(|binding| binding.key == key) {
                return Ok(binding.declaration_resolution());
            }
        }

        Ok(None)
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
        let receiver = self.fully_resolve(subject.receiver)?;
        let target = self.fully_resolve(subject.target)?;
        let key_type = self.fully_resolve(subject.key_type)?;

        Ok(dir::MemberSubject {
            receiver,
            target,
            key_type,
            ..subject
        })
    }

    /// Resolve one module's literal symbol values.
    fn static_symbol_literals(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Vec<(dir::GlobalSymbolId, dir::Literal)>> {
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
}
