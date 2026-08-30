use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{CheckState, Origin, TypeSubstitution, VariableKind};
use crate::{CompilerError, CompilerResult};

/// Stable id for one declaration-side generic parameter.
pub(in crate::sema) type GenericParameterId = dir::GlobalGenericParameterId;

/// Stable id for one generic binding site.
pub(in crate::sema) type GenericTemplateId = dir::GlobalGenericTemplateId;

impl CheckState<'_> {
    /// Return one symbol's generic template.
    pub(in crate::sema) fn symbol_template(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<GenericTemplateId>> {
        // load foreign templates before lookup
        if !self.is_own_module(symbol.module_id) {
            self.import_external_module(symbol.module_id)?;
        }

        Ok(self.loaded_symbol_template(symbol))
    }

    /// Return one symbol's already loaded template, skipping the import.
    pub(in crate::sema) fn loaded_symbol_template(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<GenericTemplateId> {
        self.template_by_symbol(symbol)
    }

    /// Return the template declared by one symbol, through its module's generic view.
    pub(in crate::sema) fn template_by_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<GenericTemplateId> {
        let module = symbol.module_id;

        // read the checked module's written stages
        if let Some(state) = self.module_maybe(module) {
            return state
                .written_template_by_symbol(symbol)
                .map(|local| local.into_global(module));
        }

        // read external committed templates
        let external = self.external_modules.get(&module)?;

        external
            .generics
            .template_by_symbol(symbol)
            .map(|local| local.into_global(module))
    }

    /// Return the template declared at one source node, through its module's generic view.
    pub(in crate::sema) fn template_by_source(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<GenericTemplateId> {
        let module = source.module_id;
        let state = self.module_maybe(module)?;

        state
            .written_template_by_source(source)
            .map(|local| local.into_global(module))
    }

    /// Return the parameter declared by one symbol, through its module's generic view.
    pub(in crate::sema) fn parameter_by_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<GenericParameterId> {
        let module = symbol.module_id;

        // read the checked module's written stages
        if let Some(state) = self.module_maybe(module) {
            return state
                .written_parameter_by_symbol(symbol)
                .map(|local| local.into_global(module));
        }

        // read external committed parameters
        let external = self.external_modules.get(&module)?;

        external
            .generics
            .parameter_by_symbol(symbol)
            .map(|local| local.into_global(module))
    }

    /// Return one generic template, reading the checked view over external tables.
    pub(in crate::sema) fn generic_template(
        &self,
        id: GenericTemplateId,
    ) -> Option<&dir::GenericTemplate> {
        // read the checked module's tail over its committed base
        if let Some(module) = self.module_maybe(id.module_id) {
            return module.generic_template(id.local_id);
        }

        // read external committed templates
        if let Some(external) = self.external_modules.get(&id.module_id) {
            return Some(external.generics.get_template(id.local_id));
        }

        None
    }

    /// Return one generic parameter, reading working segments over external tables.
    pub(in crate::sema) fn generic_parameter(
        &self,
        id: GenericParameterId,
    ) -> Option<&dir::GenericParameterBinding> {
        // read the checked module's tail over its committed base
        if let Some(module) = self.module_maybe(id.module_id) {
            return module.generic_parameter(id.local_id);
        }

        // read external committed parameters
        if let Some(external) = self.external_modules.get(&id.module_id) {
            return Some(external.generics.get_parameter(id.local_id));
        }

        None
    }

    /// Return whether one generic parameter is a memory parameter.
    pub(in crate::sema) fn is_memory_parameter(&self, id: GenericParameterId) -> bool {
        self.generic_parameter(id)
            .is_some_and(|parameter| parameter.memory_parameter().is_some())
    }

    /// Return whether one generic parameter is a lifetime.
    pub(in crate::sema) fn is_lifetime_parameter(&self, id: GenericParameterId) -> bool {
        self.generic_parameter(id).is_some_and(|parameter| {
            parameter.memory_parameter() == Some(dir::MemoryParameter::Region)
        })
    }

    /// Return the canonical type denoting one generic parameter.
    pub(in crate::sema) fn generic_parameter_type(
        &self,
        id: GenericParameterId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.generic_parameter(id)
            .map(|parameter| parameter.ty)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("generic parameter {id:?} is not bound"),
            })
    }

    /// Collect one template's parameter ids in declaration order.
    pub(in crate::sema) fn generic_template_parameters(
        &self,
        id: GenericTemplateId,
    ) -> CompilerResult<SmallVec<[GenericParameterId; 4]>> {
        let template = self.require_generic_template(id)?;

        // read each parameter under the template's module
        let parameters = template
            .parameters
            .iter()
            .map(|parameter| parameter.into_global(id.module_id))
            .collect();

        Ok(parameters)
    }

    /// Return whether one symbol's template and its owners declare region parameters only.
    pub(in crate::sema) fn symbol_template_is_region_only(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        let Some(template) = self.symbol_template(symbol)? else {
            return Ok(true);
        };

        // read the template's own and inherited parameters
        let mut parameters = self.generic_template_parameters(template)?;
        parameters.extend(self.owner_template_parameters(template)?);

        Ok(parameters.iter().all(|parameter| {
            self.generic_parameter(*parameter).is_some_and(|binding| {
                binding.memory_parameter() == Some(dir::MemoryParameter::Region)
            })
        }))
    }

    /// Return the owner parameters enclosing one member template.
    pub(in crate::sema) fn owner_template_parameters(
        &mut self,
        id: GenericTemplateId,
    ) -> CompilerResult<SmallVec<[GenericParameterId; 4]>> {
        let mut parameters = SmallVec::new();
        let mut current = self.parent_generic_template(id)?;

        // collect enclosing owner parameters outermost last
        while let Some(id) = current {
            let template = self.generic_template(id);
            let symbol = template.and_then(|template| template.symbol);
            let is_owner = match symbol {
                Some(symbol) => matches!(
                    self.definition(symbol)?,
                    Some(
                        dir::Definition::Extension(_)
                            | dir::Definition::Class(_)
                            | dir::Definition::Struct(_)
                            | dir::Definition::Enum(_)
                            | dir::Definition::Interface(_)
                            | dir::Definition::Newtype(_)
                    )
                ),
                None => false,
            };
            if is_owner {
                parameters.extend(self.generic_template_parameters(id)?);
            }
            current = self.parent_generic_template(id)?;
        }

        Ok(parameters)
    }

    /// Return the generic parameters owned by one callable signature.
    pub(in crate::sema) fn signature_generic_parameters(
        &self,
        signature: &dir::FunctionSignatureType,
    ) -> CompilerResult<SmallVec<[GenericParameterId; 4]>> {
        let Some(template_id) = signature.template else {
            return Ok(SmallVec::new());
        };
        let Some(template) = self.generic_template(template_id) else {
            return Err(CompilerError::Internal {
                message: format!("signature template {template_id:?} is missing"),
            });
        };

        // read each parameter under the template's module
        let parameters = template
            .parameters
            .iter()
            .map(|parameter| parameter.into_global(template_id.module_id))
            .collect();

        Ok(parameters)
    }

    /// Return applied generic argument bindings for one ordered parameter list.
    pub(in crate::sema) fn generic_argument_bindings(
        &mut self,
        parameters: &[GenericParameterId],
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Vec<dir::GenericArgumentBinding>> {
        // slot the written arguments over the ordered parameters
        let substitution = self.parameter_substitution(parameters, arguments)?;
        let mut bindings = Vec::with_capacity(substitution.bindings.len());

        // drop lifetime arguments from the erased instance identity
        for binding in substitution.bindings {
            if self.is_lifetime_parameter(binding.parameter) {
                continue;
            }

            let argument = self.generic_argument(binding.parameter, binding.argument)?;
            bindings.push(dir::GenericArgumentBinding::new(
                binding.parameter,
                argument,
            ));
        }

        Ok(bindings)
    }

    /// Return how many parameters accept written arguments.
    pub(in crate::sema) fn writable_parameter_count(
        &self,
        parameters: &[GenericParameterId],
    ) -> usize {
        parameters
            .iter()
            .filter(|parameter| {
                self.generic_parameter(**parameter)
                    .is_some_and(dir::GenericParameterBinding::is_writable)
            })
            .count()
    }

    /// Return one generic template that must already be loaded.
    pub(in crate::sema) fn require_generic_template(
        &self,
        id: GenericTemplateId,
    ) -> CompilerResult<&dir::GenericTemplate> {
        self.generic_template(id)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("generic template {id:?} is missing"),
            })
    }

    /// Return one generic parameter that must already be loaded.
    pub(in crate::sema) fn require_generic_parameter(
        &self,
        id: GenericParameterId,
    ) -> CompilerResult<&dir::GenericParameterBinding> {
        self.generic_parameter(id)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("generic parameter {id:?} is not bound"),
            })
    }

    /// Return applied generic argument bindings for one symbol template.
    pub(in crate::sema) fn symbol_generic_argument_bindings(
        &mut self,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Vec<dir::GenericArgumentBinding>> {
        // bind nothing for an unapplied bare declaration
        if arguments.is_empty() {
            return Ok(Vec::new());
        }
        let Some(template) = self.symbol_template(symbol)? else {
            return Err(CompilerError::Internal {
                message: format!("nongeneric symbol {symbol:?} has applied generic arguments"),
            });
        };
        let parameters = self.generic_template_parameters(template)?;

        self.generic_argument_bindings(&parameters, arguments)
    }

    /// Return one finalized generic argument.
    fn generic_argument(
        &mut self,
        parameter: GenericParameterId,
        argument: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // resolve the argument down to its root variable
        let argument = self.shallow_resolve(argument)?;
        let Some(variable) = self.root_variable(argument)? else {
            return Ok(argument);
        };

        // preserve selected declaration parameters that collected no bounds
        let state = self.infer.variable(variable)?;
        if !state.lower.is_empty() {
            return Ok(argument);
        }
        if !matches!(state.kind, VariableKind::Memory(_)) {
            let Some(binding) = self.generic_parameter(parameter) else {
                return Ok(argument);
            };
            if binding.induced_memory_parameter().is_none() {
                return Ok(argument);
            }
        }

        self.generic_parameter_type(parameter)
    }

    /// Return the uri naming one module in internal errors.
    fn module_uri(&self, module_id: ModuleId) -> String {
        self.compiler
            .module(self.context.revision(), module_id)
            .map(|module| module.uri.to_string())
            .unwrap_or_else(|_| format!("{module_id:?}"))
    }

    /// Open the generic template at one source node.
    pub(in crate::sema) fn open_generic_template(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<GenericTemplateId> {
        // reuse the template this source already opened
        if let Some(template) = self.template_by_source(source) {
            return Ok(template);
        }

        // bind the template to the scope its source introduces
        let module_id = source.module_id;
        let bindings = self.module(module_id).binding_table();
        let Some(scope) = bindings.introduced_scope(source) else {
            let uri = self.module_uri(module_id);
            return Err(CompilerError::Internal {
                message: format!("generic template source {source:?} introduces no scope in {uri}"),
            });
        };
        let symbol = bindings
            .get_scope_by_id(scope)
            .owner
            .map(|symbol| symbol.into_global(module_id));

        // keep only a symbol's own declaration template, leaving induced ones anonymous
        let symbol = symbol.filter(|symbol| {
            self.module(module_id)
                .symbol_declaration_node(symbol.local_id)
                .is_ok_and(|declaration| declaration == source.local_id)
        });

        // require one template per lexical scope
        let previous = self
            .module_maybe(module_id)
            .and_then(|module| module.generics_tail.template_by_scope(scope));
        if let Some(previous) = previous {
            let uri = self.module_uri(module_id);
            return Err(CompilerError::Internal {
                message: format!(
                    "generic template {source:?} and {previous:?} govern scope {scope:?} in {uri}"
                ),
            });
        }

        // allocate the template in its owning module, dropping the caches it invalidates
        self.assuming_scopes.clear();
        self.argument_ranks.clear();
        let module = self
            .module_maybe_mut(module_id)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check module {module_id:?} has no generic segment"),
            })?;
        let local = module
            .generics_tail
            .push_template(dir::GenericTemplate::new(source, scope, symbol));
        let id = local.into_global(module_id);

        Ok(id)
    }

    /// Push one generic parameter onto its template, dropping the caches it invalidates.
    pub(in crate::sema) fn push_generic_parameter(
        &mut self,
        template: GenericTemplateId,
        source: dir::GlobalNodeIdAny,
        symbol: Option<dir::GlobalSymbolId>,
        key: dir::GenericParameterKey,
        variance: Option<dir::VarianceModifier>,
        constraint: Option<dir::GlobalTypeId>,
        default: Option<dir::GlobalTypeId>,
        origin: dir::GenericParameterOrigin,
        kind: dir::GenericParameterKind,
        is_variadic: bool,
        is_const: bool,
    ) -> CompilerResult<GenericParameterId> {
        // precompute the parameter id before allocating its canonical type
        let module = template.module_id;
        if !self.is_own_module(module) {
            return Err(CompilerError::Internal {
                message: format!("check module {module:?} has no working generics"),
            });
        }
        self.assuming_scopes.clear();
        self.argument_ranks.clear();
        let local = dir::LocalGenericParameterId::new(self.module.generics_tail.parameter_count());
        let id = local.into_global(module);
        let ty = self
            .module
            .types_tail
            .intern_type(dir::Type::Parameter(id), dir::TypeFlags::EMPTY)
            .into_global(module);
        let binding = dir::GenericParameterBinding {
            template: template.local_id,
            source,
            symbol,
            ty,
            key,
            variance,
            constraint,
            default,
            origin,
            kind,
            is_variadic,
            is_const,
            conformances: dir::AutoInterfaceSet::new(),
        };

        // allocate the parameter in its template's working segment
        let local = self.module.generics_tail.push_template_parameter(binding);
        if local != id.local_id {
            return Err(CompilerError::Internal {
                message: format!(
                    "generic parameter allocation changed from {:?} to {:?}",
                    id.local_id, local
                ),
            });
        }

        Ok(id)
    }

    /// Return whether one written argument may fill one parameter.
    ///
    /// Region, place, and access parameters take only arguments of their own kind.
    /// An elided parameter slides the argument onward.
    /// A bare space term also fills a region parameter, lifting to the region holding that space.
    pub(in crate::sema) fn argument_fills_parameter(
        &self,
        binding: &dir::GenericParameterBinding,
        argument: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // admit the argument by the parameter's own kind
        match binding.memory_parameter() {
            Some(dir::MemoryParameter::Region) => Ok(matches!(
                self.memory_kind(argument)?,
                Some(dir::MemoryParameter::Region | dir::MemoryParameter::Place)
            )),
            Some(kind @ (dir::MemoryParameter::Place | dir::MemoryParameter::Access)) => {
                Ok(self.memory_kind(argument)? == Some(kind))
            }
            _ => Ok(true),
        }
    }

    /// Return the memory kind one type term inhabits.
    pub(in crate::sema) fn memory_kind(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::MemoryParameter>> {
        let id = self.shallow_resolve(id)?;
        // read the memory kind each term carries
        match self.ty(id)? {
            // regions are lifetime-kinded pairs
            dir::Type::Region(_) => Ok(Some(dir::MemoryParameter::Region)),

            // reserved lifetime, space, and access names write as string literals
            dir::Type::Literal(dir::Literal::String(value)) => {
                if dir::Lifetime::from_text(value).is_some() {
                    Ok(Some(dir::MemoryParameter::Region))
                } else if dir::Space::from_text(value).is_some() {
                    Ok(Some(dir::MemoryParameter::Place))
                } else if dir::Access::from_text(value).is_some() {
                    Ok(Some(dir::MemoryParameter::Access))
                } else {
                    Ok(None)
                }
            }

            // parameters inhabit their declared kind or the kind their constraint names
            dir::Type::Parameter(parameter) | dir::Type::Erased(parameter) => {
                let Some(binding) = self.generic_parameter(parameter) else {
                    return Ok(None);
                };
                match (binding.memory_parameter(), binding.constraint) {
                    (Some(kind), _) => Ok(Some(kind)),
                    (None, Some(constraint))
                        if let dir::Type::Application(_) = self.ty(constraint)? =>
                    {
                        self.memory_kind(constraint)
                    }
                    _ => Ok(None),
                }
            }

            // name each memory domain's own kind
            dir::Type::Application(instance) => Ok(self
                .language_item(instance.symbol)?
                .and_then(dir::MemoryParameter::from_language_item)
                .map(|kind| match kind {
                    dir::MemoryParameter::Space => dir::MemoryParameter::Place,
                    kind => kind,
                })),

            // joins inhabit the kind every element shares
            dir::Type::Union(union) => {
                let mut shared = None;
                for index in 0..union.elements.count {
                    let Some(element) = self.type_id_at(id.module_id, union.elements, index)?
                    else {
                        return Ok(None);
                    };
                    let Some(kind) = self.memory_kind(element)? else {
                        return Ok(None);
                    };
                    match shared {
                        Some(current) if current != kind => return Ok(None),
                        _ => shared = Some(kind),
                    }
                }

                Ok(shared)
            }

            _ => Ok(None),
        }
    }

    /// Return one memory-domain constraint type.
    pub(in crate::sema) fn memory_parameter_constraint(
        &mut self,
        kind: dir::MemoryParameter,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(symbol) = self.environment_bound.language.symbol(kind.language_item()) else {
            return Ok(None);
        };
        let arguments = self.intern_type_ids(&[])?;
        let constraint = self.intern_type(dir::Type::Application(dir::GenericApplication {
            symbol,
            arguments,
        }))?;

        Ok(Some(constraint))
    }

    /// Push one induced memory parameter.
    pub(in crate::sema) fn push_induced_memory_parameter(
        &mut self,
        template: GenericTemplateId,
        site: dir::GlobalNodeIdAny,
        kind: dir::MemoryParameter,
    ) -> CompilerResult<GenericParameterId> {
        let constraint = self.memory_parameter_constraint(kind)?;

        // reuse the parameter this site already induced
        if let Some(parameter) = self.reuse_induced_parameter(template, site, kind)? {
            return Ok(parameter);
        }

        // generate a kind-shaped name from the template position
        let number = self
            .generic_template(template)
            .map_or(0, |template| template.parameters.len());
        // name the induced parameter after its kind
        let generated = match kind {
            dir::MemoryParameter::Access => format!("A{number}"),
            dir::MemoryParameter::Ownership => format!("O{number}"),
            dir::MemoryParameter::Place => format!("P{number}"),
            dir::MemoryParameter::Space => format!("S{number}"),
            dir::MemoryParameter::Region => self.next_induced_lifetime_name(template),
        };
        let name = self.strings().intern(&generated);

        // push the induced parameter onto the template
        let parameter = self.push_generic_parameter(
            template,
            site,
            None,
            dir::GenericParameterKey::Generated(name),
            None,
            constraint,
            None,
            dir::GenericParameterOrigin::Induced,
            dir::GenericParameterKind::Memory(kind),
            false,
            false,
        )?;
        self.claimed_induced.insert(parameter);

        Ok(parameter)
    }

    /// Reuse the induced parameter one site opened on a template, once per hole.
    fn reuse_induced_parameter(
        &mut self,
        template: GenericTemplateId,
        site: dir::GlobalNodeIdAny,
        kind: dir::MemoryParameter,
    ) -> CompilerResult<Option<GenericParameterId>> {
        // claim the first parameter this site induced and left free
        let parameters = self.generic_template_parameters(template)?;
        for parameter in parameters {
            let Some(binding) = self.generic_parameter(parameter) else {
                continue;
            };
            if binding.origin != dir::GenericParameterOrigin::Induced
                || binding.source != site
                || binding.kind != dir::GenericParameterKind::Memory(kind)
                || self.claimed_induced.contains(&parameter)
            {
                continue;
            }
            self.claimed_induced.insert(parameter);

            return Ok(Some(parameter));
        }

        Ok(None)
    }

    /// Return the first free tick name for one induced lifetime parameter.
    fn next_induced_lifetime_name(&self, template: GenericTemplateId) -> String {
        // collect the tick names the template already declares
        let mut taken = Vec::new();
        if let Some(row) = self.generic_template(template) {
            for parameter in &row.parameters {
                let id = parameter.into_global(template.module_id);
                let Some(binding) = self.generic_parameter(id) else {
                    continue;
                };
                let name = match binding.key {
                    dir::GenericParameterKey::Symbol(symbol) => self.format_symbol(symbol),
                    dir::GenericParameterKey::Generated(name) => {
                        self.strings().get(name).to_string()
                    }
                };
                taken.push(name);
            }
        }

        // take the first free single letter tick
        for letter in 'a'..='z' {
            let candidate = format!("'{letter}");
            if !taken.contains(&candidate) {
                return candidate;
            }
        }

        format!("'l{}", taken.len())
    }
}

