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
    /// The outermost borrow form's region.
    region: Option<dir::GlobalTypeId>,
    /// Whether the base can still gain forms at instantiation.
    is_open: bool,
}

/// One normalized managed or owned conversion to borrowed form.
pub(in crate::sema) struct BorrowConversion {
    /// The module that interns the normalized borrow constructor.
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

    /// Return the outermost borrow form's region.
    pub(in crate::sema) fn region(&self) -> Option<dir::GlobalTypeId> {
        self.region
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
        if !self.ty(source.base())?.has_runtime_value() {
            return Ok(None);
        }

        // acquire the handle when the payload is itself a borrow
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

    /// Return the concrete space required by a nominal declaration and its heritage.
    pub(in crate::sema) fn nominal_space(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::Space>> {
        // read the remembered space
        if let Some(space) = self.nominal_spaces.get(&symbol) {
            return Ok(*space);
        }

        // derive the space through the heritage walk
        let mut active = FxIndexSet::default();
        let space = self.nominal_space_guarded(symbol, &mut active)?;

        // remember the space once every definition exists
        if !self.is_declaring() {
            self.nominal_spaces.insert(symbol, space);
        }

        Ok(space)
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

        // place a class in local space unless its declaration or heritage says otherwise
        if space.is_none() && matches!(*definition, dir::Definition::Class(_)) {
            space = Some(dir::Space::Local);
        }

        Ok(space)
    }

    /// Return the extent one region names, a bare region term naming itself.
    pub(in crate::sema) fn region_extent(
        &mut self,
        region: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let region = self.shallow_resolve(region)?;
        let extent = match self.ty(region)? {
            // read the extent of a region pair
            dir::Type::Region(pair) => pair.extent,
            // join the extents of every arm
            dir::Type::Union(union) => {
                let regions: SmallVec<[_; 8]> =
                    self.type_ids(region.module_id, union.elements)?.into();
                let mut extents = SmallVec::<[_; 8]>::with_capacity(regions.len());
                for region in regions {
                    extents.push(self.region_extent(region)?);
                }

                self.normalized_union_type(extents)?
            }
            // read a bare region term as its own extent
            _ => region,
        };

        self.shallow_resolve(extent)
    }

    /// Return the space one region names, none while the region is open.
    pub(in crate::sema) fn region_space(
        &mut self,
        region: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let region = self.shallow_resolve(region)?;
        let space = match self.ty(region)? {
            // read the space of a region pair
            dir::Type::Region(pair) => pair.space,
            // join the spaces of every arm
            dir::Type::Union(union) => {
                return self.map_union_arms(region, union, |state, arm| state.region_space(arm));
            }
            // leave an open region without a space
            _ => return Ok(None),
        };

        Ok(Some(space))
    }

    /// Map each arm of one union and join the results, none when any arm maps to none.
    fn map_union_arms(
        &mut self,
        union_type: dir::GlobalTypeId,
        union: dir::UnionType,
        mut map: impl FnMut(&mut Self, dir::GlobalTypeId) -> CompilerResult<Option<dir::GlobalTypeId>>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // map each arm, stopping at the first arm without a result
        let arms: SmallVec<[_; 8]> = self.type_ids(union_type.module_id, union.elements)?.into();
        let mut mapped = SmallVec::<[_; 8]>::with_capacity(arms.len());
        for arm in arms {
            let Some(arm) = map(self, arm)? else {
                return Ok(None);
            };
            mapped.push(arm);
        }

        self.normalized_union_type(mapped).map(Some)
    }

    /// Normalize one type naming an alias or an operation, keeping every other type.
    pub(in crate::sema) fn normalize_named(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let resolved = self.shallow_resolve(ty)?;
        if !self.is_named_type(resolved)? {
            return Ok(ty);
        }
        let expanded = self.normalize(origin, resolved)?;
        let expanded = self.shallow_resolve(expanded)?;
        if expanded != resolved {
            return Ok(expanded);
        }

        // expand an alias to the value it names
        let instance = match self.ty(resolved)? {
            dir::Type::Application(instance) => Some(instance),
            dir::Type::Reference(reference) => Some(self.declaration_instance(reference.symbol)?),
            _ => None,
        };
        let body = match instance {
            Some(instance) => self.type_alias_body(origin, resolved.module_id, &instance)?,
            None => None,
        };
        match body {
            Some(body) => self.normalize(origin, body),
            None => Ok(resolved),
        }
    }

    /// Return whether one head names a type through an alias or a projection.
    fn is_named_type(&mut self, ty: dir::GlobalTypeId) -> CompilerResult<bool> {
        Ok(match self.ty(ty)? {
            dir::Type::Reference(dir::TypeReference { symbol, .. })
            | dir::Type::Application(dir::GenericApplication { symbol, .. }) => {
                let symbol = self.resolve_symbol_alias(symbol)?;

                self.symbol_kind(symbol)? == dir::SymbolKind::TypeAlias
            }
            dir::Type::Member(_) | dir::Type::Operation(_) => true,
            _ => false,
        })
    }

    /// Return the component an elided memory position takes when nothing decides it.
    pub(in crate::sema) fn elided_memory_default(
        &mut self,
        kind: dir::MemoryParameter,
    ) -> CompilerResult<dir::GlobalTypeId> {
        match kind {
            dir::MemoryParameter::Region => self.lifetime_literal(dir::Lifetime::Frame),
            dir::MemoryParameter::Access => self.access_literal(dir::Access::Readonly),
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

    /// Borrow one value at a region and access, a readonly view clamping the access.
    pub(in crate::sema) fn borrow_value(
        &mut self,
        region: dir::GlobalTypeId,
        mut access: dir::GlobalTypeId,
        mut value: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        loop {
            value = self.shallow_resolve(value)?;
            let dir::Type::Form(payload) = self.ty(value)? else {
                break;
            };
            match payload.form {
                dir::Form::Readonly => access = self.access_literal(dir::Access::Readonly)?,
                // reborrow a borrowed payload at the clamped access
                dir::Form::Borrowed(payload_borrow) => {
                    let payload_access = self.type_borrow(value.module_id, payload_borrow)?.access;
                    let payload_access = self.shallow_resolve(payload_access)?;
                    if self.access_of(payload_access)? == Some(dir::Access::Readonly) {
                        access = payload_access;
                    }
                }
                dir::Form::Owned => {}
                dir::Form::Raw => break,
            }
            value = payload.value;
        }
        let form = self.intern_borrow(region, access)?;

        self.intern_type(dir::Type::Form(dir::FormType { form, value }))
    }

    /// Return the value one type stores beneath its ownership forms, a borrow staying whole.
    pub(in crate::sema) fn ownership_payload(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut value = self.normalize(origin, id)?;
        while let dir::Type::Form(form) = self.ty(value)?
            && matches!(form.form, dir::Form::Owned | dir::Form::Readonly)
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
        // keep the storage forms of a construction, stopping at a reference or an open variable
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
            let is_redundant = self.is_redundant_owned(origin, form.form, value)?
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
        if self.is_redundant_owned(origin, form.form, value)? {
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

        // evaluate the accessor over each element separately
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
            // project the chain's concrete access mode
            dir::LanguageItem::AccessOf => self.access_of_chain(chain),
            // leave every other item alone
            _ => Ok(None),
        }
    }

    /// Project one chain's access mode, staying stuck while the chain is open.
    fn access_of_chain(&mut self, chain: &FormChain) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // answer a borrow with its concrete access
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
            // answer a readonly view with its access
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
        let mut region = None;

        // collect memory forms outermost first, recording the outermost borrow region
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

                    self.intern_borrow(borrow.region, borrow.access)?
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

        Ok(FormChain {
            forms,
            base: current,
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
            None => self.default_ownership(origin, chain.base)?,
        };

        Ok(ownership)
    }

    /// Return the space one type's declaration fixes, read through its forms.
    pub(in crate::sema) fn type_space(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::Space>> {
        let ty = self.shallow_resolve(ty)?;
        match self.ty(ty)? {
            dir::Type::Application(instance) => self.nominal_space(instance.symbol),
            dir::Type::Reference(reference) => self.nominal_space(reference.symbol),
            dir::Type::Form(form) => self.type_space(form.value),
            _ => Ok(None),
        }
    }

    /// Return the space term one value type stores in, none while the type stays open.
    pub(in crate::sema) fn reduce_space_of(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let target = self.normalize(origin, target)?;
        match self.ty(target)? {
            // open types settle their space once they close
            dir::Type::Variable(_)
            | dir::Type::Parameter(_)
            | dir::Type::Member(_)
            | dir::Type::Operation(_)
            | dir::Type::This => Ok(None),
            // forms store where their value does
            dir::Type::Form(form) => self.reduce_space_of(origin, form.value),
            // unions store in the space of every arm
            dir::Type::Union(union) => self.map_union_arms(target, union, |state, arm| {
                state.reduce_space_of(origin, arm)
            }),
            // nominals store where their declaration says, every other value locally
            _ => {
                let space = self.type_space(target)?.unwrap_or(dir::Space::Local);

                self.space_literal(space).map(Some)
            }
        }
    }

    /// Return the space term one value type stores in, a `SpaceOf` operation while it stays open.
    pub(in crate::sema) fn space_term(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // read the space of a closed type
        if let Some(space) = self.reduce_space_of(origin, ty)? {
            return Ok(space);
        }

        self.intern_operation(dir::TypeOperation::SpaceOf(dir::UnaryType { target: ty }))
    }

    /// Return one reduced type's default ownership.
    pub(in crate::sema) fn default_ownership(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::Ownership>> {
        let ty = self.normalize(origin, ty)?;

        self.ownership(ty)
    }

    /// Return whether one form is an owned form its value already defaults to.
    pub(in crate::sema) fn is_redundant_owned(
        &mut self,
        origin: Origin,
        form: dir::Form,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // repeat a default with an owned form alone
        if form != dir::Form::Owned {
            return Ok(false);
        }

        Ok(self.default_ownership(origin, value)? == Some(dir::Ownership::Owned))
    }

    /// Return the ownership one type's head defaults to, read off the head as given.
    pub(in crate::sema) fn ownership(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::Ownership>> {
        // read the default ownership of each head
        let default = match self.ty(ty)? {
            // the object heads and the erased top live behind a managed handle
            dir::Type::Unknown
            | dir::Type::Object(_)
            | dir::Type::Dynamic(_)
            | dir::Type::Slice(_)
            | dir::Type::Function(_)
            | dir::Type::FunctionSignature(_) => Some(dir::Ownership::Managed),
            // keep the value families in their holder's storage
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
            // a nominal follows its declaration kind, a newtype or alias the type it names
            dir::Type::Application(dir::GenericApplication { symbol, .. })
            | dir::Type::Reference(dir::TypeReference { symbol, .. }) => {
                match self.symbol_kind(symbol)? {
                    dir::SymbolKind::Class
                    | dir::SymbolKind::Interface
                    | dir::SymbolKind::NewtypeInterface => Some(dir::Ownership::Managed),
                    dir::SymbolKind::Struct | dir::SymbolKind::Enum => Some(dir::Ownership::Owned),
                    dir::SymbolKind::Newtype | dir::SymbolKind::TypeAlias => {
                        return match self.definition(symbol)?.as_deref() {
                            Some(dir::Definition::Newtype(definition)) => {
                                self.ownership(definition.backing)
                            }
                            Some(dir::Definition::TypeAlias(alias)) => self.ownership(alias.value),
                            // stay open while an isolated declare pass cannot see the definition
                            None if self.is_declaring() => Ok(None),
                            _ => Err(CompilerError::Internal {
                                message: format!(
                                    "ownership read before the definition of '{}' exists",
                                    self.format_symbol(symbol)
                                ),
                            }),
                        };
                    }
                    _ => None,
                }
            }
            // a variant follows its owning enum
            dir::Type::Variant(variant) => {
                return self.ownership(variant.owner);
            }
            dir::Type::Form(form) => match form.form {
                // a readonly view leaves ownership to the value beneath
                dir::Form::Readonly => {
                    return self.ownership(form.value);
                }
                // take an explicit form as the value's ownership
                dir::Form::Owned | dir::Form::Borrowed(_) | dir::Form::Raw => form.form.ownership(),
            },
            // leave open, symbolic, and top types without a default ownership
            dir::Type::Parameter(_)
            | dir::Type::Erased(_)
            | dir::Type::Variable(_)
            | dir::Type::This
            | dir::Type::Member(_)
            | dir::Type::Operation(_)
            | dir::Type::Union(_)
            | dir::Type::Error => None,
            // intersections take the ownership their elements agree on
            dir::Type::Intersection(intersection) => {
                let mut agreed = None;
                let elements = self.type_ids(ty.module_id, intersection.elements)?;
                for element in elements {
                    let Some(default) = self.ownership(*element)? else {
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

                return self.ownership(representation);
            }
            // refinements share their base's default form
            dir::Type::Refined(refined) => {
                let refined = self.type_refined(ty.module_id, refined)?;

                return self.ownership(refined.base);
            }
        };

        Ok(default)
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
            dir::Type::Literal(dir::Literal::String(value)) => {
                dir::Access::from_text(self.strings().get(value))
            }
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
    pub(in crate::sema) fn text_literal_type(
        &mut self,
        text: &str,
    ) -> CompilerResult<dir::GlobalTypeId> {
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
            if !matches!(form.form, dir::Form::Readonly) {
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

        // require a form standing beneath the outer form
        let dir::Type::Form(payload_form) = self.ty(payload)? else {
            return Ok(ty);
        };

        // collapse the two forms where they compose
        match (form.form, payload_form.form) {
            // readonly and owned forms are idempotent
            (dir::Form::Readonly, dir::Form::Readonly) | (dir::Form::Owned, dir::Form::Owned) => {
                Ok(dir::Type::Form(payload_form))
            }
            // borrow the value beneath an owned form
            (dir::Form::Borrowed(_), dir::Form::Owned) => Ok(dir::Type::Form(dir::FormType {
                form: form.form,
                value: payload_form.value,
            })),
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
            dir::Type::Literal(dir::Literal::String(value)) => {
                dir::Access::from_text(self.strings().get(value))
            }
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

    /// Return the space one space term names, none while the term stays open.
    pub(in crate::sema) fn literal_space(
        &self,
        term: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::Space>> {
        // read the term through its solution
        let term = self.shallow_resolve(term)?;

        // read the space a closed term's string literal names
        let space = match self.ty(term)? {
            dir::Type::Literal(dir::Literal::String(value)) => {
                dir::Space::from_text(self.strings().get(value))
            }
            _ => None,
        };

        Ok(space)
    }
}
