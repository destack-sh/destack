use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{CheckState, Origin, TypeSubstitution, VariableRole};
use crate::{CompilerError, CompilerResult};

/// Stable id for one declaration-side generic parameter.
pub(in crate::check) type GenericParameterId = dir::GlobalGenericParameterId;

/// Stable id for one generic binding site.
pub(in crate::check) type GenericTemplateId = dir::GlobalGenericTemplateId;

/// The generic template whose parameters one question assumes rigid.
pub(in crate::check) type Scope = Option<dir::GlobalGenericTemplateId>;

/// One declaration type scanned for induced memory variables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct InducedParameterSite {
    /// The declaration node that receives induced parameters.
    pub(in crate::check) declaration: dir::GlobalNodeIdAny,
    /// The declaration type to traverse.
    pub(in crate::check) ty: dir::GlobalTypeId,
}

impl CheckState<'_> {
    /// Collect the induced site types pushed by one declaration.
    pub(in crate::check) fn induced_site_types(
        &self,
        declaration: dir::GlobalNodeIdAny,
    ) -> Vec<dir::GlobalTypeId> {
        self.induced_parameter_sites
            .iter()
            .filter(|site| site.declaration == declaration)
            .map(|site| site.ty)
            .collect()
    }

    /// Return one symbol's generic template.
    pub(in crate::check) fn symbol_template(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<GenericTemplateId>> {
        // load foreign templates before lookup
        if !self.is_own_module(symbol.module_id) {
            self.import_external_module(symbol.module_id)?;
        }

        Ok(self.loaded_symbol_template(symbol))
    }

    /// Return one symbol's already loaded template, without importing.
    pub(in crate::check) fn loaded_symbol_template(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<GenericTemplateId> {
        self.template_by_symbol(symbol)
    }

    /// Return the template declared by one symbol, scanning its module's entries.
    pub(in crate::check) fn template_by_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<GenericTemplateId> {
        let module = symbol.module_id;

        // scan the working and declared entries of the checked module
        if self.is_own_module(module) {
            let working =
                self.module
                    .generics_tail
                    .iter_templates()
                    .find_map(|(local, template)| {
                        (template.symbol == Some(symbol)).then(|| local.into_global(module))
                    });
            if working.is_some() {
                return working;
            }

            return self.module.declared.as_ref().and_then(|declared| {
                declared
                    .generics
                    .iter_templates()
                    .find_map(|(local, template)| {
                        (template.symbol == Some(symbol)).then(|| local.into_global(module))
                    })
            });
        }

        // read external committed templates
        let external = self.external_modules.get(&module)?;

        external
            .generics
            .template_by_symbol(symbol)
            .map(|local| local.into_global(module))
    }

    /// Return the template declared at one source node, scanning its module's entries.
    pub(in crate::check) fn template_by_source(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<GenericTemplateId> {
        let module = source.module_id;
        let working = self.module_maybe(module)?;

        // scan the working entries, then the declared stage
        let allocated = working
            .generics_tail
            .iter_templates()
            .find_map(|(local, template)| {
                (template.source == source).then(|| local.into_global(module))
            });
        if allocated.is_some() {
            return allocated;
        }

        working.declared.as_ref().and_then(|declared| {
            declared
                .generics
                .iter_templates()
                .find_map(|(local, template)| {
                    (template.source == source).then(|| local.into_global(module))
                })
        })
    }

    /// Return the parameter declared by one symbol, scanning its module's entries.
    pub(in crate::check) fn parameter_by_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<GenericParameterId> {
        let module = symbol.module_id;

        // scan the working and declared entries of the checked module
        if self.is_own_module(module) {
            let working =
                self.module
                    .generics_tail
                    .iter_parameters()
                    .find_map(|(local, binding)| {
                        (binding.symbol == Some(symbol)).then(|| local.into_global(module))
                    });
            if working.is_some() {
                return working;
            }

            return self.module.declared.as_ref().and_then(|declared| {
                declared
                    .generics
                    .iter_parameters()
                    .find_map(|(local, binding)| {
                        (binding.symbol == Some(symbol)).then(|| local.into_global(module))
                    })
            });
        }

        // read external committed parameters
        let external = self.external_modules.get(&module)?;

        external
            .generics
            .parameter_by_symbol(symbol)
            .map(|local| local.into_global(module))
    }

    /// Return one generic template, reading working segments over external tables.
    pub(in crate::check) fn generic_template(
        &self,
        id: GenericTemplateId,
    ) -> Option<&dir::GenericTemplate> {
        // read working templates before declared-stage templates
        if let Some(module) = self.module_maybe(id.module_id) {
            if let Some(template) = module.generics_tail.get_local_template(id.local_id) {
                return Some(template);
            }

            if let Some(declared) = &module.declared
                && let Some(template) = declared.generics.get_local_template(id.local_id)
            {
                return Some(template);
            }
        }

        // read external committed templates
        if let Some(external) = self.external_modules.get(&id.module_id) {
            return Some(external.generics.get_template(id.local_id));
        }

        None
    }

    /// Return one generic parameter, reading working segments over external tables.
    pub(in crate::check) fn generic_parameter(
        &self,
        id: GenericParameterId,
    ) -> Option<&dir::GenericParameterBinding> {
        // read working parameters before declared-stage parameters
        if let Some(module) = self.module_maybe(id.module_id) {
            if let Some(parameter) = module.generics_tail.get_local_parameter(id.local_id) {
                return Some(parameter);
            }

            if let Some(elaborated) = &module.elaborated
                && let Some(parameter) = elaborated.generics.get_local_parameter(id.local_id)
            {
                return Some(parameter);
            }

            if let Some(declared) = &module.declared
                && let Some(parameter) = declared.generics.get_local_parameter(id.local_id)
            {
                return Some(parameter);
            }
        }

        // read external committed parameters
        if let Some(external) = self.external_modules.get(&id.module_id) {
            return Some(external.generics.get_parameter(id.local_id));
        }

        None
    }

    /// Return the canonical type denoting one generic parameter.
    pub(in crate::check) fn generic_parameter_type(
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
    pub(in crate::check) fn generic_template_parameters(
        &self,
        id: GenericTemplateId,
    ) -> CompilerResult<SmallVec<[GenericParameterId; 4]>> {
        let template = self.require_generic_template(id)?;

        let parameters = template
            .parameters
            .iter()
            .map(|parameter| parameter.into_global(id.module_id))
            .collect();

        Ok(parameters)
    }

    /// Return the owner parameters enclosing one member template.
    pub(in crate::check) fn owner_template_parameters(
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
    pub(in crate::check) fn signature_generic_parameters(
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

        let parameters = template
            .parameters
            .iter()
            .map(|parameter| parameter.into_global(template_id.module_id))
            .collect();

        Ok(parameters)
    }

    /// Return applied generic argument bindings for one ordered parameter list.
    pub(in crate::check) fn generic_argument_bindings(
        &mut self,
        parameters: &[GenericParameterId],
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Vec<dir::GenericArgumentBinding>> {
        // drop lifetime slots from erased instance identities
        let lifetimes = parameters
            .iter()
            .map(|parameter| {
                self.generic_parameter(*parameter).is_some_and(|binding| {
                    binding.memory_parameter() == Some(dir::MemoryParameter::Lifetime)
                })
            })
            .collect::<Vec<_>>();
        let values = lifetimes.iter().filter(|lifetime| !**lifetime).count();
        if arguments.len() != parameters.len() && arguments.len() != values {
            return Err(CompilerError::Internal {
                message: format!(
                    "generic argument count {} does not match parameter count {}",
                    arguments.len(),
                    parameters.len(),
                ),
            });
        }

        // slot written lifetime arguments and skip them in the instance identity
        let written = arguments.len() == parameters.len();
        let mut bindings = Vec::with_capacity(values);
        let mut cursor = 0usize;
        for (parameter, is_lifetime) in parameters.iter().copied().zip(lifetimes) {
            if is_lifetime {
                if written {
                    self.generic_argument(parameter, arguments[cursor])?;
                    cursor += 1;
                }
                continue;
            }

            let argument = self.generic_argument(parameter, arguments[cursor])?;
            cursor += 1;
            bindings.push(dir::GenericArgumentBinding::new(parameter, argument));
        }

        Ok(bindings)
    }

    /// Return applied generic argument bindings for one symbol template.
    pub(in crate::check) fn symbol_generic_argument_bindings(
        &mut self,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Vec<dir::GenericArgumentBinding>> {
        let Some(template) = self.symbol_template(symbol)? else {
            if arguments.is_empty() {
                return Ok(Vec::new());
            }

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
        let argument = self.shallow_resolve(argument)?;
        let Some(variable) = self.root_variable(argument)? else {
            return Ok(argument);
        };

        // preserve selected declaration parameters that collected no bounds
        let state = self.infer.variable(variable)?;
        if !state.lower.is_empty() {
            return Ok(argument);
        }
        if self.infer.variable_role(variable)?.is_inference() {
            let Some(binding) = self.generic_parameter(parameter) else {
                return Ok(argument);
            };
            if binding.induced_memory_parameter().is_none() {
                return Ok(argument);
            }
        }

        self.generic_parameter_type(parameter)
    }

    /// Open the generic template at one source node.
    pub(in crate::check) fn open_generic_template(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<GenericTemplateId> {
        if let Some(template) = self.template_by_source(source) {
            return Ok(template);
        }

        // bind the template to its declaration scope
        let module_id = source.module_id;
        let bindings = self.module(module_id).binding_table();
        let scope = bindings
            .scope_for_node(source)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("generic template source {source:?} has no lexical scope"),
            })?
            .id;
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
            return Err(CompilerError::Internal {
                message: format!(
                    "generic template {source:?} and {previous:?} govern scope {scope:?}"
                ),
            });
        }

        // allocate the template in its owning module
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

    /// Push one generic parameter onto its template.
    pub(in crate::check) fn push_generic_parameter(
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

    /// Return the induced memory variables inside one type, in graph order.
    pub(in crate::check) fn induced_memory_variables(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Vec<(dir::TypeVariableId, VariableRole)>> {
        let mut variables = Vec::new();
        for variable in self.type_variables(ty)? {
            let role = self.variable_role(variable)?;
            if matches!(role, VariableRole::Memory { .. }) {
                variables.push((variable, role));
            }
        }

        Ok(variables)
    }

    /// Return whether one type is a written lifetime term.
    pub(in crate::check) fn is_lifetime_term(&self, id: dir::GlobalTypeId) -> CompilerResult<bool> {
        match self.ty(id)? {
            // lifetime literals like "static" and "frame"
            dir::Type::Memory(dir::MemoryLiteral::Lifetime(_)) => Ok(true),

            // lifetime-kinded generic parameters
            dir::Type::Parameter(parameter) => {
                Ok(self.generic_parameter(parameter).is_some_and(|binding| {
                    binding.memory_parameter() == Some(dir::MemoryParameter::Lifetime)
                }))
            }

            // unions of lifetime terms
            ty @ dir::Type::Union(_) => {
                let mut children = Vec::new();
                self.for_each_type_child(id.module_id, &ty, |child| children.push(child))?;
                for child in &children {
                    if !self.is_lifetime_term(*child)? {
                        return Ok(false);
                    }
                }

                Ok(!children.is_empty())
            }

            _ => Ok(false),
        }
    }

    /// Push one induced memory parameter.
    pub(in crate::check) fn push_induced_memory_parameter(
        &mut self,
        template: GenericTemplateId,
        site: dir::GlobalNodeIdAny,
        role: VariableRole,
    ) -> CompilerResult<GenericParameterId> {
        let VariableRole::Memory { kind, constraint } = role else {
            return Err(CompilerError::Internal {
                message: "ordinary inference variable cannot become a memory parameter".into(),
            });
        };

        // claim the parameter this site already induced
        if let Some(parameter) = self.claim_induced_parameter(template, site, kind) {
            return Ok(parameter);
        }

        // generate a kind-shaped name from the template position
        let number = self
            .generic_template(template)
            .map_or(0, |template| template.parameters.len());
        let generated = match kind {
            dir::MemoryParameter::Access => format!("A{number}"),
            dir::MemoryParameter::Ownership => format!("O{number}"),
            dir::MemoryParameter::Place => format!("P{number}"),
            dir::MemoryParameter::Space => format!("S{number}"),
            dir::MemoryParameter::Lifetime => self.next_induced_lifetime_name(template),
        };
        let name = self.strings().intern(&generated);

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

    /// Claim the induced parameter one site owns on a template, once per hole.
    fn claim_induced_parameter(
        &mut self,
        template: GenericTemplateId,
        site: dir::GlobalNodeIdAny,
        kind: dir::MemoryParameter,
    ) -> Option<GenericParameterId> {
        let parameters = self.generic_template_parameters(template).ok()?;
        for parameter in parameters {
            let binding = self.generic_parameter(parameter)?;
            if binding.origin != dir::GenericParameterOrigin::Induced
                || binding.source != site
                || binding.kind != dir::GenericParameterKind::Memory(kind)
                || self.claimed_induced.contains(&parameter)
            {
                continue;
            }
            self.claimed_induced.insert(parameter);

            return Some(parameter);
        }

        None
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
    pub(in crate::check) fn update_generic_parameter_bounds(
        &mut self,
        parameter: GenericParameterId,
        constraint: Option<dir::GlobalTypeId>,
        default: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<()> {
        let current =
            self.generic_parameter(parameter)
                .copied()
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("generic parameter {parameter:?} is not bound"),
                })?;
        let kind = if current.kind == dir::GenericParameterKind::Value {
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
        let default = match (kind, default) {
            (Some(kind), Some(default)) => {
                let origin = Origin::Node(current.source, None);

                Some(self.normalize_memory_component(origin, default, kind)?)
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

        // skip parameters the declared stage settled
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

    /// Record one walked where-clause predicate on its declaring template.
    pub(in crate::check) fn push_template_predicate(
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

        // skip templates the declared stage settled
        let Some(declared) = working
            .generics_tail
            .get_local_template_mut(template.local_id)
        else {
            return Ok(());
        };

        // skip predicates the declaration pass already recorded
        if !declared.predicates.contains(&predicate) {
            declared.predicates.push(predicate);
        }

        Ok(())
    }

    /// Return one template's declared where predicates.
    pub(in crate::check) fn template_predicates(
        &self,
        template: Option<GenericTemplateId>,
    ) -> SmallVec<[dir::WherePredicate; 2]> {
        template
            .and_then(|template| self.generic_template(template))
            .map(|template| SmallVec::from_slice(&template.predicates))
            .unwrap_or_default()
    }

    /// Return the assuming generic template of one work origin.
    pub(in crate::check) fn origin_scope(
        &mut self,
        origin: Origin,
    ) -> CompilerResult<Option<GenericTemplateId>> {
        match origin {
            Origin::Node(_, scope) => Ok(scope),
            Origin::Symbol(symbol) => self.symbol_template(symbol),
        }
    }

    /// Return one work origin re-anchored at a node under the same assumptions.
    pub(in crate::check) fn origin_at(
        &mut self,
        origin: Origin,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Origin> {
        Ok(Origin::Node(node, self.origin_scope(origin)?))
    }

    /// Collect one parameter's declared constraint and assumed bounds.
    pub(in crate::check) fn parameter_bounds(
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
    pub(in crate::check) fn assumed_parameter_bounds(
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
    pub(in crate::check) fn assumed_bounds(
        &mut self,
        origin: Origin,
        subject: impl Fn(&dir::Type) -> bool,
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 2]>> {
        let predicates = self.assumed_predicates(origin)?;

        self.subject_bounds(&predicates, subject)
    }

    /// Collect the where predicates assumed at one origin.
    pub(in crate::check) fn assumed_predicates(
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
            if subject(&left) {
                if !bounds.contains(&predicate.right) {
                    bounds.push(predicate.right);
                }
            } else if predicate.relation == dir::WhereRelation::Equal {
                // equality binds its subject from either side
                let right = self.ty(predicate.right)?;
                if subject(&right) && !bounds.contains(&predicate.left) {
                    bounds.push(predicate.left);
                }
            }
        }

        Ok(bounds)
    }

    /// Return a positional substitution for one complete template application.
    pub(in crate::check) fn template_substitution(
        &self,
        template: GenericTemplateId,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<TypeSubstitution> {
        let parameters = self.generic_template_parameters(template)?;
        if arguments.len() != parameters.len() {
            return Err(CompilerError::Internal {
                message: format!(
                    "generic template {template:?} received {} arguments for {} parameters",
                    arguments.len(),
                    parameters.len(),
                ),
            });
        }

        let bindings = parameters
            .into_iter()
            .zip(arguments.iter().copied())
            .map(|(parameter, argument)| dir::GenericArgumentBinding::new(parameter, argument))
            .collect();

        Ok(TypeSubstitution {
            bindings,
            receiver: None,
        })
    }

    /// Return the type substitution for one generic instance.
    pub(in crate::check) fn instance_substitution(
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

        // fill elided defaults before instantiating the written application
        let parameters = self.generic_template_parameters(template)?;
        if arguments.len() < parameters.len() {
            let mut substitution = TypeSubstitution::default();
            let mut cursor = 0usize;
            for parameter in parameters.iter().copied() {
                let binding = self.generic_parameter(parameter).copied().ok_or_else(|| {
                    CompilerError::Internal {
                        message: format!("generic parameter {parameter:?} is missing"),
                    }
                })?;

                // lifetime slots consume only written lifetimes
                let wants_lifetime =
                    binding.memory_parameter() == Some(dir::MemoryParameter::Lifetime);
                let next_is_lifetime = cursor < arguments.len()
                    && self.written_argument_is_lifetime(arguments[cursor])?;

                // bind the next written argument to the next writable slot
                let argument = if binding.is_writable()
                    && cursor < arguments.len()
                    && (!wants_lifetime || next_is_lifetime)
                {
                    let argument = arguments[cursor];
                    cursor += 1;

                    argument
                }
                // evaluate defaults against the application built so far
                else if let Some(default) = binding.default {
                    self.substitute_type(default, &substitution)?
                }
                // fill elided lifetimes with the frame literal
                else if binding.memory_parameter() == Some(dir::MemoryParameter::Lifetime) {
                    self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Lifetime(
                        dir::Lifetime::Frame,
                    )))?
                }
                // reject truly unbound parameters
                else {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "generic template {template:?} received {} arguments for {} parameters",
                            arguments.len(),
                            parameters.len(),
                        ),
                    });
                };
                substitution
                    .bindings
                    .push(dir::GenericArgumentBinding::new(parameter, argument));
            }

            return Ok(substitution);
        }

        self.template_substitution(template, &arguments)
    }

    /// Return whether one written argument spells a lifetime.
    pub(in crate::check) fn written_argument_is_lifetime(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        if self.is_lifetime_slot(&self.ty(ty)?)? {
            return Ok(true);
        }

        // lifetime joins write as unions of lifetimes
        if let dir::Type::Union(union) = self.ty(ty)? {
            let elements = self.type_ids(ty.module_id, union.elements)?;
            for &element in elements {
                if !self.written_argument_is_lifetime(element)? {
                    return Ok(false);
                }
            }

            return Ok(true);
        }

        // memory literals appear as strings in written type arguments
        let is_reserved_lifetime = matches!(
            self.ty(ty)?,
            dir::Type::Literal(dir::ScalarLiteral::String(name))
                if matches!(self.strings().get(name), "static" | "frame")
        );

        Ok(is_reserved_lifetime)
    }

    /// Return one instance substitution qualified by a concrete receiver.
    pub(in crate::check) fn qualified_instance_substitution(
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
    pub(in crate::check) fn declared_parameter_bounds(
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
    pub(in crate::check) fn this_bounds(
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
    pub(in crate::check) fn assuming_scope(
        &mut self,
        origin: Origin,
    ) -> CompilerResult<Option<GenericTemplateId>> {
        let mut template = self.origin_scope(origin)?;
        while let Some(id) = template {
            let declared = self.require_generic_template(id)?;
            if !declared.parameters.is_empty() || !declared.predicates.is_empty() {
                break;
            }
            template = self.parent_generic_template(id)?;
        }

        Ok(template)
    }

    /// Return the nearest template enclosing one template's lexical scope.
    fn parent_generic_template(
        &self,
        template_id: GenericTemplateId,
    ) -> CompilerResult<Option<GenericTemplateId>> {
        let template = self.require_generic_template(template_id)?;
        let bindings = self.binding_table(template_id.module_id);

        // find the nearest ancestor governed by another template
        for current in bindings.scope_ancestors(template.scope) {
            let parent = if self.is_own_module(template_id.module_id) {
                self.module_maybe(template_id.module_id)
                    .and_then(|module| {
                        // read the working template before the declared-stage one
                        module
                            .generics_tail
                            .template_by_scope(current.id)
                            .or_else(|| {
                                module.declared.as_ref().and_then(|declared| {
                                    declared.generics.template_by_scope(current.id)
                                })
                            })
                    })
                    .map(|id| id.into_global(template_id.module_id))
            } else {
                self.external_module(template_id.module_id)
                    .generics
                    .template_by_scope(current.id)
                    .map(|id| id.into_global(template_id.module_id))
            };
            if parent.is_some() {
                return Ok(parent);
            }
        }

        Ok(None)
    }

    /// Return how many parameters accept written arguments.
    pub(in crate::check) fn writable_parameter_count(
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
    pub(in crate::check) fn require_generic_template(
        &self,
        id: GenericTemplateId,
    ) -> CompilerResult<&dir::GenericTemplate> {
        self.generic_template(id)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("generic template {id:?} is missing"),
            })
    }

    /// Return one generic parameter that must already be loaded.
    pub(in crate::check) fn require_generic_parameter(
        &self,
        id: GenericParameterId,
    ) -> CompilerResult<&dir::GenericParameterBinding> {
        self.generic_parameter(id)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("generic parameter {id:?} is not bound"),
            })
    }

    /// Settle applied generic argument bindings for checked DIR.
    pub(in crate::check) fn settled_argument_bindings(
        &mut self,
        applied: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<Vec<dir::GenericArgumentBinding>> {
        let mut bindings = Vec::with_capacity(applied.len());
        for applied in applied {
            // skip lifetime arguments, which erase from instance identity
            let parameter = applied.parameter;
            let argument = self.shallow_resolve(applied.argument)?;
            let binding = self.require_generic_parameter(parameter)?;
            let is_lifetime = binding.memory_parameter() == Some(dir::MemoryParameter::Lifetime);
            if is_lifetime {
                continue;
            }
            bindings.push(dir::GenericArgumentBinding::new(parameter, argument));
        }

        Ok(bindings)
    }
}
