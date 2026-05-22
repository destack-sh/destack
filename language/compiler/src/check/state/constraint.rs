use destack_dir as dir;
use smallvec::SmallVec;

use super::{
    ArgumentTerm, FieldTerm, FormTerm, OperandTerm, OperatorTerm, Predicate, Term, VariableId,
};

/// A relation between two type variables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum TypeRelation {
    /// Types must be equal.
    Equal,
    /// Source must be assignable to target.
    Assignable,
    /// Value must satisfy a constraint.
    Satisfies,
    /// Subtype must extend supertype.
    Extends,
    /// Implementor must implement contract.
    Implements,
}

/// A relation between two static variables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum StaticRelation {
    /// Static values must be equal.
    Equal,
}

/// One check constraint.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct Constraint {
    /// The guard that controls this constraint.
    pub(in crate::check) guard: Predicate,
    /// The constraint payload.
    pub(in crate::check) kind: ConstraintKind,
}

impl Constraint {
    /// Create one bind constraint.
    pub(in crate::check) fn bind(guard: Predicate, variable: VariableId, term: Term) -> Self {
        Self {
            guard,
            kind: ConstraintKind::Bind { variable, term },
        }
    }

    /// Create one relation constraint.
    pub(in crate::check) fn relate(guard: Predicate, relation: Relation) -> Self {
        Self {
            guard,
            kind: ConstraintKind::Relate(relation),
        }
    }

    /// Create one construction constraint.
    pub(in crate::check) fn construct(guard: Predicate, construction: Construction) -> Self {
        Self {
            guard,
            kind: ConstraintKind::Construct(construction),
        }
    }

    /// Create one resolution constraint.
    pub(in crate::check) fn resolve(guard: Predicate, resolution: Resolution) -> Self {
        Self {
            guard,
            kind: ConstraintKind::Resolve(resolution),
        }
    }

    /// Return variables referenced by this constraint.
    pub(in crate::check) fn variables(&self) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();
        self.guard.variables(&mut variables);
        self.kind.variables(&mut variables);

        variables
    }
}

/// Check constraint payload.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum ConstraintKind {
    /// Bind a variable to a term.
    Bind {
        /// The variable to bind.
        variable: VariableId,
        /// The term being bound.
        term: Term,
    },
    /// Relate variables.
    Relate(Relation),
    /// Construct a derived value.
    Construct(Construction),
    /// Resolve a type-dependent selection.
    Resolve(Resolution),
}

impl ConstraintKind {
    /// Push variables referenced by this constraint payload.
    fn variables(&self, variables: &mut SmallVec<[VariableId; 4]>) {
        match self {
            Self::Bind { variable, term } => {
                variables.push(*variable);
                term_variables(term, variables);
            }
            Self::Relate(relation) => relation.variables(variables),
            Self::Construct(construction) => construction.variables(variables),
            Self::Resolve(resolution) => resolution.variables(variables),
        }
    }
}

/// Relation constraint.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Relation {
    /// Type relation.
    Type {
        /// The required relation.
        relation: TypeRelation,
        /// The left type.
        left: VariableId,
        /// The right type.
        right: VariableId,
    },
    /// Static relation.
    Static {
        /// The required relation.
        relation: StaticRelation,
        /// The left static value.
        left: VariableId,
        /// The right static value.
        right: VariableId,
    },
}

impl Relation {
    /// Push variables referenced by this relation.
    fn variables(&self, variables: &mut SmallVec<[VariableId; 4]>) {
        match self {
            Self::Type { left, right, .. } | Self::Static { left, right, .. } => {
                variables.push(*left);
                variables.push(*right);
            }
        }
    }
}

