use destack_dir as dir;
use smallvec::SmallVec;

use super::VariableId;

/// Symbol-backed callable candidate.
///
/// ```ts
/// value.toString()
/// ```
///
/// The selected `toString` symbol can become a candidate while solving the call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct CallCandidateTerm {
    /// The selected callable target.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The candidate callable type.
    pub(in crate::check) ty: VariableId,
}

/// Runtime call expression term.
///
/// ```ts
/// format(value)
/// ```
///
/// The term records the callee type and argument expression types.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct CallTerm {
    /// The source call expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The called expression type.
    pub(in crate::check) callee: VariableId,
    /// The member projection that produced this callee.
    pub(in crate::check) member: Option<MemberCallTerm>,
    /// The symbol-backed candidates visible at the call site.
    pub(in crate::check) candidates: Vec<CallCandidateTerm>,
    /// The argument expression types.
    pub(in crate::check) arguments: Vec<VariableId>,
}

/// Member projection used as a runtime call callee.
///
/// ```ts
/// value.toString()
/// ```
///
/// The term records `value` as receiver and `toString` as the member name.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct MemberCallTerm {
    /// The receiver type.
    pub(in crate::check) receiver: VariableId,
    /// The selected member name.
    pub(in crate::check) name: dir::StringId,
    /// The applied static arguments.
    pub(in crate::check) arguments: Vec<ArgumentTerm>,
}

/// Runtime binary operator expression term.
///
/// ```ts
/// left + right
/// ```
///
/// The term records both operand types and the `+` operator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct BinaryTerm {
    /// The left operand type.
    pub(in crate::check) left: VariableId,
    /// The source binary operator.
    pub(in crate::check) operator: dir::BinaryOperator,
    /// The right operand type.
    pub(in crate::check) right: VariableId,
}

/// Function type term.
///
/// ```ts
/// (value: string) => int32
/// ```
///
/// The term records one parameter type and one return type.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct FunctionTerm {
    /// The function asynchrony.
    pub(in crate::check) asynchrony: dir::Asynchrony,
    /// The generic parameter types.
    pub(in crate::check) generic_parameters: Vec<VariableId>,
    /// The optional `this` parameter type.
    pub(in crate::check) this_parameter: Option<VariableId>,
    /// The parameter types.
    pub(in crate::check) parameters: Vec<VariableId>,
    /// The optional return type.
    pub(in crate::check) return_type: Option<VariableId>,
    /// Whether this is a generator function.
    pub(in crate::check) is_generator: bool,
}

/// Tuple element term.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct TupleElementTerm {
    /// The optional label for the element.
    pub(in crate::check) label: Option<dir::StringId>,
    /// The element type.
    pub(in crate::check) ty: VariableId,
    /// Whether the element is optional.
    pub(in crate::check) is_optional: bool,
    /// Whether the element is readonly.
    pub(in crate::check) is_readonly: bool,
    /// Whether the element is a rest element.
    pub(in crate::check) is_rest: bool,
}

/// Mapped type parameter term.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct MappedParameterTerm {
    /// The parameter name.
    pub(in crate::check) name: dir::StringId,
    /// The parameter symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The source type iterated by `in`.
    pub(in crate::check) constraint: VariableId,
    /// The optional key remap.
    pub(in crate::check) key_remap: Option<VariableId>,
}

/// Shape member term.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum ShapeMemberTerm {
    /// Shape field.
    Field {
        /// The field key.
        key: dir::StaticKey,
        /// The field type.
        ty: VariableId,
        /// Whether the field is optional.
        is_optional: bool,
        /// Whether the field is readonly.
        is_readonly: bool,
    },
    /// Call signature.
    CallSignature {
        /// The signature type.
        ty: VariableId,
    },
    /// Construct signature.
    ConstructSignature {
        /// The signature type.
        ty: VariableId,
    },
    /// Index signature.
    IndexSignature {
        /// The parameter name.
        name: dir::StringId,
        /// The key type.
        key_type: VariableId,
        /// The value type.
        value_type: VariableId,
        /// Whether the index signature is optional.
        is_optional: bool,
        /// Whether the index signature is readonly.
        is_readonly: bool,
    },
}

