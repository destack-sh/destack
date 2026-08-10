use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{CheckState, Origin};

/// Memory forms stacked over one base type.
#[derive(Debug, Clone)]
pub(in crate::check) struct FormChain {
    /// The memory forms, outermost first.
    forms: SmallVec<[dir::FormType; 2]>,
    /// The unqualified base type under every memory form.
    base: dir::GlobalTypeId,
    /// Whether the base can still gain forms at instantiation.
    is_open: bool,
}

/// One normalized managed or owned conversion to borrowed form.
pub(in crate::check) struct BorrowConversion {
    /// The module that owns the normalized borrow constructor.
    pub(in crate::check) module: ModuleId,
    /// The source memory form.
    pub(in crate::check) source: FormChain,
    /// The target borrowed form.
    pub(in crate::check) target: FormChain,
    /// The target borrow constructor and payload.
    pub(in crate::check) borrow: dir::FormType,
}

impl FormChain {
    /// Return the type beneath every memory form.
    pub(in crate::check) fn base(&self) -> dir::GlobalTypeId {
        self.base
    }

    /// Return the outer ownership form and its payload.
    pub(in crate::check) fn ownership_form(&self) -> Option<dir::FormType> {
        self.forms
            .iter()
            .copied()
            .find(|entry| entry.form.ownership().is_some())
    }

    /// Return the explicit placement term.
    pub(in crate::check) fn place(&self) -> Option<dir::GlobalTypeId> {
        self.forms.iter().find_map(|entry| match entry.form {
            dir::Form::Placed { place } => Some(place),
            _ => None,
        })
    }

    /// Return whether the chain removes mutable access.
    pub(in crate::check) fn is_readonly(&self) -> bool {
        self.forms
            .iter()
            .any(|entry| entry.form == dir::Form::Readonly)
    }
}

