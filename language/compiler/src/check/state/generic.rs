use destack_dir as dir;
use smallvec::SmallVec;

use super::InferenceTable;
use crate::check::{
    CheckState, GenericArgument, Origin, StaticOperand, TypeOperand, TypeTerm, VariableId,
    VariableKind,
};
use crate::{CompilerError, CompilerResult};

/// Stable id for one declaration-side generic parameter.
pub(in crate::check) type GenericParameterId = dir::GlobalGenericParameterId;

/// Stable id for one generic binding site.
pub(in crate::check) type GenericTemplateId = dir::GlobalGenericTemplateId;

/// One generic template instance.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct GenericInstance {
    /// The applied generic template.
    pub(in crate::check) template: GenericTemplateId,
    /// The generic arguments in declaration order.
    pub(in crate::check) arguments: SmallVec<[GenericArgument; 2]>,
}

impl GenericInstance {
    /// Create one generic template instance.
    pub(in crate::check) fn new(
        template: GenericTemplateId,
        arguments: SmallVec<[GenericArgument; 2]>,
    ) -> Self {
        Self {
            template,
            arguments,
        }
    }
}

/// Stable key for one source-level generic instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct GenericInstanceKey {
    /// The syntax node that applies the generic template.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The generic template being applied.
    pub(in crate::check) template: GenericTemplateId,
}

impl GenericInstanceKey {
    /// Create one generic instance key.
    pub(in crate::check) fn new(source: dir::GlobalNodeIdAny, template: GenericTemplateId) -> Self {
        Self { source, template }
    }
}

/// Stable key for one generic argument variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct GenericArgumentKey {
    /// The syntax node that applies the generic template.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The generic parameter.
    pub(in crate::check) parameter: GenericParameterId,
}

impl GenericArgumentKey {
    /// Create one generic argument key.
    pub(in crate::check) fn new(
        source: dir::GlobalNodeIdAny,
        parameter: GenericParameterId,
    ) -> Self {
        Self { source, parameter }
    }
}

/// One declaration of generic parameters.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct GenericTemplate {
    /// The stable generic template id.
    pub(in crate::check) id: GenericTemplateId,
    /// The source node that declares this template.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The immediately enclosing generic template.
    pub(in crate::check) parent: Option<GenericTemplateId>,
    /// The generic parameters in declaration order.
    pub(in crate::check) parameters: Vec<GenericParameterId>,
}

impl GenericTemplate {
    /// Create an empty generic template.
    pub(in crate::check) fn new(
        id: GenericTemplateId,
        source: dir::GlobalNodeIdAny,
        parent: Option<GenericTemplateId>,
    ) -> Self {
        Self {
            id,
            source,
            parent,
            parameters: Vec::new(),
        }
    }
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
    /// The declaration operand to traverse.
    pub(in crate::check) operand: TypeOperand,
}

impl GenericInductionSite {
    /// Create one induction site.
    pub(in crate::check) fn new(
        declaration: dir::GlobalNodeIdAny,
        parent: Option<GenericTemplateId>,
        symbol: Option<dir::GlobalSymbolId>,
        operand: TypeOperand,
    ) -> Self {
        Self {
            declaration,
            parent,
            symbol,
            operand,
        }
    }
}

/// One generated generic parameter recipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct GenericInductionParameter {
    /// The induced variable kind.
    pub(in crate::check) kind: VariableKind,
    /// The generated parameter name prefix.
    pub(in crate::check) prefix: &'static str,
    /// The optional generated parameter type.
    pub(in crate::check) ty: Option<TypeOperand>,
    /// The reason this parameter was induced.
    pub(in crate::check) induction: dir::GenericParameterInduction,
}