/// Type-level operation term.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum TypeOperationTerm {
    /// Conditional type expression.
    ///
    /// ```ts
    /// T extends string ? A : B
    /// ```
    Conditional {
        /// The left operand.
        left: VariableId,
        /// The right operand.
        right: VariableId,
        /// The type selected when the condition holds.
        then_type: VariableId,
        /// The type selected when the condition does not hold.
        else_type: VariableId,
    },
    /// Indexed access type expression.
    ///
    /// ```ts
    /// T[K]
    /// ```
    Index {
        /// The indexed type.
        left: VariableId,
        /// The index type.
        index: VariableId,
    },
    /// Template literal type expression.
    ///
    /// ```ts
    /// `id:${T}`
    /// ```
    TemplateLiteral {
        /// The literal string segments.
        strings: Vec<dir::StringId>,
        /// The interpolated type spans.
        spans: Vec<VariableId>,
    },
    /// Type infer binding in a conditional type pattern.
    ///
    /// ```ts
    /// T extends Array<infer U> ? U : never
    /// ```
    Infer {
        /// The inferred binding name.
        name: Option<dir::StringId>,
        /// The optional inferred constraint.
        constraint: Option<VariableId>,
    },
    /// `keyof T`.
    KeyOf {
        /// The target type.
        target: VariableId,
    },
    /// Mapped type expression.
    ///
    /// ```ts
    /// { [K in keyof T]: T[K] }
    /// ```
    Mapped {
        /// The mapped parameter.
        parameter: MappedParameterTerm,
        /// The mapped modifiers.
        modifiers: dir::MappedTypeModifiers,
        /// The mapped value type.
        value: VariableId,
    },
    /// Type exclusion expression.
    ///
    /// ```ts
    /// Exclude<T, null>
    /// ```
    Exclude {
        /// The source type.
        source: VariableId,
        /// The excluded type.
        target: VariableId,
    },
    /// Compiler intrinsic returning a type.
    ///
    /// ```ts
    /// WithLifetime<T, L>
    /// ```
    Intrinsic {
        /// The intrinsic language item.
        item: dir::LanguageItem,
        /// The intrinsic arguments.
        arguments: SmallVec<[ArgumentTerm; 4]>,
    },
}

/// Term used to bind a type variable.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum TypeTerm {
    /// Concrete type.
    ///
    /// ```ts
    /// int32
    /// ```
    Type(dir::Type),
    /// Type variable.
    ///
    /// ```ts
    /// let y = x;
    /// ```
    ///
    /// The type of `y` can alias the type variable for `x`.
    Variable(VariableId),
    /// Canonical memory form over a value type.
    ///
    /// ```ts
    /// &T
    /// ```
    ///
    /// The syntax becomes a borrowed form over payload `T`.
    Form {
        /// The form constructor.
        form: FormTerm,
        /// The carried payload type.
        payload: VariableId,
    },
    /// Named type declaration reference.
    ///
    /// ```ts
    /// Map<K, V>
    /// ```
    Named {
        /// The declaration symbol.
        symbol: dir::GlobalSymbolId,
        /// The applied static arguments.
        arguments: Vec<ArgumentTerm>,
    },
    /// Homogeneous array type.
    Array {
        /// The element type.
        element: VariableId,
    },
    /// Type member projection.
    ///
    /// ```ts
    /// T.Item
    /// ```
    Member {
        /// The owner type.
        owner: VariableId,
        /// The member name.
        name: dir::StringId,
        /// The applied static arguments.
        arguments: Vec<ArgumentTerm>,
    },
    /// Fixed-length array type.
    FixedArray {
        /// The repeated element type.
        element: VariableId,
        /// The static array length.
        length: VariableId,
        /// Whether the array is readonly.
        is_readonly: bool,
    },
    /// Runtime-length homogeneous view type.
    Slice {
        /// The element type.
        element: VariableId,
        /// Whether the slice is readonly.
        is_readonly: bool,
    },
    /// Tuple type.
    Tuple {
        /// The tuple elements.
        elements: Vec<TupleElementTerm>,
        /// Whether the tuple is readonly.
        is_readonly: bool,
    },
    /// Structural object shape type.
    Shape {
        /// The shape members.
        members: Vec<ShapeMemberTerm>,
    },
    /// Function type.
    Function(FunctionTerm),
    /// Compact scalar interval type.
    Range {
        /// The inclusive lower bound.
        start: Option<dir::ScalarLiteral>,
        /// The upper bound.
        end: Option<dir::ScalarLiteral>,
        /// Whether the upper bound is included.
        is_inclusive: bool,
    },
    /// Union type.
    Union {
        /// The union elements.
        elements: Vec<VariableId>,
    },
    /// Intersection type.
    Intersection {
        /// The intersection elements.
        elements: Vec<VariableId>,
    },
    /// Type-level operation.
    ///
    /// ```ts
    /// keyof T
    /// ```
    Operation(TypeOperationTerm),
    /// Runtime call expression.
    ///
    /// ```ts
    /// fn(value)
    /// ```
    Call(CallTerm),
    /// Runtime binary operator expression.
    ///
    /// ```ts
    /// left + right
    /// ```
    Binary(BinaryTerm),
    /// Type predicate.
    Predicate {
        /// Whether this is an assertion predicate.
        asserts: bool,
        /// The predicate subject.
        subject: dir::PredicateSubject,
        /// The predicate target type.
        target: Option<VariableId>,
    },
}

