use destack_core::{FxIndexMap, FxIndexSet};
use std::sync::Arc;

use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{CheckState, Origin, VariableRole};
use crate::{CompilerError, CompilerResult};

/// Stable id for one declaration-side generic parameter.
pub(in crate::check) type GenericParameterId = dir::GlobalGenericParameterId;

/// Stable id for one generic binding site.
pub(in crate::check) type GenericTemplateId = dir::GlobalGenericTemplateId;

/// Generic parameters and predicates visible from one template.
#[derive(Debug, Default)]
pub(in crate::check) struct GenericScope {
    /// Every parameter the scope declares or encloses.
    pub(in crate::check) parameters: SmallVec<[GenericParameterId; 8]>,
    /// Every where predicate the scope assumes, own and enclosing.
    pub(in crate::check) predicates: SmallVec<[dir::WherePredicate; 4]>,
}

/// One declaration type scanned for induced memory variables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct InducedParameterSite {
    /// The declaration node that receives induced parameters.
    pub(in crate::check) declaration: dir::GlobalNodeIdAny,
    /// The enclosing generic template.
    pub(in crate::check) parent: Option<GenericTemplateId>,
    /// The declaration symbol indexed by the generated template.
    pub(in crate::check) symbol: Option<dir::GlobalSymbolId>,
    /// The declaration type to traverse.
    pub(in crate::check) ty: dir::GlobalTypeId,
}

/// Generic declaration index over working generic segments.
#[derive(Debug)]
pub(in crate::check) struct GenericIndex {
    /// Template ids keyed by declaring source node.
    templates_by_source: FxIndexMap<dir::GlobalNodeIdAny, GenericTemplateId>,
    /// Template ids keyed by declaring symbol.
    templates_by_symbol: FxIndexMap<dir::GlobalSymbolId, GenericTemplateId>,
    /// Parameter ids keyed by parameter symbol.
    parameters_by_symbol: FxIndexMap<dir::GlobalSymbolId, GenericParameterId>,
    /// Induced parameters already rebound to their sites this run.
    claimed_induced: FxIndexSet<GenericParameterId>,
    /// Declaration types scanned for induced memory variables.
    induced_parameter_sites: Vec<InducedParameterSite>,
}

impl GenericIndex {
    /// Create an empty generic index.
    pub(in crate::check) fn new() -> Self {
        Self {
            templates_by_source: FxIndexMap::default(),
            templates_by_symbol: FxIndexMap::default(),
            parameters_by_symbol: FxIndexMap::default(),
            claimed_induced: FxIndexSet::default(),
            induced_parameter_sites: Vec::new(),
        }
    }

