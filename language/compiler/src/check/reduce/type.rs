use destack_core::{FxIndexMap, FxIndexSet, ensure_sufficient_stack};
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{CheckState, Origin};
use crate::{CompilerError, CompilerResult};

/// One head reduction outcome, keeping blocked heads visible.
#[derive(Debug, Clone, PartialEq, Eq)]
enum HeadReduction {
    /// The chain closed on one head normal form.
    Closed(dir::GlobalTypeId),
    /// The chain blocked at one head awaiting open dependencies.
    Blocked(dir::GlobalTypeId),
}

impl CheckState<'_> {
    /// Return the storage representation of one checked type.
    pub(in crate::check) fn storage_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // keep written types while declaring, checking selects storage
        if self.is_declaration() {
            return Ok(ty);
        }

        // track the aliases expanded along this chain
        let mut aliases = FxIndexSet::default();

        self.normalize_storage_type(origin, ty, &mut aliases)
    }

    /// Return the canonical checked form of one foreign written type.
    ///
    /// Declared modules record written types, so foreign symbol types canonicalize on entry:
    /// signature parameters store like walked parameter declarations and other values store whole.
    pub(in crate::check) fn canonical_foreign_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // keep written types while declaring, checking selects storage
        if self.is_declaration() {
            return Ok(ty);
        }

        // signatures canonicalize per parameter, other values canonicalize whole
        let origin = Origin::Symbol(symbol);
        match self.ty(ty)? {
            dir::Type::Function(function) => {
                let signature = self.canonical_foreign_signature(origin, function.signature)?;
                if signature == function.signature {
                    return Ok(ty);
                }

                self.intern_type(dir::Type::Function(dir::FunctionType {
                    signature,
                    multiplicity: function.multiplicity,
                }))
            }
            dir::Type::FunctionPointer(function) => {
                let signature = self.canonical_foreign_signature(origin, function.signature)?;
                if signature == function.signature {
                    return Ok(ty);
                }

                self.intern_type(dir::Type::FunctionPointer(dir::FunctionPointerType {
                    signature,
                }))
            }
            dir::Type::FunctionSignature(_) => self.canonical_foreign_signature(origin, ty),
            _ => self.storage_type(origin, ty),
        }
    }

    /// Store one foreign written signature's parameters like walked parameters.
    fn canonical_foreign_signature(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let dir::Type::FunctionSignature(id) = self.ty(ty)? else {
            return Ok(ty);
        };
        let signature = self.type_signature(ty.module_id, id)?;
        let parameters = self
            .signature_parameters(ty.module_id, signature.parameters)?
            .to_vec();

        // store each written parameter like a walked parameter declaration
        let mut stored = Vec::with_capacity(parameters.len());
        let mut changed = false;
        for parameter in parameters {
            let ty = self.storage_type(origin, parameter.ty)?;
            changed |= ty != parameter.ty;
            stored.push(dir::FunctionParameterType { ty, ..parameter });
        }
        if !changed {
            return Ok(ty);
        }

        let parameters = self.intern_parameters(&stored)?;

        self.intern_signature(dir::FunctionSignatureType {
            parameters,
            ..signature
        })
    }

    /// Normalize storage while expanding transparent aliases once per active chain.
    fn normalize_storage_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        aliases: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let ty = self.shallow_resolve(ty)?;

        // expand aliases only when their bodies require storage adaptation
        if self.is_alias_instance(ty)? {
            if !aliases.insert(ty) {
                self.report_circular_type(origin)?;

                return self.intern_type(dir::Type::Error);
            }
            let dir::Type::Application(instance) = self.ty(ty)? else {
                return Err(CompilerError::Internal {
                    message: format!("transparent storage alias {ty:?} is not an application"),
                });
            };
            let body = match self.type_alias_body(origin, ty.module_id, &instance)? {
                Some(body) => body,
                None => {
                    return Err(CompilerError::Internal {
                        message: format!("storage alias {ty:?} has no transparent body"),
                    });
                }
            };
            let storage = self.normalize_storage_type(origin, body, aliases)?;
            aliases.swap_remove(&ty);

            return Ok(if storage == body { ty } else { storage });
        }

        // erase constraint-only types behind a dynamic handle
        if self.is_dynamic_storage_constraint(ty)? {
            let dynamic =
                self.intern_type(dir::Type::Dynamic(dir::DynamicType { constraint: ty }))?;

            // preserve non-default nominal placement across erased storage
            let symbol = match self.ty(ty)? {
                dir::Type::Application(instance) => Some(instance.symbol),
                _ => None,
            };
            if let Some(symbol) = symbol
                && self.nominal_space(symbol)? == Some(dir::Space::Shared)
            {
                let place = self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Place(
                    dir::Place::Space(dir::Space::Shared),
                )))?;

                return self.placed_type(Origin::Symbol(symbol), dynamic, place);
            }

            return Ok(dynamic);
        }

        // normalize composite storage through its elements
        match self.ty(ty)? {
            dir::Type::Union(union) => {
                let elements = self.type_ids(ty.module_id, union.elements)?.to_vec();
                let mut normalized = Vec::with_capacity(elements.len());
                for element in elements {
                    normalized.push(self.normalize_storage_type(origin, element, aliases)?);
                }

                self.normalized_union_type(normalized)
            }
            dir::Type::Form(form) => {
                let value = self.normalize_storage_type(origin, form.value, aliases)?;
                if value == form.value {
                    return Ok(ty);
                }

                // adopt module-local borrow rows across the rebuild
                let adopted = self.adopt_form(ty.module_id, form.form)?;

                self.intern_type(dir::Type::Form(dir::FormType {
                    form: adopted,
                    value,
                }))
            }
            _ => Ok(ty),
        }
    }

    /// Return whether one type has no direct storage representation.
    fn is_dynamic_storage_constraint(&mut self, ty: dir::GlobalTypeId) -> CompilerResult<bool> {
        match self.ty(ty)? {
            // top types have no direct layout in storage
            dir::Type::Any | dir::Type::Unknown => Ok(true),

            // interface instances are constraints, not represented values
            dir::Type::Application(instance) => Ok(matches!(
                self.symbol_kind(instance.symbol)?,
                dir::SymbolKind::Interface | dir::SymbolKind::NewtypeInterface
            )),

            // accept every other source-built type, it already has a representation
            _ => Ok(false),
        }
    }

    /// Reduce one type graph to its simplest closed form.
    pub(in crate::check) fn reduce_type(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let id = self.shallow_resolve(id)?;

        // key parameter reductions by their assuming scope
        let scope = if self.type_flags(id)?.has_parameter() {
            self.assuming_scope(origin)?
        } else {
            None
        };
        if scope.is_none()
            && let Some(reduced) = self.module.types_tail.get_reduced_type_id(id)
        {
            return Ok(reduced);
        }

        // fold the root, then normalize children with aliases kept symbolic
        let mut memo = FxIndexMap::default();
        let mut active = FxIndexSet::default();
        let head = self.reduce_type_head(origin, id)?;
        let reduced = self.reduce_type_graph(origin, head, &mut memo, &mut active)?;

        // record complete reductions as type tail entries; each id reduces
        //  under its own declaration scope, so entries key by the id alone
        if !self.type_flags(id)?.has_variable() && !self.type_flags(reduced)?.has_variable() {
            self.module.types_tail.set_type_reduction(id, reduced);
        }

        Ok(reduced)
    }

    /// Splat one signature's closed tuple rest parameter into positional parameters.
    fn reduce_signature_rest_splat(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        signature: dir::FunctionSignatureType,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let parameters = self
            .signature_parameters(id.module_id, signature.parameters)?
            .to_vec();
        let Some((rest_index, rest)) = parameters
            .iter()
            .enumerate()
            .find(|(_, parameter)| parameter.is_rest)
        else {
            return Ok(None);
        };
        let rest_ty = self.reduce_type_head(origin, rest.ty)?;
        let dir::Type::Tuple(tuple) = self.ty(rest_ty)? else {
            return Ok(None);
        };

        // rebuild positional parameters from the tuple elements
        let mut rebuilt = parameters[..rest_index].to_vec();
        for element in self
            .tuple_elements(rest_ty.module_id, tuple.elements)?
            .to_vec()
        {
            rebuilt.push(dir::FunctionParameterType {
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

    /// Reduce the head of one type to an honest value form, keeping authored names.
    pub(in crate::check) fn reduce_named_head(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut id = self.shallow_resolve(id)?;
        loop {
            let reduced = match self.ty(id)? {
                // resolve meta heads: they name computations, not values
                dir::Type::Member(_) | dir::Type::Operation(_) => {
                    self.reduce_type_head(origin, id)?
                }
                // drop redundant forms while the payload keeps its spelling
                dir::Type::Form(_) => self.reduce_redundant_forms(origin, id)?,
                _ => return Ok(id),
            };
            if reduced == id {
                return Ok(id);
            }
            id = reduced;
        }
    }

    /// Reduce the head of one type to its simplest available form.
    pub(in crate::check) fn reduce_type_head(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // an open head reduces to itself: the variable is its own value form
        Ok(match self.head_reduction(origin, id)? {
            HeadReduction::Closed(reduced) => reduced,
            HeadReduction::Blocked(head) => head,
        })
    }

    /// Reduce one type head, keeping a blocked head visible with its openness.
    fn head_reduction(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<HeadReduction> {
        let id = self.shallow_resolve(id)?;

        // key parameter reductions by their assuming scope
        let scope = if self.type_flags(id)?.has_parameter() {
            self.assuming_scope(origin)?
        } else {
            None
        };

        // replay decided reductions of closed types
        if let Some(reduced) = self.reduces.get(&(id, scope)) {
            return Ok(HeadReduction::Closed(*reduced));
        }

        let mut expanding = FxIndexSet::default();
        let reduction = self.reduce_type_chain(origin, id, &mut expanding)?;

        // decide closed reductions once, identity included: loads run
        //  synchronously inside the chain, so a closed head's reduction
        //  is final the moment it completes
        if let HeadReduction::Closed(reduced) = reduction
            && !self.type_flags(id)?.has_variable()
            && !self.type_flags(reduced)?.has_variable()
        {
            self.reduces.insert((id, scope), reduced);
        }

        Ok(reduction)
    }

    /// Reduce one settled type head with the active expansion chain tracked.
    fn reduce_type_chain(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        expanding: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<HeadReduction> {
        ensure_sufficient_stack(|| self.reduce_type_chain_recursive(origin, id, expanding))
    }

    /// Reduce one type chain on the grown stack.
    fn reduce_type_chain_recursive(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        expanding: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<HeadReduction> {
        // report circular expansions and complete the chain with the error type
        if !expanding.insert(id) {
            self.report_circular_type(origin)?;
            let error = self.intern_type(dir::Type::Error)?;

            return Ok(HeadReduction::Closed(error));
        }

        match self.ty(id)? {
            // open variables block the chain as its own head
            dir::Type::Variable(_) => Ok(HeadReduction::Blocked(id)),

            // unions normalize their canonical elements in place
            dir::Type::Union(union) => {
                let elements = self.type_ids(id.module_id, union.elements)?.to_vec();
                let normalized = self.normalized_union_type(elements)?;
                if normalized == id {
                    return Ok(HeadReduction::Closed(id));
                }

                self.reduce_type_chain_recursive(origin, normalized, expanding)
            }

            // rest parameters with closed tuple types splat positionally
            dir::Type::FunctionSignature(signature) => {
                let signature = self.type_signature(id.module_id, signature)?;
                match self.reduce_signature_rest_splat(origin, id, signature)? {
                    Some(splatted) => Ok(HeadReduction::Closed(splatted)),
                    None => Ok(HeadReduction::Closed(id)),
                }
            }

            // written applications complete their elided arguments
            dir::Type::Application(instance) => {
                if let Some(filled) = self.fill_elided_application(id.module_id, &instance)? {
                    return self.head_reduction(origin, filled);
                }

                // reduce intrinsic references to their builtin forms
                if let Some(reduced) =
                    self.reduce_intrinsic_reference(origin, id.module_id, &instance)?
                {
                    return self.head_reduction(origin, reduced);
                }

                match self.type_alias_body(origin, id.module_id, &instance)? {
                    Some(value) => {
                        let value = self.shallow_resolve(value)?;

                        self.reduce_type_chain(origin, value, expanding)
                    }
                    None => Ok(HeadReduction::Closed(id)),
                }
            }

            // member projections resolve through their owners
            dir::Type::Member(member) => {
                let member = self.type_member(id.module_id, member)?;
                // close the owner and keep its memory form
                let owner = self.reduce_type_head(origin, member.owner)?;
                let peeled = self.strip_form(origin, owner)?;

                // error owners poison their projections
                if matches!(self.ty(peeled)?, dir::Type::Error) {
                    let error = self.intern_type(dir::Type::Error)?;

                    return Ok(HeadReduction::Closed(error));
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

                    return self.reduce_type_chain(origin, rebuilt, expanding);
                }

                let projection = self.body().project_member(origin, &member)?;
                let Some(projected) = projection else {
                    return Ok(HeadReduction::Closed(id));
                };
                let projected = self.shallow_resolve(projected)?;

                self.reduce_type_chain(origin, projected, expanding)
            }

            // reduce type operations once their inputs close
            dir::Type::Operation(operation) => {
                let operation = self.type_operation(id.module_id, operation)?;
                let reduction = self.reduce_operation(origin, id, &operation)?;

                let Some(reduced) = reduction else {
                    return Ok(HeadReduction::Closed(id));
                };
                let reduced = self.shallow_resolve(reduced)?;

                self.reduce_type_chain(origin, reduced, expanding)
            }

            // borrows absorb payload forms and retain placement around the resulting handle
            dir::Type::Form(form) if let dir::Form::Borrowed(borrow) = form.form => {
                let borrow = self.type_borrow(id.module_id, borrow)?;

                // close the lifetime and access components
                let closed_lifetime = self.reduce_type_head(origin, borrow.lifetime)?;
                let closed_access = self.reduce_type_head(origin, borrow.access)?;
                let value = self.reduce_type_head(origin, form.value)?;

                // absorb the payload forms the borrow carries itself
                let (payload, closed_access, place) =
                    self.reduce_borrow_payload(origin, value, closed_access)?;
                if payload == form.value
                    && closed_lifetime == borrow.lifetime
                    && closed_access == borrow.access
                    && place.is_none()
                {
                    return Ok(HeadReduction::Closed(id));
                }

                let closed_form = self.intern_borrow(closed_lifetime, closed_access)?;
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

                self.head_reduction(origin, rebuilt)
            }

            // non-borrow forms close their payload head
            dir::Type::Form(form) => {
                let value = self.reduce_type_head(origin, form.value)?;

                // redundant wrappers reduce to their payload
                if self.is_redundant_form(origin, form.form, value)? {
                    return self.reduce_type_chain(origin, value, expanding);
                }

                if value == form.value {
                    return Ok(HeadReduction::Closed(id));
                }

                let rebuilt = self.intern_type(dir::Type::Form(dir::FormType {
                    form: form.form,
                    value,
                }))?;

                self.head_reduction(origin, rebuilt)
            }

            // intersections merge their structural shape elements
            dir::Type::Intersection(intersection) => {
                let elements: SmallVec<[_; 4]> =
                    SmallVec::from_slice(self.type_ids(id.module_id, intersection.elements)?);
                let merged = self.reduce_intersection(origin, id, &elements)?;

                Ok(HeadReduction::Closed(merged))
            }

            // return every other root unchanged, it is already simplest
            _ => Ok(HeadReduction::Closed(id)),
        }
    }

    /// Reduce forms that a borrow absorbs from its payload.
    fn reduce_borrow_payload(
        &mut self,
        _origin: Origin,
        mut value: dir::GlobalTypeId,
        mut access: dir::GlobalTypeId,
    ) -> CompilerResult<(
        dir::GlobalTypeId,
        dir::GlobalTypeId,
        Option<dir::GlobalTypeId>,
    )> {
        let mut place = None;

        // absorb each value form exposed by alias reduction
        while let dir::Type::Form(payload) = self.ty(value)? {
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

                // ownership forms contribute storage rather than another handle layer
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
    fn reduce_type_graph(
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

        let original = id;

        // keep transparent alias references symbolic in child positions
        let id = self.shallow_resolve(id)?;
        let id = match self.is_alias_instance(id)? {
            true => id,
            false => self.reduce_type_head(origin, id)?,
        };
        if let Some(done) = memo.get(&id).copied() {
            memo.insert(original, done);

            return Ok(done);
        }
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
            let reduced = self.reduce_type_graph(origin, child, memo, active)?;
            if reduced != child {
                replacements.insert(child, reduced);
            }
        }
        // keep an unchanged local root as it stands
        let target = origin.module();
        let is_union = matches!(root, dir::Type::Union(_));
        if replacements.is_empty() && id.module_id == target && !is_union {
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
                let elements = self.type_ids(target, union.elements)?.to_vec();

                self.normalized_union_type(elements)?
            }
            ty => self.intern_type(ty)?,
        };
        // reduce the rebuilt root once more when it moved
        active.swap_remove(&id);
        let rebuilt = if rebuilt == id {
            rebuilt
        } else {
            self.reduce_type_graph(origin, rebuilt, memo, active)?
        };
        memo.insert(original, rebuilt);

        Ok(rebuilt)
    }

    /// Return whether one type is a transparent alias application.
    fn is_alias_instance(&mut self, id: dir::GlobalTypeId) -> CompilerResult<bool> {
        let dir::Type::Application(instance) = self.ty(id)? else {
            return Ok(false);
        };
        if !matches!(
            self.definition(instance.symbol)?,
            Some(dir::Definition::TypeAlias(_))
        ) {
            return Ok(false);
        }

        Ok(self.language_item(instance.symbol)?.is_none())
    }

    /// Complete one under-applied application with its elided arguments.
    pub(in crate::check) fn fill_elided_application(
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

        let substitution = self.instance_substitution(module, &instance)?;
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
    fn type_alias_body(
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
