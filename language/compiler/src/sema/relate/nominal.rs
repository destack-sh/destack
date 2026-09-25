use smallvec::SmallVec;
use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::sema::{Cause, CauseId, CauseKind, CheckState, Origin, Relation, Verdict};
use crate::{CompilerError, CompilerResult};

/// One applied heritage edge in a nominal declaration closure.
#[derive(Debug, Clone)]
pub(in crate::sema) struct HeritageApplication {
    /// The source clause that introduced the application.
    pub(in crate::sema) source: dir::GlobalNodeIdAny,
    /// The applied nominal or interface type.
    pub(in crate::sema) ty: dir::GlobalTypeId,
    /// The application whose heritage clause introduced this one, at the receiver's arguments.
    pub(in crate::sema) implementer: dir::GlobalTypeId,
}

/// One interface application in a receiver's heritage, with its implementing declaration.
#[derive(Debug, Clone, Copy)]
pub(in crate::sema) struct DeclaredConformance {
    /// The application of the declaration whose heritage clause names the interface.
    pub(in crate::sema) implementer: dir::GlobalTypeId,
    /// The reached interface application.
    pub(in crate::sema) interface: dir::GlobalTypeId,
}

/// One duplicate heritage application with incompatible arguments.
#[derive(Debug, Clone)]
pub(in crate::sema) struct HeritageConflict {
    /// The source clause that introduced the conflicting application.
    pub(in crate::sema) source: dir::GlobalNodeIdAny,
    /// The conflicting inherited type.
    pub(in crate::sema) current: dir::GlobalTypeId,
}

/// One heritage branch that exposes a cycle.
#[derive(Debug, Clone)]
pub(in crate::sema) struct HeritageCycle {
    /// The source branch that exposes the cycle.
    pub(in crate::sema) source: dir::GlobalNodeIdAny,
}

/// Complete heritage closure for one nominal or interface application.
#[derive(Debug, Clone, Default)]
pub(in crate::sema) struct HeritageClosure {
    /// The inherited applications in traversal order.
    pub(in crate::sema) applications: SmallVec<[HeritageApplication; 8]>,
    /// Duplicate applications with different arguments.
    pub(in crate::sema) conflicts: SmallVec<[HeritageConflict; 2]>,
    /// Cycles found while walking heritage edges.
    pub(in crate::sema) cycles: SmallVec<[HeritageCycle; 2]>,
}

/// The declarations one declaration's heritage reaches.
#[derive(Debug, Clone)]
pub(in crate::sema) enum HeritageReach {
    /// The heritage reaches exactly these declarations, the root included.
    Closed(SmallVec<[dir::GlobalSymbolId; 8]>),
    /// The heritage passes through a parameter or an alias, so it reaches every declaration.
    Open,
}

impl HeritageClosure {
    /// Return the first application naming one symbol.
    fn application<'a>(
        &'a self,
        symbol: dir::GlobalSymbolId,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Option<&'a HeritageApplication>> {
        for application in &self.applications {
            let (_, instance) = state.nominal_application(application.ty)?;
            if instance.symbol == symbol {
                return Ok(Some(application));
            }
        }

        Ok(None)
    }
}

