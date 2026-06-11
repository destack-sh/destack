use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, CheckState, FormTerm, GenericArgument, StaticOperand, StaticTerm, TermId,
    TypeLiteralTerm, TypeOperand, TypeTerm,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    pub(in crate::check) fn reduce_memory_term(
        &mut self,
        module: ModuleId,
        item: dir::LanguageItem,
        arguments: &[GenericArgument],
    ) -> CompilerResult<Answer<TypeOperand>> {
        let Some(target) = self.generic_argument_type_operand(arguments, 0) else {
            return Err(CompilerError::Internal {
                message: format!("memory intrinsic {item:?} has no target type argument"),
            });
        };
        let Some(term) = (match item {
            dir::LanguageItem::PayloadOf => self.memory_payload_type(target)?,
            dir::LanguageItem::BaseOf => self.memory_base_type(module, target)?,
            dir::LanguageItem::WithBase => {
                let Some(base) = self.generic_argument_type_operand(arguments, 1) else {
                    return Err(CompilerError::Internal {
                        message: "WithBase intrinsic has no base type argument".into(),
                    });
                };

                self.memory_with_base_type(module, target, base)?
            }
            dir::LanguageItem::WithPlace => {
                let Some(place) = self.generic_argument_static_operand(arguments, 1)? else {
                    return Err(CompilerError::Internal {
                        message: "WithPlace intrinsic has no place argument".into(),
                    });
                };
                let Some(place) = self.place_value(module, place)? else {
                    return self.pending_memory_intrinsic(item, target, arguments);
                };
                let place = self
                    .inference
                    .push_term(StaticTerm::Literal(dir::StaticTerm::Place { place }));

                self.memory_with_place_type(module, target, place.into())?
            }
            dir::LanguageItem::WithSpace => {
                let Some(space) = self.generic_argument_static_operand(arguments, 1)? else {
                    return Err(CompilerError::Internal {
                        message: "WithSpace intrinsic has no space argument".into(),
                    });
                };
                let Some(space) = self.space_value(module, space)? else {
                    return self.pending_memory_intrinsic(item, target, arguments);
                };
                let place = self
                    .inference
                    .push_term(StaticTerm::Literal(dir::StaticTerm::Place {
                        place: dir::Place::Space(space),
                    }));

                self.memory_with_place_type(module, target, place.into())?
            }
            dir::LanguageItem::WithLifetime => {
                let Some(lifetime) = self.generic_argument_static_operand(arguments, 1)? else {
                    return Err(CompilerError::Internal {
                        message: "WithLifetime intrinsic has no lifetime argument".into(),
                    });
                };

                self.memory_with_lifetime_type(module, target, lifetime)?
            }
            dir::LanguageItem::WithAccess => {
                let Some(access) = self.generic_argument_static_operand(arguments, 1)? else {
                    return Err(CompilerError::Internal {
                        message: "WithAccess intrinsic has no access argument".into(),
                    });
                };

                self.memory_with_access_type(module, target, access)?
            }
            dir::LanguageItem::WithOwnership => {
                let Some(ownership) = self.generic_argument_static_operand(arguments, 1)? else {
                    return Err(CompilerError::Internal {
                        message: "WithOwnership intrinsic has no ownership argument".into(),
                    });
                };
                let Some(lifetime) = self.generic_argument_static_operand(arguments, 2)? else {
                    return Err(CompilerError::Internal {
                        message: "WithOwnership intrinsic has no lifetime argument".into(),
                    });
                };

                self.memory_with_ownership_type(module, target, ownership, lifetime)?
            }
            _ => {
                return Err(CompilerError::Internal {
                    message: format!("language item {item:?} is not a memory type intrinsic"),
                });
            }
        }) else {
            return self.pending_memory_intrinsic(item, target, arguments);
        };

        Ok(Answer::Ready(term))
    }

    /// Return blockers for a memory intrinsic that cannot reduce yet.
    fn pending_memory_intrinsic(
        &self,
        item: dir::LanguageItem,
        target: TypeOperand,
        arguments: &[GenericArgument],
    ) -> CompilerResult<Answer<TypeOperand>> {
        let blockers = target
            .dependencies(self)
            .into_iter()
            .chain(
                arguments
                    .iter()
                    .flat_map(|argument| argument.dependencies(self)),
            )
            .collect::<SmallVec<[_; 4]>>();

        if blockers.is_empty() {
            Err(CompilerError::Internal {
                message: format!("memory intrinsic {item:?} could not reduce"),
            })
        } else {
            Ok(Answer::pending(blockers))
        }
    }

    /// Return one type argument operand.
    pub(in crate::check) fn generic_argument_type_operand(
        &self,
        arguments: &[GenericArgument],
        index: usize,
    ) -> Option<TypeOperand> {
        arguments.get(index).and_then(GenericArgument::type_operand)
    }

    /// Return one static argument operand.
    pub(in crate::check) fn generic_argument_static_operand(
        &mut self,
        arguments: &[GenericArgument],
        index: usize,
    ) -> CompilerResult<Option<StaticOperand>> {
        let Some(argument) = arguments.get(index) else {
            return Ok(None);
        };
        if let Some(operand) = argument.static_operand() {
            return Ok(Some(operand));
        }

        let Some(operand) = argument.type_operand() else {
            return Ok(None);
        };

        self.static_operand_from_type(operand)
    }

    /// Return the static interpretation of one type operand.
    pub(in crate::check) fn static_operand_from_type(
        &mut self,
        operand: TypeOperand,
    ) -> CompilerResult<Option<StaticOperand>> {
        let Some(term) = self.type_operand_term_id(operand)? else {
            return Ok(None);
        };
        let term = match self.inference.term(term) {
            TypeTerm::Literal(TypeLiteralTerm::Scalar(value)) => {
                StaticTerm::Literal(dir::StaticTerm::ScalarLiteral { value: *value })
            }
            TypeTerm::Parameter(slot) => StaticTerm::Parameter(*slot),
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments,
            } => {
                let symbol = *symbol;
                let arguments = arguments.iter().copied().collect();
                let Some(item) = self.environment.language.item(symbol) else {
                    return Ok(None);
                };
                if !Self::static_memory_item(item) {
                    return Ok(None);
                }

                StaticTerm::Intrinsic { item, arguments }
            }
            TypeTerm::StaticValue { value } => return Ok(Some(*value)),
            TypeTerm::Type(_) => return Ok(None),
            TypeTerm::Union { elements } => {
                let elements = elements.iter().copied().collect::<Vec<_>>();
                let mut values = Vec::with_capacity(elements.len());

                for element in elements {
                    let Some(value) = self.static_operand_from_type(element)? else {
                        return Ok(None);
                    };

                    values.push(value);
                }

                StaticTerm::Union { elements: values }
            }
            TypeTerm::Literal(_)
            | TypeTerm::Intrinsic
            | TypeTerm::This
            | TypeTerm::Member(_)
            | TypeTerm::Form { .. }
            | TypeTerm::Dynamic { .. }
            | TypeTerm::Operation(_)
            | TypeTerm::Array { .. }
            | TypeTerm::FixedArray { .. }
            | TypeTerm::Range { .. }
            | TypeTerm::Slice { .. }
            | TypeTerm::Tuple { .. }
            | TypeTerm::Shape(_)
            | TypeTerm::Function(_)
            | TypeTerm::Closure { .. }
            | TypeTerm::Intersection { .. }
            | TypeTerm::TypeValue(_)
            | TypeTerm::Call(_)
            | TypeTerm::Construct(_)
            | TypeTerm::RangeValue(_)
            | TypeTerm::Tree(_)
            | TypeTerm::ImportMeta(_)
            | TypeTerm::Receiver(_)
            | TypeTerm::Super(_)
            | TypeTerm::Operator(_)
            | TypeTerm::TaggedTemplate(_)
            | TypeTerm::Template(_)
            | TypeTerm::Await(_)
            | TypeTerm::Yield(_)
            | TypeTerm::Try(_)
            | TypeTerm::TryFailure(_)
            | TypeTerm::Identity(_)
            | TypeTerm::InstanceCheck(_)
            | TypeTerm::KeyMembership(_)
            | TypeTerm::Index(_)
            | TypeTerm::IndexSet(_) => return Ok(None),
        };
        let term = self.inference.push_term(term);

        Ok(Some(term.into()))
    }

    /// Return whether one language item is a static returning memory intrinsic.
    fn static_memory_item(item: dir::LanguageItem) -> bool {
        matches!(
            item,
            dir::LanguageItem::OwnershipOf
                | dir::LanguageItem::OwnershipOr
                | dir::LanguageItem::PlaceOf
                | dir::LanguageItem::PlaceOr
                | dir::LanguageItem::PlaceIn
                | dir::LanguageItem::SpaceOf
                | dir::LanguageItem::SpaceOr
                | dir::LanguageItem::LifetimeOf
                | dir::LanguageItem::LifetimeOr
                | dir::LanguageItem::AccessOf
                | dir::LanguageItem::AccessOr
                | dir::LanguageItem::IsManaged
                | dir::LanguageItem::IsOwned
                | dir::LanguageItem::IsBorrowed
                | dir::LanguageItem::IsRaw
                | dir::LanguageItem::IsShared
                | dir::LanguageItem::IsSharedIn
        )
    }

    /// Return the immediate payload under one memory form.
    fn memory_payload_type(&mut self, target: TypeOperand) -> CompilerResult<Option<TypeOperand>> {
        let Some(target) = self.type_operand_term_id(target)? else {
            return Ok(None);
        };
        let term = match self.inference.term(target) {
            TypeTerm::Form { payload, .. } => *payload,
            _ => target.into(),
        };

        Ok(Some(term))
    }

    /// Return the unqualified base under all memory forms.
    fn memory_base_type(
        &mut self,
        module: ModuleId,
        target: TypeOperand,
    ) -> CompilerResult<Option<TypeOperand>> {
        let Some(target) = self.type_operand_term_id(target)? else {
            return Ok(None);
        };
        let term = match self.inference.term(target) {
            TypeTerm::Form { payload, .. } => {
                let payload = *payload;
                let Some(term) = self.memory_base_type(module, payload)? else {
                    return Ok(None);
                };

                term
            }
            _ => target.into(),
        };

        Ok(Some(term))
    }

    /// Replace the unqualified base under memory forms.
    fn memory_with_base_type(
        &mut self,
        module: ModuleId,
        target: TypeOperand,
        base: TypeOperand,
    ) -> CompilerResult<Option<TypeOperand>> {
        let Some(target) = self.type_operand_term_id(target)? else {
            return Ok(None);
        };
        let term = match self.inference.term(target) {
            TypeTerm::Form { form, payload } => {
                let form = *form;
                let payload = *payload;
                let Some(payload) = self.memory_with_base_type(module, payload, base)? else {
                    return Ok(None);
                };

                self.type_term_operand(TypeTerm::Form { form, payload })
            }
            _ => base,
        };

        Ok(Some(term))
    }

    /// Rewrite the ownership form inside placement wrappers.
    fn memory_with_ownership_type(
        &mut self,
        module: ModuleId,
        target: TypeOperand,
        ownership: StaticOperand,
        lifetime: StaticOperand,
    ) -> CompilerResult<Option<TypeOperand>> {
        let Some(target) = self.type_operand_term_id(target)? else {
            return Ok(None);
        };
        let Some(form) = self.ownership_form(module, ownership, lifetime)? else {
            return Ok(None);
        };
        let term = match self.inference.term(target) {
            TypeTerm::Form {
                form: wrapper,
                payload,
            } if matches!(
                self.inference.term(*wrapper),
                FormTerm::Placed { .. } | FormTerm::Readonly
            ) =>
            {
                let wrapper = *wrapper;
                let payload = *payload;
                let Some(payload) =
                    self.memory_with_ownership_type(module, payload, ownership, lifetime)?
                else {
                    return Ok(None);
                };

                self.type_term_operand(TypeTerm::Form {
                    form: wrapper,
                    payload,
                })
            }
            TypeTerm::Form { payload, .. } => self.type_term_operand(TypeTerm::Form {
                form,
                payload: *payload,
            }),
            _ => self.type_term_operand(TypeTerm::Form {
                form,
                payload: target.into(),
            }),
        };

        Ok(Some(term))
    }

    /// Rewrite the placement form around a type.
    fn memory_with_place_type(
        &mut self,
        _module: ModuleId,
        target: TypeOperand,
        place: StaticOperand,
    ) -> CompilerResult<Option<TypeOperand>> {
        let Some(target) = self.type_operand_term_id(target)? else {
            return Ok(None);
        };
        let payload = match self.inference.term(target) {
            TypeTerm::Form { form, payload }
                if matches!(self.inference.term(*form), FormTerm::Placed { .. }) =>
            {
                *payload
            }
            _ => target.into(),
        };

        let form = self.inference.push_term(FormTerm::Placed { place });

        Ok(Some(
            self.type_term_operand(TypeTerm::Form { form, payload }),
        ))
    }

    /// Rewrite the borrow lifetime inside a type.
    fn memory_with_lifetime_type(
        &mut self,
        module: ModuleId,
        target: TypeOperand,
        lifetime: StaticOperand,
    ) -> CompilerResult<Option<TypeOperand>> {
        let Some(target) = self.type_operand_term_id(target)? else {
            return Ok(None);
        };
        let term = match self.inference.term(target) {
            TypeTerm::Form { form, payload } => match self.inference.term(*form) {
                FormTerm::Borrowed { access, .. } => {
                    let access = *access;
                    let payload = *payload;
                    let form = self
                        .inference
                        .push_term(FormTerm::Borrowed { lifetime, access });

                    self.type_term_operand(TypeTerm::Form { form, payload })
                }
                FormTerm::Placed { .. } | FormTerm::Readonly => {
                    let form = *form;
                    let payload = *payload;
                    let Some(payload) =
                        self.memory_with_lifetime_type(module, payload, lifetime)?
                    else {
                        return Ok(None);
                    };

                    self.type_term_operand(TypeTerm::Form { form, payload })
                }
                FormTerm::Managed | FormTerm::Owned | FormTerm::Raw => target.into(),
            },
            _ => target.into(),
        };

        Ok(Some(term))
    }

    /// Rewrite the borrow access inside a type.
    fn memory_with_access_type(
        &mut self,
        module: ModuleId,
        target: TypeOperand,
        access: StaticOperand,
    ) -> CompilerResult<Option<TypeOperand>> {
        let Some(target) = self.type_operand_term_id(target)? else {
            return Ok(None);
        };
        let term = match self.inference.term(target) {
            TypeTerm::Form { form, payload } => match self.inference.term(*form) {
                FormTerm::Borrowed { lifetime, .. } => {
                    let lifetime = *lifetime;
                    let payload = *payload;
                    let form = self
                        .inference
                        .push_term(FormTerm::Borrowed { lifetime, access });

                    self.type_term_operand(TypeTerm::Form { form, payload })
                }
                FormTerm::Placed { .. } | FormTerm::Readonly => {
                    let form = *form;
                    let payload = *payload;
                    let Some(payload) = self.memory_with_access_type(module, payload, access)?
                    else {
                        return Ok(None);
                    };

                    self.type_term_operand(TypeTerm::Form { form, payload })
                }
                FormTerm::Managed | FormTerm::Owned | FormTerm::Raw => target.into(),
            },
            _ => target.into(),
        };

        Ok(Some(term))
    }

    /// Return an ownership form from a static ownership value.
    fn ownership_form(
        &mut self,
        module: ModuleId,
        ownership: StaticOperand,
        lifetime: StaticOperand,
    ) -> CompilerResult<Option<TermId<FormTerm>>> {
        let Some(ownership) = self.ownership_value(module, ownership)? else {
            return Ok(None);
        };
        let form = match ownership.as_str() {
            "managed" => FormTerm::Managed,
            "owned" => FormTerm::Owned,
            "borrowed" => FormTerm::Borrowed {
                lifetime,
                access: self.mutable_access_operand(),
            },
            "raw" => FormTerm::Raw,
            _ => return Ok(None),
        };

        Ok(Some(self.inference.push_term(form)))
    }

    /// Return a memory intrinsic static value from a stored term.
    pub(in crate::check) fn memory_static_value_from_term(
        &mut self,
        module: ModuleId,
        item: dir::LanguageItem,
        term: TermId<StaticTerm>,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let Some(target) = self.static_term_type_argument(term, 0) else {
            return Ok(None);
        };
        let value = match item {
            dir::LanguageItem::OwnershipOf => self.memory_ownership_value(module, target)?,
            dir::LanguageItem::OwnershipOr => {
                if let Some(payload) = self.memory_ownership_value(module, target)? {
                    Some(payload)
                } else {
                    self.static_term_argument_value(term, 1)?
                }
            }
            dir::LanguageItem::PlaceOf => self.memory_place_value(module, target)?,
            dir::LanguageItem::PlaceOr => {
                if let Some(payload) = self.memory_place_value(module, target)? {
                    Some(payload)
                } else {
                    self.static_term_argument_value(term, 1)?
                }
            }
            dir::LanguageItem::PlaceIn => {
                let Some(space) = self.static_term_static_argument(term, 1)? else {
                    return Ok(None);
                };

                self.memory_place_in_value(module, target, space)?
            }
            dir::LanguageItem::SpaceOf => self.memory_space_value(module, target)?,
            dir::LanguageItem::SpaceOr => {
                if let Some(payload) = self.memory_space_value(module, target)? {
                    Some(payload)
                } else {
                    self.static_term_argument_value(term, 1)?
                }
            }
            dir::LanguageItem::LifetimeOf => self.memory_lifetime_value(module, target)?,
            dir::LanguageItem::LifetimeOr => {
                if let Some(payload) = self.memory_lifetime_value(module, target)? {
                    Some(payload)
                } else {
                    self.static_term_argument_value(term, 1)?
                }
            }
            dir::LanguageItem::AccessOf => self.memory_access_value(module, target)?,
            dir::LanguageItem::AccessOr => {
                if let Some(payload) = self.memory_access_value(module, target)? {
                    Some(payload)
                } else {
                    self.static_term_argument_value(term, 1)?
                }
            }
            dir::LanguageItem::IsManaged
            | dir::LanguageItem::IsOwned
            | dir::LanguageItem::IsBorrowed
            | dir::LanguageItem::IsRaw => {
                self.memory_ownership_predicate_value(module, item, target)?
            }
            dir::LanguageItem::IsShared => self.memory_shared_value(module, target)?,
            dir::LanguageItem::IsSharedIn => {
                let Some(space) = self.static_term_static_argument(term, 1)? else {
                    return Ok(None);
                };

                self.memory_shared_in_value(module, target, space)?
            }
            _ => None,
        };

        Ok(value)
    }

    /// Return one type argument from a stored static intrinsic term.
    fn static_term_type_argument(
        &self,
        term: TermId<StaticTerm>,
        index: usize,
    ) -> Option<TypeOperand> {
        let StaticTerm::Intrinsic { arguments, .. } = self.inference.term(term) else {
            return None;
        };

        arguments.get(index).and_then(GenericArgument::type_operand)
    }

    /// Return one static argument from a stored static intrinsic term.
    fn static_term_static_argument(
        &mut self,
        term: TermId<StaticTerm>,
        index: usize,
    ) -> CompilerResult<Option<StaticOperand>> {
        let Some((r#static, ty)) = self.static_term_argument_operands(term, index) else {
            return Ok(None);
        };
        if let Some(operand) = r#static {
            return Ok(Some(operand));
        }
        let Some(operand) = ty else {
            return Ok(None);
        };

        self.static_operand_from_type(operand)
    }

    /// Return one static argument's solved value from a stored static intrinsic term.
    fn static_term_argument_value(
        &mut self,
        term: TermId<StaticTerm>,
        index: usize,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let Some(operand) = self.static_term_static_argument(term, index)? else {
            return Ok(None);
        };

        self.static_literal(operand)
    }

    /// Return both possible interpretations of one stored generic argument.
    fn static_term_argument_operands(
        &self,
        term: TermId<StaticTerm>,
        index: usize,
    ) -> Option<(Option<StaticOperand>, Option<TypeOperand>)> {
        let StaticTerm::Intrinsic { arguments, .. } = self.inference.term(term) else {
            return None;
        };
        let argument = arguments.get(index)?;

        Some((argument.static_operand(), argument.type_operand()))
    }

    /// Return the outer ownership value.
    fn memory_ownership_value(
        &mut self,
        module: ModuleId,
        target: TypeOperand,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let Some(target) = self.type_operand_term_id(target)? else {
            return Ok(None);
        };
        let value = match self.inference.term(target) {
            TypeTerm::Form { form, payload } => match self.inference.term(*form) {
                FormTerm::Managed => Some(self.ownership_static(module, "managed")?),
                FormTerm::Owned => Some(self.ownership_static(module, "owned")?),
                FormTerm::Borrowed { .. } => Some(self.ownership_static(module, "borrowed")?),
                FormTerm::Raw => Some(self.ownership_static(module, "raw")?),
                FormTerm::Placed { .. } | FormTerm::Readonly => {
                    self.memory_ownership_value(module, *payload)?
                }
            },
            _ => None,
        };

        Ok(value)
    }

    /// Return the outer placement value.
    fn memory_place_value(
        &mut self,
        module: ModuleId,
        target: TypeOperand,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let Some(target) = self.type_operand_term_id(target)? else {
            return Ok(None);
        };
        let value = match self.inference.term(target) {
            TypeTerm::Form { form, payload } => match self.inference.term(*form) {
                FormTerm::Placed { place } => self.static_literal(*place)?,
                FormTerm::Readonly => self.memory_place_value(module, *payload)?,
                FormTerm::Managed | FormTerm::Owned | FormTerm::Borrowed { .. } | FormTerm::Raw => {
                    None
                }
            },
            _ => None,
        };

        Ok(value)
    }

    /// Return a placement resolved within one concrete space.
    fn memory_place_in_value(
        &mut self,
        module: ModuleId,
        target: TypeOperand,
        space: StaticOperand,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let Some(space) = self.space_value(module, space)? else {
            return Ok(None);
        };
        let place = match self.memory_place_value(module, target)? {
            Some(term) => match self.place_from_static(&term, module)? {
                Some(dir::Place::Ambient) => dir::Place::Space(space),
                Some(place) => place,
                None => return Ok(None),
            },
            None => dir::Place::Space(space),
        };

        Ok(Some(dir::StaticTerm::Place { place }))
    }

    /// Return the concrete space value.
    fn memory_space_value(
        &mut self,
        module: ModuleId,
        target: TypeOperand,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let value = match self.memory_place_value(module, target)? {
            Some(term) => match self.place_from_static(&term, module)? {
                Some(dir::Place::Space(space)) => Some(dir::StaticTerm::Space { space }),
                Some(dir::Place::Ambient) | None => None,
            },
            None => None,
        };

        Ok(value)
    }

    /// Return the borrow lifetime value.
    fn memory_lifetime_value(
        &mut self,
        module: ModuleId,
        target: TypeOperand,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let Some(target) = self.type_operand_term_id(target)? else {
            return Ok(None);
        };
        let value = match self.inference.term(target) {
            TypeTerm::Form { form, payload } => match self.inference.term(*form) {
                FormTerm::Borrowed { lifetime, .. } => self.static_literal(*lifetime)?,
                FormTerm::Placed { .. } | FormTerm::Readonly => {
                    self.memory_lifetime_value(module, *payload)?
                }
                FormTerm::Managed | FormTerm::Owned | FormTerm::Raw => None,
            },
            _ => None,
        };

        Ok(value)
    }

    /// Return the borrow access value.
    fn memory_access_value(
        &mut self,
        module: ModuleId,
        target: TypeOperand,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let Some(target) = self.type_operand_term_id(target)? else {
            return Ok(None);
        };
        let value = match self.inference.term(target) {
            TypeTerm::Form { form, payload } => match self.inference.term(*form) {
                FormTerm::Borrowed { access, .. } => self.static_literal(*access)?,
                FormTerm::Placed { .. } | FormTerm::Readonly => {
                    self.memory_access_value(module, *payload)?
                }
                FormTerm::Managed | FormTerm::Owned | FormTerm::Raw => None,
            },
            _ => None,
        };

        Ok(value)
    }

    /// Return whether the outer ownership matches one predicate.
    fn memory_ownership_predicate_value(
        &mut self,
        module: ModuleId,
        item: dir::LanguageItem,
        target: TypeOperand,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let Some(ownership) = self.memory_ownership_value(module, target)? else {
            return Ok(Some(self.boolean_static(false)));
        };
        let Some(ownership) = self.ownership_name(&ownership, module)? else {
            return Ok(None);
        };
        let expected = match item {
            dir::LanguageItem::IsManaged => "managed",
            dir::LanguageItem::IsOwned => "owned",
            dir::LanguageItem::IsBorrowed => "borrowed",
            dir::LanguageItem::IsRaw => "raw",
            _ => return Ok(None),
        };

        Ok(Some(self.boolean_static(ownership == expected)))
    }

    /// Return whether a type is explicitly shared.
    fn memory_shared_value(
        &mut self,
        module: ModuleId,
        target: TypeOperand,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let value = match self.memory_space_value(module, target)? {
            Some(term) => self
                .space_from_static(&term, module)?
                .map(|space| self.boolean_static(space == dir::Space::Shared)),
            None => Some(self.boolean_static(false)),
        };

        Ok(value)
    }

    /// Return whether a type resolves to shared in one space.
    fn memory_shared_in_value(
        &mut self,
        module: ModuleId,
        target: TypeOperand,
        space: StaticOperand,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let Some(place) = self.memory_place_in_value(module, target, space)? else {
            return Ok(None);
        };
        let value = match self.place_from_static(&place, module)? {
            Some(dir::Place::Space(space)) => self.boolean_static(space == dir::Space::Shared),
            Some(dir::Place::Ambient) | None => return Ok(None),
        };

        Ok(Some(value))
    }

    /// Return one static operand's solved literal value.
    pub(in crate::check) fn static_literal(
        &mut self,
        operand: StaticOperand,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let Some(term) = self.static_operand_term_id(operand) else {
            return Ok(None);
        };
        let value = match self.inference.term(term) {
            StaticTerm::Static(_) => None,
            StaticTerm::Literal(payload) => Some(payload.clone()),
            StaticTerm::Parameter(_)
            | StaticTerm::Expression(_)
            | StaticTerm::Member { .. }
            | StaticTerm::Union { .. }
            | StaticTerm::Layout(_)
            | StaticTerm::Intrinsic { .. }
            | StaticTerm::Equal { .. }
            | StaticTerm::TypeRelation { .. }
            | StaticTerm::Conditional { .. } => None,
        };

        Ok(value)
    }

    /// Return one solved ownership spelling.
    fn ownership_value(
        &mut self,
        module: ModuleId,
        operand: StaticOperand,
    ) -> CompilerResult<Option<String>> {
        let Some(term) = self.static_literal(operand)? else {
            return Ok(None);
        };
        let module = operand
            .variable()
            .map(|variable| variable.module)
            .unwrap_or(module);
        let value = self.ownership_name(&term, module)?;
        let value = value.map(ToOwned::to_owned);

        Ok(value)
    }

    /// Return one place value.
    fn place_value(
        &mut self,
        module: ModuleId,
        operand: StaticOperand,
    ) -> CompilerResult<Option<dir::Place>> {
        let Some(term) = self.static_literal(operand)? else {
            return Ok(None);
        };
        let module = operand
            .variable()
            .map(|variable| variable.module)
            .unwrap_or(module);

        self.place_from_static(&term, module)
    }

    /// Return one space value.
    fn space_value(
        &mut self,
        module: ModuleId,
        operand: StaticOperand,
    ) -> CompilerResult<Option<dir::Space>> {
        let Some(term) = self.static_literal(operand)? else {
            return Ok(None);
        };
        let module = operand
            .variable()
            .map(|variable| variable.module)
            .unwrap_or(module);

        self.space_from_static(&term, module)
    }

    /// Return one place value from a static term.
    fn place_from_static(
        &self,
        term: &dir::StaticTerm,
        module: ModuleId,
    ) -> CompilerResult<Option<dir::Place>> {
        let place = match term {
            dir::StaticTerm::Place { place } => Some(*place),
            dir::StaticTerm::Space { space } => Some(dir::Place::Space(*space)),
            dir::StaticTerm::ScalarLiteral {
                value: dir::ScalarLiteral::String(string),
            } => match self.module(module).strings.get(*string) {
                "ambient" => Some(dir::Place::Ambient),
                "local" => Some(dir::Place::Space(dir::Space::Local)),
                "shared" => Some(dir::Place::Space(dir::Space::Shared)),
                "static" => Some(dir::Place::Space(dir::Space::Static)),
                "frame" => Some(dir::Place::Space(dir::Space::Frame)),
                _ => None,
            },
            _ => None,
        };

        Ok(place)
    }

    /// Return one space value from a static term.
    fn space_from_static(
        &self,
        term: &dir::StaticTerm,
        module: ModuleId,
    ) -> CompilerResult<Option<dir::Space>> {
        let space = match term {
            dir::StaticTerm::Space { space } => Some(*space),
            dir::StaticTerm::Place {
                place: dir::Place::Space(space),
            } => Some(*space),
            dir::StaticTerm::ScalarLiteral {
                value: dir::ScalarLiteral::String(string),
            } => self.space_from_string_in(module, *string)?,
            _ => None,
        };

        Ok(space)
    }

    /// Return one ownership spelling from a static term.
    fn ownership_name<'a>(
        &'a self,
        term: &dir::StaticTerm,
        module: ModuleId,
    ) -> CompilerResult<Option<&'a str>> {
        let name = match term {
            dir::StaticTerm::ScalarLiteral {
                value: dir::ScalarLiteral::String(string),
            } => Some(self.module(module).strings.get(*string)),
            _ => None,
        };

        Ok(name)
    }

    /// Return one space from a module string.
    fn space_from_string_in(
        &self,
        module: ModuleId,
        string: dir::StringId,
    ) -> CompilerResult<Option<dir::Space>> {
        let space = match self.module(module).strings.get(string) {
            "local" => Some(dir::Space::Local),
            "shared" => Some(dir::Space::Shared),
            "static" => Some(dir::Space::Static),
            "frame" => Some(dir::Space::Frame),
            _ => None,
        };

        Ok(space)
    }

    /// Return one static ownership label.
    fn ownership_static(&self, module: ModuleId, name: &str) -> CompilerResult<dir::StaticTerm> {
        let string = self.module(module).strings.intern(name);

        Ok(dir::StaticTerm::ScalarLiteral {
            value: dir::ScalarLiteral::String(string),
        })
    }

    /// Return one boolean static value.
    fn boolean_static(&self, value: bool) -> dir::StaticTerm {
        dir::StaticTerm::ScalarLiteral {
            value: dir::ScalarLiteral::Boolean(value),
        }
    }

    /// Return the mutable access operand.
    fn mutable_access_operand(&mut self) -> StaticOperand {
        let term = StaticTerm::Literal(dir::StaticTerm::Access {
            access: dir::Access::Mutable,
        });
        let term = self.inference.push_term(term);

        term.into()
    }
}
