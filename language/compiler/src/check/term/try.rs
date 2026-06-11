use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    Answer, CheckState, Condition, GenericArgument, MemberLookup, MemberReceiver, Origin, TermId,
    TypeLiteralTerm, TypeOperand, TypeRelation, TypeTerm, VariableId,
};
use crate::{CompilerError, CompilerResult};

/// Runtime try operator term.
///
/// ```ds
/// result?
/// result!
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct TryTerm {
    /// The source try expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The tried expression type.
    pub(in crate::check) value: TypeOperand,
    /// The try operator behavior.
    pub(in crate::check) kind: TryTermKind,
}

/// Runtime try failure projection.
///
/// ```ds
/// value?
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct TryFailureTerm {
    /// The source try expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The tried expression type.
    pub(in crate::check) value: TypeOperand,
}

/// Opened try operator shape.
struct TryOpening {
    /// The value produced when evaluation continues.
    success: TypeOperand,
    /// The value propagated when evaluation breaks.
    residual: TypeOperand,
}

/// Try operator behavior.
///
/// Examples:
/// ```ds
/// result?
/// result!
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) enum TryTermKind {
    /// Propagate failure through the enclosing return type.
    ///
    /// Examples:
    /// ```ds
    /// result?
    /// ```
    Maybe,
    /// Trap failure and produce the successful value.
    ///
    /// Examples:
    /// ```ds
    /// result!
    /// ```
    Must,
}