/// Construction constraint.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Construction {
    /// Construct a function type.
    Function {
        /// The function type result.
        result: VariableId,
        /// The parameter types.
        parameters: SmallVec<[VariableId; 4]>,
        /// The return type.
        return_type: Option<VariableId>,
    },
    /// Construct a union type.
    Union {
        /// The union result type.
        result: VariableId,
        /// The input types.
        values: SmallVec<[VariableId; 4]>,
    },
    /// Construct a tuple type.
    Tuple {
        /// The tuple result type.
        result: VariableId,
        /// The element types.
        elements: SmallVec<[VariableId; 4]>,
    },
    /// Construct an object type.
    Object {
        /// The object result type.
        result: VariableId,
        /// The direct fields.
        fields: SmallVec<[FieldTerm; 4]>,
        /// The spread source types.
        spreads: SmallVec<[VariableId; 2]>,
    },
    /// Construct a fixed array type.
    FixedArray {
        /// The fixed array result type.
        result: VariableId,
        /// The repeated value type.
        value: VariableId,
        /// The static array length.
        length: VariableId,
    },
    /// Construct a memory form type.
    Form {
        /// The form result type.
        result: VariableId,
        /// The form constructor.
        form: FormTerm,
        /// The carried value type.
        value: VariableId,
    },
    /// Construct a layout for a type.
    LayoutOf {
        /// The layout result.
        result: VariableId,
        /// The type being laid out.
        ty: VariableId,
    },
    /// Construct a static size from a layout.
    SizeOf {
        /// The static size result.
        result: VariableId,
        /// The source layout.
        layout: VariableId,
    },
    /// Construct a static alignment from a layout.
    AlignOf {
        /// The static alignment result.
        result: VariableId,
        /// The source layout.
        layout: VariableId,
    },
    /// Construct a static field offset from a layout.
    OffsetOf {
        /// The static offset result.
        result: VariableId,
        /// The source layout.
        layout: VariableId,
        /// The field key.
        field: VariableId,
    },
}

impl Construction {
    /// Push variables referenced by this construction.
    fn variables(&self, variables: &mut SmallVec<[VariableId; 4]>) {
        match self {
            Self::Function {
                result,
                parameters,
                return_type,
            } => {
                variables.push(*result);
                variables.extend(parameters.iter().copied());
                variables.extend(return_type.iter().copied());
            }
            Self::Union { result, values }
            | Self::Tuple {
                result,
                elements: values,
            } => {
                variables.push(*result);
                variables.extend(values.iter().copied());
            }
            Self::Object {
                result,
                fields,
                spreads,
            } => {
                variables.push(*result);
                variables.extend(fields.iter().map(|field| field.value));
                variables.extend(spreads.iter().copied());
            }
            Self::FixedArray {
                result,
                value,
                length,
            } => {
                variables.push(*result);
                variables.push(*value);
                variables.push(*length);
            }
            Self::Form {
                result,
                form,
                value,
            } => {
                variables.push(*result);
                variables.push(*value);
                form_variables(form, variables);
            }
            Self::LayoutOf { result, ty } => {
                variables.push(*result);
                variables.push(*ty);
            }
            Self::SizeOf { result, layout } | Self::AlignOf { result, layout } => {
                variables.push(*result);
                variables.push(*layout);
            }
            Self::OffsetOf {
                result,
                layout,
                field,
            } => {
                variables.push(*result);
                variables.push(*layout);
                variables.push(*field);
            }
        }
    }
}

