use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::lower::Binding;
use crate::lower::function::member::FieldStorage;
use crate::{CompilerError, CompilerResult};

/// One resolved place base.
#[derive(Clone, Copy)]
pub(in crate::lower) enum PlaceRoot {
    /// A mutable local holding the base aggregate.
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
        ty: mir::LocalNodeId<mir::Type>,
    },
    /// One union case selected behind the variant discriminant.
    Downcast {
        /// The case payload type, resolved to its case in the addressed variant.
        ty: mir::LocalNodeId<mir::Type>,
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
                dir::Dereference {
                    target: dir::DereferenceTarget::Direct,
                    ..
                },
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
    pub(in crate::lower) fn resolved_type(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> mir::LocalNodeId<mir::Type> {
        self.builder.tree().represented(ty)
    }

    /// Return whether one home holds a reference a place can root at.
    fn is_reference_local(&self, held: mir::LocalNodeId<mir::Type>) -> bool {
        matches!(
            self.builder.tree().get(self.resolved_type(held)),
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
        let node = expression.into_global_any(self.source);
        let Some(narrowing) = self.source().decisions.narrowing(node).cloned() else {
            return Ok(false);
        };

        Ok(self.union_leaves(&narrowing.arms)?.len() > 1)
    }

    /// Project one place onto the one case a read's narrowing proves, else keep it whole.
    fn narrowed_place(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        place: Place,
    ) -> CompilerResult<Place> {
        let node = expression.into_global_any(self.source);
        let Some(narrowing) = self.source().decisions.narrowing(node).cloned() else {
            return Ok(place);
        };
        match self.union_leaves(&narrowing.arms)?.as_slice() {
            [member] => self.downcast_place(place, *member),
            _ => Ok(place),
        }
    }

    /// Root one place holding a reference at the storage the reference addresses.
    pub(in crate::lower) fn through_handle(&mut self, place: Place) -> CompilerResult<Place> {
        let held = self.place_type(&place)?;
        let held = self.resolved_type(held);
        if !matches!(
            self.builder.tree().get(held),
            mir::Type::Reference { .. } | mir::Type::Pointer { .. }
        ) {
            return Ok(place);
        }
        let value = self.read_place(&place)?;
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
                // unwrap a newtype layer as its payload field, a transparent layer staying put
                dir::ReceiverAdjustment::NewtypePayload { ty, .. } => {
                    let held = self.place_type(&place)?;
                    let held = self.builder.tree().represented(held);
                    if !matches!(self.builder.tree().get(held), mir::Type::Newtype { .. }) {
                        continue;
                    }
                    let ty = *ty;
                    let ty = self.lower_type(ty)?;
                    place.path.push(PlaceProjection::Field { field: 0, ty });
                }
                // read the reference and root the place behind it, a view strip keeping it
                dir::ReceiverAdjustment::Dereference(dereference) => {
                    if self.is_view_strip(dereference)? {
                        continue;
                    }
                    let value = self.read_place(&place)?;
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
                // project a flow-proven union case behind its discriminant, a niched union whole
                dir::ReceiverAdjustment::UnionPayload { ty, .. } => {
                    let held = self.place_type(&place)?;
                    let held = self.builder.tree().represented(held);
                    if !matches!(self.builder.tree().get(held), mir::Type::Variant { .. }) {
                        continue;
                    }
                    let ty = *ty;
                    let ty = self.lower_type(ty)?;
                    place.path.push(PlaceProjection::Downcast { ty });
                }
                // report a borrow adjustment inside a place path
                dir::ReceiverAdjustment::Borrow { .. } => {
                    return Err(CompilerError::Internal {
                        message: "a place walking a borrow adjustment".to_string(),
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
        field: mir::LocalNodeId<mir::Type>,
        access: mir::Access,
    ) -> mir::Value {
        // interior addresses inherit the base reference's storage
        let mut pointee = field;
        let mut storage = mir::Storage::Heap(mir::Space::Local);
        if let Some(ty) = self.builder.value_type(reference)
            && let Some(base) = self.builder.tree().get(ty).reference_storage()
        {
            storage = base;
        }

        // project an uninitialized field out of an uninitialized aggregate
        if let Some(ty) = self.builder.value_type(reference)
            && let mir::Type::Reference {
                pointee: aggregate, ..
            } = self.builder.tree().get(ty)
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
            kind: mir::ReferenceKind::Borrowed,
            lifetime,
            storage,
            access,
            pointee,
        });

        self.builder
            .field_addr(reference, index, address, mir::AddressKind::Projection)
    }

    /// Load the references stored behind an address until it addresses an aggregate.
    fn innermost_address(&mut self, mut address: mir::Value) -> CompilerResult<mir::Value> {
        // load one reference layer at a time until the pointee is an aggregate
        loop {
            let pointee = self.reference_pointee(address)?;
            if !matches!(
                self.builder.tree().get(self.resolved_type(pointee)),
                mir::Type::Reference { .. } | mir::Type::Pointer { .. }
            ) {
                return Ok(address);
            }

            address = self.builder.load(address, pointee);
        }
    }

    /// Address the case of one variant reference holding one payload type.
    fn payload_address(
        &mut self,
        reference: mir::Value,
        payload: mir::LocalNodeId<mir::Type>,
        access: mir::Access,
        leaf: Option<(mir::LocalNodeId<mir::Type>, mir::AddressKind)>,
    ) -> CompilerResult<mir::Value> {
        // find the case in the addressed variant by its payload representation
        let received = self.value_representation(reference)?;
        let (storage, pointee) = match self.builder.tree().get(self.resolved_type(received)) {
            mir::Type::Reference {
                storage, pointee, ..
            } => (*storage, *pointee),
            _ => {
                return Err(CompilerError::Internal {
                    message: "a payload address through a non-reference value".to_string(),
                });
            }
        };
        let case = self.payload_case(pointee, payload)?;

        // address the payload at the leaf form the caller asked for, or borrow it
        let (address, kind) = match leaf {
            Some(leaf) => leaf,
            None => {
                let lifetime = self.reborrow_lifetime(reference);
                let address = self.builder.tree_mut().intern_type(mir::Type::Reference {
                    kind: mir::ReferenceKind::Borrowed,
                    lifetime,
                    storage,
                    access,
                    pointee: payload,
                });
                (address, mir::AddressKind::Projection)
            }
        };

        Ok(self
            .builder
            .variant_payload_addr(reference, case, address, kind))
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

    /// Return whether one local's value copies.
    pub(in crate::lower) fn copies_local(&self, local: mir::LocalNodeId<mir::Local>) -> bool {
        let ty = self.builder.tree().get(local).ty;

        self.copies(mir::TypeId::from(ty))
    }

    /// Return the type one place holds.
    pub(in crate::lower) fn place_type(
        &self,
        place: &Place,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        match place.path.last() {
            Some(PlaceProjection::Field { ty, .. } | PlaceProjection::Downcast { ty }) => Ok(*ty),
            None => match place.root {
                PlaceRoot::Local(local) => Ok(self.builder.tree().get(local).ty),
                PlaceRoot::Global(global) => Ok(self.builder.tree().get(global).ty),
                PlaceRoot::Reference { value, .. } => self.reference_pointee(value),
            },
        }
    }

    /// Read one place: a whole or copied local by value, every other place through its address.
    pub(in crate::lower) fn read_place(&mut self, place: &Place) -> CompilerResult<mir::Value> {
        if let PlaceRoot::Local(local) = place.root
            && (place.path.is_empty() || self.copies_local(local))
        {
            let mut value = self.builder.local_get(local);
            for projection in &place.path {
                value = match *projection {
                    PlaceProjection::Field { field, .. } => self.builder.field_get(value, field),
                    PlaceProjection::Downcast { ty } => {
                        let held = self.value_representation(value)?;
                        let variant = self.builder.tree().represented(held);
                        let case = self.payload_case(variant, ty)?;

                        self.builder.variant_payload(value, case)
                    }
                };
            }

            return Ok(value);
        }

        // load the leaf through its address
        let address = self.place_address(place, mir::Access::Readonly)?;
        let leaf = match place.path.last() {
            Some(PlaceProjection::Field { ty, .. } | PlaceProjection::Downcast { ty }) => *ty,
            None => self.reference_pointee(address)?,
        };

        Ok(self.builder.load(address, leaf))
    }

    /// Write one value through a place.
    pub(in crate::lower) fn write_place(
        &mut self,
        place: &Place,
        value: mir::Value,
    ) -> CompilerResult<()> {
        // store whole locals by value
        if let PlaceRoot::Local(local) = place.root
            && place.path.is_empty()
        {
            self.builder.local_set(local, value);

            return Ok(());
        }

        // store through the leaf address with the rights the root grants
        let address = self.place_address(place, mir::Access::Mutable)?;
        self.builder.store(address, value);

        Ok(())
    }

    /// Return the access one reference grants over its storage.
    pub(in crate::lower) fn rooted_access(
        &self,
        reference: mir::LocalNodeId<mir::Type>,
    ) -> Option<mir::Access> {
        let (mir::Type::Reference { kind, access, .. }
        | mir::Type::Slice { kind, access, .. }
        | mir::Type::Dynamic { kind, access, .. }
        | mir::Type::Function { kind, access, .. }) =
            *self.builder.tree().get(self.resolved_type(reference))
        else {
            // a handle outside this tree's representations grants its managed access
            return matches!(
                self.builder.tree().get(self.resolved_type(reference)),
                mir::Type::Application { .. }
            )
            .then_some(mir::Access::Mutable);
        };

        // a readonly reference lends readonly access whatever its kind
        if access == mir::Access::Readonly {
            return Some(access);
        }

        Some(match kind {
            mir::ReferenceKind::Borrowed => access,
            mir::ReferenceKind::Unique => mir::Access::Mutable,
            mir::ReferenceKind::Managed => mir::Access::Mutable,
        })
    }

    /// Return the pointee type of one reference value.
    fn reference_pointee(&self, value: mir::Value) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
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

    /// Address the storage one place selects at the requested access.
    pub(in crate::lower) fn place_address(
        &mut self,
        place: &Place,
        access: mir::Access,
    ) -> CompilerResult<mir::Value> {
        // address the root, an unprojected place ending there
        let (reference, access) = self.place_root_address(place, access)?;
        if place.path.is_empty() {
            return Ok(reference);
        }

        // walk the projection path to the leaf address
        let (address, _) = self.path_address(place.root, reference, access, &place.path, None)?;

        Ok(address)
    }

    /// Address the root of one place, answering the access the path may use.
    fn place_root_address(
        &mut self,
        place: &Place,
        access: mir::Access,
    ) -> CompilerResult<(mir::Value, mir::Access)> {
        match place.root {
            // address locals inside the frame
            PlaceRoot::Local(local) => {
                let pointee = self.builder.tree().get(local).ty;
                let address = self.root_address_type(pointee, mir::Storage::Frame, access);

                Ok((
                    self.builder
                        .local_addr(local, address, mir::AddressKind::Projection),
                    access,
                ))
            }
            // address globals in their space
            PlaceRoot::Global(global) => {
                let pointee = self.builder.tree().get(global).ty;
                let space = self.builder.tree().get(global).space;
                let granted = match space {
                    mir::Space::Local => mir::Access::Mutable,
                    mir::Space::Constant => mir::Access::Readonly,
                    mir::Space::Parameter(_) => {
                        return Err(CompilerError::Internal {
                            message: "a global in a parameter space".to_string(),
                        });
                    }
                    mir::Space::Shared => mir::Access::Mutable,
                };
                let access = access.min(granted);
                let address = self.root_address_type(pointee, mir::Storage::global(space), access);

                Ok((
                    self.builder
                        .global_addr(global, address, mir::AddressKind::Projection),
                    access,
                ))
            }
            // take the reference a rooted place already holds at its own access
            PlaceRoot::Reference {
                value,
                access: held,
            } => Ok((value, held.min(access))),
        }
    }

    /// Trap unless one addressed variant holds the case of a payload, a narrowing gone stale
    /// through an alias.
    fn check_payload_case(
        &mut self,
        reference: mir::Value,
        payload: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<()> {
        let variant = self.reference_pointee(reference)?;
        let case = self.payload_case(variant, payload)?;
        let stored = self.builder.tree().storage_type(mir::TypeId::from(variant));
        let mir::Type::Variant { cases, .. } = self.builder.tree().get(stored) else {
            return Err(CompilerError::Internal {
                message: "a case check outside a variant".to_string(),
            });
        };
        let discriminant = match cases[case as usize].discriminant {
            mir::Constant::UInt { value, .. } => value as i128,
            mir::Constant::Int { value, .. } => value,
            mir::Constant::Boolean { value } => value as i128,
            _ => {
                return Err(CompilerError::Internal {
                    message: "a case check on a non-scalar discriminant".to_string(),
                });
            }
        };
        let live = self.builder.block();
        let stale = self.builder.block();
        let tag = self.builder.variant_tag_load(reference, variant);
        self.builder.switch(tag, stale, vec![(discriminant, live)]);
        self.builder.switch_to_block(stale);
        self.builder.panic(None);
        self.builder.switch_to_block(live);

        Ok(())
    }

    /// Return whether one place root is owned storage: a frame local, a global, or a unique box.
    fn is_owned_root(&self, root: PlaceRoot) -> CompilerResult<bool> {
        let PlaceRoot::Reference { value, .. } = root else {
            return Ok(true);
        };
        let held = self.value_representation(value)?;

        Ok(self
            .builder
            .tree()
            .get(self.resolved_type(held))
            .is_unique_storage())
    }

    /// Return the lifetime a borrow taken through one reference value lives for: the borrow's own
    /// region, a managed object's while the borrow is held, a unique pointee's frame.
    pub(in crate::lower) fn reborrow_lifetime(&self, reference: mir::Value) -> mir::Lifetime {
        let Some(ty) = self.builder.value_type(reference) else {
            unreachable!("a reborrow through an untyped value");
        };
        match self.builder.tree().get(self.resolved_type(ty)) {
            mir::Type::Reference {
                kind: mir::ReferenceKind::Borrowed,
                lifetime,
                ..
            }
            | mir::Type::Slice {
                kind: mir::ReferenceKind::Borrowed,
                lifetime,
                ..
            } => lifetime.clone(),
            mir::Type::Reference {
                kind: mir::ReferenceKind::Managed,
                ..
            }
            | mir::Type::Slice {
                kind: mir::ReferenceKind::Managed,
                ..
            } => mir::Lifetime::managed(),
            _ => mir::Lifetime::frame(),
        }
    }

    /// Intern the borrowed reference type addressing one root place in its storage, a frame
    /// place living for the frame and a global for the program.
    fn root_address_type(
        &mut self,
        pointee: mir::LocalNodeId<mir::Type>,
        storage: mir::Storage,
        access: mir::Access,
    ) -> mir::LocalNodeId<mir::Type> {
        let lifetime = match storage.residence() {
            mir::Residence::Frame => mir::Lifetime::frame(),
            mir::Residence::Static => mir::Lifetime::static_storage(),
            mir::Residence::Heap => unreachable!("a root place address in heap storage"),
        };
        self.builder.tree_mut().intern_type(mir::Type::Reference {
            kind: mir::ReferenceKind::Borrowed,
            lifetime,
            storage,
            access,
            pointee,
        })
    }

    /// Borrow the storage one place selects at one reference type.
    pub(in crate::lower) fn borrow_place(
        &mut self,
        place: &Place,
        target: mir::LocalNodeId<mir::Type>,
        kind: mir::AddressKind,
    ) -> CompilerResult<mir::Value> {
        // reborrow a bare handle as the target form
        if place.path.is_empty()
            && let PlaceRoot::Reference { value, .. } = place.root
            && !matches!(
                self.builder.tree().get(self.value_representation(value)?),
                mir::Type::Reference { .. } | mir::Type::Pointer { .. }
            )
        {
            return Ok(self.builder.cast(mir::CastOperator::Bitcast, value, target));
        }

        // read a fat owner's descriptor at the borrowed form its target names
        let mir::Type::Reference { access, .. } = *self.builder.tree().get(target) else {
            let address = self.place_address(place, mir::Access::Readonly)?;

            return Ok(self.builder.load(address, target));
        };

        // borrow a whole root directly, reborrowing a bare reference as the target form
        if place.path.is_empty() {
            return Ok(match place.root {
                PlaceRoot::Local(local) => self.builder.local_addr(local, target, kind),
                PlaceRoot::Global(global) => self.builder.global_addr(global, target, kind),
                PlaceRoot::Reference { value, .. } => {
                    self.builder.cast(mir::CastOperator::Bitcast, value, target)
                }
            });
        }

        // borrow a projected place at its leaf address
        let (root, access) = self.place_root_address(place, access)?;
        let (address, _) =
            self.path_address(place.root, root, access, &place.path, Some((target, kind)))?;

        Ok(address)
    }

    /// Return the place one borrow expression addresses: a reference's pointee, a named place,
    /// or the frame slot any other value spills into.
    pub(in crate::lower) fn borrowed_place(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Place> {
        // reborrow indirect sources as a form change, owners keep their place
        let source = self.node_type_id(expression)?;
        if self.lower.indirection(source, &self.scope)?.is_some()
            && self.lower.ownership(source)? != dir::Ownership::Owned
        {
            let value = self.lower_value(expression)?;

            return self.place_behind(value);
        }

        // a dereference of a reference reborrows the reference itself
        if let dir::Expression::Unary {
            operator: dir::UnaryOperator::Dereference,
            right,
        } = *self.source().tree().get(expression)
            && self
                .lower
                .indirection(self.node_type_id(right)?, &self.scope)?
                .is_some()
        {
            let value = self.lower_value(right)?;

            return self.place_behind(value);
        }

        // reborrow the address an Index protocol read returns
        if let dir::Expression::Index {
            left, is_optional, ..
        } = *self.source().tree().get(expression)
            && let dir::OperationResolution::One(subscript) = self.subscript_decision(expression)?
            && let dir::SubscriptTarget::Index(read) = subscript.target
        {
            let address = self.lower_index_address(left, read, is_optional)?;

            return self.place_behind(address);
        }

        // address a place, spilling any other value into the frame
        match self.is_place_expression(expression) {
            true => self.receiver_place(expression),
            false => {
                let value = self.lower_value(expression)?;

                Ok(Place::local(self.home(value)))
            }
        }
    }

    /// Return the place one reference value addresses.
    pub(in crate::lower) fn place_behind(&mut self, value: mir::Value) -> CompilerResult<Place> {
        let received = self.value_representation(value)?;
        let Some(access) = self.rooted_access(received) else {
            return Err(CompilerError::Internal {
                message: format!(
                    "a place behind a non-reference value {:?}",
                    self.builder.tree().get(self.resolved_type(received))
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
        root: PlaceRoot,
        reference: mir::Value,
        access: mir::Access,
        path: &[PlaceProjection],
        leaf: Option<(mir::LocalNodeId<mir::Type>, mir::AddressKind)>,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
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
                        Some((result_type, kind)) => {
                            self.builder.field_addr(current, field, result_type, kind)
                        }
                        None => self.field_address(current, field, ty, access),
                    }
                }
                // project a case of owned storage under borrowck
                PlaceProjection::Downcast { ty } if self.is_owned_root(root)? => {
                    let current = self.innermost_address(current)?;
                    self.payload_address(current, ty, access, leaf)?
                }
                // project a case of aliasable storage behind a fresh tag check, an inline Copy
                // payload through a frame copy
                PlaceProjection::Downcast { ty } => {
                    let current = self.innermost_address(current)?;
                    self.check_payload_case(current, ty)?;
                    let is_inline_copy = self.copies(mir::TypeId::from(ty))
                        && !self
                            .builder
                            .tree()
                            .get(self.resolved_type(ty))
                            .is_reference_representation();
                    match leaf {
                        Some((result_type, kind)) if is_inline_copy => {
                            let slot =
                                self.payload_address(current, ty, mir::Access::Readonly, None)?;
                            let value = self.builder.load(slot, ty);
                            let local = self.home(value);

                            self.builder.local_addr(local, result_type, kind)
                        }
                        _ => self.payload_address(current, ty, access, leaf)?,
                    }
                }
            };
        }

        // answer the leaf type of the final projection
        let (PlaceProjection::Field { ty: leaf, .. } | PlaceProjection::Downcast { ty: leaf }) =
            path[path.len() - 1];

        Ok((current, leaf))
    }

    /// Lower one expression into a borrow of its place or reference.
    pub(in crate::lower) fn lower_borrowed_place(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        target: mir::LocalNodeId<mir::Type>,
        kind: mir::AddressKind,
    ) -> CompilerResult<mir::Value> {
        let place = self.borrowed_place(expression)?;

        self.borrow_place(&place, target, kind)
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
        } = *self.builder.tree().get(held)
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
            } = *self.builder.tree().get(stored)
            else {
                break;
            };
            value = self.builder.load(value, pointee);
            access = access.min(inner_access);
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
            let pointee = self.lower_type(layer.stored)?;
            value = self.builder.load(value, pointee);
            access = access.min(inner.access);
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
        )))
    }
}
