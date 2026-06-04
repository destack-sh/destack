use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    CheckState, GenericArgument, Origin, StaticOperand, TypeOperand, TypeTerm, VariableKind,
};

use super::VariableId;

/// Stable id for one declaration-side generic parameter.
pub(in crate::check) type GenericParameterId = dir::GlobalGenericParameterId;

/// One generic template instance.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct GenericInstance {
    /// The applied generic owner.
    pub(in crate::check) owner: dir::GlobalSymbolId,
    /// The generic arguments in declaration order.
    pub(in crate::check) arguments: SmallVec<[GenericArgument; 2]>,
}

/// Stable key for one source-level generic instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct GenericInstanceKey {
    /// The syntax node that applies the generic owner.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The generic owner being applied.
    pub(in crate::check) owner: dir::GlobalSymbolId,
}

/// One owner-level declaration of generic parameters.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct GenericTemplate {
    /// The symbol that owns this generic template.
    pub(in crate::check) owner: dir::GlobalSymbolId,
    /// The generic parameters in declaration order.
    pub(in crate::check) parameters: Vec<GenericParameterId>,
}

impl GenericTemplate {
    /// Create an empty generic template.
    pub(in crate::check) fn new(owner: dir::GlobalSymbolId) -> Self {
        Self {
            owner,
            parameters: Vec::new(),
        }
    }
}

/// One declaration operand that can induce owner generics.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct GenericInductionRoot {
    /// The declaration that receives induced generic parameters.
    pub(in crate::check) owner: dir::GlobalSymbolId,
    /// The declaration operand to traverse.
    pub(in crate::check) operand: TypeOperand,
}

/// One escaping inference variable that may become an induced owner generic.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct GenericInduction {
    /// The variable rewritten to the generated parameter.
    pub(in crate::check) variable: VariableId,
    /// The generated parameter recipe.
    pub(in crate::check) parameter: GenericInductionParameter,
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

/// Generic parameter identity shared by explicit and induced generic parameters.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct GenericParameterIdentity {
    /// The stable generic parameter id.
    pub(in crate::check) id: GenericParameterId,
    /// The generic owner symbol.
    pub(in crate::check) owner: dir::GlobalSymbolId,
    /// The parameter key.
    pub(in crate::check) key: dir::GenericParameterKey,
    /// The parameter origin.
    pub(in crate::check) origin: dir::GenericParameterOrigin,
}

impl GenericParameterIdentity {
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
        /// The parameter identity.
        identity: GenericParameterIdentity,
        /// The parameter variance.
        variance: Option<dir::VarianceModifier>,
        /// The optional type constraint.
        constraint: Option<TypeOperand>,
        /// The optional type default.
        default: Option<TypeOperand>,
    },
    /// Variadic type generic parameter.
    VariadicType {
        /// The parameter identity.
        identity: GenericParameterIdentity,
        /// The parameter variance.
        variance: Option<dir::VarianceModifier>,
        /// The optional type constraint.
        constraint: Option<TypeOperand>,
        /// The optional type default.
        default: Option<TypeOperand>,
    },
    /// Static generic parameter.
    Static {
        /// The parameter identity.
        identity: GenericParameterIdentity,
        /// The optional static value type constraint.
        constraint: Option<TypeOperand>,
        /// The optional static default.
        default: Option<StaticOperand>,
    },
    /// Variadic static generic parameter.
    VariadicStatic {
        /// The parameter identity.
        identity: GenericParameterIdentity,
        /// The optional static value type constraint.
        constraint: Option<TypeOperand>,
        /// The optional static default.
        default: Option<StaticOperand>,
    },
}

