use destack_core::{FxIndexMap, FxIndexSet, ensure_sufficient_stack};
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{Answer, CheckState, Dependency, Origin, answer};
use crate::{CompilerError, CompilerResult};

/// One head reduction outcome, keeping blocked heads visible.
#[derive(Debug, Clone, PartialEq, Eq)]
enum HeadReduction {
    /// The chain closed on one head normal form.
    Closed(dir::GlobalTypeId),
    /// The chain blocked at one head awaiting open dependencies.
    Blocked(dir::GlobalTypeId, SmallVec<[Dependency; 2]>),
}

/// Unwrap one inner reduction, reporting the blocked head on open dependencies.
macro_rules! blocked {
    ($id:expr, $answer:expr) => {
        match $answer {
            Answer::Ready(value) => value,
            Answer::Pending(dependencies) => {
                return Ok(HeadReduction::Blocked($id, dependencies));
            }
        }
    };
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
        if self.is_declaration() {
            return Ok(ty);
        }

        let origin = Origin::Symbol(symbol);
        match self.ty(ty)? {
            dir::Type::Function(function) => {
                let signature = self.canonical_foreign_signature(origin, function.signature)?;
                if signature == function.signature {
                    return Ok(ty);
                }

                self.intern_type(dir::Type::Function(dir::FunctionType {
                    signature,
                    environment: function.environment,
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
        let ty = self.settled_root(ty)?;

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
                Answer::Ready(Some(body)) => body,
                Answer::Ready(None) => {
                    return Err(CompilerError::Internal {
                        message: format!("storage alias {ty:?} has no transparent body"),
                    });
                }
                Answer::Pending(blockers) => {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "storage alias {ty:?} depends on open inference state {blockers:?}"
                        ),
                    });
                }
            };
            let storage = self.normalize_storage_type(origin, body, aliases)?;
            aliases.swap_remove(&ty);

            return Ok(if storage == body { ty } else { storage });
        }

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

    /// Reduce one type graph to its simplest available form.
    pub(in crate::check) fn reduce_type(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let id = self.settled_root(id)?;

        // key parameter reductions by their assuming scope
        let scope = match self.type_flags(id)?.has_parameter() {
            true => self.origin_scope(origin)?,
            false => None,
        };
        if let Some(reduced) = self.reduced_graphs.get(&(id, scope)) {
            return Ok(Answer::Ready(*reduced));
        }

        let mut memo = FxIndexMap::default();
        let mut active = FxIndexSet::default();

        // fold the root, then normalize children with aliases kept symbolic
        let reduced = answer!(self.reduce_type_head(origin, id)?);
        let answer = self.reduce_type_graph(origin, reduced, &mut memo, &mut active)?;

        // memoize complete closed reductions
        if let Answer::Ready(reduced) = answer
            && self.type_variables(id)?.is_empty()
            && self.type_variables(reduced)?.is_empty()
        {
            self.reduced_graphs.insert((id, scope), reduced);
        }

        Ok(answer)
    }

    /// Splat one signature's closed tuple rest parameter into positional parameters.
    fn reduce_signature_rest_splat(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        signature: dir::FunctionSignatureType,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let parameters = self
            .signature_parameters(id.module_id, signature.parameters)?
            .to_vec();
        let Some((rest_index, rest)) = parameters
            .iter()
            .enumerate()
            .find(|(_, parameter)| parameter.is_rest)
        else {
            return Ok(Answer::Ready(None));
        };
        let rest_ty = answer!(self.reduce_type_head(origin, rest.ty)?);
        let dir::Type::Tuple(tuple) = self.ty(rest_ty)? else {
            return Ok(Answer::Ready(None));
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
        let _module = id.module_id;
        let parameters = self.intern_parameters(&rebuilt)?;
        let splatted = self.intern_signature(dir::FunctionSignatureType {
            parameters,
            ..signature
        })?;

        Ok(Answer::Ready(Some(splatted)))
    }

    /// Reduce the head of one type to an honest value form, keeping authored names.
    pub(in crate::check) fn reduce_named_head(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let mut id = self.settled_root(id)?;
        loop {
            let reduced = match self.ty(id)? {
                // resolve meta heads: they name computations, not values
                dir::Type::Member(_) | dir::Type::Operation(_) => {
                    answer!(self.reduce_type_head(origin, id)?)
                }
                // drop redundant forms while the payload keeps its spelling
                dir::Type::Form(_) => answer!(self.reduce_redundant_forms(origin, id)?),
                _ => return Ok(Answer::Ready(id)),
            };
            if reduced == id {
                return Ok(Answer::Ready(id));
            }
            id = reduced;
        }
    }

    /// Reduce the head of one type to its simplest available form.
    pub(in crate::check) fn reduce_type_head(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        Ok(match self.head_reduction(origin, id)? {
            HeadReduction::Closed(reduced) => Answer::Ready(reduced),
            HeadReduction::Blocked(_, dependencies) => Answer::Pending(dependencies),
        })
    }

    /// Return the apparent head of one type under the current solutions.
    pub(in crate::check) fn apparent_head(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        Ok(match self.head_reduction(origin, id)? {
            HeadReduction::Closed(head) | HeadReduction::Blocked(head, _) => head,
        })
    }

    /// Reduce one type head, keeping a blocked head visible with its openness.
    fn head_reduction(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<HeadReduction> {
        let id = self.settled_root(id)?;

        // key parameter reductions by their assuming scope
        let scope = match self.type_flags(id)?.has_parameter() {
            true => self.origin_scope(origin)?,
            false => None,
        };

        // replay memoized closed reductions
        if let Some(reduced) = self.reduced_heads.get(&(id, scope)) {
            return Ok(HeadReduction::Closed(*reduced));
        }

        let mut expanding = FxIndexSet::default();
        let reduction = self.reduce_type_chain(origin, id, &mut expanding)?;

        // memoize closed reductions
        if let HeadReduction::Closed(reduced) = reduction
            && self.type_variables(id)?.is_empty()
            && self.type_variables(reduced)?.is_empty()
        {
            self.reduced_heads.insert((id, scope), reduced);
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
            let _module = origin.module();
            let error = self.intern_type(dir::Type::Error)?;

            return Ok(HeadReduction::Closed(error));
        }

        match self.ty(id)? {
            // open variables block the chain as its own head
            dir::Type::Variable(variable) => Ok(HeadReduction::Blocked(
                id,
                SmallVec::from_iter([Dependency::Variable(variable)]),
            )),
            // rest parameters with closed tuple types splat positionally
            dir::Type::FunctionSignature(signature) => {
                let signature = self.type_signature(id.module_id, signature)?;
                match blocked!(id, self.reduce_signature_rest_splat(origin, id, signature)?) {
                    Some(splatted) => Ok(HeadReduction::Closed(splatted)),
                    None => Ok(HeadReduction::Closed(id)),
                }
            }

            // transparent alias references expand to their substituted bodies
            dir::Type::Application(instance) => {
                // close foreign applications unreduced while declaring
                if self.is_declaration() && !self.is_own_module(instance.symbol.module_id) {
                    return Ok(HeadReduction::Closed(id));
                }

                // written applications complete their elided arguments
                if !self.is_declaration()
                    && let Some(filled) = self.fill_elided_application(id.module_id, &instance)?
                {
                    return self.head_reduction(origin, filled);
                }

                // reduce intrinsic references to their builtin forms
                if let Some(reduced) = blocked!(
                    id,
                    self.reduce_intrinsic_reference(origin, id.module_id, &instance)?
                ) {
                    return self.head_reduction(origin, reduced);
                }

                match blocked!(id, self.type_alias_body(origin, id.module_id, &instance)?) {
                    Some(value) => {
                        let value = self.settled_root(value)?;

                        self.reduce_type_chain(origin, value, expanding)
                    }
                    None => Ok(HeadReduction::Closed(id)),
                }
            }

            // member projections resolve through their owners
            dir::Type::Member(member) => {
                let member = self.type_member(id.module_id, member)?;
                // members live beneath memory forms, so owners shed them
                let owner = blocked!(id, self.strip_form(origin, member.owner)?);
                // unqualified projections select one declaring interface
                let mut qualifier = member.qualifier;
                if qualifier.is_none() {
                    qualifier = blocked!(
                        id,
                        self.body()
                            .projection_qualifier(origin, owner, member.key,)?
                    );
                }
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
                let Some(projected) = blocked!(id, projection) else {
                    return Ok(HeadReduction::Closed(id));
                };
                let projected = self.settled_root(projected)?;

                self.reduce_type_chain(origin, projected, expanding)
            }

            // reduce type operations once their inputs close
            dir::Type::Operation(operation) => {
                let operation = self.type_operation(id.module_id, operation)?;
                let reduction = self.reduce_operation(origin, id, &operation)?;

                let Some(reduced) = blocked!(id, reduction) else {
                    return Ok(HeadReduction::Closed(id));
                };
                let reduced = self.settled_root(reduced)?;

                self.reduce_type_chain(origin, reduced, expanding)
            }

            // borrows absorb payload forms and retain placement around the resulting handle
            dir::Type::Form(form) if let dir::Form::Borrowed(borrow) = form.form => {
                let borrow = self.type_borrow(id.module_id, borrow)?;

                // close the lifetime and access components
                let closed_lifetime = match self.reduce_type_head(origin, borrow.lifetime)? {
                    Answer::Ready(closed) => closed,
                    Answer::Pending(_) => borrow.lifetime,
                };
                let closed_access = match self.reduce_type_head(origin, borrow.access)? {
                    Answer::Ready(closed) => closed,
                    Answer::Pending(_) => borrow.access,
                };

                let value = match self.reduce_type_head(origin, form.value)? {
                    Answer::Ready(value) => value,
                    // open payloads stay structural until they close
                    Answer::Pending(_) => return Ok(HeadReduction::Closed(id)),
                };
                let (inner, closed_access, place) =
                    self.reduce_borrow_payload(origin, value, closed_access)?;
                if inner == form.value
                    && closed_lifetime == borrow.lifetime
                    && closed_access == borrow.access
                    && place.is_none()
                {
                    return Ok(HeadReduction::Closed(id));
                }

                let closed_form = self.intern_borrow(closed_lifetime, closed_access)?;
                let mut rebuilt = self.intern_type(dir::Type::Form(dir::FormType {
                    form: closed_form,
                    value: inner,
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
                let value = match self.reduce_type_head(origin, form.value)? {
                    Answer::Ready(value) => value,
                    Answer::Pending(_) => return Ok(HeadReduction::Closed(id)),
                };

                // redundant wrappers reduce to their payload
                if blocked!(id, self.is_redundant_form(origin, form.form, value)?) {
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
                let merged = blocked!(id, self.reduce_intersection(origin, id, &elements)?);

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
        while let dir::Type::Form(inner) = self.ty(value)? {
            match inner.form {
                // readonly payloads clamp the borrow access
                dir::Form::Readonly => {
                    access = self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Access(
                        dir::Access::Readonly,
                    )))?;
                    value = inner.value;
                }

                // borrowed payloads reborrow at the clamped access
                dir::Form::Borrowed(inner_borrow) => {
                    let inner_access = self.type_borrow(value.module_id, inner_borrow)?.access;
                    let inner_access = self.settled_root(inner_access)?;
                    if matches!(
                        self.ty(inner_access)?,
                        dir::Type::Memory(dir::MemoryLiteral::Access(dir::Access::Readonly))
                    ) {
                        access = inner_access;
                    }
                    value = inner.value;
                }

                // ownership forms contribute storage rather than another handle layer
                dir::Form::Managed | dir::Form::Owned => value = inner.value,

                // placement qualifies the resulting borrow handle
                dir::Form::Placed { place: current } => {
                    place = Some(current);
                    value = inner.value;
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
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        if let Some(done) = memo.get(&id).copied() {
            return Ok(Answer::Ready(done));
        }

        let original = id;

        // keep transparent alias references symbolic in child positions
        let id = self.settled_root(id)?;
        let id = match self.is_alias_instance(id)? {
            true => id,
            false => answer!(self.reduce_type_head(origin, id)?),
        };
        if let Some(done) = memo.get(&id).copied() {
            memo.insert(original, done);

            return Ok(Answer::Ready(done));
        }
        if !active.insert(id) {
            memo.insert(original, id);

            return Ok(Answer::Ready(id));
        }

        // reduce children first so rebuilt composite roots can reduce
        let mut replacements = FxIndexMap::default();
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        let mut children = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        let root = self.ty(id)?;
        self.for_each_type_child(id.module_id, &root, |child| children.push(child))?;
        for child in children {
            match self.reduce_type_graph(origin, child, memo, active)? {
                Answer::Ready(reduced) => {
                    if reduced != child {
                        replacements.insert(child, reduced);
                    }
                }
                Answer::Pending(dependencies) => blockers.extend(dependencies),
            }
        }
        if !blockers.is_empty() {
            active.swap_remove(&id);

            return Ok(Answer::pending(blockers));
        }
        let target = origin.module();
        let is_union = matches!(root, dir::Type::Union(_));
        if replacements.is_empty() && id.module_id == target && !is_union {
            active.swap_remove(&id);
            memo.insert(original, id);

            return Ok(Answer::Ready(id));
        }

        // read payloads from their owner, intern the rebuilt type in this component
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
        active.swap_remove(&id);
        let rebuilt = if rebuilt == id {
            rebuilt
        } else {
            answer!(self.reduce_type_graph(origin, rebuilt, memo, active)?)
        };
        memo.insert(original, rebuilt);

        Ok(Answer::Ready(rebuilt))
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
    ///
    /// Written types elide defaulted and lifetime arguments, so canonicalization completes them:
    /// defaults substitute against the arguments so far and elided lifetimes take the frame.
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
    fn type_alias_body(
        &mut self,
        _origin: Origin,
        instance_module: ModuleId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        // expand transparent alias definitions only
        let value = {
            let Some(definition) = self.definition(instance.symbol)? else {
                return Ok(Answer::Ready(None));
            };
            let dir::Definition::TypeAlias(definition) = definition else {
                return Ok(Answer::Ready(None));
            };

            definition.value
        };

        let substitution = self.instance_substitution(instance_module, instance)?;

        // substitute applied arguments through the body
        let substituted = self.substitute_type(value, &substitution)?;

        Ok(Answer::Ready(Some(substituted)))
    }
}