impl CheckState<'_> {
    /// Return whether one type is represented by a safe reference.
    pub(in crate::check) fn type_is_reference(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let chain = self.form_chain(origin, ty)?;

        self.form_is_reference(origin, &chain)
    }

    /// Return whether one memory form is represented by a safe reference.
    pub(in crate::check) fn form_is_reference(
        &mut self,
        origin: Origin,
        chain: &FormChain,
    ) -> CompilerResult<bool> {
        let ownership = self.form_ownership(origin, chain)?;
        let is_reference = match ownership {
            Some(dir::Ownership::Managed | dir::Ownership::Borrowed) => true,
            Some(dir::Ownership::Owned) => {
                let is_explicit = chain.ownership_form().is_some();
                let base = self.form_chain(origin, chain.base())?;
                let default = self.form_ownership(origin, &base)?;

                is_explicit && default == Some(dir::Ownership::Managed)
            }
            Some(dir::Ownership::Raw) | None => false,
        };

        Ok(is_reference)
    }

    /// Classify one normalized conversion to borrowed form.
    pub(in crate::check) fn borrow_conversion(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<BorrowConversion>> {
        let source = self.form_chain(origin, source)?;
        let target = self.form_chain(origin, target)?;
        let Some(borrow) = target.ownership_form() else {
            return Ok(None);
        };
        if !matches!(borrow.form, dir::Form::Borrowed(_)) {
            return Ok(None);
        }
        // require a placeable value, a borrow only targets owned storage
        if !self.ty(source.base())?.is_placeable() {
            return Ok(None);
        }

        // explicit borrowed and raw values reborrow through the receiver ladder
        if source.ownership_form().is_some_and(|form| {
            matches!(
                form.form.ownership(),
                Some(dir::Ownership::Borrowed | dir::Ownership::Raw)
            )
        }) {
            return Ok(None);
        }

        Ok(Some(BorrowConversion {
            module: origin.module(),
            source,
            target,
            borrow,
        }))
    }

    /// Return the closed access literal behind one access term, or none while open.
    pub(in crate::check) fn access_literal(
        &mut self,
        origin: Origin,
        access: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::Access>> {
        let access = self.reduce_type_head(origin, access)?;
        let literal = match self.ty(access)? {
            dir::Type::Memory(dir::MemoryLiteral::Access(access)) => Some(access),
            _ => None,
        };

        Ok(literal)
    }

    /// Return the concrete space required by a nominal declaration and its heritage.
    pub(in crate::check) fn nominal_space(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::Space>> {
        let mut active = FxIndexSet::default();

        self.nominal_space_guarded(symbol, &mut active)
    }

    /// Return one nominal space while terminating invalid heritage cycles.
    fn nominal_space_guarded(
        &mut self,
        symbol: dir::GlobalSymbolId,
        active: &mut FxIndexSet<dir::GlobalSymbolId>,
    ) -> CompilerResult<Option<dir::Space>> {
        if !active.insert(symbol) {
            return Ok(None);
        }
        let Some(definition) = self.definition(symbol)?.cloned() else {
            active.shift_remove(&symbol);

            return Ok(None);
        };
        let mut space = definition.space();

        // inherit the first concrete base, as heritage validation rejects disagreement
        for heritage in definition.bases() {
            let (_, inherited) = self.nominal_application(heritage.ty)?;
            let inherited = self.nominal_space_guarded(inherited.symbol, active)?;
            if space.is_none() {
                space = inherited;
            }
        }

        // inherit the first concrete interface requirement
        for conformance in definition.implementations() {
            let (_, inherited) = self.nominal_application(conformance.interface)?;
            let inherited = self.nominal_space_guarded(inherited.symbol, active)?;
            if space.is_none() {
                space = inherited;
            }
        }
        active.shift_remove(&symbol);

        Ok(space)
    }

    /// Return the outer placement term of one type.
    pub(in crate::check) fn type_place(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let mut current = ty;
        while let dir::Type::Form(form) = self.ty(current)? {
            if let dir::Form::Placed { place } = form.form {
                return Ok(Some(place));
            }
            current = form.value;
        }

        Ok(None)
    }

    /// Return whether one value crosses a call boundary in an immediate carrier.
    pub(in crate::check) fn is_immediate_value(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let ty = self.reduce_type_head(origin, ty)?;
        let is_immediate = match self.ty(ty)? {
            dir::Type::Null
            | dir::Type::Undefined
            | dir::Type::Key(_)
            | dir::Type::Memory(_)
            | dir::Type::Static(_)
            | dir::Type::Range(_) => true,
            dir::Type::Variant(variant) => {
                return self.is_immediate_value(origin, variant.owner);
            }
            dir::Type::Primitive(primitive) => primitive.representation_item().is_none(),
            dir::Type::Literal(literal) => matches!(
                literal.widen(),
                dir::Type::Primitive(primitive) if primitive.representation_item().is_none()
            ),
            _ => false,
        };

        Ok(is_immediate)
    }

    /// Place one binding value in its explicitly declared storage space.
    pub(in crate::check) fn place_binding_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let space = self
            .module(symbol.module_id)
            .binding_table()
            .get_symbol(symbol.local_id)
            .binding_space;
        let Some(space) = space else {
            return Ok(ty);
        };

        let place = self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Place(
            dir::Place::Space(space),
        )))?;

        self.placed_type(Origin::Symbol(symbol), ty, place)
    }

    /// Place one value type without requiring its payload to be solved.
    pub(in crate::check) fn placed_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        place: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // open types retain the complete placement supplied by their solution
        if self.ty(ty)?.is_open() || !self.ty(ty)?.is_placeable() {
            return Ok(ty);
        }

        self.intern_memory_type(
            origin,
            dir::Type::Form(dir::FormType {
                form: dir::Form::Placed { place },
                value: ty,
            }),
        )
    }

    /// Resolve one type's relative placement in the requested space.
    pub(in crate::check) fn resolve_relative_place(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        place: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let chain = self.form_chain(origin, ty)?;

        // return an open form chain unchanged
        if chain.is_open {
            return Ok(ty);
        }

        // types without runtime values have no placement
        if chain.forms.is_empty() && !self.ty(chain.base)?.is_placeable() {
            return Ok(ty);
        }

        // immediate values pass in registers and take no placement
        if self.is_immediate_value(origin, ty)? {
            return Ok(ty);
        }

        // preserve concrete placement and qualify one relative chain
        if let Some(current) = chain.place()
            && !self.is_memory_component(current, "relative")?
        {
            return Ok(ty);
        }

        // apply placement around the written type so aliases stay visible
        self.placed_type(origin, ty, place)
    }

    /// Strip every explicit memory form from one type.
    pub(in crate::check) fn strip_form(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut value = self.reduce_type_head(origin, id)?;
        while let dir::Type::Form(form) = self.ty(value)? {
            value = self.reduce_type_head(origin, form.value)?;
        }

        Ok(value)
    }

    /// Return the unqualified value accepted by one construction target.
    pub(in crate::check) fn construction_value(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let mut value = self.reduce_type_head(origin, target)?;

        // construction owns storage forms but never manufactures references
        while let dir::Type::Form(form) = self.ty(value)? {
            if matches!(form.form, dir::Form::Borrowed(_) | dir::Form::Raw) {
                return Ok(None);
            }
            value = self.reduce_type_head(origin, form.value)?;
        }

        Ok(Some(value))
    }

    /// Replace the value beneath every explicit memory form.
    pub(in crate::check) fn replace_form_value(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let head = self.reduce_type_head(origin, ty)?;
        let dir::Type::Form(form) = self.ty(head)? else {
            return Ok(value);
        };

        let payload = self.replace_form_value(origin, form.value, value)?;
        let form = self.adopt_form(head.module_id, form.form)?;
        let rebuilt = self.intern_type(dir::Type::Form(dir::FormType {
            form,
            value: payload,
        }))?;

        Ok(rebuilt)
    }

    /// Normalize one component to its canonical memory literal.
    pub(in crate::check) fn normalize_memory_component(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
        kind: dir::MemoryParameter,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some(text) = self.memory_component_text(origin, value)? else {
            return Ok(value);
        };
        let Some(literal) = dir::MemoryLiteral::from_text(kind, &text) else {
            return Ok(value);
        };

        self.intern_memory_literal(origin, literal)
    }

    /// Return whether one explicit ownership form matches its payload's default form.
    pub(in crate::check) fn is_default_ownership_form(
        &mut self,
        origin: Origin,
        form: dir::Form,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        if !matches!(form, dir::Form::Managed | dir::Form::Owned) {
            return Ok(false);
        }

        let Some(default) = self.default_ownership(origin, value)? else {
            return Ok(false);
        };

        Ok(form.ownership() == Some(default))
    }

    /// Drop one type's redundant explicit forms, keeping its authored payload.
    pub(in crate::check) fn reduce_redundant_forms(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut id = id;
        loop {
            let dir::Type::Form(form) = self.ty(id)? else {
                return Ok(id);
            };

            // decide on the reduced payload, return the authored spelling
            let value = self.reduce_type_head(origin, form.value)?;
            let drops = self.is_redundant_form(origin, form.form, value)?;
            if !drops {
                return Ok(id);
            }
            id = form.value;
        }
    }

    /// Return whether one explicit form grants its payload nothing.
    pub(in crate::check) fn is_redundant_form(
        &mut self,
        origin: Origin,
        form: dir::Form,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        if self.is_default_ownership_form(origin, form, value)? {
            return Ok(true);
        }

        // placement does not qualify types without runtime values
        if matches!(form, dir::Form::Placed { .. }) && !self.ty(value)?.is_placeable() {
            return Ok(true);
        }

        // readonly views over immutable payloads grant nothing less
        if form == dir::Form::Readonly {
            let mut active = SmallVec::new();
            if self.type_is_immutable(origin, value, &mut active)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Reduce one unary form constructor application.
    pub(in crate::check) fn reduce_form_constructor(
        &mut self,
        origin: Origin,
        module: ModuleId,
        instance: &dir::GenericApplication,
        form: dir::Form,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(value) = self.type_ids(module, instance.arguments)?.first().copied() else {
            return Ok(None);
        };
        let formed = dir::Type::Form(dir::FormType { form, value });
        let id = self.intern_memory_type(origin, formed)?;

        Ok(Some(id))
    }

    /// Reduce one borrowed form constructor application.
    pub(in crate::check) fn reduce_borrowed_constructor(
        &mut self,
        origin: Origin,
        module: ModuleId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let arguments = self.type_ids(module, instance.arguments)?.to_vec();
        let Some(value) = arguments.first().copied() else {
            return Ok(None);
        };
        let Some(lifetime) = arguments.get(1).copied() else {
            return Ok(None);
        };

        // missing access arguments default to mutable
        let access = match arguments.get(2).copied() {
            Some(access) => {
                self.normalize_memory_component(origin, access, dir::MemoryParameter::Access)?
            }
            None => self
                .intern_memory_literal(origin, dir::MemoryLiteral::Access(dir::Access::Mutable))?,
        };
        let lifetime =
            self.normalize_memory_component(origin, lifetime, dir::MemoryParameter::Lifetime)?;
        let form = self.intern_borrow(lifetime, access)?;
        let formed = dir::Type::Form(dir::FormType { form, value });
        let id = self.intern_memory_type(origin, formed)?;

        Ok(Some(id))
    }

    /// Reduce one placed form constructor application.
    pub(in crate::check) fn reduce_placed_constructor(
        &mut self,
        origin: Origin,
        module: ModuleId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let arguments = self.type_ids(module, instance.arguments)?.to_vec();
        let Some(value) = arguments.first().copied() else {
            return Ok(None);
        };
        let Some(place) = arguments.get(1).copied() else {
            return Ok(None);
        };

        let place = self.normalize_memory_component(origin, place, dir::MemoryParameter::Place)?;
        let formed = dir::Type::Form(dir::FormType {
            form: dir::Form::Placed { place },
            value,
        });
        let id = self.intern_memory_type(origin, formed)?;

        Ok(Some(id))
    }

    /// Evaluate one memory accessor, distributing over union targets.
    pub(in crate::check) fn reduce_memory_accessor(
        &mut self,
        origin: Origin,
        module: ModuleId,
        item: dir::LanguageItem,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(target) = self.type_ids(module, instance.arguments)?.first().copied() else {
            return Ok(None);
        };

        // close the inspected target first
        let target = self.reduce_type_head(origin, target)?;

        // distribute the accessor over union targets
        let elements = match self.ty(target)? {
            dir::Type::Union(union) => {
                SmallVec::<[_; 4]>::from_slice(self.type_ids(target.module_id, union.elements)?)
            }
            _ => SmallVec::from_slice(&[target]),
        };
        let mut reduced = Vec::with_capacity(elements.len());
        for element in elements {
            let element = self.reduce_type_head(origin, element)?;

            match self.reduce_element_accessor(origin, module, item, instance, element)? {
                // one symbolic element keeps the whole accessor symbolic
                None => return Ok(None),
                Some(accessor) => reduced.push(accessor),
            }
        }

        // join the element results, dropping never like any union would
        let mut kept = Vec::with_capacity(reduced.len());
        for accessor in reduced {
            if matches!(self.ty(accessor)?, dir::Type::Never) {
                continue;
            }
            if !kept.contains(&accessor) {
                kept.push(accessor);
            }
        }
        let joined = match kept.as_slice() {
            [] => self.intern_memory_type(origin, dir::Type::Never)?,
            [single] => *single,
            _ => self.normalized_union_type(kept)?,
        };

        Ok(Some(joined))
    }

    /// Evaluate one memory accessor over one closed element.
    fn reduce_element_accessor(
        &mut self,
        origin: Origin,
        module: ModuleId,
        item: dir::LanguageItem,
        instance: &dir::GenericApplication,
        element: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // close the element's form chain first
        let chain = self.form_chain(origin, element)?;
        self.reduce_stack_accessor(origin, module, item, instance, element, &chain)
    }

    /// Evaluate one memory accessor over one closed form chain.
    fn reduce_stack_accessor(
        &mut self,
        origin: Origin,
        module: ModuleId,
        item: dir::LanguageItem,
        instance: &dir::GenericApplication,
        element: dir::GlobalTypeId,
        chain: &FormChain,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        match item {
            // PayloadOf<T> removes one outer form
            dir::LanguageItem::PayloadOf => match chain.forms.first() {
                Some(outer) => Ok(Some(outer.value)),
                None if chain.is_open => Ok(None),
                None => Ok(Some(chain.base)),
            },
            // BaseOf<T> removes every form
            dir::LanguageItem::BaseOf => {
                if chain.is_open {
                    Ok(None)
                } else {
                    Ok(Some(chain.base))
                }
            }

            // ownership component
            dir::LanguageItem::OwnershipOf => self.ownership(origin, chain),
            dir::LanguageItem::OwnershipOr => {
                let ownership = self.ownership(origin, chain)?;

                self.component_or_default(origin, module, instance, ownership)
            }
            dir::LanguageItem::IsManaged => {
                self.ownership_predicate(origin, chain, dir::Ownership::Managed)
            }
            dir::LanguageItem::IsOwned => {
                self.ownership_predicate(origin, chain, dir::Ownership::Owned)
            }
            dir::LanguageItem::IsBorrowed => {
                self.ownership_predicate(origin, chain, dir::Ownership::Borrowed)
            }
            dir::LanguageItem::IsRaw => {
                self.ownership_predicate(origin, chain, dir::Ownership::Raw)
            }

            // access component
            dir::LanguageItem::AccessOf => self.access(origin, chain),
            dir::LanguageItem::AccessOr => {
                let access = self.access(origin, chain)?;

                self.component_or_default(origin, module, instance, access)
            }

            // placement component
            dir::LanguageItem::PlaceOf => self.place(origin, chain),
            dir::LanguageItem::PlaceOr => {
                let place = self.place(origin, chain)?;

                self.component_or_default(origin, module, instance, place)
            }
            dir::LanguageItem::PlaceIn => {
                let place = self.place(origin, chain)?;
                let Some(place) = place else {
                    return Ok(None);
                };

                // relative placement resolves to the given concrete space
                if self.is_memory_component(place, "relative")? {
                    let Some(space) = self.type_ids(module, instance.arguments)?.get(1).copied()
                    else {
                        return Ok(None);
                    };

                    Ok(Some(self.normalize_component_text(origin, space)?))
                } else {
                    Ok(Some(place))
                }
            }
            dir::LanguageItem::SpaceOf => self.space(origin, chain),
            dir::LanguageItem::SpaceOr => {
                let space = self.space(origin, chain)?;

                self.component_or_default(origin, module, instance, space)
            }
            dir::LanguageItem::IsShared => {
                let space = self.space(origin, chain)?;
                let Some(space) = space else {
                    return Ok(None);
                };
                let is_shared = self.is_memory_component(space, "shared")?;
                let is_shared = self.boolean_literal_type(origin, is_shared)?;

                Ok(is_shared)
            }
            dir::LanguageItem::IsSharedIn => {
                let place = self.place(origin, chain)?;
                let Some(place) = place else {
                    return Ok(None);
                };

                // relative placement resolves to the given concrete space first
                let resolved = if self.is_memory_component(place, "relative")? {
                    let Some(space) = self.type_ids(module, instance.arguments)?.get(1).copied()
                    else {
                        return Ok(None);
                    };

                    self.normalize_component_text(origin, space)?
                } else {
                    place
                };
                let is_shared = self.is_memory_component(resolved, "shared")?;
                let is_shared = self.boolean_literal_type(origin, is_shared)?;

                Ok(is_shared)
            }

            // lifetime component
            dir::LanguageItem::LifetimeOf => self.lifetime(origin, chain),
            dir::LanguageItem::LifetimeOr => {
                let lifetime = self.lifetime(origin, chain)?;

                self.component_or_default(origin, module, instance, lifetime)
            }

            // form rewriting
            dir::LanguageItem::WithBase => {
                if chain.is_open {
                    return Ok(None);
                }
                let Some(base) = self.type_ids(module, instance.arguments)?.get(1).copied() else {
                    return Ok(None);
                };

                Ok(Some(self.wrap_forms(origin, &chain.forms, base)?))
            }
            dir::LanguageItem::WithOwnership => {
                self.with_ownership(origin, module, instance, element)
            }
            dir::LanguageItem::WithPlace => {
                self.with_place(origin, module, instance, chain, element)
            }
            dir::LanguageItem::WithSpace => {
                self.with_space(origin, module, instance, chain, element)
            }
            dir::LanguageItem::WithLifetime => self.with_lifetime(origin, module, instance, chain),
            dir::LanguageItem::WithAccess => {
                self.with_access(origin, module, instance, chain, element)
            }
            _ => Ok(None),
        }
    }

    /// Walk one element's form chain down to its unqualified base.
    pub(in crate::check) fn form_chain(
        &mut self,
        origin: Origin,
        element: dir::GlobalTypeId,
    ) -> CompilerResult<FormChain> {
        let mut forms = SmallVec::<[dir::FormType; 2]>::new();
        let mut current = element;

        // collect memory forms outermost first
        loop {
            let root = self.shallow_resolve(current)?;
            current = self.reduce_type_head(origin, root)?;
            let dir::Type::Form(form) = self.ty(current)? else {
                break;
            };

            // rehome borrow payloads so chain rebuilds stay module-local
            let head = match form.form {
                dir::Form::Borrowed(borrow) => {
                    let borrow = self.type_borrow(current.module_id, borrow)?;

                    self.intern_borrow(borrow.lifetime, borrow.access)?
                }
                head => head,
            };
            forms.push(dir::FormType {
                form: head,
                value: form.value,
            });
            current = form.value;
        }

        // open and unreduced bases can gain forms when inference settles
        let is_open = self.ty(current)?.is_open();

        // nominal declarations contribute their intrinsic or inherited space
        if !forms
            .iter()
            .any(|entry| matches!(entry.form, dir::Form::Placed { .. }))
        {
            let symbol = match self.ty(current)? {
                dir::Type::Application(instance) => Some(instance.symbol),
                dir::Type::Reference(reference) => Some(reference.symbol),
                _ => None,
            };
            if let Some(space) = symbol
                .map(|symbol| self.nominal_space(symbol))
                .transpose()?
                .flatten()
            {
                let place = self.intern_memory_literal(
                    origin,
                    dir::MemoryLiteral::Place(dir::Place::Space(space)),
                )?;
                forms.push(dir::FormType {
                    form: dir::Form::Placed { place },
                    value: current,
                });
            }
        }

        Ok(FormChain {
            forms,
            base: current,
            is_open,
        })
    }

    /// Resolve one chain's explicit or default ownership.
    pub(in crate::check) fn form_ownership(
        &mut self,
        origin: Origin,
        chain: &FormChain,
    ) -> CompilerResult<Option<dir::Ownership>> {
        let ownership = match chain.ownership_form() {
            Some(form) => form.form.ownership(),
            None if chain.is_open => None,
            None => self.default_ownership(origin, chain.base)?,
        };

        Ok(ownership)
    }

    /// Return one chain's ownership kind as a type literal.
    fn ownership(
        &mut self,
        origin: Origin,
        chain: &FormChain,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let ownership = match self.form_ownership(origin, chain)? {
            Some(ownership) => Some(self.text_literal_type(origin, ownership.text())?),
            None if chain.is_open => None,
            None => Some(self.intern_memory_type(origin, dir::Type::Never)?),
        };

        Ok(ownership)
    }

    /// Return whether one chain's ownership matches a constructor.
    fn ownership_predicate(
        &mut self,
        origin: Origin,
        chain: &FormChain,
        ownership: dir::Ownership,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let matches_ownership = match self.form_ownership(origin, chain)? {
            Some(found) => self.boolean_literal_type(origin, found == ownership)?,
            None if chain.is_open => None,
            None => self.boolean_literal_type(origin, false)?,
        };

        Ok(matches_ownership)
    }

    /// Return whether one type's family defaults to managed storage.
    pub(in crate::check) fn defaults_to_managed(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let ownership = self.default_ownership(origin, ty)?;

        Ok(ownership == Some(dir::Ownership::Managed))
    }

    /// Return one reduced type's default ownership.
    pub(in crate::check) fn default_ownership(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::Ownership>> {
        let ty = self.reduce_type_head(origin, ty)?;
        let default = match self.ty(ty)? {
            dir::Type::Any
            | dir::Type::Unknown
            | dir::Type::Object(_)
            | dir::Type::Dynamic(_)
            | dir::Type::Shape(_)
            | dir::Type::Array(_)
            | dir::Type::Slice(_)
            | dir::Type::Function(_) => Some(dir::Ownership::Managed),
            dir::Type::Never
            | dir::Type::Void
            | dir::Type::Undefined
            | dir::Type::Null
            | dir::Type::Memory(_)
            | dir::Type::Static(_)
            | dir::Type::Intrinsic
            | dir::Type::Key(_)
            | dir::Type::Range(_)
            | dir::Type::Tuple(_)
            | dir::Type::FixedArray(_)
            | dir::Type::FunctionPointer(_) => Some(dir::Ownership::Owned),
            dir::Type::Primitive(primitive) => {
                if primitive.representation_item().is_some() {
                    Some(dir::Ownership::Managed)
                } else {
                    Some(dir::Ownership::Owned)
                }
            }
            dir::Type::Application(instance) => {
                let symbol = instance.symbol;
                match self.definition(symbol)?.cloned() {
                    Some(dir::Definition::Class(_) | dir::Definition::Interface(_)) => {
                        Some(dir::Ownership::Managed)
                    }
                    Some(dir::Definition::Struct(_) | dir::Definition::Enum(_)) => {
                        Some(dir::Ownership::Owned)
                    }
                    Some(dir::Definition::Newtype(definition)) => {
                        let backing = self.reduce_type_head(origin, definition.backing)?;

                        return self.default_ownership(origin, backing);
                    }
                    _ => None,
                }
            }
            dir::Type::Variant(variant) => {
                return self.default_ownership(origin, variant.owner);
            }
            dir::Type::Form(form) => match form.form {
                dir::Form::Readonly | dir::Form::Placed { .. } => {
                    return self.default_ownership(origin, form.value);
                }
                // take an explicit form as the value's own ownership
                dir::Form::Managed | dir::Form::Owned | dir::Form::Borrowed(_) | dir::Form::Raw => {
                    form.form.ownership()
                }
            },
            dir::Type::Reference(_)
            | dir::Type::Parameter(_)
            | dir::Type::Erased(_)
            | dir::Type::Variable(_)
            | dir::Type::This
            | dir::Type::Member(_)
            | dir::Type::Operation(_)
            | dir::Type::FunctionSignature(_)
            | dir::Type::Intersection(_)
            | dir::Type::Union(_)
            | dir::Type::Error => None,
            // literals use the ownership of their runtime scalar carrier
            dir::Type::Literal(literal) => {
                let carrier = self.intern_memory_type(origin, literal.widen())?;

                return self.default_ownership(origin, carrier);
            }
            // refinements share their base's default form
            dir::Type::Refined(refined) => {
                let refined = self.type_refined(ty.module_id, refined)?;

                return self.default_ownership(origin, refined.base);
            }
        };

        Ok(default)
    }

    /// Return one chain's access mode as a type literal.
    fn access(
        &mut self,
        origin: Origin,
        chain: &FormChain,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        for entry in &chain.forms {
            match entry.form {
                dir::Form::Readonly => return self.text_literal_type(origin, "readonly").map(Some),
                dir::Form::Borrowed(borrow) => {
                    let access = self.type_borrow(origin.module(), borrow)?.access;

                    return self.normalize_component_text(origin, access).map(Some);
                }
                _ => {}
            }
        }

        if chain.is_open {
            Ok(None)
        } else {
            self.text_literal_type(origin, "mutable").map(Some)
        }
    }

    /// Return one chain's placement as a type literal.
    fn place(
        &mut self,
        origin: Origin,
        chain: &FormChain,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        for entry in &chain.forms {
            if let dir::Form::Placed { place } = entry.form {
                return self.normalize_component_text(origin, place).map(Some);
            }
        }

        if chain.is_open {
            Ok(None)
        } else {
            self.text_literal_type(origin, "relative").map(Some)
        }
    }

    /// Return one chain's explicit concrete space as a type literal.
    fn space(
        &mut self,
        origin: Origin,
        chain: &FormChain,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let place = self.place(origin, chain)?;
        let Some(place) = place else {
            return Ok(None);
        };

        // relative placement names no concrete space
        if self.is_memory_component(place, "relative")? {
            Ok(Some(self.intern_memory_type(origin, dir::Type::Never)?))
        } else {
            Ok(Some(place))
        }
    }

    /// Return one chain's borrow lifetime type.
    fn lifetime(
        &mut self,
        origin: Origin,
        chain: &FormChain,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        for entry in &chain.forms {
            if let dir::Form::Borrowed(borrow) = entry.form {
                return Ok(Some(self.type_borrow(origin.module(), borrow)?.lifetime));
            }
        }

        if chain.is_open {
            Ok(None)
        } else {
            Ok(Some(self.intern_memory_type(origin, dir::Type::Never)?))
        }
    }

    /// Return a component value unless it is never, otherwise return the `*Or` default.
    fn component_or_default(
        &mut self,
        origin: Origin,
        module: ModuleId,
        instance: &dir::GenericApplication,
        component: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(component) = component else {
            return Ok(None);
        };

        if matches!(self.ty(component)?, dir::Type::Never) {
            let Some(default) = self.type_ids(module, instance.arguments)?.get(1).copied() else {
                return Ok(None);
            };

            Ok(Some(self.normalize_component_text(origin, default)?))
        } else {
            Ok(Some(component))
        }
    }

    /// Apply one requested ownership form.
    fn with_ownership(
        &mut self,
        origin: Origin,
        module: ModuleId,
        instance: &dir::GenericApplication,
        element: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let arguments = self.type_ids(module, instance.arguments)?.to_vec();
        let Some(ownership) = arguments.get(1).copied() else {
            return Ok(None);
        };
        let ownership =
            self.normalize_memory_component(origin, ownership, dir::MemoryParameter::Ownership)?;
        let dir::Type::Memory(dir::MemoryLiteral::Ownership(kind)) = self.ty(ownership)? else {
            return Ok(None);
        };

        let form = match kind {
            dir::Ownership::Managed => dir::Form::Managed,
            dir::Ownership::Owned => dir::Form::Owned,
            dir::Ownership::Raw => dir::Form::Raw,
            dir::Ownership::Borrowed => {
                // borrowing needs the explicit lifetime argument
                let Some(lifetime) = arguments.get(2).copied() else {
                    return Ok(None);
                };
                let lifetime = self.normalize_memory_component(
                    origin,
                    lifetime,
                    dir::MemoryParameter::Lifetime,
                )?;
                let access = self.intern_memory_literal(
                    origin,
                    dir::MemoryLiteral::Access(dir::Access::Mutable),
                )?;

                self.intern_borrow(lifetime, access)?
            }
        };
        let formed = dir::Type::Form(dir::FormType {
            form,
            value: element,
        });

        Ok(Some(self.intern_memory_type(origin, formed)?))
    }

    /// Return whether two related types differ only by concrete placement.
    pub(in crate::check) fn is_place_relabel(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let source_chain = self.form_chain(origin, source)?;
        let target_chain = self.form_chain(origin, target)?;
        if source_chain.base != target_chain.base {
            return Ok(false);
        }
        let (Some(source_place), Some(target_place)) = (source_chain.place(), target_chain.place())
        else {
            return Ok(false);
        };
        if source_place == target_place {
            return Ok(false);
        }

        // require the chains to agree on every form other than placement
        let placeless = |chain: &FormChain| {
            chain
                .forms
                .iter()
                .filter(|entry| !matches!(entry.form, dir::Form::Placed { .. }))
                .cloned()
                .collect::<Vec<_>>()
        };

        Ok(placeless(&source_chain) == placeless(&target_chain))
    }

    /// Replace one placement form.
    fn with_place(
        &mut self,
        origin: Origin,
        module: ModuleId,
        instance: &dir::GenericApplication,
        chain: &FormChain,
        element: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(place) = self.type_ids(module, instance.arguments)?.get(1).copied() else {
            return Ok(None);
        };
        let place = self.normalize_memory_component(origin, place, dir::MemoryParameter::Place)?;

        // replace placement in its existing chain position
        let position = chain
            .forms
            .iter()
            .position(|entry| matches!(entry.form, dir::Form::Placed { .. }));
        match position {
            Some(position) => {
                let mut forms = chain.forms.clone();
                forms[position].form = dir::Form::Placed { place };

                Ok(Some(self.wrap_forms(origin, &forms, chain.base)?))
            }
            // add placement to an unplaced chain
            None => {
                let formed = dir::Type::Form(dir::FormType {
                    form: dir::Form::Placed { place },
                    value: element,
                });

                Ok(Some(self.intern_memory_type(origin, formed)?))
            }
        }
    }

    /// Replace one borrow lifetime component.
    fn with_lifetime(
        &mut self,
        origin: Origin,
        module: ModuleId,
        instance: &dir::GenericApplication,
        chain: &FormChain,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(lifetime) = self.type_ids(module, instance.arguments)?.get(1).copied() else {
            return Ok(None);
        };
        let lifetime =
            self.normalize_memory_component(origin, lifetime, dir::MemoryParameter::Lifetime)?;

        self.replace_borrow(origin, chain, Some(lifetime), None)
    }

    /// Resolve one relative placement in a concrete space.
    fn with_space(
        &mut self,
        origin: Origin,
        module: ModuleId,
        instance: &dir::GenericApplication,
        chain: &FormChain,
        element: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // preserve an existing concrete placement
        let position = chain
            .forms
            .iter()
            .position(|entry| matches!(entry.form, dir::Form::Placed { .. }));
        if let Some(position) = position {
            let dir::Form::Placed { place } = chain.forms[position].form else {
                unreachable!("placement position must point at a placement form");
            };
            if self.is_closed_place(place)? {
                return Ok(Some(self.wrap_forms(origin, &chain.forms, chain.base)?));
            }
        }

        self.with_place(origin, module, instance, chain, element)
    }

    /// Apply one requested access form.
    fn with_access(
        &mut self,
        origin: Origin,
        module: ModuleId,
        instance: &dir::GenericApplication,
        chain: &FormChain,
        element: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(access) = self.type_ids(module, instance.arguments)?.get(1).copied() else {
            return Ok(None);
        };
        let access =
            self.normalize_memory_component(origin, access, dir::MemoryParameter::Access)?;

        // borrow access lives on the borrow form itself
        if chain
            .forms
            .iter()
            .any(|entry| matches!(entry.form, dir::Form::Borrowed(_)))
        {
            return self.replace_borrow(origin, chain, None, Some(access));
        }

        // non-borrow readonly access is a readonly form over the same value
        match self.ty(access)? {
            dir::Type::Memory(dir::MemoryLiteral::Access(dir::Access::Readonly)) => {
                if chain
                    .forms
                    .iter()
                    .any(|entry| matches!(entry.form, dir::Form::Readonly))
                {
                    Ok(Some(element))
                } else {
                    let mut forms = chain.forms.clone();
                    forms.push(dir::FormType {
                        form: dir::Form::Readonly,
                        value: chain.base,
                    });

                    Ok(Some(self.wrap_forms(origin, &forms, chain.base)?))
                }
            }
            dir::Type::Memory(dir::MemoryLiteral::Access(dir::Access::Mutable)) => {
                let forms = chain
                    .forms
                    .iter()
                    .copied()
                    .filter(|entry| !matches!(entry.form, dir::Form::Readonly))
                    .collect::<SmallVec<[_; 2]>>();

                Ok(Some(self.wrap_forms(origin, &forms, chain.base)?))
            }
            dir::Type::Memory(dir::MemoryLiteral::Access(dir::Access::Exclusive)) => {
                Ok(Some(self.intern_memory_type(origin, dir::Type::Never)?))
            }
            _ => Ok(None),
        }
    }

    /// Rebuild one chain's borrow form with replaced components.
    fn replace_borrow(
        &mut self,
        origin: Origin,
        chain: &FormChain,
        lifetime: Option<dir::GlobalTypeId>,
        access: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let position = chain
            .forms
            .iter()
            .position(|entry| matches!(entry.form, dir::Form::Borrowed(_)));
        let Some(position) = position else {
            return Ok(None);
        };

        let mut forms = chain.forms.clone();
        if let dir::Form::Borrowed(borrow) = forms[position].form {
            let borrow = self.type_borrow(self.module_id, borrow)?;
            forms[position].form = self.intern_borrow(
                lifetime.unwrap_or(borrow.lifetime),
                access.unwrap_or(borrow.access),
            )?;
        }

        Ok(Some(self.wrap_forms(origin, &forms, chain.base)?))
    }

    /// Rebuild one form chain over a new base, innermost first.
    fn wrap_forms(
        &mut self,
        origin: Origin,
        forms: &[dir::FormType],
        base: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut current = base;

        for entry in forms.iter().rev() {
            current = self.intern_memory_type(
                origin,
                dir::Type::Form(dir::FormType {
                    form: entry.form,
                    value: current,
                }),
            )?;
        }

        Ok(current)
    }

    /// Return one memory component as its canonical text written form.
    fn normalize_component_text(
        &mut self,
        origin: Origin,
        component: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        match self.memory_component_text(origin, component)? {
            Some(text) => self.text_literal_type(origin, &text),
            None => Ok(component),
        }
    }

    /// Return the text behind one closed memory component.
    fn memory_component_text(
        &mut self,
        origin: Origin,
        component: dir::GlobalTypeId,
    ) -> CompilerResult<Option<String>> {
        let component = self.reduce_type_head(origin, component)?;

        let text = match self.ty(component)? {
            dir::Type::Literal(dir::ScalarLiteral::String(value)) => {
                Some(self.strings().get(value).to_string())
            }
            dir::Type::Memory(literal) => Some(literal.text().to_string()),
            _ => None,
        };

        Ok(text)
    }

    /// Return whether one type is a specific memory component literal.
    pub(in crate::check) fn is_memory_component(
        &self,
        ty: dir::GlobalTypeId,
        text: &str,
    ) -> CompilerResult<bool> {
        let is_match = match self.ty(ty)? {
            dir::Type::Literal(dir::ScalarLiteral::String(value)) => {
                value == dir::StringId::for_text(text)
            }
            dir::Type::Memory(literal) => literal.text() == text,
            _ => false,
        };

        Ok(is_match)
    }

    /// Push one string literal type.
    fn text_literal_type(
        &mut self,
        origin: Origin,
        text: &str,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let value = self.strings().intern(text);

        self.intern_memory_type(
            origin,
            dir::Type::Literal(dir::ScalarLiteral::String(value)),
        )
    }

    /// Push one boolean literal type.
    fn boolean_literal_type(
        &mut self,
        origin: Origin,
        value: bool,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let ty = self.intern_memory_type(
            origin,
            dir::Type::Literal(dir::ScalarLiteral::Boolean(value)),
        )?;

        Ok(Some(ty))
    }

    /// Push one memory literal written form.
    fn intern_memory_literal(
        &mut self,
        origin: Origin,
        literal: dir::MemoryLiteral,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.intern_memory_type(origin, dir::Type::Memory(literal))
    }

    /// Push one memory type at the accessor's origin.
    fn intern_memory_type(
        &mut self,
        _origin: Origin,
        ty: dir::Type,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.intern_type(ty)
    }

    /// Resolve one type to its readable value.
    pub(in crate::check) fn readable_value(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut current = self.shallow_resolve(ty)?;
        while let dir::Type::Form(form) = self.ty(current)? {
            // only alias-transparent forms disappear for reads
            if !matches!(form.form, dir::Form::Managed | dir::Form::Readonly) {
                break;
            }
            current = self.shallow_resolve(form.value)?;
        }

        Ok(current)
    }

    /// Rewrite one form composition into its canonical interned order.
    pub(in crate::check) fn canonical_form_type(
        &mut self,
        ty: dir::Type,
    ) -> CompilerResult<dir::Type> {
        let dir::Type::Form(form) = ty else {
            return Ok(ty);
        };
        let dir::Type::Form(inner) = self.ty(form.value)? else {
            return Ok(ty);
        };

        match (form.form, inner.form) {
            // an explicit inner placement absorbs the outer request
            (dir::Form::Placed { place }, dir::Form::Placed { place: existing }) => {
                if self.is_relative_place(existing)? {
                    let value = inner.value;

                    return Ok(dir::Type::Form(dir::FormType {
                        form: dir::Form::Placed { place },
                        value,
                    }));
                }
                if self.is_closed_place(existing)? {
                    return Ok(dir::Type::Form(inner));
                }

                Ok(ty)
            }
            // readonly views are idempotent
            (dir::Form::Readonly, dir::Form::Readonly) => Ok(dir::Type::Form(inner)),
            // placement commutes with every other axis: `^shared T` is `shared ^T`
            (form_kind, dir::Form::Placed { place }) => {
                let value = self.intern_type(dir::Type::Form(dir::FormType {
                    form: form_kind,
                    value: inner.value,
                }))?;

                Ok(dir::Type::Form(dir::FormType {
                    form: dir::Form::Placed { place },
                    value,
                }))
            }
            _ => Ok(ty),
        }
    }

    /// Return whether one place singleton is the relative placeholder.
    fn is_relative_place(&self, place: dir::GlobalTypeId) -> CompilerResult<bool> {
        self.is_memory_component(place, "relative")
    }

    /// Return whether one place singleton names a concrete space.
    fn is_closed_place(&self, place: dir::GlobalTypeId) -> CompilerResult<bool> {
        Ok(self.place_space(place)?.is_some())
    }

    /// Return the concrete space one place singleton names.
    pub(in crate::check) fn place_space(
        &self,
        place: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::Space>> {
        let space = match self.ty(place)? {
            dir::Type::Memory(dir::MemoryLiteral::Place(dir::Place::Space(space)))
            | dir::Type::Memory(dir::MemoryLiteral::Space(space)) => Some(space),
            dir::Type::Literal(dir::ScalarLiteral::String(value)) => {
                [dir::Space::Local, dir::Space::Shared]
                    .into_iter()
                    .find(|space| value == dir::StringId::for_text(space.text()))
            }
            _ => None,
        };

        Ok(space)
    }
}