/// Term used to bind a static variable.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum StaticTerm {
    /// Concrete static term.
    ///
    /// ```ts
    /// "mutable"
    /// ```
    Value(dir::StaticTerm),
    /// Static variable alias.
    ///
    /// ```ts
    /// import { L } from "./lifetimes";
    /// ```
    Variable(VariableId),
    /// Source expression evaluated as a static term.
    ///
    /// ```ts
    /// N + 1
    /// ```
    Expression(dir::GlobalNodeId<dir::Expression>),
    /// Lifetime union produced by type-position `|`.
    ///
    /// ```ts
    /// L | R
    /// ```
    LifetimeJoin {
        /// The lifetime values being joined.
        elements: Vec<VariableId>,
    },
    /// Compiler intrinsic returning a static value.
    ///
    /// ```ts
    /// LifetimeOf<T>
    /// ```
    Intrinsic {
        /// The intrinsic language item.
        item: dir::LanguageItem,
        /// The intrinsic arguments.
        arguments: SmallVec<[ArgumentTerm; 4]>,
    },
}

/// Check-local memory form term.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum FormTerm {
    /// Managed value form.
    Managed,
    /// Owned value form.
    Owned,
    /// Borrowed value form.
    Borrowed {
        /// The solved lifetime value.
        lifetime: VariableId,
        /// The solved access value.
        access: VariableId,
    },
    /// Raw pointer form.
    Raw,
    /// Placed value form.
    Placed {
        /// The solved place value.
        place: VariableId,
    },
    /// Readonly view form.
    Readonly,
}

/// Argument supplied to a generic application.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum ArgumentTerm {
    /// Type argument.
    Type(VariableId),
    /// Static argument.
    Static(VariableId),
    /// Spread type argument.
    SpreadType(VariableId),
    /// Static spread argument.
    SpreadStatic(VariableId),
}

impl ArgumentTerm {
    /// Return the variable referenced by this argument.
    pub(in crate::check) fn variable(&self) -> VariableId {
        match self {
            Self::Type(variable)
            | Self::Static(variable)
            | Self::SpreadType(variable)
            | Self::SpreadStatic(variable) => *variable,
        }
    }
}

impl TypeTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        match self {
            Self::Variable(variable) => variables.push(*variable),
            Self::Form { form, payload } => {
                variables.push(*payload);
                variables.extend(form.referenced_variables());
            }
            Self::Named {
                symbol: _,
                arguments,
            } => {
                variables.extend(arguments.iter().map(ArgumentTerm::variable));
            }
            Self::Array { element } => variables.push(*element),
            Self::Member {
                owner,
                name: _,
                arguments,
            } => {
                variables.push(*owner);
                variables.extend(arguments.iter().map(ArgumentTerm::variable));
            }
            Self::FixedArray {
                element,
                length,
                is_readonly: _,
            } => {
                variables.push(*element);
                variables.push(*length);
            }
            Self::Slice {
                element,
                is_readonly: _,
            } => variables.push(*element),
            Self::Tuple {
                elements,
                is_readonly: _,
            } => {
                variables.extend(elements.iter().map(|element| element.ty));
            }
            Self::Shape { members } => {
                for member in members {
                    variables.extend(member.referenced_variables());
                }
            }
            Self::Function(function) => variables.extend(function.referenced_variables()),
            Self::Range {
                start: _,
                end: _,
                is_inclusive: _,
            } => {}
            Self::Union { elements } | Self::Intersection { elements } => {
                variables.extend(elements.iter().copied());
            }
            Self::Operation(operation) => variables.extend(operation.referenced_variables()),
            Self::Call(call) => variables.extend(call.referenced_variables()),
            Self::Binary(binary) => variables.extend(binary.referenced_variables()),
            Self::Predicate {
                asserts: _,
                subject: _,
                target,
            } => variables.extend(target.iter().copied()),
            Self::Type(_) => {}
        }

        variables
    }
}

