use destack_dir as dir;
use smallvec::SmallVec;

use super::VariableId;
use crate::check::{
    CheckState, GenericArgument, Origin, StaticOperand, TypeOperand, TypeTerm, VariableKind,
};

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

/// One declaration operand that can induce template generics.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct GenericInductionSource {
    /// The source node that receives induced generic parameters.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The enclosing generic template.
    pub(in crate::check) parent: Option<GenericTemplateId>,
    /// The declaration symbol indexed by the generated template.
    pub(in crate::check) symbol: Option<dir::GlobalSymbolId>,
    /// The declaration operand to traverse.
    pub(in crate::check) operand: TypeOperand,
}

impl GenericInductionSource {
    /// Create one symbol-backed induction source.
    pub(in crate::check) fn symbol(
        source: dir::GlobalNodeIdAny,
        parent: Option<GenericTemplateId>,
        symbol: dir::GlobalSymbolId,
        operand: TypeOperand,
    ) -> Self {
        Self {
            source,
            parent,
            symbol: Some(symbol),
            operand,
        }
    }
}

/// One escaping inference variable that may become an induced owner generic.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct GenericInduction {
    /// The variable rewritten to the generated parameter.
    pub(in crate::check) variable: VariableId,
    /// The generated parameter recipe.
    pub(in crate::check) parameter: GenericInductionParameter,
}

impl GenericInduction {
    /// Create one generic induction.
    pub(in crate::check) fn new(
        variable: VariableId,
        parameter: GenericInductionParameter,
    ) -> Self {
        Self {
            variable,
            parameter,
        }
    }
}

/// One generated generic parameter recipe.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct GenericInductionParameter {
    /// The induced variable kind.
    pub(in crate::check) kind: VariableKind,
    /// The generated parameter name prefix.
    pub(in crate::check) prefix: &'static str,
    /// The optional generated parameter constraint.
    pub(in crate::check) constraint: Option<TypeOperand>,
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
            constraint,
            induction,
        }
    }

    /// Create one induced static parameter.
    pub(in crate::check) fn r#static(
        prefix: &'static str,
        constraint: Option<TypeOperand>,
        induction: dir::GenericParameterInduction,
    ) -> Self {
        Self {
            kind: VariableKind::Static,
            prefix,
            constraint,
            induction,
        }
    }
}

/// Generic parameter shared by explicit and induced generic parameters.
#[derive(Debug, Clone, PartialEq)]
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

    /// Return a stable parameter id.
    pub(in crate::check) fn id(&self) -> GenericParameterId {
        self.id
    }
}

/// Generic parameter metadata shared by explicit and induced parameters.
#[derive(Debug, Clone, PartialEq)]
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
        /// The optional static value type constraint.
        constraint: Option<TypeOperand>,
        /// The optional static default.
        default: Option<StaticOperand>,
    },
    /// Variadic static generic parameter.
    VariadicStatic {
        /// The generic parameter.
        parameter: GenericParameter,
        /// The optional static value type constraint.
        constraint: Option<TypeOperand>,
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
        constraint: Option<TypeOperand>,
        default: Option<StaticOperand>,
    ) -> Self {
        Self::Static {
            parameter,
            constraint,
            default,
        }
    }

    /// Create one variadic static generic parameter binding.
    pub(in crate::check) fn variadic_static(
        parameter: GenericParameter,
        constraint: Option<TypeOperand>,
        default: Option<StaticOperand>,
    ) -> Self {
        Self::VariadicStatic {
            parameter,
            constraint,
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

    /// Return this generic parameter's type constraint.
    pub(in crate::check) fn type_constraint(&self) -> Option<TypeOperand> {
        match self {
            Self::Type { constraint, .. } | Self::VariadicType { constraint, .. } => *constraint,
            Self::Static { .. } | Self::VariadicStatic { .. } => None,
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
    ) -> GenericTemplateId {
        if let Some(template) = self.inference.generic_template_by_source(source) {
            return template;
        }

        let id = self
            .module_mut(source.module_id)
            .allocate_generic_template_id();
        let template = GenericTemplate::new(id, source, parent);

        self.inference.insert_generic_template(template, symbol);

        id
    }

    /// Induce one static generic constrained by a language item type.
    pub(in crate::check) fn induce_language_static_generic(
        &mut self,
        variable: VariableId,
        prefix: &'static str,
        item: dir::LanguageItem,
    ) {
        let symbol = self.language_symbol(variable.module, item);
        let constraint = TypeTerm::Reference {
            origin: Origin::Symbol(symbol),
            symbol,
            arguments: Vec::new().into(),
        };
        let constraint = self.inference.push_term(constraint).into();

        let induction = GenericInduction::new(
            variable,
            GenericInductionParameter::r#static(
                prefix,
                Some(constraint),
                dir::GenericParameterInduction::Form,
            ),
        );

        self.inference.insert_generic_induction(induction);
    }

    /// Allocate one induced generic parameter for one template.
    pub(in crate::check) fn allocate_generic_induction_parameter(
        &mut self,
        template: GenericTemplateId,
        prefix: &str,
        induction: dir::GenericParameterInduction,
    ) -> GenericParameter {
        let id = self
            .module_mut(template.module_id)
            .allocate_generic_parameter_id();
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

    /// Allocate one symbol-keyed generic parameter for one template.
    pub(in crate::check) fn allocate_symbol_generic_parameter(
        &mut self,
        template: GenericTemplateId,
        symbol: dir::GlobalSymbolId,
        origin: dir::GenericParameterOrigin,
    ) -> GenericParameter {
        let id = self
            .module_mut(template.module_id)
            .allocate_generic_parameter_id();

        GenericParameter::new(
            id,
            template,
            dir::GenericParameterKey::Symbol(symbol),
            origin,
        )
    }
}
