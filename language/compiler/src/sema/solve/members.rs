use destack_dir as dir;
use destack_dir::TypeFold;
use destack_source::ModuleId;

use crate::sema::{CheckState, Origin};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Re-key every stored member site on its resolved subject.
    pub(in crate::sema) fn resolve_member_subjects(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<()> {
        for (site, stored, key) in self.member_sites(module) {
            self.resolve_member_site(module, site, stored, key)?;
        }

        Ok(())
    }

    /// Return the member sites this pass stored, in selection order.
    fn member_sites(
        &self,
        module: ModuleId,
    ) -> Vec<(dir::MemberSite, dir::MemberSubject, Option<dir::StaticKey>)> {
        self.module(module)
            .iter_member_subjects()
            .collect::<Vec<_>>()
    }

    /// Re-key one stored member site on its resolved subject.
    fn resolve_member_site(
        &mut self,
        module: ModuleId,
        site: dir::MemberSite,
        stored: dir::MemberSubject,
        key: Option<dir::StaticKey>,
    ) -> CompilerResult<dir::MemberSubject> {
        let subject = self.resolved_member_subject(stored)?;
        if subject != stored {
            self.module_mut(module)
                .members_tail
                .commit_subject(site, subject, key);
        }

        Ok(subject)
    }

    /// Store the membership each resolved subject selects.
    pub(in crate::sema) fn settle_member_bindings(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<()> {
        for (site, stored, key) in self.member_sites(module) {
            // re-key the site against the subject inference solved
            let subject = self.resolve_member_site(module, site, stored, key)?;

            // membership is a function of the resolved subject, one projection stands for all sites
            if self.module(module).membership(&subject).is_some() {
                continue;
            }

            // store the membership the first resolving site projects
            let membership = self.resolved_membership(site, module, subject)?;
            self.module_mut(module)
                .members_tail
                .set_membership(subject, membership);
        }

        Ok(())
    }

    /// Project one resolved subject's membership, resolving the types of its bindings.
    fn resolved_membership(
        &mut self,
        site: dir::MemberSite,
        module: ModuleId,
        subject: dir::MemberSubject,
    ) -> CompilerResult<dir::Membership> {
        let origin = Origin::Node(site.node(), subject.scope);
        let mut membership = self.project_membership(origin, module, subject)?;

        // resolve the open types left in a structural binding
        for binding in &mut membership.structural {
            binding.map_types(&mut |ty| {
                if !self.type_flags(ty)?.has_variable() {
                    return Ok(ty);
                }
                let ty = self.erase_instantiations(ty)?;

                self.fully_resolve(ty)
            })?;
        }

        Ok(membership)
    }

    /// Replace instantiation variables in one type with erased parameter holes.
    pub(in crate::sema) fn erase_instantiations(
        &mut self,
        mut id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        for variable in self.type_variables(id)? {
            let Some(parameter) = self.infer.variable(variable)?.parameter else {
                continue;
            };
            let from = self.intern_type(dir::Type::Variable(variable))?;
            let to = self.intern_type(dir::Type::Erased(parameter))?;
            id = self.replace_type(id, from, to)?;
        }

        Ok(id)
    }

    /// Project the membership one resolved subject selects.
    /// NOTE #Incomplete: extension members compose into the structural bindings.
    fn project_membership(
        &mut self,
        origin: Origin,
        module: ModuleId,
        subject: dir::MemberSubject,
    ) -> CompilerResult<dir::Membership> {
        // reduce projection heads to their identities first
        let subject = match self.ty(subject.key_source)? {
            dir::Type::Member(_) | dir::Type::Operation(_) => {
                let reduced = self.normalize_computation(origin, subject.key_source)?;

                subject.with_key_source(reduced)
            }
            _ => subject,
        };

        // strip the receiver's form once for every projection below
        let receiver = self.strip_form(origin, subject.receiver)?;

        // reference declaration-backed subjects by their owner and arguments
        let core = self.strip_form(origin, subject.key_source)?;
        let instance = self.apparent_instance(core)?;
        let is_newtype = match &instance {
            Some(instance) => matches!(
                self.definition(instance.symbol)?.as_deref(),
                Some(dir::Definition::Newtype(_))
            ),
            None => false,
        };
        if let Some(instance) = instance
            && !is_newtype
            && self
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
            let root = dir::TypeRoot::Declaration(instance.symbol);
            let extensions =
                self.subject_extensions(origin, module, subject.receiver, core, root)?;
            for extension in extensions {
                let Some(source) = self.decide_extension_source(
                    origin,
                    module,
                    subject.receiver,
                    core,
                    extension,
                )?
                else {
                    continue;
                };
                let arguments = self.intern_type_ids(&source.arguments)?;
                membership.sources.push(dir::MemberSource {
                    owner: source.extension,
                    arguments,
                });
            }

            return Ok(membership);
        }

        // merge generic parameter subjects over their bounds' sources
        if let dir::Type::Parameter(parameter) = self.ty(subject.key_source)? {
            let bounds = self.parameter_bounds(origin, parameter)?;
            let mut membership = dir::Membership {
                receiver,
                sources: Vec::new(),
                structural: Vec::new(),
            };
            for bound in bounds {
                let projected =
                    self.project_membership(origin, module, subject.with_key_source(bound))?;

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

        // project structural subjects through keyed lookups
        let keys = self.subject_member_keys(origin, module, &subject)?;
        let mut structural = Vec::with_capacity(keys.len());
        for key in keys {
            let lookup = self.lookup_member(origin, module, subject, key)?;
            if let Some(binding) = self.member_binding(key, &lookup)? {
                structural.push(binding);
            }
        }

        Ok(dir::Membership {
            receiver,
            sources: Vec::new(),
            structural,
        })
    }

    /// Write the declaration each member site's stored bindings select at its retained key.
    pub(in crate::sema) fn settle_member_resolutions(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<()> {
        for (site, _, key) in self.member_sites(module) {
            // pass over the shape sites, which project a membership and retain no key
            let Some(key) = key else {
                continue;
            };

            // report an unknown key written as a refinement argument
            let Some(resolution) = self.member_site_resolution(module, site, key)? else {
                if let dir::MemberSite::Node(node) = site
                    && node.local_id.ty == dir::NodeType::GenericArgument
                    && let Some((subject, _)) = self.module(module).member_subject(site)
                {
                    let key = self.format_static_key(&key);
                    let origin = Origin::Node(node, subject.scope);
                    self.report_missing_member(origin, subject.target, key, None, None)?;
                }

                continue;
            };

            // settle the site on the declaration it selected
            match site {
                dir::MemberSite::Path { node, segment } => {
                    self.commit_path_resolution(node, segment, resolution)?;
                }
                dir::MemberSite::Node(node) => {
                    self.commit_name(node, resolution)?;
                }
            }
        }

        Ok(())
    }

    /// Return the declaration one site's resolved subject selects at one key.
    fn member_site_resolution(
        &mut self,
        module: ModuleId,
        site: dir::MemberSite,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<dir::NameResolution>> {
        // look the key up at the resolved subject, memberships projecting only at analysis
        let Some((subject, _)) = self.module(module).member_subject(site) else {
            return Ok(None);
        };
        let origin = Origin::Node(site.node(), subject.scope);
        let lookup = self.lookup_member(origin, module, subject, key)?;

        Ok(self
            .member_binding(key, &lookup)?
            .and_then(|binding| binding.declaration_resolution()))
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

        // collapse identical re-derivations and reject conflicting ones
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

        // commit the resolution at the path segment
        self.module_mut(node.module_id)
            .resolutions
            .set_path_resolution(node, segment, resolution);

        Ok(())
    }

    /// Return one member lookup subject over its resolved types.
    fn resolved_member_subject(
        &mut self,
        subject: dir::MemberSubject,
    ) -> CompilerResult<dir::MemberSubject> {
        let receiver = self.fully_resolve(subject.receiver)?;
        let target = self.fully_resolve(subject.target)?;
        let key_source = self.fully_resolve(subject.key_source)?;

        Ok(dir::MemberSubject {
            receiver,
            target,
            key_source,
            ..subject
        })
    }
}
