use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::lower::Binding;
use crate::lower::function::member::FieldStorage;
use crate::{CompilerError, CompilerResult};

/// One resolved place base.
#[derive(Clone, Copy)]
pub(in crate::lower) enum PlaceRoot {
    /// A local holding the base aggregate.
    Local(mir::LocalNodeId<mir::Local>),
    /// A module global holding the base aggregate.
    Global(mir::LocalNodeId<mir::Global>),
    /// A reference value addressing heap storage.
    Reference {
        /// The reference value.
        value: mir::Value,
        /// The access exposed through the reference.
        access: mir::Access,
    },
}

/// One typed projection step within a place.
#[derive(Clone, Copy)]
pub(in crate::lower) enum PlaceProjection {
    /// One field within its aggregate.
    Field {
        /// The field index within its aggregate.
        field: u32,
        /// The projected value type.
        ty: mir::TypeId,
    },
    /// One union case selected behind the variant discriminant.
    Downcast {
        /// The case index in the union order.
        case: u32,
        /// The case payload type.
        ty: mir::TypeId,
    },
}

/// One resolved place: a base with its projection path.
#[derive(Clone)]
pub(in crate::lower) struct Place {
    /// The base holding the outermost aggregate.
    pub(in crate::lower) root: PlaceRoot,
    /// The typed projections from the base to the place.
    pub(in crate::lower) path: Vec<PlaceProjection>,
}

