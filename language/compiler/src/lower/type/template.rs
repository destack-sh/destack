use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;
use destack_mir as mir;

use crate::lower::ModuleLowerer;
use crate::{CompilerError, CompilerResult};

/// What `this` names inside the bounds of one template's parameters.
#[derive(Clone, Copy)]
pub(in crate::lower) enum BoundReceiver {
    /// No receiver.
    None,
    /// The receiver one callable declares.
    OfCallable(dir::GlobalSymbolId),
    /// One lowered receiver type.
    Lowered(mir::TypeId),
}

/// One dependent's index and domain in the parameter space of a definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::lower) struct Dependent {
    /// The dependent type as first recorded.
    pub(in crate::lower) ty: dir::GlobalTypeId,
    /// The generic index the dependent takes.
    pub(in crate::lower) index: u32,
    /// The memory kind the dependent qualifies, none for a type dependent.
    pub(in crate::lower) kind: Option<dir::MemoryParameter>,
}

/// The identity one dependent is recorded under: a projection by its owner and key, else its type.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::lower) enum DependentKey {
    /// A member projection by owner, key, and qualifier.
    Projection {
        /// The projected owner.
        owner: dir::GlobalTypeId,
        /// The projected member key.
        key: dir::StaticKey,
        /// The qualifying interface declaration.
        qualifier: Option<dir::GlobalSymbolId>,
    },
    /// Any other dependent type by identity.
    Type(dir::GlobalTypeId),
}

impl DependentKey {
    /// Return the identity one dependent type is recorded under.
    pub(in crate::lower) fn of(
        lower: &mut ModuleLowerer<'_>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Self> {
        Ok(match lower.member_projection(ty)? {
            Some((member, qualifier)) => Self::Projection {
                owner: member.owner,
                key: member.key,
                qualifier,
            },
            None => Self::Type(ty),
        })
    }
}

/// The lifetime slots and parameter indices one definition lowers under.
#[derive(Clone, Default)]
pub(in crate::lower) struct GenericScope {
    /// The index of each instance parameter, owner parameters first, the receiver among them.
    pub(in crate::lower) parameters: FxIndexMap<dir::GlobalGenericParameterId, u32>,
    /// The index of the interface receiver parameter, on interface member templates.
    pub(in crate::lower) receiver: Option<u32>,
    /// The declared type of the receiver the scope's callable leads with.
    pub(in crate::lower) this_parameter: Option<dir::GlobalTypeId>,
    /// The type an extension member's `this` names, the extension's target.
    pub(in crate::lower) extension_target: Option<dir::GlobalTypeId>,
    /// Each dependent the templates declare by its identity, indexed after the parameters.
    pub(in crate::lower) dependents: FxIndexMap<DependentKey, Dependent>,
    /// The argument each index takes under a grounded application; empty outside one.
    pub(in crate::lower) grounding: Vec<mir::GenericArgument>,
    /// The number of indices the owner template takes ahead of the signature's own.
    pub(in crate::lower) owner_count: u32,
    /// The function-local slot of each lifetime parameter.
    pub(in crate::lower) slots: FxIndexMap<dir::GlobalGenericParameterId, mir::RegionBound>,
    /// The declared name of each slot, without the tick.
    pub(in crate::lower) names: Vec<String>,
    /// The declared outlives pairs between slots, left outliving right.
    pub(in crate::lower) outlives: Vec<(mir::RegionBound, mir::RegionBound)>,
    /// The slot a constructor borrows its constructed storage at, after the declared ones.
    pub(in crate::lower) receiver_slot: Option<mir::RegionBound>,
    /// Whether a region parameter outside the scope erases, where keys hold erased types.
    erases_regions: bool,
}

impl GenericScope {
    /// Return the memory kind and const flag of each index from one index on, in index order.
    pub(in crate::lower) fn index_domains(
        &self,
        lower: &mut ModuleLowerer<'_>,
        first: u32,
    ) -> CompilerResult<Vec<(Option<dir::MemoryParameter>, bool)>> {
        // read each parameter's declared kind
        let mut domains = vec![None; self.count().saturating_sub(first) as usize];
        for (parameter, index) in &self.parameters {
            if *index >= first {
                let binding = lower
                    .state(parameter.module_id)?
                    .generics
                    .get_parameter(parameter.local_id);
                domains[(*index - first) as usize] =
                    Some((binding.memory_parameter(), binding.is_const));
            }
        }

        // read each dependent's memory kind
        for dependent in self.dependents.values() {
            if dependent.index >= first {
                domains[(dependent.index - first) as usize] = Some((dependent.kind, false));
            }
        }

        // require a domain at every index
        domains
            .into_iter()
            .zip(first..)
            .map(|(domain, index)| {
                domain.ok_or_else(|| CompilerError::Internal {
                    message: format!("generic index {index} without a parameter"),
                })
            })
            .collect()
    }

