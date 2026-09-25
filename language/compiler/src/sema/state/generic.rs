use smallvec::SmallVec;
use tspp_artifact::{DirBound, DirParsed, DirResolved};
use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::sema::{CheckState, Origin, TypeSubstitution, VariableKind};
use crate::{CompilerError, CompilerResult};

/// One template parameter as its declaration writes it.
#[derive(Debug, Clone, Copy)]
pub(in crate::sema) struct TemplateParameter {
    /// The kind the declaration and its constraint name.
    pub(in crate::sema) kind: dir::GenericParameterKind,
    /// Whether the declaration writes a default.
    pub(in crate::sema) has_default: bool,
}

/// Stable id for one declaration-side generic parameter.
pub(in crate::sema) type GenericParameterId = dir::GlobalGenericParameterId;

/// Stable id for one generic binding site.
pub(in crate::sema) type GenericTemplateId = dir::GlobalGenericTemplateId;

impl CheckState<'_> {
    /// Return one symbol's generic template.
    pub(in crate::sema) fn symbol_template(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<GenericTemplateId>> {
        self.template_by_symbol(symbol)
    }

    /// Return the template declared by one symbol, through its module's generic view.
    pub(in crate::sema) fn template_by_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<GenericTemplateId>> {
        let module = symbol.module_id;

        // read the checked module's written stages
        if let Some(state) = self.module_maybe(module) {
            return Ok(state
                .written_template_by_symbol(symbol)
                .map(|local| local.into_global(module)));
        }

        // read external committed templates
        Ok(self.external(module)?.and_then(|external| {
            external
                .generics()
                .template_by_symbol(symbol)
                .map(|local| local.into_global(module))
        }))
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
    ) -> CompilerResult<Option<GenericParameterId>> {
        let module = symbol.module_id;

        // read the checked module's written stages
        if let Some(state) = self.module_maybe(module) {
            return Ok(state
                .written_parameter_by_symbol(symbol)
                .map(|local| local.into_global(module)));
        }

        // read external committed parameters
        Ok(self.external(module)?.and_then(|external| {
            external
                .generics()
                .parameter_by_symbol(symbol)
                .map(|local| local.into_global(module))
        }))
    }

    /// Return one generic template, reading the checked view over external tables.
    pub(in crate::sema) fn generic_template(
        &self,
        id: GenericTemplateId,
    ) -> CompilerResult<Option<&dir::GenericTemplate>> {
        // read the checked module's tail over its committed base
        if self.is_own_module(id.module_id) {
            return Ok(self.module.generic_template(id.local_id));
        }

        // read external committed templates
        Ok(self
            .external(id.module_id)?
            .map(|external| external.generics().get_template(id.local_id)))
    }

    /// Return one generic parameter, reading the module's own segments over external tables.
    pub(in crate::sema) fn generic_parameter(
        &self,
        id: GenericParameterId,
    ) -> CompilerResult<Option<&dir::GenericParameterBinding>> {
        // read the checked module's tail over its committed base
        if self.is_own_module(id.module_id) {
            return Ok(self.module.generic_parameter(id.local_id));
        }

        // read external committed parameters
        Ok(self
            .external(id.module_id)?
            .map(|external| external.generics().get_parameter(id.local_id)))
    }

    /// Return whether one generic parameter is a memory parameter.
    pub(in crate::sema) fn is_memory_parameter(
        &self,
        id: GenericParameterId,
    ) -> CompilerResult<bool> {
        Ok(self
            .generic_parameter(id)?
            .is_some_and(|parameter| parameter.memory_parameter().is_some()))
    }

    /// Return whether one generic parameter is a lifetime.
    pub(in crate::sema) fn is_lifetime_parameter(
        &self,
        id: GenericParameterId,
    ) -> CompilerResult<bool> {
        Ok(self.generic_parameter(id)?.is_some_and(|parameter| {
            parameter.memory_parameter() == Some(dir::MemoryParameter::Region)
        }))
    }

    /// Return the canonical type denoting one generic parameter.
    pub(in crate::sema) fn generic_parameter_type(
        &self,
        id: GenericParameterId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.generic_parameter(id)?
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

        // read each written parameter under the template's module
        let mut parameters = SmallVec::new();
        for parameter in template.parameters.clone() {
            let parameter = parameter.into_global(id.module_id);
            if self.require_generic_parameter(parameter)?.is_writable() {
                parameters.push(parameter);
            }
        }

        Ok(parameters)
    }

    /// Return the owner parameters enclosing one member template.
    pub(in crate::sema) fn owner_template_parameters(
        &mut self,
        id: GenericTemplateId,
    ) -> CompilerResult<SmallVec<[GenericParameterId; 4]>> {
        let mut parameters = SmallVec::new();
        for owner in self.owner_templates(id)?.into_iter().rev() {
            parameters.extend(self.generic_template_parameters(owner)?);
        }

        Ok(parameters)
    }

    /// Return the templates of the definitions enclosing one template, outermost first.
    pub(in crate::sema) fn owner_templates(
        &mut self,
        id: GenericTemplateId,
    ) -> CompilerResult<SmallVec<[GenericTemplateId; 4]>> {
        let mut owners = SmallVec::new();
        let mut current = self.parent_generic_template(id)?;

        // collect enclosing owner templates innermost first, then turn them outermost first
        while let Some(id) = current {
            let template = self.generic_template(id)?;
            let symbol = template.and_then(|template| template.symbol);
            let is_owner = match symbol {
                Some(symbol) => matches!(
                    self.definition(symbol)?.as_deref(),
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
                owners.push(id);
            }
            current = self.parent_generic_template(id)?;
        }
        owners.reverse();

        Ok(owners)
    }

    /// Return the unbound generic parameters of one callable signature.
    pub(in crate::sema) fn signature_generic_parameters(
        &mut self,
        module: ModuleId,
        signature: &dir::FunctionSignatureType,
    ) -> CompilerResult<SmallVec<[GenericParameterId; 4]>> {
        let Some(template_id) = signature.template else {
            return Ok(SmallVec::new());
        };

        // omit parameters fixed by an explicit application
        let arguments = self
            .signature_arguments(module, signature.arguments)?
            .to_vec();
        let mut parameters = self.generic_template_parameters(template_id)?;
        if signature.is_construct {
            let mut enclosing = self.owner_template_parameters(template_id)?;
            enclosing.extend(parameters);
            parameters = enclosing;
        }
        parameters.retain(|parameter| {
            !arguments
                .iter()
                .any(|binding| binding.parameter == *parameter)
        });

        Ok(parameters)
    }

    /// Return the generic argument bindings one nominal application carries, empty for other heads.
    pub(in crate::sema) fn application_generic_argument_bindings(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Vec<dir::GenericArgumentBinding>> {
        let Some((module, instance)) = self.nominal_application_maybe(ty)? else {
            return Ok(Vec::new());
        };
        let arguments = self.type_ids(module, instance.arguments)?;

        self.symbol_generic_argument_bindings(instance.symbol, arguments)
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
            if self.is_lifetime_parameter(binding.parameter)? {
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
    ) -> CompilerResult<usize> {
        let mut count = 0;
        for parameter in parameters {
            if self
                .generic_parameter(*parameter)?
                .is_some_and(dir::GenericParameterBinding::is_writable)
            {
                count += 1;
            }
        }

        Ok(count)
    }

    /// Return one generic template that must already be loaded.
    pub(in crate::sema) fn require_generic_template(
        &self,
        id: GenericTemplateId,
    ) -> CompilerResult<&dir::GenericTemplate> {
        self.generic_template(id)?
            .ok_or_else(|| CompilerError::Internal {
                message: format!("generic template {id:?} is missing"),
            })
    }

    /// Return one generic parameter that must already be loaded.
    pub(in crate::sema) fn require_generic_parameter(
        &self,
        id: GenericParameterId,
    ) -> CompilerResult<&dir::GenericParameterBinding> {
        self.generic_parameter(id)?
            .ok_or_else(|| CompilerError::Internal {
                message: format!("generic parameter {id:?} is not bound"),
            })
    }

    /// Build one class construction target, keying the class at its own bindings.
    pub(in crate::sema) fn class_construct_target(
        &mut self,
        symbol: dir::GlobalSymbolId,
        constructor: dir::ClassConstructor,
        arguments: Vec<dir::GenericArgumentBinding>,
    ) -> CompilerResult<dir::ConstructTarget> {
        // key the class at its own bindings
        let key = self.instance_key_bindings(symbol, &arguments)?;

        Ok(dir::ConstructTarget::Class {
            key: dir::InstanceKey::new(symbol, key),
            constructor,
            arguments,
        })
    }

    /// Return the bindings one instance key of a symbol closes, outermost template first.
    pub(in crate::sema) fn instance_key_bindings(
        &mut self,
        symbol: dir::GlobalSymbolId,
        bindings: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<Vec<dir::GenericArgumentBinding>> {
        // find the nearest template of the symbol or of its owners
        let mut template = None;
        let mut current = Some(symbol);
        while let Some(symbol) = current {
            template = self.symbol_template(symbol)?;
            if template.is_some() {
                break;
            }
            current = self.member_owner(symbol)?;
        }

        // collect the enclosing template chain outermost first
        let mut chain = Vec::new();
        while let Some(parent) = template {
            chain.push(parent);
            template = self.parent_generic_template(parent)?;
        }
        chain.reverse();

        // keep the bindings of the chain's parameters in declaration order
        let mut kept = Vec::new();
        for template in chain {
            for parameter in self.generic_template_parameters(template)? {
                kept.extend(
                    bindings
                        .iter()
                        .copied()
                        .find(|binding| binding.parameter == parameter),
                );
            }
        }

        Ok(kept)
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
            let Some(binding) = self.generic_parameter(parameter)? else {
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
        parent: Option<GenericTemplateId>,
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

        // nest the template under the one its opener walks in, a parent living in the same module
        if let Some(parent) = parent
            && parent.module_id != module_id
        {
            return Err(CompilerError::Internal {
                message: format!(
                    "generic template {source:?} nests under foreign template {parent:?}"
                ),
            });
        }
        let parent = parent.map(|parent| parent.local_id);

        // allocate the template in its owning module, dropping the caches it invalidates
        self.argument_ranks.clear();
        let module = self
            .module_maybe_mut(module_id)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check module {module_id:?} has no generic segment"),
            })?;
        let local = module
            .generics_tail
            .push_template(dir::GenericTemplate::new(source, scope, symbol, parent));
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
                message: format!("a generic parameter of the foreign module {module:?}"),
            });
        }
        self.argument_ranks.clear();
        let local = dir::LocalGenericParameterId::new(self.module.generics_tail.parameter_count());
        let id = local.into_global(module);
        let ty = self
            .module
            .types_tail
            .intern_type(dir::Type::Parameter(id), kind.parameter_flags())
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

        // allocate the parameter in its template's own segment
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
    pub(in crate::sema) fn argument_fills_parameter(
        &self,
        binding: &dir::GenericParameterBinding,
        argument: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        self.argument_fills_kind(binding.kind, argument)
    }

    /// Return whether one written argument fills a parameter of the given kind.
    pub(in crate::sema) fn argument_fills_kind(
        &self,
        kind: dir::GenericParameterKind,
        argument: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // admit the argument by the parameter kind
        match kind {
            dir::GenericParameterKind::Memory(memory) => {
                Ok(self.memory_kind(argument)? == Some(memory))
            }
            dir::GenericParameterKind::Type => Ok(true),
        }
    }

    /// Return one template's parameters in declaration order with their kinds and defaults.
    pub(in crate::sema) fn template_parameters(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Vec<TemplateParameter>> {
        let module = symbol.module_id;
        let parsed = self
            .artifacts
            .read::<DirParsed>(module)
            .map_err(CompilerError::from)?;
        let bound = self
            .artifacts
            .read::<DirBound>((module, self.profile))
            .map_err(CompilerError::from)?;
        let resolved = self
            .artifacts
            .read::<DirResolved>((module, self.profile))
            .map_err(CompilerError::from)?;
        let bindings = dir::BindingTable::from_segment(bound.bindings.clone());
        let Some(declaration) =
            bindings
                .get_symbol(symbol.local_id)
                .declaration
                .and_then(|declaration| {
                    declaration
                        .local_id
                        .try_into_typed::<dir::Declaration>()
                        .ok()
                })
        else {
            return Err(CompilerError::Internal {
                message: "a template application of a symbol without a declaration".to_string(),
            });
        };
        let view = dir::View::new(&parsed.tree);
        let mut parameters = Vec::new();
        for id in view
            .get(declaration)
            .generic_parameters()
            .unwrap_or_default()
        {
            let parameter = view.get(*id);
            let constraint = match parameter {
                dir::GenericParameter::Type { constraint, .. }
                | dir::GenericParameter::VariadicType { constraint, .. } => *constraint,
                _ => None,
            };
            let has_default = matches!(
                parameter,
                dir::GenericParameter::Type {
                    default: Some(_),
                    ..
                } | dir::GenericParameter::VariadicType {
                    default: Some(_),
                    ..
                }
            );
            parameters.push(TemplateParameter {
                kind: self
                    .declared_parameter_kind(module, &view, &resolved, parameter, constraint)?,
                has_default,
            });
        }

        Ok(parameters)
    }

    /// Classify one declared parameter by its declaration and its constraint's language item.
    pub(in crate::sema) fn declared_parameter_kind(
        &self,
        module: ModuleId,
        view: &dir::View<'_>,
        resolved: &DirResolved,
        parameter: &dir::GenericParameter,
        constraint: Option<dir::LocalNodeId<dir::TypeExpression>>,
    ) -> CompilerResult<dir::GenericParameterKind> {
        let item = match constraint {
            Some(constraint) => resolved
                .references
                .get(constraint.into_global_any(module))
                .and_then(|reference| reference.symbols())
                .and_then(|symbols| symbols.first().copied())
                .map(|target| self.language_item(target))
                .transpose()?
                .flatten(),
            None => None,
        };
        let kind = dir::GenericParameterKind::declared(parameter, item);

        // read the memory domain a constraint of reserved literals names, a subset of one kind
        let domain = match (kind, constraint) {
            (dir::GenericParameterKind::Type, Some(constraint)) => {
                self.literal_memory_domain(view, constraint)
            }
            _ => None,
        };

        Ok(match domain {
            Some(memory) => dir::GenericParameterKind::Memory(memory),
            None => kind,
        })
    }

    /// Return the memory domain every literal of one constraint inhabits.
    fn literal_memory_domain(
        &self,
        view: &dir::View<'_>,
        constraint: dir::LocalNodeId<dir::TypeExpression>,
    ) -> Option<dir::MemoryParameter> {
        match view.get(constraint) {
            dir::TypeExpression::Literal {
                value: dir::Literal::String(text),
            } => {
                let text = self.strings().get(*text);
                if dir::Lifetime::parse(text).is_some() {
                    Some(dir::MemoryParameter::Region)
                } else if dir::Access::from_text(text).is_some() {
                    Some(dir::MemoryParameter::Access)
                } else {
                    None
                }
            }
            dir::TypeExpression::Union { elements } => {
                let mut shared = None;
                for element in elements {
                    let domain = self.literal_memory_domain(view, *element)?;
                    match shared {
                        Some(current) if current != domain => return None,
                        _ => shared = Some(domain),
                    }
                }

                shared
            }
            _ => None,
        }
    }

    /// Return the memory kind one type term inhabits.
    pub(in crate::sema) fn memory_kind(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::MemoryParameter>> {
        let id = self.shallow_resolve(id)?;

        // read the memory kind of each term
        match self.ty(id)? {
            // build the region pair at the lifetime kind
            dir::Type::Region(_) => Ok(Some(dir::MemoryParameter::Region)),

            // reserved lifetime, space, and access names write as string literals
            dir::Type::Literal(dir::Literal::String(value)) => {
                if dir::Lifetime::parse(self.strings().get(value)).is_some() {
                    Ok(Some(dir::MemoryParameter::Region))
                } else if dir::Access::from_text(self.strings().get(value)).is_some() {
                    Ok(Some(dir::MemoryParameter::Access))
                } else {
                    Ok(None)
                }
            }

            // parameters inhabit their declared kind or the kind their constraint names
            dir::Type::Parameter(parameter) | dir::Type::Erased(parameter) => {
                let Some(binding) = self.generic_parameter(parameter)? else {
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

            // name the kind of each memory domain
            dir::Type::Application(instance) => Ok(self
                .language_item(instance.symbol)?
                .and_then(dir::MemoryParameter::from_language_item)),

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
        memory: dir::MemoryParameter,
    ) -> CompilerResult<GenericParameterId> {
        let constraint = self.memory_parameter_constraint(memory)?;

        // push the induced parameter onto the template
        self.push_generic_parameter(
            template,
            site,
            None,
            dir::GenericParameterKey::Anonymous,
            None,
            constraint,
            None,
            dir::GenericParameterOrigin::Induced,
            dir::GenericParameterKind::Memory(memory),
            false,
            false,
        )
    }

    /// Push one walked where-clause predicate onto its declaring template.
    pub(in crate::sema) fn push_template_predicate(
        &mut self,
        template: GenericTemplateId,
        predicate: dir::WherePredicate,
    ) -> CompilerResult<()> {
        let module = template.module_id;
        let state = self
            .module_maybe_mut(module)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("a generic parameter of the foreign module {module:?}"),
            })?;

        // skip templates the declared stage closed
        let Some(declared) = state
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
    ) -> CompilerResult<SmallVec<[dir::WherePredicate; 2]>> {
        let Some(template) = template else {
            return Ok(SmallVec::new());
        };

        Ok(self
            .generic_template(template)?
            .map(|template| SmallVec::from_slice(&template.predicates))
            .unwrap_or_default())
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

    /// Collect one parameter's bounds at an origin.
    pub(in crate::sema) fn parameter_bounds(
        &mut self,
        origin: Origin,
        parameter: GenericParameterId,
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 2]>> {
        let scope = self.origin_scope(origin)?;

        self.parameter_bounds_under(scope, parameter)
    }

    /// Collect one parameter's bounds under its declaring template.
    pub(in crate::sema) fn declared_parameter_bounds(
        &mut self,
        parameter: GenericParameterId,
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 2]>> {
        let template = self
            .require_generic_parameter(parameter)?
            .template
            .into_global(parameter.module_id);

        self.parameter_bounds_under(Some(template), parameter)
    }

    /// Collect one parameter's bounds under one template chain.
    fn parameter_bounds_under(
        &mut self,
        template: Option<GenericTemplateId>,
        parameter: GenericParameterId,
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 2]>> {
        // read the declared constraint and the predicates bounding the parameter
        let mut bounds = SmallVec::new();
        let constraint = self
            .generic_parameter(parameter)?
            .ok_or_else(|| CompilerError::Internal {
                message: format!("a generic parameter {parameter:?} without its binding"),
            })?
            .constraint;
        bounds.extend(constraint);
        let predicates = self.predicates_under(template)?;
        for bound in self.subject_bounds(
            &predicates,
            |ty| matches!(ty, dir::Type::Parameter(subject) if *subject == parameter),
        )? {
            if !bounds.contains(&bound) {
                bounds.push(bound);
            }
        }

        // lend the bound of an associated member a bound equates to the parameter
        let direct = bounds.clone();
        for bound in direct {
            for element in self.implied_bounds(bound, parameter)? {
                if !bounds.contains(&element) {
                    bounds.push(element);
                }
            }
        }
        Ok(bounds)
    }

    /// Return the interface applications one bound lends a parameter through an equated member.
    fn implied_bounds(
        &mut self,
        bound: dir::GlobalTypeId,
        parameter: GenericParameterId,
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 2]>> {
        let mut implied = SmallVec::new();
        let (base, bindings) = self.refinements(bound)?;
        for (key, value) in bindings {
            let value = self.shallow_resolve(value)?;
            if !matches!(self.ty(value)?, dir::Type::Parameter(bound_parameter) if bound_parameter == parameter)
            {
                continue;
            }
            let Some(constraint) = self.associated_member_constraint(base, key)? else {
                continue;
            };
            let constraint = self.instantiate_interface_type(constraint, bound, value)?;
            let elements = match self.ty(constraint)? {
                dir::Type::Intersection(intersection) => SmallVec::<[_; 4]>::from_slice(
                    self.type_ids(constraint.module_id, intersection.elements)?,
                ),
                _ => SmallVec::from_slice(&[constraint]),
            };

            // lend the interface applications alone
            for element in elements {
                if self.is_conformance_target(element)? && !implied.contains(&element) {
                    implied.push(element);
                }
            }
        }

        Ok(implied)
    }

    /// Return the bound one interface declares on an associated member.
    pub(in crate::sema) fn associated_member_constraint(
        &mut self,
        interface: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some((_, instance)) = self.nominal_application_maybe(interface)? else {
            return Ok(None);
        };
        let declared = self.definition(instance.symbol)?;
        let Some(dir::Definition::Interface(definition)) = declared.as_deref() else {
            return Ok(None);
        };
        let member = definition
            .members
            .iter()
            .find(|member| member.is_associated_at(key))
            .cloned();

        Ok(match member {
            Some(dir::DefinitionMember::AssociatedType(associated)) => associated.constraint,
            Some(dir::DefinitionMember::AssociatedConst(associated)) => {
                Some(self.symbol_type(associated.symbol)?)
            }
            _ => None,
        })
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
        let scope = self.origin_scope(origin)?;

        self.predicates_under(scope)
    }

    /// Collect the where predicates one template chain declares.
    fn predicates_under(
        &mut self,
        template: Option<GenericTemplateId>,
    ) -> CompilerResult<SmallVec<[dir::WherePredicate; 2]>> {
        let mut template = template;
        let mut predicates = SmallVec::new();
        while let Some(id) = template {
            let declared = self.require_generic_template(id)?;
            predicates.extend_from_slice(&declared.predicates);
            template = self.parent_generic_template(id)?;
        }

        Ok(predicates)
    }

    /// Collect the bounds one predicate set grants a matching subject.
    pub(in crate::sema) fn subject_bounds(
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

    /// Return the substitution one applied argument list selects, filling elided slots.
    pub(in crate::sema) fn applied_substitution(
        &mut self,
        template: GenericTemplateId,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<TypeSubstitution> {
        let parameters = self.generic_template_parameters(template)?;

        self.parameter_substitution(&parameters, arguments)
    }

    /// Slot one applied argument list over ordered parameters, filling elided slots.
    pub(in crate::sema) fn parameter_substitution(
        &mut self,
        parameters: &[GenericParameterId],
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<TypeSubstitution> {
        // reject argument lists longer than the parameter list
        if arguments.len() > parameters.len() {
            return Err(CompilerError::Internal {
                message: format!(
                    "an application received {} arguments for {} parameters",
                    arguments.len(),
                    parameters.len(),
                ),
            });
        }

        // bind a complete argument list positionally, blind to unresolved argument shapes
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

        // slot each parameter over the written arguments in order
        let mut substitution = TypeSubstitution::default();
        let mut cursor = 0usize;
        for parameter in parameters.iter().copied() {
            let binding = self.generic_parameter(parameter)?.cloned().ok_or_else(|| {
                CompilerError::Internal {
                    message: format!("generic parameter {parameter:?} is missing"),
                }
            })?;

            // consume only written arguments of the memory parameter's kind
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
            // keep omitted region parameters symbolic for the erased instance
            else if binding.memory_parameter() == Some(dir::MemoryParameter::Region) {
                self.intern_type(dir::Type::Parameter(parameter))?
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

            // keep foreign applications symbolic while declaring
            if !self.reads_module(instance.symbol.module_id) {
                return Ok(TypeSubstitution::default());
            }

            let name = self.format_symbol(instance.symbol);
            let mut rendered = Vec::new();
            for argument in self.type_ids(module, instance.arguments)? {
                rendered.push(self.format_type(*argument));
            }
            let rendered = rendered.join(", ");

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

    /// Return the nearest template enclosing one template's scope.
    pub(in crate::sema) fn parent_generic_template(
        &self,
        template_id: GenericTemplateId,
    ) -> CompilerResult<Option<GenericTemplateId>> {
        let parent = self.require_generic_template(template_id)?.parent;

        Ok(parent.map(|parent| parent.into_global(template_id.module_id)))
    }

    /// Update one declared parameter's walked bounds.
    pub(in crate::sema) fn update_generic_parameter_bounds(
        &mut self,
        parameter: GenericParameterId,
        constraint: Option<dir::GlobalTypeId>,
        default: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<()> {
        let current =
            self.generic_parameter(parameter)?
                .cloned()
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("generic parameter {parameter:?} is not bound"),
                })?;

        // store memory defaults canonically, like written memory arguments
        let default = match (current.kind, default) {
            (dir::GenericParameterKind::Memory(_), Some(default)) => {
                let origin = Origin::Node(current.source, None);

                Some(self.normalize_memory_component(origin, default)?)
            }
            _ => default,
        };

        // commit the completed binding after classification
        let module = parameter.module_id;
        let state = self
            .module_maybe_mut(module)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("a generic parameter of the foreign module {module:?}"),
            })?;

        // skip parameters the declared stage closed
        let Some(binding) = state
            .generics_tail
            .get_local_parameter_mut(parameter.local_id)
        else {
            return Err(CompilerError::Internal {
                message: format!("a generic parameter {parameter:?} outside its module's generics"),
            });
        };
        binding.constraint = constraint;
        binding.default = default;

        Ok(())
    }

    /// Push the receiver parameter an interface template declares implicitly, bounded by itself.
    pub(in crate::sema) fn push_receiver_parameter(
        &mut self,
        template: GenericTemplateId,
        source: dir::GlobalNodeIdAny,
        interface: dir::GlobalSymbolId,
    ) -> CompilerResult<GenericParameterId> {
        let application = self.declaration_instance(interface)?;
        let constraint = self.intern_type(dir::Type::Application(application))?;

        self.push_generic_parameter(
            template,
            source,
            None,
            dir::GenericParameterKey::Anonymous,
            None,
            Some(constraint),
            None,
            dir::GenericParameterOrigin::Receiver,
            dir::GenericParameterKind::Type,
            false,
            false,
        )
    }

    /// Return whether one node lies inside an interface template, `this` being its receiver.
    pub(in crate::sema) fn is_within_interface(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<bool> {
        let mut current = self.template_at_node(node)?;
        while let Some(id) = current {
            let symbol = self
                .generic_template(id)?
                .and_then(|template| template.symbol);
            if let Some(symbol) = symbol
                && self.symbol_kind(symbol)?.is_interface()
            {
                return Ok(true);
            }
            current = self.parent_generic_template(id)?;
        }

        Ok(false)
    }

    /// Return one origin anchored at a node under the template in effect there.
    pub(in crate::sema) fn anchored_origin(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Origin> {
        Ok(Origin::Node(node, self.template_at_node(node)?))
    }

    /// Return the template in effect at one node of a loaded module.
    pub(in crate::sema) fn template_at_node(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Option<GenericTemplateId>> {
        // read the scope the node sits in of a readable module
        let module = node.module_id;
        if !self.reads_module(module) {
            return Ok(None);
        }
        let bindings = self.binding_table(module)?;
        let Some(scope) = bindings.scope_for_node(node) else {
            return Ok(None);
        };
        let scopes: Vec<_> = std::iter::once(scope.id)
            .chain(bindings.scope_ancestors(scope.id).map(|scope| scope.id))
            .collect();

        // take the nearest template governing that scope or an ancestor
        for scope in scopes {
            if let Some(template) = self.scope_template(module, scope)? {
                return Ok(Some(template));
            }
        }

        Ok(None)
    }

    /// Return the template governing one scope of a loaded module.
    fn scope_template(
        &self,
        module: ModuleId,
        scope: dir::LocalScopeId,
    ) -> CompilerResult<Option<GenericTemplateId>> {
        // read the template the scope writes
        let local = match self.module_maybe(module) {
            Some(state) => state.written_template_by_scope(scope),
            None => self
                .external(module)?
                .and_then(|external| external.generics().template_by_scope(scope)),
        };

        Ok(local.map(|id| id.into_global(module)))
    }

    /// Settle applied generic argument bindings for checked DIR.
    pub(in crate::sema) fn resolved_argument_bindings(
        &mut self,
        applied: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<Vec<dir::GenericArgumentBinding>> {
        // resolve each argument, every region with the storage its instance takes
        let mut bindings = Vec::with_capacity(applied.len());
        for applied in applied {
            let argument = self.shallow_resolve(applied.argument)?;
            bindings.push(dir::GenericArgumentBinding::new(
                applied.parameter,
                argument,
            ));
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

    /// Return the region bindings one substitution solved, the lifetimes an instance key erases.
    pub(in crate::sema) fn resolved_region_bindings(
        &mut self,
        applied: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<Vec<dir::GenericArgumentBinding>> {
        let mut bindings = Vec::new();
        for binding in applied {
            if !self.is_lifetime_parameter(binding.parameter)? {
                continue;
            }
            let argument = self.shallow_resolve(binding.argument)?;
            bindings.push(dir::GenericArgumentBinding::new(
                binding.parameter,
                argument,
            ));
        }

        Ok(bindings)
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
        let Some(declared) = self.generic_parameter(parameter)? else {
            return Ok((usize::MAX, None, usize::MAX));
        };
        let template = declared.template.into_global(parameter.module_id);
        let position = self
            .generic_template(template)?
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
