use destack_core::{FxIndexMap, FxIndexSet, ensure_sufficient_stack};
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::sema::{CheckState, Origin};

/// Settled projections shared across nodes, keyed by their assuming scope.
pub(in crate::sema) type ProjectionMemo =
    FxIndexMap<(Option<dir::GlobalGenericTemplateId>, dir::GlobalTypeId), dir::GlobalTypeId>;

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
            let definition = self.definition(instance.symbol)?;
            let Some(dir::Definition::TypeAlias(alias)) = definition.as_deref() else {
                break;
            };
            let body = alias.value;
            ty = self.shallow_resolve(body)?;
        }

        // read the constraint each erased head names
        match self.ty(ty)? {
            // explicit erasure names its constraint
            dir::Type::Dynamic(dynamic) => Ok(Some(dynamic.constraint)),

            // carry an unknown dynamic payload directly
            dir::Type::Unknown => Ok(Some(ty)),

            // erase an object shape that declares index signatures
            dir::Type::Object(shape)
                if !self
                    .object_index_signatures(ty.module_id, shape.index_signatures)?
                    .is_empty() =>
            {
                Ok(Some(ty))
            }

            // interface-typed values erase behind their constraint
            dir::Type::Application(instance) => Ok(matches!(
                self.symbol_kind(instance.symbol)?,
                dir::SymbolKind::Interface | dir::SymbolKind::NewtypeInterface
            )
            .then_some(ty)),

            // keep every other value type at its representation
            _ => Ok(None),
        }
    }

    /// Settle solved variables through one type graph, keeping open holes.
    pub(in crate::sema) fn deeply_resolve(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut shared = FxIndexMap::default();

        self.deeply_resolve_shared(origin, id, &mut shared)
    }

    /// Settle solved variables through one type graph, reusing scoped heads.
    pub(in crate::sema) fn deeply_resolve_shared(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        shared: &mut FxIndexMap<
            (Option<dir::GlobalGenericTemplateId>, dir::GlobalTypeId),
            dir::GlobalTypeId,
        >,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // reuse the root this scope already resolved
        let id = self.shallow_resolve(id)?;
        let scope = self.origin_scope(origin)?;
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
    pub(in crate::sema) fn reduce_tuple_rest_splice(
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
        let Some(spread) = self.rest_tuple_elements(origin, elements[rest_index].ty)? else {
            return Ok(None);
        };

        // splice the spread elements in place
        let mut spliced: SmallVec<[_; 4]> = elements[..rest_index].into();
        spliced.extend(spread);
        spliced.extend(elements[rest_index + 1..].iter().copied());
        let spliced = self.intern_elements(&spliced)?;
        let spliced = self.intern_type(dir::Type::Tuple(dir::TupleType {
            form: tuple.form,
            elements: spliced,
        }))?;

        Ok(Some(spliced))
    }

    /// Spread one signature's closed tuple rest parameter into positional parameters.
    fn reduce_signature_rest_spread(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        signature: dir::FunctionSignatureType,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // locate the rest parameter among the written positions
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

        // read the spread as a closed tuple
        let Some(elements) = self.rest_tuple_elements(origin, rest.ty)? else {
            return Ok(None);
        };

        // rebuild positional parameters from the spread elements
        let mut rebuilt: SmallVec<[_; 4]> = parameters[..rest_index].into();
        for element in elements {
            rebuilt.push(dir::FunctionParameterType {
                name: None,
                ty: element.ty,
                is_optional: element.is_optional,
                is_rest: element.is_rest,
            });
        }
        rebuilt.extend(parameters[rest_index + 1..].iter().copied());

        // intern the spread signature
        let parameters = self.intern_parameters(&rebuilt)?;
        let spread = self.intern_signature(dir::FunctionSignatureType {
            parks: signature.parks,
            parameters,
            ..signature
        })?;

        Ok(Some(spread))
    }

    /// Return the elements one rest parameter spreads, when its head settles to a closed tuple.
    fn rest_tuple_elements(
        &mut self,
        origin: Origin,
        rest: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SmallVec<[dir::TypeElement; 4]>>> {
        let rest = self.structurally_normalize(origin, rest)?;
        let dir::Type::Tuple(tuple) = self.ty(rest)? else {
            return Ok(None);
        };
        let elements = self.tuple_elements(rest.module_id, tuple.elements)?.into();

        Ok(Some(elements))
    }

    /// Normalize one closed head once at construction.
    pub(in crate::sema) fn normalize_closed(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // keep the reductions of open heads lazy under their assuming scope
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
        // name the declaration owning the head
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
        // reduce by the head the type carries
        match self.ty(id)? {
            // reduce meta heads, which name computations
            dir::Type::Member(_) | dir::Type::Operation(_) => self.normalize(origin, id),
            // reduce memory accessor applications, which name computations over forms
            dir::Type::Application(instance)
                if self
                    .language_item(instance.symbol)?
                    .is_some_and(|item| matches!(item, dir::LanguageItem::WithAccess)) =>
            {
                self.normalize(origin, id)
            }
            // keep every other head as written
            _ => Ok(id),
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

    /// Return whether one normalized head stays stuck on a variable.
    pub(in crate::sema) fn is_stuck_head(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        Ok(match self.ty(id)? {
            dir::Type::Variable(_)
            | dir::Type::Parameter(_)
            | dir::Type::Erased(_)
            | dir::Type::This
            | dir::Type::Operation(_)
            | dir::Type::Member(_) => true,
            // an intrinsic alias its arguments leave unreduced, or a bare generic parameter name
            dir::Type::Application(instance) => {
                (self.is_intrinsic_alias(instance.symbol)?
                    && self
                        .reduce_intrinsic_reference(origin, id.module_id, &instance)?
                        .is_none())
                    || instance.arguments.is_empty()
                        && self.symbol_kind(instance.symbol)?
                            == dir::SymbolKind::GenericTypeParameter
            }
            _ => false,
        })
    }

    /// Normalize the head of one type to its simplest available form, memoizing closed heads.
    pub(in crate::sema) fn normalize(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.counters.reduces += 1;
        let id = self.shallow_resolve(id)?;
        let flags = self.type_flags(id)?;

        // reduce parameter and This heads under their assuming template
        let assumes = self.decision_scope(origin, &[id])?;

        // reuse decided reductions
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
            // store the fixed point of a resolved head, skipping the declaring pass
            if let Some(assumes) = assumes
                && !flags.has_variable()
                && !self.is_declaring()
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
            && !self.is_declaring()
        {
            self.normalizations.insert((id, assumes), reduced);
        }

        Ok(reduced)
    }

    /// Reduce one resolved type head with the active expansion chain tracked.
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

        let reduced = self.normalize_chain_step(origin, id, expanding);
        expanding.swap_remove(&id);

        reduced
    }

    /// Reduce one type head with the head held on the active expansion path.
    fn normalize_chain_step(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        expanding: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // read the head through its solution
        let id = self.shallow_resolve(id)?;

        // follow the chain by the head the type carries
        match self.ty(id)? {
            // block the chain on an open variable
            dir::Type::Variable(_) => Ok(id),

            // normalize a union's canonical elements in place
            dir::Type::Union(union) => {
                let elements: SmallVec<[_; 8]> =
                    self.type_ids(id.module_id, union.elements)?.into();
                let mut reduced = SmallVec::<[_; 8]>::with_capacity(elements.len());
                for element in elements {
                    reduced.push(self.normalize(origin, element)?);
                }
                let normalized = self.normalized_union_type(reduced)?;
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

            // spread a closed tuple rest parameter positionally
            dir::Type::FunctionSignature(signature) => {
                let signature = self.type_signature(id.module_id, signature)?;
                match self.reduce_signature_rest_spread(origin, id, signature)? {
                    Some(spread) => Ok(spread),
                    None => Ok(id),
                }
            }

            // complete the elided arguments of a written application
            dir::Type::Application(instance) => {
                if let Some(filled) = self.fill_elided_application(id.module_id, &instance)? {
                    return self.normalize(origin, filled);
                }

                // reduce nominal declarations standing for builtin types
                if let Some(reduced) =
                    self.reduce_representation_declaration(origin, id.module_id, &instance)?
                {
                    return self.normalize(origin, reduced);
                }

                match self.type_alias_body(origin, id.module_id, &instance)? {
                    Some(value) => {
                        let value = self.shallow_resolve(value)?;

                        self.normalize_chain(origin, value, expanding)
                    }
                    None => Ok(id),
                }
            }

            // resolve a member projection through its owner
            dir::Type::Member(member) => {
                let member = self.type_member(id.module_id, member)?;

                // read the owner whole, its forms shed for an unqualified projection
                let owner = self.shallow_resolve(member.owner)?;
                let peeled = self.strip_form(origin, owner)?;

                // error owners poison their projections
                if matches!(self.ty(peeled)?, dir::Type::Error) {
                    let error = self.intern_type(dir::Type::Error)?;

                    return Ok(error);
                }

                // select one declaring interface for an unqualified projection
                let mut qualifier = member.qualifier;
                if qualifier.is_none() {
                    qualifier = self.select_associated_qualifier(origin, peeled, member.key)?;
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

                // continue the chain through the projected type
                let projection = self.project_member(origin, &member)?;
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

            // rebuild a borrow over the payload forms it absorbs
            dir::Type::Form(form) if let dir::Form::Borrowed(borrow) = form.form => {
                let borrow = self.type_borrow(id.module_id, borrow)?;
                let rebuilt = self.borrow_value(borrow.region, borrow.access, form.value)?;
                if rebuilt == id {
                    return Ok(id);
                }

                self.normalize(origin, rebuilt)
            }

            // reduce family-default ownership constructors anywhere in the form chain
            dir::Type::Form(_) => {
                let reduced = self.reduce_default_ownership_chain(origin, id)?;
                if reduced == id {
                    return Ok(id);
                }

                self.normalize_chain(origin, reduced, expanding)
            }

            // merge the structural shape elements of an intersection
            dir::Type::Intersection(intersection) => {
                let elements: SmallVec<[_; 4]> =
                    SmallVec::from_slice(self.type_ids(id.module_id, intersection.elements)?);
                let merged = self.reduce_intersection(origin, id, &elements)?;

                Ok(merged)
            }

            // return every other root unchanged, already at its simplest
            _ => Ok(id),
        }
    }

    /// Reduce one type graph with the active reduction path tracked.
    fn normalize_graph(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        memo: &mut FxIndexMap<dir::GlobalTypeId, dir::GlobalTypeId>,
        active: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // reuse the reduction already recorded for this node
        if let Some(done) = memo.get(&id).copied() {
            return Ok(done);
        }

        // reuse the reduction recorded for the resolved head
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

        // keep an unchanged local root as it stands
        let target = self.module_id;
        let is_union = matches!(root, dir::Type::Union(_));
        let is_computation = matches!(root, dir::Type::Operation(_) | dir::Type::Member(_))
            || self.is_closed_intrinsic_application(id)?
            || self.is_redundant_owned_form(id)?;
        if replacements.is_empty() && id.module_id == target && !is_union && !is_computation {
            active.swap_remove(&id);
            memo.insert(original, id);

            return Ok(id);
        }

        // read payloads from their owner, intern the rebuilt type in this module
        let ty = self.ty(id)?;
        let ty = self.map_type_children(id.module_id, ty, &mut |_state, child| {
            Ok(replacements.get(&child).copied().unwrap_or(child))
        })?;
        // rebuild the head over its replaced children
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
        let is_renormalized = matches!(self.ty(rebuilt)?, dir::Type::Member(_))
            || self.is_redundant_owned_form(rebuilt)?;
        let rebuilt = match self.ty(rebuilt)? {
            dir::Type::Operation(operation) => {
                let operation = self.type_operation(rebuilt.module_id, operation)?;
                match self.reduce_operation(origin, rebuilt, &operation)? {
                    Some(reduced) => self.normalize_graph(origin, reduced, memo, active)?,
                    None => rebuilt,
                }
            }
            dir::Type::Application(instance)
                if self.is_closed_intrinsic_application(rebuilt)? =>
            {
                match self.reduce_intrinsic_reference(origin, rebuilt.module_id, &instance)? {
                    Some(reduced) => self.normalize_graph(origin, reduced, memo, active)?,
                    None => rebuilt,
                }
            }
            // reduce a projection or a redundant owned form one normalize step further
            _ if is_renormalized => {
                let reduced = self.normalize(origin, rebuilt)?;
                match reduced == rebuilt {
                    true => rebuilt,
                    false => self.normalize_graph(origin, reduced, memo, active)?,
                }
            }
            _ => rebuilt,
        };

        memo.insert(original, rebuilt);

        Ok(rebuilt)
    }

    /// Reduce the member projections one type holds once their owners closed.
    pub(in crate::sema) fn settle_projections(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        memo: &mut ProjectionMemo,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // share settled types across nodes of one assuming scope
        let scope = self.origin_scope(origin)?;
        if let Some(done) = memo.get(&(scope, id)).copied() {
            return Ok(done);
        }
        let resolved = self.shallow_resolve(id)?;
        memo.insert((scope, id), resolved);

        let settled = match self.ty(resolved)? {
            dir::Type::Member(_) => self.normalize(origin, resolved)?,
            dir::Type::Application(_) if self.is_closed_intrinsic_application(resolved)? => {
                self.normalize(origin, resolved)?
            }
            head => {
                let mapped =
                    self.map_type_children(resolved.module_id, head, &mut |state, child| {
                        state.settle_projections(origin, child, memo)
                    })?;
                match mapped == head {
                    true => resolved,
                    false => self.intern_type(mapped)?,
                }
            }
        };
        memo.insert((scope, id), settled);

        Ok(settled)
    }

    /// Complete one under-applied application with its elided arguments, once per application.
    pub(in crate::sema) fn fill_elided_application(
        &mut self,
        module: ModuleId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let instance_key = (module, *instance);
        if let Some(filled) = self.infer.filled_applications.get(&instance_key) {
            return Ok(Some(*filled));
        }
        let filled = self.complete_elided_application(module, instance)?;
        if let Some(filled) = filled {
            self.infer.filled_applications.insert(instance_key, filled);
        }

        Ok(filled)
    }

    /// Complete one under-applied application with the arguments its template elides.
    fn complete_elided_application(
        &mut self,
        module: ModuleId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // fill only an application written short of its parameters
        let Some(template) = self.symbol_template(instance.symbol)? else {
            return Ok(None);
        };
        let parameters = self.generic_template_parameters(template)?;
        let written = self.type_ids(module, instance.arguments)?.len();
        if written >= parameters.len() {
            return Ok(None);
        }

        // take the elided arguments from the instance substitution
        let substitution = self.instance_substitution(module, instance)?;
        let arguments = substitution
            .bindings
            .iter()
            .map(|binding| binding.argument)
            .collect::<Vec<_>>();
        if arguments.len() != parameters.len() {
            return Ok(None);
        }

        // apply the name with its completed arguments
        let arguments = self.intern_type_ids(&arguments)?;
        let filled = self.intern_type(dir::Type::Application(dir::GenericApplication {
            symbol: instance.symbol,
            arguments,
        }))?;

        Ok(Some(filled))
    }

    /// Return the substituted body of one transparent alias application.
    pub(in crate::sema) fn type_alias_body(
        &mut self,
        origin: Origin,
        instance_module: ModuleId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // expand transparent alias definitions only
        let value = {
            let Some(definition) = self.definition(instance.symbol)? else {
                return Ok(None);
            };
            let dir::Definition::TypeAlias(definition) = &*definition else {
                return Ok(None);
            };

            definition.value
        };
        if matches!(self.ty(value)?, dir::Type::Intrinsic) {
            return self.reduce_intrinsic_reference(origin, instance_module, instance);
        }

        // apply the instance arguments to the alias body
        let substitution = self.instance_substitution(instance_module, instance)?;

        // substitute applied arguments through the body
        let substituted = self.substitute_type(value, &substitution)?;

        Ok(Some(substituted))
    }

    /// Return whether one head is an owned form its value already defaults to.
    pub(in crate::sema) fn is_redundant_owned_form(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let dir::Type::Form(form) = self.ty(id)? else {
            return Ok(false);
        };
        if !matches!(form.form, dir::Form::Owned) {
            return Ok(false);
        }
        let value = self.shallow_resolve(form.value)?;
        if matches!(self.ty(value)?, dir::Type::Form(_)) {
            return Ok(false);
        }

        Ok(self.ownership(value)? == Some(dir::Ownership::Owned))
    }

    /// Return whether one head applies an intrinsic alias over closed arguments.
    fn is_closed_intrinsic_application(&mut self, id: dir::GlobalTypeId) -> CompilerResult<bool> {
        let dir::Type::Application(instance) = self.ty(id)? else {
            return Ok(false);
        };
        let flags = self.type_flags(id)?;
        if flags.has_this() || flags.has_type_parameter() || flags.has_variable() {
            return Ok(false);
        }

        self.is_intrinsic_alias(instance.symbol)
    }

    /// Return whether one declared name is an alias whose body is the intrinsic type.
    pub(in crate::sema) fn is_intrinsic_alias(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        let value = match self.definition(symbol)?.as_deref() {
            Some(dir::Definition::TypeAlias(alias)) => alias.value,
            _ => return Ok(false),
        };

        Ok(matches!(self.ty(value)?, dir::Type::Intrinsic))
    }

    /// Return the substituted backing of one newtype application, the storage its values share.
    pub(in crate::sema) fn newtype_backing_body(
        &mut self,
        instance_module: ModuleId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let declared = self.definition(instance.symbol)?;
        let Some(dir::Definition::Newtype(definition)) = declared.as_deref() else {
            return Ok(None);
        };
        let backing = definition.backing;
        let substitution = self.instance_substitution(instance_module, instance)?;

        Ok(Some(self.substitute_type(backing, &substitution)?))
    }
}