impl GenericInductionParameter {
    /// Create one induced type parameter.
    pub(in crate::check) fn r#type(
        prefix: &'static str,
        constraint: Option<TypeOperand>,
        induction: dir::GenericParameterInduction,
    ) -> Self {
        Self {
            kind: VariableKind::Type,
            prefix,
            ty: constraint,
            induction,
        }
    }

    /// Create one induced static parameter.
    pub(in crate::check) fn r#static(
        prefix: &'static str,
        ty: Option<TypeOperand>,
        induction: dir::GenericParameterInduction,
    ) -> Self {
        Self {
            kind: VariableKind::Static,
            prefix,
            ty,
            induction,
        }
    }
}

/// Generic parameter shared by explicit and induced generic parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct GenericParameter {
    /// The stable generic parameter id.
    pub(in crate::check) id: GenericParameterId,
    /// The generic template that declares this parameter.
    pub(in crate::check) template: GenericTemplateId,
    /// The parameter key.
    pub(in crate::check) key: dir::GenericParameterKey,
    /// The parameter origin.
    pub(in crate::check) origin: dir::GenericParameterOrigin,
}

impl GenericParameter {
    /// Create one generic parameter.
    pub(in crate::check) fn new(
        id: GenericParameterId,
        template: GenericTemplateId,
        key: dir::GenericParameterKey,
        origin: dir::GenericParameterOrigin,
    ) -> Self {
        Self {
            id,
            template,
            key,
            origin,
        }
    }
}

/// Generic parameter metadata shared by explicit and induced parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) enum GenericParameterBinding {
    /// Type generic parameter.
    Type {
        /// The generic parameter.
        parameter: GenericParameter,
        /// The parameter variance.
        variance: Option<dir::VarianceModifier>,
        /// The optional type constraint.
        constraint: Option<TypeOperand>,
        /// The optional type default.
        default: Option<TypeOperand>,
    },
    /// Variadic type generic parameter.
    VariadicType {
        /// The generic parameter.
        parameter: GenericParameter,
        /// The parameter variance.
        variance: Option<dir::VarianceModifier>,
        /// The optional type constraint.
        constraint: Option<TypeOperand>,
        /// The optional type default.
        default: Option<TypeOperand>,
    },
    /// Static generic parameter.
    Static {
        /// The generic parameter.
        parameter: GenericParameter,
        /// The optional static value type.
        ty: Option<TypeOperand>,
        /// The optional static default.
        default: Option<StaticOperand>,
    },
    /// Variadic static generic parameter.
    VariadicStatic {
        /// The generic parameter.
        parameter: GenericParameter,
        /// The optional static value type.
        ty: Option<TypeOperand>,
        /// The optional static default.
        default: Option<StaticOperand>,
    },
}

