use std::sync::Arc;

use destack_dir as dir;
use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::check::{CheckState, Origin};
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

/// One declaration operand scanned for induced template generics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct GenericInductionSite {
    /// The declaration node that receives induced generic parameters.
    pub(in crate::check) declaration: dir::GlobalNodeIdAny,
    /// The enclosing generic template.
    pub(in crate::check) parent: Option<GenericTemplateId>,
    /// The declaration symbol indexed by the generated template.
    pub(in crate::check) symbol: Option<dir::GlobalSymbolId>,
    /// The declaration type to traverse.
    pub(in crate::check) ty: dir::GlobalTypeId,
}

/// One generic parameter induced from a declaration type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct GenericInductionParameter {
    /// The generated parameter name prefix.
    pub(in crate::check) name_prefix: &'static str,
    /// The optional generated parameter constraint.
    pub(in crate::check) constraint: Option<dir::GlobalTypeId>,
    /// Whether arguments must solve to singleton types.
    pub(in crate::check) is_comptime: bool,
    /// The reason this parameter was induced.
    pub(in crate::check) induction: dir::GenericParameterInduction,
}

/// Generic declaration index over working generic segments.
#[derive(Debug)]
pub(in crate::check) struct GenericIndex {
    /// Template ids keyed by declaring source node.
    templates_by_source: IndexMap<dir::GlobalNodeIdAny, GenericTemplateId>,
    /// Template ids keyed by declaring symbol.
    templates_by_symbol: IndexMap<dir::GlobalSymbolId, GenericTemplateId>,
    /// Parameter ids keyed by parameter symbol.
    parameters_by_symbol: IndexMap<dir::GlobalSymbolId, GenericParameterId>,
    /// Declaration types scanned for induced template generics.
    induction_sites: Vec<GenericInductionSite>,
    /// Variables that can induce template generics.
    inductions: IndexMap<dir::TypeVariableId, GenericInductionParameter>,
}