impl FunctionLowerer<'_, '_, '_> {
    /// Return the place selected by one assignment resolution.
    pub(in crate::lower) fn place(
        &mut self,
        assignment: &dir::AssignmentDecision,
    ) -> CompilerResult<Place> {
        // read the target expression the assignment writes
        let Ok(source) = assignment
            .target
            .local_id
            .try_into_typed::<dir::Expression>()
        else {
            return Err(CompilerError::Internal {
                message: "a non-expression node in a place".to_string(),
            });
        };

        match &assignment.write {
            // write a plain binding
            dir::WriteResolution::Binding { symbol, .. } => self.binding_place(symbol.local_id),
            // write a field of the receiver's place
            dir::WriteResolution::Member(resolution) => {
                // require a direct, unadjusted field member
                let dir::OperationResolution::One(access) = resolution else {
                    return Err(self.unsupported("a field write on a union receiver"));
                };
                let dir::MemberTarget::Field(field) = &access.target else {
                    return Err(self.internal("a write through non-field member storage"));
                };
                let dir::MemberReceiver::Direct(receiver) = &field.receiver else {
                    return Err(self.unsupported("a field write through dynamic dispatch"));
                };

                // lower the storage selection
                let index = self.member_field_index(field)?;
                let ty = self.lower_type(field.ty)?;

                // read the receiver chain through its member resolutions
                let dir::Expression::Member { left, .. } = *self.source().tree().get(source) else {
                    return Err(CompilerError::Internal {
                        message: "a field write on a non-member place".to_string(),
                    });
                };

                // project the written field onto the receiver's adjusted place
                let place = self.receiver_place(left)?;
                let mut place = self.project_place_adjustments(place, &receiver.adjustments)?;
                place.path.push(PlaceProjection::Field { field: index, ty });

                Ok(place)
            }
            // write through the reference a dereference reads
            dir::WriteResolution::Dereference(dir::OperationResolution::One(
                dir::Dereference { protocol: None, .. },
            )) => {
                let dir::Expression::Unary { right, .. } = *self.source().tree().get(source) else {
                    return Err(CompilerError::Internal {
                        message: "a dereference write on a non-unary place".to_string(),
                    });
                };
                let value = self.lower_value(right)?;
                let received = self.value_representation(value)?;
                let Some(access) = self.rooted_access(received) else {
                    return Err(CompilerError::Internal {
                        message: "a dereference write through a non-reference value".to_string(),
                    });
                };

                Ok(Place {
                    root: PlaceRoot::Reference { value, access },
                    path: Vec::new(),
                })
            }
            // reject every other write target
            other => Err(self.unsupported(format!("a write through {} storage", other.name()))),
        }
    }

    /// Return one type through its lifetime application and representation.
    pub(in crate::lower) fn resolved_type(&mut self, ty: mir::TypeId) -> mir::TypeId {
        mir::Substitution::resolve(ty, self.builder.tree_mut())
    }

    /// Return whether one home holds a reference a place can root at.
    fn is_reference_local(&mut self, held: mir::TypeId) -> bool {
        let held = self.resolved_type(held);

        matches!(
            self.builder.tree().get(held),
            mir::Type::Reference { .. } | mir::Type::Pointer { .. }
        )
    }

    /// Return the declared type of the storage one expression names, a field by its declaration.
    pub(in crate::lower) fn storage_type(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if let dir::Expression::Member { .. } = *self.source().tree().get(expression)
            && let dir::OperationResolution::One(access) = self.member_decision(expression)?
            && let dir::MemberTarget::Field(field) = &access.target
        {
            return Ok(field.ty);
        }

        self.node_type_id(expression)
    }

    /// Return whether one expression's storage holds a reference, a binding judged by its home.
    pub(in crate::lower) fn indirect_storage(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<bool> {
        let binding = match *self.source().tree().get(expression) {
            dir::Expression::This => self.this,
            dir::Expression::Identifier { .. } => {
                let node = expression.into_global_any(self.source);
                let symbol = self.lower.resolved_symbol(node)?;

                self.values.get(&symbol.local_id).copied()
            }
            _ => None,
        };
        if let Some(binding) = binding {
            let held = self.binding_representation(binding);

            return Ok(self.is_reference_local(held));
        }
        let declared = self.storage_type(expression)?;

        Ok(self.lower.indirection(declared, &self.scope)?.is_some()
            && self.lower.ownership(declared)? != dir::Ownership::Owned)
    }

    /// Return the place behind one receiver expression, peeling its member chain.
    pub(in crate::lower) fn receiver_place(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Place> {
        // root the place at the uninitialized storage a constructor's receiver fills
        if let dir::Expression::This = *self.source().tree().get(expression)
            && self.this_fills_uninitialized_storage()
        {
            let storage = self.lower_this_storage(expression)?;

            return self.place_behind(storage);
        }

        // root the place at the reference value for indirect receivers, owners keep their place
        if self.indirect_storage(expression)? {
            return self.reference_place(expression);
        }

        self.storage_place(expression)
    }

    /// Return the place one expression's own storage occupies.
    fn storage_place(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Place> {
        match *self.source().tree().get(expression) {
            // a written dereference names the place behind its reference
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Dereference,
                right,
            } => self.reference_place(right),
            // project one more field of the member chain
            dir::Expression::Member { left, .. } => {
                // require a direct field member
                let resolution = self.member_decision(expression)?;
                let dir::OperationResolution::One(access) = &resolution else {
                    return Err(self.unsupported("a place projection on a union receiver"));
                };
                let dir::MemberTarget::Field(field) = &access.target else {
                    return Err(self.internal("a place projection through non-field storage"));
                };
                let dir::MemberReceiver::Direct(receiver) = &field.receiver else {
                    return Err(self.unsupported("a place projection through dynamic dispatch"));
                };

                // project this field onto the receiver's adjusted place, behind a held handle
                let index = self.member_field_index(field)?;
                let ty = self.lower_type(field.ty)?;
                let place = self.receiver_place(left)?;
                let place = self.project_place_adjustments(place, &receiver.adjustments)?;
                let mut place = self.through_handle(place)?;
                place.path.push(PlaceProjection::Field { field: index, ty });

                self.narrowed_place(expression, place)
            }
            // root the place at the receiver's home, narrowed to the case a read proves
            dir::Expression::This => {
                let Some(binding) = self.this else {
                    return Err(CompilerError::Internal {
                        message: "a this outside a method body".to_string(),
                    });
                };
                let place = self.binding_home(binding)?;

                self.narrowed_place(expression, place)
            }
            // bind the place base at the identifier, narrowed to the case a read proves
            dir::Expression::Identifier { .. } => {
                let node = expression.into_global_any(self.source);
                let symbol = self.lower.resolved_symbol(node)?;
                let place = self.binding_place(symbol.local_id)?;

                self.narrowed_place(expression, place)
            }
            // reject every other place expression
            ref other => Err(self.unsupported(format!(
                "a write through '{}' expressions",
                other.variant_name()
            ))),
        }
    }

    /// Return whether one read's narrowing keeps several cases live, converting the read value.
    pub(in crate::lower) fn narrows_onto_cases(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<bool> {
        Ok(self
            .representation_narrowing(expression)?
            .is_some_and(|narrowing| narrowing.arms.len() > 1))
    }

    /// Project one place onto the one case a read's narrowing proves, else keep it whole.
    fn narrowed_place(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        place: Place,
    ) -> CompilerResult<Place> {
        let Some(narrowing) = self.representation_narrowing(expression)? else {
            return Ok(place);
        };
        match narrowing.arms.as_slice() {
            [member] => self.downcast_place(place, narrowing.union, *member),
            _ => Ok(place),
        }
    }

    /// Root one place holding a reference at the storage the reference addresses.
    pub(in crate::lower) fn through_handle(&mut self, place: Place) -> CompilerResult<Place> {
        let held = self.place_type(&place)?;
        let held = self.resolved_type(held);
        if !matches!(
            self.builder.tree().type_definition(held),
            mir::Type::Reference { .. } | mir::Type::Pointer { .. }
        ) {
            return Ok(place);
        }
        let value = self.place_reborrow(&place)?;
        let Some((reference, access)) = self.innermost_reference(value, None)? else {
            return Ok(Place::local(self.home(value)));
        };

        Ok(Place {
            root: PlaceRoot::Reference {
                value: reference,
                access,
            },
            path: Vec::new(),
        })
    }

    /// Return whether one expression names storage: a binding, the receiver, or a field chain.
    pub(in crate::lower) fn names_storage(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<bool> {
        // find the binding the expression roots at
        let binding = match *self.source().tree().get(expression) {
            dir::Expression::This => self.this,
            dir::Expression::Identifier { .. } => {
                let node = expression.into_global_any(self.source);
                let symbol = self.lower.resolved_symbol(node)?;

                self.values.get(&symbol.local_id).copied()
            }
            // a field chain names the storage its receiver does
            dir::Expression::Member { left, .. } => {
                let dir::OperationResolution::One(access) = self.member_decision(expression)?
                else {
                    return Ok(false);
                };
                let dir::MemberTarget::Field(field) = &access.target else {
                    return Ok(false);
                };
                if !matches!(field.receiver, dir::MemberReceiver::Direct(_)) {
                    return Ok(false);
                }

                return self.names_storage(left);
            }
            // a builtin dereference names the storage behind its reference
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Dereference,
                right,
            } => {
                let dir::OperationResolution::One(dir::OperatorApplication::Unary {
                    target: dir::OperatorTarget::Builtin(_),
                    ..
                }) = self.operator_decision(expression)?
                else {
                    return Ok(false);
                };

                return Ok(self
                    .lower
                    .indirection(self.node_type_id(right)?, &self.scope)?
                    .is_some());
            }
            _ => None,
        };

        Ok(matches!(
            binding,
            Some(Binding::Local(_) | Binding::Captured { .. })
        ))
    }

    /// Home one value in a fresh local.
    pub(in crate::lower) fn home(&mut self, value: mir::Value) -> mir::LocalNodeId<mir::Local> {
        let ty = self
            .builder
            .value_type(value)
            .unwrap_or_else(|| unreachable!("a homed value is typed"));
        let local = self.builder.local(ty, mir::Mutability::Mutable);
        self.builder.local_set(local, value);

        local
    }

    /// Home one symbol's value in a local at the symbol's declared type.
    pub(in crate::lower) fn bind_local(
        &mut self,
        symbol: dir::GlobalSymbolId,
        value: mir::Value,
    ) -> CompilerResult<mir::LocalNodeId<mir::Local>> {
        let ty = self.lower.symbol_type(symbol)?;
        let slot = self.lower_type(ty)?;
        let local = self.builder.local(slot, mir::Mutability::Mutable);
        self.builder.local_set(local, value);
        self.values.insert(symbol.local_id, Binding::Local(local));

        Ok(local)
    }

    /// Bind one symbol to its value: a lifted binding in its frame, every other in a local.
    pub(in crate::lower) fn bind_symbol(
        &mut self,
        symbol: dir::GlobalSymbolId,
        value: mir::Value,
    ) -> CompilerResult<()> {
        if self.bind_lifted(symbol, value)? {
            return Ok(());
        }
        self.bind_local(symbol, value)?;

        Ok(())
    }

    /// Return the place behind one binding symbol.
    fn binding_place(&mut self, symbol: dir::LocalSymbolId) -> CompilerResult<Place> {
        let global = dir::GlobalSymbolId::new(self.source, symbol);

        match self.values.get(&symbol).copied() {
            // a homed binding roots its own place
            Some(binding) => self.binding_home(binding),
            // root module constants at their globals
            None => match self.constant_global(global)? {
                Some(global) => Ok(Place {
                    root: PlaceRoot::Global(global),
                    path: Vec::new(),
                }),
                None => Err(self.unsupported("a place through a module binding")),
            },
        }
    }

    /// Return the writable place rooted at one binding's home.
    pub(in crate::lower) fn binding_home(&self, binding: Binding) -> CompilerResult<Place> {
        match binding {
            // mutable locals root their own place
            Binding::Local(local) => Ok(Place {
                root: PlaceRoot::Local(local),
                path: Vec::new(),
            }),
            // captured bindings live behind their frame reference
            Binding::Captured { frame, field, ty } => Ok(Place {
                root: PlaceRoot::Reference {
                    value: frame,
                    access: mir::Access::Mutable,
                },
                path: vec![PlaceProjection::Field { field, ty }],
            }),
        }
    }

    /// Project one place through recorded receiver adjustment steps.
    pub(in crate::lower) fn project_place_adjustments(
        &mut self,
        mut place: Place,
        steps: &[dir::ReceiverAdjustment],
    ) -> CompilerResult<Place> {
        for step in steps {
            match step {
                // unwrap a newtype layer as its payload field, an intrinsic newtype as is
                dir::ReceiverAdjustment::NewtypePayload { key, .. } => {
                    let backing = match self.lower.definition(key.symbol)? {
                        Some(dir::Definition::Newtype(newtype)) => Some(newtype.backing),
                        _ => None,
                    };
                    let is_intrinsic = match backing {
                        Some(backing) => matches!(self.lower.ty(backing)?, dir::Type::Intrinsic),
                        None => false,
                    };
                    if !is_intrinsic {
                        let held = self.place_type(&place)?;
                        let held = mir::Substitution::resolve(held, self.builder.tree_mut());
                        let mir::Type::Newtype { inner, .. } =
                            *self.builder.tree().type_definition(held)
                        else {
                            return Err(self.internal("a newtype payload over a non-newtype place"));
                        };
                        place.path.push(PlaceProjection::Field {
                            field: 0,
                            ty: inner,
                        });
                    }
                }
                // read the reference and root the place behind it, a view keeping it
                dir::ReceiverAdjustment::Dereference(dereference) => {
                    if self.dereference_is_view(dereference)? {
                        continue;
                    }
                    let value = self.place_reborrow(&place)?;
                    let received = self.value_representation(value)?;
                    let Some(access) = self.rooted_access(received) else {
                        return Err(CompilerError::Internal {
                            message: "a place dereferencing a non-reference value".to_string(),
                        });
                    };
                    place = Place {
                        root: PlaceRoot::Reference { value, access },
                        path: Vec::new(),
                    };
                }
                // project the flow-proven union case, a niched union as is
                dir::ReceiverAdjustment::UnionPayload { union, arm, .. } => {
                    let Some(variant) = self.place_variant(&place)? else {
                        continue;
                    };
                    let members = self.lower.union_members(*union)?;
                    let case = self.case(&members, *arm)?;
                    let Some(ty) = self.builder.tree_mut().case_payload(variant, case) else {
                        return Err(self.internal("a union payload outside the variant cases"));
                    };
                    place.path.push(PlaceProjection::Downcast { case, ty });
                }
                // report a call adjustment inside a place path
                dir::ReceiverAdjustment::Borrow { .. } | dir::ReceiverAdjustment::Upcast { .. } => {
                    return Err(CompilerError::Internal {
                        message: "a place walking a call adjustment".to_string(),
                    });
                }
            }
        }

        Ok(place)
    }

    /// Project one field address through an aggregate reference.
    pub(in crate::lower) fn field_address(
        &mut self,
        reference: mir::Value,
        index: u32,
        field: mir::TypeId,
        access: mir::Access,
    ) -> CompilerResult<mir::Value> {
        let mut pointee = field;

        // project an uninitialized field out of an uninitialized aggregate
        if let Some(ty) = self.builder.value_type(reference)
            && let mir::Type::Reference {
                pointee: aggregate, ..
            } = self.builder.tree().type_definition(ty)
            && matches!(
                self.builder.tree().get(*aggregate),
                mir::Type::Uninit { .. }
            )
        {
            pointee = self
                .builder
                .tree_mut()
                .intern_type(mir::Type::Uninit { value: field });
        }

        // borrow interior addresses from the object reference, for as long as it lives
        let lifetime = self.reborrow_lifetime(reference);
        let address = self.builder.tree_mut().intern_type(mir::Type::Reference {
            kind: mir::Reference::Borrowed,
            lifetime,
            access,
            pointee,
        });

        let place = mir::Place::value(reference)
            .with_projection(mir::Projection::Deref)
            .with_projection(mir::Projection::Field { index });

        Ok(self.builder.address(place, address))
    }

    /// Read a stored reference as a borrowed view without transferring its owner.
    pub(in crate::lower) fn dereference(
        &mut self,
        pointer: mir::Value,
    ) -> CompilerResult<mir::Value> {
        let stored = self.reference_pointee(pointer)?;
        let held = self.value_representation(pointer)?;
        let through = self
            .rooted_access(held)
            .ok_or_else(|| CompilerError::Internal {
                message: "a dereference through a pointer without a rooted access".to_string(),
            })?;
        let place = mir::Place::value(pointer).with_projection(mir::Projection::Deref);

        // view a stored handle's object for the object's life, else for the pointer's extent
        let is_handle = matches!(
            self.builder.tree().type_definition(stored).reference_kind(),
            Some(mir::Reference::Managed(_))
        );
        let lifetime = if is_handle {
            self.reborrow_lifetime_of(stored)
        } else {
            self.reborrow_lifetime(pointer)
        };

        // load a raw pointer, read a handle out at the view type, else borrow the referent
        let Some(target) = self.reborrowed_type(stored, through, lifetime)? else {
            return Ok(self.load_place(place, stored));
        };
        if is_handle {
            // view a handle's object at the path's access
            let view = self.builder.tree().get(target).clone();
            let target = self
                .builder
                .tree_mut()
                .intern_type(view.with_reference_access(through));
            let handle = self.load_place(place, stored);

            return Ok(self
                .builder
                .cast(mir::CastOperator::Bitcast, handle, target));
        }

        Ok(self
            .builder
            .address(place.with_projection(mir::Projection::Deref), target))
    }

    /// Return the access one global's space grants.
    fn global_access(&self, global: mir::LocalNodeId<mir::Global>) -> mir::Access {
        match self.builder.tree().get(global).space {
            mir::Space::Local | mir::Space::Shared => mir::Access::Mutable,
            mir::Space::Constant => mir::Access::Readonly,
        }
    }

    /// Reborrow the reference one place holds through the place itself.
    pub(in crate::lower) fn place_reborrow(&mut self, place: &Place) -> CompilerResult<mir::Value> {
        let held = self.place_type(place)?;
        let held = self.resolved_type(held);
        let slot = place.lower(self)?;

        // grant at most the access the root lends
        let through = match place.root {
            PlaceRoot::Reference { access, .. } => access,
            PlaceRoot::Local(_) => mir::Access::Exclusive,
            PlaceRoot::Global(global) => self.global_access(global),
        };
        let lifetime = self.reborrow_lifetime_of(held);

        // load a raw pointer, else borrow the referent
        let Some(target) = self.reborrowed_type(held, through, lifetime)? else {
            return Ok(self.load_place(slot, held));
        };

        Ok(self
            .builder
            .address(slot.with_projection(mir::Projection::Deref), target))
    }

    /// Return one stored reference's type reborrowed at a lifetime and access, none for a pointer.
    fn reborrowed_type(
        &mut self,
        held: mir::TypeId,
        through: mir::Access,
        lifetime: mir::Lifetime,
    ) -> CompilerResult<Option<mir::TypeId>> {
        let mut target = self.builder.tree().type_definition(held).clone();
        match &mut target {
            mir::Type::Reference {
                kind,
                lifetime: extent,
                access,
                ..
            }
            | mir::Type::Slice {
                kind,
                lifetime: extent,
                access,
                ..
            }
            | mir::Type::Dynamic {
                kind,
                lifetime: extent,
                access,
                ..
            }
            | mir::Type::Function {
                kind,
                lifetime: extent,
                access,
                ..
            } => {
                *kind = mir::Reference::Borrowed;
                *extent = lifetime;
                *access = access.meet(through);
            }
            mir::Type::Pointer { .. } => return Ok(None),
            _ => return Err(self.internal("a reborrow of storage without a reference")),
        }

        Ok(Some(self.builder.tree_mut().intern_type(target)))
    }

    /// Reborrow one borrowed or unique reference at a target type.
    pub(in crate::lower) fn reborrow_or_reinterpret(
        &mut self,
        value: mir::Value,
        target: mir::TypeId,
    ) -> CompilerResult<mir::Value> {
        let received = self.value_representation(value)?;
        let kind = self
            .builder
            .tree()
            .type_definition(received)
            .reference_kind();

        Ok(match kind {
            Some(mir::Reference::Borrowed | mir::Reference::Unique) => {
                let referent = mir::Place::value(value).with_projection(mir::Projection::Deref);

                self.builder.address(referent, target)
            }
            _ => self.builder.cast(mir::CastOperator::Bitcast, value, target),
        })
    }

    /// Load the references stored behind an address until it addresses an aggregate.
    fn innermost_address(&mut self, mut address: mir::Value) -> CompilerResult<mir::Value> {
        // load one reference layer at a time until the pointee is an aggregate
        loop {
            let pointee = self.reference_pointee(address)?;
            let pointee = self.resolved_type(pointee);
            if !matches!(
                self.builder.tree().get(pointee),
                mir::Type::Reference { .. } | mir::Type::Pointer { .. }
            ) {
                return Ok(address);
            }

            address = self.dereference(address)?;
        }
    }

    /// Address the case of one variant reference holding one payload type.
    fn payload_address(
        &mut self,
        reference: mir::Value,
        case: u32,
        payload: mir::TypeId,
        access: mir::Access,
        leaf: Option<mir::TypeId>,
    ) -> CompilerResult<mir::Value> {
        // address the payload at the leaf form the caller asked for, or borrow it
        let address = match leaf {
            Some(leaf) => leaf,
            None => {
                let lifetime = self.reborrow_lifetime(reference);

                self.builder.tree_mut().intern_type(mir::Type::Reference {
                    kind: mir::Reference::Borrowed,
                    lifetime,
                    access,
                    pointee: payload,
                })
            }
        };

        let place = mir::Place::value(reference)
            .with_projection(mir::Projection::Deref)
            .with_projection(mir::Projection::Variant { case });

        Ok(self.builder.address(place, address))
    }

    /// Return the place one reference expression addresses, rooted at the reference value.
    fn reference_place(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Place> {
        let declared = self.storage_type(expression)?;
        let value = self.lower_value(expression)?;
        let Some((value, _)) = self.innermost_reference(value, Some(declared))? else {
            return Err(CompilerError::Internal {
                message: "a reference receiver without an indirection".to_string(),
            });
        };
        let received = self.value_representation(value)?;
        let received = self.resolved_type(received);
        let Some(access) = self.rooted_access(received) else {
            return Err(CompilerError::Internal {
                message: "a reference receiver without an indirection".to_string(),
            });
        };

        Ok(Place {
            root: PlaceRoot::Reference { value, access },
            path: Vec::new(),
        })
    }

    /// Return whether one expression names a place: a binding, a member chain, or a dereference.
    pub(in crate::lower) fn is_place_expression(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        matches!(
            self.source().tree().get(expression),
            dir::Expression::This
                | dir::Expression::Identifier { .. }
                | dir::Expression::Member { .. }
                | dir::Expression::Unary {
                    operator: dir::UnaryOperator::Dereference,
                    ..
                }
        )
    }

    /// Return the type one place holds.
    pub(in crate::lower) fn place_type(&mut self, place: &Place) -> CompilerResult<mir::TypeId> {
        match place.path.last() {
            Some(PlaceProjection::Field { ty, .. } | PlaceProjection::Downcast { ty, .. }) => {
                Ok(*ty)
            }
            None => match place.root {
                PlaceRoot::Local(local) => Ok(self.builder.tree().get(local).ty),
                PlaceRoot::Global(global) => Ok(self.builder.tree().get(global).ty),
                PlaceRoot::Reference { value, .. } => self.reference_pointee(value),
            },
        }
    }

    /// Read one place, loading its value.
    pub(in crate::lower) fn read_place(&mut self, place: &Place) -> CompilerResult<mir::Value> {
        let ty = self.place_type(place)?;
        let place = place.lower(self)?;

        Ok(self.load_place(place, ty))
    }

    /// Load one place, a moving reference reborrowed at each read instead.
    pub(in crate::lower) fn load_place(
        &mut self,
        place: mir::Place,
        ty: mir::TypeId,
    ) -> mir::Value {
        // reborrow every read of a non-copy borrow
        let is_reborrow = matches!(
            self.builder.tree().type_definition(ty),
            mir::Type::Reference { kind: mir::Reference::Borrowed, access, .. }
            | mir::Type::Slice { kind: mir::Reference::Borrowed, access, .. }
            if !access.copies()
        );
        if is_reborrow {
            let referent = place.with_projection(mir::Projection::Deref);

            return self.builder.address(referent, ty);
        }

        self.builder.load(place, ty)
    }

    /// Write one value through a place.
    pub(in crate::lower) fn write_place(
        &mut self,
        place: &Place,
        value: mir::Value,
    ) -> CompilerResult<()> {
        let place = place.lower(self)?;
        self.builder.store(place, value);

        Ok(())
    }

    /// Return the access one reference grants over its storage.
    pub(in crate::lower) fn rooted_access(
        &mut self,
        reference: mir::TypeId,
    ) -> Option<mir::Access> {
        let reference = self.resolved_type(reference);
        let (mir::Type::Reference { kind, access, .. }
        | mir::Type::Slice { kind, access, .. }
        | mir::Type::Dynamic { kind, access, .. }
        | mir::Type::Function { kind, access, .. }) = *self.builder.tree().get(reference)
        else {
            // a handle outside this tree's representations grants its managed access
            return matches!(
                self.builder.tree().get(reference),
                mir::Type::Application { .. }
            )
            .then_some(mir::Access::Mutable);
        };

        // a readonly reference lends readonly access whatever its kind
        if access == mir::Access::Readonly {
            return Some(access);
        }

        Some(match kind {
            mir::Reference::Borrowed | mir::Reference::Raw => access,
            mir::Reference::Unique => mir::Access::Mutable,
            mir::Reference::Managed(_) => mir::Access::Mutable,
        })
    }

    /// Return the pointee type of one reference value.
    pub(in crate::lower) fn reference_pointee(
        &mut self,
        value: mir::Value,
    ) -> CompilerResult<mir::TypeId> {
        let received = self.value_representation(value)?;
        let resolved = self.resolved_type(received);
        match self.builder.tree().get(resolved) {
            mir::Type::Reference { pointee, .. } => Ok(*pointee),
            // a handle outside this tree's representations addresses its own application
            mir::Type::Application { .. } => Ok(resolved),
            _ => Err(CompilerError::Internal {
                message: "a place rooted at a non-reference value".to_string(),
            }),
        }
    }

    /// Return the lifetime a borrow taken through one reference value lives for.
    pub(in crate::lower) fn reborrow_lifetime(&mut self, reference: mir::Value) -> mir::Lifetime {
        let Some(ty) = self.builder.value_type(reference) else {
            unreachable!("a reborrow through an untyped value");
        };

        self.reborrow_lifetime_of(ty)
    }

    /// Return the lifetime a borrow taken through one reference type lives for.
    fn reborrow_lifetime_of(&mut self, ty: mir::TypeId) -> mir::Lifetime {
        let ty = self.resolved_type(ty);

        match self.builder.tree().get(ty) {
            mir::Type::Reference {
                kind: mir::Reference::Borrowed,
                lifetime,
                ..
            }
            | mir::Type::Slice {
                kind: mir::Reference::Borrowed,
                lifetime,
                ..
            } => lifetime.clone(),
            mir::Type::Reference {
                kind: mir::Reference::Managed(_),
                ..
            }
            | mir::Type::Slice {
                kind: mir::Reference::Managed(_),
                ..
            } => mir::Lifetime::managed(),
            _ => mir::Lifetime::frame(),
        }
    }

    /// Borrow the storage one place selects at one reference type.
    pub(in crate::lower) fn borrow_place(
        &mut self,
        place: &Place,
        target: mir::TypeId,
    ) -> CompilerResult<mir::Value> {
        // reborrow a whole root through the reference it holds,
        //  a local only when the target borrows its referent
        if place.path.is_empty() {
            let reference = match place.root {
                PlaceRoot::Reference { value, .. } => Some(value),
                PlaceRoot::Local(local) => {
                    let held = self.builder.tree().get(local).ty;
                    self.borrows_referent(held, target)
                        .then(|| self.load_place(mir::Place::local(local), held))
                }
                PlaceRoot::Global(_) => None,
            };
            if let Some(reference) = reference {
                return self.reborrow_or_reinterpret(reference, target);
            }
        }

        // read a fat owner's descriptor at the borrowed form its target names
        let mir::Type::Reference { access, .. } = *self.builder.tree().get(target) else {
            let referent = place.lower(self)?.with_projection(mir::Projection::Deref);

            return Ok(self.builder.address(referent, target));
        };

        // address a frame or global place, a rooted reference through its path
        match place.root {
            PlaceRoot::Local(_) | PlaceRoot::Global(_) => {
                let projected = place.lower(self)?;

                Ok(self.builder.address(projected, target))
            }
            PlaceRoot::Reference {
                value,
                access: held,
            } => self.path_address(value, held.meet(access), &place.path, Some(target)),
        }
    }

    /// Return whether one borrow target borrows the referent of a held reference type.
    fn borrows_referent(&self, held: mir::TypeId, target: mir::TypeId) -> bool {
        let (
            mir::Type::Reference { pointee, .. },
            mir::Type::Reference {
                pointee: borrowed, ..
            },
        ) = (
            self.builder.tree().type_definition(held),
            self.builder.tree().type_definition(target),
        )
        else {
            return false;
        };

        pointee == borrowed
    }

    /// Return the place one borrow addresses: the expression's storage, else its referent.
    pub(in crate::lower) fn borrowed_place(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        target: mir::TypeId,
    ) -> CompilerResult<Place> {
        // reborrow the address an Index protocol read returns
        if let dir::Expression::Index {
            left, is_optional, ..
        } = *self.source().tree().get(expression)
            && let dir::OperationResolution::One(subscript) = self.subscript_decision(expression)?
            && let dir::SubscriptTarget::Index(read) = subscript.target
            && read.dereference.is_some()
        {
            let address = self.lower_index_address(left, read, is_optional)?;

            return self.place_behind(address);
        }

        // reborrow through the reference the expression holds when the target lies past it
        let source = self.node_type_id(expression)?;
        let held = self.lower_type(source)?;
        if !self.addresses_value(target, held) && self.reborrows_through(held) {
            let value = self.lower_value(expression)?;

            return self.place_behind(value);
        }

        // address a place, spilling a converted or computed value into the frame
        if self.is_place_expression(expression) && self.coercion(expression).is_none() {
            self.storage_place(expression)
        } else {
            let value = self.lower_value(expression)?;

            Ok(Place::local(self.home(value)))
        }
    }

    /// Return whether a borrow of one held value reborrows it; an owner keeps its storage.
    pub(in crate::lower) fn reborrows_through(&mut self, held: mir::TypeId) -> bool {
        let held = self.resolved_type(held);

        matches!(
            self.builder.tree().type_definition(held).reference_kind(),
            Some(mir::Reference::Borrowed | mir::Reference::Managed(_) | mir::Reference::Raw)
        )
    }

    /// Return whether one reference type addresses a value of the held type itself.
    pub(in crate::lower) fn addresses_value(
        &mut self,
        target: mir::TypeId,
        held: mir::TypeId,
    ) -> bool {
        match self.builder.tree().type_definition(target) {
            mir::Type::Reference { pointee, .. } => {
                self.resolved_type(*pointee) == self.resolved_type(held)
            }
            _ => false,
        }
    }

    /// Return the place one reference value addresses.
    pub(in crate::lower) fn place_behind(&mut self, value: mir::Value) -> CompilerResult<Place> {
        let received = self.value_representation(value)?;
        let received = self.resolved_type(received);
        let Some(access) = self.rooted_access(received) else {
            return Err(CompilerError::Internal {
                message: format!(
                    "a place behind a non-reference value {:?}",
                    self.builder.tree().get(received)
                ),
            });
        };

        Ok(Place {
            root: PlaceRoot::Reference { value, access },
            path: Vec::new(),
        })
    }

    /// Project one address chain to the leaf of a path, borrowing the leaf at the kind asked for.
    fn path_address(
        &mut self,
        reference: mir::Value,
        access: mir::Access,
        path: &[PlaceProjection],
        leaf: Option<mir::TypeId>,
    ) -> CompilerResult<mir::Value> {
        // require at least one projection to address
        if path.is_empty() {
            return Err(CompilerError::Internal {
                message: "a reference place addressed without a projection".to_string(),
            });
        }

        // descend one address per path element, reading through references stored on the way
        let mut current = reference;
        let last = path.len() - 1;
        for (index, projection) in path.iter().enumerate() {
            let leaf = (index == last).then_some(leaf).flatten();
            current = match *projection {
                PlaceProjection::Field { field, ty } => {
                    let current = self.innermost_address(current)?;
                    match leaf {
                        Some(result_type) => {
                            let place = mir::Place::value(current)
                                .with_projection(mir::Projection::Deref)
                                .with_projection(mir::Projection::Field { index: field });

                            self.builder.address(place, result_type)
                        }
                        None => self.field_address(current, field, ty, access)?,
                    }
                }
                // address the case selected by the narrowing
                PlaceProjection::Downcast { case, ty } => {
                    let current = self.innermost_address(current)?;
                    self.payload_address(current, case, ty, access, leaf)?
                }
            };
        }

        Ok(current)
    }

    /// Lower one expression into a borrow of its place or reference.
    pub(in crate::lower) fn lower_borrowed_place(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        target: mir::TypeId,
    ) -> CompilerResult<mir::Value> {
        let place = self.borrowed_place(expression, target)?;

        self.borrow_place(&place, target)
    }
}

impl FunctionLowerer<'_, '_, '_> {
    /// Load one reference down to the one addressing its aggregate, narrowing the access.
    pub(in crate::lower) fn innermost_reference(
        &mut self,
        value: mir::Value,
        declared: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<(mir::Value, mir::Access)>> {
        let held = self.value_representation(value)?;
        let held = self.resolved_type(held);
        let mir::Type::Reference {
            access, pointee, ..
        } = *self.builder.tree().type_definition(held)
        else {
            return self.declared_reference(value, declared);
        };

        // load each reference stored behind another
        let mut value = value;
        let mut access = access;
        let mut pointee = pointee;
        loop {
            let stored = self.resolved_type(pointee);
            let mir::Type::Reference {
                access: inner_access,
                pointee: inner,
                ..
            } = *self.builder.tree().type_definition(stored)
            else {
                break;
            };
            value = self.dereference(value)?;
            access = access.meet(inner_access);
            pointee = inner;
        }

        Ok(Some((value, access)))
    }

    /// Load one handle down by the layers its declared type writes.
    fn declared_reference(
        &mut self,
        value: mir::Value,
        declared: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<(mir::Value, mir::Access)>> {
        let Some(declared) = declared else {
            return Ok(None);
        };
        let Some(mut layer) = self.lower.indirection(declared, &self.scope)? else {
            return Ok(None);
        };
        let mut value = value;
        let mut access = layer.access;
        while let Some(inner) = self.lower.indirection(layer.stored, &self.scope)? {
            // stop at the implicit managed layer a bare reference family stores itself under
            if inner.stored == layer.stored {
                break;
            }
            value = self.dereference(value)?;
            access = access.meet(inner.access);
            layer = inner;
        }

        Ok(Some((value, access)))
    }

    /// Address one field through every reference layer of its receiver.
    pub(in crate::lower) fn innermost_field_address(
        &mut self,
        value: mir::Value,
        receiver: dir::GlobalTypeId,
        storage: &FieldStorage,
    ) -> CompilerResult<Option<mir::Value>> {
        let Some((reference, access)) = self.innermost_reference(value, Some(receiver))? else {
            return Ok(None);
        };

        Ok(Some(self.field_address(
            reference,
            storage.index,
            storage.read,
            access,
        )?))
    }
}

impl Place {
    /// Lower the selected storage to one MIR operand.
    pub(in crate::lower) fn lower(
        &self,
        lower: &mut FunctionLowerer<'_, '_, '_>,
    ) -> CompilerResult<mir::Place> {
        // select the root storage
        let (mut place, mut ty) = match self.root {
            PlaceRoot::Local(local) => {
                (mir::Place::local(local), lower.builder.tree().get(local).ty)
            }
            PlaceRoot::Global(global) => (
                mir::Place::global(global),
                lower.builder.tree().get(global).ty,
            ),
            PlaceRoot::Reference { value, .. } => (
                mir::Place::value(value).with_projection(mir::Projection::Deref),
                lower.reference_pointee(value)?,
            ),
        };

        // traverse stored references before selecting each aggregate member
        for projection in &self.path {
            loop {
                let resolved = lower.resolved_type(ty);
                match lower.builder.tree().get(resolved) {
                    mir::Type::Reference { pointee, .. } | mir::Type::Pointer { pointee, .. } => {
                        ty = *pointee;
                        place.push(mir::Projection::Deref);
                    }
                    _ => break,
                }
            }

            // retain the field or case and its type
            let (projection, selected) = match *projection {
                PlaceProjection::Field { field, ty } => {
                    (mir::Projection::Field { index: field }, ty)
                }
                PlaceProjection::Downcast { case, ty } => (mir::Projection::Variant { case }, ty),
            };
            place.push(projection);
            ty = selected;
        }

        Ok(place)
    }
}