impl CheckState<'_> {
    /// Update one declared parameter's walked bounds.
    pub(in crate::sema) fn update_generic_parameter_bounds(
        &mut self,
        parameter: GenericParameterId,
        constraint: Option<dir::GlobalTypeId>,
        default: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<()> {
        // classify a memory parameter from the constraint it declares
        let current =
            self.generic_parameter(parameter)
                .cloned()
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("generic parameter {parameter:?} is not bound"),
                })?;
        let kind = if current.kind == dir::GenericParameterKind::Type {
            let item = match constraint
                .map(|constraint| self.ty(constraint))
                .transpose()?
            {
                Some(dir::Type::Application(instance)) => self.language_item(instance.symbol)?,
                _ => None,
            };

            item.and_then(dir::MemoryParameter::from_language_item)
        } else {
            None
        };

        // store memory defaults canonically, like written memory arguments
        let default = match (kind.is_some(), default) {
            (true, Some(default)) => {
                let origin = Origin::Node(current.source, None);

                Some(self.normalize_memory_component(origin, default)?)
            }
            _ => default,
        };

        // commit the completed binding after classification
        let module = parameter.module_id;
        let working = self
            .module_maybe_mut(module)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check module {module:?} has no working generics"),
            })?;

        // skip parameters the declared stage closed
        let Some(binding) = working
            .generics_tail
            .get_local_parameter_mut(parameter.local_id)
        else {
            return Ok(());
        };
        binding.constraint = constraint;
        binding.default = default;
        if let Some(kind) = kind {
            binding.kind = dir::GenericParameterKind::Memory(kind);
        }

        Ok(())
    }

    /// Push one walked where-clause predicate onto its declaring template.
    pub(in crate::sema) fn push_template_predicate(
        &mut self,
        template: GenericTemplateId,
        predicate: dir::WherePredicate,
    ) -> CompilerResult<()> {
        let module = template.module_id;
        let working = self
            .module_maybe_mut(module)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check module {module:?} has no working generics"),
            })?;

        // skip templates the declared stage closed
        let Some(declared) = working
            .generics_tail
            .get_local_template_mut(template.local_id)
        else {
            return Ok(());
        };

        // skip predicates the declaration pass already pushed
        if !declared.predicates.contains(&predicate) {
            declared.predicates.push(predicate);
        }

        Ok(())
    }

    /// Return one template's declared where predicates.
    pub(in crate::sema) fn template_predicates(
        &self,
        template: Option<GenericTemplateId>,
    ) -> SmallVec<[dir::WherePredicate; 2]> {
        template
            .and_then(|template| self.generic_template(template))
            .map(|template| SmallVec::from_slice(&template.predicates))
            .unwrap_or_default()
    }

    /// Return the assuming generic template of one work origin.
    pub(in crate::sema) fn origin_scope(
        &mut self,
        origin: Origin,
    ) -> CompilerResult<Option<GenericTemplateId>> {
        match origin {
            Origin::Node(_, scope) => Ok(scope),
            Origin::Symbol(symbol) => self.symbol_template(symbol),
        }
    }

    /// Return one work origin re-anchored at a node under the same assumptions.
    pub(in crate::sema) fn origin_at(
        &mut self,
        origin: Origin,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Origin> {
        Ok(Origin::Node(node, self.origin_scope(origin)?))
    }

    /// Collect one parameter's declared constraint and assumed bounds.
    pub(in crate::sema) fn parameter_bounds(
        &mut self,
        origin: Origin,
        parameter: GenericParameterId,
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 2]>> {
        let mut bounds = SmallVec::new();
        let constraint = self
            .generic_parameter(parameter)
            .and_then(|binding| binding.constraint);
        bounds.extend(constraint);
        bounds.extend(self.assumed_parameter_bounds(origin, parameter)?);

        Ok(bounds)
    }

    /// Collect the where-clause bounds one origin assumes for a parameter.
    pub(in crate::sema) fn assumed_parameter_bounds(
        &mut self,
        origin: Origin,
        parameter: GenericParameterId,
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 2]>> {
        self.assumed_bounds(
            origin,
            |ty| matches!(ty, dir::Type::Parameter(subject) if *subject == parameter),
        )
    }

    /// Collect the assumed bounds whose predicate subject matches.
    pub(in crate::sema) fn assumed_bounds(
        &mut self,
        origin: Origin,
        subject: impl Fn(&dir::Type) -> bool,
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 2]>> {
        let predicates = self.assumed_predicates(origin)?;

        self.subject_bounds(&predicates, subject)
    }

    /// Collect the where predicates assumed at one origin.
    pub(in crate::sema) fn assumed_predicates(
        &mut self,
        origin: Origin,
    ) -> CompilerResult<SmallVec<[dir::WherePredicate; 2]>> {
        let mut template = self.origin_scope(origin)?;
        let mut predicates = SmallVec::new();
        while let Some(id) = template {
            let declared = self.require_generic_template(id)?;
            predicates.extend_from_slice(&declared.predicates);
            template = self.parent_generic_template(id)?;
        }

        Ok(predicates)
    }

    /// Collect the bounds one predicate set grants a matching subject.
    fn subject_bounds(
        &self,
        predicates: &[dir::WherePredicate],
        subject: impl Fn(&dir::Type) -> bool,
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 2]>> {
        let mut bounds = SmallVec::new();
        for predicate in predicates {
            let left = self.ty(predicate.left)?;

            // take the right side as a bound where the subject stands left
            if subject(&left) {
                if !bounds.contains(&predicate.right) {
                    bounds.push(predicate.right);
                }
            }
            // equality binds its subject from either side
            else if predicate.relation == dir::WhereRelation::Equal {
                let right = self.ty(predicate.right)?;
                if subject(&right) && !bounds.contains(&predicate.left) {
                    bounds.push(predicate.left);
                }
            }
        }

        Ok(bounds)
    }

    /// Return the substitution one applied argument row selects, filling elided slots.
    pub(in crate::sema) fn applied_substitution(
        &mut self,
        template: GenericTemplateId,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<TypeSubstitution> {
        let parameters = self.generic_template_parameters(template)?;

        self.parameter_substitution(&parameters, arguments)
    }

    /// Slot one applied argument row over ordered parameters, filling elided slots.
    fn parameter_substitution(
        &mut self,
        parameters: &[GenericParameterId],
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<TypeSubstitution> {
        // reject rows longer than the parameter list
        if arguments.len() > parameters.len() {
            return Err(CompilerError::Internal {
                message: format!(
                    "an application received {} arguments for {} parameters",
                    arguments.len(),
                    parameters.len(),
                ),
            });
        }

        // bind a complete row positionally, blind to unresolved argument shapes
        if arguments.len() == parameters.len() {
            let bindings = parameters
                .iter()
                .cloned()
                .zip(arguments.iter().copied())
                .map(|(parameter, argument)| dir::GenericArgumentBinding::new(parameter, argument))
                .collect();

            return Ok(TypeSubstitution {
                bindings,
                receiver: None,
            });
        }

        // slot each parameter over the written row in order
        let mut substitution = TypeSubstitution::default();
        let mut cursor = 0usize;
        for parameter in parameters.iter().copied() {
            let binding = self.generic_parameter(parameter).cloned().ok_or_else(|| {
                CompilerError::Internal {
                    message: format!("generic parameter {parameter:?} is missing"),
                }
            })?;

            // memory parameters consume only written arguments of their own kind
            let kind_matches = cursor < arguments.len()
                && self.argument_fills_parameter(&binding, arguments[cursor])?;

            // bind the next written argument to the next writable slot
            let argument = if binding.is_writable() && kind_matches {
                let argument = arguments[cursor];
                cursor += 1;

                argument
            }
            // evaluate defaults against the application built so far
            else if let Some(default) = binding.default {
                self.substitute_type(default, &substitution)?
            }
            // fill elided lifetimes with the frame literal
            else if binding.memory_parameter() == Some(dir::MemoryParameter::Region) {
                self.lifetime_literal(dir::Lifetime::Frame)?
            }
            // fill elided places with the local literal
            else if binding.memory_parameter() == Some(dir::MemoryParameter::Place) {
                self.place_literal(dir::Space::Local)?
            }
            // reject truly unbound parameters
            else {
                return Err(CompilerError::Internal {
                    message: format!(
                        "an application received {} arguments for {} parameters",
                        arguments.len(),
                        parameters.len(),
                    ),
                });
            };
            substitution
                .bindings
                .push(dir::GenericArgumentBinding::new(parameter, argument));
        }

        Ok(substitution)
    }

    /// Return the type substitution for one generic instance.
    pub(in crate::sema) fn instance_substitution(
        &mut self,
        module: ModuleId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<TypeSubstitution> {
        let Some(template) = self.symbol_template(instance.symbol)? else {
            if instance.arguments.is_empty() {
                return Ok(TypeSubstitution::default());
            }

            // keep unloaded foreign applications symbolic
            if !self.is_loaded_module(instance.symbol.module_id) {
                return Ok(TypeSubstitution::default());
            }

            let name = self.format_symbol(instance.symbol);
            let rendered = self
                .type_ids(module, instance.arguments)?
                .iter()
                .map(|argument| self.format_type(*argument))
                .collect::<Vec<_>>()
                .join(", ");

            return Err(CompilerError::Internal {
                message: format!(
                    "nongeneric symbol {name} ({:?}) has {} applied type arguments: ({rendered})",
                    instance.symbol,
                    instance.arguments.len(),
                ),
            });
        };
        let arguments = SmallVec::<[dir::GlobalTypeId; 4]>::from_slice(
            self.type_ids(module, instance.arguments)?,
        );

        self.applied_substitution(template, &arguments)
    }

    /// Return one instance substitution qualified by a concrete receiver.
    pub(in crate::sema) fn qualified_instance_substitution(
        &mut self,
        module: ModuleId,
        instance: &dir::GenericApplication,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<TypeSubstitution> {
        let mut substitution = self.instance_substitution(module, instance)?;
        let receiver_substitution = TypeSubstitution::default().with_receiver(receiver);
        for binding in &mut substitution.bindings {
            binding.argument = self.substitute_type(binding.argument, &receiver_substitution)?;
        }
        substitution.receiver = Some(receiver);

        Ok(substitution)
    }

    /// Collect one parameter's inline and where-clause bounds.
    pub(in crate::sema) fn declared_parameter_bounds(
        &self,
        parameter: GenericParameterId,
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 2]>> {
        let binding = self.require_generic_parameter(parameter)?;
        let mut bounds = SmallVec::new();
        bounds.extend(binding.constraint);

        // collect predicates on the parameter's declaring template
        let template = binding.template.into_global(parameter.module_id);
        for bound in self.subject_bounds(
            &self.require_generic_template(template)?.predicates.clone(),
            |ty| matches!(ty, dir::Type::Parameter(subject) if *subject == parameter),
        )? {
            if !bounds.contains(&bound) {
                bounds.push(bound);
            }
        }

        Ok(bounds)
    }

    /// Collect the interface applications visible on `this` at one origin.
    pub(in crate::sema) fn this_bounds(
        &mut self,
        origin: Origin,
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 2]>> {
        let mut bounds = self.assumed_bounds(origin, |ty| matches!(ty, dir::Type::This))?;
        let mut template = self.origin_scope(origin)?;

        // derive enclosing interface applications from their templates
        while let Some(id) = template {
            let symbol = self.require_generic_template(id)?.symbol;
            if let Some(symbol) = symbol
                && self.symbol_kind(symbol)?.is_interface()
            {
                let application = self.declaration_instance(symbol)?;
                let bound = self.intern_type(dir::Type::Application(application))?;
                if !bounds.contains(&bound) {
                    bounds.push(bound);
                }
            }
            template = self.parent_generic_template(id)?;
        }

        Ok(bounds)
    }

    /// Return the nearest scope carrying assumptions at one origin.
    pub(in crate::sema) fn assuming_scope(
        &mut self,
        origin: Origin,
    ) -> CompilerResult<Option<GenericTemplateId>> {
        // the walk is a pure function of the declared scope
        let scope = self.origin_scope(origin)?;
        if let Some(assuming) = self.assuming_scopes.get(&scope) {
            return Ok(*assuming);
        }

        // climb to the nearest template declaring parameters or predicates
        let mut template = scope;
        while let Some(id) = template {
            let declared = self.require_generic_template(id)?;
            if !declared.parameters.is_empty() || !declared.predicates.is_empty() {
                break;
            }
            template = self.parent_generic_template(id)?;
        }
        self.assuming_scopes.insert(scope, template);

        Ok(template)
    }

    /// Return the nearest template enclosing one template's lexical scope.
    pub(in crate::sema) fn parent_generic_template(
        &self,
        template_id: GenericTemplateId,
    ) -> CompilerResult<Option<GenericTemplateId>> {
        let template = self.require_generic_template(template_id)?;
        let bindings = self.binding_table(template_id.module_id);

        // find the nearest ancestor governed by another template
        for current in bindings.scope_ancestors(template.scope) {
            if let Some(parent) = self.scope_template(template_id.module_id, current.id) {
                return Ok(Some(parent));
            }
        }

        Ok(None)
    }

    /// Return the template in effect at one node of a loaded module.
    pub(in crate::sema) fn template_at_node(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> Option<GenericTemplateId> {
        // read the scope the node sits in of a loaded module
        let module = node.module_id;
        if !self.is_loaded_module(module) {
            return None;
        }
        let bindings = self.binding_table(module);
        let scope = bindings.scope_for_node(node)?.id;

        // take the nearest template governing that scope or an ancestor
        std::iter::once(scope)
            .chain(bindings.scope_ancestors(scope).map(|scope| scope.id))
            .find_map(|scope| self.scope_template(module, scope))
    }

    /// Return the template governing one scope of a loaded module.
    fn scope_template(
        &self,
        module: ModuleId,
        scope: dir::LocalScopeId,
    ) -> Option<GenericTemplateId> {
        // read the template the scope writes
        let local = match self.module_maybe(module) {
            Some(state) => state.written_template_by_scope(scope),
            None => self
                .external_modules
                .get(&module)
                .and_then(|external| external.generics.template_by_scope(scope)),
        };

        local.map(|id| id.into_global(module))
    }

    /// Settle applied generic argument bindings for checked DIR.
    pub(in crate::sema) fn resolved_argument_bindings(
        &mut self,
        applied: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<Vec<dir::GenericArgumentBinding>> {
        // resolve each argument, skipping the lifetimes that erase from instance identity
        let mut bindings = Vec::with_capacity(applied.len());
        for applied in applied {
            let parameter = applied.parameter;
            let argument = self.shallow_resolve(applied.argument)?;
            if self.is_lifetime_parameter(parameter) {
                continue;
            }
            bindings.push(dir::GenericArgumentBinding::new(parameter, argument));
        }

        // sort in written order
        let mut keyed = Vec::with_capacity(bindings.len());
        for binding in bindings {
            let rank = self.written_argument_rank(binding.parameter)?;
            keyed.push((rank, binding));
        }
        keyed.sort_by_key(|(rank, binding)| (*rank, binding.parameter));

        Ok(keyed.into_iter().map(|(_, binding)| binding).collect())
    }

    /// Return one parameter's written argument rank.
    fn written_argument_rank(
        &mut self,
        parameter: GenericParameterId,
    ) -> CompilerResult<(usize, Option<GenericTemplateId>, usize)> {
        // the rank is a pure function of the declared templates
        if let Some(rank) = self.argument_ranks.get(&parameter) {
            return Ok(*rank);
        }
        let rank = self.derive_argument_rank(parameter)?;
        self.argument_ranks.insert(parameter, rank);

        Ok(rank)
    }

    /// Derive one parameter's written argument rank from its template chain.
    fn derive_argument_rank(
        &mut self,
        parameter: GenericParameterId,
    ) -> CompilerResult<(usize, Option<GenericTemplateId>, usize)> {
        // read the parameter's position in the template that declares it
        let Some(declared) = self.generic_parameter(parameter) else {
            return Ok((usize::MAX, None, usize::MAX));
        };
        let template = declared.template.into_global(parameter.module_id);
        let position = self
            .generic_template(template)
            .map(|template| {
                template
                    .parameters
                    .iter()
                    .position(|declared| *declared == parameter.local_id)
                    .unwrap_or(usize::MAX)
            })
            .unwrap_or(usize::MAX);

        // outer templates rank before the chains nested under them
        let mut depth = 0usize;
        let mut current = template;
        while let Some(parent) = self.parent_generic_template(current)? {
            depth += 1;
            current = parent;
        }

        Ok((depth, Some(template), position))
    }
}