impl BinaryTerm {
    /// Return variables referenced by this term.
    fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        smallvec::smallvec![self.left, self.right]
    }
}

impl CallTerm {
    /// Return variables referenced by this term.
    fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        if self.member.is_none() && self.candidates.is_empty() {
            variables.push(self.callee);
        }
        if !self.candidates.is_empty() {
            variables.extend(self.candidates.iter().map(|candidate| candidate.ty));
        }
        if let Some(member) = &self.member {
            variables.push(member.receiver);
            variables.extend(member.arguments.iter().map(ArgumentTerm::variable));
        }
        variables.extend(self.arguments.iter().copied());

        variables
    }
}

impl FunctionTerm {
    /// Return variables referenced by this term.
    fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        variables.extend(self.generic_parameters.iter().copied());
        variables.extend(self.this_parameter);
        variables.extend(self.parameters.iter().copied());
        variables.extend(self.return_type.iter().copied());

        variables
    }
}

impl ShapeMemberTerm {
    /// Return variables referenced by this term.
    fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        match self {
            Self::Field {
                key: _,
                ty,
                is_optional: _,
                is_readonly: _,
            }
            | Self::CallSignature { ty }
            | Self::ConstructSignature { ty } => variables.push(*ty),
            Self::IndexSignature {
                name: _,
                key_type,
                value_type,
                is_optional: _,
                is_readonly: _,
            } => {
                variables.push(*key_type);
                variables.push(*value_type);
            }
        }

        variables
    }
}

impl TypeOperationTerm {
    /// Return variables referenced by this term.
    fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        match self {
            Self::Conditional {
                left,
                right,
                then_type,
                else_type,
            } => {
                variables.push(*left);
                variables.push(*right);
                variables.push(*then_type);
                variables.push(*else_type);
            }
            Self::Index { left, index } => {
                variables.push(*left);
                variables.push(*index);
            }
            Self::TemplateLiteral { strings: _, spans } => {
                variables.extend(spans.iter().copied());
            }
            Self::Infer {
                name: _,
                constraint,
            } => {
                variables.extend(constraint.iter().copied());
            }
            Self::KeyOf { target } => variables.push(*target),
            Self::Mapped {
                parameter,
                modifiers: _,
                value,
            } => {
                variables.extend(parameter.referenced_variables());
                variables.push(*value);
            }
            Self::Exclude { source, target } => {
                variables.push(*source);
                variables.push(*target);
            }
            Self::Intrinsic { item: _, arguments } => {
                variables.extend(arguments.iter().map(ArgumentTerm::variable));
            }
        }

        variables
    }
}

impl MappedParameterTerm {
    /// Return variables referenced by this term.
    fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        variables.push(self.constraint);
        variables.extend(self.key_remap);

        variables
    }
}

impl StaticTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        match self {
            Self::Variable(variable) => variables.push(*variable),
            Self::LifetimeJoin { elements } => variables.extend(elements.iter().copied()),
            Self::Intrinsic { item: _, arguments } => {
                variables.extend(arguments.iter().map(ArgumentTerm::variable));
            }
            Self::Value(_) | Self::Expression(_) => {}
        }

        variables
    }
}

impl FormTerm {
    /// Return whether two forms share the same constructor.
    pub(in crate::check) fn same_constructor(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Owned, Self::Owned)
                | (Self::Managed, Self::Managed)
                | (
                    Self::Borrowed {
                        lifetime: _,
                        access: _,
                    },
                    Self::Borrowed {
                        lifetime: _,
                        access: _,
                    },
                )
                | (Self::Raw, Self::Raw)
                | (Self::Placed { place: _ }, Self::Placed { place: _ })
                | (Self::Readonly, Self::Readonly)
        )
    }

    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        match self {
            Self::Borrowed { lifetime, access } => {
                variables.push(*lifetime);
                variables.push(*access);
            }
            Self::Placed { place } => variables.push(*place),
            Self::Managed | Self::Owned | Self::Raw | Self::Readonly => {}
        }

        variables
    }
}