    /// Capture the enclosing type and lifetime parameters used by generated code.
    pub(in crate::lower) fn capture(
        &mut self,
        types: impl IntoIterator<Item = dir::GlobalTypeId>,
        enclosing: &Self,
        lower: &mut ModuleLowerer<'_>,
    ) -> CompilerResult<()> {
        // visit each type once, in the order supplied
        let mut pending: FxIndexSet<_> = types.into_iter().collect();
        let mut slots = vec![None; enclosing.names.len()];
        let mut position = 0;
        while position < pending.len() {
            let id = pending[position];
            position += 1;

            // retain dependent types as the parameters already selected by sema
            let key = DependentKey::of(lower, id)?;
            if let Some(enclosed) = enclosing.dependents.get(&key) {
                let index = self.count();
                self.dependents.entry(key).or_insert(Dependent {
                    ty: id,
                    index,
                    kind: enclosed.kind,
                });

                continue;
            }

            // collect explicit parameters and the types their declarations require
            let ty = lower.ty(id)?;
            if let dir::Type::Parameter(parameter) = ty {
                if enclosing.parameters.contains_key(&parameter) {
                    let index = self.count();
                    self.parameters.entry(parameter).or_insert(index);
                    if enclosing.parameter_index(parameter) == enclosing.receiver {
                        self.receiver = self.parameter_index(parameter);
                    }
                } else if let Some(slot) = enclosing.slots.get(&parameter)
                    && !self.slots.contains_key(&parameter)
                {
                    let index = mir::RegionBound::new(self.names.len() as u32);
                    self.slots.insert(parameter, index);
                    self.names
                        .push(enclosing.names[slot.index as usize].clone());
                }
                if let Some(slot) = enclosing.slots.get(&parameter) {
                    slots[slot.index as usize] = self.slots.get(&parameter).copied();
                }

                // include the types and lifetimes the filled bounds and where clauses require
                let state = lower.state(parameter.module_id)?;
                let binding = state.generics.get_parameter(parameter.local_id);
                if let Some(bounds) = state.generics.parameter_bounds(parameter.local_id) {
                    pending.extend(state.types.type_ids(bounds));
                }
                if let Some(bounds) = state
                    .generics
                    .assumed_bounds(binding.template, parameter.local_id)
                {
                    pending.extend(state.types.type_ids(bounds));
                }
            } else {
                lower.types(id.module_id)?.for_each_child(&ty, |child| {
                    pending.insert(child);
                });
            }
        }

        // translate the retained lifetime relations into this function's indices
        for (left, right) in &enclosing.outlives {
            if let (Some(left), Some(right)) =
                (slots[left.index as usize], slots[right.index as usize])
                && !self.outlives.contains(&(left, right))
            {
                self.outlives.push((left, right));
            }
        }

        Ok(())
    }

    /// Append the slot a constructor borrows its constructed storage at.
    pub(in crate::lower) fn push_receiver_slot(&mut self) -> mir::RegionBound {
        let name = dir::free_region_name(self.names.iter().map(String::as_str));
        let slot = mir::RegionBound::new(self.names.len() as u32);
        self.names.push(name);
        self.receiver_slot = Some(slot);

        slot
    }

    /// Return this scope with every region erased.
    pub(in crate::lower) fn erased(&self) -> Self {
        Self {
            erases_regions: true,
            ..self.clone()
        }
    }

    /// Collect the parameters one signature closes over.
    pub(in crate::lower) fn for_signature(
        lower: &mut ModuleLowerer<'_>,
        template: Option<dir::GlobalGenericTemplateId>,
        declaration: Option<dir::GlobalSymbolId>,
    ) -> CompilerResult<Self> {
        // instantiate a declaration's regions, bind a function type's late at each call
        let mut parameters = Self::default();
        if let Some(template) = template {
            parameters.collect_instance_parameters(lower, template, true, declaration.is_some())?;
            parameters.collect_lifetimes(lower, template)?;
        }
        if let Some(declaration) = declaration {
            parameters.collect_dependents(lower, declaration)?;
        }

        Ok(parameters)
    }

