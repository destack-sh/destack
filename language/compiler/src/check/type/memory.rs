use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Dependency, Origin};

/// The memory form wrappers stacked over one base type.
#[derive(Debug, Clone)]
struct FormChain {
    /// The form wrappers, outermost first.
    forms: SmallVec<[dir::FormType; 2]>,
    /// The unqualified base type under every wrapper.
    base: dir::GlobalTypeId,
    /// Whether the base can still gain forms at instantiation.
    is_open: bool,
}

impl CheckState<'_> {
    /// Reduce one memory intrinsic application.
    pub(in crate::check) fn evaluate_intrinsic_reference(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let Some(item) = self.environment.language.item(instance.symbol) else {
            return Ok(Answer::Ready(None));
        };

        match item {
            // collection constructors normalize to their structural views
            dir::LanguageItem::Array => {
                let [element] = instance.arguments.as_slice() else {
                    return Ok(Answer::Ready(None));
                };
                let view = dir::Type::Array(dir::ArrayType { element: *element });

                self.push_intrinsic_view(origin, view)
            }
            dir::LanguageItem::Slice => {
                let [element] = instance.arguments.as_slice() else {
                    return Ok(Answer::Ready(None));
                };
                let view = dir::Type::Slice(dir::SliceType { element: *element });

                self.push_intrinsic_view(origin, view)
            }
            dir::LanguageItem::FixedArray => {
                let [element, count] = instance.arguments.as_slice() else {
                    return Ok(Answer::Ready(None));
                };
                let view = dir::Type::FixedArray(dir::FixedArrayType {
                    element: *element,
                    count: *count,
                });

                self.push_intrinsic_view(origin, view)
            }

            // form constructors normalize to their canonical form written form
            dir::LanguageItem::Managed => {
                self.evaluate_form_constructor(origin, instance, dir::Form::Managed)
            }
            dir::LanguageItem::Owned => {
                self.evaluate_form_constructor(origin, instance, dir::Form::Owned)
            }
            dir::LanguageItem::Raw => {
                self.evaluate_form_constructor(origin, instance, dir::Form::Raw)
            }
            dir::LanguageItem::Readonly => {
                self.evaluate_form_constructor(origin, instance, dir::Form::Readonly)
            }
            dir::LanguageItem::Borrowed => self.evaluate_borrowed_constructor(origin, instance),
            dir::LanguageItem::Placed => self.evaluate_placed_constructor(origin, instance),

            // accessors evaluate over closed form chains
            dir::LanguageItem::PayloadOf
            | dir::LanguageItem::BaseOf
            | dir::LanguageItem::OwnershipOf
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
            | dir::LanguageItem::WithBase
            | dir::LanguageItem::WithOwnership
            | dir::LanguageItem::WithPlace
            | dir::LanguageItem::WithSpace
            | dir::LanguageItem::WithLifetime
            | dir::LanguageItem::WithAccess => {
                self.evaluate_memory_accessor(origin, item, instance)
            }

            _ => Ok(Answer::Ready(None)),
        }
    }

    /// Allocate one normalized intrinsic view.
    fn push_intrinsic_view(
        &mut self,
        origin: Origin,
        view: dir::Type,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let source = self.origin_source_node(origin)?;
        let view = self.push_type(origin.module(), view, source)?;

        Ok(Answer::Ready(Some(view)))
    }

    /// Normalize one unary form constructor application.
    fn evaluate_form_constructor(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
        form: dir::Form,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let Some(value) = instance.arguments.first().copied() else {
            return Ok(Answer::Ready(None));
        };
        let formed = dir::Type::Form(dir::FormType { form, value });
        let id = self.push_memory_answer(origin, formed)?;

        Ok(Answer::Ready(Some(id)))
    }

    /// Normalize one borrowed form constructor application.
    fn evaluate_borrowed_constructor(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let Some(value) = instance.arguments.first().copied() else {
            return Ok(Answer::Ready(None));
        };
        let Some(lifetime) = instance.arguments.get(1).copied() else {
            return Ok(Answer::Ready(None));
        };

        // missing access arguments default to mutable
        let access = match instance.arguments.get(2).copied() {
            Some(access) => self.normalize_access(origin, access)?,
            None => {
                self.push_memory_literal(origin, dir::MemoryLiteral::Access(dir::Access::Mutable))?
            }
        };
        let lifetime = self.normalize_lifetime(origin, lifetime)?;
        let formed = dir::Type::Form(dir::FormType {
            form: dir::Form::Borrowed { lifetime, access },
            value,
        });
        let id = self.push_memory_answer(origin, formed)?;

        Ok(Answer::Ready(Some(id)))
    }

    /// Normalize one placed form constructor application.
    fn evaluate_placed_constructor(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let Some(value) = instance.arguments.first().copied() else {
            return Ok(Answer::Ready(None));
        };
        let Some(place) = instance.arguments.get(1).copied() else {
            return Ok(Answer::Ready(None));
        };

        let place = self.normalize_place(origin, place)?;
        let formed = dir::Type::Form(dir::FormType {
            form: dir::Form::Placed { place },
            value,
        });
        let id = self.push_memory_answer(origin, formed)?;

        Ok(Answer::Ready(Some(id)))
    }

    /// Evaluate one memory accessor, distributing over union targets.
    fn evaluate_memory_accessor(
        &mut self,
        origin: Origin,
        item: dir::LanguageItem,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let Some(target) = instance.arguments.first().copied() else {
            return Ok(Answer::Ready(None));
        };

        // close the inspected target first
        let target = match self.evaluate_root(origin, target)? {
            Answer::Ready(target) => target,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        // distribute the accessor over union targets
        let elements = match self.ty(target)? {
            dir::Type::Union(union) => union.elements.iter().copied().collect::<SmallVec<[_; 4]>>(),
            _ => {
                let mut single = SmallVec::new();
                single.push(target);

                single
            }
        };
        let mut answers = Vec::with_capacity(elements.len());
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        for element in elements {
            let element = match self.evaluate_root(origin, element)? {
                Answer::Ready(element) => element,
                Answer::Pending(dependencies) => {
                    blockers.extend(dependencies);

                    continue;
                }
            };

            match self.evaluate_element_accessor(origin, item, instance, element)? {
                // one symbolic element keeps the whole accessor symbolic
                Answer::Ready(None) => return Ok(Answer::Ready(None)),
                Answer::Ready(Some(answer)) => answers.push(answer),
                Answer::Pending(dependencies) => {
                    blockers.extend(dependencies);

                    continue;
                }
            }
        }
        if !blockers.is_empty() {
            return Ok(Answer::pending(blockers));
        }

        // join element answers, dropping never like any union would
        let mut kept = Vec::with_capacity(answers.len());
        for answer in answers {
            if matches!(self.ty(answer)?, dir::Type::Never) {
                continue;
            }
            if !kept.contains(&answer) {
                kept.push(answer);
            }
        }
        let joined = match kept.as_slice() {
            [] => self.push_memory_answer(origin, dir::Type::Never)?,
            [single] => *single,
            _ => self
                .push_memory_answer(origin, dir::Type::Union(dir::UnionType { elements: kept }))?,
        };

        Ok(Answer::Ready(Some(joined)))
    }

    /// Evaluate one memory accessor over one closed element.
    /// Returns none while the element's forms stay symbolic.
    fn evaluate_element_accessor(
        &mut self,
        origin: Origin,
        item: dir::LanguageItem,
        instance: &dir::GenericInstance,
        element: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        // close the element's form chain first
        let chain = match self.form_chain(origin, element)? {
            Answer::Ready(chain) => chain,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let answer = self.evaluate_stack_accessor(origin, item, instance, element, &chain)?;

        Ok(Answer::Ready(answer))
    }

    /// Evaluate one memory accessor over one closed form chain.
    fn evaluate_stack_accessor(
        &mut self,
        origin: Origin,
        item: dir::LanguageItem,
        instance: &dir::GenericInstance,
        element: dir::GlobalTypeId,
        chain: &FormChain,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        match item {
            // PayloadOf<T> removes one outer form
            dir::LanguageItem::PayloadOf => match chain.forms.first() {
                Some(outer) => Ok(Some(outer.value)),
                None if chain.is_open => Ok(None),
                None => Ok(Some(chain.base)),
            },
            // BaseOf<T> removes every form
            dir::LanguageItem::BaseOf => {
                if chain.is_open {
                    Ok(None)
                } else {
                    Ok(Some(chain.base))
                }
            }

            // ownership axis
            dir::LanguageItem::OwnershipOf => self.ownership_answer(origin, chain),
            dir::LanguageItem::OwnershipOr => {
                let answer = self.ownership_answer(origin, chain)?;

                self.fallback_answer(origin, instance, answer)
            }
            dir::LanguageItem::IsManaged => {
                self.ownership_predicate(origin, chain, dir::Form::Managed)
            }
            dir::LanguageItem::IsOwned => self.ownership_predicate(origin, chain, dir::Form::Owned),
            dir::LanguageItem::IsBorrowed => {
                let found = self.chain_ownership(chain);
                match found {
                    Some(form) => {
                        self.boolean_answer(origin, matches!(form, dir::Form::Borrowed { .. }))
                    }
                    None if chain.is_open => Ok(None),
                    None => self.boolean_answer(origin, false),
                }
            }
            dir::LanguageItem::IsRaw => self.ownership_predicate(origin, chain, dir::Form::Raw),

            // access axis
            dir::LanguageItem::AccessOf => self.access_answer(origin, chain),
            dir::LanguageItem::AccessOr => {
                let answer = self.access_answer(origin, chain)?;

                self.fallback_answer(origin, instance, answer)
            }

            // placement axis
            dir::LanguageItem::PlaceOf => self.place_answer(origin, chain),
            dir::LanguageItem::PlaceOr => {
                let answer = self.place_answer(origin, chain)?;

                self.fallback_answer(origin, instance, answer)
            }
            dir::LanguageItem::PlaceIn => {
                let answer = self.place_answer(origin, chain)?;
                let Some(answer) = answer else {
                    return Ok(None);
                };

                // ambient placement resolves to the given concrete space
                if self.is_text_literal(answer, "ambient")? {
                    let Some(space) = instance.arguments.get(1).copied() else {
                        return Ok(None);
                    };

                    Ok(Some(self.normalize_axis_text(origin, space)?))
                } else {
                    Ok(Some(answer))
                }
            }
            dir::LanguageItem::SpaceOf => self.space_answer(origin, chain),
            dir::LanguageItem::SpaceOr => {
                let answer = self.space_answer(origin, chain)?;

                self.fallback_answer(origin, instance, answer)
            }
            dir::LanguageItem::IsShared => {
                let answer = self.space_answer(origin, chain)?;
                let Some(answer) = answer else {
                    return Ok(None);
                };
                let is_shared = self.is_text_literal(answer, "shared")?;

                self.boolean_answer(origin, is_shared)
            }
            dir::LanguageItem::IsSharedIn => {
                let place = self.place_answer(origin, chain)?;
                let Some(place) = place else {
                    return Ok(None);
                };

                // ambient resolves to the given concrete space first
                let resolved = if self.is_text_literal(place, "ambient")? {
                    let Some(space) = instance.arguments.get(1).copied() else {
                        return Ok(None);
                    };

                    self.normalize_axis_text(origin, space)?
                } else {
                    place
                };
                let is_shared = self.is_text_literal(resolved, "shared")?;

                self.boolean_answer(origin, is_shared)
            }

            // lifetime axis
            dir::LanguageItem::LifetimeOf => self.lifetime_answer(origin, chain),
            dir::LanguageItem::LifetimeOr => {
                let answer = self.lifetime_answer(origin, chain)?;

                self.fallback_answer(origin, instance, answer)
            }

            // rewriting helpers
            dir::LanguageItem::WithBase => {
                if chain.is_open {
                    return Ok(None);
                }
                let Some(base) = instance.arguments.get(1).copied() else {
                    return Ok(None);
                };

                Ok(Some(self.wrap_forms(origin, &chain.forms, base)?))
            }
            dir::LanguageItem::WithOwnership => {
                self.with_ownership_answer(origin, instance, element)
            }
            dir::LanguageItem::WithPlace | dir::LanguageItem::WithSpace => {
                self.with_place_answer(origin, instance, chain, element)
            }
            dir::LanguageItem::WithLifetime => {
                let Some(lifetime) = instance.arguments.get(1).copied() else {
                    return Ok(None);
                };
                let lifetime = self.normalize_lifetime(origin, lifetime)?;

                self.replace_borrow(origin, chain, Some(lifetime), None)
            }
            dir::LanguageItem::WithAccess => self.with_access_answer(origin, instance, chain),

            _ => Ok(None),
        }
    }

    /// Walk one element's form chain down to its unqualified base.
    fn form_chain(
        &mut self,
        origin: Origin,
        element: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<FormChain>> {
        let mut forms = SmallVec::new();
        let mut current = element;

        // collect form wrappers outermost first, closing each payload
        loop {
            current = match self.evaluate_root(origin, current)? {
                Answer::Ready(current) => current,
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            };
            let dir::Type::Form(form) = self.ty(current)? else {
                break;
            };

            forms.push(dir::FormType {
                form: form.form,
                value: form.value,
            });
            current = form.value;
        }

        // open bases can gain forms when their parameters substitute
        let is_open = matches!(
            self.ty(current)?,
            dir::Type::Parameter(_)
                | dir::Type::Variable(_)
                | dir::Type::This
                | dir::Type::Member(_)
                | dir::Type::Operation(_)
        );

        Ok(Answer::Ready(FormChain {
            forms,
            base: current,
            is_open,
        }))
    }

    /// Return the outermost ownership form on one chain.
    fn chain_ownership(&self, chain: &FormChain) -> Option<dir::Form> {
        chain.forms.iter().map(|entry| entry.form).find(|form| {
            matches!(
                form,
                dir::Form::Managed | dir::Form::Owned | dir::Form::Borrowed { .. } | dir::Form::Raw
            )
        })
    }

    /// Return one chain's ownership kind as a literal answer.
    fn ownership_answer(
        &mut self,
        origin: Origin,
        chain: &FormChain,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        match self.chain_ownership(chain) {
            Some(dir::Form::Managed) => self.text_answer(origin, "managed").map(Some),
            Some(dir::Form::Owned) => self.text_answer(origin, "owned").map(Some),
            Some(dir::Form::Borrowed { .. }) => self.text_answer(origin, "borrowed").map(Some),
            Some(dir::Form::Raw) => self.text_answer(origin, "raw").map(Some),
            // open bases may still gain ownership at instantiation
            Some(_) | None if chain.is_open => Ok(None),
            Some(_) | None => Ok(Some(self.push_memory_answer(origin, dir::Type::Never)?)),
        }
    }

    /// Return whether one chain's ownership matches a form constructor.
    fn ownership_predicate(
        &mut self,
        origin: Origin,
        chain: &FormChain,
        form: dir::Form,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        match self.chain_ownership(chain) {
            Some(found) => self.boolean_answer(
                origin,
                std::mem::discriminant(&found) == std::mem::discriminant(&form),
            ),
            None if chain.is_open => Ok(None),
            None => self.boolean_answer(origin, false),
        }
    }

    /// Return one chain's access mode as a literal answer.
    /// Unqualified concrete chains default to mutable access.
    fn access_answer(
        &mut self,
        origin: Origin,
        chain: &FormChain,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        for entry in &chain.forms {
            match entry.form {
                dir::Form::Readonly => return self.text_answer(origin, "readonly").map(Some),
                dir::Form::Borrowed { access, .. } => {
                    return self.normalize_axis_text(origin, access).map(Some);
                }
                _ => {}
            }
        }

        if chain.is_open {
            Ok(None)
        } else {
            self.text_answer(origin, "mutable").map(Some)
        }
    }

    /// Return one chain's placement as a literal answer.
    /// Unqualified concrete chains stay ambient.
    fn place_answer(
        &mut self,
        origin: Origin,
        chain: &FormChain,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        for entry in &chain.forms {
            if let dir::Form::Placed { place } = entry.form {
                return self.normalize_axis_text(origin, place).map(Some);
            }
        }

        if chain.is_open {
            Ok(None)
        } else {
            self.text_answer(origin, "ambient").map(Some)
        }
    }

    /// Return one chain's explicit concrete space as a literal answer.
    fn space_answer(
        &mut self,
        origin: Origin,
        chain: &FormChain,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let place = self.place_answer(origin, chain)?;
        let Some(place) = place else {
            return Ok(None);
        };

        // ambient placement names no concrete space
        if self.is_text_literal(place, "ambient")? {
            Ok(Some(self.push_memory_answer(origin, dir::Type::Never)?))
        } else {
            Ok(Some(place))
        }
    }

    /// Return one chain's borrow lifetime as a type answer.
    /// Borrows carry their lifetime argument; managed handles answer the
    /// frame lifetime of the automatic root that pins them.
    fn lifetime_answer(
        &mut self,
        origin: Origin,
        chain: &FormChain,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        for entry in &chain.forms {
            match entry.form {
                dir::Form::Borrowed { lifetime, .. } => return Ok(Some(lifetime)),
                dir::Form::Managed => {
                    return Ok(Some(self.push_memory_literal(
                        origin,
                        dir::MemoryLiteral::Lifetime(dir::Lifetime::Frame),
                    )?));
                }
                _ => {}
            }
        }

        if chain.is_open {
            Ok(None)
        } else {
            Ok(Some(self.push_memory_answer(origin, dir::Type::Never)?))
        }
    }

    /// Replace one never answer with the accessor's fallback argument.
    fn fallback_answer(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
        answer: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(answer) = answer else {
            return Ok(None);
        };

        if matches!(self.ty(answer)?, dir::Type::Never) {
            let Some(fallback) = instance.arguments.get(1).copied() else {
                return Ok(None);
            };

            Ok(Some(self.normalize_axis_text(origin, fallback)?))
        } else {
            Ok(Some(answer))
        }
    }

    /// Wrap one target in the requested ownership form.
    fn with_ownership_answer(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
        element: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(ownership) = instance.arguments.get(1).copied() else {
            return Ok(None);
        };
        let Some(text) = self.memory_axis_text(origin, ownership)? else {
            return Ok(None);
        };

        let form = match text.as_str() {
            "managed" => dir::Form::Managed,
            "owned" => dir::Form::Owned,
            "raw" => dir::Form::Raw,
            "borrowed" => {
                // borrowing needs the explicit lifetime argument
                let Some(lifetime) = instance.arguments.get(2).copied() else {
                    return Ok(None);
                };
                let lifetime = self.normalize_lifetime(origin, lifetime)?;
                let access = self.push_memory_literal(
                    origin,
                    dir::MemoryLiteral::Access(dir::Access::Mutable),
                )?;

                dir::Form::Borrowed { lifetime, access }
            }
            _ => return Ok(None),
        };
        let formed = dir::Type::Form(dir::FormType {
            form,
            value: element,
        });

        Ok(Some(self.push_memory_answer(origin, formed)?))
    }

    /// Replace or add one placement wrapper.
    fn with_place_answer(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
        chain: &FormChain,
        element: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(place) = instance.arguments.get(1).copied() else {
            return Ok(None);
        };
        let place = self.normalize_place(origin, place)?;

        // replace an existing placement wrapper in its chain position
        let position = chain
            .forms
            .iter()
            .position(|entry| matches!(entry.form, dir::Form::Placed { .. }));
        match position {
            Some(position) => {
                let mut forms = chain.forms.clone();
                forms[position].form = dir::Form::Placed { place };

                Ok(Some(self.wrap_forms(origin, &forms, chain.base)?))
            }
            // wrap unplaced chains outside
            None => {
                let formed = dir::Type::Form(dir::FormType {
                    form: dir::Form::Placed { place },
                    value: element,
                });

                Ok(Some(self.push_memory_answer(origin, formed)?))
            }
        }
    }

    /// Replace or insert one access wrapper.
    fn with_access_answer(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
        chain: &FormChain,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(access) = instance.arguments.get(1).copied() else {
            return Ok(None);
        };
        let Some(text) = self.memory_axis_text(origin, access)? else {
            return Ok(None);
        };

        // access is a borrow component: WithAccess replaces the access
        // of the chain's borrow and stays symbolic everywhere else
        let _ = text;
        if chain
            .forms
            .iter()
            .any(|entry| matches!(entry.form, dir::Form::Borrowed { .. }))
        {
            let normalized = self.normalize_access(origin, access)?;

            return self.replace_borrow(origin, chain, None, Some(normalized));
        }

        Ok(None)
    }

    /// Rebuild one chain's borrow wrapper with replaced components.
    fn replace_borrow(
        &mut self,
        origin: Origin,
        chain: &FormChain,
        lifetime: Option<dir::GlobalTypeId>,
        access: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let position = chain
            .forms
            .iter()
            .position(|entry| matches!(entry.form, dir::Form::Borrowed { .. }));
        let Some(position) = position else {
            return Ok(None);
        };

        let mut forms = chain.forms.clone();
        if let dir::Form::Borrowed {
            lifetime: old_lifetime,
            access: old_access,
        } = forms[position].form
        {
            forms[position].form = dir::Form::Borrowed {
                lifetime: lifetime.unwrap_or(old_lifetime),
                access: access.unwrap_or(old_access),
            };
        }

        Ok(Some(self.wrap_forms(origin, &forms, chain.base)?))
    }

    /// Rebuild one form chain over a new base, innermost first.
    fn wrap_forms(
        &mut self,
        origin: Origin,
        forms: &[dir::FormType],
        base: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut current = base;

        for entry in forms.iter().rev() {
            current = self.push_memory_answer(
                origin,
                dir::Type::Form(dir::FormType {
                    form: entry.form,
                    value: current,
                }),
            )?;
        }

        Ok(current)
    }

    /// Normalize one access component to its canonical memory literal.
    fn normalize_access(
        &mut self,
        origin: Origin,
        component: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some(text) = self.memory_axis_text(origin, component)? else {
            return Ok(component);
        };
        let access = match text.as_str() {
            "readonly" => dir::Access::Readonly,
            "mutable" => dir::Access::Mutable,
            "exclusive" => dir::Access::Exclusive,
            _ => return Ok(component),
        };

        self.push_memory_literal(origin, dir::MemoryLiteral::Access(access))
    }

    /// Normalize one place component to its canonical memory literal.
    fn normalize_place(
        &mut self,
        origin: Origin,
        component: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some(text) = self.memory_axis_text(origin, component)? else {
            return Ok(component);
        };
        let place = match text.as_str() {
            "ambient" => dir::Place::Ambient,
            "local" => dir::Place::Space(dir::Space::Local),
            "shared" => dir::Place::Space(dir::Space::Shared),
            "static" => dir::Place::Space(dir::Space::Static),
            "frame" => dir::Place::Space(dir::Space::Frame),
            _ => return Ok(component),
        };

        self.push_memory_literal(origin, dir::MemoryLiteral::Place(place))
    }

    /// Normalize one lifetime component to its canonical memory literal.
    fn normalize_lifetime(
        &mut self,
        origin: Origin,
        component: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some(text) = self.memory_axis_text(origin, component)? else {
            return Ok(component);
        };

        if text == "static" {
            self.push_memory_literal(origin, dir::MemoryLiteral::Lifetime(dir::Lifetime::Static))
        } else {
            Ok(component)
        }
    }

    /// Return one axis component as its canonical text written form.
    fn normalize_axis_text(
        &mut self,
        origin: Origin,
        component: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        match self.memory_axis_text(origin, component)? {
            Some(text) => self.text_answer(origin, &text),
            None => Ok(component),
        }
    }

    /// Return the text behind one closed axis component.
    fn memory_axis_text(
        &mut self,
        origin: Origin,
        component: dir::GlobalTypeId,
    ) -> CompilerResult<Option<String>> {
        let component = match self.evaluate_root(origin, component)? {
            Answer::Ready(component) => component,
            Answer::Pending(_) => return Ok(None),
        };

        let text = match self.ty(component)? {
            dir::Type::Literal(dir::ScalarLiteral::String(value)) => {
                let module = origin.module();

                Some(self.module(module).strings.get(*value).to_string())
            }
            dir::Type::Memory(literal) => Some(literal.text().to_string()),
            _ => None,
        };

        Ok(text)
    }

    /// Return whether one answer is a specific string literal.
    fn is_text_literal(&self, answer: dir::GlobalTypeId, text: &str) -> CompilerResult<bool> {
        match self.ty(answer)? {
            dir::Type::Literal(dir::ScalarLiteral::String(value)) => {
                Ok(*value == dir::StringId::for_text(text))
            }
            _ => Ok(false),
        }
    }

    /// Push one string literal answer.
    fn text_answer(&mut self, origin: Origin, text: &str) -> CompilerResult<dir::GlobalTypeId> {
        let module = origin.module();
        let value = self.module_mut(module).strings.intern(text);

        self.push_memory_answer(
            origin,
            dir::Type::Literal(dir::ScalarLiteral::String(value)),
        )
    }

    /// Push one boolean literal answer.
    fn boolean_answer(
        &mut self,
        origin: Origin,
        value: bool,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let answer = self.push_memory_answer(
            origin,
            dir::Type::Literal(dir::ScalarLiteral::Boolean(value)),
        )?;

        Ok(Some(answer))
    }

    /// Push one memory literal written form.
    fn push_memory_literal(
        &mut self,
        origin: Origin,
        literal: dir::MemoryLiteral,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.push_memory_answer(origin, dir::Type::Memory(literal))
    }

    /// Push one accessor answer at the accessor's origin.
    fn push_memory_answer(
        &mut self,
        origin: Origin,
        ty: dir::Type,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let module = origin.module();
        let source = self.origin_source_node(origin)?;

        self.push_type(module, ty, source)
    }
}

impl CheckState<'_> {
    /// Resolve one type to its readable value.
    /// Reads see through managed handles and readonly views; the
    /// payload carries the fields and elements a read consumes.
    pub(in crate::check) fn readable_value(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut current = self.resolve_root(ty)?;
        while let dir::Type::Form(form) = self.ty(current)? {
            // only alias-transparent forms peel for reads
            if !matches!(form.form, dir::Form::Managed | dir::Form::Readonly) {
                break;
            }
            current = self.resolve_root(form.value)?;
        }

        Ok(current)
    }
}
