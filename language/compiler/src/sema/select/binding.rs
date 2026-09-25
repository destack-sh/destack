use std::sync::Arc;

use smallvec::SmallVec;
use tspp_core::{FxIndexMap, FxIndexSet};
use tspp_dir as dir;
use tspp_dir::TypeFold;

use crate::CompilerResult;
use crate::sema::{
    ApparentInstance, CandidateSource, CheckState, DeclaredSource, LookupReceiver, MemberCandidate,
    MemberLookup, MemberRole, Origin,
};

impl CheckState<'_> {
    /// Return one closed subject's inherent members in declaration preorder, grouped by key.
    pub(in crate::sema) fn inherent_member_table(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        instance: &ApparentInstance,
        space: dir::MemberSpace,
    ) -> CompilerResult<FxIndexMap<dir::StaticKey, MemberLookup>> {
        // derive the table from the owner's canonical member bindings
        if let Some(bindings) = self.member_bindings(instance.symbol, space)? {
            let mut table = FxIndexMap::default();
            for binding in bindings.iter() {
                let candidates =
                    self.binding_member_candidates(origin, receiver, instance, binding, space)?;
                table.insert(binding.key, candidates.into());
            }

            return Ok(table);
        }

        // walk the declaration levels in preorder
        let mut table = FxIndexMap::<dir::StaticKey, MemberLookup>::default();
        let mut stack = vec![instance.clone()];
        let mut visited = FxIndexSet::default();
        while let Some(level) = stack.pop() {
            if !visited.insert((level.symbol, level.arguments.clone())) {
                continue;
            }

            let Some(definition) = self.definition(level.symbol)? else {
                continue;
            };

            // collect the keys this level declares in the searched space
            let mut keys = Vec::new();
            for member in definition.members() {
                let Some(key) = member.key() else {
                    continue;
                };
                if member.space() == space && !table.contains_key(&key) && !keys.contains(&key) {
                    keys.push(key);
                }
            }

            let heritages = definition
                .bases()
                .iter()
                .map(|heritage| heritage.ty)
                .collect::<SmallVec<[_; 2]>>();

            // build the level's candidates for each unclaimed key
            let receiver_value = self.strip_form(origin, receiver)?;
            let substitution = level.substitution(self)?.with_receiver(receiver_value);
            for key in keys {
                let members = self
                    .definition(level.symbol)?
                    .map(|definition| {
                        definition
                            .members_with_key(space, key)
                            .cloned()
                            .collect::<SmallVec<[_; 2]>>()
                    })
                    .unwrap_or_default();
                let candidates = self.instance_member_candidates(
                    origin,
                    receiver,
                    &level,
                    &substitution,
                    &members,
                    key,
                )?;
                if !candidates.is_empty() {
                    table.insert(key, candidates.into());
                }
            }

            // push heritage levels in reverse for preorder traversal
            for heritage in heritages.into_iter().rev() {
                let heritage = self.substitute_type(heritage, &substitution)?;
                let (heritage_module, heritage) = self.nominal_application(heritage)?;
                let arguments = self.type_ids(heritage_module, heritage.arguments)?;
                stack.push(ApparentInstance {
                    symbol: heritage.symbol,
                    arguments: arguments.iter().copied().collect(),
                });
            }
        }

        Ok(table)
    }

    /// Build one owner's canonical member bindings with `this` symbolic.
    pub(in crate::sema) fn canonical_member_bindings(
        &mut self,
        instance: &ApparentInstance,
        space: dir::MemberSpace,
    ) -> CompilerResult<Option<Vec<dir::MemberBinding>>> {
        let mut bindings: Vec<dir::MemberBinding> = Vec::new();
        let mut stack = vec![(instance.clone(), false)];
        let mut visited = FxIndexSet::default();

        // walk the declaration levels in preorder, first level per key wins
        while let Some((level, is_conformance)) = stack.pop() {
            if !visited.insert((level.symbol, level.arguments.clone())) {
                continue;
            }

            let Some(definition) = self.definition(level.symbol)? else {
                return Ok(None);
            };

            let substitution = level.substitution(self)?;

            // bind each declared member the nearer levels left open
            for member in definition.members() {
                let Some(key) = member.key() else {
                    continue;
                };
                if member.space() != space {
                    continue;
                }

                // conformance levels serve their default members only
                if is_conformance && !self.definition_member_has_default(member)? {
                    continue;
                }

                let Some(declared) = self.declared_member(member)? else {
                    continue;
                };

                let ty = match declared.ty {
                    Some(ty) => ty,
                    // valueless associated members project through receivers
                    None if declared.role == MemberRole::Associated => {
                        let arguments = self.intern_type_ids(&[])?;
                        let owner = level.intern(self)?;

                        self.intern_member(dir::MemberType {
                            owner,
                            key,
                            arguments,
                            qualifier: None,
                        })?
                    }
                    None => continue,
                };

                // apply this level's arguments and read the member's operations
                let ty = self.substitute_type(ty, &substitution)?;
                let ty = self.shallow_resolve(ty)?;
                let callable = declared.callable_type(ty);
                let access = declared.access(self, ty)?;

                let declaration = dir::MemberDeclaration {
                    symbol: declared.symbol,
                    owner: level.symbol,
                    origin: dir::MemberOrigin::Declaration,
                    role: declared.role,
                    callable_type: callable,
                };

                // overloads and accessor pairs extend their key in place
                if let Some(binding) = bindings.iter_mut().find(|binding| binding.key == key) {
                    let same_level = binding
                        .declarations
                        .first()
                        .is_some_and(|first| first.owner == level.symbol);
                    let known = binding
                        .declarations
                        .iter()
                        .any(|previous| previous.symbol == declared.symbol);
                    if same_level && !known {
                        // join an accessor pair with its counterpart's access
                        binding.access = match (binding.access.read(), declared.role) {
                            (Some(read), MemberRole::Setter) => {
                                match binding.access.write().or(access.write()) {
                                    Some(write) => dir::PropertyAccess::ReadWrite { read, write },
                                    None => binding.access,
                                }
                            }
                            (None, MemberRole::Getter) => match access.read() {
                                Some(read) => match binding.access.write() {
                                    Some(write) => dir::PropertyAccess::ReadWrite { read, write },
                                    None => dir::PropertyAccess::Read(read),
                                },
                                None => binding.access,
                            },
                            _ => binding.access,
                        };

                        binding.declarations.push(declaration);
                    }

                    continue;
                }

                bindings.push(dir::MemberBinding::new(
                    key,
                    declared.kind,
                    access,
                    declared.is_optional,
                    vec![declaration],
                ));
            }

            // push substituted heritage levels in reverse for preorder
            let heritages = definition
                .bases()
                .iter()
                .map(|heritage| (heritage.ty, is_conformance))
                .chain(
                    definition
                        .implementations()
                        .map(|conformance| (conformance.interface, true)),
                )
                .collect::<SmallVec<[_; 2]>>();
            for (heritage, is_conformance) in heritages.into_iter().rev() {
                let heritage = self.substitute_type(heritage, &substitution)?;
                let Some((heritage_module, heritage)) = self.nominal_application_maybe(heritage)?
                else {
                    continue;
                };

                // push the base's apparent instance at its arguments
                let arguments = self.type_ids(heritage_module, heritage.arguments)?;
                stack.push((
                    ApparentInstance {
                        symbol: heritage.symbol,
                        arguments: arguments.iter().copied().collect(),
                    },
                    is_conformance,
                ));
            }
        }

        Ok(Some(bindings))
    }

    /// Return the applied heritage level of one declaring owner.
    fn heritage_level_instance(
        &mut self,
        instance: &ApparentInstance,
        owner: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<ApparentInstance>> {
        let mut stack = vec![instance.clone()];
        let mut visited = FxIndexSet::default();

        // walk the substituted heritage levels until the owner appears
        while let Some(level) = stack.pop() {
            if level.symbol == owner {
                return Ok(Some(level));
            }
            if !visited.insert((level.symbol, level.arguments.clone())) {
                continue;
            }

            let Some(definition) = self.definition(level.symbol)? else {
                continue;
            };

            let heritages = definition
                .bases()
                .iter()
                .map(|heritage| heritage.ty)
                .collect::<SmallVec<[_; 2]>>();
            let substitution = level.substitution(self)?;

            for heritage in heritages {
                let heritage = self.substitute_type(heritage, &substitution)?;
                let Some((heritage_module, heritage)) = self.nominal_application_maybe(heritage)?
                else {
                    continue;
                };

                // push the base's apparent instance at its arguments
                let arguments = self.type_ids(heritage_module, heritage.arguments)?;
                stack.push(ApparentInstance {
                    symbol: heritage.symbol,
                    arguments: arguments.iter().copied().collect(),
                });
            }
        }

        Ok(None)
    }

    /// Derive one key's member lookup from the owner's canonical member bindings.
    pub(super) fn stored_member_lookup(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        instance: &ApparentInstance,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        // derive the canonicalized key only; absent owners searched live already
        let Some(bindings) = self.member_bindings(instance.symbol, space)? else {
            let table = self.inherent_member_table(origin, receiver, instance, space)?;

            return Ok(table.get(&key).cloned().unwrap_or_default());
        };
        let Some(binding) = bindings.iter().find(|binding| binding.key == key) else {
            return Ok(MemberLookup::default());
        };

        // substitute the binding through this instance and receiver
        let candidates =
            self.binding_member_candidates(origin, receiver, instance, binding, space)?;

        Ok(candidates.into())
    }

    /// Return one owner's canonical member bindings, memoized per run.
    pub(in crate::sema) fn member_bindings(
        &mut self,
        symbol: dir::GlobalSymbolId,
        space: dir::MemberSpace,
    ) -> CompilerResult<Option<Arc<Vec<dir::MemberBinding>>>> {
        // serve the memo
        if let Some(bindings) = self.member_bindings.get(&(symbol, space)) {
            self.counters.binding_reuses += 1;

            return Ok(bindings.clone());
        }

        // count this derivation
        self.counters.binding_derivations += 1;

        // read owners their module's elaborate pass already flattened
        let stored = self.stored_member_bindings(symbol, space)?;

        // build the canonical bindings for unflattened owners
        let bindings = match stored {
            Some(bindings) => Some(Arc::new(bindings)),
            None => {
                let application = self.declaration_instance(symbol)?;
                let module = self.module_id;
                let arguments: SmallVec<[_; 8]> =
                    self.type_ids(module, application.arguments)?.into();
                let instance = ApparentInstance {
                    symbol,
                    arguments: arguments.into_iter().collect(),
                };

                self.canonical_member_bindings(&instance, space)?
                    .map(Arc::new)
            }
        };

        // memoize the bindings for every later goal
        self.member_bindings
            .insert((symbol, space), bindings.clone());

        Ok(bindings)
    }

    /// Return the member bindings one owner's elaborate pass flattened.
    fn stored_member_bindings(
        &self,
        symbol: dir::GlobalSymbolId,
        space: dir::MemberSpace,
    ) -> CompilerResult<Option<Vec<dir::MemberBinding>>> {
        // read own owners from the pass tail over the committed base
        if self.is_own_module(symbol.module_id) {
            let module = &self.module;
            if let Some(bindings) = module.members_tail.bindings(symbol, space) {
                return Ok(Some(bindings.to_vec()));
            }

            return Ok(module
                .members
                .iter()
                .find_map(|base| base.bindings(symbol, space))
                .map(<[dir::MemberBinding]>::to_vec));
        }

        // read foreign owners from their module's elaborated bindings
        Ok(self
            .external(symbol.module_id)?
            .and_then(|external| external.members().bindings(symbol, space))
            .map(<[dir::MemberBinding]>::to_vec))
    }

    /// Derive one stored member binding's candidates for a lookup instance.
    fn binding_member_candidates(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        instance: &ApparentInstance,
        binding: &dir::MemberBinding,
        space: dir::MemberSpace,
    ) -> CompilerResult<Vec<MemberCandidate>> {
        // substitute the canonical types for this instance and the receiver's object
        let receiver_value = self.strip_form(origin, receiver)?;
        let substitution = instance.substitution(self)?.with_receiver(receiver_value);
        let generic_arguments =
            self.symbol_generic_argument_bindings(instance.symbol, &instance.arguments)?;

        // build one candidate per declaration behind the key
        let mut candidates = Vec::with_capacity(binding.declarations.len());
        for declaration in &binding.declarations {
            // carry the arguments of the declaring heritage level
            let level = match declaration.owner == instance.symbol {
                true => None,
                false => self.heritage_level_instance(instance, declaration.owner)?,
            };
            let generic_arguments = match &level {
                Some(level) => {
                    self.symbol_generic_argument_bindings(level.symbol, &level.arguments)?
                }
                None => generic_arguments.clone(),
            };

            // project a rigid receiver's associated members through itself
            let mut access = binding.access;
            if declaration.role == dir::MemberRole::Associated
                && self.is_rigid_projection_owner(receiver)?
            {
                let arguments = self.intern_type_ids(&[])?;
                let qualifier = Some(level.as_ref().unwrap_or(instance).qualifier(self)?);
                let projected = self.intern_member(dir::MemberType {
                    owner: receiver,
                    key: binding.key,
                    arguments,
                    qualifier,
                })?;
                access = dir::PropertyAccess::Read(projected);
            } else {
                access.map_types(&mut |ty| self.substitute_type(ty, &substitution))?;
            }
            access.map_types(&mut |ty| {
                self.projected_member_type(origin, Some(receiver), declaration.role, ty)
            })?;

            // read a method as its own signature at the receiver
            let callable = match declaration.callable_type {
                Some(callable) => Some(self.substitute_type(callable, &substitution)?),
                None => None,
            };
            if let Some(callable) = callable
                && declaration.role == dir::MemberRole::Method
            {
                access = dir::PropertyAccess::Read(callable);
            }

            // substitute the stored static value through the same instance
            let value = self.symbol_static_id(declaration.symbol)?;
            let value_type = match self.static_value(declaration.symbol)? {
                Some(written) => Some(self.substitute_type(written, &substitution)?),
                None => None,
            };

            // read the interface whose requirement the member implements
            let requirement = match declaration.origin {
                dir::MemberOrigin::Declaration => None,
                _ => self.requirement_interface(declaration.owner, binding.key)?,
            };
            let mut declared = DeclaredSource::new(
                declaration.symbol,
                declaration.owner,
                declaration.origin,
                generic_arguments,
            );
            declared.region_arguments = self.resolved_region_bindings(&substitution.bindings)?;
            declared.requirement = requirement;
            declared.value = value;
            declared.value_type = value_type;
            candidates.push(MemberCandidate {
                source: CandidateSource::Declared(declared),
                space,
                role: declaration.role,
                kind: binding.kind,
                access,
                callable,
                is_optional: binding.is_optional,
                receiver: LookupReceiver::Direct(Vec::new()),
                arm: None,
            });
        }

        Ok(candidates)
    }
}
