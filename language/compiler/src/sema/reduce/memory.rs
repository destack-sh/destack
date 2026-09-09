use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{CheckState, Origin};
use crate::{CompilerError, CompilerResult};

/// Memory forms stacked over one base type.
#[derive(Debug, Clone)]
pub(in crate::sema) struct FormChain {
    /// The memory forms, outermost first.
    forms: SmallVec<[dir::FormType; 2]>,
    /// The unqualified base type under every memory form.
    base: dir::GlobalTypeId,
    /// The outermost reference form's referent place, or the declared nominal space.
    place: Option<dir::GlobalTypeId>,
    /// The outermost borrow form's region.
    region: Option<dir::GlobalTypeId>,
    /// Whether the base can still gain forms at instantiation.
    is_open: bool,
}

/// One normalized managed or owned conversion to borrowed form.
pub(in crate::sema) struct BorrowConversion {
    /// The module that owns the normalized borrow constructor.
    pub(in crate::sema) module: ModuleId,
    /// The source memory form.
    pub(in crate::sema) source: FormChain,
    /// Whether the borrow acquires the source handle itself.
    pub(in crate::sema) acquires_handle: bool,
    /// The target borrow constructor and payload.
    pub(in crate::sema) borrow: dir::FormType,
}

impl FormChain {
    /// Return the type beneath every memory form.
    pub(in crate::sema) fn base(&self) -> dir::GlobalTypeId {
        self.base
    }

    /// Return the outermost referent place the chain records.
    pub(in crate::sema) fn region(&self) -> Option<dir::GlobalTypeId> {
        self.region
    }

    /// Return the outermost reference form's referent place.
    pub(in crate::sema) fn place(&self) -> Option<dir::GlobalTypeId> {
        self.place
    }

    /// Return the memory forms, outermost first.
    pub(in crate::sema) fn forms(&self) -> &[dir::FormType] {
        &self.forms
    }

    /// Return the outer ownership form and its payload.
    pub(in crate::sema) fn ownership_form(&self) -> Option<dir::FormType> {
        self.forms
            .iter()
            .copied()
            .find(|entry| entry.form.ownership().is_some())
    }

    /// Return whether the chain removes mutable access.
    pub(in crate::sema) fn is_readonly(&self) -> bool {
        self.forms
            .iter()
            .any(|entry| entry.form == dir::Form::Readonly)
    }
}

