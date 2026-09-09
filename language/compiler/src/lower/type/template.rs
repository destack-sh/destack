use destack_core::FxIndexMap;
use destack_dir as dir;
use destack_mir as mir;

use crate::lower::ModuleLowerer;
use crate::{CompilerError, CompilerResult, LowerError};

/// What `this` names inside the bounds of one template's parameters.
#[derive(Clone, Copy)]
pub(in crate::lower) enum BoundReceiver {
    /// No receiver.
    None,
    /// The receiver one callable declares.
    OfCallable(dir::GlobalSymbolId),
    /// One lowered receiver type.
    Lowered(mir::LocalNodeId<mir::Type>),
}

/// The lifetime slots and parameter indices one definition lowers under.
#[derive(Clone, Default)]
pub(in crate::lower) struct GenericScope {
    /// The index of each instance parameter, owner parameters first, the receiver among them.
    pub(in crate::lower) parameters: FxIndexMap<dir::GlobalGenericParameterId, u32>,
    /// The index of the interface receiver parameter, on interface member templates.
    pub(in crate::lower) receiver: Option<u32>,
    /// The index of each dependent the templates declare, after their parameters.
    pub(in crate::lower) dependents: FxIndexMap<dir::GlobalTypeId, u32>,
    /// The function-local slot of each lifetime parameter.
    pub(in crate::lower) slots: FxIndexMap<dir::GlobalGenericParameterId, mir::LifetimeSlot>,
    /// The declared name of each slot, without the tick.
    pub(in crate::lower) names: Vec<String>,
    /// The declared outlives pairs between slots, left outliving right.
    pub(in crate::lower) outlives: Vec<(mir::LifetimeSlot, mir::LifetimeSlot)>,
    /// The slot a constructor borrows its constructed storage at, after the declared ones.
    pub(in crate::lower) receiver_slot: Option<mir::LifetimeSlot>,
    /// Whether a region parameter outside the scope erases, at the seams keyed on erased types.
    erases_regions: bool,
}

impl GenericScope {
    /// Append the slot a constructor borrows its constructed storage at, named by the first tick
    /// letter free among the declared slots.
    pub(in crate::lower) fn push_receiver_slot(&mut self) -> mir::LifetimeSlot {
        let name = dir::free_region_name(self.names.iter().map(String::as_str));
        let slot = mir::LifetimeSlot(self.names.len() as u32);
        self.names.push(name);
        self.receiver_slot = Some(slot);

        slot
    }

    /// Return this scope with every region erased, the shape witness keys, instance keys, and
    /// dispatch implementers lower under.
    pub(in crate::lower) fn erased(&self) -> Self {
        Self {
            erases_regions: true,
            ..self.clone()
        }
    }

    /// Collect the parameters one signature closes over: its own template's and the dependents
    /// sema records for the declaration by symbol.
    pub(in crate::lower) fn for_signature(
        lower: &mut ModuleLowerer<'_>,
        template: Option<dir::GlobalGenericTemplateId>,
        declaration: Option<dir::GlobalSymbolId>,
    ) -> CompilerResult<Self> {
        let mut parameters = Self::from_templates(lower, None, template)?;
        if let Some(declaration) = declaration {
            parameters.collect_dependents(lower, declaration)?;
        }

        Ok(parameters)
    }

    /// Collect the parameters one type declaration's representation is generic over, its
    /// regions among them.
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
        for template in owner.into_iter().chain(signature) {
            parameters.collect_instance_parameters(lower, template, true, false)?;
        }
        for template in owner.into_iter().chain(signature) {
            parameters.collect_lifetimes(lower, template)?;
        }

