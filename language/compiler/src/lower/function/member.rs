use destack_dir as dir;
use destack_mir as mir;

use crate::lower::function::call::ReceiverUse;
use crate::lower::function::place::PlaceProjection;
use crate::lower::function::union::UnionDispatch;
use crate::lower::{FunctionLowerer, GenericScope};
use crate::{CompilerError, CompilerResult};

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one member read through its resolution.
    pub(in crate::lower) fn lower_member(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        // read the access's own optionality for its chain guard
        let is_optional = matches!(
            self.source().tree().get(expression),
            dir::Expression::Member {
                is_optional: true,
                ..
            }
        );

        // construct the case value for enum members
        if let dir::Type::Variant(variant) = self.node_type(expression)? {
            return self.lower_variant_member(&variant);
        }

        // read a decided reference as its resolved symbol
        let node = expression.into_global_any(self.source);
        let named = self.source().decisions.function_symbol(node).or_else(|| {
            self.source()
                .resolutions
                .name_resolution(node)
                .and_then(|resolution| resolution.single_symbol())
        });
        if let Some(symbol) = named {
            return self.read_symbol(expression, symbol);
        }

        // read the single access selected for this member
        let resolution = self.member_decision(expression)?;
        let dir::OperationResolution::One(access) = &resolution else {
            return self.lower_union_member_read(expression, left, resolution.arms());
        };

        // lower the read at the selected target
        match &access.target {
            // project a compiler-defined member off the receiver
            dir::MemberTarget::Projection {
                receiver,
                projection,
                ..
            } => {
                let receiver = self.lower_adjusted_receiver(
                    left,
                    receiver,
                    is_optional,
                    ReceiverUse::Storage,
                )?;

                self.lower_member_projection(receiver, projection)
            }
            // call the resolved getter
            dir::MemberTarget::Call(call) => {
                let dir::Call {
                    target:
                        dir::CallableTarget::Symbol {
                            function,
                            dispatch: dir::FunctionDispatch::Direct,
                        },
                    ..
                } = &**call
                else {
                    return Err(self.unsupported("a dynamic property read"));
                };
                let value =
                    self.lower_function_target_call(left, call, function, None, is_optional)?;

                value.ok_or_else(|| CompilerError::Internal {
                    message: "a void result from a getter".to_string(),
                })
            }
            // read the resolved field
            dir::MemberTarget::Field(field) => {
                self.lower_field_read(expression, left, field, is_optional)
            }
            // read the declaration a member names, a type having no runtime value
            dir::MemberTarget::Symbol(candidate) => {
                let symbol = candidate.key.symbol;
                let kind = self
                    .lower
                    .state(symbol.module_id)?
                    .bindings
                    .get_symbol(symbol.local_id)
                    .kind;
                if kind == dir::SymbolKind::AssociatedConst {
                    return self.lower_associated_const_read(expression, candidate);
                }

                self.read_symbol(expression, symbol)
            }
            // reject every other member read
            other => Err(self.unsupported(format!("a '{}' member read", other.name()))),
        }
    }

    /// Lower one associated const read at the witness or global answering it.
    fn lower_associated_const_read(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        candidate: &dir::MemberCandidate,
    ) -> CompilerResult<mir::Value> {
        // read an implementer's const through its global
        let symbol = candidate.key.symbol;
        let owner = candidate.owner;
        let is_requirement = matches!(
            self.lower.definition(owner)?,
            Some(dir::Definition::Interface(_))
        );
        if !is_requirement {
            return self.read_symbol(expression, symbol);
        }

        // read a requirement through the witness of the type it was read on
        let receiver = self.static_receiver_type(candidate.receiver.source())?;
        let owner_template = self
            .lower
            .definition(owner)?
            .and_then(|definition| definition.template())
            .map(|template| template.into_global(owner.module_id));
        let chain = GenericScope::from_templates(self.lower, owner_template, None)?;
        let arguments = self.selection_arguments(receiver, &candidate.key, &chain)?;
        let (receiver, interface) =
            self.lower_witness_types(owner, receiver, &chain, &arguments)?;
        let Some(member) = self.lower.symbol_name(symbol)? else {
            return Err(CompilerError::Internal {
                message: "an associated const without a name".to_string(),
            });
        };
        let ty = self.lower_type(self.node_type_id(expression)?)?;

        Ok(self.builder.constant(
            mir::Constant::Witness {
                receiver,
                interface,
                member,
            },
            ty,
        ))
    }

    /// Return the type one static member was read on, a type held in a static term unwrapped.
    fn static_receiver_type(
        &mut self,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let dir::Type::Static(value) = self.lower.ty(receiver)? else {
            return Ok(receiver);
        };
        let statics = &self.lower.state(value.module_id)?.statics;
        match statics.get_static_maybe(value.local_id) {
            Some(dir::StaticTerm::Type { ty }) => Ok(*ty),
            _ => Ok(receiver),
        }
    }

    /// Lower one subscript read through its resolution.
    pub(in crate::lower) fn lower_subscript(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        index: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<mir::Value> {
        // read the access's own optionality for its chain guard
        let is_optional = matches!(
            self.source().tree().get(expression),
            dir::Expression::Index {
                is_optional: true,
                ..
            }
        );

        // read the single access selected for this subscript
        let resolution = self.subscript_decision(expression)?;
        let dir::OperationResolution::One(subscript) = resolution else {
            return Err(self.unsupported("a subscript read on a union receiver"));
        };

        // lower the read at the selected target
        match subscript.target {
            // read the member the constant key names
            dir::SubscriptTarget::Member(access) => match access.target {
                // project a compiler-defined member off the receiver
                dir::MemberTarget::Projection {
                    receiver,
                    projection,
                    ..
                } => {
                    let receiver = self.lower_adjusted_receiver(
                        left,
                        &receiver,
                        is_optional,
                        ReceiverUse::Storage,
                    )?;

                    // evaluate the computed singleton key
                    if let Some(index) = index {
                        self.lower_const_expression(index)?;
                    }

                    self.lower_member_projection(receiver, &projection)
                }
                // read the constant key straight off its field
                dir::MemberTarget::Field(field) => {
                    let value = self.lower_member_receiver(left, &field.receiver, is_optional)?;

                    // evaluate the computed singleton key
                    if let Some(index) = index {
                        self.lower_const_expression(index)?;
                    }

                    self.lower_field_value(expression, value, &field)
                }
                // find the computed key through the receiver's dynamic table
                dir::MemberTarget::Index(read)
                    if matches!(read.target, dir::IndexTarget::Signature(_)) =>
                {
                    // dispatch a keyed find by name over string domains only
                    let domain = read.key_type;
                    if !matches!(
                        self.lower.ty(domain)?,
                        dir::Type::Primitive(dir::PrimitiveType::String)
                    ) {
                        return Err(self.unsupported("a non-string signature key domain"));
                    }
                    let key = index.ok_or_else(|| CompilerError::Internal {
                        message: "a signature subscript read without a key expression".to_string(),
                    })?;

                    self.lower_dynamic_signature_read(expression, left, key)
                }
                // reject every other subscript read
                other => Err(self.unsupported(format!("a '{}' subscript read", other.name()))),
            },
            // call the resolved subscript getter
            dir::SubscriptTarget::Call(call) => {
                let dir::Call {
                    target:
                        dir::CallableTarget::Symbol {
                            function,
                            dispatch: dir::FunctionDispatch::Direct,
                        },
                    ..
                } = &call
                else {
                    return Err(self.unsupported("a dynamic subscript read"));
                };
                let value =
                    self.lower_function_target_call(left, &call, function, None, is_optional)?;

                value.ok_or_else(|| CompilerError::Internal {
                    message: "a void result from a subscript read".to_string(),
                })
            }
            // load the value behind the returned address, or take a by-value read's value
            dir::SubscriptTarget::Index(read) => {
                let Some(dereference) = read.dereference.clone() else {
                    return self.lower_index_address(left, read, is_optional);
                };
                let result_type = self.lower_type(dereference.ty)?;
                let address = self.lower_index_address(left, read, is_optional)?;
                let place = mir::Place::value(address).with_projection(mir::Projection::Deref);

                Ok(self.load_place(place, result_type))
            }
        }
    }

    /// Lower the address one Index protocol read returns.
    pub(in crate::lower) fn lower_index_address(
        &mut self,
        left: dir::LocalNodeId<dir::Expression>,
        read: dir::IndexProjection,
        is_optional: bool,
    ) -> CompilerResult<mir::Value> {
        if read.missing.is_some() {
            return Err(self.internal("an Index read with a missing result"));
        }

        // require a directly dispatched Index call
        let call = read.call;
        let dir::Call {
            target:
                dir::CallableTarget::Symbol {
                    function,
                    dispatch: dir::FunctionDispatch::Direct,
                },
            ..
        } = &call
        else {
            return Err(self.unsupported("a dynamic subscript read"));
        };
        let value = self.lower_function_target_call(left, &call, function, None, is_optional)?;

        value.ok_or_else(|| CompilerError::Internal {
            message: "a void result from a subscript read".to_string(),
        })
    }

    /// Lower one field read.
    fn lower_field_read(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        field: &dir::FieldResolution,
        is_optional: bool,
    ) -> CompilerResult<mir::Value> {
        // read erased receivers through their dispatch table
        if let dir::MemberReceiver::Dynamic(dispatch) = &field.receiver {
            let dispatch = dispatch.clone();

            return self.lower_dynamic_member_read(expression, left, field, &dispatch);
        }

        // read a field of owned storage through its place, keeping the stored value whole
        if let dir::MemberReceiver::Direct(receiver) = &field.receiver
            && receiver.adjustments.is_empty()
            && !is_optional
            && self.is_owned_value_place(left, receiver.source)?
        {
            return self.lower_field_place_read(expression, left, field, receiver.source);
        }

        // lower the receiver the resolution selected
        let value = self.lower_member_receiver(left, &field.receiver, is_optional)?;

        self.lower_field_value(expression, value, field)
    }

    /// Lower one field read by projecting the receiver's place and loading the field.
    fn lower_field_place_read(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        field: &dir::FieldResolution,
        held: dir::GlobalTypeId,
    ) -> CompilerResult<mir::Value> {
        // resolve the field's stored representation off the held concrete type
        let storage = self.field_storage(field)?;
        let receiver = self.lower_type(held)?;
        let concrete = mir::Substitution::resolve(receiver, self.builder.tree_mut());
        let stored = self.property_representation(concrete, storage.index as usize)?;

        // load the field through the projected place
        let mut place = self.receiver_place(left)?;
        place.path.push(PlaceProjection::Field {
            field: storage.index,
            ty: stored,
        });
        let value = self.read_place(&place)?;

        self.lower_narrowing(expression, value)
    }

    /// Return whether one expression names owned storage holding a value directly.
    fn is_owned_value_place(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // a reference receiver reads through its indirection
        if self.lower.indirection(ty, &self.scope)?.is_some() {
            return Ok(false);
        }

        self.names_storage(expression)
    }

    /// Lower one field read from its already adjusted receiver value.
    fn lower_field_value(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        value: mir::Value,
        field: &dir::FieldResolution,
    ) -> CompilerResult<mir::Value> {
        // dispatch a field the arms of a union receiver share
        let receiver = field.receiver.ty();
        let indirect = self
            .lower
            .indirection(receiver, &self.scope)?
            .map(|layer| layer.stored);
        let subject = indirect.unwrap_or(receiver);
        if self.lower.union_members_maybe(subject)?.is_some() {
            return Err(CompilerError::Internal {
                message: "a field read on an undispatched union receiver".to_string(),
            });
        }

        // resolve the selected storage field at its stored representation
        let storage = self.field_storage(field)?;

        // load fields through addresses for reference receivers
        let value = match self.innermost_field_address(value, receiver, &storage)? {
            Some(address) => {
                let place = mir::Place::value(address).with_projection(mir::Projection::Deref);

                self.load_place(place, storage.read)
            }
            None => self.builder.field_get(value, storage.index),
        };

        self.lower_narrowing(expression, value)
    }

    /// Resolve one field's storage index and representation off its receiver representation.
    fn field_storage(&mut self, field: &dir::FieldResolution) -> CompilerResult<FieldStorage> {
        let index = self.member_field_index(field)?;
        let read = field.ty;
        let read = self.lower_type(read)?;

        Ok(FieldStorage { index, read })
    }

    /// Lower one payload-free variant member to its case construction.
    fn lower_variant_member(&mut self, variant: &dir::VariantType) -> CompilerResult<mir::Value> {
        // materialize the owner representation and select the declared case
        let dir::Type::Application(owner) = self.lower.ty(variant.owner)? else {
            return Err(CompilerError::Internal {
                message: "a variant without its owner instance".to_string(),
            });
        };
        let ty = self.lower_type(variant.owner)?;
        let case = self.lower.variant_position(owner.symbol, variant.variant)?;

        Ok(self.builder.variant_new(ty, case, None))
    }

    /// Lower one compiler-defined member projection.
    fn lower_member_projection(
        &mut self,
        receiver: mir::Value,
        projection: &dir::Projection,
    ) -> CompilerResult<mir::Value> {
        // lower the projection the member names
        match projection {
            // read the structural discriminant
            dir::Projection::Discriminant {
                union, cases, ty, ..
            } => self.lower_discriminant_value(receiver, *union, cases, *ty),
            // reject every other projection
            other => Err(self.unsupported(format!("a '{}' member projection", other.name()))),
        }
    }

    /// Lower one structural discriminant to its source property value.
    fn lower_discriminant_value(
        &mut self,
        receiver: mir::Value,
        union: dir::GlobalTypeId,
        cases: &[dir::DiscriminantCase],
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<mir::Value> {
        // require at least one runtime case
        if cases.is_empty() {
            return Err(CompilerError::Internal {
                message: "a discriminant projection without a reachable case".to_string(),
            });
        }

        // map physical union cases to the literals they project
        let members = self.lower.union_members(union)?;
        let mut mappings = Vec::with_capacity(cases.len());
        for case in cases {
            let source_index = self.case(&members, case.arm)?;
            mappings.push((source_index as i128, case.value));
        }

        // dispatch on the physical tag and construct the corresponding source value
        let tag = self.lower_discriminant_tag(receiver, union)?;
        let result_type = self.lower_type(ty)?;
        let result = self.builder.local(result_type, mir::Mutability::Immutable);
        let exit = self.builder.block();
        let unreachable = self.builder.block();
        let blocks = mappings
            .iter()
            .map(|(source, _)| (*source, self.builder.block()))
            .collect::<Vec<_>>();
        self.builder.switch(tag, unreachable, blocks.clone());

        // materialize the projected literal in every reachable case
        for ((_, literal), (_, block)) in mappings.into_iter().zip(blocks) {
            self.builder.switch_to_block(block);
            let value = self.lower_constant(literal, result_type)?;
            self.builder.local_set(result, value);
            self.builder.jump(exit);
        }

        // reject any physical case excluded by the flow type
        self.builder.switch_to_block(unreachable);
        self.builder.unreachable();
        self.builder.switch_to_block(exit);

        Ok(self.builder.local_get(result))
    }

    /// Lower the physical tag of one union receiver.
    pub(in crate::lower) fn lower_discriminant_tag(
        &mut self,
        receiver: mir::Value,
        union: dir::GlobalTypeId,
    ) -> CompilerResult<mir::Value> {
        // read the receiver's own representation
        let receiver_type = self.value_representation(receiver)?;

        // stored unions expose their tag through their address
        if matches!(
            self.builder.tree().get(receiver_type),
            mir::Type::Reference { .. } | mir::Type::Pointer { .. }
        ) {
            let union = self.lower_type(union)?;

            return Ok(self.builder.variant_tag_load(
                mir::Place::value(receiver).with_projection(mir::Projection::Deref),
                union,
            ));
        }

        Ok(self.builder.variant_tag(receiver))
    }

    /// Apply the selected receiver adjustments before one member read.
    fn lower_member_receiver(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        receiver: &dir::MemberReceiver,
        is_optional: bool,
    ) -> CompilerResult<mir::Value> {
        // require a direct receiver
        let dir::MemberReceiver::Direct(receiver) = receiver else {
            return Err(self.unsupported("a member read through dynamic dispatch"));
        };

        self.lower_adjusted_receiver(expression, receiver, is_optional, ReceiverUse::Storage)
    }

    /// Return the storage index selected by one field resolution.
    pub(in crate::lower) fn member_field_index(
        &mut self,
        field: &dir::FieldResolution,
    ) -> CompilerResult<u32> {
        // locate storage beneath every reference layer of the selected receiver
        let mut stored = field.receiver.ty();
        while let Some(layer) = self.lower.indirection(stored, &self.scope)? {
            if layer.stored == stored {
                break;
            }
            stored = layer.stored;
        }
        let mut stored = self.lower.stored(stored)?;

        // follow transparent alias and newtype definitions, re-peeling their owners
        while let dir::Type::Application(application) = self.lower.ty(stored)? {
            let defined = match self.lower.definition(application.symbol)? {
                Some(dir::Definition::TypeAlias(_)) => {
                    self.lower.symbol_type(application.symbol)?
                }
                Some(dir::Definition::Newtype(newtype)) => newtype.backing,
                _ => break,
            };

            stored = self.lower.stored(defined)?;
        }

        // find the field's position in the storage the receiver declares
        let index = match self.lower.ty(stored)? {
            dir::Type::Tuple(_) => match field.target.key() {
                dir::StaticKey::Index(index) => Some(index),
                _ => None,
            },
            dir::Type::Application(instance) => {
                let fields = self.lower.nominal_fields(instance.symbol)?;

                fields.iter().position(|stored| match field.target {
                    dir::FieldTarget::Structural { key, .. } => stored.key == key,
                    dir::FieldTarget::Member { symbol, .. } => stored.symbol == symbol,
                })
            }
            dir::Type::Object(shape) => {
                let properties = self
                    .lower
                    .types(stored.module_id)?
                    .properties(shape.properties);

                properties.iter().position(|stored| match field.target {
                    dir::FieldTarget::Structural { key, .. } => stored.key == key,
                    dir::FieldTarget::Member { .. } => false,
                })
            }
            stored_head => {
                return Err(self.unsupported(format!(
                    "a member read on a '{}' receiver",
                    stored_head.variant_name()
                )));
            }
        };

        index
            .map(|index| index as u32)
            .ok_or_else(|| CompilerError::Internal {
                message: "a field absent from its lowered receiver".to_string(),
            })
    }

    /// Dispatch one member read over the union arms its resolution recorded.
    fn lower_union_member_read(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        arms: &[dir::MemberAccess],
    ) -> CompilerResult<mir::Value> {
        // route the arms, then evaluate the receiver once and walk the shared steps
        let routes = self.union_dispatch(arms)?;
        let UnionDispatch {
            chains,
            prefix,
            targets,
        } = &routes;
        let (prefix, targets) = (*prefix, targets);
        let receiver = self.lower_value(left)?;
        let dispatch = self.lower_receiver_adjustments(receiver, &chains[0][..prefix])?;

        // dispatch on the value's own discriminant, loading it behind stored receivers
        let received = self.value_representation(dispatch)?;
        let stored = match self.builder.tree().type_definition(received).clone() {
            // switch a bare variant receiver on its own case
            mir::Type::Variant { .. } => {
                self.builder.variant_switch(dispatch, None, targets.clone());

                false
            }
            // load the encoded tag behind a stored union receiver
            mir::Type::Reference { pointee, .. } | mir::Type::Pointer { pointee, .. } => {
                let tag = self.builder.variant_tag_load(
                    mir::Place::value(dispatch).with_projection(mir::Projection::Deref),
                    pointee,
                );
                let cases = targets
                    .iter()
                    .map(|(position, block)| (i128::from(*position), *block))
                    .collect();
                let unreached = self.builder.block();
                self.builder.switch(tag, unreached, cases);
                self.builder.switch_to_block(unreached);
                self.builder.unreachable();

                true
            }
            // reject every other receiver representation
            _ => {
                return Err(CompilerError::Internal {
                    message: "a union member read outside a variant representation".to_string(),
                });
            }
        };

        // join stored field arms by address
        if stored && let Some(storages) = self.union_field_storage(arms)? {
            return self
                .lower_union_field_address_read(expression, dispatch, arms, &routes, &storages);
        }

        // run each arm's narrowed read and join the member values
        let ty = self.node_type_id(expression)?;
        let representation = self.lower_type(ty)?;
        let slot = self
            .builder
            .local(representation, mir::Mutability::Immutable);
        let exit = self.builder.block();
        for ((arm, chain), &(_, block)) in arms.iter().zip(chains).zip(targets) {
            self.builder.switch_to_block(block);

            // select the payload and apply its recorded receiver adjustments
            let narrowed = self.lower_receiver_adjustments(dispatch, &chain[prefix..])?;

            // read the member at the arm's selected target
            let value = match &arm.target {
                dir::MemberTarget::Field(field) => {
                    self.lower_field_value(expression, narrowed, field)?
                }
                dir::MemberTarget::Projection { projection, .. } => {
                    self.lower_member_projection(narrowed, projection)?
                }
                other => {
                    return Err(self.unsupported(format!(
                        "a '{}' member read on a union receiver",
                        other.name()
                    )));
                }
            };

            // join the arm value at the member's committed representation
            self.builder.local_set(slot, value);
            self.builder.jump(exit);
        }

        // continue lowering at the exit block
        self.builder.switch_to_block(exit);

        Ok(self.builder.local_get(slot))
    }

    /// Return every arm's field storage when the arms read one addressed field alike.
    fn union_field_storage(
        &mut self,
        arms: &[dir::MemberAccess],
    ) -> CompilerResult<Option<Vec<FieldStorage>>> {
        let mut storages = Vec::with_capacity(arms.len());
        for arm in arms {
            let dir::MemberTarget::Field(field) = &arm.target else {
                return Ok(None);
            };
            if self
                .lower
                .indirection(field.receiver.ty(), &self.scope)?
                .is_none()
            {
                return Ok(None);
            }
            storages.push(self.field_storage(field)?);
        }

        // require one read representation across the arms
        let shared = storages
            .iter()
            .all(|storage| storage.read == storages[0].read);

        Ok(shared.then_some(storages))
    }

    /// Dispatch one field read over addressed union arms, joining the field addresses.
    fn lower_union_field_address_read(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        dispatch: mir::Value,
        arms: &[dir::MemberAccess],
        routes: &UnionDispatch,
        storages: &[FieldStorage],
    ) -> CompilerResult<mir::Value> {
        let prefix = routes.prefix;
        let exit = self.builder.block();
        let mut slot = None;
        for (((arm, chain), &(_, block)), storage) in arms
            .iter()
            .zip(&routes.chains)
            .zip(&routes.targets)
            .zip(storages)
        {
            self.builder.switch_to_block(block);

            // select the payload and apply its recorded receiver adjustments
            let narrowed = self.lower_receiver_adjustments(dispatch, &chain[prefix..])?;
            let dir::MemberTarget::Field(field) = &arm.target else {
                return Err(CompilerError::Internal {
                    message: "an address-joined union arm outside a field read".to_string(),
                });
            };

            // join the field address at the arms' shared address representation
            let Some(address) =
                self.innermost_field_address(narrowed, field.receiver.ty(), storage)?
            else {
                return Err(CompilerError::Internal {
                    message: "an address-joined union arm behind a value receiver".to_string(),
                });
            };
            let slot = match slot {
                Some(slot) => slot,
                None => {
                    let representation = self.value_representation(address)?;
                    *slot.insert(
                        self.builder
                            .local(representation, mir::Mutability::Immutable),
                    )
                }
            };
            self.builder.local_set(slot, address);
            self.builder.jump(exit);
        }
        let Some(slot) = slot else {
            return Err(CompilerError::Internal {
                message: "a union field read without arms".to_string(),
            });
        };

        // load the joined field once at the exit
        self.builder.switch_to_block(exit);
        let address = self.builder.local_get(slot);
        let place = mir::Place::value(address).with_projection(mir::Projection::Deref);
        let value = self.load_place(place, storages[0].read);

        self.lower_narrowing(expression, value)
    }
}

/// One field's storage index and read representation within its receiver.
pub(in crate::lower) struct FieldStorage {
    /// The field's index within the receiver's aggregate.
    pub(in crate::lower) index: u32,
    /// The representation the read produces.
    pub(in crate::lower) read: mir::TypeId,
}
