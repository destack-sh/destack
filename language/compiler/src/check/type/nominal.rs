use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, MemberLookup, Origin, Relation};

impl CheckState<'_> {
    /// Decide one check-only constraint relation between closed roots.
    pub(in crate::check) fn decide_satisfies(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        if self.decide_equal(origin, source, target)? == Answer::Ready(true) {
            return Ok(Answer::Ready(true));
        }

        // nominal sources meet nominal constraints through their declarations
        let instances = match (self.ty(source)?, self.ty(target)?) {
            (dir::Type::Reference(source), dir::Type::Reference(target)) => {
                Some((source.clone(), target.clone()))
            }
            _ => None,
        };

        match (instances, relation) {
            (Some((source_instance, target_instance)), _) => {
                self.decide_nominal_satisfies(origin, source, &source_instance, &target_instance)
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
        let heritage = self.heritage_instance(origin, source_instance, target_instance.symbol)?;
        match heritage {
            Answer::Ready(Some(heritage)) => {
                return self.decide_each_argument(origin, &heritage, target_instance);
            }
            Answer::Ready(None) => {}
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        }

        // structural interfaces satisfy member-wise
        let target_definition = self.definition(target_instance.symbol);
        let is_structural_interface = matches!(
            target_definition,
            Some(dir::Definition::Interface(interface)) if !interface.is_nominal
        );
        if is_structural_interface {
            return self.decide_member_satisfies(origin, source, target_instance);
        }

        Ok(Answer::Ready(false))
    }

    /// Decide assignability between different nominal applications.
    pub(in crate::check) fn decide_nominal_assignable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let (source_instance, target_instance) = match (self.ty(source)?, self.ty(target)?) {
            (dir::Type::Reference(source), dir::Type::Reference(target)) => {
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
            // fresh literals may only supply known properties
            if self.is_fresh_literal(source)? {
                let keys = self.nominal_member_keys(target_instance.symbol);
                if let dir::Type::Shape(shape) = self.ty(source)? {
                    for field in &shape.fields {
                        if !keys.contains(&field.key) {
                            return Ok(Answer::Ready(false));
                        }
                    }
                }
            }

            return self.decide_member_satisfies(origin, source, target_instance);
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
            if decision == Answer::Ready(false) {
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
            for heritage in definition.heritages() {
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
            for heritage in definition.heritages() {
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
        let fields = match self.ty(target)? {
            dir::Type::Shape(shape) => shape
                .fields
                .iter()
                .map(|field| (field.key, field.ty, field.is_optional))
                .collect::<SmallVec<[_; 4]>>(),
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
                    if decision == Answer::Ready(false) {
                        return Ok(decision);
                    }
                }
            }
        }

        Ok(decision)
    }

    /// Decide whether one source exposes every member of one interface application.
    pub(in crate::check) fn decide_member_satisfies(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target_instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<bool>> {
        let module = origin.module();
        let Some(definition) = self.definition(target_instance.symbol) else {
            return Ok(Answer::Ready(false));
        };

        // collect required members across both spaces
        let members = definition
            .members()
            .iter()
            .filter_map(|member| member.key().map(|key| (member.space(), key, member.ty())))
            .collect::<SmallVec<[_; 4]>>();
        // the satisfying source binds the interface's `this`
        let substitution = self
            .parameter_substitution(target_instance)?
            .with_receiver(source);
        let source_node = self.origin_source_node(origin)?;

        // require each interface member from the source
        let mut decision = Answer::Ready(true);
        for (space, key, member_type) in members {
            let lookup = self.lookup_member(origin, module, source, space, key)?;

            let found = match lookup {
                MemberLookup::Field(ty) => Some(ty),
                MemberLookup::Found(candidates) => candidates.first().map(|candidate| candidate.ty),
                MemberLookup::Missing => None,
                MemberLookup::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            };
            let Some(found) = found else {
                return Ok(Answer::Ready(false));
            };

            // associated types without values only need presence
            let Some(member_type) = member_type else {
                continue;
            };
            let member_type = if substitution.is_empty() {
                member_type
            } else {
                self.fold_type(module, source_node, member_type, substitution.rewrite())?
            };

            decision = decision.and(self.decide_relation(
                origin,
                Relation::Assignable,
                found,
                member_type,
            )?);
            if decision == Answer::Ready(false) {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Find one heritage application naming a target symbol, transitively.
    fn heritage_instance(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
        target: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<Option<dir::GenericInstance>>> {
        let mut visited = SmallVec::new();
        self.heritage_instance_guarded(origin, instance, target, &mut visited)
    }

    /// Find one heritage application with the visited chain tracked.
    fn heritage_instance_guarded(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
        target: dir::GlobalSymbolId,
        visited: &mut SmallVec<[dir::GlobalSymbolId; 8]>,
    ) -> CompilerResult<Answer<Option<dir::GenericInstance>>> {
        // heritage cycles carry no relation
        if visited.contains(&instance.symbol) {
            return Ok(Answer::Ready(None));
        }
        visited.push(instance.symbol);

        let Some(definition) = self.definition(instance.symbol) else {
            return Ok(Answer::Ready(None));
        };
        let heritages = definition
            .heritages()
            .iter()
            .map(|heritage| (heritage.symbol, heritage.arguments.clone()))
            .collect::<SmallVec<[_; 2]>>();
        let substitution = self.parameter_substitution(instance)?;
        let module = origin.module();
        let source = self.origin_source_node(origin)?;

        // search substituted heritage applications transitively
        for (symbol, arguments) in heritages {
            let mut arguments = arguments;
            for argument in &mut arguments {
                if !substitution.is_empty() {
                    *argument =
                        self.fold_type(module, source, *argument, substitution.rewrite())?;
                }
            }
            let heritage = dir::GenericInstance { symbol, arguments };

            // direct heritage names the target
            if symbol == target {
                return Ok(Answer::Ready(Some(heritage)));
            }

            // otherwise search the heritage's own chain
            match self.heritage_instance_guarded(origin, &heritage, target, visited)? {
                Answer::Ready(Some(found)) => return Ok(Answer::Ready(Some(found))),
                Answer::Ready(None) => {}
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            }
        }

        Ok(Answer::Ready(None))
    }

    /// Decide argument-wise equality of two same-template applications.
    fn decide_each_argument(
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
            dir::Type::Reference(instance.clone()),
            source,
        )
    }
}