impl CheckState<'_> {
    /// Reduce one try operator term.
    pub(in crate::check) fn reduce_try_term(
        &mut self,
        origin: Origin,
        module: ModuleId,
        tried: TryTerm,
    ) -> CompilerResult<Answer<TypeOperand>> {
        let Some(value) = self.try_output_type_operand(origin, module, tried.value)? else {
            return Ok(Answer::pending(tried.value.dependencies(self)));
        };

        Ok(Answer::Ready(value))
    }

    /// Reduce one try failure projection.
    pub(in crate::check) fn reduce_try_failure_term(
        &mut self,
        origin: Origin,
        module: ModuleId,
        tried: TryFailureTerm,
    ) -> CompilerResult<Answer<TypeOperand>> {
        let Some(failure) = self.try_residual_type_operand(origin, module, tried.value)? else {
            return Ok(Answer::pending(tried.value.dependencies(self)));
        };

        Ok(Answer::Ready(failure))
    }

    /// Check a try value projection to produce the expected result.
    pub(in crate::check) fn expect_try_term(
        &mut self,
        origin: Origin,
        tried: &TryTerm,
        result: VariableId,
    ) -> CompilerResult<Answer<()>> {
        let Some(value) = self.try_output_type_operand(origin, result.module, tried.value)? else {
            return Ok(Answer::pending(tried.value.dependencies(self)));
        };

        self.constrain_type(
            origin,
            TypeRelation::Assignable,
            value,
            result,
            Condition::Always,
        );

        Ok(Answer::Ready(()))
    }

    /// Check a try failure projection to produce the expected result.
    pub(in crate::check) fn expect_try_failure_term(
        &mut self,
        origin: Origin,
        tried: &TryFailureTerm,
        result: VariableId,
    ) -> CompilerResult<Answer<()>> {
        let Some(failure) = self.try_residual_type_operand(origin, result.module, tried.value)?
        else {
            return Ok(Answer::pending(tried.value.dependencies(self)));
        };

        self.constrain_type(
            origin,
            TypeRelation::Assignable,
            failure,
            result,
            Condition::Always,
        );

        Ok(Answer::Ready(()))
    }

    /// Decide whether a propagated try failure fits an enclosing return type.
    pub(in crate::check) fn decide_try_propagation(
        &mut self,
        source: dir::GlobalNodeIdAny,
        value: TypeOperand,
        return_type: TypeOperand,
    ) -> CompilerResult<Answer<bool>> {
        let Some(failure) =
            self.try_residual_type_operand(Origin::Node(source), source.module_id, value)?
        else {
            return Ok(Answer::pending(value.dependencies(self)));
        };
        let target = self.from_residual_type(source, failure)?;
        let Answer::Ready(return_type) =
            self.reduce_type_operand(Origin::Node(source), return_type)?
        else {
            return Ok(Answer::pending(return_type.dependencies(self)));
        };

        let target = self.inference.push_term(target);

        self.decide_type_relation(TypeRelation::Implements, return_type, target)
    }

    /// Return one try associated type.
    fn try_output_type_operand(
        &mut self,
        origin: Origin,
        module: ModuleId,
        value: TypeOperand,
    ) -> CompilerResult<Option<TypeOperand>> {
        let Some(opening) = self.open_try_operand(origin, module, value)? else {
            return Ok(None);
        };

        Ok(Some(opening.success))
    }

    /// Return one try residual type.
    fn try_residual_type_operand(
        &mut self,
        origin: Origin,
        module: ModuleId,
        value: TypeOperand,
    ) -> CompilerResult<Option<TypeOperand>> {
        let Some(opening) = self.open_try_operand(origin, module, value)? else {
            return Ok(None);
        };

        Ok(Some(opening.residual))
    }

    /// Return the opened success and residual types for one try operand.
    fn open_try_operand(
        &mut self,
        origin: Origin,
        module: ModuleId,
        value: TypeOperand,
    ) -> CompilerResult<Option<TryOpening>> {
        let Answer::Ready(value) = self.reduce_type_operand(origin, value)? else {
            return Ok(None);
        };
        let Some(term) = self.type_operand_term_id(value)? else {
            return Ok(None);
        };

        let opening = self.open_nullish_type_term(term)?;
        let success =
            self.try_associated_type_operand(origin, module, opening.success, "Output")?;
        let residual =
            self.try_associated_type_operand(origin, module, opening.success, "Residual")?;

        // keep nullish-only opening when the value is not a try carrier
        let Answer::Ready(success) = success else {
            return Ok(None);
        };
        let Answer::Ready(residual) = residual else {
            return Ok(None);
        };
        let success = success.unwrap_or(opening.success);
        let residual = match residual {
            Some(residual) => residual,
            None => self
                .inference
                .push_term(TypeTerm::Literal(TypeLiteralTerm::Never))
                .into(),
        };
        let residual = self.union_type_operands([opening.residual, residual])?;

        Ok(Some(TryOpening { success, residual }))
    }

    /// Return one type term after removing outer nullish values.
    fn open_nullish_type_term(&mut self, term: TermId<TypeTerm>) -> CompilerResult<TryOpening> {
        let term_id = term;
        let term = self.inference.term(term_id);
        match term {
            TypeTerm::Literal(literal) if literal.is_nullish() => {
                let literal = literal.clone();
                let success = self
                    .inference
                    .push_term(TypeTerm::Literal(TypeLiteralTerm::Never))
                    .into();
                let residual = self.inference.push_term(TypeTerm::Literal(literal)).into();

                Ok(TryOpening { success, residual })
            }
            TypeTerm::Union { elements } => {
                let elements = elements.iter().copied().collect();

                self.open_nullish_union(elements)
            }
            _ => {
                let success = TypeOperand::Term(term_id);
                let residual = self
                    .inference
                    .push_term(TypeTerm::Literal(TypeLiteralTerm::Never))
                    .into();

                Ok(TryOpening { success, residual })
            }
        }
    }

    /// Return one union after removing outer nullish values.
    fn open_nullish_union(&mut self, elements: Vec<TypeOperand>) -> CompilerResult<TryOpening> {
        let mut success = Vec::with_capacity(elements.len());
        let mut residual = Vec::new();

        // split nullish and non-nullish union members
        for element in elements {
            let Some(term) = self.type_operand_term_id(element)? else {
                return Ok(TryOpening {
                    success: self.type_term_operand(TypeTerm::Union { elements: success }),
                    residual: self.type_term_operand(TypeTerm::Union { elements: residual }),
                });
            };
            let TypeTerm::Literal(literal) = self.inference.term(term) else {
                success.push(element);

                continue;
            };

            if literal.is_nullish() {
                residual.push(element);
            } else {
                success.push(element);
            }
        }

        // collapse simple union results
        let success = self.union_type_operands_or_never(success)?;
        let residual = self.union_type_operands_or_never(residual)?;

        Ok(TryOpening { success, residual })
    }

    /// Return one try associated type when the opened value is a try carrier.
    fn try_associated_type_operand(
        &mut self,
        origin: Origin,
        module: ModuleId,
        value: TypeOperand,
        name: &str,
    ) -> CompilerResult<Answer<Option<TypeOperand>>> {
        let key = dir::StaticKey::Name(dir::StringId::for_text(name));
        let receiver = MemberReceiver::Value(value);
        let lookup = self.resolve_member(origin, module, &receiver, &key)?;

        match lookup {
            MemberLookup::Pending(blockers) => Ok(Answer::Pending(blockers)),
            MemberLookup::Missing => Ok(Answer::Ready(None)),
            lookup => {
                let Some(member) =
                    self.member_type_from_lookup(module, origin, key, &[], lookup, None)?
                else {
                    return Err(CompilerError::Internal {
                        message: format!("try associated type {name} has no readable member type"),
                    });
                };

                Ok(Answer::Ready(Some(member)))
            }
        }
    }

    /// Return a compact union from operands.
    fn union_type_operands(
        &mut self,
        elements: impl IntoIterator<Item = TypeOperand>,
    ) -> CompilerResult<TypeOperand> {
        let mut elements = elements.into_iter().collect::<Vec<_>>();
        let mut kept = Vec::with_capacity(elements.len());

        // remove never members
        for element in elements.drain(..) {
            let Some(term) = self.type_operand_term_id(element)? else {
                kept.push(element);

                continue;
            };
            if !matches!(
                self.inference.term(term),
                TypeTerm::Literal(TypeLiteralTerm::Never)
            ) {
                kept.push(element);
            }
        }

        self.union_type_operands_or_never(kept)
    }

    /// Return a compact union from operands or never.
    fn union_type_operands_or_never(
        &mut self,
        elements: Vec<TypeOperand>,
    ) -> CompilerResult<TypeOperand> {
        if elements.is_empty() {
            Ok(self
                .inference
                .push_term(TypeTerm::Literal(TypeLiteralTerm::Never))
                .into())
        } else if elements.len() == 1 {
            Ok(elements[0])
        } else {
            Ok(self.type_term_operand(TypeTerm::Union { elements }))
        }
    }

    /// Return the `FromResidual<R>` protocol type.
    pub(in crate::check) fn from_residual_type(
        &mut self,
        source: dir::GlobalNodeIdAny,
        failure: TypeOperand,
    ) -> CompilerResult<TypeTerm> {
        let symbol = self.language_symbol(dir::LanguageItem::FromResidual);
        let argument = GenericArgument::Type(failure);

        Ok(TypeTerm::Reference {
            origin: Origin::Node(source),
            symbol,
            arguments: vec![argument].into(),
        })
    }
}
