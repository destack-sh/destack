use destack_dir as dir;

use crate::sema::derive::{Component, ComponentProjection, Composite, Derivation};
use crate::sema::{CheckState, Origin, ProtocolCall, Value};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Return the components one composite receiver derives over, mirroring the structural rule.
    pub(super) fn derived_family(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Option<(Composite, Vec<(ComponentProjection, dir::GlobalTypeId)>)>> {
        // resolve the receiver to the family it derives over
        let ty = self.normalize(origin, receiver)?;
        let ty = self.shallow_resolve(ty)?;
        let module = ty.module_id;
        let kind = self.ty(ty)?;

        // read through the memory forms to the value they hold
        if let dir::Type::Form(form) = kind {
            return match form.form {
                dir::Form::Borrowed(_) | dir::Form::Raw => Ok(None),
                dir::Form::Readonly | dir::Form::Owned => self.derived_family(origin, form.value),
            };
        }

        // read the components the receiver stores, in storage order
        let (shape, components) = match kind {
            dir::Type::Tuple(tuple) => {
                let elements = self.tuple_elements(module, tuple.elements)?;
                let components = elements
                    .iter()
                    .enumerate()
                    .map(|(index, element)| {
                        let read = ComponentProjection::Field {
                            key: dir::StaticKey::Index(index),
                            symbol: None,
                        };

                        (read, element.ty)
                    })
                    .collect::<Vec<_>>();

                (Composite::Tuple, components)
            }
            dir::Type::Union(union) => {
                let members = self.type_ids(module, union.elements)?;
                let components = members
                    .iter()
                    .map(|member| (ComponentProjection::Member, *member))
                    .collect::<Vec<_>>();

                (Composite::Union, components)
            }
            dir::Type::Application(application) => {
                let Some(definition) = self.definition(application.symbol)? else {
                    return Ok(None);
                };
                let substitution = self.instance_substitution(module, &application)?;
                let (shape, mut components) = match &*definition {
                    dir::Definition::Struct(definition) => (
                        Composite::Struct(application.symbol),
                        self.field_components(&definition.members)?,
                    ),
                    dir::Definition::Class(definition) => {
                        let mut components = self.field_components(&definition.members)?;
                        if let Some(extends) = &definition.extends {
                            components.push((ComponentProjection::Base, extends.ty));
                        }

                        (Composite::Class(application.symbol), components)
                    }
                    dir::Definition::Newtype(definition) => (
                        Composite::Newtype(application.symbol),
                        vec![(ComponentProjection::Backing, definition.backing)],
                    ),
                    _ => return Ok(None),
                };
                for component in &mut components {
                    component.1 = self.substitute_type(component.1, &substitution)?;
                }

                (shape, components)
            }
            _ => return Ok(None),
        };

        Ok(Some((shape, components)))
    }

    /// Select the protocol call each component runs, its receiver borrowed like this.
    pub(super) fn derived_components(
        &mut self,
        frame: &mut Derivation,
        origin: Origin,
        interface: dir::AutoInterface,
        item: dir::LanguageMember,
        components: Vec<(ComponentProjection, dir::GlobalTypeId)>,
    ) -> CompilerResult<Vec<Component>> {
        // read the place the components live at: the receiver's own region and space
        let place = match frame.this {
            Some(this) => Some(self.value_place(
                origin,
                Value {
                    ty: this,
                    node: None,
                    place: None,
                    is_fresh: false,
                },
            )?),
            None => None,
        };
        let mut selected = Vec::with_capacity(components.len());
        for (read, ty) in components {
            // skip the protocol call for a unit member
            let call = match self.is_unit_type(ty)? {
                true => None,
                false => Some(self.derived_component_call(
                    origin,
                    ty,
                    place,
                    interface,
                    item,
                    frame.parameters.first().map(|parameter| parameter.2),
                )?),
            };
            selected.push(Component { read, ty, call });
        }

        Ok(selected)
    }

    /// Replace the base of one class's components by the base's own fields, down the heritage.
    pub(super) fn class_field_chain(
        &mut self,
        origin: Origin,
        components: Vec<(ComponentProjection, dir::GlobalTypeId)>,
    ) -> CompilerResult<Vec<(ComponentProjection, dir::GlobalTypeId)>> {
        let mut chain = Vec::with_capacity(components.len());
        for (read, ty) in components {
            if read != ComponentProjection::Base {
                chain.push((read, ty));

                continue;
            }
            let Some((_, base)) = self.derived_family(origin, ty)? else {
                return Err(CompilerError::Internal {
                    message: "a class base without a derivable family".to_owned(),
                });
            };
            chain.extend(self.class_field_chain(origin, base)?);
        }

        Ok(chain)
    }

    /// Return the instance fields one declaration stores as components, in storage order.
    fn field_components(
        &mut self,
        members: &[dir::DefinitionMember],
    ) -> CompilerResult<Vec<(ComponentProjection, dir::GlobalTypeId)>> {
        // keep the instance fields, in declaration order
        let mut components = Vec::new();
        for member in members {
            if let dir::DefinitionMember::Field(field) = member
                && field.space == dir::MemberSpace::Instance
            {
                let read = ComponentProjection::Field {
                    key: field.key,
                    symbol: Some(field.symbol),
                };
                components.push((read, self.symbol_type(field.symbol)?));
            }
        }

        Ok(components)
    }

    /// Return whether one type is its own value, storing nothing to run a protocol over.
    fn is_unit_type(&self, ty: dir::GlobalTypeId) -> CompilerResult<bool> {
        Ok(matches!(
            self.ty(ty)?,
            dir::Type::Literal(_)
                | dir::Type::Null
                | dir::Type::Undefined
                | dir::Type::Void
                | dir::Type::Never
        ))
    }

    /// Return one component type under the borrow form another value holds.
    pub(super) fn borrowed_like(
        &mut self,
        like: Option<dir::GlobalTypeId>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // borrow like this, else for the frame a value receiver lives in
        if let Some(like) = like
            && let dir::Type::Form(form) = self.ty(like)?
            && matches!(form.form, dir::Form::Borrowed(_))
        {
            let form = form.form;

            return self.intern_type(dir::Type::Form(dir::FormType { form, value: ty }));
        }

        // borrow every other value for the protocol to read through its storage
        match self.frame_borrow_of(ty, dir::Access::Immutable)? {
            Some(borrowed) => Ok(borrowed),
            None => Ok(ty),
        }
    }

    /// Select the protocol call one component runs over itself.
    pub(super) fn derived_component_call(
        &mut self,
        origin: Origin,
        component: dir::GlobalTypeId,
        place: Option<dir::PlaceResolution>,
        interface: dir::AutoInterface,
        item: dir::LanguageMember,
        argument: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<ProtocolCall> {
        // shape the protocol member by its interface: its space, type arguments, and argument
        let (space, type_arguments, sources): (_, &[dir::GlobalTypeId], Vec<_>) = match interface {
            dir::AutoInterface::Equal | dir::AutoInterface::PartialEqual => {
                let mut sources = Vec::new();
                if let Some(argument) = argument {
                    let other = self.borrowed_like(Some(argument), component)?;
                    sources.push(dir::ArgumentSource::Static(other));
                }

                (dir::MemberSpace::Instance, &[component], sources)
            }
            dir::AutoInterface::Hash => (
                dir::MemberSpace::Instance,
                &[],
                argument
                    .map(dir::ArgumentSource::Static)
                    .into_iter()
                    .collect(),
            ),
            dir::AutoInterface::Default => (dir::MemberSpace::Static, &[], Vec::new()),
            _ => (dir::MemberSpace::Instance, &[], Vec::new()),
        };

        // select the protocol member over the component, at the receiver's place
        let receiver = Value {
            ty: component,
            node: None,
            place,
            is_fresh: false,
        };
        let selected = self.select_language_protocol_call(
            origin,
            receiver,
            component,
            space,
            item.key,
            item.owner,
            type_arguments,
            type_arguments,
            &sources,
        )?;
        let Some((_, call)) = selected else {
            return Err(CompilerError::Internal {
                message: format!(
                    "a derived component of {} outside its protocol",
                    self.format_type(component)
                ),
            });
        };

        Ok(call)
    }
}