    /// Collect the parameters one type declaration's representation is generic over.
    pub(in crate::lower) fn for_declaration(
        lower: &mut ModuleLowerer<'_>,
        template: dir::GlobalGenericTemplateId,
    ) -> CompilerResult<Self> {
        let mut parameters = Self::default();
        parameters.collect_instance_parameters(lower, template, false, true)?;

        Ok(parameters)
    }

    /// Collect the type parameters of an owner and a signature template, with their lifetimes.
    pub(in crate::lower) fn from_templates(
        lower: &mut ModuleLowerer<'_>,
        owner: Option<dir::GlobalGenericTemplateId>,
        signature: Option<dir::GlobalGenericTemplateId>,
    ) -> CompilerResult<Self> {
        let mut parameters = Self::default();
        if let Some(owner) = owner {
            parameters.collect_instance_parameters(lower, owner, true, true)?;
            parameters.owner_count = parameters.count();
        }
        if let Some(signature) = signature {
            parameters.collect_instance_parameters(lower, signature, true, false)?;
        }
        for template in owner.into_iter().chain(signature) {
            parameters.collect_lifetimes(lower, template)?;
        }

        Ok(parameters)
    }

    /// Index the parameters one template declares, its regions among them when they instantiate.
    fn collect_instance_parameters(
        &mut self,
        lower: &mut ModuleLowerer<'_>,
        template: dir::GlobalGenericTemplateId,
        receiver: bool,
        regions: bool,
    ) -> CompilerResult<()> {
        // index each parameter in declaration order
        let generics = &lower.state(template.module_id)?.generics;
        let declared = generics.get_template(template.local_id);
        for parameter in &declared.parameters {
            let binding = generics.get_parameter(*parameter);
            let is_receiver = binding.origin == dir::GenericParameterOrigin::Receiver;
            let is_generic = match regions {
                true => binding.is_representation_parameter(),
                false => binding.is_instance_parameter(),
            };
            if is_generic && (receiver || !is_receiver) {
                let index = self.count();
                self.parameters
                    .insert(parameter.into_global(template.module_id), index);
                if is_receiver {
                    self.receiver = Some(index);
                }
            }
        }
        let symbol = declared.symbol;

        // index the declaration's dependents after its parameters
        if let Some(symbol) = symbol {
            self.collect_dependents(lower, symbol)?;
        }

        Ok(())
    }

    /// Index the dependents one declaration writes after the parameters collected so far.
    pub(in crate::lower) fn collect_dependents(
        &mut self,
        lower: &mut ModuleLowerer<'_>,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        let generics = &lower.state(symbol.module_id)?.generics;
        let dependents = generics
            .symbol_dependents(symbol.local_id)
            .map(<[_]>::to_vec);
        let Some(dependents) = dependents else {
            return Err(CompilerError::Internal {
                message: format!(
                    "a declaration '{}' without its recorded dependents",
                    lower.symbol_path(symbol)?
                ),
            });
        };
        for dependent in dependents {
            let kind = lower.dependent_memory_kind(symbol, dependent)?;
            let key = DependentKey::of(lower, dependent)?;
            match self.dependents.get_mut(&key) {
                Some(recorded) => {
                    // reject one dependent recorded at two memory kinds
                    if let (Some(kind), Some(held)) = (kind, recorded.kind)
                        && kind != held
                    {
                        return Err(CompilerError::Internal {
                            message: format!(
                                "a dependent of '{}' at two memory kinds",
                                lower.symbol_path(symbol)?
                            ),
                        });
                    }
                    recorded.kind = kind.or(recorded.kind);
                }
                None => {
                    let index = self.count();
                    self.dependents.insert(
                        key,
                        Dependent {
                            ty: dependent,
                            index,
                            kind,
                        },
                    );
                }
            }
        }

        Ok(())
    }

    /// Return the number of parameters collected, the interface receiver included.
    pub(in crate::lower) fn count(&self) -> u32 {
        (self.parameters.len() + self.dependents.len()) as u32
    }

