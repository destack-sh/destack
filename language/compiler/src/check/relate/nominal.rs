use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Dependency, Origin, Relation, answer};

/// One applied heritage edge in a nominal declaration closure.
#[derive(Debug, Clone)]
pub(in crate::check) struct HeritageApplication {
    /// The source clause that introduced the application.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The applied nominal or interface instance.
    pub(in crate::check) instance: dir::GenericInstance,
}

/// One duplicate heritage application with incompatible arguments.
#[derive(Debug, Clone)]
pub(in crate::check) struct HeritageConflict {
    /// The source clause that introduced the conflicting application.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The conflicting inherited application.
    pub(in crate::check) current: dir::GenericInstance,
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
    fn application(&self, symbol: dir::GlobalSymbolId) -> Option<&HeritageApplication> {
        self.applications
            .iter()
            .find(|application| application.instance.symbol == symbol)
    }
}

impl CheckState<'_> {
    /// Return the substituted backing type for one newtype instance.
    /// `instance_module` is the owner of the instance's argument list.
    pub(in crate::check) fn newtype_backing_type(
        &mut self,
        origin: Origin,
        instance_module: destack_source::ModuleId,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(dir::Definition::Newtype(definition)) = self.definition(instance.symbol) else {
            return Ok(None);
        };
        let backing = definition.value;
        let substitution = self.instance_substitution(instance_module, instance)?;
        let backing = self.substitute_type(origin.module(), backing, &substitution)?;

        Ok(Some(backing))
    }

    /// Re-root one instance's argument list into the origin module.
    /// Nominal decisions pass instances around freely, so one uniform
    /// list owner keeps the heritage machinery resolvable everywhere.
    pub(in crate::check) fn origin_instance(
        &mut self,
        origin: Origin,
        instance_module: destack_source::ModuleId,
        instance: dir::GenericInstance,
    ) -> CompilerResult<dir::GenericInstance> {
        let module = origin.module();
        if instance_module == module || instance.arguments.is_empty() {
            return Ok(instance);
        }

        let arguments = SmallVec::<[dir::GlobalTypeId; 4]>::from_slice(
            self.type_ids(instance_module, instance.arguments)?,
        );
        let arguments = self.intern_type_ids(module, &arguments)?;

        Ok(dir::GenericInstance {
            symbol: instance.symbol,
            arguments,
        })
    }

    /// Decide one check-only constraint relation between closed roots.
    pub(in crate::check) fn decide_satisfies(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        if self.decide_equal(origin, source, target)?.is_ready_true() {
            return Ok(Answer::Ready(true));
        }

        // memory singletons inhabit their stdlib singleton kind
        let memory_kind = match self.ty(source)? {
            dir::Type::Memory(source) => Some(source.domain_language_item()),
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
        if let dir::Type::EnumMember(member) = self.ty(source)? {
            return self.decide_relation(origin, relation, member.owner, target);
        }

        // generic parameters prove relations through their active bounds
        if let dir::Type::Parameter(parameter) | dir::Type::Erased(parameter) = self.ty(source)? {
            let decision = self.decide_parameter_relation(origin, relation, parameter, target)?;

            return self.decide_union_membership(origin, relation, decision, source, target);
        }

        // comptime scalars inhabit closed enums by member value
        if let dir::Type::Literal(literal) = self.ty(source)?
            && let Some(symbol) = self.type_symbol(target)?
            && let Some(dir::Definition::Enum(definition)) = self.definition(symbol)
        {
            let members = definition.members.clone();
            for member in &members {
                let dir::DefinitionMember::Variant(variant) = member else {
                    continue;
                };
                let value = self.static_value(variant.symbol);
                let Some(value) = value else {
                    continue;
                };
                if self.ty(value)? == dir::Type::Literal(literal) {
                    return Ok(Answer::Ready(true));
                }
            }

            return Ok(Answer::Ready(false));
        }

        // static scalar operations relate through their result type
        if matches!(
            self.ty(source)?,
            dir::Type::Operation(
                dir::TypeOperation::StaticBinary(_) | dir::TypeOperation::StaticUnary(_)
            )
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
            let mut decision = Answer::Ready(true);
            for element in elements {
                decision = decision.and(self.decide_relation(origin, relation, source, element)?);
                if decision.is_ready_false() {
                    break;
                }
            }

            return Ok(decision);
        }

        // intersection sources satisfy through any element
        if let dir::Type::Intersection(intersection) = self.ty(source)? {
            let elements = self
                .type_ids(source.module_id, intersection.elements)?
                .to_vec();
            let mut decision = Answer::Ready(false);
            for element in elements {
                decision = decision.or(self.decide_relation(origin, relation, element, target)?);
                if decision.is_ready_true() {
                    break;
                }
            }

            return Ok(decision);
        }

        // nominal sources meet nominal constraints through their declarations
        let instances = match (self.ty(source)?, self.ty(target)?) {
            (dir::Type::Instance(source), dir::Type::Instance(target)) => Some((source, target)),
            _ => None,
        };
        let target_instance = match self.ty(target)? {
            dir::Type::Instance(target) => Some(target),
            _ => None,
        };

        match (instances, relation) {
            (Some((source_instance, target_instance)), _) => self.decide_nominal_satisfies(
                origin,
                source,
                &source_instance,
                target,
                &target_instance,
            ),

            // auto interfaces decide by their derivation rules
            (None, _)
                if let Some(interface) = target_instance
                    .as_ref()
                    .and_then(|instance| self.language_item(instance.symbol).ok().flatten())
                    .and_then(dir::AutoInterface::from_language_item)
                    .filter(|interface| interface.has_auto_conformance()) =>
            {
                self.satisfies_auto_interface(origin, source, interface)
            }

            // check extension implementations over any receiver form
            (None, _) if self.is_interface_instance(target_instance.as_ref()) => {
                let Some(target_instance) = target_instance.as_ref() else {
                    return Ok(Answer::Ready(false));
                };
                let module = origin.module();
                let implemented = self.decide_extension_implementation(
                    origin,
                    module,
                    target.module_id,
                    source,
                    target_instance,
                )?;
                if !matches!(implemented, Answer::Ready(false)) {
                    return Ok(implemented);
                }

                self.decide_assignable(origin, source, target)
            }

            // explicit implements requires the heritage relation
            (None, Relation::Implements) => Ok(Answer::Ready(false)),

            // everything else satisfies through assignability,
            // or sits inside a union target as a member
            (None, _) => {
                let assignable = self.decide_assignable(origin, source, target)?;

                self.decide_union_membership(origin, relation, assignable, source, target)
            }
        }
    }

    /// Decide whether one nominal application satisfies another.
    fn decide_nominal_satisfies(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        source_instance: &dir::GenericInstance,
        target: dir::GlobalTypeId,
        target_instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<bool>> {
        // same-symbol interface applications compare by declared variance
        if source_instance.symbol == target_instance.symbol {
            if matches!(
                self.definition(target_instance.symbol),
                Some(dir::Definition::Interface(_))
            ) {
                let source_arguments = self
                    .type_ids(source.module_id, source_instance.arguments)?
                    .to_vec();
                let target_arguments = self
                    .type_ids(target.module_id, target_instance.arguments)?
                    .to_vec();

                return self.relate_type_arguments(
                    origin,
                    target_instance.symbol,
                    &source_arguments,
                    &target_arguments,
                );
            }

            return self.constrain_instance_arguments(
                origin,
                source.module_id,
                source_instance,
                target.module_id,
                target_instance,
            );
        }

        // heritage carries the relation when it names the target;
        // heritage argument list always land in origin.module()
        if let Some(heritage) = answer!(self.heritage_instance(
            origin,
            source.module_id,
            source_instance,
            target_instance.symbol
        )?) {
            let is_interface = matches!(
                self.definition(target_instance.symbol),
                Some(dir::Definition::Interface(_))
            );
            let arguments = if is_interface {
                let heritage_arguments =
                    self.type_ids(origin.module(), heritage.arguments)?.to_vec();
                let target_arguments = self
                    .type_ids(target.module_id, target_instance.arguments)?
                    .to_vec();

                self.relate_type_arguments(
                    origin,
                    target_instance.symbol,
                    &heritage_arguments,
                    &target_arguments,
                )?
            } else {
                self.constrain_instance_arguments(
                    origin,
                    origin.module(),
                    &heritage,
                    target.module_id,
                    target_instance,
                )?
            };
            if !is_interface {
                return Ok(arguments);
            }

            let members =
                self.decide_interface_satisfied(origin, source, target.module_id, target_instance)?;

            return Ok(arguments.and(members));
        }

        // check visible extension implementations
        if matches!(
            self.definition(target_instance.symbol),
            Some(dir::Definition::Interface(_))
        ) {
            let module = origin.module();
            let implemented = self.decide_extension_implementation(
                origin,
                module,
                target.module_id,
                source,
                target_instance,
            )?;
            if !matches!(implemented, Answer::Ready(false)) {
                return Ok(implemented);
            }
        }

        // structural interfaces satisfy member-wise
        let target_definition = self.definition(target_instance.symbol);
        let is_structural_interface = matches!(
            target_definition,
            Some(dir::Definition::Interface(interface)) if !interface.is_nominal
        );
        if is_structural_interface {
            return self.decide_interface_satisfied(
                origin,
                source,
                target.module_id,
                target_instance,
            );
        }

        Ok(Answer::Ready(false))
    }

    /// Return whether one instance target names an interface.
    fn is_interface_instance(&self, instance: Option<&dir::GenericInstance>) -> bool {
        let Some(instance) = instance else {
            return false;
        };

        matches!(
            self.definition(instance.symbol),
            Some(dir::Definition::Interface(_))
        )
    }

    /// Decide assignability between different nominal applications.
    pub(in crate::check) fn decide_nominal_assignable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let (source_instance, target_instance) = match (self.ty(source)?, self.ty(target)?) {
            (dir::Type::Instance(source), dir::Type::Instance(target)) => (source, target),
            _ => return Ok(Answer::Ready(false)),
        };

        self.decide_nominal_satisfies(origin, source, &source_instance, target, &target_instance)
    }

    /// Decide whether one structural shape satisfies one reference target.
    /// `target_module` is the owner of `target_instance`'s argument list.
    pub(in crate::check) fn decide_source_against_reference(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target_module: destack_source::ModuleId,
        target_instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<bool>> {
        // shapes satisfy structural interfaces member-wise
        let is_structural_interface = matches!(
            self.definition(target_instance.symbol),
            Some(dir::Definition::Interface(interface)) if !interface.is_nominal
        );
        if is_structural_interface {
            return self.decide_interface_satisfied(origin, source, target_module, target_instance);
        }

        Ok(Answer::Ready(false))
    }

    /// Decide whether one literal shape constructs one struct.
    ///
    /// Construction writes every declared instance field; methods and
    /// associated members never come from literals.
    pub(in crate::check) fn decide_struct_construction(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        target_instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<bool>> {
        // collect the written literal fields
        let written = match self.ty(source)? {
            dir::Type::Shape(shape) => self
                .shape_fields(source.module_id, shape.fields)?
                .iter()
                .map(|field| (field.key, field.ty))
                .collect::<SmallVec<[_; 8]>>(),
            _ => return Ok(Answer::Ready(false)),
        };

        // require every declared instance field from the literal
        let module = origin.module();
        let mut decision = Answer::Ready(true);
        for (key, has_initializer) in self.nominal_instance_fields(target_instance.symbol) {
            let lookup = answer!(self.lookup_member(
                origin,
                module,
                target,
                dir::MemberSpace::Instance,
                key
            )?);
            let declared = match lookup.field_type() {
                Some(ty) => ty,
                None => continue,
            };

            // omitted fields fill from their initializers or undefined
            let supplied = written.iter().find(|(written, _)| *written == key);
            let Some((_, supplied)) = supplied else {
                if answer!(self.field_may_be_omitted(origin, declared, has_initializer)?) {
                    continue;
                }

                return Ok(Answer::Ready(false));
            };

            // written fields must hold an assignable type
            decision = decision.and(self.decide_relation(
                origin,
                Relation::Assignable,
                *supplied,
                declared,
            )?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Return the first declared field missing from one struct construction.
    pub(in crate::check) fn first_missing_struct_field(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        let source_fields = match self.ty(source)? {
            dir::Type::Shape(shape) => self
                .shape_fields(source.module_id, shape.fields)?
                .iter()
                .map(|field| field.key)
                .collect::<SmallVec<[_; 8]>>(),
            _ => return Ok(None),
        };

        let dir::Type::Instance(target_instance) = self.ty(target)? else {
            return Ok(None);
        };
        let Some(dir::Definition::Struct(_)) = self.definition(target_instance.symbol) else {
            return Ok(None);
        };

        // scan declared fields in construction order
        let module = origin.module();
        for (key, has_initializer) in self.nominal_instance_fields(target_instance.symbol) {
            if source_fields.contains(&key) {
                continue;
            }

            let lookup = match self.lookup_member(
                origin,
                module,
                target,
                dir::MemberSpace::Instance,
                key,
            )? {
                Answer::Ready(lookup) => lookup,
                Answer::Pending(_) => return Ok(None),
            };
            let Some(declared) = lookup.field_type() else {
                continue;
            };

            let may_omit = match self.field_may_be_omitted(origin, declared, has_initializer)? {
                Answer::Ready(may_omit) => may_omit,
                Answer::Pending(_) => return Ok(None),
            };
            if may_omit {
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
                .shape_fields(source.module_id, shape.fields)?
                .iter()
                .map(|field| field.key)
                .collect::<SmallVec<[_; 8]>>(),
            _ => return Ok(None),
        };

        let dir::Type::Instance(target_instance) = self.ty(target)? else {
            return Ok(None);
        };
        let Some(dir::Definition::Struct(_)) = self.definition(target_instance.symbol) else {
            return Ok(None);
        };

        // compare against declared construction fields
        let fields = self.nominal_field_keys(target_instance.symbol);
        for key in source_fields {
            if !fields.contains(&key) {
                return Ok(Some(key));
            }
        }

        Ok(None)
    }

    /// Return whether one literal may omit one declared field.
    ///
    /// Fields with declared initializers fill themselves, and optional
    /// fields admit their absence as undefined.
    pub(in crate::check) fn field_may_be_omitted(
        &mut self,
        origin: Origin,
        declared: dir::GlobalTypeId,
        has_initializer: bool,
    ) -> CompilerResult<Answer<bool>> {
        if has_initializer {
            return Ok(Answer::Ready(true));
        }
        let undefined = self.intern_type(origin.module(), dir::Type::Undefined)?;

        self.decide_relation(origin, Relation::Assignable, undefined, declared)
    }

    /// Bound open construction arguments from written literal fields.
    ///
    /// The writable relation reports the construction afterwards, so
    /// this only pushes bounds.
    pub(in crate::check) fn constrain_struct_construction(
        &mut self,
        origin: Origin,
        source_fields: &[dir::TypeField],
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        let target = answer!(self.reduce_type_head(origin, target)?);
        let dir::Type::Instance(instance) = self.ty(target)? else {
            return Ok(Answer::Ready(()));
        };

        let module = origin.module();
        for key in self.nominal_field_keys(instance.symbol) {
            let lookup = answer!(self.lookup_member(
                origin,
                module,
                target,
                dir::MemberSpace::Instance,
                key
            )?);
            let Some(declared) = lookup.field_type() else {
                continue;
            };
            let Some(field) = source_fields.iter().find(|field| field.key == key) else {
                continue;
            };

            answer!(self.constrain(origin, Relation::Assignable, field.ty, declared)?);
        }

        Ok(Answer::Ready(()))
    }

    /// Collect one definition's instance field keys through heritage.
    pub(in crate::check) fn nominal_field_keys(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> SmallVec<[dir::StaticKey; 8]> {
        self.nominal_instance_fields(symbol)
            .into_iter()
            .map(|(key, _)| key)
            .collect()
    }

    /// Collect one definition's instance fields with their initializer
    /// presence through heritage.
    pub(in crate::check) fn nominal_instance_fields(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> SmallVec<[(dir::StaticKey, bool); 8]> {
        let mut keys = SmallVec::new();
        let mut pending = SmallVec::<[dir::GlobalSymbolId; 4]>::new();
        let mut visited = SmallVec::<[dir::GlobalSymbolId; 4]>::new();
        pending.push(symbol);

        while let Some(symbol) = pending.pop() {
            if visited.contains(&symbol) {
                continue;
            }
            visited.push(symbol);

            let Some(definition) = self.definition(symbol) else {
                continue;
            };
            for member in definition.members() {
                if let dir::DefinitionMember::Field(field) = member
                    && field.space == dir::MemberSpace::Instance
                {
                    keys.push((field.key, field.initializer.is_some()));
                }
            }
            for heritage in definition.bases() {
                pending.push(heritage.symbol);
            }
        }

        keys
    }

    /// Collect one definition's member keys through its heritage chain.
    pub(in crate::check) fn nominal_member_keys(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> SmallVec<[dir::StaticKey; 8]> {
        let mut keys = SmallVec::new();
        let mut pending = SmallVec::<[dir::GlobalSymbolId; 4]>::new();
        let mut visited = SmallVec::<[dir::GlobalSymbolId; 4]>::new();
        pending.push(symbol);

        while let Some(symbol) = pending.pop() {
            if visited.contains(&symbol) {
                continue;
            }
            visited.push(symbol);

            let Some(definition) = self.definition(symbol) else {
                continue;
            };
            for member in definition.members() {
                if let Some(key) = member.key() {
                    keys.push(key);
                }
            }
            for heritage in definition.bases() {
                pending.push(heritage.symbol);
            }
        }

        keys
    }

    /// Decide whether one reference source satisfies one structural shape target.
    /// `source_module` is the owner of `source_instance`'s argument list.
    pub(in crate::check) fn decide_reference_against_target(
        &mut self,
        origin: Origin,
        source_module: destack_source::ModuleId,
        source_instance: &dir::GenericInstance,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // require each target field from the source fields
        let (fields, index_signatures) = match self.ty(target)? {
            dir::Type::Shape(shape) => (
                self.shape_fields(target.module_id, shape.fields)?
                    .iter()
                    .map(|field| (field.key, field.ty, field.is_optional))
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
            let lookup = answer!(self.lookup_member(
                origin,
                module,
                source,
                dir::MemberSpace::Instance,
                key
            )?);

            let member = lookup.value_type();

            match member {
                // missing members satisfy optional targets only
                None => {
                    if !is_optional {
                        return Ok(Answer::Ready(false));
                    }
                }
                Some(member) => {
                    decision = decision.and(self.decide_relation(
                        origin,
                        Relation::Assignable,
                        member,
                        field_type,
                    )?);
                    if decision.is_ready_false() {
                        return Ok(decision);
                    }
                }
            }
        }
        for signature in index_signatures {
            decision =
                decision.and(self.decide_index_signature_satisfied(origin, source, &signature)?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Return the full heritage closure for one nominal application.
    /// `instance_module` is the owner of `instance`'s argument list.
    pub(in crate::check) fn heritage_closure(
        &mut self,
        origin: Origin,
        instance_module: destack_source::ModuleId,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<HeritageClosure>> {
        let mut closure = HeritageClosure::default();
        let mut active = SmallVec::<[dir::GlobalSymbolId; 8]>::new();
        let mut blockers = SmallVec::<[Dependency; 2]>::new();

        // extensions implement each application independently, so
        // same-symbol instantiations dispatch by form instead of conflicting
        let independent = matches!(
            self.definition(instance.symbol),
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

        let blockers = self.live_blockers(blockers);

        Ok(Answer::ready_unless_blocked(closure, blockers))
    }

    /// Collect inherited applications from one nominal application.
    /// `instance_module` is the owner of `instance`'s argument list; every
    /// rebuilt application below interns fresh lists into `origin.module()`.
    fn collect_heritage(
        &mut self,
        origin: Origin,
        instance_module: destack_source::ModuleId,
        instance: &dir::GenericInstance,
        branch_source: Option<dir::GlobalNodeIdAny>,
        independent: bool,
        active: &mut SmallVec<[dir::GlobalSymbolId; 8]>,
        closure: &mut HeritageClosure,
        blockers: &mut SmallVec<[Dependency; 2]>,
    ) -> CompilerResult<()> {
        let Some(definition) = self.definition(instance.symbol) else {
            return Ok(());
        };
        let heritages = definition
            .heritages()
            .iter()
            .map(|heritage| (*heritage).clone())
            .collect::<SmallVec<[_; 2]>>();
        let substitution = self.instance_substitution(instance_module, instance)?;
        let module = origin.module();

        // walk direct heritage edges with applied arguments
        for heritage in heritages {
            let mut arguments = heritage.arguments;
            for argument in &mut arguments {
                *argument = self.substitute_type(origin.module(), *argument, &substitution)?;
            }
            let arguments = self.intern_type_ids(module, &arguments)?;
            let application = HeritageApplication {
                source: branch_source.unwrap_or(heritage.source),
                instance: dir::GenericInstance {
                    symbol: heritage.symbol,
                    arguments,
                },
            };

            // cycles are reported at the branch that exposed the cycle
            if active.contains(&application.instance.symbol) {
                closure.cycles.push(HeritageCycle {
                    source: application.source,
                });
                continue;
            }

            // duplicate applications must use the same arguments,
            // except under declarations that implement each independently
            if let Some(previous) = closure.application(application.instance.symbol) {
                if independent {
                    continue;
                }
                match self.constrain_instance_arguments(
                    origin,
                    module,
                    &previous.instance,
                    module,
                    &application.instance,
                )? {
                    Answer::Ready(true) => {}
                    Answer::Ready(false) => closure.conflicts.push(HeritageConflict {
                        source: application.source,
                        current: application.instance,
                    }),
                    Answer::Pending(pending) => blockers.extend(pending),
                }
                continue;
            }

            // recurse through newly reached applications
            closure.applications.push(application.clone());
            active.push(application.instance.symbol);
            self.collect_heritage(
                origin,
                module,
                &application.instance,
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
    /// `instance_module` is the owner of `instance`'s argument list; the
    /// returned application's argument list always live in `origin.module()`.
    pub(in crate::check) fn heritage_instance(
        &mut self,
        origin: Origin,
        instance_module: destack_source::ModuleId,
        instance: &dir::GenericInstance,
        target: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<Option<dir::GenericInstance>>> {
        let closure = answer!(self.heritage_closure(origin, instance_module, instance)?);
        if let Some(application) = closure.application(target) {
            return Ok(Answer::Ready(Some(application.instance)));
        }

        Ok(Answer::Ready(None))
    }

    /// Relate arguments of two same-template applications.
    pub(in crate::check) fn constrain_instance_arguments(
        &mut self,
        origin: Origin,
        source_module: destack_source::ModuleId,
        source: &dir::GenericInstance,
        target_module: destack_source::ModuleId,
        target: &dir::GenericInstance,
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

        let mut decision = Answer::Ready(true);
        for (left, right) in pairs {
            decision = decision.and(self.constrain(origin, Relation::Equal, left, right)?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Allocate one reference type for a nominal application.
    /// `instance_module` is the owner of `instance`'s argument list.
    fn reference_type(
        &mut self,
        origin: Origin,
        instance_module: destack_source::ModuleId,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let instance = self.origin_instance(origin, instance_module, *instance)?;

        self.intern_type(origin.module(), dir::Type::Instance(instance))
    }
}
