use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    ArgumentTerm, CheckState, ConstraintOrigin, FormTerm, StaticTerm, TermId, TypeTerm, VariableId,
};

impl CheckState<'_> {
    pub(in crate::check) fn reduce_memory_term(
        &mut self,
        module: ModuleId,
        item: dir::LanguageItem,
        arguments: &[TermId<ArgumentTerm>],
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(target) = self.generic_argument_type_variable(arguments, 0) else {
            return Ok(None);
        };
        let origin = self.variable_origin(target)?;
        let Some(term) = (match item {
            dir::LanguageItem::PayloadOf => self.memory_payload_type(target)?,
            dir::LanguageItem::BaseOf => self.memory_base_type(module, target)?,
            dir::LanguageItem::WithBase => {
                let Some(base) = self.generic_argument_type_variable(arguments, 1) else {
                    return Ok(None);
                };

                self.memory_with_base_type(module, origin, target, base)?
            }
            dir::LanguageItem::WithPlace => {
                let Some(place) = self.generic_argument_static_variable(arguments, 1) else {
                    return Ok(None);
                };
                let Some(place) = self.place_value(place)? else {
                    return Ok(None);
                };
                let place = self.solve_anonymous_static_value(
                    module,
                    origin,
                    dir::StaticTerm::Place { place },
                )?;

                self.memory_with_place_type(module, origin, target, place)?
            }
            dir::LanguageItem::WithSpace => {
                let Some(space) = self.generic_argument_static_variable(arguments, 1) else {
                    return Ok(None);
                };
                let Some(space) = self.space_value(space)? else {
                    return Ok(None);
                };
                let place = self.solve_anonymous_static_value(
                    module,
                    origin,
                    dir::StaticTerm::Place {
                        place: dir::Place::Space(space),
                    },
                )?;

                self.memory_with_place_type(module, origin, target, place)?
            }
            dir::LanguageItem::WithLifetime => {
                let Some(lifetime) = self.generic_argument_static_variable(arguments, 1) else {
                    return Ok(None);
                };

                self.memory_with_lifetime_type(module, origin, target, lifetime)?
            }
            dir::LanguageItem::WithAccess => {
                let Some(access) = self.generic_argument_static_variable(arguments, 1) else {
                    return Ok(None);
                };

                self.memory_with_access_type(module, origin, target, access)?
            }
            dir::LanguageItem::WithOwnership => {
                let Some(ownership) = self.generic_argument_static_variable(arguments, 1) else {
                    return Ok(None);
                };
                let Some(lifetime) = self.generic_argument_static_variable(arguments, 2) else {
                    return Ok(None);
                };

                self.memory_with_ownership_type(module, origin, target, ownership, lifetime)?
            }
            _ => return Ok(None),
        }) else {
            return Ok(None);
        };

        Ok(Some(term))
    }

    /// Return one type argument.
    pub(in crate::check) fn generic_argument_type_variable(
        &self,
        arguments: &[TermId<ArgumentTerm>],
        index: usize,
    ) -> Option<VariableId> {
        arguments
            .get(index)
            .and_then(|argument| self.argument_type_variable(*argument))
    }

    /// Return one static argument.
    pub(in crate::check) fn generic_argument_static_variable(
        &self,
        arguments: &[TermId<ArgumentTerm>],
        index: usize,
    ) -> Option<VariableId> {
        arguments
            .get(index)
            .and_then(|argument| self.argument_static_variable(*argument))
    }

    /// Return the immediate payload under one memory form.
    fn memory_payload_type(&mut self, target: VariableId) -> CompilerResult<Option<TypeTerm>> {
        let Some(target) = self.solved_type_term(target)? else {
            return Ok(None);
        };
        let term = match target {
            TypeTerm::Form { payload, .. } => TypeTerm::Variable(payload),
            _ => target,
        };

        Ok(Some(term))
    }

    /// Return the unqualified base under all memory forms.
    fn memory_base_type(
        &mut self,
        module: ModuleId,
        target: VariableId,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(target) = self.solved_type_term(target)? else {
            return Ok(None);
        };
        let term = match target {
            TypeTerm::Form { payload, .. } => {
                let Some(term) = self.memory_base_type(module, payload)? else {
                    return Ok(None);
                };

                term
            }
            _ => target,
        };

        Ok(Some(term))
    }

    /// Replace the unqualified base under memory forms.
    fn memory_with_base_type(
        &mut self,
        module: ModuleId,
        origin: ConstraintOrigin,
        target: VariableId,
        base: VariableId,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(target) = self.solved_type_term(target)? else {
            return Ok(None);
        };
        let term = match target {
            TypeTerm::Form { form, payload } => {
                let Some(payload) = self.memory_with_base_type(module, origin, payload, base)?
                else {
                    return Ok(None);
                };
                let payload = self.solve_anonymous_type(module, origin, payload)?;

                TypeTerm::Form { form, payload }
            }
            _ => TypeTerm::Variable(base),
        };

        Ok(Some(term))
    }

    /// Rewrite the ownership form inside placement wrappers.
    fn memory_with_ownership_type(
        &mut self,
        module: ModuleId,
        origin: ConstraintOrigin,
        target: VariableId,
        ownership: VariableId,
        lifetime: VariableId,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(target) = self.solved_type_term(target)? else {
            return Ok(None);
        };
        let Some(form) = self.ownership_form(ownership, lifetime)? else {
            return Ok(None);
        };
        let term = match target {
            TypeTerm::Form {
                form: wrapper,
                payload,
            } if matches!(
                self.terms.get(wrapper),
                FormTerm::Placed { .. } | FormTerm::Readonly
            ) => {
                let Some(payload) =
                    self.memory_with_ownership_type(module, origin, payload, ownership, lifetime)?
                else {
                    return Ok(None);
                };
                let payload = self.solve_anonymous_type(module, origin, payload)?;

                TypeTerm::Form {
                    form: wrapper,
                    payload,
                }
            }
            TypeTerm::Form { payload, .. } => TypeTerm::Form { form, payload },
            _ => TypeTerm::Form {
                form,
                payload: self.solve_anonymous_type(module, origin, target)?,
            },
        };

        Ok(Some(term))
    }

    /// Rewrite the placement form around a type.
    fn memory_with_place_type(
        &mut self,
        module: ModuleId,
        origin: ConstraintOrigin,
        target: VariableId,
        place: VariableId,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(target) = self.solved_type_term(target)? else {
            return Ok(None);
        };
        let payload = match target {
            TypeTerm::Form { form, payload }
                if matches!(self.terms.get(form), FormTerm::Placed { .. }) =>
            {
                payload
            }
            _ => self.solve_anonymous_type(module, origin, target)?,
        };

        let form = self.terms.push(FormTerm::Placed { place });

        Ok(Some(TypeTerm::Form { form, payload }))
    }

    /// Rewrite the borrow lifetime inside a type.
    fn memory_with_lifetime_type(
        &mut self,
        module: ModuleId,
        origin: ConstraintOrigin,
        target: VariableId,
        lifetime: VariableId,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(target) = self.solved_type_term(target)? else {
            return Ok(None);
        };
        let term = match target {
            TypeTerm::Form { form, payload } => match self.terms.get(form).clone() {
                FormTerm::Borrowed { access, .. } => {
                    let form = self.terms.push(FormTerm::Borrowed { lifetime, access });

                    TypeTerm::Form { form, payload }
                }
                FormTerm::Placed { .. } | FormTerm::Readonly => {
                    let Some(payload) =
                        self.memory_with_lifetime_type(module, origin, payload, lifetime)?
                    else {
                        return Ok(None);
                    };
                    let payload = self.solve_anonymous_type(module, origin, payload)?;

                    TypeTerm::Form { form, payload }
                }
                FormTerm::Managed | FormTerm::Owned | FormTerm::Raw => target,
            },
            _ => target,
        };

        Ok(Some(term))
    }

    /// Rewrite the borrow access inside a type.
    fn memory_with_access_type(
        &mut self,
        module: ModuleId,
        origin: ConstraintOrigin,
        target: VariableId,
        access: VariableId,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(target) = self.solved_type_term(target)? else {
            return Ok(None);
        };
        let term = match target {
            TypeTerm::Form { form, payload } => match self.terms.get(form).clone() {
                FormTerm::Borrowed { lifetime, .. } => {
                    let form = self.terms.push(FormTerm::Borrowed { lifetime, access });

                    TypeTerm::Form { form, payload }
                }
                FormTerm::Placed { .. } | FormTerm::Readonly => {
                    let Some(payload) =
                        self.memory_with_access_type(module, origin, payload, access)?
                    else {
                        return Ok(None);
                    };
                    let payload = self.solve_anonymous_type(module, origin, payload)?;

                    TypeTerm::Form { form, payload }
                }
                FormTerm::Managed | FormTerm::Owned | FormTerm::Raw => target,
            },
            _ => target,
        };

        Ok(Some(term))
    }

    /// Return an ownership form from a static ownership value.
    fn ownership_form(
        &mut self,
        ownership: VariableId,
        lifetime: VariableId,
    ) -> CompilerResult<Option<TermId<FormTerm>>> {
        let origin = self.variable_origin(ownership)?;
        let Some(ownership) = self.ownership_value(ownership)? else {
            return Ok(None);
        };
        let form = match ownership.as_str() {
            "managed" => FormTerm::Managed,
            "owned" => FormTerm::Owned,
            "borrowed" => FormTerm::Borrowed {
                lifetime,
                access: self.mutable_access_variable(lifetime.module, origin)?,
            },
            "raw" => FormTerm::Raw,
            _ => return Ok(None),
        };

        Ok(Some(self.terms.push(form)))
    }

    /// Return a memory intrinsic static value.
    pub(in crate::check) fn memory_static_value(
        &self,
        module: ModuleId,
        item: dir::LanguageItem,
        arguments: &[TermId<ArgumentTerm>],
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let Some(target) = self.generic_argument_type_variable(arguments, 0) else {
            return Ok(None);
        };
        let value = match item {
            dir::LanguageItem::OwnershipOf => self.memory_ownership_value(target)?,
            dir::LanguageItem::OwnershipOr => {
                if let Some(payload) = self.memory_ownership_value(target)? {
                    Some(payload)
                } else {
                    self.static_argument_value(arguments, 1)?
                }
            }
            dir::LanguageItem::PlaceOf => self.memory_place_value(module, target)?,
            dir::LanguageItem::PlaceOr => {
                if let Some(payload) = self.memory_place_value(module, target)? {
                    Some(payload)
                } else {
                    self.static_argument_value(arguments, 1)?
                }
            }
            dir::LanguageItem::PlaceIn => {
                let Some(space) = self.generic_argument_static_variable(arguments, 1) else {
                    return Ok(None);
                };

                self.memory_place_in_value(module, target, space)?
            }
            dir::LanguageItem::SpaceOf => self.memory_space_value(module, target)?,
            dir::LanguageItem::SpaceOr => {
                if let Some(payload) = self.memory_space_value(module, target)? {
                    Some(payload)
                } else {
                    self.static_argument_value(arguments, 1)?
                }
            }
            dir::LanguageItem::LifetimeOf => self.memory_lifetime_value(module, target)?,
            dir::LanguageItem::LifetimeOr => {
                if let Some(payload) = self.memory_lifetime_value(module, target)? {
                    Some(payload)
                } else {
                    self.static_argument_value(arguments, 1)?
                }
            }
            dir::LanguageItem::AccessOf => self.memory_access_value(module, target)?,
            dir::LanguageItem::AccessOr => {
                if let Some(payload) = self.memory_access_value(module, target)? {
                    Some(payload)
                } else {
                    self.static_argument_value(arguments, 1)?
                }
            }
            dir::LanguageItem::IsManaged
            | dir::LanguageItem::IsOwned
            | dir::LanguageItem::IsBorrowed
            | dir::LanguageItem::IsRaw => self.memory_ownership_predicate_value(item, target)?,
            dir::LanguageItem::IsShared => self.memory_shared_value(module, target)?,
            dir::LanguageItem::IsSharedIn => {
                let Some(space) = self.generic_argument_static_variable(arguments, 1) else {
                    return Ok(None);
                };

                self.memory_shared_in_value(module, target, space)?
            }
            _ => None,
        };

        Ok(value)
    }

    /// Return the outer ownership value.
    fn memory_ownership_value(
        &self,
        target: VariableId,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let module = target.module;
        let Some(target) = self.solved_type_term(target)? else {
            return Ok(None);
        };
        let value = match target {
            TypeTerm::Form { form, payload } => match self.terms.get(form) {
                FormTerm::Managed => Some(self.ownership_static(module, "managed")?),
                FormTerm::Owned => Some(self.ownership_static(module, "owned")?),
                FormTerm::Borrowed { .. } => Some(self.ownership_static(module, "borrowed")?),
                FormTerm::Raw => Some(self.ownership_static(module, "raw")?),
                FormTerm::Placed { .. } | FormTerm::Readonly => {
                    self.memory_ownership_value(payload)?
                }
            },
            _ => None,
        };

        Ok(value)
    }

    /// Return the outer placement value.
    fn memory_place_value(
        &self,
        module: ModuleId,
        target: VariableId,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let Some(target) = self.solved_type_term(target)? else {
            return Ok(None);
        };
        let value = match target {
            TypeTerm::Form { form, payload } => match self.terms.get(form) {
                FormTerm::Placed { place } => self.static_value(*place)?,
                FormTerm::Readonly => self.memory_place_value(module, payload)?,
                FormTerm::Managed | FormTerm::Owned | FormTerm::Borrowed { .. } | FormTerm::Raw => None,
            },
            _ => None,
        };

        Ok(value)
    }

    /// Return a placement resolved within one concrete space.
    fn memory_place_in_value(
        &self,
        module: ModuleId,
        target: VariableId,
        space: VariableId,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let Some(space) = self.space_value(space)? else {
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
        &self,
        module: ModuleId,
        target: VariableId,
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
        &self,
        module: ModuleId,
        target: VariableId,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let Some(target) = self.solved_type_term(target)? else {
            return Ok(None);
        };
        let value = match target {
            TypeTerm::Form { form, payload } => match self.terms.get(form) {
                FormTerm::Borrowed { lifetime, .. } => self.static_value(*lifetime)?,
                FormTerm::Placed { .. } | FormTerm::Readonly => {
                    self.memory_lifetime_value(module, payload)?
                }
                FormTerm::Managed | FormTerm::Owned | FormTerm::Raw => None,
            },
            _ => None,
        };

        Ok(value)
    }

    /// Return the borrow access value.
    fn memory_access_value(
        &self,
        module: ModuleId,
        target: VariableId,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let Some(target) = self.solved_type_term(target)? else {
            return Ok(None);
        };
        let value = match target {
            TypeTerm::Form { form, payload } => match self.terms.get(form) {
                FormTerm::Borrowed { access, .. } => self.static_value(*access)?,
                FormTerm::Placed { .. } | FormTerm::Readonly => {
                    self.memory_access_value(module, payload)?
                }
                FormTerm::Managed | FormTerm::Owned | FormTerm::Raw => None,
            },
            _ => None,
        };

        Ok(value)
    }

    /// Return whether the outer ownership matches one predicate.
    fn memory_ownership_predicate_value(
        &self,
        item: dir::LanguageItem,
        target: VariableId,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let Some(ownership) = self.memory_ownership_value(target)? else {
            return Ok(Some(self.boolean_static(false)));
        };
        let Some(ownership) = self.ownership_name(&ownership, target.module)? else {
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
        &self,
        module: ModuleId,
        target: VariableId,
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
        &self,
        module: ModuleId,
        target: VariableId,
        space: VariableId,
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

    /// Return one static variable's solved DIR value.
    pub(in crate::check) fn static_value(
        &self,
        variable: VariableId,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let Some(term) = self.solved_static_term(variable)? else {
            return Ok(None);
        };
        let value = match term {
            StaticTerm::Literal(payload) => Some(payload),
            StaticTerm::Variable(_)
            | StaticTerm::Expression(_)
            | StaticTerm::Member { .. }
            | StaticTerm::Join { .. }
            | StaticTerm::Layout(_)
            | StaticTerm::Intrinsic { .. }
            | StaticTerm::Equal { .. }
            | StaticTerm::TypeRelation { .. }
            | StaticTerm::Conditional { .. } => None,
        };

        Ok(value)
    }

    /// Return one static argument's solved value.
    fn static_argument_value(
        &self,
        arguments: &[TermId<ArgumentTerm>],
        index: usize,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let Some(variable) = self.generic_argument_static_variable(arguments, index) else {
            return Ok(None);
        };

        self.static_value(variable)
    }

    /// Return one solved ownership spelling.
    fn ownership_value(&self, variable: VariableId) -> CompilerResult<Option<String>> {
        let Some(term) = self.static_value(variable)? else {
            return Ok(None);
        };
        let value = self.ownership_name(&term, variable.module)?;
        let value = value.map(ToOwned::to_owned);

        Ok(value)
    }

    /// Return one normalized place value.
    fn place_value(&self, variable: VariableId) -> CompilerResult<Option<dir::Place>> {
        let Some(term) = self.static_value(variable)? else {
            return Ok(None);
        };

        self.place_from_static(&term, variable.module)
    }

    /// Return one normalized space value.
    fn space_value(&self, variable: VariableId) -> CompilerResult<Option<dir::Space>> {
        let Some(term) = self.static_value(variable)? else {
            return Ok(None);
        };

        self.space_from_static(&term, variable.module)
    }

    /// Return one normalized place value from a static term.
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
            } => match self.module(module)?.input.strings.get(*string) {
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

    /// Return one normalized space value from a static term.
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
            } => Some(self.input(module).strings.get(*string)),
            _ => None,
        };

        Ok(name)
    }

    /// Return one normalized space from a module string.
    fn space_from_string_in(
        &self,
        module: ModuleId,
        string: dir::StringId,
    ) -> CompilerResult<Option<dir::Space>> {
        let space = match self.input(module).strings.get(string) {
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
        let string = self.input(module).strings.intern(name);

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

    /// Return a solved mutable access variable.
    fn mutable_access_variable(
        &mut self,
        module: ModuleId,
        origin: ConstraintOrigin,
    ) -> CompilerResult<VariableId> {
        self.solve_anonymous_static_value(
            module,
            origin,
            dir::StaticTerm::Access {
                access: dir::Access::Mutable,
            },
        )
    }
}