    /// Return these parameters indexed after the parameters of an enclosing definition.
    pub(in crate::lower) fn with_parameters_of(
        mut self,
        enclosing: &Self,
        lower: &mut ModuleLowerer<'_>,
        tree: &mir::Tree,
    ) -> CompilerResult<Self> {
        // remember the nested parameters and the index each of them had
        let nested: Vec<_> = self.parameters.keys().copied().collect();
        let nested_dependents: Vec<_> = self
            .dependents
            .iter()
            .map(|(key, dependent)| (*key, *dependent))
            .collect();
        let reindexed: Vec<_> = nested
            .iter()
            .map(|parameter| self.parameters[parameter])
            .collect();
        let nested_receiver = self.receiver;

        // start from the enclosing definition's parameters
        self.parameters = enclosing.parameters.clone();
        self.dependents = enclosing.dependents.clone();
        self.receiver = enclosing.receiver;
        self.this_parameter = self.this_parameter.or(enclosing.this_parameter);
        self.extension_target = self.extension_target.or(enclosing.extension_target);

        // append the nested region slots after the enclosing ones
        let offset = enclosing.slots.len() as u32;
        let nested_slots: Vec<_> = self.slots.drain(..).collect();
        let nested_names: Vec<_> = self.names.drain(..).collect();
        let nested_outlives: Vec<_> = self.outlives.drain(..).collect();
        self.slots = enclosing.slots.clone();
        self.names = enclosing.names.clone();
        self.outlives = enclosing.outlives.clone();
        for (parameter, slot) in nested_slots {
            self.slots
                .insert(parameter, mir::RegionBound::new(slot.index + offset));
        }
        self.names.extend(nested_names);
        for (left, right) in nested_outlives {
            self.outlives.push((
                mir::RegionBound::new(left.index + offset),
                mir::RegionBound::new(right.index + offset),
            ));
        }

        // append each nested parameter to the enclosing definition
        for (parameter, previous) in nested.into_iter().zip(reindexed) {
            let index = match self.parameters.get(&parameter) {
                Some(index) => *index,
                None => {
                    let index = self.count();
                    self.parameters.insert(parameter, index);
                    index
                }
            };
            if nested_receiver == Some(previous) {
                self.receiver = Some(index);
            }
        }

        // append each nested dependent new to the enclosing definition
        for (key, dependent) in nested_dependents {
            if !self.dependents.contains_key(&key) {
                let index = self.count();
                self.dependents
                    .insert(key, Dependent { index, ..dependent });
            }
        }

        // ground the nested indices at themselves under a grounded enclosing definition
        if !enclosing.grounding.is_empty() {
            self.grounding = enclosing.grounding.clone();
            let nested = self.identity_arguments(lower, tree, enclosing.count())?;
            self.grounding.extend(nested);
        }

        Ok(self)
    }

    /// Nest these parameters beneath one enclosing scope, its region slots one binder out.
    pub(in crate::lower) fn nested_in(
        mut self,
        enclosing: &Self,
        lower: &mut ModuleLowerer<'_>,
        tree: &mir::Tree,
    ) -> CompilerResult<Self> {
        let nested_slots: Vec<_> = self.slots.drain(..).collect();
        let nested_names: Vec<_> = self.names.drain(..).collect();
        let nested_outlives: Vec<_> = self.outlives.drain(..).collect();
        self = self.with_parameters_of(enclosing, lower, tree)?;

        // shift the enclosing slots out one binder and declare the nested slots
        self.slots = enclosing
            .slots
            .iter()
            .map(|(parameter, slot)| {
                let outer = mir::RegionBound {
                    depth: slot.depth + 1,
                    index: slot.index,
                };

                (*parameter, outer)
            })
            .collect();
        self.names = nested_names;
        self.outlives = nested_outlives;
        for (parameter, slot) in nested_slots {
            self.slots.insert(parameter, slot);
        }

        Ok(self)
    }

    /// Return the argument standing for each index from one position on: the index itself.
    pub(in crate::lower) fn identity_arguments(
        &self,
        lower: &mut ModuleLowerer<'_>,
        tree: &mir::Tree,
        from: u32,
    ) -> CompilerResult<Vec<mir::GenericArgument>> {
        let domains = self.index_domains(lower, from)?;
        let mut arguments = Vec::with_capacity(domains.len());
        for ((kind, is_const), index) in domains.into_iter().zip(from..) {
            arguments.push(lower.index_argument(tree, index, kind, is_const)?);
        }

        Ok(arguments)
    }

    /// Return the index of one dependent the templates declare.
    pub(in crate::lower) fn dependent_index(
        &self,
        lower: &mut ModuleLowerer<'_>,
        dependent: dir::GlobalTypeId,
    ) -> CompilerResult<Option<u32>> {
        if self.dependents.is_empty() {
            return Ok(None);
        }
        let key = DependentKey::of(lower, dependent)?;

        Ok(self.dependents.get(&key).map(|dependent| dependent.index))
    }

    /// Return the index of one collected parameter.
    pub(in crate::lower) fn parameter_index(
        &self,
        parameter: dir::GlobalGenericParameterId,
    ) -> Option<u32> {
        self.parameters.get(&parameter).copied()
    }

