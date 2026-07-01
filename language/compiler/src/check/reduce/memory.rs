use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Dependency, Origin, answer};

/// Memory forms stacked over one base type.
#[derive(Debug, Clone)]
struct FormChain {
    /// The memory forms, outermost first.
    forms: SmallVec<[dir::FormType; 2]>,
    /// The unqualified base type under every memory form.
    base: dir::GlobalTypeId,
    /// Whether the base can still gain forms at instantiation.
    is_open: bool,
}

/// Borrow required by an implicit autoref.
pub(in crate::check) struct ImplicitBorrow {
    /// The borrowed target type.
    pub(in crate::check) target: dir::GlobalTypeId,
    /// The required borrow lifetime.
    pub(in crate::check) lifetime: dir::GlobalTypeId,
    /// The required borrow access.
    pub(in crate::check) access: dir::GlobalTypeId,
}

impl CheckState<'_> {
    /// Normalize one static value against a memory-domain language item.
    pub(in crate::check) fn normalize_memory_domain_value(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
        domain: dir::LanguageItem,
    ) -> CompilerResult<dir::GlobalTypeId> {
        match domain {
            dir::LanguageItem::Access => self.normalize_access(origin, value),
            dir::LanguageItem::Lifetime => self.normalize_lifetime(origin, value),
            dir::LanguageItem::Place => self.normalize_place(origin, value),
            dir::LanguageItem::Space => self.normalize_space(origin, value),
            _ => Ok(value),
        }
    }

    /// Return whether one explicit ownership form matches its payload's default form.
    pub(in crate::check) fn is_default_ownership_form(
        &mut self,
        origin: Origin,
        form: dir::Form,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        if !matches!(form, dir::Form::Managed | dir::Form::Owned) {
            return Ok(Answer::Ready(false));
        }

        let Some(default) = answer!(self.default_ownership(origin, value)?) else {
            return Ok(Answer::Ready(false));
        };
        let matches_default = std::mem::discriminant(&default) == std::mem::discriminant(&form);

        Ok(Answer::Ready(matches_default))
    }

    /// Reduce one unary form constructor application.
    pub(in crate::check) fn reduce_form_constructor(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
        form: dir::Form,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let Some(value) = instance.arguments.first().copied() else {
            return Ok(Answer::Ready(None));
        };
        let formed = dir::Type::Form(dir::FormType { form, value });
        let id = self.push_memory_type(origin, formed)?;

        Ok(Answer::Ready(Some(id)))
    }

    /// Reduce one borrowed form constructor application.
    pub(in crate::check) fn reduce_borrowed_constructor(
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
        let id = self.push_memory_type(origin, formed)?;

        Ok(Answer::Ready(Some(id)))
    }

    /// Reduce one placed form constructor application.
    pub(in crate::check) fn reduce_placed_constructor(
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
        let id = self.push_memory_type(origin, formed)?;

        Ok(Answer::Ready(Some(id)))
    }

    /// Evaluate one memory accessor, distributing over union targets.
    pub(in crate::check) fn reduce_memory_accessor(
        &mut self,
        origin: Origin,
        item: dir::LanguageItem,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let Some(target) = instance.arguments.first().copied() else {
            return Ok(Answer::Ready(None));
        };

        // close the inspected target first
        let target = answer!(self.reduce_type_head(origin, target)?);

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
            let element = match self.reduce_type_head(origin, element)? {
                Answer::Ready(element) => element,
                Answer::Pending(dependencies) => {
                    blockers.extend(dependencies);

                    continue;
                }
            };

            match self.reduce_element_accessor(origin, item, instance, element)? {
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
            [] => self.push_memory_type(origin, dir::Type::Never)?,
            [single] => *single,
            _ => {
                let module = origin.module();
                let source = self.origin_source_node(origin)?;

                self.normalized_union_type(module, kept, source)?
            }
        };

        Ok(Answer::Ready(Some(joined)))
    }

    /// Evaluate one memory accessor over one closed element.
    /// Returns none while the element's forms stay symbolic.
    fn reduce_element_accessor(
        &mut self,
        origin: Origin,
        item: dir::LanguageItem,
        instance: &dir::GenericInstance,
        element: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        // close the element's form chain first
        let chain = answer!(self.form_chain(origin, element)?);
        self.reduce_stack_accessor(origin, item, instance, element, &chain)
    }

    /// Evaluate one memory accessor over one closed form chain.
    fn reduce_stack_accessor(
        &mut self,
        origin: Origin,
        item: dir::LanguageItem,
        instance: &dir::GenericInstance,
        element: dir::GlobalTypeId,
        chain: &FormChain,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        match item {
            // PayloadOf<T> removes one outer form
            dir::LanguageItem::PayloadOf => match chain.forms.first() {
                Some(outer) => Ok(Answer::Ready(Some(outer.value))),
                None if chain.is_open => Ok(Answer::Ready(None)),
                None => Ok(Answer::Ready(Some(chain.base))),
            },
            // BaseOf<T> removes every form
            dir::LanguageItem::BaseOf => {
                if chain.is_open {
                    Ok(Answer::Ready(None))
                } else {
                    Ok(Answer::Ready(Some(chain.base)))
                }
            }

            // ownership component
            dir::LanguageItem::OwnershipOf => self.ownership(origin, chain),
            dir::LanguageItem::OwnershipOr => {
                let ownership = answer!(self.ownership(origin, chain)?);

                self.component_or_default(origin, instance, ownership)
                    .map(Answer::Ready)
            }
            dir::LanguageItem::IsManaged => {
                self.ownership_predicate(origin, chain, dir::Form::Managed)
            }
            dir::LanguageItem::IsOwned => self.ownership_predicate(origin, chain, dir::Form::Owned),
            dir::LanguageItem::IsBorrowed => {
                let found = self.chain_ownership(chain);
                let is_borrowed = match found {
                    Some(form) => self
                        .boolean_literal_type(origin, matches!(form, dir::Form::Borrowed { .. }))?,
                    None if chain.is_open => None,
                    None => self.boolean_literal_type(origin, false)?,
                };

                Ok(Answer::Ready(is_borrowed))
            }
            dir::LanguageItem::IsRaw => self.ownership_predicate(origin, chain, dir::Form::Raw),

            // access component
            dir::LanguageItem::AccessOf => self.access(origin, chain).map(Answer::Ready),
            dir::LanguageItem::AccessOr => {
                let access = self.access(origin, chain)?;

                self.component_or_default(origin, instance, access)
                    .map(Answer::Ready)
            }

            // placement component
            dir::LanguageItem::PlaceOf => self.place(origin, chain).map(Answer::Ready),
            dir::LanguageItem::PlaceOr => {
                let place = self.place(origin, chain)?;

                self.component_or_default(origin, instance, place)
                    .map(Answer::Ready)
            }
            dir::LanguageItem::PlaceIn => {
                let place = self.place(origin, chain)?;
                let Some(place) = place else {
                    return Ok(Answer::Ready(None));
                };

                // ambient placement resolves to the given concrete space
                if self.is_memory_component(place, "ambient")? {
                    let Some(space) = instance.arguments.get(1).copied() else {
                        return Ok(Answer::Ready(None));
                    };

                    Ok(Answer::Ready(Some(
                        self.normalize_component_text(origin, space)?,
                    )))
                } else {
                    Ok(Answer::Ready(Some(place)))
                }
            }
            dir::LanguageItem::SpaceOf => self.space(origin, chain).map(Answer::Ready),
            dir::LanguageItem::SpaceOr => {
                let space = self.space(origin, chain)?;

                self.component_or_default(origin, instance, space)
                    .map(Answer::Ready)
            }
            dir::LanguageItem::IsShared => {
                let space = self.space(origin, chain)?;
                let Some(space) = space else {
                    return Ok(Answer::Ready(None));
                };
                let is_shared = self.is_memory_component(space, "shared")?;
                let is_shared = self.boolean_literal_type(origin, is_shared)?;

                Ok(Answer::Ready(is_shared))
            }
            dir::LanguageItem::IsSharedIn => {
                let place = self.place(origin, chain)?;
                let Some(place) = place else {
                    return Ok(Answer::Ready(None));
                };

                // ambient resolves to the given concrete space first
                let resolved = if self.is_memory_component(place, "ambient")? {
                    let Some(space) = instance.arguments.get(1).copied() else {
                        return Ok(Answer::Ready(None));
                    };

                    self.normalize_component_text(origin, space)?
                } else {
                    place
                };
                let is_shared = self.is_memory_component(resolved, "shared")?;
                let is_shared = self.boolean_literal_type(origin, is_shared)?;

                Ok(Answer::Ready(is_shared))
            }

            // lifetime component
            dir::LanguageItem::LifetimeOf => self.lifetime(origin, chain).map(Answer::Ready),
            dir::LanguageItem::LifetimeOr => {
                let lifetime = self.lifetime(origin, chain)?;

                self.component_or_default(origin, instance, lifetime)
                    .map(Answer::Ready)
            }

            // form rewriting
            dir::LanguageItem::WithBase => {
                if chain.is_open {
                    return Ok(Answer::Ready(None));
                }
                let Some(base) = instance.arguments.get(1).copied() else {
                    return Ok(Answer::Ready(None));
                };

                Ok(Answer::Ready(Some(self.wrap_forms(
                    origin,
                    &chain.forms,
                    base,
                )?)))
            }
            dir::LanguageItem::WithOwnership => self
                .with_ownership(origin, instance, element)
                .map(Answer::Ready),
            dir::LanguageItem::WithPlace | dir::LanguageItem::WithSpace => self
                .with_place(origin, instance, chain, element)
                .map(Answer::Ready),
            dir::LanguageItem::WithLifetime => {
                let Some(lifetime) = instance.arguments.get(1).copied() else {
                    return Ok(Answer::Ready(None));
                };
                let lifetime = self.normalize_lifetime(origin, lifetime)?;

                self.replace_borrow(origin, chain, Some(lifetime), None)
                    .map(Answer::Ready)
            }
            dir::LanguageItem::WithAccess => self
                .with_access(origin, instance, chain, element)
                .map(Answer::Ready),

            _ => Ok(Answer::Ready(None)),
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

        // collect memory forms outermost first, closing each payload
        loop {
            current = answer!(self.reduce_type_head(origin, current)?);
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

    /// Return one chain's ownership kind as a type literal.
    fn ownership(
        &mut self,
        origin: Origin,
        chain: &FormChain,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let ownership = match self.chain_ownership(chain) {
            Some(dir::Form::Managed) => self.text_literal_type(origin, "managed").map(Some)?,
            Some(dir::Form::Owned) => self.text_literal_type(origin, "owned").map(Some)?,
            Some(dir::Form::Borrowed { .. }) => {
                self.text_literal_type(origin, "borrowed").map(Some)?
            }
            Some(dir::Form::Raw) => self.text_literal_type(origin, "raw").map(Some)?,
            // open bases may still gain ownership at instantiation
            None if chain.is_open => None,
            None => {
                let Some(form) = answer!(self.default_ownership(origin, chain.base)?) else {
                    return Ok(Answer::Ready(Some(
                        self.push_memory_type(origin, dir::Type::Never)?,
                    )));
                };
                match form {
                    dir::Form::Managed => self.text_literal_type(origin, "managed").map(Some)?,
                    dir::Form::Owned => self.text_literal_type(origin, "owned").map(Some)?,
                    _ => None,
                }
            }
            Some(_) if chain.is_open => None,
            Some(_) => Some(self.push_memory_type(origin, dir::Type::Never)?),
        };

        Ok(Answer::Ready(ownership))
    }

    /// Return whether one chain's ownership matches a form constructor.
    fn ownership_predicate(
        &mut self,
        origin: Origin,
        chain: &FormChain,
        form: dir::Form,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let matches_ownership = match self.chain_ownership(chain) {
            Some(found) => self.boolean_literal_type(
                origin,
                std::mem::discriminant(&found) == std::mem::discriminant(&form),
            )?,
            None if chain.is_open => None,
            None => {
                let Some(found) = answer!(self.default_ownership(origin, chain.base)?) else {
                    return self.boolean_literal_type(origin, false).map(Answer::Ready);
                };
                self.boolean_literal_type(
                    origin,
                    std::mem::discriminant(&found) == std::mem::discriminant(&form),
                )?
            }
        };

        Ok(Answer::Ready(matches_ownership))
    }

    /// Return one reduced type's default ownership form.
    fn default_ownership(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::Form>>> {
        let ty = answer!(self.reduce_type_head(origin, ty)?);
        let default = match self.ty(ty)? {
            dir::Type::Any
            | dir::Type::Unknown
            | dir::Type::Object
            | dir::Type::Dynamic(_)
            | dir::Type::Shape(_)
            | dir::Type::Array(_)
            | dir::Type::Function(_) => Some(dir::Form::Managed),
            dir::Type::Never
            | dir::Type::Void
            | dir::Type::Undefined
            | dir::Type::Null
            | dir::Type::Memory(_)
            | dir::Type::Static(_)
            | dir::Type::Intrinsic
            | dir::Type::Literal(_)
            | dir::Type::Key(_)
            | dir::Type::Range(_)
            | dir::Type::EnumMember(_)
            | dir::Type::Tuple(_)
            | dir::Type::Slice(_)
            | dir::Type::FixedArray(_)
            | dir::Type::FunctionPointer(_) => Some(dir::Form::Owned),
            dir::Type::Primitive(primitive) => {
                if primitive.representation_item().is_some() {
                    Some(dir::Form::Managed)
                } else {
                    Some(dir::Form::Owned)
                }
            }
            dir::Type::Instance(instance) => {
                let symbol = instance.symbol;
                match self.definition(symbol).cloned() {
                    Some(dir::Definition::Class(_) | dir::Definition::Interface(_)) => {
                        Some(dir::Form::Managed)
                    }
                    Some(dir::Definition::Struct(_) | dir::Definition::Enum(_)) => {
                        Some(dir::Form::Owned)
                    }
                    Some(dir::Definition::Newtype(definition)) => {
                        let backing = answer!(self.reduce_type_head(origin, definition.value)?);

                        return self.default_ownership(origin, backing);
                    }
                    _ => None,
                }
            }
            dir::Type::Form(form) => match form.form {
                dir::Form::Readonly | dir::Form::Placed { .. } => {
                    return self.default_ownership(origin, form.value);
                }
                dir::Form::Managed
                | dir::Form::Owned
                | dir::Form::Borrowed { .. }
                | dir::Form::Raw => None,
            },
            dir::Type::Reference(_)
            | dir::Type::Parameter(_)
            | dir::Type::Variable(_)
            | dir::Type::This
            | dir::Type::Member(_)
            | dir::Type::Operation(_)
            | dir::Type::FunctionSignature(_)
            | dir::Type::Intersection(_)
            | dir::Type::Union(_)
            | dir::Type::Error => None,
        };

        Ok(Answer::Ready(default))
    }

    /// Return one chain's access mode as a type literal.
    /// Unqualified concrete chains default to mutable access.
    fn access(
        &mut self,
        origin: Origin,
        chain: &FormChain,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        for entry in &chain.forms {
            match entry.form {
                dir::Form::Readonly => return self.text_literal_type(origin, "readonly").map(Some),
                dir::Form::Borrowed { access, .. } => {
                    return self.normalize_component_text(origin, access).map(Some);
                }
                _ => {}
            }
        }

        if chain.is_open {
            Ok(None)
        } else {
            self.text_literal_type(origin, "mutable").map(Some)
        }
    }

    /// Return one chain's placement as a type literal.
    /// Unqualified concrete chains stay ambient.
    fn place(
        &mut self,
        origin: Origin,
        chain: &FormChain,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        for entry in &chain.forms {
            if let dir::Form::Placed { place } = entry.form {
                return self.normalize_component_text(origin, place).map(Some);
            }
        }

        if chain.is_open {
            Ok(None)
        } else {
            self.text_literal_type(origin, "ambient").map(Some)
        }
    }

    /// Return one chain's explicit concrete space as a type literal.
    fn space(
        &mut self,
        origin: Origin,
        chain: &FormChain,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let place = self.place(origin, chain)?;
        let Some(place) = place else {
            return Ok(None);
        };

        // ambient placement names no concrete space
        if self.is_memory_component(place, "ambient")? {
            Ok(Some(self.push_memory_type(origin, dir::Type::Never)?))
        } else {
            Ok(Some(place))
        }
    }

    /// Return one chain's borrow lifetime type.
    /// Borrows carry their lifetime argument; managed handles answer the
    /// frame lifetime of the automatic root that pins them.
    fn lifetime(
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
            Ok(Some(self.push_memory_type(origin, dir::Type::Never)?))
        }
    }

    /// Return a component value unless it is never, otherwise return the `*Or` default.
    fn component_or_default(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
        component: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(component) = component else {
            return Ok(None);
        };

        if matches!(self.ty(component)?, dir::Type::Never) {
            let Some(default) = instance.arguments.get(1).copied() else {
                return Ok(None);
            };

            Ok(Some(self.normalize_component_text(origin, default)?))
        } else {
            Ok(Some(component))
        }
    }

    /// Apply one requested ownership form.
    fn with_ownership(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
        element: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(ownership) = instance.arguments.get(1).copied() else {
            return Ok(None);
        };
        let Some(text) = self.memory_component_text(origin, ownership)? else {
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

        Ok(Some(self.push_memory_type(origin, formed)?))
    }

    /// Resolve one ambient placement form.
    fn with_place(
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

        // preserve concrete placement already carried by the chain
        let position = chain
            .forms
            .iter()
            .position(|entry| matches!(entry.form, dir::Form::Placed { .. }));
        match position {
            Some(position) => {
                let dir::Form::Placed { place: current } = chain.forms[position].form else {
                    unreachable!("placement position must point at a placement form");
                };
                if !self.is_memory_component(current, "ambient")? {
                    return Ok(Some(self.wrap_forms(origin, &chain.forms, chain.base)?));
                }

                // resolve ambient placement in its existing chain position
                let mut forms = chain.forms.clone();
                forms[position].form = dir::Form::Placed { place };

                Ok(Some(self.wrap_forms(origin, &forms, chain.base)?))
            }
            // unplaced chains are ambient by default
            None => {
                let formed = dir::Type::Form(dir::FormType {
                    form: dir::Form::Placed { place },
                    value: element,
                });

                Ok(Some(self.push_memory_type(origin, formed)?))
            }
        }
    }

    /// Apply one requested access form.
    fn with_access(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
        chain: &FormChain,
        element: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(access) = instance.arguments.get(1).copied() else {
            return Ok(None);
        };
        let access = self.normalize_access(origin, access)?;

        // borrow access lives on the borrow form itself
        if chain
            .forms
            .iter()
            .any(|entry| matches!(entry.form, dir::Form::Borrowed { .. }))
        {
            return self.replace_borrow(origin, chain, None, Some(access));
        }

        // non-borrow readonly access is a readonly form over the same value
        match self.ty(access)? {
            dir::Type::Memory(dir::MemoryLiteral::Access(dir::Access::Readonly)) => {
                if chain
                    .forms
                    .iter()
                    .any(|entry| matches!(entry.form, dir::Form::Readonly))
                {
                    Ok(Some(element))
                } else {
                    let mut forms = chain.forms.clone();
                    forms.push(dir::FormType {
                        form: dir::Form::Readonly,
                        value: chain.base,
                    });

                    Ok(Some(self.wrap_forms(origin, &forms, chain.base)?))
                }
            }
            dir::Type::Memory(dir::MemoryLiteral::Access(dir::Access::Mutable)) => {
                let forms = chain
                    .forms
                    .iter()
                    .copied()
                    .filter(|entry| !matches!(entry.form, dir::Form::Readonly))
                    .collect::<SmallVec<[_; 2]>>();

                Ok(Some(self.wrap_forms(origin, &forms, chain.base)?))
            }
            dir::Type::Memory(dir::MemoryLiteral::Access(dir::Access::Exclusive)) => {
                Ok(Some(self.push_memory_type(origin, dir::Type::Never)?))
            }
            _ => Ok(None),
        }
    }

    /// Rebuild one chain's borrow form with replaced components.
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
            current = self.push_memory_type(
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
        let Some(text) = self.memory_component_text(origin, component)? else {
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
        let Some(text) = self.memory_component_text(origin, component)? else {
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

    /// Normalize one space component to its canonical memory literal.
    fn normalize_space(
        &mut self,
        origin: Origin,
        component: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some(text) = self.memory_component_text(origin, component)? else {
            return Ok(component);
        };
        let space = match text.as_str() {
            "local" => dir::Space::Local,
            "shared" => dir::Space::Shared,
            "static" => dir::Space::Static,
            "frame" => dir::Space::Frame,
            _ => return Ok(component),
        };

        self.push_memory_literal(origin, dir::MemoryLiteral::Space(space))
    }

    /// Normalize one lifetime component to its canonical memory literal.
    fn normalize_lifetime(
        &mut self,
        origin: Origin,
        component: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some(text) = self.memory_component_text(origin, component)? else {
            return Ok(component);
        };

        let lifetime = match text.as_str() {
            "static" => dir::Lifetime::Static,
            "frame" => dir::Lifetime::Frame,
            _ => return Ok(component),
        };

        self.push_memory_literal(origin, dir::MemoryLiteral::Lifetime(lifetime))
    }

    /// Return one memory component as its canonical text written form.
    fn normalize_component_text(
        &mut self,
        origin: Origin,
        component: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        match self.memory_component_text(origin, component)? {
            Some(text) => self.text_literal_type(origin, &text),
            None => Ok(component),
        }
    }

    /// Return the text behind one closed memory component.
    fn memory_component_text(
        &mut self,
        origin: Origin,
        component: dir::GlobalTypeId,
    ) -> CompilerResult<Option<String>> {
        let Some(component) = self.reduce_type_head(origin, component)?.ready() else {
            return Ok(None);
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

    /// Return whether one type is a specific memory component spelling.
    fn is_memory_component(&self, ty: dir::GlobalTypeId, text: &str) -> CompilerResult<bool> {
        let is_match = match self.ty(ty)? {
            dir::Type::Literal(dir::ScalarLiteral::String(value)) => {
                *value == dir::StringId::for_text(text)
            }
            dir::Type::Memory(literal) => literal.text() == text,
            _ => false,
        };

        Ok(is_match)
    }

    /// Push one string literal type.
    fn text_literal_type(
        &mut self,
        origin: Origin,
        text: &str,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let module = origin.module();
        let value = self.module_mut(module).strings.intern(text);

        self.push_memory_type(
            origin,
            dir::Type::Literal(dir::ScalarLiteral::String(value)),
        )
    }

    /// Push one boolean literal type.
    fn boolean_literal_type(
        &mut self,
        origin: Origin,
        value: bool,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let ty = self.push_memory_type(
            origin,
            dir::Type::Literal(dir::ScalarLiteral::Boolean(value)),
        )?;

        Ok(Some(ty))
    }

    /// Push one memory literal written form.
    fn push_memory_literal(
        &mut self,
        origin: Origin,
        literal: dir::MemoryLiteral,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.push_memory_type(origin, dir::Type::Memory(literal))
    }

    /// Push one memory type at the accessor's origin.
    fn push_memory_type(
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
    /// Return the borrow required by one expected type.
    pub(in crate::check) fn implicit_borrow(
        &mut self,
        origin: Origin,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<ImplicitBorrow>>> {
        let reduced = answer!(self.reduce_type(origin, ty)?);
        if let dir::Type::Form(form) = self.ty(reduced)?.clone()
            && let dir::Form::Borrowed { lifetime, access } = form.form
        {
            return Ok(Answer::Ready(Some(ImplicitBorrow {
                target: reduced,
                lifetime,
                access,
            })));
        }

        let dir::Type::Instance(instance) = self.ty(ty)?.clone() else {
            return Ok(Answer::Ready(None));
        };
        if self.language_item(instance.symbol)? != Some(dir::LanguageItem::WithAccess)
            || instance.arguments.len() != 2
        {
            return Ok(Answer::Ready(None));
        }

        let value = answer!(self.reduce_type(origin, instance.arguments[0])?);
        let access = instance.arguments[1];
        let dir::Type::Form(form) = self.ty(value)?.clone() else {
            return Ok(Answer::Ready(None));
        };
        let dir::Form::Borrowed { lifetime, .. } = form.form else {
            return Ok(Answer::Ready(None));
        };
        let target = self.push_type(
            module,
            dir::Type::Form(dir::FormType {
                form: dir::Form::Borrowed { lifetime, access },
                value: form.value,
            }),
            source,
        )?;

        Ok(Answer::Ready(Some(ImplicitBorrow {
            target,
            lifetime,
            access,
        })))
    }

    /// Resolve one type to its readable value.
    /// Reads see through managed handles and readonly forms; the
    /// payload carries the fields and elements a read consumes.
    pub(in crate::check) fn readable_value(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut current = self.settled_root(ty)?;
        while let dir::Type::Form(form) = self.ty(current)? {
            // only alias-transparent forms disappear for reads
            if !matches!(form.form, dir::Form::Managed | dir::Form::Readonly) {
                break;
            }
            current = self.settled_root(form.value)?;
        }

        Ok(current)
    }
}