impl GenericParameterBinding {
    /// Create one type generic parameter binding.
    pub(in crate::check) fn r#type(
        parameter: GenericParameter,
        variance: Option<dir::VarianceModifier>,
        constraint: Option<TypeOperand>,
        default: Option<TypeOperand>,
    ) -> Self {
        Self::Type {
            parameter,
            variance,
            constraint,
            default,
        }
    }

    /// Create one variadic type generic parameter binding.
    pub(in crate::check) fn variadic_type(
        parameter: GenericParameter,
        variance: Option<dir::VarianceModifier>,
        constraint: Option<TypeOperand>,
        default: Option<TypeOperand>,
    ) -> Self {
        Self::VariadicType {
            parameter,
            variance,
            constraint,
            default,
        }
    }

    /// Create one static generic parameter binding.
    pub(in crate::check) fn r#static(
        parameter: GenericParameter,
        ty: Option<TypeOperand>,
        default: Option<StaticOperand>,
    ) -> Self {
        Self::Static {
            parameter,
            ty,
            default,
        }
    }

    /// Create one variadic static generic parameter binding.
    pub(in crate::check) fn variadic_static(
        parameter: GenericParameter,
        ty: Option<TypeOperand>,
        default: Option<StaticOperand>,
    ) -> Self {
        Self::VariadicStatic {
            parameter,
            ty,
            default,
        }
    }

    /// Return this generic parameter.
    pub(in crate::check) fn parameter(&self) -> &GenericParameter {
        match self {
            Self::Type { parameter, .. }
            | Self::VariadicType { parameter, .. }
            | Self::Static { parameter, .. }
            | Self::VariadicStatic { parameter, .. } => parameter,
        }
    }

    /// Return whether this is a type-level generic parameter.
    pub(in crate::check) fn is_type(&self) -> bool {
        matches!(self, Self::Type { .. } | Self::VariadicType { .. })
    }

    /// Return whether this is a static generic parameter.
    pub(in crate::check) fn is_static(&self) -> bool {
        matches!(self, Self::Static { .. } | Self::VariadicStatic { .. })
    }

    /// Return the value space of this generic parameter.
    pub(in crate::check) fn kind(&self) -> VariableKind {
        match self {
            Self::Type { .. } | Self::VariadicType { .. } => VariableKind::Type,
            Self::Static { .. } | Self::VariadicStatic { .. } => VariableKind::Static,
        }
    }

    /// Return whether this generic parameter accepts multiple arguments.
    pub(in crate::check) fn is_variadic(&self) -> bool {
        matches!(
            self,
            Self::VariadicType { .. } | Self::VariadicStatic { .. }
        )
    }

    /// Return this generic parameter's variance annotation.
    pub(in crate::check) fn variance(&self) -> Option<dir::VarianceModifier> {
        match self {
            Self::Type { variance, .. } | Self::VariadicType { variance, .. } => *variance,
            Self::Static { .. } | Self::VariadicStatic { .. } => None,
        }
    }

    /// Return this generic parameter's type constraint.
    pub(in crate::check) fn type_constraint(&self) -> Option<TypeOperand> {
        match self {
            Self::Type { constraint, .. } | Self::VariadicType { constraint, .. } => *constraint,
            Self::Static { .. } | Self::VariadicStatic { .. } => None,
        }
    }

    /// Return this generic parameter's type default.
    pub(in crate::check) fn type_default(&self) -> Option<TypeOperand> {
        match self {
            Self::Type { default, .. } | Self::VariadicType { default, .. } => *default,
            Self::Static { .. } | Self::VariadicStatic { .. } => None,
        }
    }

    /// Return this static generic parameter's value type.
    pub(in crate::check) fn static_ty(&self) -> Option<TypeOperand> {
        match self {
            Self::Static { ty, .. } | Self::VariadicStatic { ty, .. } => *ty,
            Self::Type { .. } | Self::VariadicType { .. } => None,
        }
    }

    /// Return this generic parameter's static default.
    pub(in crate::check) fn static_default(&self) -> Option<StaticOperand> {
        match self {
            Self::Static { default, .. } | Self::VariadicStatic { default, .. } => *default,
            Self::Type { .. } | Self::VariadicType { .. } => None,
        }
    }
}