impl GenericParameterBinding {
    /// Return this generic parameter's parameter identity.
    pub(in crate::check) fn identity(&self) -> &GenericParameterIdentity {
        match self {
            Self::Type { identity, .. }
            | Self::VariadicType { identity, .. }
            | Self::Static { identity, .. }
            | Self::VariadicStatic { identity, .. } => identity,
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

    /// Return whether this is a variadic generic parameter.
    pub(in crate::check) fn is_variadic(&self) -> bool {
        matches!(
            self,
            Self::VariadicType { .. } | Self::VariadicStatic { .. }
        )
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
    /// Push one declaration operand that can induce owner generics.
    pub(in crate::check) fn push_generic_induction_root(
        &mut self,
        owner: dir::GlobalSymbolId,
        operand: TypeOperand,
    ) {
        let root = GenericInductionRoot { owner, operand };

        self.inference.push_generic_induction_root(root);
    }

    /// Induce one static generic from an escaping variable.
    pub(in crate::check) fn induce_static_generic(
        &mut self,
        variable: VariableId,
        prefix: &'static str,
        constraint: Option<TypeOperand>,
        induction: dir::GenericParameterInduction,
    ) {
        let generic_induction = GenericInduction {
            variable,
            parameter: GenericInductionParameter::r#static(prefix, constraint, induction),
        };

        self.inference.insert_generic_induction(generic_induction);
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

        self.induce_static_generic(
            variable,
            prefix,
            Some(constraint),
            dir::GenericParameterInduction::Form,
        );
    }

    /// Induce one type generic from an escaping variable.
    pub(in crate::check) fn induce_type_generic(
        &mut self,
        variable: VariableId,
        prefix: &'static str,
        constraint: Option<TypeOperand>,
        induction: dir::GenericParameterInduction,
    ) {
        let generic_induction = GenericInduction {
            variable,
            parameter: GenericInductionParameter::r#type(prefix, constraint, induction),
        };

        self.inference.insert_generic_induction(generic_induction);
    }

    /// Insert one generic instance for a source node.
    pub(in crate::check) fn insert_generic_instance(
        &mut self,
        source: dir::GlobalNodeIdAny,
        owner: dir::GlobalSymbolId,
        arguments: SmallVec<[GenericArgument; 2]>,
    ) -> GenericInstance {
        let key = GenericInstanceKey { source, owner };

        self.inference.insert_generic_instance(key, arguments)
    }

    /// Add one type constraint to an existing generic type parameter.
    pub(in crate::check) fn constrain_generic_type_parameter(
        &mut self,
        parameter_id: GenericParameterId,
        constraint: TypeOperand,
    ) {
        let current = self
            .inference
            .generic_parameter(parameter_id)
            .type_constraint();
        let constraint = match current {
            Some(current) => TypeOperand::Term(self.inference.push_term(TypeTerm::Intersection {
                elements: vec![current, constraint],
            })),
            None => constraint,
        };
        let generic = self.inference.generic_parameter_by_id_mut(parameter_id);

        match generic {
            GenericParameterBinding::Type {
                constraint: current,
                ..
            }
            | GenericParameterBinding::VariadicType {
                constraint: current,
                ..
            } => *current = Some(constraint),
            GenericParameterBinding::Static { .. }
            | GenericParameterBinding::VariadicStatic { .. } => {
                unreachable!(
                    "internal invariant: generic parameter {parameter_id:?} is not a type parameter"
                )
            }
        }
    }

    /// Insert one generic parameter.
    pub(in crate::check) fn insert_generic_parameter(&mut self, generic: GenericParameterBinding) {
        self.inference.insert_generic_parameter(generic);
    }

    /// Allocate one explicit generic parameter for one owner.
    pub(in crate::check) fn allocate_explicit_generic_parameter(
        &mut self,
        owner: dir::GlobalSymbolId,
        symbol: dir::GlobalSymbolId,
    ) -> GenericParameterIdentity {
        self.allocate_symbol_generic_parameter(owner, symbol, dir::GenericParameterOrigin::Explicit)
    }

    /// Allocate one induced generic parameter for one source symbol.
    pub(in crate::check) fn allocate_induced_symbol_generic_parameter(
        &mut self,
        owner: dir::GlobalSymbolId,
        symbol: dir::GlobalSymbolId,
    ) -> GenericParameterIdentity {
        self.allocate_symbol_generic_parameter(
            owner,
            symbol,
            dir::GenericParameterOrigin::Induced(dir::GenericParameterInduction::Comptime),
        )
    }

    /// Allocate one induced generic parameter for one owner.
    pub(in crate::check) fn allocate_generic_induction_parameter(
        &mut self,
        owner: dir::GlobalSymbolId,
        prefix: &str,
        induction: dir::GenericParameterInduction,
    ) -> GenericParameterIdentity {
        let id = self.allocate_generic_parameter_id(owner);
        let number = self.next_generic_parameter_number(owner);
        let name = self.generated_generic_name(owner, prefix, number);

        GenericParameterIdentity {
            id,
            owner,
            key: dir::GenericParameterKey::Generated(name),
            origin: dir::GenericParameterOrigin::Induced(induction),
        }
    }

    /// Return the generated local name for one generic parameter.
    fn generated_generic_name(
        &mut self,
        owner: dir::GlobalSymbolId,
        prefix: &str,
        number: u32,
    ) -> dir::StringId {
        self.module_mut(owner.module_id)
            .strings
            .intern(&format!("{prefix}{number}"))
    }

    /// Allocate the next generic parameter id for one module.
    fn allocate_generic_parameter_id(&mut self, owner: dir::GlobalSymbolId) -> GenericParameterId {
        self.module_mut(owner.module_id)
            .allocate_generic_parameter_id()
    }

    /// Return the next generic parameter number for one owner.
    fn next_generic_parameter_number(&mut self, owner: dir::GlobalSymbolId) -> u32 {
        self.inference.next_generic_parameter_number(owner)
    }

    /// Allocate one source-symbol generic parameter for one owner.
    fn allocate_symbol_generic_parameter(
        &mut self,
        owner: dir::GlobalSymbolId,
        symbol: dir::GlobalSymbolId,
        origin: dir::GenericParameterOrigin,
    ) -> GenericParameterIdentity {
        GenericParameterIdentity {
            id: self.allocate_generic_parameter_id(owner),
            owner,
            key: dir::GenericParameterKey::Symbol(symbol),
            origin,
        }
    }
}
