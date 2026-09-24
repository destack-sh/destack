use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{Cause, CauseKind, CheckState, Origin, Relation, TypeSubstitution, Verdict};
use crate::{CompilerError, CompilerResult};

/// The key one auto interface decision memoizes under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) struct DecisionKey {
    /// The canonical type.
    pub(in crate::sema) ty: dir::GlobalTypeId,
    /// The decided interface.
    pub(in crate::sema) interface: dir::AutoInterface,
    /// The assuming template.
    pub(in crate::sema) assumes: Option<dir::GlobalGenericTemplateId>,
}

impl CheckState<'_> {
    /// Decide one applied compiler-known interface intrinsically.
    pub(in crate::sema) fn decide_intrinsic_interface(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<Verdict> {
        // read the compared type of each intrinsic binary interface
        if matches!(
            interface,
            dir::AutoInterface::Equal
                | dir::AutoInterface::PartialEqual
                | dir::AutoInterface::Compare
                | dir::AutoInterface::PartialCompare
                | dir::AutoInterface::StrictEqual
        ) {
            let (module, application) = self.nominal_application(target)?;
            let arguments = self.type_ids(module, application.arguments)?;

            // read the compared operand the application writes
            let other = match arguments {
                [] => None,
                [other] => Some(*other),
                _ => {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "intrinsic {} application has {} arguments",
                            interface.name(),
                            arguments.len()
                        ),
                    });
                }
            };

            // relate the receiver against the compared operand
            if let Some(other) = other {
                // read the argument solutions the bound picked up after queuing
                let other = self.shallow_resolve(other)?;
                let substitution = TypeSubstitution::default().with_receiver(ty);
                let other = self.substitute_type(other, &substitution)?;

                // decide strict equality from both exact operands
                if interface == dir::AutoInterface::StrictEqual {
                    if !self.collect_open_variables([ty, other])?.is_empty() {
                        return Ok(Verdict::Ambiguous);
                    }
                    let is_equatable = self.has_strict_equal_conformance(origin, ty, other)?;

                    return Ok(Verdict::decided(is_equatable));
                }

                // compare the receiver with itself, an open operand waiting, a literal widened
                if !self.collect_open_variables([other])?.is_empty() {
                    return Ok(Verdict::Ambiguous);
                }
                let compared = match self.is_literal_shape(ty)? {
                    true => self.widen_type(ty)?,
                    false => ty,
                };
                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
                let receiver =
                    self.constrain_type(origin, cause, Relation::Equal, other, compared)?;
                if receiver != Verdict::Holds {
                    return Ok(receiver);
                }
            }
        }

        // decide the interface's own conformance rule, propagating its verdict
        self.decide_auto_interface(origin, ty, interface)
    }

    /// Decide one compiler-known auto interface for one type.
    pub(in crate::sema) fn decide_auto_interface(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<Verdict> {
        // decide an open numeric variable at its family default
        if let Some(variable) = self.root_variable(ty)? {
            return match self.root_kind(variable)?.fallback() {
                Some(fallback) => {
                    let fallback = self.intern_type(fallback)?;

                    self.decide_auto_interface(origin, fallback, interface)
                }
                None => Ok(Verdict::Ambiguous),
            };
        }

        // decide conformance once over variable-free types under the assuming template
        let flags = self.type_flags(ty)?;
        let key = if flags.has_variable() {
            None
        } else {
            self.decision_scope(origin, &[ty])?
                .map(|assumes| DecisionKey {
                    ty,
                    interface,
                    assumes,
                })
        };

        // serve the memoized verdict
        if let Some(key) = &key
            && let Some(holds) = self.conformances.get(key)
        {
            return Ok(Verdict::decided(*holds));
        }

        // use bounds declared by generic types
        if let Some(decision) = self.decide_generic_auto_interface(origin, ty, interface)? {
            if let Some(key) = key {
                self.conformances.insert(key, decision);
            }

            return Ok(Verdict::decided(decision));
        }

        // let a derive list or a negative implementation refuse the interface
        if let dir::Type::Application(instance) = self.ty(ty)? {
            let excluded = interface.is_auto_derivable()
                && self
                    .definition(instance.symbol)?
                    .as_deref()
                    .and_then(dir::Definition::derives)
                    .is_some_and(|derives| !derives.contains(&interface));
            if excluded || self.declares_negative(instance.symbol, interface)? {
                if let Some(key) = key {
                    self.conformances.insert(key, false);
                }

                return Ok(Verdict::Fails);
            }
        }

        // dispatch compiler-known conformance rules
        let mut active = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        let verdict = match interface {
            dir::AutoInterface::AtomicSafe => self.is_atomic_safe(ty).map(Verdict::decided),
            dir::AutoInterface::DynamicSafe => self.decide_dynamic_safe(origin, ty, &mut active),
            dir::AutoInterface::Integer => self
                .has_scalar_representation(ty, dir::ScalarDomain::Integer)
                .map(Verdict::decided),
            dir::AutoInterface::IntegerDomain => self
                .is_scalar_domain_only(origin, ty, dir::ScalarDomain::Integer)
                .map(Verdict::decided),
            dir::AutoInterface::Float => self
                .has_scalar_representation(ty, dir::ScalarDomain::Float)
                .map(Verdict::decided),
            dir::AutoInterface::FloatDomain => self
                .is_scalar_domain_only(origin, ty, dir::ScalarDomain::Float)
                .map(Verdict::decided),
            dir::AutoInterface::Copy => self.decide_copy(origin, ty, &mut active),
            dir::AutoInterface::Drop => self.decide_drop(origin, ty, &mut active),
            dir::AutoInterface::SharedSafe => self.is_shared_safe(origin, ty).map(Verdict::decided),
            dir::AutoInterface::Concrete => self.is_concrete(origin, ty).map(Verdict::decided),
            dir::AutoInterface::StrictEqual => self
                .has_strict_equal_conformance(origin, ty, ty)
                .map(Verdict::decided),
            dir::AutoInterface::Clone => match self.decide_copy(origin, ty, &mut active)? {
                Verdict::Holds => Ok(Verdict::Holds),
                _ => self.decide_derivable(origin, ty, interface),
            },
            dir::AutoInterface::Equal
            | dir::AutoInterface::PartialEqual
            | dir::AutoInterface::Debug
            | dir::AutoInterface::Display
            | dir::AutoInterface::Hash
            | dir::AutoInterface::Default
            | dir::AutoInterface::Unpin
            | dir::AutoInterface::Zeroable => self.decide_derivable(origin, ty, interface),
            // order scalars intrinsically
            dir::AutoInterface::Compare | dir::AutoInterface::PartialCompare => {
                Ok(Verdict::decided(
                    self.ty(ty)?
                        .scalar_domain()
                        .and_then(|domain| domain.conforms_to(interface))
                        .unwrap_or(false),
                ))
            }
            // leave serialization to explicit derives
            dir::AutoInterface::Serialize | dir::AutoInterface::Deserialize => Ok(Verdict::Fails),
        }?;

        // memoize a decided verdict, leaving an ambiguous one uncached
        if let Some(key) = key
            && verdict != Verdict::Ambiguous
        {
            self.conformances.insert(key, verdict.holds());
        }

        Ok(verdict)
    }

    /// Return whether one nominal or one of its root extensions negatively implements an interface.
    pub(in crate::sema) fn declares_negative(
        &mut self,
        symbol: dir::GlobalSymbolId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<bool> {
        let item = dir::LanguageItem::from(interface);
        for conformer in self.nominal_conformers(symbol)? {
            let Some(definition) = self.definition(conformer)? else {
                continue;
            };
            for negative in definition.negatives() {
                let Some(negated) = self.ty(negative.interface)?.symbol() else {
                    continue;
                };
                if self.language_item(negated)? == Some(item) {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Decide auto conformance for one generic type.
    pub(in crate::sema) fn decide_generic_auto_interface(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<Option<bool>> {
        // read the resolved head of the subject
        let ty = self.shallow_resolve(ty)?;

        // select the bounds owned by each generic form
        let decision = match self.ty(ty)? {
            dir::Type::Parameter(parameter) | dir::Type::Erased(parameter) => {
                let item = dir::LanguageItem::from(interface);
                let target = self.language_type(item, &[])?;
                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));

                Some(
                    self.relate_parameter_bounds(
                        origin,
                        cause,
                        Relation::Subtype,
                        parameter,
                        target,
                    )?
                    .holds(),
                )
            }
            dir::Type::This => {
                let item = dir::LanguageItem::from(interface);
                let target = self.language_type(item, &[])?;
                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));

                Some(
                    self.relate_this_bounds(origin, cause, Relation::Subtype, target)?
                        .holds(),
                )
            }
            _ => None,
        };

        Ok(decision)
    }

    /// Return whether two operand types have builtin `StrictEqual<R>` conformance.
    fn has_strict_equal_conformance(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // require builtin equality over overlapping operands
        let has_builtin = self.has_builtin_strict_equality(origin, left, right)?;
        let overlaps = self.types_may_overlap(origin, left, right)?;

        Ok(has_builtin && overlaps)
    }

    /// Return whether one type has one builtin scalar representation.
    fn has_scalar_representation(
        &mut self,
        ty: dir::GlobalTypeId,
        domain: dir::ScalarDomain,
    ) -> CompilerResult<bool> {
        // match the primitive's own scalar domain
        let is_representation = matches!(
            self.ty(ty)?,
            dir::Type::Primitive(primitive) if primitive.scalar_domain() == domain
        );

        Ok(is_representation)
    }

    /// Return whether one type belongs entirely to one scalar domain.
    fn is_scalar_domain_only(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        domain: dir::ScalarDomain,
    ) -> CompilerResult<bool> {
        // require every family to sit inside that domain
        let families = self.scalar_families(origin, ty)?;
        let is_only_domain = families.is_some_and(|families| families.is_only_domain(domain));

        Ok(is_only_domain)
    }
}