impl CheckState<'_> {
    /// Return the generic template declared at one source node.
    pub(in crate::check) fn declare_generic_template(
        &mut self,
        source: dir::GlobalNodeIdAny,
        parent: Option<GenericTemplateId>,
        symbol: Option<dir::GlobalSymbolId>,
    ) -> CompilerResult<GenericTemplateId> {
        if let Some(template) = self.inference.generic_template_by_source(source) {
            return Ok(template);
        }

        let id = self
            .module_mut(source.module_id)
            .fresh_generic_template_id();
        let template = GenericTemplate::new(id, source, parent);

        self.inference.insert_generic_template(template, symbol)?;

        Ok(id)
    }

    /// Induce one static generic constrained by a language item type.
    pub(in crate::check) fn induce_language_static_generic(
        &mut self,
        variable: VariableId,
        prefix: &'static str,
        item: dir::LanguageItem,
    ) -> CompilerResult<()> {
        let symbol = self.language_symbol(item);
        let ty = TypeTerm::Reference {
            origin: Origin::Symbol(symbol),
            symbol,
            arguments: Vec::new().into(),
        };
        let ty = self.inference.push_term(ty).into();

        let induction = GenericInductionParameter::r#static(
            prefix,
            Some(ty),
            dir::GenericParameterInduction::Form,
        );

        self.inference.insert_generic_induction(variable, induction)
    }

    /// Return one fresh induced generic parameter for one template.
    pub(in crate::check) fn fresh_induced_generic_parameter(
        &mut self,
        template: GenericTemplateId,
        prefix: &str,
        induction: dir::GenericParameterInduction,
    ) -> GenericParameter {
        let id = self
            .module_mut(template.module_id)
            .fresh_generic_parameter_id();
        let number = self.inference.next_generic_parameter_number(template);
        let name = self
            .module_mut(template.module_id)
            .strings
            .intern(&format!("{prefix}{number}"));

        GenericParameter::new(
            id,
            template,
            dir::GenericParameterKey::Generated(name),
            dir::GenericParameterOrigin::Induced(induction),
        )
    }

    /// Return one fresh symbol-keyed generic parameter for one template.
    pub(in crate::check) fn fresh_symbol_generic_parameter(
        &mut self,
        template: GenericTemplateId,
        symbol: dir::GlobalSymbolId,
        origin: dir::GenericParameterOrigin,
    ) -> GenericParameter {
        let id = self
            .module_mut(template.module_id)
            .fresh_generic_parameter_id();

        GenericParameter::new(
            id,
            template,
            dir::GenericParameterKey::Symbol(symbol),
            origin,
        )
    }
}

impl InferenceTable {
    /// Record one induction site into the current segment.
    pub(in crate::check) fn record_generic_induction_site(&mut self, site: GenericInductionSite) {
        self.current_mut().generic_induction_sites.push(site);
    }

    /// Iterate generic induction sites in component order.
    pub(in crate::check) fn generic_induction_sites(
        &self,
    ) -> impl Iterator<Item = &GenericInductionSite> {
        self.segments
            .iter()
            .flat_map(|segment| segment.generic_induction_sites.iter())
    }

    /// Mark one variable as able to induce a template generic.
    pub(in crate::check) fn insert_generic_induction(
        &mut self,
        variable: VariableId,
        parameter: GenericInductionParameter,
    ) -> CompilerResult<()> {
        let previous = self
            .current_mut()
            .generic_inductions
            .insert(variable, parameter);

        if previous.is_some() {
            return Err(CompilerError::Internal {
                message: format!("variable {variable:?} has conflicting generic induction"),
            });
        }

        Ok(())
    }

