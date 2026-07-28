use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{Answer, Cause, CauseKind, CheckState, Dependency, Origin, Relation, answer};
use crate::{CompilerError, CompilerResult};

/// One applied heritage edge in a nominal declaration closure.
#[derive(Debug, Clone)]
pub(in crate::check) struct HeritageApplication {
    /// The source clause that introduced the application.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The applied nominal or interface type.
    pub(in crate::check) ty: dir::GlobalTypeId,
}

/// One duplicate heritage application with incompatible arguments.
#[derive(Debug, Clone)]
pub(in crate::check) struct HeritageConflict {
    /// The source clause that introduced the conflicting application.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The conflicting inherited type.
    pub(in crate::check) current: dir::GlobalTypeId,
}

/// One heritage branch that exposes a cycle.
#[derive(Debug, Clone)]
pub(in crate::check) struct HeritageCycle {
    /// The source branch that exposes the cycle.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
}

/// Complete heritage closure for one nominal or interface application.
#[derive(Debug, Clone, Default)]
pub(in crate::check) struct HeritageClosure {
    /// The inherited applications in traversal order.
    pub(in crate::check) applications: SmallVec<[HeritageApplication; 8]>,
    /// Duplicate applications with different arguments.
    pub(in crate::check) conflicts: SmallVec<[HeritageConflict; 2]>,
    /// Cycles found while walking heritage edges.
    pub(in crate::check) cycles: SmallVec<[HeritageCycle; 2]>,
}

impl HeritageClosure {
    /// Return the first application naming one symbol.
    fn application<'a>(
        &'a self,
        symbol: dir::GlobalSymbolId,
        state: &CheckState<'_>,
    ) -> CompilerResult<Option<&'a HeritageApplication>> {
        for application in &self.applications {
            let (_, instance) = state.require_nominal_application(application.ty)?;
            if instance.symbol == symbol {
                return Ok(Some(application));
            }
        }

        Ok(None)
    }
}