        Ok(parameters)
    }

    /// Index the instance parameters one template declares, a type declaration's regions among
    /// them and a function's left to its binder.
    fn collect_instance_parameters(
        &mut self,
        lower: &mut ModuleLowerer<'_>,
        template: dir::GlobalGenericTemplateId,
        receiver: bool,
        regions: bool,
    ) -> CompilerResult<()> {
        // index each instance parameter in declaration order
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
            if !self.dependents.contains_key(&dependent) {
                let index = self.count();
                self.dependents.insert(dependent, index);
            }
        }

        Ok(())
    }

    /// Return the number of parameters collected, the interface receiver included.
    pub(in crate::lower) fn count(&self) -> u32 {
        (self.parameters.len() + self.dependents.len()) as u32
    }

    /// Return these parameters indexed after the parameters of an enclosing definition, its
    /// region slots ahead of the own ones.
    pub(in crate::lower) fn with_parameters_of(mut self, enclosing: &Self) -> Self {
        // remember the own parameters and the index each of them had
        let own: Vec<_> = self.parameters.keys().copied().collect();
        let own_dependents: Vec<_> = self.dependents.keys().copied().collect();
        let reindexed: Vec<_> = own
            .iter()
            .map(|parameter| self.parameters[parameter])
            .collect();
        let own_receiver = self.receiver;

        // start from the enclosing definition's parameters
        self.parameters = enclosing.parameters.clone();
        self.dependents = enclosing.dependents.clone();
        self.receiver = enclosing.receiver;

        // append the own region slots after the enclosing ones
        let offset = enclosing.slots.len() as u32;
        let own_slots: Vec<_> = self.slots.drain(..).collect();
        let own_names: Vec<_> = self.names.drain(..).collect();
        let own_outlives: Vec<_> = self.outlives.drain(..).collect();
        self.slots = enclosing.slots.clone();
        self.names = enclosing.names.clone();
        self.outlives = enclosing.outlives.clone();
        for (parameter, slot) in own_slots {
            self.slots
                .insert(parameter, mir::LifetimeSlot(slot.0 + offset));
        }
        self.names.extend(own_names);
        for (left, right) in own_outlives {
            self.outlives.push((
                mir::LifetimeSlot(left.0 + offset),
                mir::LifetimeSlot(right.0 + offset),
            ));
        }

        // append each own parameter to the enclosing definition
        for (parameter, previous) in own.into_iter().zip(reindexed) {
            let index = match self.parameters.get(&parameter) {
                Some(index) => *index,
                None => {
                    let index = self.count();
                    self.parameters.insert(parameter, index);
                    index
                }
            };
            if own_receiver == Some(previous) {
                self.receiver = Some(index);
            }
        }

        // append each own dependent new to the enclosing definition
        for dependent in own_dependents {
            if !self.dependents.contains_key(&dependent) {
                let index = self.count();
                self.dependents.insert(dependent, index);
            }
        }

        self
    }

    /// Return the index of one dependent the templates declare.
    pub(in crate::lower) fn dependent_index(&self, dependent: dir::GlobalTypeId) -> Option<u32> {
        self.dependents.get(&dependent).copied()
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
            let slot = mir::LifetimeSlot(self.slots.len() as u32);
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
    ) -> CompilerResult<Option<mir::LifetimeSlot>> {
        let dir::Type::Parameter(parameter) = lower.ty(ty)? else {
            return Ok(None);
        };

        Ok(self.slots.get(&parameter).copied())
    }

    /// Return the outlives slots declared for one slot.
    fn slot_outlives(&self, slot: mir::LifetimeSlot) -> Vec<mir::LifetimeSlot> {
        self.outlives
            .iter()
            .filter(|(left, _)| *left == slot)
            .map(|(_, right)| *right)
            .collect()
    }

    /// Declare these lifetime slots on one function header.
    pub(in crate::lower) fn declare<'a>(
        &self,
        mut header: mir::FunctionHeaderBuilder<'a>,
    ) -> mir::FunctionHeaderBuilder<'a> {
        for (slot, name) in self.names.iter().enumerate() {
            let outlives = self.slot_outlives(mir::LifetimeSlot(slot as u32));
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
                let outlives = self.slot_outlives(mir::LifetimeSlot(slot as u32));

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

    /// Lower the generic parameters one template declares with their names, domains, and bounds,
    /// `this` in the bounds read as the receiver names.
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
        for (parameter, index) in scope.parameters.clone() {
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
                Some(dir::MemoryParameter::Space | dir::MemoryParameter::Place) => {
                    mir::GenericParameterDomain::Space
                }
                Some(dir::MemoryParameter::Access) => mir::GenericParameterDomain::Access,
                Some(dir::MemoryParameter::Region) => mir::GenericParameterDomain::Region {
                    outlives: self.region_outlives(parameter, scope)?,
                },
                Some(dir::MemoryParameter::Ownership) => {
                    return Err(LowerError::Unsupported {
                        anchor: self.module.into(),
                        construct: "an ownership instance parameter".to_string(),
                    }
                    .into());
                }
                None if binding.is_const => {
                    let Some(constraint) = binding.constraint else {
                        return Err(CompilerError::Internal {
                            message: "a const parameter without its value type".to_string(),
                        });
                    };
                    let ty = self.type_lowerer(tree, scope).lower(constraint)?;

                    mir::GenericParameterDomain::Value {
                        ty: mir::TypeId::from(ty),
                    }
                }
                None => {
                    // bound the parameter by its written constraint, its recorded bounds, and the
                    // callable's where clauses
                    let mut bounds = {
                        let state = self.state(parameter.module_id)?;
                        let mut bounds = Vec::from_iter(binding.constraint);
                        if let Some(list) = state.generics.parameter_bounds(parameter.local_id) {
                            for bound in state.types.type_ids(list) {
                                if !bounds.contains(bound) {
                                    bounds.push(*bound);
                                }
                            }
                        }

                        bounds
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

        // name each dependent by its position, an unbounded type the instance closes
        for (position, index) in scope.dependents.values().enumerate() {
            let name = self.strings.intern(&format!("P{position}"));
            generics[*index as usize] = Some(mir::GenericParameter {
                name,
                domain: mir::GenericParameterDomain::Type { bounds: Vec::new() },
            });
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
    fn bound_this(
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

    /// Lower one lifetime into the current lifetime environment.
    pub(in crate::lower) fn lower_lifetime(
        &mut self,
        lifetime: dir::GlobalTypeId,
        parameters: &GenericScope,
    ) -> CompilerResult<mir::Lifetime> {
        match self.ty(lifetime)? {
            // read the extent of a region pair
            dir::Type::Region(region) => self.lower_lifetime(region.extent, parameters),
            // name a type declaration's region generic, or a function's binder slot
            dir::Type::Parameter(parameter) => {
                if parameters.erases_regions {
                    return Ok(mir::Lifetime::empty());
                }
                if let Some(index) = parameters.parameter_index(parameter) {
                    return Ok(mir::Lifetime::new([mir::LifetimeTerm::Parameter(index)]));
                }
                match parameters.slots.get(&parameter) {
                    Some(slot) => Ok(mir::Lifetime::slot(slot.0)),
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
            // retain every region term of a lifetime union
            dir::Type::Union(union) => {
                let elements = self
                    .types(lifetime.module_id)?
                    .type_ids(union.elements)
                    .to_vec();

                // lower each element of the union into its terms
                let mut terms = Vec::new();
                for element in elements {
                    let lifetime = self.lower_lifetime(element, parameters)?;
                    terms.extend(lifetime.terms);
                }

                Ok(mir::Lifetime::new(terms))
            }
            // lower concrete lifetime values
            _ => match self
                .memory_text(lifetime)?
                .and_then(|value| dir::Lifetime::parse(self.strings.get(value)))
            {
                Some(dir::Lifetime::Static) => Ok(mir::Lifetime::static_storage()),
                Some(dir::Lifetime::Frame) => Ok(mir::Lifetime::frame()),
                Some(dir::Lifetime::Managed) => Ok(mir::Lifetime::managed()),
                // an instance's positional region erases with the instance it names
                Some(dir::Lifetime::Bound(_)) => Ok(mir::Lifetime::empty()),
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