    /// Return the template declared at one source node.
    pub(in crate::check) fn template_by_source(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<GenericTemplateId> {
        self.templates_by_source.get(&source).copied()
    }

    /// Return the template declared by one symbol.
    pub(in crate::check) fn template_by_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<GenericTemplateId> {
        self.templates_by_symbol.get(&symbol).copied()
    }

    /// Return the parameter declared by one symbol.
    pub(in crate::check) fn parameter_by_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<GenericParameterId> {
        self.parameters_by_symbol.get(&symbol).copied()
    }

    /// Push one induced parameter site.
    pub(in crate::check) fn push_induced_parameter_site(&mut self, site: InducedParameterSite) {
        self.induced_parameter_sites.push(site);
    }

    /// Drain the induced parameter sites collected since the last propagation.
    pub(in crate::check) fn drain_induced_parameter_sites(&mut self) -> Vec<InducedParameterSite> {
        std::mem::take(&mut self.induced_parameter_sites)
    }

    /// Collect the site types pushed by one declaration.
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
}

impl CheckState<'_> {
    /// Index one module's declared generic identities for re-derivation.
    pub(in crate::check) fn index_declared_generics(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<()> {
        let segment = &self.module(module).generics;

        // collect declared rows before touching index and symbol state
        let mut templates = Vec::new();
        let mut parameters = Vec::new();
        for (local, template) in segment.iter_templates() {
            templates.push((local.into_global(module), template.source, template.symbol));
        }
        for (local, binding) in segment.iter_parameters() {
            if let dir::GenericParameterKey::Symbol(symbol) = binding.key {
                parameters.push((local.into_global(module), symbol, binding.ty));
            }
        }

        for (id, source, symbol) in templates {
            self.generics.templates_by_source.insert(source, id);
            if let Some(symbol) = symbol {
                self.generics.templates_by_symbol.insert(symbol, id);
            }
        }
        for (id, symbol, ty) in parameters {
            self.generics.parameters_by_symbol.insert(symbol, id);
            self.commit_declaration_type(symbol, ty)?;
        }

        Ok(())
    }

    /// Return one symbol's generic template, reading the walk index
    /// over committed definitions.
    pub(in crate::check) fn symbol_template(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<GenericTemplateId>> {
        if let Some(template) = self.generics.template_by_symbol(symbol) {
            return Ok(Some(template));
        }

        // committed definitions carry their declared templates
        let template = self
            .definition(symbol)?
            .and_then(|definition| definition.template());

        Ok(template.map(|template| template.into_global(symbol.module_id)))
    }

    /// Return one symbol's already loaded template, without importing.
    pub(in crate::check) fn loaded_symbol_template(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<GenericTemplateId> {
        if let Some(template) = self.generics.template_by_symbol(symbol) {
            return Some(template);
        }

        let template = self.loaded_definition(symbol)?.template()?;

        Some(template.into_global(symbol.module_id))
    }

    /// Return one generic template, reading working segments over external tables.
    pub(in crate::check) fn generic_template(
        &self,
        id: GenericTemplateId,
    ) -> Option<&dir::GenericTemplate> {
        // read working component templates first
        if let Some(module) = self.modules.get(&id.module_id)
            && let Some(template) = module.generics.get_local_template(id.local_id)
        {
            return Some(template);
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
        // read working component parameters first
        if let Some(module) = self.modules.get(&id.module_id)
            && let Some(parameter) = module.generics.get_local_parameter(id.local_id)
        {
            return Some(parameter);
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
    ) -> SmallVec<[GenericParameterId; 4]> {
        let Some(template) = self.generic_template(id) else {
            return SmallVec::new();
        };

        template
            .parameters
            .iter()
            .map(|parameter| parameter.into_global(id.module_id))
            .collect()
    }

    /// Return the owner parameters enclosing one member template.
    pub(in crate::check) fn owner_template_parameters(
        &mut self,
        id: GenericTemplateId,
    ) -> CompilerResult<SmallVec<[GenericParameterId; 4]>> {
        let mut parameters = SmallVec::new();
        let mut current = self
            .generic_template(id)
            .and_then(|template| template.parent)
            .map(|parent| parent.into_global(id.module_id));

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
                parameters.extend(self.generic_template_parameters(id));
            }
            current = self
                .generic_template(id)
                .and_then(|template| template.parent)
                .map(|parent| parent.into_global(id.module_id));
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
        if parameters.len() != arguments.len() {
            return Err(CompilerError::Internal {
                message: format!(
                    "generic argument count {} does not match parameter count {}",
                    arguments.len(),
                    parameters.len(),
                ),
            });
        }

        let mut bindings = Vec::with_capacity(parameters.len());
        for (parameter, argument) in parameters.iter().copied().zip(arguments.iter().copied()) {
            // lifetimes are proof-only and erase from instance identity,
            //  though their arguments still settle like every other slot
            let argument = self.generic_argument(parameter, argument)?;
            let is_lifetime = self.generic_parameter(parameter).is_some_and(|binding| {
                binding.memory_parameter() == Some(dir::MemoryParameter::Lifetime)
            });
            if is_lifetime {
                continue;
            }
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
        let parameters = self.generic_template_parameters(template);

        self.generic_argument_bindings(&parameters, arguments)
    }

    /// Return one finalized generic argument.
    fn generic_argument(
        &mut self,
        parameter: GenericParameterId,
        argument: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let argument = self.settled_root(argument)?;
        let Some(variable) = self.root_variable(argument)? else {
            return Ok(argument);
        };

        // preserve selected declaration parameters that did not receive evidence
        let state = self.solver.variable(variable)?;
        if !state.lower.is_empty() {
            return Ok(argument);
        }
        if self.solver.variable_role(variable)?.is_inference() {
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
        parent: Option<GenericTemplateId>,
        symbol: Option<dir::GlobalSymbolId>,
    ) -> CompilerResult<GenericTemplateId> {
        if let Some(template) = self.generics.template_by_source(source) {
            return Ok(template);
        }

        // templates nest lexically inside their own module
        let module = source.module_id;
        let parent = match parent {
            Some(parent) if parent.module_id != module => {
                return Err(CompilerError::Internal {
                    message: format!("generic template parent {parent:?} crosses modules"),
                });
            }
            Some(parent) => Some(parent.local_id),
            None => None,
        };

        // allocate the template in its module's working segment
        let working = self
            .modules
            .get_mut(&module)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check module {module:?} has no working generics"),
            })?;
        let local = working
            .generics
            .push_template(dir::GenericTemplate::new(source, symbol, parent));
        let id = local.into_global(module);

        self.generics.templates_by_source.insert(source, id);
        if let Some(symbol) = symbol {
            self.generics.templates_by_symbol.insert(symbol, id);
        }

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
        let working = self
            .modules
            .get_mut(&module)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check module {module:?} has no working generics"),
            })?;
        let local = dir::LocalGenericParameterId::new(working.generics.parameter_count());
        let id = local.into_global(module);
        let ty = working
            .types_tail
            .intern_type(dir::Type::Parameter(id), dir::TypeFlags::EMPTY)
            .into_global(module);
        let binding = dir::GenericParameterBinding {
            template: template.local_id,
            source,
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
        let local = working.generics.push_template_parameter(binding);
        if local != id.local_id {
            return Err(CompilerError::Internal {
                message: format!(
                    "generic parameter allocation changed from {:?} to {:?}",
                    id.local_id, local
                ),
            });
        }

        if let Some(symbol) = symbol {
            self.generics.parameters_by_symbol.insert(symbol, id);
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

        // claim the parameter this site already induced, in this run or
        //  during declaration
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
        self.generics.claimed_induced.insert(parameter);

        Ok(parameter)
    }

    /// Claim the induced parameter one site owns on a template, once per hole.
    fn claim_induced_parameter(
        &mut self,
        template: GenericTemplateId,
        site: dir::GlobalNodeIdAny,
        kind: dir::MemoryParameter,
    ) -> Option<GenericParameterId> {
        let parameters = self.generic_template_parameters(template);
        for parameter in parameters {
            let binding = self.generic_parameter(parameter)?;
            if binding.origin != dir::GenericParameterOrigin::Induced
                || binding.source != site
                || binding.kind != dir::GenericParameterKind::Memory(kind)
                || self.generics.claimed_induced.contains(&parameter)
            {
                continue;
            }
            self.generics.claimed_induced.insert(parameter);

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

        // commit the completed binding after classification
        let module = parameter.module_id;
        let working = self
            .modules
            .get_mut(&module)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check module {module:?} has no working generics"),
            })?;
        let Some(binding) = working.generics.get_local_parameter_mut(parameter.local_id) else {
            return Err(CompilerError::Internal {
                message: format!("generic parameter {parameter:?} is not in its working segment"),
            });
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
            .modules
            .get_mut(&module)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check module {module:?} has no working generics"),
            })?;
        let Some(declared) = working.generics.get_local_template_mut(template.local_id) else {
            return Err(CompilerError::Internal {
                message: format!("generic template {template:?} is not in its working segment"),
            });
        };

        // skip predicates the declaration pass already carries
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

    /// Return the assuming generic template carried by one work origin.
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
        let scope = self.origin_scope(origin)?;
        let scope = self.generic_scope(scope);

        // keep the bounds whose predicate subject matches
        let mut bounds = SmallVec::new();
        for predicate in scope.predicates.iter() {
            let left = self.ty(predicate.left)?;
            if subject(&left) {
                bounds.push(predicate.right);
            }
        }

        Ok(bounds)
    }

    /// Return one template's flattened generic scope, computing it once.
    pub(in crate::check) fn generic_scope(
        &mut self,
        scope: Option<GenericTemplateId>,
    ) -> Arc<GenericScope> {
        let Some(root) = scope else {
            return Arc::new(GenericScope::default());
        };
        if let Some(scope) = self.scopes.get(&root) {
            return scope.clone();
        }

        // flatten the scope chain once, innermost first
        let mut environment = GenericScope::default();
        let mut scope = Some(root);
        while let Some(id) = scope {
            let Some(template) = self.generic_template(id) else {
                break;
            };
            environment.parameters.extend(
                template
                    .parameters
                    .iter()
                    .map(|local| local.into_global(id.module_id)),
            );
            environment
                .predicates
                .extend(template.predicates.iter().cloned());

            scope = template
                .parent
                .map(|parent| parent.into_global(id.module_id));
        }

        let environment = Arc::new(environment);
        self.scopes.insert(root, environment.clone());

        environment
    }
}