impl CheckState<'_> {
    /// Return whether one type is represented by a safe reference.
    pub(in crate::sema) fn type_is_reference(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let chain = self.form_chain(origin, ty)?;

        self.form_is_reference(origin, &chain)
    }

    /// Return whether one type's value lives behind an aliasing handle.
    pub(in crate::sema) fn type_is_aliased(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // reuse the alias decision a closed type already made
        let ty = self.shallow_resolve(ty)?;
        let flags = self.type_flags(ty)?;
        let is_closed = !flags.has_variable() && !flags.has_parameter() && !flags.has_this();
        if is_closed && let Some(is_aliased) = self.aliasing.get(&ty) {
            return Ok(*is_aliased);
        }

        // read the aliasing off the form chain's ownership
        let chain = self.form_chain(origin, ty)?;
        let ownership = self.form_ownership(origin, &chain)?;
        let is_aliased = matches!(
            ownership,
            Some(dir::Ownership::Managed | dir::Ownership::Borrowed)
        );
        if is_closed {
            self.aliasing.insert(ty, is_aliased);
        }

        Ok(is_aliased)
    }

    /// Return whether one memory form is represented by a safe reference.
    pub(in crate::sema) fn form_is_reference(
        &mut self,
        origin: Origin,
        chain: &FormChain,
    ) -> CompilerResult<bool> {
        // read whether the chain itself carries a reference
        let ownership = self.form_ownership(origin, chain)?;
        let is_reference = match ownership {
            // managed and borrowed handles are references themselves
            Some(dir::Ownership::Managed | dir::Ownership::Borrowed) => true,
            // an explicit owned form over a managed base still holds a reference
            Some(dir::Ownership::Owned) => {
                let is_explicit = chain.ownership_form().is_some();
                let base = self.form_chain(origin, chain.base())?;
                let default = self.form_ownership(origin, &base)?;

                is_explicit && default == Some(dir::Ownership::Managed)
            }
            // raw pointers and open chains stand outside the safe references
            Some(dir::Ownership::Raw) | None => false,
        };

        Ok(is_reference)
    }

    /// Classify one normalized conversion to borrowed form.
    pub(in crate::sema) fn borrow_conversion(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<BorrowConversion>> {
        // normalize both sides so accessor intrinsics expose their borrow representations
        let source = self.normalize(origin, source)?;
        let source = self.form_chain(origin, source)?;
        let target = self.normalize(origin, target)?;
        let target = self.form_chain(origin, target)?;

        // require a borrowed target form
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

        // note when a borrow-shaped target payload acquires the handle itself
        let payload = self.normalize(origin, borrow.value)?;
        let payload = self.form_chain(origin, payload)?;
        let acquires_handle = payload
            .ownership_form()
            .is_some_and(|form| matches!(form.form, dir::Form::Borrowed(_)));

        // leave explicit borrowed and raw values to the receiver ladder's reborrow
        if !acquires_handle
            && source.ownership_form().is_some_and(|form| {
                matches!(
                    form.form.ownership(),
                    Some(dir::Ownership::Borrowed | dir::Ownership::Raw)
                )
            })
        {
            return Ok(None);
        }

        Ok(Some(BorrowConversion {
            module: origin.module(),
            source,
            acquires_handle,
            borrow,
        }))
    }

    /// Return the space one nominal declaration writes, reading committed space across modules.
    pub(in crate::sema) fn declared_space(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::Space>> {
        // read the committed space when the declaring module is out of reach
        let Some(module) = self.module_maybe(symbol.module_id) else {
            return self.nominal_space(symbol);
        };

        // read the place the declaration writes
        let source = module.symbol_declaration_node(symbol.local_id)?;
        let Ok(declaration) = source.try_into_typed::<dir::Declaration>() else {
            return Ok(None);
        };
        let place = module.parsed.tree.get(declaration).place();

        Ok(place.map(dir::PlaceModifier::space))
    }

    /// Return the concrete space required by a nominal declaration and its heritage.
    pub(in crate::sema) fn nominal_space(
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
        // stop at a symbol already on the heritage walk
        if !active.insert(symbol) {
            return Ok(None);
        }

        // start from the space the declaration itself writes
        let Some(definition) = self.definition(symbol)? else {
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
    pub(in crate::sema) fn type_place(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let mut current = ty;
        while let dir::Type::Form(form) = self.ty(current)? {
            match form.form {
                // a managed form writes its referent place directly
                dir::Form::Managed { place } => return Ok(Some(place)),
                // a borrow keeps its referent space inside the region
                dir::Form::Borrowed(borrow) => {
                    let borrow = self.type_borrow(current.module_id, borrow)?;

                    // read the spaces a written region pair exposes
                    let region = self.shallow_resolve(borrow.region)?;

                    return Ok(match self.ty(region)? {
                        dir::Type::Region(region) => Some(region.space),
                        _ => None,
                    });
                }
                // every other form leaves placement to the value beneath
                dir::Form::Owned | dir::Form::Readonly | dir::Form::Raw => {}
            }

            current = form.value;
        }

        Ok(None)
    }

    /// Return whether one value passes to a call in an immediate representation.
    pub(in crate::sema) fn is_immediate_value(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // read whether the value passes in registers
        let ty = self.normalize(origin, ty)?;
        let is_immediate = match self.ty(ty)? {
            // the scalar-shaped types pass in registers
            dir::Type::Null
            | dir::Type::Undefined
            | dir::Type::Key(_)
            | dir::Type::Static(_)
            | dir::Type::Range(_) => true,
            // a variant travels the way its owning enum does
            dir::Type::Variant(variant) => {
                return self.is_immediate_value(origin, variant.owner);
            }
            // a primitive is immediate until it names a representation item
            dir::Type::Primitive(primitive) => primitive.representation_item().is_none(),
            // a literal travels the way the scalar it widens to does
            dir::Type::Literal(literal) => matches!(
                literal.widen(),
                dir::Type::Primitive(primitive) if primitive.representation_item().is_none()
            ),
            // every remaining type passes behind a handle
            _ => false,
        };

        Ok(is_immediate)
    }

    /// Place one binding value in its explicitly declared storage space.
    pub(in crate::sema) fn place_binding_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // leave a binding without a written space at its inferred type
        let space = self
            .module(symbol.module_id)
            .binding_table()
            .get_symbol(symbol.local_id)
            .binding_space;
        let Some(space) = space else {
            return Ok(ty);
        };

        // place the value at the space the binding writes
        let place = self.place_literal(space)?;

        self.placed_type(Origin::Symbol(symbol), ty, place)
    }

    /// Place one value type without requiring its payload to be solved.
    pub(in crate::sema) fn placed_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        place: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // open types retain the complete placement supplied by their solution
        let resolved = self.shallow_resolve(ty)?;
        if self.ty(resolved)?.is_open() || !self.ty(resolved)?.is_placeable() {
            return Ok(ty);
        }

        self.with_referent_place(origin, resolved, place)
    }

    /// Write one referent place onto a type's outermost reference constructor.
    pub(in crate::sema) fn with_referent_place(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        place: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // place the form the type carries
        let resolved = self.shallow_resolve(ty)?;
        match self.ty(resolved)? {
            dir::Type::Form(form) => match form.form {
                dir::Form::Managed { .. } => self.intern_type(dir::Type::Form(dir::FormType {
                    form: dir::Form::Managed { place },
                    value: form.value,
                })),
                // access views pass the place through to their payloads
                dir::Form::Readonly => {
                    let value = self.with_referent_place(origin, form.value, place)?;

                    self.intern_type(dir::Type::Form(dir::FormType {
                        form: dir::Form::Readonly,
                        value,
                    }))
                }
                // borrow referent spaces live inside the region itself
                dir::Form::Borrowed(_) | dir::Form::Owned | dir::Form::Raw => Ok(ty),
            },
            dir::Type::Slice(slice) => self.intern_type(dir::Type::Slice(dir::SliceType {
                element: slice.element,
                place,
            })),
            dir::Type::Dynamic(dynamic) => self.intern_type(dir::Type::Dynamic(dir::DynamicType {
                constraint: dynamic.constraint,
                place,
            })),
            dir::Type::Function(function) => {
                self.intern_type(dir::Type::Function(dir::FunctionType { place, ..function }))
            }
            // managed-family bases materialize their implicit managed form
            _ if self.default_ownership(origin, resolved)? == Some(dir::Ownership::Managed) => self
                .intern_type(dir::Type::Form(dir::FormType {
                    form: dir::Form::Managed { place },
                    value: resolved,
                })),
            // hand every value family back as written
            _ => Ok(ty),
        }
    }

    /// Settle one type's relative placement in the requested space.
    pub(in crate::sema) fn resolve_relative_place(
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

        // keep a type without runtime values as it is
        if chain.forms.is_empty() && !self.ty(chain.base)?.is_placeable() {
            return Ok(ty);
        }

        // leave an immediate value unplaced, it passes in registers
        if self.is_immediate_value(origin, ty)? {
            return Ok(ty);
        }

        // preserve placement the chain already carries
        if chain.place().is_some() {
            return Ok(ty);
        }

        // apply placement around the written type so aliases stay visible
        self.placed_type(origin, ty, place)
    }

    /// Return the first pair of contradicting concrete places in one form chain.
    pub(in crate::sema) fn conflicting_places(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<(dir::Space, dir::Space)>> {
        // walk inward through aliases, keeping the outermost concrete place
        let mut outer: Option<dir::Space> = None;
        let mut current = self.normalize(origin, ty)?;
        while let dir::Type::Form(form) = self.ty(current)? {
            if let dir::Form::Managed { place } = form.form
                && let Some(space) = self.place_space(place)?
            {
                match outer {
                    None => outer = Some(space),
                    Some(written) if written != space => return Ok(Some((written, space))),
                    Some(_) => {}
                }
            }
            current = self.normalize(origin, form.value)?;
        }

        Ok(None)
    }

    /// Return the component an elided memory position takes when nothing decides it: the frame
    /// extent, readonly access, or the local place.
    pub(in crate::sema) fn elided_memory_default(
        &mut self,
        kind: dir::MemoryParameter,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // elide each memory parameter at its default
        match kind {
            dir::MemoryParameter::Region => self.lifetime_literal(dir::Lifetime::Frame),
            dir::MemoryParameter::Access => self.access_literal(dir::Access::Readonly),
            dir::MemoryParameter::Place | dir::MemoryParameter::Space => self.local_place(),
            dir::MemoryParameter::Ownership => Err(CompilerError::Internal {
                message: "an ownership position opened an elided memory hole".to_string(),
            }),
        }
    }

    /// Return the receiver term one callable value elides.
    pub(in crate::sema) fn elided_receiver(&mut self) -> CompilerResult<dir::GlobalTypeId> {
        self.receiver_literal(dir::ReceiverMode::ELIDED)
    }

    /// Strip every explicit memory form from one type.
    pub(in crate::sema) fn strip_form(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut value = self.normalize(origin, id)?;
        while let dir::Type::Form(form) = self.ty(value)? {
            value = self.normalize(origin, form.value)?;
        }

        Ok(value)
    }

    /// Return the value one type stores beneath its ownership forms, a borrow staying whole.
    pub(in crate::sema) fn ownership_payload(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut value = self.normalize(origin, id)?;
        while let dir::Type::Form(form) = self.ty(value)?
            && matches!(
                form.form,
                dir::Form::Owned | dir::Form::Managed { .. } | dir::Form::Readonly
            )
        {
            value = self.normalize(origin, form.value)?;
        }

        Ok(value)
    }

    /// Return the unqualified value accepted by one construction target.
    pub(in crate::sema) fn construction_value(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // construction owns storage forms, stopping at a reference or an open variable
        let mut value = target;
        loop {
            value = self.shallow_resolve(value)?;
            if self.root_variable(value)?.is_some() {
                return Ok(None);
            }
            value = self.normalize(origin, value)?;
            let dir::Type::Form(form) = self.ty(value)? else {
                return Ok(Some(value));
            };
            if matches!(form.form, dir::Form::Borrowed(_) | dir::Form::Raw) {
                return Ok(None);
            }
            value = form.value;
        }
    }

    /// Replace the value beneath every explicit memory form.
    pub(in crate::sema) fn replace_form_value(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // substitute the new value once the last form is behind us
        let head = self.normalize(origin, ty)?;
        let dir::Type::Form(form) = self.ty(head)? else {
            return Ok(value);
        };

        // rebuild this form around the replaced payload
        let payload = self.replace_form_value(origin, form.value, value)?;
        let form = self.adopt_form(head.module_id, form.form)?;
        let rebuilt = self.intern_type(dir::Type::Form(dir::FormType {
            form,
            value: payload,
        }))?;

        Ok(rebuilt)
    }

    /// Normalize one component to its canonical memory literal.
    pub(in crate::sema) fn normalize_memory_component(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some(text) = self.memory_component_text(origin, value)? else {
            return Ok(value);
        };

        self.text_literal_type(&text)
    }

    /// Return whether one explicit ownership form redundantly repeats its payload's default.
    ///
    /// Only the ownership constructors participate: borrows and raw pointers always layer.
    pub(in crate::sema) fn is_default_ownership_form(
        &mut self,
        origin: Origin,
        form: dir::Form,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // only the ownership constructors can repeat a default
        if !matches!(form, dir::Form::Owned | dir::Form::Managed { .. }) {
            return Ok(false);
        }

        // a managed form is the default only at the value's own pinned space
        if let dir::Form::Managed { place } = form {
            let pinned = match self.resolved_ty(value)? {
                dir::Type::Application(instance) => self.nominal_space(instance.symbol)?,
                dir::Type::Reference(reference) => self.nominal_space(reference.symbol)?,
                _ => None,
            };
            let Some(pinned) = pinned else {
                return Ok(false);
            };
            if self.place_space(place)? != Some(pinned) {
                return Ok(false);
            }
        }

        // compare the written ownership against the payload's own default
        let Some(default) = self.default_ownership(origin, value)? else {
            return Ok(false);
        };

        Ok(form.ownership() == Some(default))
    }

    /// Drop one type's redundant explicit forms, keeping its authored payload.
    pub(in crate::sema) fn reduce_redundant_forms(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut id = self.shallow_resolve(id)?;
        loop {
            let dir::Type::Form(form) = self.ty(id)? else {
                return Ok(id);
            };

            // drop an ownership form repeating the payload's default
            let value = self.normalize(origin, form.value)?;
            let mut active = SmallVec::new();
            let is_redundant = self.is_default_ownership_form(origin, form.form, value)?
                || (form.form == dir::Form::Readonly
                    && self.is_immutable(origin, value, &mut active)?);
            if !is_redundant {
                return Ok(id);
            }
            id = self.shallow_resolve(form.value)?;
        }
    }

    /// Reduce form constructors redundantly repeating family defaults to their payloads.
    pub(in crate::sema) fn reduce_default_ownership_chain(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let resolved = self.shallow_resolve(id)?;
        let dir::Type::Form(form) = self.ty(resolved)? else {
            return Ok(resolved);
        };

        // reduce beneath first so a stacked default collapses fully
        let value = self.reduce_default_ownership_chain(origin, form.value)?;

        // drop the constructor when it redundantly repeats the payload's default ownership
        if self.is_default_ownership_form(origin, form.form, value)? {
            return Ok(value);
        }

        // keep the authored type when nothing beneath reduced
        if value == form.value {
            return Ok(resolved);
        }

        // rebuild around the reduced payload, adopting the constructor into this module
        let constructor = self.adopt_form(resolved.module_id, form.form)?;
        self.intern_type(dir::Type::Form(dir::FormType {
            form: constructor,
            value,
        }))
    }

    /// Evaluate one memory accessor, distributing over union targets.
    pub(in crate::sema) fn reduce_memory_accessor(
        &mut self,
        origin: Origin,
        module: ModuleId,
        item: dir::LanguageItem,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // read the accessor's inspected target
        let Some(target) = self.type_ids(module, instance.arguments)?.first().copied() else {
            return Ok(None);
        };

        // close the inspected target first
        let target = self.normalize(origin, target)?;

        // distribute the accessor over union targets
        let elements = match self.ty(target)? {
            dir::Type::Union(union) => {
                SmallVec::<[_; 4]>::from_slice(self.type_ids(target.module_id, union.elements)?)
            }
            _ => SmallVec::from_slice(&[target]),
        };

        // evaluate the accessor over each element on its own
        let mut reduced = Vec::with_capacity(elements.len());
        for element in elements {
            let element = self.normalize(origin, element)?;

            match self.reduce_element_accessor(origin, module, item, instance, element)? {
                // keep the whole accessor symbolic for one symbolic element
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

        // join what the elements left
        let joined = match kept.as_slice() {
            // every element answered never
            [] => self.intern_type(dir::Type::Never)?,
            // the elements agreed on one answer
            [single] => *single,
            // the answers differ, so join them
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
        // apply the memory item the instance names
        match item {
            // reborrow with the written access
            dir::LanguageItem::WithAccess => {
                self.with_access(origin, module, instance, chain, element)
            }
            // project the chain's concrete referent space
            dir::LanguageItem::PlaceOf => self.place_of_chain(chain),
            // project the chain's concrete access mode
            dir::LanguageItem::AccessOf => self.access_of_chain(chain),
            // leave every other item alone
            _ => Ok(None),
        }
    }

    /// Project one chain's referent space, staying stuck while the space is opaque.
    fn place_of_chain(&mut self, chain: &FormChain) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(place) = chain.place() else {
            return Ok(None);
        };
        let Some(space) = self.place_space(place)? else {
            return Ok(None);
        };

        Ok(Some(self.text_literal_type(space.text())?))
    }

    /// Project one chain's access mode, staying stuck while the chain is open.
    fn access_of_chain(&mut self, chain: &FormChain) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // borrows answer with their own concrete access
        let borrow = chain
            .forms
            .iter()
            .find_map(|entry| match entry.form {
                dir::Form::Borrowed(borrow) => Some(borrow),
                _ => None,
            })
            .map(|borrow| self.type_borrow(self.module_id, borrow))
            .transpose()?;
        let access = match borrow {
            Some(borrow) => self.access_of(borrow.access)?,
            // leave an open chain opaque, it can still gain forms
            None if chain.is_open => None,
            // a readonly view answers with its own access
            None if chain.is_readonly() => Some(dir::Access::Readonly),
            // an owning value answers mutable
            None => Some(dir::Access::Mutable),
        };
        let Some(access) = access else {
            return Ok(None);
        };

        Ok(Some(self.text_literal_type(access.text())?))
    }

    /// Walk one element's form chain down to its unqualified base.
    pub(in crate::sema) fn form_chain(
        &mut self,
        origin: Origin,
        element: dir::GlobalTypeId,
    ) -> CompilerResult<FormChain> {
        let mut forms = SmallVec::<[dir::FormType; 2]>::new();
        let mut current = element;
        let mut place = None;
        let mut region = None;

        // collect memory forms outermost first, recording the first referent place and region
        loop {
            let root = self.shallow_resolve(current)?;

            // resolve representation applications so no head hides its components
            let root = match self.ty(root)? {
                dir::Type::Form(_) => root,
                _ => {
                    let reduced = self.normalize(origin, root)?;

                    self.shallow_resolve(reduced)?
                }
            };
            current = root;

            // stop at the first head that carries no memory form
            let dir::Type::Form(form) = self.ty(current)? else {
                break;
            };

            // rehome borrow payloads so chain rebuilds stay module-local
            let head = match form.form {
                dir::Form::Borrowed(borrow) => {
                    let borrow = self.type_borrow(current.module_id, borrow)?;
                    if region.is_none() {
                        region = Some(borrow.region);
                    }
                    if place.is_none()
                        && let resolved = self.shallow_resolve(borrow.region)?
                        && let dir::Type::Region(region) = self.ty(resolved)?
                    {
                        place = Some(region.space);
                    }

                    self.intern_borrow(borrow.region, borrow.access)?
                }
                head => {
                    if let dir::Form::Managed { place: managed } = head
                        && place.is_none()
                    {
                        place = Some(managed);
                    }

                    head
                }
            };
            forms.push(dir::FormType {
                form: head,
                value: form.value,
            });

            current = form.value;
        }

        // fat pointer bases carry their own referent place
        if place.is_none() {
            place = match self.ty(current)? {
                dir::Type::Slice(slice) => Some(slice.place),
                dir::Type::Dynamic(dynamic) => Some(dynamic.place),
                dir::Type::Function(function) => Some(function.place),
                _ => None,
            };
        }

        // open and unreduced bases can gain forms when inference settles
        let is_open = self.ty(current)?.is_open();

        // nominal declarations contribute their intrinsic or inherited space
        if place.is_none() {
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
                place = Some(self.text_literal_type(space.text())?);
            }
        }

        Ok(FormChain {
            forms,
            base: current,
            place,
            region,
            is_open,
        })
    }

    /// Settle one chain's explicit or default ownership.
    pub(in crate::sema) fn form_ownership(
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

    /// Return one reduced type's default ownership.
    pub(in crate::sema) fn default_ownership(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::Ownership>> {
        // read the ownership each family defaults to
        let ty = self.normalize(origin, ty)?;
        let default = match self.ty(ty)? {
            // the object families live behind a managed handle
            dir::Type::Unknown
            | dir::Type::Object(_)
            | dir::Type::Dynamic(_)
            | dir::Type::Slice(_)
            | dir::Type::Function(_) => Some(dir::Ownership::Managed),
            // the value families sit in their holder's own storage
            dir::Type::Never
            | dir::Type::Void
            | dir::Type::Undefined
            | dir::Type::Null
            | dir::Type::Static(_)
            | dir::Type::Intrinsic
            | dir::Type::Key(_)
            | dir::Type::Region(_)
            | dir::Type::Range(_)
            | dir::Type::Tuple(_)
            | dir::Type::FixedArray(_)
            | dir::Type::FunctionPointer(_) => Some(dir::Ownership::Owned),
            // a primitive follows the representation item standing behind it
            dir::Type::Primitive(primitive) => {
                if primitive.representation_item().is_some() {
                    Some(dir::Ownership::Managed)
                } else {
                    Some(dir::Ownership::Owned)
                }
            }
            // a nominal follows the declaration it names
            dir::Type::Application(dir::GenericApplication { symbol, .. })
            | dir::Type::Reference(dir::TypeReference { symbol }) => {
                match self.definition(symbol)?.as_deref() {
                    Some(dir::Definition::Class(_) | dir::Definition::Interface(_)) => {
                        Some(dir::Ownership::Managed)
                    }
                    Some(dir::Definition::Struct(_) | dir::Definition::Enum(_)) => {
                        Some(dir::Ownership::Owned)
                    }
                    Some(dir::Definition::Newtype(definition)) => {
                        let backing = self.normalize(origin, definition.backing)?;

                        return self.default_ownership(origin, backing);
                    }
                    _ => None,
                }
            }
            // a variant follows its owning enum
            dir::Type::Variant(variant) => {
                return self.default_ownership(origin, variant.owner);
            }
            dir::Type::Form(form) => match form.form {
                // a readonly view leaves ownership to the value beneath
                dir::Form::Readonly => {
                    return self.default_ownership(origin, form.value);
                }
                // take an explicit form as the value's own ownership
                dir::Form::Managed { .. }
                | dir::Form::Owned
                | dir::Form::Borrowed(_)
                | dir::Form::Raw => form.form.ownership(),
            },
            // the open and symbolic types answer once inference settles them
            dir::Type::Parameter(_)
            | dir::Type::Erased(_)
            | dir::Type::Variable(_)
            | dir::Type::This
            | dir::Type::Member(_)
            | dir::Type::Operation(_)
            | dir::Type::FunctionSignature(_)
            | dir::Type::Union(_)
            | dir::Type::Error => None,
            // intersections take the ownership their elements agree on
            dir::Type::Intersection(intersection) => {
                let mut agreed = None;
                let elements = self.type_ids(ty.module_id, intersection.elements)?;
                for element in elements {
                    let Some(default) = self.default_ownership(origin, *element)? else {
                        continue;
                    };
                    match agreed {
                        Some(ownership) if ownership != default => return Ok(None),
                        _ => agreed = Some(default),
                    }
                }

                agreed
            }
            // literals use the ownership of their runtime scalar representation
            dir::Type::Literal(literal) => {
                let representation = self.intern_type(literal.widen())?;

                return self.default_ownership(origin, representation);
            }
            // refinements share their base's default form
            dir::Type::Refined(refined) => {
                let refined = self.type_refined(ty.module_id, refined)?;

                return self.default_ownership(origin, refined.base);
            }
        };

        Ok(default)
    }

    /// Return whether two related types differ only by concrete placement.
    pub(in crate::sema) fn is_place_relabel(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // require both chains to stand over the same base
        let source_chain = self.form_chain(origin, source)?;
        let target_chain = self.form_chain(origin, target)?;
        if source_chain.base != target_chain.base {
            return Ok(false);
        }

        // require two concrete places that actually differ
        let (Some(source_place), Some(target_place)) = (source_chain.place(), target_chain.place())
        else {
            return Ok(false);
        };
        if source_place == target_place {
            return Ok(false);
        }

        // require the chains to agree on every form other than placement
        let local = self.local_place()?;
        let placeless =
            |state: &mut Self, chain: &FormChain| -> CompilerResult<Vec<dir::FormType>> {
                let mut forms = Vec::with_capacity(chain.forms.len());
                for entry in &chain.forms {
                    let form = match entry.form {
                        dir::Form::Managed { .. } => dir::Form::Managed { place: local },
                        // rehome the borrow so it resolves in the checking module
                        dir::Form::Borrowed(borrow) => {
                            let borrow = state.type_borrow(state.module_id, borrow)?;
                            let region = state.shallow_resolve(borrow.region)?;
                            let region = match state.ty(region)? {
                                dir::Type::Region(region) => {
                                    state.intern_region(region.extent, local)?
                                }
                                _ => borrow.region,
                            };

                            state.intern_borrow(region, borrow.access)?
                        }
                        form => form,
                    };
                    forms.push(dir::FormType {
                        form,
                        value: entry.value,
                    });
                }

                Ok(forms)
            };
        let source_forms = placeless(self, &source_chain)?;
        let target_forms = placeless(self, &target_chain)?;

        Ok(source_forms == target_forms)
    }

    /// Rebuild one form chain over a new base, innermost first.
    pub(in crate::sema) fn wrap_forms(
        &mut self,
        forms: &[dir::Form],
        base: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut current = base;

        // layer the forms back on from the innermost outward
        for form in forms.iter().rev() {
            current = self.intern_type(dir::Type::Form(dir::FormType {
                form: *form,
                value: current,
            }))?;
        }

        Ok(current)
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
        // read the requested access out of the accessor's arguments
        let Some(access) = self.type_ids(module, instance.arguments)?.get(1).copied() else {
            return Ok(None);
        };
        let access = self.normalize_memory_component(origin, access)?;

        // borrow access lives on the borrow form itself
        if chain
            .forms
            .iter()
            .any(|entry| matches!(entry.form, dir::Form::Borrowed(_)))
        {
            return self.replace_borrow(chain, None, Some(access));
        }

        // outside a borrow, access rides on a readonly form over the same value
        let requested = match self.ty(access)? {
            dir::Type::Literal(dir::Literal::String(value)) => dir::Access::from_text(value),
            _ => None,
        };
        // layer the requested access over the chain
        match requested {
            // layer a readonly form over a value that lacks one
            Some(dir::Access::Readonly) => {
                if chain
                    .forms
                    .iter()
                    .any(|entry| matches!(entry.form, dir::Form::Readonly))
                {
                    Ok(Some(element))
                } else {
                    let mut forms = chain
                        .forms
                        .iter()
                        .map(|entry| entry.form)
                        .collect::<SmallVec<[_; 2]>>();
                    forms.push(dir::Form::Readonly);

                    Ok(Some(self.wrap_forms(&forms, chain.base)?))
                }
            }
            // strip the readonly views back off
            Some(dir::Access::Mutable) => {
                let forms = chain
                    .forms
                    .iter()
                    .map(|entry| entry.form)
                    .filter(|form| !matches!(form, dir::Form::Readonly))
                    .collect::<SmallVec<[_; 2]>>();

                Ok(Some(self.wrap_forms(&forms, chain.base)?))
            }
            // stay stuck while the requested access is opaque
            _ => Ok(None),
        }
    }

    /// Rebuild one chain's borrow form with replaced components.
    fn replace_borrow(
        &mut self,
        chain: &FormChain,
        lifetime: Option<dir::GlobalTypeId>,
        access: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // find the borrow form the replacement lands on
        let position = chain
            .forms
            .iter()
            .position(|entry| matches!(entry.form, dir::Form::Borrowed(_)));
        let Some(position) = position else {
            return Ok(None);
        };

        // rebuild the borrow, keeping every component the caller left out
        let mut forms = chain
            .forms
            .iter()
            .map(|entry| entry.form)
            .collect::<SmallVec<[_; 2]>>();
        if let dir::Form::Borrowed(borrow) = forms[position] {
            let borrow = self.type_borrow(self.module_id, borrow)?;
            forms[position] = self.intern_borrow(
                lifetime.unwrap_or(borrow.region),
                access.unwrap_or(borrow.access),
            )?;
        }

        Ok(Some(self.wrap_forms(&forms, chain.base)?))
    }

    /// Return the text behind one closed memory component.
    pub(in crate::sema) fn memory_component_text(
        &mut self,
        origin: Origin,
        component: dir::GlobalTypeId,
    ) -> CompilerResult<Option<String>> {
        let component = self.normalize(origin, component)?;

        // a closed component settles at the string literal naming it
        let text = match self.ty(component)? {
            dir::Type::Literal(dir::Literal::String(value)) => {
                Some(self.strings().get(value).to_string())
            }
            _ => None,
        };

        Ok(text)
    }

    /// Push one reserved string literal type, interned once per text.
    fn text_literal_type(&mut self, text: &str) -> CompilerResult<dir::GlobalTypeId> {
        if let Some(id) = self.memory_literals.get(text) {
            return Ok(*id);
        }
        let value = self.strings().intern(text);
        let id = self.intern_type(dir::Type::Literal(dir::Literal::String(value)))?;
        self.memory_literals.insert(text.to_string(), id);

        Ok(id)
    }

    /// Settle one type to its readable value.
    pub(in crate::sema) fn readable_value(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut current = self.shallow_resolve(ty)?;
        while let dir::Type::Form(form) = self.ty(current)? {
            // drop alias-transparent forms for reads
            if !matches!(form.form, dir::Form::Managed { .. } | dir::Form::Readonly) {
                break;
            }
            current = self.shallow_resolve(form.value)?;
        }

        Ok(current)
    }

    /// Rewrite one form composition into its canonical interned order.
    pub(in crate::sema) fn canonical_form_type(
        &mut self,
        ty: dir::Type,
    ) -> CompilerResult<dir::Type> {
        let dir::Type::Form(form) = ty else {
            return Ok(ty);
        };

        // read the payload head through any solved variable
        let payload = self.shallow_resolve(form.value)?;

        // read a readonly view of a constant as the constant
        if form.form == dir::Form::Readonly && self.is_literal_shape(payload)? {
            return self.ty(payload);
        }

        // collapse a managed form over a fat pointer onto the pointer's own place
        if let dir::Form::Managed { place } = form.form {
            match self.ty(payload)? {
                dir::Type::Slice(slice) => {
                    return Ok(dir::Type::Slice(dir::SliceType {
                        element: slice.element,
                        place,
                    }));
                }
                dir::Type::Dynamic(dynamic) => {
                    return Ok(dir::Type::Dynamic(dir::DynamicType {
                        constraint: dynamic.constraint,
                        place,
                    }));
                }
                dir::Type::Function(function) => {
                    return Ok(dir::Type::Function(dir::FunctionType { place, ..function }));
                }
                _ => {}
            }
        }

        // require a form standing beneath the outer form
        let dir::Type::Form(payload_form) = self.ty(payload)? else {
            return Ok(ty);
        };

        // collapse the two forms where they compose
        match (form.form, payload_form.form) {
            // a managed form over a borrow collapses onto the borrow
            (dir::Form::Managed { place }, dir::Form::Borrowed(borrow)) => {
                let Some(borrow) = self.borrow_maybe(borrow) else {
                    return Ok(ty);
                };
                let region = self.with_region_space(borrow.region, place)?;
                let form = self.intern_borrow(region, borrow.access)?;

                Ok(dir::Type::Form(dir::FormType {
                    form,
                    value: payload_form.value,
                }))
            }
            // own the object beneath a managed value and let the container place it
            (dir::Form::Owned, dir::Form::Managed { .. }) => Ok(dir::Type::Form(dir::FormType {
                form: dir::Form::Owned,
                value: payload_form.value,
            })),
            // readonly views are idempotent
            (dir::Form::Readonly, dir::Form::Readonly) => Ok(dir::Type::Form(payload_form)),
            // absorb an ownership request into the equal ownership below it
            (dir::Form::Managed { place }, dir::Form::Managed { place: existing }) => {
                if place == existing {
                    return Ok(dir::Type::Form(payload_form));
                }

                Ok(dir::Type::Form(dir::FormType {
                    form: dir::Form::Managed { place },
                    value: payload_form.value,
                }))
            }
            // owned forms are idempotent
            (dir::Form::Owned, dir::Form::Owned) => Ok(dir::Type::Form(payload_form)),
            // absorb a readonly payload view into a readonly borrow
            (dir::Form::Borrowed(borrow), dir::Form::Readonly)
                if let Some(row) = self.borrow_maybe(borrow)
                    && self.access_of(row.access)? == Some(dir::Access::Readonly) =>
            {
                Ok(dir::Type::Form(dir::FormType {
                    form: form.form,
                    value: payload_form.value,
                }))
            }
            // leave every other composition as written
            _ => Ok(ty),
        }
    }

    /// Return the concrete access one access singleton names.
    pub(in crate::sema) fn access_of(
        &self,
        access: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::Access>> {
        let access = self.shallow_resolve(access)?;
        let literal = match self.ty(access)? {
            dir::Type::Literal(dir::Literal::String(value)) => dir::Access::from_text(value),
            _ => None,
        };

        Ok(literal)
    }

    /// Return the concrete lifetime one lifetime singleton names.
    pub(in crate::sema) fn lifetime_of(
        &self,
        lifetime: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::Lifetime>> {
        let lifetime = self.shallow_resolve(lifetime)?;
        let literal = match self.ty(lifetime)? {
            dir::Type::Literal(dir::Literal::String(value)) => {
                dir::Lifetime::parse(self.strings().get(value))
            }
            _ => None,
        };

        Ok(literal)
    }

    /// Return the concrete space one place singleton names.
    pub(in crate::sema) fn place_space(
        &self,
        place: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::Space>> {
        // read the place through its solution
        let place = self.shallow_resolve(place)?;

        // a closed place settles at the string literal naming its space
        let space = match self.ty(place)? {
            dir::Type::Literal(dir::Literal::String(value)) => dir::Space::from_text(value),
            _ => None,
        };

        Ok(space)
    }
}