impl CheckState<'_> {
    /// Decide one check-only constraint relation between closed roots.
    pub(in crate::check) fn decide_satisfies(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // memory forms decide like closed form assignability
        if (matches!(self.ty(source)?, dir::Type::Form(_))
            || matches!(self.ty(target)?, dir::Type::Form(_)))
            && let Some(decision) =
                self.constrain_form_assignable_rooted(origin, relation, source, target)?
        {
            return Ok(decision);
        }

        // memory singletons inhabit their stdlib singleton kind
        let memory_kind = match self.ty(source)? {
            dir::Type::Memory(source) => Some(source.kind_language_item()),
            _ => None,
        };
        let target_item = self
            .type_symbol(target)?
            .map(|symbol| self.language_item(symbol))
            .transpose()?
            .flatten();
        if let Some(memory_kind) = memory_kind
            && target_item == Some(memory_kind)
        {
            return Ok(Answer::Ready(true));
        }

        // enum members satisfy constraints through their owner
        if let dir::Type::Variant(member) = self.ty(source)? {
            return self.decide_relation(origin, relation, member.owner, target);
        }

        // generic parameters prove relations through their active bounds
        if let dir::Type::Parameter(parameter) | dir::Type::Erased(parameter) = self.ty(source)? {
            let decision = self.decide_parameter_relation(origin, relation, parameter, target)?;

            return self.decide_union_membership(origin, relation, decision, source, target);
        }

        // union sources must satisfy the target through every element
        if let dir::Type::Union(union) = self.ty(source)? {
            let elements = self.type_ids(source.module_id, union.elements)?.to_vec();

            return self.decide_all_sources(origin, relation, &elements, target);
        }

        // union targets accept when any element accepts the source
        if let dir::Type::Union(union) = self.ty(target)? {
            let elements = self.type_ids(target.module_id, union.elements)?.to_vec();

            return self.decide_any_target(origin, relation, source, &elements);
        }

        // comptime scalars inhabit closed enums by member value
        if let dir::Type::Literal(literal) = self.ty(source)?
            && let Some(symbol) = self.type_symbol(target)?
            && let Some(dir::Definition::Enum(definition)) = self.definition(symbol)?
        {
            let members = definition.members.clone();
            for member in &members {
                let dir::DefinitionMember::EnumVariant(variant) = member else {
                    continue;
                };
                if dir::ScalarLiteral::from(variant.value) == literal {
                    return Ok(Answer::Ready(true));
                }
            }

            return Ok(Answer::Ready(false));
        }

        // static scalar operations relate through their result type
        if matches!(
            self.operation_head(source)?,
            Some(dir::TypeOperation::StaticBinary(_) | dir::TypeOperation::StaticUnary(_))
        ) && let Some(result) = answer!(self.static_operation_type(origin, source)?)
        {
            let decision = self.decide_relation(origin, relation, result, target)?;
            if !matches!(decision, Answer::Ready(false)) {
                return Ok(decision);
            }
        }

        // intersection targets require every element under the same relation
        if let dir::Type::Intersection(intersection) = self.ty(target)? {
            let elements = self
                .type_ids(target.module_id, intersection.elements)?
                .to_vec();

            return self.decide_all_targets(origin, relation, source, &elements);
        }

        // intersection sources satisfy through any element
        if let dir::Type::Intersection(intersection) = self.ty(source)? {
            let elements = self
                .type_ids(source.module_id, intersection.elements)?
                .to_vec();

            return self.decide_any_source(origin, relation, &elements, target);
        }

        let target_instance = match self.ty(target)? {
            dir::Type::Application(target) => Some(target),
            _ => None,
        };

        // route every applied interface through one implementation selection path
        if let Some(target_instance) = target_instance.as_ref()
            && matches!(
                self.definition(target_instance.symbol)?,
                Some(dir::Definition::Interface(_))
            )
        {
            return self.decide_interface_relation(origin, relation, source, target);
        }

        // nominal sources meet nominal constraints through their declarations
        let instances = match (self.ty(source)?, target_instance) {
            (dir::Type::Application(source), Some(target)) => Some((source, target)),
            _ => None,
        };

        match (instances, relation) {
            (Some((source_instance, target_instance)), _) => self.decide_application_relation(
                origin,
                relation,
                source,
                &source_instance,
                target,
                &target_instance,
            ),

            // explicit implements requires the heritage relation
            (None, Relation::Implements) => Ok(Answer::Ready(false)),

            // everything else satisfies through assignability,
            //  or sits inside a union target as a member
            (None, _) => {
                // check-only relations read structural pairs covariantly
                if let (dir::Type::Shape(_), dir::Type::Shape(_)) =
                    (self.ty(source)?, self.ty(target)?)
                {
                    return self.decide_shape_relation(origin, relation, source, target);
                }
                let assignable = self.decide_assignable(origin, relation, source, target)?;

                self.decide_union_membership(origin, relation, assignable, source, target)
            }
        }
    }

    /// Decide one relation between applied declarations.
    pub(in crate::check) fn decide_application_relation(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        source_instance: &dir::GenericApplication,
        target: dir::GlobalTypeId,
        target_instance: &dir::GenericApplication,
    ) -> CompilerResult<Answer<bool>> {
        // interface targets select one implementation path
        if self.symbol_kind(target_instance.symbol).is_interface() {
            return self.decide_interface_relation(origin, relation, source, target);
        }

        // select the source application of the target declaration
        let application = if source_instance.symbol == target_instance.symbol {
            Some((source.module_id, *source_instance))
        } else {
            let heritage = answer!(self.heritage_instance(
                origin,
                source.module_id,
                source_instance,
                target_instance.symbol,
            )?);

            match heritage {
                Some(heritage) => Some(self.require_nominal_application(heritage)?),
                None => None,
            }
        };

        // compare every nominal path through one argument relation
        if let Some((application_module, application)) = application {
            let source_arguments = self
                .type_ids(application_module, application.arguments)?
                .to_vec();
            let target_arguments = self
                .type_ids(target.module_id, target_instance.arguments)?
                .to_vec();
            let form = self.default_variance_form(target_instance.symbol);

            let arguments = if relation == Relation::Subtype {
                self.decide_type_arguments(
                    origin,
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

        Ok(Answer::Ready(false))
    }

    /// Decide assignability between different nominal applications.
    pub(in crate::check) fn decide_application_assignable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let (source_instance, target_instance) = match (self.ty(source)?, self.ty(target)?) {
            (dir::Type::Application(source), dir::Type::Application(target)) => (source, target),
            _ => return Ok(Answer::Ready(false)),
        };

        self.decide_application_relation(
            origin,
            Relation::Assignable,
            source,
            &source_instance,
            target,
            &target_instance,
        )
    }

    /// Return the first declared field missing from one struct construction.
    pub(in crate::check) fn first_missing_struct_field(
        &mut self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        let source_fields = match self.ty(source)? {
            dir::Type::Shape(shape) => self
                .shape_properties(source.module_id, shape.properties)?
                .iter()
                .map(|field| field.key)
                .collect::<SmallVec<[_; 8]>>(),
            _ => return Ok(None),
        };

        let dir::Type::Application(target_instance) = self.ty(target)? else {
            return Ok(None);
        };
        let Some(dir::Definition::Struct(_)) = self.definition(target_instance.symbol)? else {
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
    pub(in crate::check) fn first_excess_struct_field(
        &mut self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        let source_fields = match self.ty(source)? {
            dir::Type::Shape(shape) => self
                .shape_properties(source.module_id, shape.properties)?
                .iter()
                .map(|field| field.key)
                .collect::<SmallVec<[_; 8]>>(),
            _ => return Ok(None),
        };

        let dir::Type::Application(target_instance) = self.ty(target)? else {
            return Ok(None);
        };
        let Some(dir::Definition::Struct(_)) = self.definition(target_instance.symbol)? else {
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
    pub(in crate::check) fn struct_fields(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<SmallVec<[dir::TypeProperty; 8]>>> {
        let receiver = answer!(self.reduce_type_head(origin, target)?);
        let chain = self.form_chain(origin, receiver)?;
        let target = chain.base();
        let dir::Type::Application(instance) = self.ty(target)? else {
            return Ok(Answer::Ready(SmallVec::new()));
        };

        self.instantiate_struct_fields(origin, receiver, target.module_id, &instance)
    }

    /// Return substituted fields accepted by one struct constructor.
    pub(in crate::check) fn struct_constructor_fields(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<SmallVec<[dir::TypeProperty; 8]>>> {
        let mut fields = answer!(self.struct_fields(origin, target)?);
        let receiver = answer!(self.reduce_type_head(origin, target)?);
        let chain = self.form_chain(origin, receiver)?;
        let target = chain.base();
        let dir::Type::Application(instance) = self.ty(target)? else {
            return Ok(Answer::Ready(fields));
        };
        let Some(dir::Definition::Struct(definition)) = self.definition(instance.symbol)? else {
            return Ok(Answer::Ready(fields));
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

        Ok(Answer::Ready(fields))
    }

    /// Return substituted direct fields for one struct instance.
    fn instantiate_struct_fields(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        instance_module: ModuleId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<Answer<SmallVec<[dir::TypeProperty; 8]>>> {
        let Some(dir::Definition::Struct(definition)) = self.definition(instance.symbol)?.cloned()
        else {
            return Ok(Answer::Ready(SmallVec::new()));
        };
        let substitution = self
            .instance_substitution(instance_module, instance)?
            .with_receiver(receiver);
        let mut fields = SmallVec::new();

        // collect direct instance fields through the selected target
        for member in definition.members {
            let dir::DefinitionMember::Field(field) = member else {
                continue;
            };
            if field.space != dir::MemberSpace::Instance {
                continue;
            }
            let Some(ty) = self.symbol_type_maybe(field.symbol) else {
                return Ok(Answer::pending([Dependency::SymbolType(field.symbol)]));
            };
            let ty = self.substitute_type(origin.module(), ty, &substitution)?;
            fields.push(dir::TypeProperty {
                key: field.key,
                access: dir::PropertyAccess::ReadWrite {
                    read: ty,
                    write: ty,
                },
                is_optional: field.is_optional,
            });
        }

        Ok(Answer::Ready(fields))
    }

    /// Bound open construction arguments from written literal fields.
    pub(in crate::check) fn constrain_struct_construction(
        &mut self,
        origin: Origin,
        source_fields: &[dir::TypeProperty],
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        let declared_fields = answer!(self.struct_fields(origin, target)?);
        for declared in declared_fields {
            let Some(field) = source_fields.iter().find(|field| field.key == declared.key) else {
                continue;
            };

            let field_cause =
                self.intern_cause(Cause::root(origin, CauseKind::Field { key: field.key }));
            answer!(self.constrain_type(
                origin,
                field_cause,
                Relation::Assignable,
                field.access.store(),
                declared.access.store()
            )?);
        }

        Ok(Answer::Ready(()))
    }

    /// Collect one definition's instance field keys through heritage.
    pub(in crate::check) fn nominal_field_keys(
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
    pub(in crate::check) fn nominal_instance_fields(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<SmallVec<[(dir::StaticKey, bool); 8]>> {
        let mut keys = SmallVec::new();
        let mut pending = SmallVec::<[dir::GlobalSymbolId; 4]>::new();
        let mut visited = SmallVec::<[dir::GlobalSymbolId; 4]>::new();
        pending.push(symbol);

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
            for member in members {
                if let dir::DefinitionMember::Field(field) = member
                    && field.space == dir::MemberSpace::Instance
                {
                    let is_required = !field.is_optional && field.initializer.is_none();
                    keys.push((field.key, is_required));
                }
            }
            for heritage in bases {
                let (_, base) = self.require_nominal_application(heritage)?;
                pending.push(base.symbol);
            }
        }

        Ok(keys)
    }

    /// Collect one definition's member keys through its heritage chain.
    pub(in crate::check) fn nominal_member_keys(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<SmallVec<[dir::StaticKey; 8]>> {
        let mut keys = SmallVec::new();
        let mut pending = SmallVec::<[dir::GlobalSymbolId; 4]>::new();
        let mut visited = SmallVec::<[dir::GlobalSymbolId; 4]>::new();
        pending.push(symbol);

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
            for member in members {
                if let Some(key) = member.key() {
                    keys.push(key);
                }
            }
            for heritage in bases {
                let (_, base) = self.require_nominal_application(heritage)?;
                pending.push(base.symbol);
            }
        }

        Ok(keys)
    }

    /// Decide whether one reference source satisfies one structural shape target.
    pub(in crate::check) fn decide_reference_against_target(
        &mut self,
        origin: Origin,
        relation: Relation,
        source_module: ModuleId,
        source_instance: &dir::GenericApplication,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // require each target field from the source fields
        let (fields, index_signatures) = match self.ty(target)? {
            dir::Type::Shape(shape) => (
                self.shape_properties(target.module_id, shape.properties)?
                    .iter()
                    .map(|field| (field.key, field.access.store(), field.is_optional))
                    .collect::<SmallVec<[_; 4]>>(),
                self.shape_index_signatures(target.module_id, shape.index_signatures)?
                    .to_vec(),
            ),
            _ => return Ok(Answer::Ready(false)),
        };
        let module = origin.module();
        let source = self.reference_type(origin, source_module, source_instance)?;
        let mut decision = Answer::Ready(true);
        for (key, field_type, is_optional) in fields {
            let lookup = answer!(self.body().lookup_member(
                origin,
                module,
                source,
                dir::MemberSpace::Instance,
                key
            )?);

            let member = self.body().member_read_type(origin, &lookup)?;

            match member {
                // missing members satisfy optional targets only
                None => {
                    if !is_optional {
                        return Ok(Answer::Ready(false));
                    }
                }
                Some(member) => {
                    decision =
                        decision.and(self.decide_relation(origin, relation, member, field_type)?);
                    if decision.is_ready_false() {
                        return Ok(decision);
                    }
                }
            }
        }
        for signature in index_signatures {
            decision = decision
                .and(self.decide_index_signature_satisfied(origin, relation, source, &signature)?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Return the full heritage closure for one nominal application.
    pub(in crate::check) fn heritage_closure(
        &mut self,
        origin: Origin,
        instance_module: ModuleId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<Answer<HeritageClosure>> {
        let mut closure = HeritageClosure::default();
        let mut active = SmallVec::<[dir::GlobalSymbolId; 8]>::new();
        let mut blockers = SmallVec::<[Dependency; 2]>::new();

        // extensions implement each application independently, so
        //  same-symbol instantiations dispatch by form instead of conflicting
        let independent = matches!(
            self.definition(instance.symbol)?,
            Some(dir::Definition::Extension(_))
        );

        active.push(instance.symbol);
        self.collect_heritage(
            origin,
            instance_module,
            instance,
            None,
            independent,
            &mut active,
            &mut closure,
            &mut blockers,
        )?;

        Ok(Answer::ready_unless_blocked(closure, blockers))
    }

    /// Collect inherited applications from one nominal application.
    fn collect_heritage(
        &mut self,
        origin: Origin,
        instance_module: ModuleId,
        instance: &dir::GenericApplication,
        branch_source: Option<dir::GlobalNodeIdAny>,
        independent: bool,
        active: &mut SmallVec<[dir::GlobalSymbolId; 8]>,
        closure: &mut HeritageClosure,
        blockers: &mut SmallVec<[Dependency; 2]>,
    ) -> CompilerResult<()> {
        let definition =
            self.definition(instance.symbol)?
                .ok_or_else(|| CompilerError::Internal {
                    message: format!(
                        "heritage application {:?} has no checked definition",
                        instance.symbol
                    ),
                })?;
        let heritages = definition
            .heritages()
            .iter()
            .map(|heritage| (*heritage).clone())
            .collect::<SmallVec<[_; 2]>>();
        let substitution = self.instance_substitution(instance_module, instance)?;
        let module = origin.module();

        // walk direct heritage edges with applied arguments
        for heritage in heritages {
            let ty = self.substitute_type(module, heritage.ty, &substitution)?;
            let (application_module, instance) = self.require_nominal_application(ty)?;
            let application = HeritageApplication {
                source: branch_source.unwrap_or(heritage.source),
                ty,
            };

            // cycles are reported at the branch that exposed the cycle
            if active.contains(&instance.symbol) {
                closure.cycles.push(HeritageCycle {
                    source: application.source,
                });
                continue;
            }

            // duplicate applications must use the same arguments,
            //  except under declarations that implement each independently
            if let Some(previous) = closure.application(instance.symbol, self)? {
                if independent {
                    continue;
                }
                let (previous_module, previous_instance) =
                    self.require_nominal_application(previous.ty)?;
                match self.constrain_instance_arguments(
                    origin,
                    previous_module,
                    &previous_instance,
                    application_module,
                    &instance,
                )? {
                    Answer::Ready(true) => {}
                    Answer::Ready(false) => closure.conflicts.push(HeritageConflict {
                        source: application.source,
                        current: application.ty,
                    }),
                    Answer::Pending(pending) => blockers.extend(pending),
                }
                continue;
            }

            // recurse through newly reached applications
            closure.applications.push(application.clone());
            active.push(instance.symbol);
            self.collect_heritage(
                origin,
                application_module,
                &instance,
                Some(application.source),
                independent,
                active,
                closure,
                blockers,
            )?;
            active.pop();
        }

        Ok(())
    }

    /// Find one heritage application naming a target symbol, transitively.
    pub(in crate::check) fn heritage_instance(
        &mut self,
        origin: Origin,
        instance_module: ModuleId,
        instance: &dir::GenericApplication,
        target: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let closure = answer!(self.heritage_closure(origin, instance_module, instance)?);
        if let Some(application) = closure.application(target, self)? {
            return Ok(Answer::Ready(Some(application.ty)));
        }

        Ok(Answer::Ready(None))
    }

    /// Relate arguments of two same-template applications.
    pub(in crate::check) fn constrain_instance_arguments(
        &mut self,
        origin: Origin,
        source_module: ModuleId,
        source: &dir::GenericApplication,
        target_module: ModuleId,
        target: &dir::GenericApplication,
    ) -> CompilerResult<Answer<bool>> {
        if source.arguments.len() != target.arguments.len() {
            return Ok(Answer::Ready(false));
        }

        let pairs = self
            .type_ids(source_module, source.arguments)?
            .iter()
            .copied()
            .zip(
                self.type_ids(target_module, target.arguments)?
                    .iter()
                    .copied(),
            )
            .collect::<SmallVec<[_; 4]>>();

        let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
        let mut decision = Answer::Ready(true);
        for (source_argument, target_argument) in pairs {
            decision = decision.and(self.constrain_type(
                origin,
                cause,
                Relation::Equal,
                source_argument,
                target_argument,
            )?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Allocate one reference type for a nominal application.
    fn reference_type(
        &mut self,
        origin: Origin,
        instance_module: ModuleId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let arguments = self.type_ids(instance_module, instance.arguments)?.to_vec();
        let arguments = self.intern_type_ids(origin.module(), &arguments)?;
        let instance = dir::GenericApplication {
            symbol: instance.symbol,
            arguments,
        };

        self.intern_type(origin.module(), dir::Type::Application(instance))
    }
}