    /// Collect the lifetime slots declared by one generic template.
    fn collect_lifetimes(
        &mut self,
        lower: &mut ModuleLowerer<'_>,
        template: dir::GlobalGenericTemplateId,
    ) -> CompilerResult<()> {
        // give every declared lifetime parameter its own slot and printed name
        let generics = &lower.state(template.module_id)?.generics;
        let declared = generics.get_template(template.local_id);
        let predicates = declared.predicates.clone();
        let regions: Vec<_> = declared
            .parameters
            .iter()
            .map(|parameter| (*parameter, generics.get_parameter(*parameter)))
            .filter(|(_, binding)| binding.memory_parameter() == Some(dir::MemoryParameter::Region))
            .map(|(parameter, binding)| (parameter, binding.key))
            .collect();
        for (parameter, key) in regions {
            // skip a type definition's lifetime, closed per instance without a slot
            if self
                .parameters
                .contains_key(&parameter.into_global(template.module_id))
            {
                continue;
            }
            let slot = mir::RegionBound::new(self.slots.len() as u32);
            let name = match key {
                dir::GenericParameterKey::Symbol(symbol) => lower.symbol_name(symbol)?,
                dir::GenericParameterKey::Anonymous => None,
            };
            let name = match name {
                // keep tick names verbatim and add a tick to bare names
                Some(name) => {
                    let name = lower.strings.get(name);
                    match name.starts_with('\'') {
                        true => name.to_string(),
                        false => format!("'{name}"),
                    }
                }
                None => dir::free_region_name(self.names.iter().map(String::as_str)),
            };
            self.names.push(name);
            self.slots
                .insert(parameter.into_global(template.module_id), slot);
        }

        // record the declared outlives predicates between the collected slots
        for predicate in &predicates {
            if predicate.relation != dir::WhereRelation::Satisfies {
                continue;
            }

            // record the slots the predicate relates
            let left = self.parameter_slot(lower, predicate.left)?;
            let right = self.parameter_slot(lower, predicate.right)?;
            if let (Some(left), Some(right)) = (left, right) {
                self.outlives.push((left, right));
            }
        }

        Ok(())
    }

    /// Return the slot of one type when it names a collected lifetime parameter.
    fn parameter_slot(
        &mut self,
        lower: &mut ModuleLowerer<'_>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<mir::RegionBound>> {
        let dir::Type::Parameter(parameter) = lower.ty(ty)? else {
            return Ok(None);
        };

        Ok(self.slots.get(&parameter).copied())
    }

    /// Return the outlives slots declared for one slot.
    fn slot_outlives(&self, slot: mir::RegionBound) -> mir::Lifetime {
        mir::Lifetime::new(
            self.outlives
                .iter()
                .filter(|(left, _)| *left == slot)
                .map(|(_, right)| mir::Extent::Bound(*right)),
        )
    }

    /// Declare these lifetime slots on one function header.
    pub(in crate::lower) fn declare<'a>(
        &self,
        mut header: mir::FunctionHeaderBuilder<'a>,
    ) -> mir::FunctionHeaderBuilder<'a> {
        for (slot, name) in self.names.iter().enumerate() {
            let outlives = self.slot_outlives(mir::RegionBound::new(slot as u32));
            header = header.lifetime_outlives(name, outlives);
        }

        header
    }

    /// Return declarations for these lifetime slots.
    pub(in crate::lower) fn declarations(
        &self,
        strings: &destack_core::StringPool,
    ) -> Vec<mir::LifetimeParameter> {
        self.names
            .iter()
            .enumerate()
            .map(|(slot, name)| {
                let name = strings.intern(name);
                let outlives = self.slot_outlives(mir::RegionBound::new(slot as u32));

                mir::LifetimeParameter::with_outlives(Some(name), outlives)
            })
            .collect()
    }
}