impl CheckState<'_> {
    /// Relate two applied declarations.
    pub(in crate::sema) fn relate_application(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        source_instance: &dir::GenericApplication,
        target: dir::GlobalTypeId,
        target_instance: &dir::GenericApplication,
    ) -> CompilerResult<Verdict> {
        // keep nominal applications symbolic while declaring
        let has_target_definition = self.definition(target_instance.symbol)?.is_some();
        let has_source_definition = self.definition(source_instance.symbol)?.is_some();
        if !has_target_definition || !has_source_definition {
            if self.is_declaring() {
                return Ok(Verdict::Holds);
            }

            return Err(CompilerError::Internal {
                message: format!(
                    "nominal relation has missing definitions in {}: source = {}, target = {}",
                    self.format_module_label(source.module_id),
                    self.format_symbol(source_instance.symbol),
                    self.format_symbol(target_instance.symbol),
                ),
            });
        }

        // read the kind the target declares
        let target_kind = self.symbol_kind(target_instance.symbol)?;

        // interface targets select one implementation path
        if target_kind.is_interface() {
            return self.relate_interface(origin, cause, relation, source, target);
        }

        // select the source application of the target declaration
        let application =
            self.heritage_application(origin, source, source_instance, target_instance.symbol)?;

        // compare every nominal path through one argument relation
        if let Some((application_module, application)) = application {
            let source_arguments: SmallVec<[_; 8]> = self
                .type_ids(application_module, application.arguments)?
                .into();
            let target_arguments: SmallVec<[_; 8]> = self
                .type_ids(target.module_id, target_instance.arguments)?
                .into();
            let form = self.default_variance_form(target_instance.symbol)?;

            let arguments = if relation == Relation::Subtype {
                self.relate_application_arguments(
                    origin,
                    cause,
                    target_instance.symbol,
                    form,
                    relation,
                    &source_arguments,
                    &target_arguments,
                )?
            } else {
                self.constrain_instance_arguments(
                    origin,
                    application_module,
                    &application,
                    target.module_id,
                    target_instance,
                )?
            };

            return Ok(arguments);
        }

        Ok(Verdict::Fails)
    }

    /// Return the first declared field missing from one struct construction.
    pub(in crate::sema) fn first_missing_struct_field(
        &mut self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        // read the written fields of the structural source
        let source_fields = match self.ty(source)? {
            dir::Type::Object(shape) => self
                .object_properties(source.module_id, shape.properties)?
                .iter()
                .map(|field| field.key)
                .collect::<SmallVec<[_; 8]>>(),
            _ => return Ok(None),
        };

        // read construction fields from a struct application only
        let dir::Type::Application(target_instance) = self.ty(target)? else {
            return Ok(None);
        };
        let definition = self.definition(target_instance.symbol)?;
        let Some(dir::Definition::Struct(_)) = definition.as_deref() else {
            return Ok(None);
        };

        // scan declared fields in construction order
        for (key, is_required) in self.nominal_instance_fields(target_instance.symbol)? {
            if source_fields.contains(&key) {
                continue;
            }

            if !is_required {
                continue;
            }

            return Ok(Some(key));
        }

        Ok(None)
    }

    /// Return the first undeclared field supplied to one struct construction.
    pub(in crate::sema) fn first_excess_struct_field(
        &mut self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        // read the written fields of the structural source
        let source_fields = match self.ty(source)? {
            dir::Type::Object(shape) => self
                .object_properties(source.module_id, shape.properties)?
                .iter()
                .map(|field| field.key)
                .collect::<SmallVec<[_; 8]>>(),
            _ => return Ok(None),
        };

        // read construction fields from a struct application only
        let dir::Type::Application(target_instance) = self.ty(target)? else {
            return Ok(None);
        };
        let definition = self.definition(target_instance.symbol)?;
        let Some(dir::Definition::Struct(_)) = definition.as_deref() else {
            return Ok(None);
        };

        // compare against declared construction fields
        let fields = self.nominal_field_keys(target_instance.symbol)?;
        for key in source_fields {
            if !fields.contains(&key) {
                return Ok(Some(key));
            }
        }

        Ok(None)
    }

    /// Return substituted direct instance fields for one struct.
    pub(in crate::sema) fn struct_fields(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<SmallVec<[dir::TypeProperty; 8]>> {
        // read the nominal application beneath the target's memory forms
        let receiver = target;
        let chain = self.form_chain(origin, receiver)?;
        let target = chain.base();
        let dir::Type::Application(instance) = self.ty(target)? else {
            return Ok(SmallVec::new());
        };

        self.instantiate_struct_fields(origin, receiver, target.module_id, &instance)
    }

    /// Return substituted fields accepted by one struct constructor.
    pub(in crate::sema) fn struct_constructor_fields(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<SmallVec<[dir::TypeProperty; 8]>> {
        // read the struct declaration behind the target's instance fields
        let mut fields = self.struct_fields(origin, target)?;
        let receiver = target;
        let chain = self.form_chain(origin, receiver)?;
        let target = chain.base();
        let dir::Type::Application(instance) = self.ty(target)? else {
            return Ok(fields);
        };
        let declared = self.definition(instance.symbol)?;
        let Some(dir::Definition::Struct(definition)) = declared.as_deref() else {
            return Ok(fields);
        };

        // initialized fields may be omitted from construction
        for field in &mut fields {
            let is_initialized = definition.members.iter().any(|member| match member {
                dir::DefinitionMember::Field(declared) => {
                    declared.space == dir::MemberSpace::Instance
                        && declared.key == field.key
                        && declared.initializer.is_some()
                }
                _ => false,
            });
            field.is_optional |= is_initialized;
        }

        Ok(fields)
    }

    /// Return substituted direct fields for one struct instance.
    fn instantiate_struct_fields(
        &mut self,
        _origin: Origin,
        receiver: dir::GlobalTypeId,
        instance_module: ModuleId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<SmallVec<[dir::TypeProperty; 8]>> {
        let declared = self.definition(instance.symbol)?;
        let Some(dir::Definition::Struct(definition)) = declared.as_deref() else {
            return Ok(SmallVec::new());
        };

        // substitute the instance arguments over the declared field types
        let substitution = self
            .instance_substitution(instance_module, instance)?
            .with_receiver(receiver);
        let mut fields = SmallVec::new();

        // collect direct instance fields through the selected target
        for member in &definition.members {
            let dir::DefinitionMember::Field(field) = member else {
                continue;
            };
            if field.space != dir::MemberSpace::Instance {
                continue;
            }

            let ty = self.symbol_type(field.symbol)?;
            let ty = self.substitute_type(ty, &substitution)?;
            fields.push(dir::TypeProperty {
                key: field.key,
                access: dir::PropertyAccess::ReadWrite {
                    read: ty,
                    write: ty,
                },
                is_optional: field.is_optional,
            });
        }

        Ok(fields)
    }

    /// Bound open construction arguments from written literal fields.
    pub(in crate::sema) fn constrain_struct_construction(
        &mut self,
        origin: Origin,
        source_fields: &[dir::TypeProperty],
        target: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        // relate each written field against its declared counterpart
        let declared_fields = self.struct_fields(origin, target)?;
        for declared in declared_fields {
            let Some(field) = source_fields.iter().find(|field| field.key == declared.key) else {
                continue;
            };

            let field_cause =
                self.intern_cause(Cause::root(origin, CauseKind::Field { key: field.key }));
            self.constrain_type(
                origin,
                field_cause,
                Relation::Storable,
                field.access.store(),
                declared.access.store(),
            )?;
        }

        Ok(())
    }

    /// Collect one definition's instance field keys through heritage.
    pub(in crate::sema) fn nominal_field_keys(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<SmallVec<[dir::StaticKey; 8]>> {
        Ok(self
            .nominal_instance_fields(symbol)?
            .into_iter()
            .map(|(key, _)| key)
            .collect())
    }

    /// Collect one definition's instance fields with their required presence.
    pub(in crate::sema) fn nominal_instance_fields(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<SmallVec<[(dir::StaticKey, bool); 8]>> {
        // walk the heritage chain from the declaration, visiting each symbol once
        let mut keys = SmallVec::new();
        let mut pending = SmallVec::<[dir::GlobalSymbolId; 4]>::new();
        let mut visited = SmallVec::<[dir::GlobalSymbolId; 4]>::new();
        pending.push(symbol);

        // walk the heritage above the declaration
        while let Some(symbol) = pending.pop() {
            if visited.contains(&symbol) {
                continue;
            }
            visited.push(symbol);

            let Some(definition) = self.definition(symbol)? else {
                continue;
            };
            let members = definition.members();
            let bases = definition
                .bases()
                .iter()
                .map(|heritage| heritage.ty)
                .collect::<SmallVec<[_; 2]>>();

            // keep each declared instance field with its required presence
            for member in members {
                if let dir::DefinitionMember::Field(field) = member
                    && field.space == dir::MemberSpace::Instance
                {
                    let is_required = !field.is_optional && field.initializer.is_none();
                    keys.push((field.key, is_required));
                }
            }

            // queue the bases this declaration inherits from
            for heritage in bases {
                let (_, base) = self.nominal_application(heritage)?;
                pending.push(base.symbol);
            }
        }

        Ok(keys)
    }

    /// Collect one definition's member keys through its heritage chain.
    pub(in crate::sema) fn nominal_member_keys(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<SmallVec<[dir::StaticKey; 8]>> {
        // walk the heritage chain from the declaration, visiting each symbol once
        let mut keys = SmallVec::new();
        let mut pending = SmallVec::<[dir::GlobalSymbolId; 4]>::new();
        let mut visited = SmallVec::<[dir::GlobalSymbolId; 4]>::new();
        pending.push(symbol);

        // walk the heritage above the declaration
        while let Some(symbol) = pending.pop() {
            if visited.contains(&symbol) {
                continue;
            }
            visited.push(symbol);

            let Some(definition) = self.definition(symbol)? else {
                continue;
            };
            let members = definition.members();
            let bases = definition
                .bases()
                .iter()
                .map(|heritage| heritage.ty)
                .collect::<SmallVec<[_; 2]>>();

            // keep every keyed member of this declaration
            for member in members {
                if let Some(key) = member.key() {
                    keys.push(key);
                }
            }

            // queue the bases this declaration inherits from
            for heritage in bases {
                let (_, base) = self.nominal_application(heritage)?;
                pending.push(base.symbol);
            }
        }

        Ok(keys)
    }

    /// Relate one reference source against a structural shape target.
    pub(in crate::sema) fn relate_reference_against_target(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source_module: ModuleId,
        source_instance: &dir::GenericApplication,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // require each target field from the source fields
        let (fields, index_signatures) = match self.ty(target)? {
            dir::Type::Object(shape) => (
                self.object_properties(target.module_id, shape.properties)?
                    .iter()
                    .map(|field| (field.key, field.access.store(), field.is_optional))
                    .collect::<SmallVec<[_; 4]>>(),
                SmallVec::<[_; 4]>::from(
                    self.object_index_signatures(target.module_id, shape.index_signatures)?,
                ),
            ),
            _ => return Ok(Verdict::Fails),
        };
        let module = origin.module();
        let source = self.reference_type(origin, source_module, source_instance)?;

        // look each target key up on the source and relate what it finds
        let mut verdict = Verdict::Holds;
        for (key, field_type, is_optional) in fields {
            let subject =
                self.member_subject(origin, source, source, dir::MemberSpace::Instance)?;
            let lookup = self.lookup_member(origin, module, subject, key)?;
            let member = self.member_read_type(&lookup)?;

            match member {
                // missing members satisfy optional targets only
                None => {
                    if !is_optional {
                        return Ok(Verdict::Fails);
                    }
                }
                // relate the found member against the target field
                Some(member) => {
                    verdict = verdict
                        .and(self.constrain_type(origin, cause, relation, member, field_type)?);
                    if verdict == Verdict::Fails {
                        return Ok(Verdict::Fails);
                    }
                }
            }
        }

        // require each target index signature from the source
        for signature in index_signatures {
            verdict = verdict
                .and(self.relate_index_signature(origin, cause, relation, source, &signature)?);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        Ok(verdict)
    }

    /// Return the full heritage closure for one nominal application.
    pub(in crate::sema) fn heritage_closure(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<HeritageClosure> {
        let ty = self.heritage_root(origin, ty)?;

        // close a structural entry over an empty heritage
        if self.nominal_application_maybe(ty)?.is_none() {
            return Ok(HeritageClosure::default());
        }

        self.instance_heritage_closure(origin, ty, ty)
    }

    /// Settle one heritage root, reducing aliases to their nominal application.
    fn heritage_root(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // normalize an alias application before reading its head
        let is_alias = match self.nominal_application_maybe(ty)? {
            Some((_, instance)) => matches!(
                self.definition(instance.symbol)?.as_deref(),
                Some(dir::Definition::TypeAlias(_))
            ),
            None => true,
        };

        Ok(match is_alias {
            true => self.normalize(origin, ty)?,
            false => ty,
        })
    }

    /// Build the heritage closure from one nominal instance under one receiver.
    pub(in crate::sema) fn instance_heritage_closure(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        root: dir::GlobalTypeId,
    ) -> CompilerResult<HeritageClosure> {
        let (instance_module, instance) = self.nominal_application(root)?;
        let mut closure = HeritageClosure::default();
        let mut active = SmallVec::<[dir::GlobalSymbolId; 8]>::new();

        // let extension declarations repeat one application with different arguments
        let independent = matches!(
            self.definition(instance.symbol)?.as_deref(),
            Some(dir::Definition::Extension(_))
        );

        // walk the heritage edges from the root
        active.push(instance.symbol);
        self.collect_heritage(
            origin,
            receiver,
            instance_module,
            &instance,
            root,
            None,
            independent,
            &mut active,
            &mut closure,
        )?;

        Ok(closure)
    }

    /// Collect inherited applications from one nominal application.
    fn collect_heritage(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        instance_module: ModuleId,
        instance: &dir::GenericApplication,
        implementer: dir::GlobalTypeId,
        branch_source: Option<dir::GlobalNodeIdAny>,
        independent: bool,
        active: &mut SmallVec<[dir::GlobalSymbolId; 8]>,
        closure: &mut HeritageClosure,
    ) -> CompilerResult<()> {
        // read the declaration this application instantiates, staying symbolic while declaring
        let declaring = self.is_declaring();
        let declared = self.definition(instance.symbol)?;
        let definition = match declared.as_deref() {
            Some(definition) => definition,
            None if declaring => return Ok(()),
            None => {
                return Err(CompilerError::Internal {
                    message: format!(
                        "heritage application {:?} has no checked definition",
                        instance.symbol
                    ),
                });
            }
        };

        // read the bases and implemented interfaces this declaration inherits from
        let heritages = definition.heritage_edges();

        // apply this instance's arguments to every edge below
        let substitution =
            self.qualified_instance_substitution(instance_module, instance, receiver)?;

        // walk direct heritage edges with applied arguments
        for heritage in heritages {
            let ty = self.substitute_type(heritage.ty, &substitution)?;
            let (application_module, instance) = self.nominal_application(ty)?;
            let application = HeritageApplication {
                source: branch_source.unwrap_or(heritage.source),
                ty,
                implementer,
            };

            // report cycles at the branch that exposed them
            if active.contains(&instance.symbol) {
                closure.cycles.push(HeritageCycle {
                    source: application.source,
                });
                continue;
            }

            // require duplicate applications to use the same arguments
            if let Some(previous) = closure.application(instance.symbol, self)? {
                if independent {
                    continue;
                }

                let (previous_module, previous_instance) = self.nominal_application(previous.ty)?;
                let arguments = self.constrain_instance_arguments(
                    origin,
                    previous_module,
                    &previous_instance,
                    application_module,
                    &instance,
                )?;
                if arguments == Verdict::Fails {
                    closure.conflicts.push(HeritageConflict {
                        source: application.source,
                        current: application.ty,
                    });
                }
                continue;
            }

            // recurse through newly reached applications, keeping this at the root receiver
            let next_implementer = match self.definition(instance.symbol)?.as_deref() {
                Some(dir::Definition::Interface(_)) => implementer,
                _ => ty,
            };
            closure.applications.push(application.clone());
            active.push(instance.symbol);
            self.collect_heritage(
                origin,
                receiver,
                application_module,
                &instance,
                next_implementer,
                Some(application.source),
                independent,
                active,
                closure,
            )?;
            active.pop();
        }

        Ok(())
    }

    /// Return whether one declaration's heritage reaches a target declaration.
    pub(in crate::sema) fn reaches_heritage(
        &mut self,
        symbol: dir::GlobalSymbolId,
        target: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        // decide the reach once per declaration
        if !self.heritages.contains_key(&symbol) {
            let reach = self.collect_heritage_reach(symbol)?;
            self.heritages.insert(symbol, reach);
        }

        Ok(match &self.heritages[&symbol] {
            HeritageReach::Closed(symbols) => symbols.contains(&target),
            HeritageReach::Open => true,
        })
    }

    /// Collect the declarations one declaration's heritage reaches, walking its edges.
    fn collect_heritage_reach(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<HeritageReach> {
        // seed the walk with the declaration itself
        let mut reached = SmallVec::<[dir::GlobalSymbolId; 8]>::new();
        let mut frontier = SmallVec::<[dir::GlobalSymbolId; 8]>::new();
        reached.push(symbol);
        frontier.push(symbol);

        // walk the frontier of reached declarations
        while let Some(current) = frontier.pop() {
            // an unchecked or aliased declaration leaves the heritage open
            let Some(definition) = self.definition(current)? else {
                return Ok(HeritageReach::Open);
            };
            if matches!(*definition, dir::Definition::TypeAlias(_)) {
                return Ok(HeritageReach::Open);
            }

            // read the bases and implemented interfaces this declaration inherits from
            let edges = definition.heritage_edges();

            // an edge written over a parameter or an alias leaves the heritage open
            for edge in edges {
                let Some((_, instance)) = self.nominal_application_maybe(edge.ty)? else {
                    return Ok(HeritageReach::Open);
                };

                // queue each newly reached declaration
                if !reached.contains(&instance.symbol) {
                    reached.push(instance.symbol);
                    frontier.push(instance.symbol);
                }
            }
        }

        Ok(HeritageReach::Closed(reached))
    }

    /// Select the application of one declaration a source application is or inherits.
    pub(in crate::sema) fn heritage_application(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        source_instance: &dir::GenericApplication,
        target: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<(ModuleId, dir::GenericApplication)>> {
        // accept the source naming the declaration
        if source_instance.symbol == target {
            return Ok(Some((source.module_id, *source_instance)));
        }

        // the source's heritage reaches the declaration
        match self.declared_conformance(origin, source, source, target)? {
            Some(conformance) => Ok(Some(self.nominal_application(conformance.interface)?)),
            None => Ok(None),
        }
    }

    /// Find every application of one interface in a type's heritage closure.
    pub(in crate::sema) fn heritage_applications(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalSymbolId,
    ) -> CompilerResult<SmallVec<[(ModuleId, dir::GenericApplication); 2]>> {
        // read the nominal the source names beneath its handle forms
        let source = self.ownership_payload(origin, source)?;
        let dir::Type::Application(source_instance) = self.ty(source)? else {
            return Ok(SmallVec::new());
        };

        // accept the source naming the declaration
        if source_instance.symbol == target {
            return Ok(SmallVec::from_elem((source.module_id, source_instance), 1));
        }

        // collect each application of the declaration the source's heritage names
        let root = self.heritage_root(origin, source)?;
        if self.nominal_application_maybe(root)?.is_none() {
            return Ok(SmallVec::new());
        }
        let closure = self.instance_heritage_closure(origin, source, root)?;
        let mut found = SmallVec::new();
        for application in &closure.applications {
            let (module, instance) = self.nominal_application(application.ty)?;
            if instance.symbol == target {
                found.push((module, instance));
            }
        }

        Ok(found)
    }

    /// Find the application of one interface in a type's heritage closure.
    pub(in crate::sema) fn declared_conformance(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        receiver: dir::GlobalTypeId,
        interface: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<DeclaredConformance>> {
        let ty = self.heritage_root(origin, ty)?;
        if self.nominal_application_maybe(ty)?.is_none() {
            return Ok(None);
        }
        let closure = self.instance_heritage_closure(origin, receiver, ty)?;
        let conformance =
            closure
                .application(interface, self)?
                .map(|application| DeclaredConformance {
                    implementer: application.implementer,
                    interface: application.ty,
                });

        Ok(conformance)
    }

    /// Relate arguments of two same-template applications.
    pub(in crate::sema) fn constrain_instance_arguments(
        &mut self,
        origin: Origin,
        source_module: ModuleId,
        source: &dir::GenericApplication,
        target_module: ModuleId,
        target: &dir::GenericApplication,
    ) -> CompilerResult<Verdict> {
        // complete the elided arguments of written applications
        let mut source = *source;
        let mut target = *target;
        let mut source_module = source_module;
        let mut target_module = target_module;
        if source.arguments.len() != target.arguments.len() {
            if let Some(filled) = self.fill_elided_application(source_module, &source)? {
                let dir::Type::Application(filled) = self.ty(filled)? else {
                    unreachable!("filled application is not an application");
                };
                source = filled;
                source_module = self.module_id;
            }

            if let Some(filled) = self.fill_elided_application(target_module, &target)? {
                let dir::Type::Application(filled) = self.ty(filled)? else {
                    unreachable!("filled application is not an application");
                };
                target = filled;
                target_module = self.module_id;
            }
        }

        // require both applications to carry the same arity
        if source.arguments.len() != target.arguments.len() {
            return Ok(Verdict::Fails);
        }

        // pair the arguments positionally
        let source_arguments = self.type_ids(source_module, source.arguments)?;
        let pairs = source_arguments
            .iter()
            .copied()
            .zip(
                self.type_ids(target_module, target.arguments)?
                    .iter()
                    .copied(),
            )
            .collect::<SmallVec<[_; 4]>>();

        // erase extent arguments from instance identity, Verify enforces them
        let lifetimes = match self.symbol_template(source.symbol)? {
            Some(template) => {
                let mut lifetimes = SmallVec::<[bool; 4]>::new();
                for parameter in self.generic_template_parameters(template)? {
                    lifetimes.push(self.is_lifetime_parameter(parameter)?);
                }

                lifetimes
            }
            None => SmallVec::new(),
        };

        // require every remaining pair to be equal
        let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
        let mut verdict = Verdict::Holds;
        for (index, (source_argument, target_argument)) in pairs.into_iter().enumerate() {
            if lifetimes.get(index).copied().unwrap_or(false) {
                continue;
            }

            let relation = Relation::Equal;
            let decided =
                self.constrain_type(origin, cause, relation, source_argument, target_argument)?;

            // stop at the first failing pair
            verdict = verdict.and(decided);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        Ok(verdict)
    }

    /// Allocate one reference type for a nominal application.
    fn reference_type(
        &mut self,
        _origin: Origin,
        instance_module: ModuleId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // adopt the arguments into this module before interning
        let arguments: SmallVec<[_; 8]> =
            self.type_ids(instance_module, instance.arguments)?.into();
        let arguments = self.intern_type_ids(&arguments)?;
        let instance = dir::GenericApplication {
            symbol: instance.symbol,
            arguments,
        };

        self.intern_type(dir::Type::Application(instance))
    }
}
