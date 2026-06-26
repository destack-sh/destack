use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Dependency, MemberLookup, Origin, Relation, answer};

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
        let target_symbol = match self.ty(target)? {
            dir::Type::Instance(target) => Some(target.symbol),
            _ => None,
        };
        let target_item = match target_symbol {
            Some(target_symbol) => self.language_item(target_symbol)?,
            None => None,
        };
        if let Some(memory_kind) = memory_kind {
            if target_item == Some(memory_kind) {
                return Ok(Answer::Ready(true));
            }
        }

        // nominal sources meet nominal constraints through their declarations
        let instances = match (self.ty(source)?, self.ty(target)?) {
            (dir::Type::Instance(source), dir::Type::Instance(target)) => {
                Some((source.clone(), target.clone()))
            }
            _ => None,
        };
        let target_instance = match self.ty(target)? {
            dir::Type::Instance(target) => Some(target.clone()),
            _ => None,
        };

        match (instances, relation) {
            (Some((source_instance, target_instance)), _) => {
                self.decide_nominal_satisfies(origin, source, &source_instance, &target_instance)
            }

            // check extension implementations over any receiver form
            (None, _) if self.is_interface_instance(target_instance.as_ref()) => {
                let Some(target_instance) = target_instance.as_ref() else {
                    return Ok(Answer::Ready(false));
                };
                let module = origin.module();
                let implemented =
                    self.decide_extension_implementation(origin, module, source, target_instance)?;
                if !matches!(implemented, Answer::Ready(false)) {
                    return Ok(implemented);
                }

                self.decide_assignable(origin, source, target)
            }

            // explicit implements requires the heritage relation
            (None, Relation::Implements) => Ok(Answer::Ready(false)),

            // everything else satisfies through assignability
            (None, _) => self.decide_assignable(origin, source, target),
        }
    }

    /// Decide whether one nominal application satisfies another.
    fn decide_nominal_satisfies(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        source_instance: &dir::GenericInstance,
        target_instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<bool>> {
        // same-symbol applications compare arguments
        if source_instance.symbol == target_instance.symbol {
            return self.decide_each_argument(origin, source_instance, target_instance);
        }

        // heritage carries the relation when it names the target
        if let Some(heritage) =
            answer!(self.heritage_instance(origin, source_instance, target_instance.symbol)?)
        {
            let arguments = self.decide_each_argument(origin, &heritage, target_instance)?;
            if !matches!(
                self.definition(target_instance.symbol),
                Some(dir::Definition::Interface(_))
            ) {
                return Ok(arguments);
            }

            let members = self.decide_interface_satisfied(origin, source, target_instance)?;

            return Ok(arguments.and(members));
        }

        // check visible extension implementations
        if matches!(
            self.definition(target_instance.symbol),
            Some(dir::Definition::Interface(_))
        ) {
            let module = origin.module();
            let implemented =
                self.decide_extension_implementation(origin, module, source, target_instance)?;
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
            return self.decide_interface_satisfied(origin, source, target_instance);
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
            (dir::Type::Instance(source), dir::Type::Instance(target)) => {
                (source.clone(), target.clone())
            }
            _ => return Ok(Answer::Ready(false)),
        };

        self.decide_nominal_satisfies(origin, source, &source_instance, &target_instance)
    }

    /// Decide whether one structural shape satisfies one reference target.
    pub(in crate::check) fn decide_source_against_reference(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target_instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<bool>> {
        // shapes satisfy structural interfaces member-wise
        let is_structural_interface = matches!(
            self.definition(target_instance.symbol),
            Some(dir::Definition::Interface(interface)) if !interface.is_nominal
        );
        if is_structural_interface {
            return self.decide_interface_satisfied(origin, source, target_instance);
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
            dir::Type::Shape(shape) => shape
                .fields
                .iter()
                .map(|field| (field.key, field.ty))
                .collect::<SmallVec<[_; 8]>>(),
            _ => return Ok(Answer::Ready(false)),
        };

        // require every declared instance field from the literal
        let module = origin.module();
        let mut decision = Answer::Ready(true);
        for key in self.nominal_field_keys(target_instance.symbol) {
            let lookup =
                self.lookup_member(origin, module, target, dir::MemberSpace::Instance, key)?;
            let declared = match lookup {
                MemberLookup::Field(ty) => ty,
                MemberLookup::Found(_) | MemberLookup::Missing => continue,
                MemberLookup::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            };

            // the field must be written at an assignable type
            let Some((_, supplied)) = written.iter().find(|(written, _)| *written == key) else {
                return Ok(Answer::Ready(false));
            };
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

    /// Collect one definition's instance field keys through heritage.
    fn nominal_field_keys(&self, symbol: dir::GlobalSymbolId) -> SmallVec<[dir::StaticKey; 8]> {
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
                    keys.push(field.key);
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
    pub(in crate::check) fn decide_reference_against_target(
        &mut self,
        origin: Origin,
        source_instance: &dir::GenericInstance,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // require each target field from the source fields
        let (fields, index_signatures) = match self.ty(target)? {
            dir::Type::Shape(shape) => (
                shape
                    .fields
                    .iter()
                    .map(|field| (field.key, field.ty, field.is_optional))
                    .collect::<SmallVec<[_; 4]>>(),
                shape.index_signatures.clone(),
            ),
            _ => return Ok(Answer::Ready(false)),
        };
        let module = origin.module();
        let source = self.reference_type(origin, source_instance)?;
        let mut decision = Answer::Ready(true);
        for (key, field_type, is_optional) in fields {
            let lookup =
                self.lookup_member(origin, module, source, dir::MemberSpace::Instance, key)?;

            let member = match lookup {
                MemberLookup::Field(ty) => Some(ty),
                MemberLookup::Found(candidates) => candidates.first().map(|candidate| candidate.ty),
                MemberLookup::Missing => None,
                MemberLookup::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            };

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
    pub(in crate::check) fn heritage_closure(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<HeritageClosure>> {
        let mut closure = HeritageClosure::default();
        let mut active = SmallVec::<[dir::GlobalSymbolId; 8]>::new();
        let mut blockers = SmallVec::<[Dependency; 2]>::new();

        active.push(instance.symbol);
        self.collect_heritage(
            origin,
            instance,
            None,
            &mut active,
            &mut closure,
            &mut blockers,
        )?;

        let blockers = self.live_blockers(blockers);

        Ok(Answer::ready_unless_blocked(closure, blockers))
    }

    /// Collect inherited applications from one nominal application.
    fn collect_heritage(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
        branch_source: Option<dir::GlobalNodeIdAny>,
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
        let substitution = self.instance_substitution(instance)?;
        let module = origin.module();
        let source = self.origin_source_node(origin)?;

        // walk direct heritage edges in their applied view
        for heritage in heritages {
            let mut arguments = heritage.arguments;
            for argument in &mut arguments {
                if !substitution.is_empty() {
                    *argument =
                        self.fold_type(module, source, *argument, substitution.rewrite())?;
                }
            }
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

            // duplicate applications must use the same arguments
            if let Some(previous) = closure.application(application.instance.symbol) {
                match self.decide_each_argument(
                    origin,
                    &previous.instance,
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
                &application.instance,
                Some(application.source),
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
        instance: &dir::GenericInstance,
        target: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<Option<dir::GenericInstance>>> {
        let closure = answer!(self.heritage_closure(origin, instance)?);
        if let Some(application) = closure.application(target) {
            return Ok(Answer::Ready(Some(application.instance.clone())));
        }

        Ok(Answer::Ready(None))
    }

    /// Decide argument-wise equality of two same-template applications.
    pub(in crate::check) fn decide_each_argument(
        &mut self,
        origin: Origin,
        source: &dir::GenericInstance,
        target: &dir::GenericInstance,
    ) -> CompilerResult<Answer<bool>> {
        if source.arguments.len() != target.arguments.len() {
            return Ok(Answer::Ready(false));
        }

        let pairs = source
            .arguments
            .iter()
            .copied()
            .zip(target.arguments.iter().copied())
            .collect::<SmallVec<[_; 4]>>();

        self.decide_each(origin, Relation::Equal, &pairs)
    }

    /// Allocate one reference type for a nominal application.
    fn reference_type(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let source = self.origin_source_node(origin)?;

        self.push_type(
            origin.module(),
            dir::Type::Instance(instance.clone()),
            source,
        )
    }
}
