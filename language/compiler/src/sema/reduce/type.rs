use destack_core::{FxIndexMap, FxIndexSet, ensure_sufficient_stack};
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::sema::{CheckState, Origin};

impl CheckState<'_> {
    /// Return whether one value type is carried by the erased dynamic payload.
    pub(in crate::sema) fn is_erased_value(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        Ok(self.erased_constraint(ty)?.is_some())
    }

    /// Return the runtime constraint carried by one erased value type.
    pub(in crate::sema) fn erased_constraint(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // settle solved variables and transparent aliases first
        let mut ty = self.shallow_resolve(ty)?;
        let mut seen = FxIndexSet::default();
        while seen.insert(ty) {
            let dir::Type::Application(instance) = self.ty(ty)? else {
                break;
            };
            if self.language_item(instance.symbol)?.is_some() {
                break;
            }
            let Some(dir::Definition::TypeAlias(alias)) = self.definition(instance.symbol)? else {
                break;
            };
            let body = alias.value;
            ty = self.shallow_resolve(body)?;
        }

        match self.ty(ty)? {
            // explicit erasure names its constraint
            dir::Type::Dynamic(dynamic) => Ok(Some(dynamic.constraint)),

            // top types carry the dynamic payload at themselves
            dir::Type::Any | dir::Type::Unknown => Ok(Some(ty)),

            // interface-typed values erase behind their constraint
            dir::Type::Application(instance) => Ok(matches!(
                self.symbol_kind(instance.symbol)?,
                dir::SymbolKind::Interface | dir::SymbolKind::NewtypeInterface
            )
            .then_some(ty)),

            // every other value type carries its own representation
            _ => Ok(None),
        }
    }

    /// Resolve solved variables through one type graph, keeping open holes.
    pub(in crate::sema) fn deeply_resolve(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut shared = FxIndexMap::default();

        self.deeply_resolve_shared(origin, id, &mut shared)
    }

    /// Resolve solved variables through one type graph, replaying scoped heads.
    pub(in crate::sema) fn deeply_resolve_shared(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        shared: &mut FxIndexMap<
            (Option<dir::GlobalGenericTemplateId>, dir::GlobalTypeId),
            dir::GlobalTypeId,
        >,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // replay the root this scope already resolved
        let id = self.shallow_resolve(id)?;
        let scope = self.assuming_scope(origin)?;
        if let Some(resolved) = shared.get(&(scope, id)) {
            return Ok(*resolved);
        }

        // normalize the children with aliases kept symbolic
        let mut memo = FxIndexMap::default();
        let mut active = FxIndexSet::default();
        let resolved = self.normalize_graph(origin, id, &mut memo, &mut active)?;
        shared.insert((scope, id), resolved);

        Ok(resolved)
    }

    /// Splice one tuple's closed rest spreads into positional elements.
    fn reduce_tuple_rest_splice(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        tuple: dir::TupleType,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // locate the rest element among the written positions
        let elements: SmallVec<[_; 4]> = self.tuple_elements(id.module_id, tuple.elements)?.into();
        let Some(rest_index) = elements.iter().position(|element| element.is_rest) else {
            return Ok(None);
        };

        // read the spread as a closed tuple
        let rest = self.structurally_normalize(origin, elements[rest_index].ty)?;
        let dir::Type::Tuple(spread) = self.ty(rest)? else {
            return Ok(None);
        };

        // splice the spread elements in place
        let mut spliced: SmallVec<[_; 4]> = elements[..rest_index].into();
        spliced.extend(
            self.tuple_elements(rest.module_id, spread.elements)?
                .iter()
                .copied(),
        );
        spliced.extend(elements[rest_index + 1..].iter().copied());
        let spliced = self.intern_elements(&spliced)?;
        let spliced = self.intern_type(dir::Type::Tuple(dir::TupleType {
            form: tuple.form,
            elements: spliced,
        }))?;

        Ok(Some(spliced))
    }

    /// Splat one signature's closed tuple rest parameter into positional parameters.
    fn reduce_signature_rest_splat(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        signature: dir::FunctionSignatureType,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let parameters: SmallVec<[_; 4]> = self
            .signature_parameters(id.module_id, signature.parameters)?
            .into();
        let Some((rest_index, rest)) = parameters
            .iter()
            .enumerate()
            .find(|(_, parameter)| parameter.is_rest)
        else {
            return Ok(None);
        };
        // resolve a stuck rest head to its closed tuple
        let rest_ty = self.structurally_normalize(origin, rest.ty)?;
        let dir::Type::Tuple(tuple) = self.ty(rest_ty)? else {
            return Ok(None);
        };

        // rebuild positional parameters from the tuple elements
        let mut rebuilt: SmallVec<[_; 4]> = parameters[..rest_index].into();
        for element in self.tuple_elements(rest_ty.module_id, tuple.elements)? {
            rebuilt.push(dir::FunctionParameterType {
                name: None,
                ty: element.ty,
                is_optional: element.is_optional,
                is_rest: false,
            });
        }
        rebuilt.extend(parameters[rest_index + 1..].iter().copied());

        let parameters = self.intern_parameters(&rebuilt)?;
        let splatted = self.intern_signature(dir::FunctionSignatureType {
            parameters,
            ..signature
        })?;

        Ok(Some(splatted))
    }

    /// Normalize one closed head once at construction.
    pub(in crate::sema) fn normalize_closed(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // keep the reductions of parameter carriers lazy under their assuming scope
        let flags = self.type_flags(id)?;
        if flags.has_parameter() || flags.has_this() {
            return Ok(id);
        }

        // reduce alias applications and closed projections at their declaration
        let Some(origin) = self.head_origin(id)? else {
            return Ok(id);
        };
        let mut expanding = FxIndexSet::default();

        self.normalize_chain(origin, id, &mut expanding)
    }

    /// Return the declaration origin owning one reducible head.
    fn head_origin(&self, id: dir::GlobalTypeId) -> CompilerResult<Option<Origin>> {
        let origin = match self.ty(id)? {
            dir::Type::Application(instance) => Some(Origin::Symbol(instance.symbol)),
            dir::Type::Reference(reference) => Some(Origin::Symbol(reference.symbol)),
            dir::Type::Member(member) => {
                let member = self.type_member(id.module_id, member)?;

                return self.head_origin(member.owner);
            }
            dir::Type::Refined(refined) => {
                let refined = self.type_refined(id.module_id, refined)?;

                return self.head_origin(refined.base);
            }
            _ => None,
        };

        Ok(origin)
    }

    /// Normalize one anonymous computation head, keeping names and forms rigid.
    pub(in crate::sema) fn normalize_computation(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let id = self.shallow_resolve(id)?;
        match self.ty(id)? {
            // reduce meta heads, which name computations
            dir::Type::Member(_) | dir::Type::Operation(_) => self.normalize(origin, id),
            // reduce memory accessor applications, which name computations over forms
            dir::Type::Application(instance)
                if self.language_item(instance.symbol)?.is_some_and(|item| {
                    matches!(
                        item,
                        dir::LanguageItem::WithBase
                            | dir::LanguageItem::WithOwnership
                            | dir::LanguageItem::WithPlace
                            | dir::LanguageItem::WithSpace
                            | dir::LanguageItem::WithLifetime
                            | dir::LanguageItem::WithAccess
                    )
                }) =>
            {
                self.normalize(origin, id)
            }
            // keep every other head as written
            _ => Ok(id),
        }
    }

    /// Normalize one transparent alias head to its body, keeping every other head rigid.
    pub(in crate::sema) fn normalize_alias_head(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut current = id;
        loop {
            let symbol = match self.ty(current)? {
                dir::Type::Application(instance) => instance.symbol,
                dir::Type::Reference(reference) => reference.symbol,
                _ => return Ok(current),
            };
            if !matches!(
                self.definition(symbol)?,
                Some(dir::Definition::TypeAlias(_))
            ) {
                return Ok(current);
            }
            let reduced = self.structurally_normalize(origin, current)?;
            if reduced == current {
                return Ok(current);
            }
            current = reduced;
        }
    }

    /// Normalize one named head to a rigid structural form, keeping memory forms rigid.
    pub(in crate::sema) fn structurally_normalize(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // reduce only the named families a stuck relation can still unblock
        let id = self.shallow_resolve(id)?;
        if !matches!(
            self.ty(id)?,
            dir::Type::Application(_)
                | dir::Type::Reference(_)
                | dir::Type::Member(_)
                | dir::Type::Operation(_)
                | dir::Type::FunctionSignature(_)
        ) {
            return Ok(id);
        }

        self.normalize(origin, id)
    }

    /// Normalize the head of one type to its simplest available form.
    pub(in crate::sema) fn normalize(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.counters.reduces += 1;

        self.normalize_head(origin, id)
    }

    /// Reduce one type head, recording the reductions that close.
    fn normalize_head(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let id = self.shallow_resolve(id)?;
        let flags = self.type_flags(id)?;

        // parameter and This heads reduce under their assuming template
        let assumes = self.decision_scope(origin, flags)?;

        // replay decided reductions
        if let Some(assumes) = assumes
            && let Some(reduced) = self.normalizations.get(&(id, assumes))
        {
            return Ok(*reduced);
        }

        // close heads outside the reducible families, which are already normal
        if !matches!(
            self.ty(id)?,
            dir::Type::Variable(_)
                | dir::Type::Union(_)
                | dir::Type::FunctionSignature(_)
                | dir::Type::Application(_)
                | dir::Type::Member(_)
                | dir::Type::Operation(_)
                | dir::Type::Form(_)
                | dir::Type::Intersection(_)
                | dir::Type::Tuple(_)
        ) {
            // record the fixed point of a settled head, skipping the declaring pass
            if let Some(assumes) = assumes
                && !flags.has_variable()
                && !self.is_declaration()
            {
                self.normalizations.insert((id, assumes), id);
            }

            return Ok(id);
        }

        // walk the reduction chain, tracking the heads it expands
        let mut expanding = FxIndexSet::default();
        let reduced = self.normalize_chain(origin, id, &mut expanding)?;

        // decide variable-free reductions once, keeping variable heads open
        if let Some(assumes) = assumes
            && !flags.has_variable()
            && !self.type_flags(reduced)?.has_variable()
            && !self.is_declaration()
        {
            self.normalizations.insert((id, assumes), reduced);
        }

        Ok(reduced)
    }

    /// Reduce one settled type head with the active expansion chain tracked.
    fn normalize_chain(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        expanding: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        ensure_sufficient_stack(|| self.normalize_chain_recursive(origin, id, expanding))
    }

    /// Reduce one type chain on the grown stack.
    fn normalize_chain_recursive(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        expanding: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // report circular expansions and complete the chain with the error type
        if !expanding.insert(id) {
            self.report_circular_type(origin)?;
            let error = self.intern_type(dir::Type::Error)?;

            return Ok(error);
        }

        // load foreign heads before their chains expand
        if !self.is_own_module(id.module_id) {
            self.import_external_module(id.module_id)?;
        }

        match self.ty(id)? {
            // open variables block the chain as its own head
            dir::Type::Variable(_) => Ok(id),

            // continue written names as their nominal application
            dir::Type::Reference(reference) => {
                // keep bare names of generic symbols with required parameters symbolic
                if let Some(template) = self.symbol_template(reference.symbol)? {
                    let parameters = self.generic_template_parameters(template)?;
                    for parameter in parameters {
                        let Some(binding) = self.generic_parameter(parameter).copied() else {
                            continue;
                        };
                        if binding.is_writable()
                            && binding.default.is_none()
                            && binding.memory_parameter() != Some(dir::MemoryParameter::Lifetime)
                        {
                            return Ok(id);
                        }
                    }
                }

                // apply the name with no arguments and continue the chain
                let application = dir::GenericApplication {
                    symbol: reference.symbol,
                    arguments: self.intern_type_ids(&[])?,
                };
                let applied = self.intern_type(dir::Type::Application(application))?;
                if applied == id {
                    return Ok(id);
                }

                self.normalize_chain(origin, applied, expanding)
            }

            // unions normalize their canonical elements in place
            dir::Type::Union(union) => {
                let elements: SmallVec<[_; 8]> =
                    self.type_ids(id.module_id, union.elements)?.into();
                let normalized = self.normalized_union_type(elements)?;
                if normalized == id {
                    return Ok(id);
                }

                self.normalize_chain_recursive(origin, normalized, expanding)
            }

            // splice closed rest spreads into their tuple positionally
            dir::Type::Tuple(tuple) => match self.reduce_tuple_rest_splice(origin, id, tuple)? {
                Some(spliced) => Ok(spliced),
                None => Ok(id),
            },

            // rest parameters with closed tuple types splat positionally
            dir::Type::FunctionSignature(signature) => {
                let signature = self.type_signature(id.module_id, signature)?;
                match self.reduce_signature_rest_splat(origin, id, signature)? {
                    Some(splatted) => Ok(splatted),
                    None => Ok(id),
                }
            }

            // written applications complete their elided arguments
            dir::Type::Application(instance) => {
                if let Some(filled) = self.fill_elided_application(id.module_id, &instance)? {
                    return self.normalize_head(origin, filled);
                }

                // reduce intrinsic references to their builtin forms
                if let Some(reduced) =
                    self.reduce_intrinsic_reference(origin, id.module_id, &instance)?
                {
                    return self.normalize_head(origin, reduced);
                }

                match self.type_alias_body(origin, id.module_id, &instance)? {
                    Some(value) => {
                        let value = self.shallow_resolve(value)?;

                        self.normalize_chain(origin, value, expanding)
                    }
                    None => Ok(id),
                }
            }

            // member projections resolve through their owners
            dir::Type::Member(member) => {
                let member = self.type_member(id.module_id, member)?;

                // peel the owner's forms for the projection
                let owner = member.owner;
                let peeled = self.strip_form(origin, owner)?;

                // error owners poison their projections
                if matches!(self.ty(peeled)?, dir::Type::Error) {
                    let error = self.intern_type(dir::Type::Error)?;

                    return Ok(error);
                }

                // unqualified projections select one declaring interface
                let mut qualifier = member.qualifier;
                if qualifier.is_none() {
                    qualifier = self
                        .body()
                        .select_associated_qualifier(origin, peeled, member.key)?;
                }

                // shed owner forms for unqualified member lookup
                let owner = if qualifier.is_none() { peeled } else { owner };

                // rebuild the projection when its owner or qualifier moved
                if owner != member.owner || qualifier != member.qualifier {
                    let arguments = SmallVec::<[dir::GlobalTypeId; 4]>::from_slice(
                        self.type_ids(id.module_id, member.arguments)?,
                    );
                    let arguments = self.intern_type_ids(&arguments)?;
                    let rebuilt = self.intern_member(dir::MemberType {
                        owner,
                        key: member.key,
                        arguments,
                        qualifier,
                    })?;

                    return self.normalize_chain(origin, rebuilt, expanding);
                }

                let projection = self.body().project_member(origin, &member)?;
                let Some(projected) = projection else {
                    return Ok(id);
                };
                let projected = self.shallow_resolve(projected)?;

                self.normalize_chain(origin, projected, expanding)
            }

            // reduce type operations once their inputs close
            dir::Type::Operation(operation) => {
                let operation = self.type_operation(id.module_id, operation)?;
                let reduction = self.reduce_operation(origin, id, &operation)?;

                let Some(reduced) = reduction else {
                    return Ok(id);
                };
                let reduced = self.shallow_resolve(reduced)?;

                self.normalize_chain(origin, reduced, expanding)
            }

            // borrows absorb payload forms and retain placement around the resulting handle
            dir::Type::Form(form) if let dir::Form::Borrowed(borrow) = form.form => {
                let borrow = self.type_borrow(id.module_id, borrow)?;

                // absorb the payload forms the borrow carries itself
                let (payload, access, place) =
                    self.reduce_borrow_payload(form.value, borrow.access)?;
                if payload == form.value && access == borrow.access && place.is_none() {
                    return Ok(id);
                }

                // rebuild the borrow around the absorbed payload
                let closed_form = self.intern_borrow(borrow.lifetime, access)?;
                let mut rebuilt = self.intern_type(dir::Type::Form(dir::FormType {
                    form: closed_form,
                    value: payload,
                }))?;
                if let Some(place) = place {
                    rebuilt = self.intern_type(dir::Type::Form(dir::FormType {
                        form: dir::Form::Placed { place },
                        value: rebuilt,
                    }))?;
                }

                self.normalize_head(origin, rebuilt)
            }

            // reduce family-default ownership constructors anywhere in the form chain
            dir::Type::Form(_) => {
                let reduced = self.reduce_default_ownership_chain(origin, id)?;
                if reduced == id {
                    return Ok(id);
                }

                self.normalize_chain(origin, reduced, expanding)
            }

            // intersections merge their structural shape elements
            dir::Type::Intersection(intersection) => {
                let elements: SmallVec<[_; 4]> =
                    SmallVec::from_slice(self.type_ids(id.module_id, intersection.elements)?);
                let merged = self.reduce_intersection(origin, id, &elements)?;

                Ok(merged)
            }

            // return every other root unchanged, it is already simplest
            _ => Ok(id),
        }
    }

    /// Reduce forms that a borrow absorbs from its payload.
    fn reduce_borrow_payload(
        &mut self,
        mut value: dir::GlobalTypeId,
        mut access: dir::GlobalTypeId,
    ) -> CompilerResult<(
        dir::GlobalTypeId,
        dir::GlobalTypeId,
        Option<dir::GlobalTypeId>,
    )> {
        let mut place = None;

        // absorb each value form exposed by alias reduction
        loop {
            value = self.shallow_resolve(value)?;
            let dir::Type::Form(payload) = self.ty(value)? else {
                break;
            };
            match payload.form {
                // readonly payloads clamp the borrow access
                dir::Form::Readonly => {
                    access = self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Access(
                        dir::Access::Readonly,
                    )))?;
                    value = payload.value;
                }

                // borrowed payloads reborrow at the clamped access
                dir::Form::Borrowed(payload_borrow) => {
                    let payload_access = self.type_borrow(value.module_id, payload_borrow)?.access;
                    let payload_access = self.shallow_resolve(payload_access)?;
                    if matches!(
                        self.ty(payload_access)?,
                        dir::Type::Memory(dir::MemoryLiteral::Access(dir::Access::Readonly))
                    ) {
                        access = payload_access;
                    }
                    value = payload.value;
                }

                // look through ownership forms, which contribute storage
                dir::Form::Managed | dir::Form::Owned => value = payload.value,

                // placement qualifies the resulting borrow handle
                dir::Form::Placed { place: current } => {
                    place = Some(current);
                    value = payload.value;
                }

                // raw payloads keep their written form
                dir::Form::Raw => break,
            }
        }

        Ok((value, access, place))
    }

    /// Reduce one type graph with the active reduction path tracked.
    fn normalize_graph(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        memo: &mut FxIndexMap<dir::GlobalTypeId, dir::GlobalTypeId>,
        active: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // replay the reduction already recorded for this node
        if let Some(done) = memo.get(&id).copied() {
            return Ok(done);
        }

        // replay the reduction recorded for the settled head
        let original = id;
        let id = self.shallow_resolve(id)?;
        if let Some(done) = memo.get(&id).copied() {
            memo.insert(original, done);

            return Ok(done);
        }

        // stop at a node already on the active reduction path
        if !active.insert(id) {
            memo.insert(original, id);

            return Ok(id);
        }

        // reduce children first so rebuilt composite roots can reduce
        let mut replacements = FxIndexMap::default();
        let mut children = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        let root = self.ty(id)?;
        self.for_each_type_child(id.module_id, &root, |child| children.push(child))?;
        for child in children {
            let reduced = self.normalize_graph(origin, child, memo, active)?;
            if reduced != child {
                replacements.insert(child, reduced);
            }
        }

        // keep an unchanged local root as it stands, leaving computation
        //  heads to reduce once their operands close
        let target = origin.module();
        let is_union = matches!(root, dir::Type::Union(_));
        let is_computation = matches!(root, dir::Type::Operation(_));
        if replacements.is_empty() && id.module_id == target && !is_union && !is_computation {
            active.swap_remove(&id);
            memo.insert(original, id);

            return Ok(id);
        }

        // read payloads from their owner, intern the rebuilt type in this module
        let ty = self.ty(id)?;
        let ty = self.map_type_children(id.module_id, target, ty, &mut |_state, child| {
            Ok(replacements.get(&child).copied().unwrap_or(child))
        })?;
        let rebuilt = match ty {
            dir::Type::Union(union) => {
                let elements: SmallVec<[_; 8]> = self.type_ids(target, union.elements)?.into();

                self.normalized_union_type(elements)?
            }
            ty => self.intern_type(ty)?,
        };

        // reduce the rebuilt root once more when it moved
        active.swap_remove(&id);
        let rebuilt = if rebuilt == id {
            rebuilt
        } else {
            self.normalize_graph(origin, rebuilt, memo, active)?
        };

        // reduce a computation head once its operands close
        let rebuilt = match self.ty(rebuilt)? {
            dir::Type::Operation(operation) => {
                let operation = self.type_operation(rebuilt.module_id, operation)?;
                match self.reduce_operation(origin, rebuilt, &operation)? {
                    Some(reduced) => self.normalize_graph(origin, reduced, memo, active)?,
                    None => rebuilt,
                }
            }
            _ => rebuilt,
        };
        memo.insert(original, rebuilt);

        Ok(rebuilt)
    }

    /// Complete one under-applied application with its elided arguments.
    pub(in crate::sema) fn fill_elided_application(
        &mut self,
        module: ModuleId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(template) = self.symbol_template(instance.symbol)? else {
            return Ok(None);
        };
        let parameters = self.generic_template_parameters(template)?;
        let written = self.type_ids(module, instance.arguments)?.len();
        if written >= parameters.len() {
            return Ok(None);
        }

        let substitution = self.instance_substitution(module, instance)?;
        let arguments = substitution
            .bindings
            .iter()
            .map(|binding| binding.argument)
            .collect::<Vec<_>>();
        if arguments.len() != parameters.len() {
            return Ok(None);
        }
        let arguments = self.intern_type_ids(&arguments)?;
        let filled = self.intern_type(dir::Type::Application(dir::GenericApplication {
            symbol: instance.symbol,
            arguments,
        }))?;

        Ok(Some(filled))
    }

    /// Return the substituted body of one transparent type alias application.
    pub(super) fn type_alias_body(
        &mut self,
        _origin: Origin,
        instance_module: ModuleId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // expand transparent alias definitions only
        let value = {
            let Some(definition) = self.definition(instance.symbol)? else {
                return Ok(None);
            };
            let dir::Definition::TypeAlias(definition) = definition else {
                return Ok(None);
            };

            definition.value
        };

        let substitution = self.instance_substitution(instance_module, instance)?;

        // substitute applied arguments through the body
        let substituted = self.substitute_type(value, &substitution)?;

        Ok(Some(substituted))
    }
}