/// Resolution constraint.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Resolution {
    /// Resolve a name.
    Name {
        /// The selected result.
        result: VariableId,
        /// The selected key.
        key: dir::StaticKey,
        /// The selected symbol space.
        space: dir::SymbolSpace,
    },
    /// Resolve a member.
    Member {
        /// The selected result.
        result: VariableId,
        /// The receiver type.
        receiver: VariableId,
        /// The member key.
        key: dir::StaticKey,
    },
    /// Resolve a call.
    Call {
        /// The call result.
        result: VariableId,
        /// The callee type.
        callee: VariableId,
        /// The generic arguments.
        generics: SmallVec<[ArgumentTerm; 4]>,
        /// The call arguments.
        arguments: SmallVec<[ArgumentTerm; 4]>,
    },
    /// Resolve a constructor call.
    New {
        /// The constructed result.
        result: VariableId,
        /// The callee type.
        callee: VariableId,
        /// The generic arguments.
        generics: SmallVec<[ArgumentTerm; 4]>,
        /// The call arguments.
        arguments: SmallVec<[ArgumentTerm; 4]>,
    },
    /// Resolve an index operation.
    Index {
        /// The index result.
        result: VariableId,
        /// The indexed receiver type.
        receiver: VariableId,
        /// The optional index type.
        index: Option<VariableId>,
    },
    /// Resolve an operator operation.
    Operator {
        /// The operator result.
        result: VariableId,
        /// The operator.
        operator: OperatorTerm,
        /// The operands.
        operands: SmallVec<[OperandTerm; 2]>,
    },
    /// Instantiate a generic target.
    Instantiate {
        /// The instantiated result.
        result: VariableId,
        /// The target type.
        target: VariableId,
        /// The arguments.
        arguments: SmallVec<[ArgumentTerm; 4]>,
    },
    /// Resolve an associated type.
    AssociatedType {
        /// The associated type result.
        result: VariableId,
        /// The owner type.
        owner: VariableId,
        /// The member key.
        key: dir::StaticKey,
        /// The arguments.
        arguments: SmallVec<[ArgumentTerm; 4]>,
    },
    /// Resolve an associated const.
    AssociatedConst {
        /// The associated const result.
        result: VariableId,
        /// The owner type.
        owner: VariableId,
        /// The member key.
        key: dir::StaticKey,
        /// The arguments.
        arguments: SmallVec<[ArgumentTerm; 4]>,
    },
}

impl Resolution {
    /// Push variables referenced by this resolution.
    fn variables(&self, variables: &mut SmallVec<[VariableId; 4]>) {
        match self {
            Self::Name { result, .. } => variables.push(*result),
            Self::Member {
                result, receiver, ..
            } => {
                variables.push(*result);
                variables.push(*receiver);
            }
            Self::Call {
                result,
                callee,
                generics,
                arguments,
            }
            | Self::New {
                result,
                callee,
                generics,
                arguments,
            } => {
                variables.push(*result);
                variables.push(*callee);
                variables.extend(generics.iter().map(ArgumentTerm::variable));
                variables.extend(arguments.iter().map(ArgumentTerm::variable));
            }
            Self::Index {
                result,
                receiver,
                index,
            } => {
                variables.push(*result);
                variables.push(*receiver);
                variables.extend(index.iter().copied());
            }
            Self::Operator {
                result, operands, ..
            } => {
                variables.push(*result);
                variables.extend(operands.iter().map(OperandTerm::variable));
            }
            Self::Instantiate {
                result,
                target,
                arguments,
            }
            | Self::AssociatedType {
                result,
                owner: target,
                arguments,
                ..
            }
            | Self::AssociatedConst {
                result,
                owner: target,
                arguments,
                ..
            } => {
                variables.push(*result);
                variables.push(*target);
                variables.extend(arguments.iter().map(ArgumentTerm::variable));
            }
        }
    }
}

/// Push variables referenced by a term.
fn term_variables(term: &Term, variables: &mut SmallVec<[VariableId; 4]>) {
    match term {
        Term::Type(term) => {
            if let super::TypeTerm::Variable(variable) = term {
                variables.push(*variable);
            }
        }
        Term::Static(term) => {
            if let super::StaticTerm::Variable(variable) = term {
                variables.push(*variable);
            }
        }
        Term::Layout(term) => {
            if let super::LayoutTerm::Variable(variable) = term {
                variables.push(*variable);
            }
        }
        Term::Resolution(_) => {}
    }
}

/// Push variables referenced by a form term.
fn form_variables(form: &FormTerm, variables: &mut SmallVec<[VariableId; 4]>) {
    match form {
        FormTerm::Borrowed { lifetime, access } => {
            variables.push(*lifetime);
            variables.push(*access);
        }
        FormTerm::Placed { place } => variables.push(*place),
        FormTerm::Managed | FormTerm::Owned | FormTerm::Raw | FormTerm::Readonly => {}
    }
}