impl ModuleLowerer<'_> {
    /// Return whether one generic parameter ranges over regions.
    pub(in crate::lower) fn is_region_parameter(
        &mut self,
        parameter: dir::GlobalGenericParameterId,
    ) -> CompilerResult<bool> {
        let binding = self
            .state(parameter.module_id)?
            .generics
            .get_parameter(parameter.local_id);

        Ok(binding.memory_parameter() == Some(dir::MemoryParameter::Region))
    }

    /// Return whether one parameter is a region an owner declaration closes per instance.
    pub(in crate::lower) fn is_declaration_region_parameter(
        &mut self,
        parameter: dir::GlobalGenericParameterId,
    ) -> CompilerResult<bool> {
        if !self.is_region_parameter(parameter)? {
            return Ok(false);
        }
        let generics = &self.state(parameter.module_id)?.generics;
        let template = generics.get_parameter(parameter.local_id).template;

        // leave an induced template out, it belongs to no declaration
        let Some(symbol) = generics.get_template(template).symbol else {
            return Ok(false);
        };
        let definition = self.definition(symbol)?;

        Ok(matches!(
            definition,
            Some(
                dir::Definition::Struct(_)
                    | dir::Definition::Class(_)
                    | dir::Definition::Enum(_)
                    | dir::Definition::Newtype(_)
                    | dir::Definition::Interface(_)
                    | dir::Definition::Extension(_)
            )
        ))
    }

    /// Return the region generics one region parameter of a type declaration outlives.
    fn region_outlives(
        &mut self,
        parameter: dir::GlobalGenericParameterId,
        scope: &GenericScope,
    ) -> CompilerResult<Vec<u32>> {
        let generics = &self.state(parameter.module_id)?.generics;
        let template = generics.get_parameter(parameter.local_id).template;
        let predicates = generics.get_template(template).predicates.clone();

        // read each satisfies predicate the parameter is the left side of
        let mut outlives = Vec::new();
        for predicate in predicates {
            if predicate.relation != dir::WhereRelation::Satisfies {
                continue;
            }
            let dir::Type::Parameter(left) = self.ty(predicate.left)? else {
                continue;
            };
            let dir::Type::Parameter(right) = self.ty(predicate.right)? else {
                continue;
            };
            if left == parameter
                && let Some(index) = scope.parameter_index(right)
            {
                outlives.push(index);
            }
        }

        Ok(outlives)
    }

    /// Return the name one generic parameter prints under.
    pub(in crate::lower) fn format_parameter_name(
        &mut self,
        parameter: dir::GlobalGenericParameterId,
    ) -> CompilerResult<String> {
        let binding = self
            .state(parameter.module_id)?
            .generics
            .get_parameter(parameter.local_id)
            .clone();
        match binding.key {
            dir::GenericParameterKey::Symbol(symbol) => match self.symbol_name(symbol)? {
                Some(name) => Ok(self.strings.get(name).to_string()),
                None => Err(CompilerError::Internal {
                    message: "a generic parameter without a name".to_string(),
                }),
            },
            dir::GenericParameterKey::Anonymous => {
                Ok(binding.canonical_name(0, std::iter::empty()))
            }
        }
    }

    /// Return the where-clause bounds one callable and its owner assume for a parameter.
    fn where_bounds(
        &mut self,
        callable: dir::GlobalSymbolId,
        parameter: dir::GlobalGenericParameterId,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        // collect the callable's signature template and its owner's template
        let mut templates = Vec::new();
        let ty = self.symbol_type(callable)?;
        let (signature, module) = self.signature(ty)?;
        templates.extend(self.types(module)?.signature(signature).template);
        templates.extend(
            self.state(callable.module_id)?
                .generics
                .template_by_symbol(callable)
                .map(|template| template.into_global(callable.module_id)),
        );
        if let Some(member) = self.imported_member(callable)?
            && let Some(template) = self
                .definition(member.owner)?
                .and_then(|definition| definition.template())
        {
            templates.push(dir::GlobalGenericTemplateId::new(
                member.owner.module_id,
                template,
            ));
        }

        // assume the where clauses of every enclosing template, a closure's enclosing function too
        let mut position = 0;
        while position < templates.len() {
            let template = templates[position];
            position += 1;
            let state = self.state(template.module_id)?;
            let parent = state
                .generics
                .get_template(template.local_id)
                .parent
                .map(|parent| dir::GlobalGenericTemplateId::new(template.module_id, parent));
            if let Some(parent) = parent
                && !templates.contains(&parent)
            {
                templates.push(parent);
            }
        }

        // read the filled bounds each template recorded for the parameter
        let mut bounds = Vec::new();
        for template in templates {
            if template.module_id != parameter.module_id {
                continue;
            }
            let state = self.state(template.module_id)?;
            let Some(list) = state
                .generics
                .assumed_bounds(template.local_id, parameter.local_id)
            else {
                continue;
            };
            for bound in state.types.type_ids(list) {
                if !bounds.contains(bound) {
                    bounds.push(*bound);
                }
            }
        }

        Ok(bounds)
    }

    /// Lower the generic parameters one template declares with their names, domains, and bounds.
    pub(in crate::lower) fn generic_parameters(
        &mut self,
        tree: &mut mir::Tree,
        scope: &GenericScope,
        receiver: BoundReceiver,
    ) -> CompilerResult<Vec<mir::GenericParameter>> {
        // open one lowered parameter per collected index
        let (callable, this, lowered_this) = match receiver {
            BoundReceiver::None => (None, None, None),
            BoundReceiver::OfCallable(callable) => {
                (Some(callable), self.bound_this(callable)?, None)
            }
            BoundReceiver::Lowered(this) => (None, None, Some(this)),
        };
        let mut generics: Vec<Option<mir::GenericParameter>> = vec![None; scope.count() as usize];

        // lower each collected parameter at its index
        for (&parameter, &index) in &scope.parameters {
            // name the parameter as declared, an anonymous one by its kind and scope index
            let binding = self
                .state(parameter.module_id)?
                .generics
                .get_parameter(parameter.local_id)
                .clone();
            let name = match binding.key {
                dir::GenericParameterKey::Symbol(symbol) => self.symbol_name(symbol)?,
                dir::GenericParameterKey::Anonymous => {
                    let name = binding.canonical_name(index as usize, std::iter::empty());

                    Some(self.strings.intern(&name))
                }
            };
            let Some(name) = name else {
                return Err(CompilerError::Internal {
                    message: "a generic parameter without a name".to_string(),
                });
            };

            // read the domain the parameter ranges over
            let domain = match binding.memory_parameter() {
                Some(dir::MemoryParameter::Access) => mir::GenericParameterDomain::Access,
                Some(dir::MemoryParameter::Region) => mir::GenericParameterDomain::Region {
                    outlives: self.region_outlives(parameter, scope)?,
                },
                None if binding.is_const => {
                    let Some(constraint) = binding.constraint else {
                        return Err(CompilerError::Internal {
                            message: "a const parameter without its value type".to_string(),
                        });
                    };
                    let ty = self.type_lowerer(tree, scope).lower(constraint)?;

                    mir::GenericParameterDomain::Value { ty }
                }
                None => {
                    // bound the parameter by the bounds sema filled, then the where clauses
                    let mut bounds = {
                        let state = self.state(parameter.module_id)?;
                        match state.generics.parameter_bounds(parameter.local_id) {
                            Some(list) => state.types.type_ids(list).to_vec(),
                            None => Vec::new(),
                        }
                    };
                    if let Some(callable) = callable {
                        for bound in self.where_bounds(callable, parameter)? {
                            if !bounds.contains(&bound) {
                                bounds.push(bound);
                            }
                        }
                    }
                    let mut mentions_this = false;
                    for bound in &bounds {
                        let flags = self.types(bound.module_id)?.get_type_flags(bound.local_id);
                        mentions_this |= flags.has_this();
                    }
                    let mut lowered_bounds = Vec::new();
                    for bound in bounds {
                        let mut lowerer = self.type_lowerer(tree, scope);
                        lowerer.this_type = match lowered_this {
                            Some(this) => Some(this),
                            None => this
                                .filter(|_| mentions_this)
                                .map(|this| lowerer.lower(this))
                                .transpose()?,
                        };
                        lowered_bounds.extend(lowerer.lower_bounds(bound)?);
                    }

                    mir::GenericParameterDomain::Type {
                        bounds: lowered_bounds,
                    }
                }
            };

            generics[index as usize] = Some(mir::GenericParameter { name, domain });
        }

        // name each dependent by its position, an unbounded value of its kind the instance closes
        for (position, dependent) in scope.dependents.values().enumerate() {
            let name = self.strings.intern(&format!("P{position}"));
            let domain = match dependent.kind {
                Some(dir::MemoryParameter::Access) => mir::GenericParameterDomain::Access,
                Some(dir::MemoryParameter::Region) => mir::GenericParameterDomain::Region {
                    outlives: Vec::new(),
                },
                None => mir::GenericParameterDomain::Type { bounds: Vec::new() },
            };
            generics[dependent.index as usize] = Some(mir::GenericParameter { name, domain });
        }

        generics
            .into_iter()
            .map(|parameter| {
                parameter.ok_or_else(|| CompilerError::Internal {
                    message: "a template parameter index without its parameter".to_string(),
                })
            })
            .collect()
    }

    /// Return the type `this` names inside the bounds of one callable's template.
    pub(in crate::lower) fn bound_this(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // leave the receiver slot to instance members
        let owner = match self.imported_member(symbol)? {
            Some(member) if member.is_static => return Ok(None),
            Some(member) => member.owner,
            None => symbol,
        };

        // an extension names its target, a definition names itself
        Ok(match self.definition(owner)? {
            Some(dir::Definition::Extension(extension)) => Some(extension.target.r#type()),
            Some(
                dir::Definition::Struct(_)
                | dir::Definition::Class(_)
                | dir::Definition::Enum(_)
                | dir::Definition::Newtype(_),
            ) => Some(self.symbol_type(owner)?),
            _ => None,
        })
    }

    /// Return the generic argument naming one index in its own domain.
    pub(in crate::lower) fn index_argument(
        &self,
        tree: &mir::Tree,
        index: u32,
        kind: Option<dir::MemoryParameter>,
        is_const: bool,
    ) -> CompilerResult<mir::GenericArgument> {
        Ok(match kind {
            Some(dir::MemoryParameter::Region) => {
                mir::GenericArgument::Region(mir::Lifetime::new([mir::Extent::Parameter(index)]))
            }
            Some(dir::MemoryParameter::Access) => {
                mir::GenericArgument::Access(mir::Access::Parameter(index))
            }
            None if is_const => {
                mir::GenericArgument::Value(tree.intern_static(mir::Static::Parameter(index)))
            }
            None => mir::GenericArgument::Type(tree.intern_type(mir::Type::Parameter {
                index,
                referent: false,
            })),
        })
    }

    /// Lower one lifetime into the current lifetime environment, grounded where the scope is.
    pub(in crate::lower) fn lower_lifetime(
        &mut self,
        tree: &mir::Tree,
        lifetime: dir::GlobalTypeId,
        parameters: &GenericScope,
    ) -> CompilerResult<mir::Lifetime> {
        let lifetime = self.lower_lifetime_terms(lifetime, parameters)?;
        if parameters.grounding.is_empty() {
            return Ok(lifetime);
        }

        Ok(mir::Substitution::new(tree, &parameters.grounding).lifetime(&lifetime))
    }

    /// Lower one lifetime's terms into the current lifetime environment.
    fn lower_lifetime_terms(
        &mut self,
        lifetime: dir::GlobalTypeId,
        parameters: &GenericScope,
    ) -> CompilerResult<mir::Lifetime> {
        match self.ty(lifetime)? {
            // read the extent of a region pair
            dir::Type::Region(region) => self.lower_lifetime_terms(region.extent, parameters),
            // name a type declaration's region generic, or a function's binder slot
            dir::Type::Parameter(parameter) => {
                if parameters.erases_regions {
                    return Ok(mir::Lifetime::empty());
                }
                if let Some(index) = parameters.parameter_index(parameter) {
                    return Ok(mir::Lifetime::new([mir::Extent::Parameter(index)]));
                }
                match parameters.slots.get(&parameter) {
                    Some(slot) => Ok(mir::Lifetime::new([mir::Extent::Bound(*slot)])),
                    None => {
                        let template = self
                            .state(parameter.module_id)?
                            .generics
                            .get_parameter(parameter.local_id)
                            .template;
                        let owner = self
                            .state(parameter.module_id)?
                            .generics
                            .get_template(template)
                            .symbol;
                        let owner = match owner {
                            Some(symbol) => self.symbol_path(symbol)?,
                            None => "an anonymous template".to_string(),
                        };

                        Err(CompilerError::Internal {
                            message: format!(
                                "a region parameter '{}' of '{owner}' outside its scope",
                                self.format_parameter_name(parameter)?
                            ),
                        })
                    }
                }
            }
            // retain every extent of a lifetime union
            dir::Type::Union(union) => {
                let elements = self
                    .types(lifetime.module_id)?
                    .type_ids(union.elements)
                    .to_vec();

                // lower each element of the union into its extents
                let mut extents = Vec::new();
                for element in elements {
                    let lifetime = self.lower_lifetime_terms(element, parameters)?;
                    extents.extend(lifetime.extents);
                }

                Ok(mir::Lifetime::new(extents))
            }
            // lower concrete lifetime values
            _ => match self
                .memory_text(lifetime)?
                .and_then(|value| dir::Lifetime::parse(self.strings.get(value)))
            {
                Some(dir::Lifetime::Static) => Ok(mir::Lifetime::static_storage()),
                Some(dir::Lifetime::Frame) => Ok(mir::Lifetime::frame()),
                Some(dir::Lifetime::Managed) => Ok(mir::Lifetime::managed()),
                // an instance's positional region names the binder its specialization declares
                Some(dir::Lifetime::Bound(index)) => Ok(mir::Lifetime::bound(index)),
                None => Err(CompilerError::Internal {
                    message: format!(
                        "an unknown lifetime value in a '{}' type",
                        self.ty(lifetime)?.variant_name()
                    ),
                }),
            },
        }
    }
}