impl GenericIndex {
    /// Create an empty generic index.
    pub(in crate::check) fn new() -> Self {
        Self {
            templates_by_source: IndexMap::new(),
            templates_by_symbol: IndexMap::new(),
            parameters_by_symbol: IndexMap::new(),
            induction_sites: Vec::new(),
            inductions: IndexMap::new(),
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

    /// Push one induction site.
    pub(in crate::check) fn push_induction_site(&mut self, site: GenericInductionSite) {
        self.induction_sites.push(site);
    }

    /// Iterate induction sites in component order.
    pub(in crate::check) fn induction_sites(
        &self,
    ) -> impl Iterator<Item = &GenericInductionSite> + '_ {
        self.induction_sites.iter()
    }

    /// Mark one variable as able to induce a template generic.
    pub(in crate::check) fn insert_induction(
        &mut self,
        variable: dir::TypeVariableId,
        parameter: GenericInductionParameter,
    ) -> CompilerResult<()> {
        let previous = self.inductions.insert(variable, parameter);

        if previous.is_some() {
            return Err(CompilerError::Internal {
                message: format!("variable {variable:?} has conflicting generic induction"),
            });
        }

        Ok(())
    }

    /// Move one variable's induced parameter onto its representative.
    pub(in crate::check) fn merge_induction(
        &mut self,
        variable: dir::TypeVariableId,
        target: dir::TypeVariableId,
    ) {
        let Some(induction) = self.inductions.swap_remove(&variable) else {
            return;
        };
        self.inductions.entry(target).or_insert(induction);
    }

    /// Return the generic parameter induced by one variable.
    pub(in crate::check) fn induction(
        &self,
        variable: dir::TypeVariableId,
    ) -> Option<GenericInductionParameter> {
        self.inductions.get(&variable).copied()
    }
}

impl CheckState<'_> {
    /// Return one symbol's generic template, reading the walk index
    /// over committed definitions for external symbols.
    pub(in crate::check) fn symbol_template(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<GenericTemplateId> {
        if let Some(template) = self.generics.template_by_symbol(symbol) {
            return Some(template);
        }

        // committed definitions carry their declared templates
        let template = self.definition(symbol)?.template()?;

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
        &self,
        id: GenericTemplateId,
    ) -> SmallVec<[GenericParameterId; 4]> {
        let mut parameters = SmallVec::new();
        let mut current = self
            .generic_template(id)
            .and_then(|template| template.parent)
            .map(|parent| parent.into_global(id.module_id));

        // collect enclosing owner parameters outermost last
        while let Some(id) = current {
            let template = self.generic_template(id);
            let symbol = template.and_then(|template| template.symbol);
            let is_owner = symbol.is_some_and(|symbol| {
                matches!(
                    self.definition(symbol),
                    Some(
                        dir::Definition::Extension(_)
                            | dir::Definition::Class(_)
                            | dir::Definition::Struct(_)
                            | dir::Definition::Enum(_)
                            | dir::Definition::Interface(_)
                            | dir::Definition::Newtype(_)
                    )
                )
            });
            if is_owner {
                parameters.extend(self.generic_template_parameters(id));
            }
            current = self
                .generic_template(id)
                .and_then(|template| template.parent)
                .map(|parent| parent.into_global(id.module_id));
        }

        parameters
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
        &self,
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

        let bindings = parameters
            .iter()
            .copied()
            .zip(arguments.iter().copied())
            .map(|(parameter, argument)| dir::GenericArgumentBinding::new(parameter, argument))
            .collect();

        Ok(bindings)
    }

    /// Return applied generic argument bindings for one symbol template.
    pub(in crate::check) fn symbol_generic_argument_bindings(
        &self,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Vec<dir::GenericArgumentBinding>> {
        let Some(template) = self.symbol_template(symbol) else {
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
        binding: dir::GenericParameterBinding,
        template: GenericTemplateId,
        symbol: Option<dir::GlobalSymbolId>,
    ) -> CompilerResult<GenericParameterId> {
        // allocate the parameter in its template's working segment
        let module = template.module_id;
        let working = self
            .modules
            .get_mut(&module)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check module {module:?} has no working generics"),
            })?;
        let local = working.generics.push_template_parameter(binding);
        let id = local.into_global(module);

        if let Some(symbol) = symbol {
            self.generics.parameters_by_symbol.insert(symbol, id);
        }

        Ok(id)
    }

    /// Push one induced generic parameter.
    pub(in crate::check) fn push_induced_generic_parameter(
        &mut self,
        template: GenericTemplateId,
        parameter: GenericInductionParameter,
    ) -> CompilerResult<GenericParameterId> {
        // generate the parameter name from its template position
        let number = self
            .generic_template(template)
            .map_or(0, |template| template.parameters.len());
        let name = self
            .module_mut(template.module_id)
            .strings
            .intern(&format!("{}{number}", parameter.name_prefix));

        let binding = dir::GenericParameterBinding {
            template: template.local_id,
            key: dir::GenericParameterKey::Generated(name),
            origin: dir::GenericParameterOrigin::Induced(parameter.induction),
            variance: None,
            constraint: parameter.constraint,
            default: None,
            is_variadic: false,
            is_const: false,
            is_comptime: parameter.is_comptime,
        };

        self.push_generic_parameter(binding, template, None)
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
        declared.predicates.push(predicate);

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
    pub(in crate::check) fn origin_scope(&self, origin: Origin) -> Option<GenericTemplateId> {
        match origin {
            Origin::Node(_, scope) => scope,
            Origin::Symbol(symbol) => self.symbol_template(symbol),
        }
    }

    /// Return one work origin re-anchored at a node under the same assumptions.
    pub(in crate::check) fn origin_at(&self, origin: Origin, node: dir::GlobalNodeIdAny) -> Origin {
        Origin::Node(node, self.origin_scope(origin))
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
    ///
    /// The scope chain walks enclosing templates, so a method assumes
    /// its own predicates and those of its enclosing declarations.
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
        let scope = self.origin_scope(origin);
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
