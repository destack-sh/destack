use destack_dir as dir;
use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::check::{CheckState, Origin, Substitution, Widening};
use crate::{CompilerError, CompilerResult};

/// Stable id for one declaration-side generic parameter.
pub(in crate::check) type GenericParameterId = dir::GlobalGenericParameterId;

/// Stable id for one generic binding site.
pub(in crate::check) type GenericTemplateId = dir::GlobalGenericTemplateId;

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

/// One generated generic parameter recipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct GenericInductionParameter {
    /// The generated parameter name prefix.
    pub(in crate::check) prefix: &'static str,
    /// The optional generated parameter constraint.
    pub(in crate::check) constraint: Option<dir::GlobalTypeId>,
    /// Whether arguments must solve to singleton types.
    pub(in crate::check) is_comptime: bool,
    /// The reason this parameter was induced.
    pub(in crate::check) induction: dir::GenericParameterInduction,
}

/// Inference bookkeeping over working generic segments.
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

    /// Record one induction site.
    pub(in crate::check) fn record_induction_site(&mut self, site: GenericInductionSite) {
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

    /// Return the induced generic parameter recipe for one variable.
    pub(in crate::check) fn induction(
        &self,
        variable: dir::TypeVariableId,
    ) -> Option<GenericInductionParameter> {
        self.inductions.get(&variable).copied()
    }

    /// Remove one variable's induction recipe.
    pub(in crate::check) fn remove_induction(
        &mut self,
        variable: dir::TypeVariableId,
    ) -> Option<GenericInductionParameter> {
        self.inductions.shift_remove(&variable)
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
            && let Some(template) = module.working.generics.get_local_template(id.local_id)
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
            && let Some(parameter) = module.working.generics.get_local_parameter(id.local_id)
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

    /// Return the generic template declared at one source node, declaring it once.
    pub(in crate::check) fn declare_generic_template(
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
            .working
            .generics
            .push_template(dir::GenericTemplate::new(source, symbol, parent));
        let id = local.into_global(module);

        self.generics.templates_by_source.insert(source, id);
        if let Some(symbol) = symbol {
            self.generics.templates_by_symbol.insert(symbol, id);
        }

        Ok(id)
    }

    /// Declare one generic parameter on its template.
    pub(in crate::check) fn declare_generic_parameter(
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
        let local = working.working.generics.push_template_parameter(binding);
        let id = local.into_global(module);

        if let Some(symbol) = symbol {
            self.generics.parameters_by_symbol.insert(symbol, id);
        }

        Ok(id)
    }

    /// Declare one induced generic parameter from its recipe.
    pub(in crate::check) fn declare_induced_generic_parameter(
        &mut self,
        template: GenericTemplateId,
        parameter: GenericInductionParameter,
    ) -> CompilerResult<GenericParameterId> {
        // generate the parameter name from its template position
        let number = self
            .generic_template(template)
            .map(|template| template.parameters.len())
            .unwrap_or(0); // (we actually do want to begin with 0 here)
        let name = self
            .module_mut(template.module_id)
            .strings
            .intern(&format!("{}{number}", parameter.prefix));

        let binding = dir::GenericParameterBinding {
            template: template.local_id,
            key: dir::GenericParameterKey::Generated(name),
            origin: dir::GenericParameterOrigin::Induced(parameter.induction),
            variance: None,
            constraint: parameter.constraint,
            default: None,
            is_variadic: false,
            is_comptime: parameter.is_comptime,
        };

        self.declare_generic_parameter(binding, template, None)
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
        let Some(binding) = working
            .working
            .generics
            .get_local_parameter_mut(parameter.local_id)
        else {
            return Err(CompilerError::Internal {
                message: format!("generic parameter {parameter:?} is not in its working segment"),
            });
        };
        binding.constraint = constraint;
        binding.default = default;

        Ok(())
    }

    /// Hypothesize fresh variables for one template's parameters.
    /// Returns the substitution from parameters to variable types.
    pub(in crate::check) fn instantiate_template(
        &mut self,
        origin: Origin,
        template: GenericTemplateId,
    ) -> CompilerResult<Substitution> {
        let parameters = self.generic_template_parameters(template);
        let source = self.origin_source_node(origin)?;
        let mut arguments = SmallVec::new();

        // allocate one hypothesis variable per parameter
        for parameter in parameters.iter().copied() {
            let variable = self.allocate_variable(origin.module(), origin, Widening::Preserve);
            let ty = self.push_variable_type(variable, source)?;

            // seed declared constraints as upper bounds
            let constraint = self
                .generic_parameter(parameter)
                .and_then(|binding| binding.constraint);
            if let Some(constraint) = constraint {
                self.push_upper_bound(variable, constraint)?;
            }

            arguments.push(ty);
        }

        Ok(Substitution {
            parameters,
            arguments,
            receiver: None,
        })
    }
}
