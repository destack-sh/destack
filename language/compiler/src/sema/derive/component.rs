use destack_dir as dir;

use crate::sema::derive::{Component, ComponentCall, ComponentProjection, Composite, Derivation};
use crate::sema::{CheckState, Origin, Value};
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
                dir::Form::Managed { .. } | dir::Form::Borrowed(_) | dir::Form::Raw => Ok(None),
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
            Some(this) => self.derived_this_place(origin, this)?,
            None => None,
        };
        let mut selected = Vec::with_capacity(components.len());
        for (read, ty) in components {
            // skip the protocol call for a unit member
            let call = match self.is_unit_type(ty)? {
                true => None,
                false => {
                    let Some(mut call) =
                        self.derived_component_call(origin, ty, place, ty, interface, item)?
                    else {
                        selected.push(Component {
                            read,
                            ty,
                            call: None,
                        });

                        continue;
                    };

                    // solve the regions the member's own this and parameters supply
                    self.solve_derived_call_regions(frame, origin, &mut call.call)?;

                    // borrow the component at the receiver's own region
                    if let dir::CallableTarget::Symbol { function, .. } = &mut call.call.target
                        && let Some(receiver) = &mut function.receiver
                    {
                        for adjustment in &mut receiver.adjustments {
                            if let dir::ReceiverAdjustment::Borrow { ty: borrowed } = adjustment {
                                *borrowed = self.borrowed_like(frame.this, ty)?;
                            }
                        }
                    }

                    Some(call)
                }
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

        // borrow every other value for the protocol to read
        match self.frame_borrow_of(ty, dir::Access::Readonly)? {
            Some(borrowed) => Ok(borrowed),
            None => Ok(ty),
        }
    }

    /// Select the protocol call one component runs over the value reading it.
    pub(super) fn derived_component_call(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
        place: Option<dir::PlaceResolution>,
        component: dir::GlobalTypeId,
        interface: dir::AutoInterface,
        item: dir::LanguageMember,
    ) -> CompilerResult<Option<ComponentCall>> {
        // bind the peer the comparison takes and the state the hash writes
        let (space, written, sources): (_, &[dir::GlobalTypeId], &[dir::ArgumentSource]) =
            match interface {
                dir::AutoInterface::Equal | dir::AutoInterface::PartialEqual => (
                    dir::MemberSpace::Instance,
                    &[component],
                    &[dir::ArgumentSource::Supplied],
                ),
                dir::AutoInterface::Hash => (
                    dir::MemberSpace::Instance,
                    &[],
                    &[dir::ArgumentSource::Supplied],
                ),
                dir::AutoInterface::Default => (dir::MemberSpace::Static, &[], &[]),
                _ => (dir::MemberSpace::Instance, &[], &[]),
            };

        // select the protocol member over the value the component reads, at the receiver's place
        let receiver = Value {
            ty: value,
            node: None,
            place,
            is_fresh: false,
        };
        let selected = self.select_language_protocol_call(
            origin, receiver, component, space, item.key, item.owner, written, written, sources,
        )?;
        let Some((_, call)) = selected else {
            return Err(CompilerError::Internal {
                message: format!("a derived component of {component:?} outside its protocol"),
            });
        };

        // take the member and call a single resolution names
        match (call.member, call.resolution) {
            (dir::OperationResolution::One(member), dir::OperationResolution::One(call)) => {
                Ok(Some(ComponentCall { member, call }))
            }
            // dispatch a union beneath the component per arm
            _ => self.derived_whole_component_call(origin, value, component, item, space),
        }
    }

    /// Solve every region one derived body's call leaves open: the callee's receiver region at
    /// the member's own this region, each value parameter's region at the member's parameter
    /// supplied at that position, the body reading its own arguments through them.
    pub(super) fn solve_derived_call_regions(
        &mut self,
        frame: &Derivation,
        origin: Origin,
        call: &mut dir::Call,
    ) -> CompilerResult<()> {
        let dir::CallableTarget::Symbol { function, .. } = &call.target else {
            return Ok(());
        };
        let symbol = function.key.symbol;
        let declared = self.symbol_type(symbol)?;
        let Some(head) = self.signature_head(declared)? else {
            return Ok(());
        };

        // pair each region parameter of the callee with the extent the member supplies: its own
        // this region, or the frame holding a this it takes by value
        let mut supplied = Vec::new();
        if let Some(this) = head.this_parameter
            && let Some(parameter) = self.region_parameter_of(Origin::Symbol(symbol), this)?
            && let Some(own) = frame.this
        {
            let extent = match self.form_chain(origin, own)?.region() {
                Some(held) => self.region_extent(held)?,
                None => self.lifetime_literal(dir::Lifetime::Frame)?,
            };
            supplied.push((parameter, extent));
        }
        let parameters = self
            .signature_parameters(declared.module_id, head.parameters)?
            .to_vec();
        for (index, parameter) in parameters.iter().enumerate() {
            if let Some(region) = self.region_parameter_of(Origin::Symbol(symbol), parameter.ty)?
                && let Some((_, _, own)) = frame.parameters.get(index)
                && let Some(held) = self.form_chain(origin, *own)?.region()
            {
                let extent = self.region_extent(held)?;
                supplied.push((region, extent));
            }
        }

        // solve each open region at the extent supplied for it
        for binding in &mut call.regions {
            let Some(variable) = self.root_variable(binding.argument)? else {
                continue;
            };
            let Some((_, extent)) = supplied
                .iter()
                .find(|(parameter, _)| *parameter == binding.parameter)
            else {
                continue;
            };
            self.commit_solution(variable, *extent)?;
            binding.argument = self.shallow_resolve(binding.argument)?;
        }

        Ok(())
    }

    /// Return the region parameter one declared borrow names as its extent.
    fn region_parameter_of(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalGenericParameterId>> {
        let Some(region) = self.form_chain(origin, ty)?.region() else {
            return Ok(None);
        };
        let extent = self.region_extent(region)?;
        Ok(match self.ty(extent)? {
            dir::Type::Parameter(parameter) => Some(parameter),
            _ => None,
        })
    }

    /// Return the place one derived member's components live at, projected from its this.
    fn derived_this_place(
        &mut self,
        origin: Origin,
        this: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::PlaceResolution>> {
        let chain = self.form_chain(origin, this)?;
        let Some(region) = chain.region() else {
            return Ok(None);
        };
        let (lifetime, placement) = match self.ty(region)? {
            dir::Type::Region(pair) => (pair.extent, pair.space),
            _ => {
                let placement = match chain.place() {
                    Some(place) => place,
                    None => self.local_place()?,
                };

                (region, placement)
            }
        };
        let access = match chain.is_readonly() {
            true => dir::Access::Readonly,
            false => dir::Access::Mutable,
        };
        let access = self.access_literal(access)?;

        Ok(Some(dir::PlaceResolution {
            placement,
            lifetime,
            access,
        }))
    }

    /// Return the call of one component's own derived member.
    fn derived_whole_component_call(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
        component: dir::GlobalTypeId,
        item: dir::LanguageMember,
        space: dir::MemberSpace,
    ) -> CompilerResult<Option<ComponentCall>> {
        // find the requirement the interface declares under the member's key
        let interface = self.language_symbol(item.owner)?;
        let declared = self.definition(interface)?;
        let Some(dir::Definition::Interface(definition)) = declared.as_deref() else {
            return Err(CompilerError::Internal {
                message: "a derived protocol outside an interface".to_owned(),
            });
        };
        let requirement = definition.members.iter().find_map(|member| match member {
            dir::DefinitionMember::Method(method) if member.key() == Some(item.key) => {
                Some(method.symbol)
            }
            _ => None,
        });
        let Some(requirement) = requirement else {
            return Err(CompilerError::Internal {
                message: "a derived protocol without its requirement".to_owned(),
            });
        };

        // synthesize the component's own member at its normal form, an alias read through
        let component = self.normalize(origin, component)?;
        let Origin::Node(source, _) = origin else {
            return Err(CompilerError::Internal {
                message: "a derived component outside a node origin".to_owned(),
            });
        };
        let Some(synthesized) = self.build_derived_member(requirement, component, &[], source)?
        else {
            return Ok(None);
        };

        // call the member through a borrow of the component, typed by the caller afterwards
        let key =
            dir::InstanceKey::new(synthesized.symbol, Vec::new()).with_receiver(Some(component));
        let Some((_, signature)) = self.callable_signature_type(origin, synthesized.callable)?
        else {
            return Err(CompilerError::Internal {
                message: "a derived member without a signature".to_owned(),
            });
        };
        let return_type = match signature.return_type {
            Some(ty) => ty,
            None => self.intern_type(dir::Type::Void)?,
        };
        let receiver = dir::AdjustedReceiver {
            source: value,
            adjustments: vec![dir::ReceiverAdjustment::Borrow { ty: value }],
        };
        let candidate = dir::MemberCandidate {
            receiver: dir::MemberReceiver::Direct(receiver.clone()),
            space,
            owner: interface,
            access_type: synthesized.callable,
            callable_type: Some(synthesized.callable),
            key: key.clone(),
            regions: Vec::new(),
        };
        let member = dir::MemberAccess::new(
            value,
            dir::MemberTarget::Symbol(candidate),
            synthesized.callable,
        );
        let arguments = synthesized
            .parameters()
            .map(|(_, ty)| dir::ArgumentBinding {
                parameter_type: ty,
                argument_type: ty,
                source: dir::ArgumentSource::Supplied,
            })
            .collect();
        let call = dir::Call {
            regions: Vec::new(),
            target: dir::CallableTarget::Symbol {
                function: dir::FunctionTarget {
                    receiver: Some(receiver),
                    generic_scope: None,
                    key,
                },
                dispatch: dir::FunctionDispatch::Direct,
            },
            callable_type: synthesized.callable,
            arguments,
            return_type,
        };

        Ok(Some(ComponentCall { member, call }))
    }
}