    /// Return the active induced generic parameter for one variable.
    pub(in crate::check) fn generic_induction_parameter(
        &self,
        variable: VariableId,
    ) -> Option<GenericInductionParameter> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.generic_inductions.get(&variable).copied())
    }

    /// Return generic parameters in template order.
    pub(in crate::check) fn generic_parameters(
        &self,
    ) -> impl Iterator<Item = (GenericParameterId, &GenericParameterBinding)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.generic_templates.values())
            .flat_map(|template| template.parameters.iter())
            .filter_map(|parameter_id| {
                self.generic_parameter(*parameter_id)
                    .map(|parameter| (*parameter_id, parameter))
            })
    }

    /// Return generic parameters declared by one template.
    pub(in crate::check) fn generic_template_parameters(
        &self,
        template: GenericTemplateId,
    ) -> impl Iterator<Item = (GenericParameterId, &GenericParameterBinding)> + '_ {
        self.segments
            .iter()
            .flat_map(move |segment| segment.generic_templates.get(&template))
            .flat_map(|template| template.parameters.iter().copied())
            .filter_map(|parameter_id| {
                self.generic_parameter(parameter_id)
                    .map(|parameter| (parameter_id, parameter))
            })
    }

    /// Return one active generic template.
    pub(in crate::check) fn generic_template(
        &self,
        template: GenericTemplateId,
    ) -> Option<&GenericTemplate> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.generic_templates.get(&template))
    }

    /// Iterate generic templates in allocation order.
    pub(in crate::check) fn generic_templates(
        &self,
    ) -> impl Iterator<Item = (GenericTemplateId, &GenericTemplate)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.generic_templates.iter())
            .map(|(template, value)| (*template, value))
    }

    /// Return one active generic template id by source node.
    pub(in crate::check) fn generic_template_by_source(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<GenericTemplateId> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.generic_templates_by_source.get(&source).copied())
    }

    /// Return one active generic template id by declaration symbol.
    pub(in crate::check) fn generic_template_by_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<GenericTemplateId> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.generic_templates_by_symbol.get(&symbol).copied())
    }

    /// Upsert one declaration symbol to generic template mapping.
    pub(in crate::check) fn upsert_generic_template_symbol(
        &mut self,
        template: GenericTemplateId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        if self.generic_template(template).is_none() {
            return Err(CompilerError::Internal {
                message: format!("generic template {template:?} is not allocated"),
            });
        }
        if let Some(existing) = self.generic_template_by_symbol(symbol) {
            if existing != template {
                return Err(CompilerError::Internal {
                    message: format!("generic template symbol {symbol:?} points at two templates"),
                });
            }

            return Ok(());
        }

        self.current_mut()
            .generic_templates_by_symbol
            .insert(symbol, template);

        Ok(())
    }

    /// Return one active generic parameter.
    pub(in crate::check) fn generic_parameter(
        &self,
        parameter_id: GenericParameterId,
    ) -> Option<&GenericParameterBinding> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.generic_parameters_by_id.get(&parameter_id))
    }

    /// Return one generic parameter or report an internal table error.
    pub(in crate::check) fn generic_parameter_binding(
        &self,
        parameter_id: GenericParameterId,
    ) -> CompilerResult<&GenericParameterBinding> {
        self.generic_parameter(parameter_id)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("generic parameter {parameter_id:?} does not exist"),
            })
    }

    /// Return one active generic instance.
    pub(in crate::check) fn generic_instance(
        &self,
        key: GenericInstanceKey,
    ) -> Option<&GenericInstance> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.generic_instances.get(&key))
    }

    /// Iterate active source-level generic instances.
    pub(in crate::check) fn generic_instances(
        &self,
    ) -> impl Iterator<Item = (GenericInstanceKey, &GenericInstance)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.generic_instances.iter())
            .map(|(key, instance)| (*key, instance))
    }

    /// Return one active generic argument variable.
    pub(in crate::check) fn generic_argument(&self, key: GenericArgumentKey) -> Option<VariableId> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.generic_arguments.get(&key).copied())
    }

    /// Upsert one generic argument variable into the current segment.
    pub(in crate::check) fn upsert_generic_argument(
        &mut self,
        key: GenericArgumentKey,
        variable: VariableId,
    ) -> VariableId {
        if let Some(existing) = self.generic_argument(key) {
            return existing;
        }

        self.current_mut().generic_arguments.insert(key, variable);

        variable
    }

    /// Insert one generic parameter into the current segment.
    pub(in crate::check) fn insert_generic_parameter(
        &mut self,
        generic: GenericParameterBinding,
    ) -> CompilerResult<()> {
        let template = generic.parameter().template;
        let key = generic.parameter().key;
        let parameter_id = generic.parameter().id;
        if self.generic_parameter(parameter_id).is_some() {
            return Err(CompilerError::Internal {
                message: format!("generic parameter {parameter_id:?} is already inserted"),
            });
        }

        let segment = self.current_mut();
        segment
            .generic_parameters_by_id
            .insert(parameter_id, generic);
        let Some(template) = segment.generic_templates.get_mut(&template) else {
            return Err(CompilerError::Internal {
                message: format!("generic template {template:?} is not allocated"),
            });
        };
        template.parameters.push(parameter_id);
        if let dir::GenericParameterKey::Symbol(symbol) = key {
            segment
                .generic_parameters_by_symbol
                .insert(symbol, parameter_id);
        }

        Ok(())
    }

    /// Update one inserted generic parameter.
    pub(in crate::check) fn update_generic_parameter(
        &mut self,
        generic: GenericParameterBinding,
    ) -> CompilerResult<()> {
        let parameter_id = generic.parameter().id;

        for segment in self.segments.iter_mut().rev() {
            if let Some(current) = segment.generic_parameters_by_id.get_mut(&parameter_id) {
                *current = generic;

                return Ok(());
            }
        }

        Err(CompilerError::Internal {
            message: format!("generic parameter {parameter_id:?} is not inserted"),
        })
    }

    /// Insert one generic template into the current segment.
    pub(in crate::check) fn insert_generic_template(
        &mut self,
        template: GenericTemplate,
        symbol: Option<dir::GlobalSymbolId>,
    ) -> CompilerResult<()> {
        if self.generic_template(template.id).is_some() {
            return Err(CompilerError::Internal {
                message: format!("generic template {:?} is already inserted", template.id),
            });
        }
        if self.generic_template_by_source(template.source).is_some() {
            return Err(CompilerError::Internal {
                message: format!(
                    "generic template source {:?} is already inserted",
                    template.source
                ),
            });
        }

        let segment = self.current_mut();
        if let Some(symbol) = symbol {
            let previous = segment
                .generic_templates_by_symbol
                .insert(symbol, template.id);

            if previous.is_some() {
                return Err(CompilerError::Internal {
                    message: format!("generic template symbol {symbol:?} is already inserted"),
                });
            }
        }
        segment
            .generic_templates_by_source
            .insert(template.source, template.id);
        segment.generic_templates.insert(template.id, template);

        Ok(())
    }

    /// Return one symbol's active generic parameter id.
    pub(in crate::check) fn generic_parameter_by_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<GenericParameterId> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.generic_parameters_by_symbol.get(&symbol).copied())
    }

    /// Return one active generic parameter mutably.
    fn generic_parameter_mut(
        &mut self,
        parameter_id: GenericParameterId,
    ) -> CompilerResult<&mut GenericParameterBinding> {
        self.segments
            .iter_mut()
            .rev()
            .find_map(|segment| segment.generic_parameters_by_id.get_mut(&parameter_id))
            .ok_or_else(|| CompilerError::Internal {
                message: format!("generic parameter {parameter_id:?} does not exist"),
            })
    }

    /// Add one type constraint to an existing type generic parameter.
    pub(in crate::check) fn constrain_generic_type_parameter(
        &mut self,
        parameter_id: GenericParameterId,
        constraint: TypeOperand,
    ) -> CompilerResult<()> {
        let current = self
            .generic_parameter_binding(parameter_id)?
            .type_constraint();
        let constraint = match current {
            Some(current) => TypeOperand::Term(self.push_term(TypeTerm::Intersection {
                elements: vec![current, constraint],
            })),
            None => constraint,
        };
        let generic = self.generic_parameter_mut(parameter_id)?;

        match generic {
            GenericParameterBinding::Type {
                constraint: current,
                ..
            }
            | GenericParameterBinding::VariadicType {
                constraint: current,
                ..
            } => {
                *current = Some(constraint);

                Ok(())
            }
            GenericParameterBinding::Static { .. }
            | GenericParameterBinding::VariadicStatic { .. } => Err(CompilerError::Internal {
                message: format!("generic parameter {parameter_id:?} is not a type parameter"),
            }),
        }
    }

    /// Return the next generic parameter number for one template.
    pub(in crate::check) fn next_generic_parameter_number(
        &self,
        template: GenericTemplateId,
    ) -> u32 {
        self.segments
            .iter()
            .filter_map(|segment| segment.generic_templates.get(&template))
            .map(|template| template.parameters.len())
            .sum::<usize>() as u32
    }
}
