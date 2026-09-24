use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::lower::Binding;
use crate::{CompilerError, CompilerResult};

/// One entry stored in a synthesized closure environment.
enum EnvironmentEntry {
    /// A captured binding stored by value.
    Direct {
        /// The captured symbol.
        symbol: dir::GlobalSymbolId,
        /// The captured symbol's declared type.
        ty: dir::GlobalTypeId,
    },
    /// The captured receiver stored by value.
    This {
        /// The receiver's declared type.
        ty: dir::GlobalTypeId,
    },
    /// A lifted managed frame stored by reference.
    Frame {
        /// The lifted frame the closure reads through.
        frame: dir::LocalCaptureFrameId,
    },
}

impl FunctionLowerer<'_, '_, '_> {
    /// Release one moved-out environment allocation as uninitialized storage.
    pub(in crate::lower) fn release_emptied(
        &mut self,
        environment: mir::Value,
        reference: mir::TypeId,
    ) {
        let emptied = self.builder.tree_mut().emptied_type(reference);
        let environment = self
            .builder
            .cast(mir::CastOperator::Bitcast, environment, emptied);
        self.builder.release(environment);
    }

    /// Bind the captured environment of one closure body, a once body taking it whole.
    pub(in crate::lower) fn bind_captures(
        &mut self,
        symbol: dir::GlobalSymbolId,
        is_entry: bool,
    ) -> CompilerResult<()> {
        let Some(capture) = self.source().captures.capture(symbol) else {
            return Ok(());
        };
        if capture.captures.is_empty() && capture.this.is_none() {
            return Ok(());
        }

        // receive the environment at the kind the callable owns it in
        let capture = capture.clone();
        let entries = self.environment_entries(&capture)?;
        let kind = self.environment_kind(&capture);
        let (pointee, reference) = self.environment_types(&entries, kind)?;
        let environment = self.received_environment(reference);

        // take an owned environment out of its allocation, an entry leaving it to its body
        let taken = match (kind, is_entry) {
            (mir::Reference::Unique, false) => {
                let place = mir::Place::value(environment).with_projection(mir::Projection::Deref);
                let taken = self.load_place(place, pointee);
                self.release_emptied(environment, reference);

                Some(taken)
            }
            _ => None,
        };

        // unpack values, the receiver, and frame references from the environment
        for (index, entry) in entries.iter().enumerate() {
            let index = index as u32;
            match entry {
                EnvironmentEntry::Direct { symbol, ty } => {
                    let ty = self.lower_type(*ty)?;
                    let binding = match taken {
                        // home a taken value in the frame
                        Some(taken) => {
                            let value = self.builder.field_get(taken, index);
                            let local = self.builder.local(ty, mir::Mutability::Mutable);
                            self.builder.local_set(local, value);

                            Binding::Local(local)
                        }
                        // read a shared value through its environment field
                        None => Binding::Captured {
                            frame: environment,
                            field: index,
                            ty,
                        },
                    };
                    self.values.insert(symbol.local_id, binding);
                }
                EnvironmentEntry::This { ty } => {
                    let ty = self.lower_type(*ty)?;
                    let value = self.environment_field(taken, environment, index, ty)?;
                    let local = self.home(value);
                    self.this = Some(Binding::Local(local));
                }
                EnvironmentEntry::Frame { frame } => {
                    let frame = self.source().captures.get_frame(*frame).clone();
                    let frame_type = self.lower_type(frame.ty)?;
                    let loaded = self.environment_field(taken, environment, index, frame_type)?;
                    self.frames.insert(frame.scope, loaded);
                    self.bind_frame_captures(&capture, &frame, frame_type, loaded)?;
                }
            }
        }

        Ok(())
    }

    /// Read one environment field, off the taken value or through the environment reference.
    fn environment_field(
        &mut self,
        taken: Option<mir::Value>,
        environment: mir::Value,
        index: u32,
        ty: mir::TypeId,
    ) -> CompilerResult<mir::Value> {
        Ok(match taken {
            Some(taken) => self.builder.field_get(taken, index),
            None => self.read_binding(Binding::Captured {
                frame: environment,
                field: index,
                ty,
            })?,
        })
    }

    /// Return the kind one closure owns its environment in, as its capture record decides.
    fn environment_kind(&self, capture: &dir::Capture) -> mir::Reference {
        match capture.ownership {
            dir::Ownership::Owned => mir::Reference::Unique,
            _ => mir::Reference::Managed(mir::Space::Local),
        }
    }

    /// Bind the fields of one lifted frame the closure captures through it.
    fn bind_frame_captures(
        &mut self,
        capture: &dir::Capture,
        frame: &dir::CaptureFrame,
        frame_type: mir::TypeId,
        environment: mir::Value,
    ) -> CompilerResult<()> {
        for (field, entry) in frame.fields.iter().enumerate() {
            let is_captured = capture.captures.iter().any(|captured| {
                matches!(captured, dir::CapturedBinding::Manage { symbol, .. } if *symbol == entry.symbol)
            });
            if !is_captured {
                continue;
            }
            let field = field as u32;
            let ty = self.frame_field_type(frame_type, field)?;
            self.values.insert(
                entry.symbol.local_id,
                Binding::Captured {
                    frame: environment,
                    field,
                    ty,
                },
            );
        }

        Ok(())
    }

    /// Return the environment reference type one capturing callable receives.
    pub(in crate::lower) fn capture_environment_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<mir::TypeId>> {
        let Some(capture) = self.source().captures.capture(symbol) else {
            return Ok(None);
        };
        if capture.captures.is_empty() && capture.this.is_none() {
            return Ok(None);
        }
        let capture = capture.clone();
        let entries = self.environment_entries(&capture)?;
        let kind = self.environment_kind(&capture);

        Ok(Some(self.environment_types(&entries, kind)?.1))
    }

    /// Return the environment frame of one closure creation, when it captures.
    pub(in crate::lower) fn capture_environment(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<mir::Value>> {
        let Some(capture) = self.source().captures.capture(symbol) else {
            return Ok(None);
        };
        if capture.captures.is_empty() && capture.this.is_none() {
            return Ok(None);
        }

        // aggregate values, the receiver, and frame references on the heap
        let capture = capture.clone();
        let entries = self.environment_entries(&capture)?;
        let mut values = Vec::with_capacity(entries.len());
        for entry in &entries {
            let value = match entry {
                EnvironmentEntry::Direct { symbol, .. } => {
                    let Some(binding) = self.values.get(&symbol.local_id).copied() else {
                        return Err(CompilerError::Internal {
                            message: "a captured binding without a home".to_string(),
                        });
                    };

                    self.read_binding(binding)?
                }
                EnvironmentEntry::This { ty } => {
                    let Some(binding) = self.this else {
                        return Err(CompilerError::Internal {
                            message: "a captured receiver outside a method".to_string(),
                        });
                    };
                    let value = self.read_binding(binding)?;

                    self.constructed_this_at(*ty, value)?
                }
                EnvironmentEntry::Frame { frame } => {
                    let frame = self.source().captures.get_frame(*frame).clone();

                    self.allocate_frame_maybe(frame.scope, frame.ty)?
                }
            };
            values.push(value);
        }

        // build the environment the body reads its captures out of
        let kind = self.environment_kind(&capture);
        let (pointee, reference) = self.environment_types(&entries, kind)?;
        let aggregate = self.builder.aggregate(pointee, values);

        Ok(Some(self.builder.new_complete(aggregate, reference)))
    }

    /// Return the capture environment this body receives, either forwarded or read from the frame.
    fn received_environment(&mut self, reference: mir::TypeId) -> mir::Value {
        match self.captures {
            Some(environment) => environment,
            None => self.builder.function_environment_current(reference),
        }
    }

    /// Return the entries one environment stores: the captures, the receiver, then each frame.
    fn environment_entries(
        &mut self,
        capture: &dir::Capture,
    ) -> CompilerResult<Vec<EnvironmentEntry>> {
        let mut entries = Vec::new();
        for captured in &capture.captures {
            match captured {
                dir::CapturedBinding::Manage { .. } => {}
                dir::CapturedBinding::Copy { symbol, ty }
                | dir::CapturedBinding::Move { symbol, ty } => {
                    entries.push(EnvironmentEntry::Direct {
                        symbol: *symbol,
                        ty: *ty,
                    });
                }
                dir::CapturedBinding::Borrow { .. } => {
                    return Err(self.unsupported("a 'borrow' closure capture"));
                }
            }
        }
        if let Some(receiver) = &capture.this {
            match receiver.mode {
                dir::CaptureMode::Copy | dir::CaptureMode::Move | dir::CaptureMode::Manage => {
                    entries.push(EnvironmentEntry::This { ty: receiver.ty });
                }
                dir::CaptureMode::Borrow => {
                    return Err(self.unsupported("a 'borrow' receiver capture"));
                }
            }
        }
        for frame in &capture.frames {
            entries.push(EnvironmentEntry::Frame { frame: *frame });
        }

        Ok(entries)
    }

    /// Intern the struct and reference types of one synthesized environment.
    fn environment_types(
        &mut self,
        entries: &[EnvironmentEntry],
        kind: mir::Reference,
    ) -> CompilerResult<(mir::TypeId, mir::TypeId)> {
        let mut slots = Vec::with_capacity(entries.len());
        for entry in entries {
            let ty = match entry {
                EnvironmentEntry::Direct { ty, .. } | EnvironmentEntry::This { ty } => {
                    self.lower_type(*ty)?
                }
                EnvironmentEntry::Frame { frame } => {
                    let frame = self.source().captures.get_frame(*frame).clone();

                    self.lower_type(frame.ty)?
                }
            };
            slots.push(ty);
        }

        Ok(self.environment_reference_types(&slots, kind))
    }

    /// Intern the struct holding one environment's slots and the reference addressing it.
    pub(in crate::lower) fn environment_reference_types(
        &mut self,
        slots: &[mir::TypeId],
        kind: mir::Reference,
    ) -> (mir::TypeId, mir::TypeId) {
        let fields = slots
            .iter()
            .map(|ty| {
                self.builder.tree_mut().intern_field(mir::Field {
                    name: None,
                    ty: *ty,
                    attributes: Vec::new(),
                })
            })
            .collect();
        let pointee = self
            .builder
            .tree_mut()
            .intern_type(mir::Type::Struct { fields });
        let lifetime = match kind {
            mir::Reference::Borrowed => mir::Lifetime::frame(),
            _ => mir::Lifetime::empty(),
        };
        let reference = self.builder.tree_mut().intern_type(mir::Type::Reference {
            kind,
            lifetime,
            access: mir::Access::Mutable,
            pointee,
        });

        (pointee, reference)
    }

    /// Bind one declared symbol through its lifted frame, when one lifts it.
    pub(in crate::lower) fn bind_lifted(
        &mut self,
        symbol: dir::GlobalSymbolId,
        value: mir::Value,
    ) -> CompilerResult<bool> {
        let Some((scope, frame_ty, field)) = self.lifted_field(symbol) else {
            return Ok(false);
        };

        // write the value through the frame and bind the field as its home
        let frame = self.allocate_frame_maybe(scope, frame_ty)?;
        let frame_type = self.lower_type(frame_ty)?;
        let ty = self.frame_field_type(frame_type, field)?;
        let place = mir::Place::value(frame)
            .with_projection(mir::Projection::Deref)
            .with_projection(mir::Projection::Field { index: field });
        self.builder.store(place, value);
        self.values
            .insert(symbol.local_id, Binding::Captured { frame, field, ty });

        Ok(true)
    }

    /// Return whether a nested scope captures one symbol into a frame.
    pub(in crate::lower) fn is_lifted(&self, symbol: dir::GlobalSymbolId) -> bool {
        self.lifted_field(symbol).is_some()
    }

    /// Return the lifted frame field declared for one symbol, when one lifts it.
    fn lifted_field(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<(dir::GlobalScopeId, dir::GlobalTypeId, u32)> {
        // find the frame field declared for this symbol
        for (_, frame) in self.source().captures.iter_frames() {
            if let Some(index) = frame.fields.iter().position(|field| field.symbol == symbol) {
                return Some((frame.scope, frame.ty, index as u32));
            }
        }

        None
    }

    /// Return the frame allocated for one scope, allocating it on first touch.
    fn allocate_frame_maybe(
        &mut self,
        scope: dir::GlobalScopeId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<mir::Value> {
        if let Some(frame) = self.frames.get(&scope) {
            return Ok(*frame);
        }

        // allocate zeroed managed storage for the frame activation
        let frame_type = self.lower_type(ty)?;
        let mir::Type::Reference { pointee, .. } = self.builder.tree().get(frame_type) else {
            return Err(CompilerError::Internal {
                message: "a capture frame outside a managed reference".to_string(),
            });
        };
        let pointee = *pointee;
        let space = self.allocation_space(frame_type)?;
        let frame = self.builder.new_zeroed(pointee, frame_type, space);
        self.frames.insert(scope, frame);

        Ok(frame)
    }

    /// Return the stored field type at one index of a lifted frame.
    fn frame_field_type(&self, frame_type: mir::TypeId, field: u32) -> CompilerResult<mir::TypeId> {
        // peel the managed reference down to its struct storage
        let tree = self.builder.tree();
        let mir::Type::Reference { pointee, .. } = tree.get(frame_type) else {
            return Err(CompilerError::Internal {
                message: "a capture frame outside a managed reference".to_string(),
            });
        };

        // read the frame's field list out of its struct storage
        let mir::Type::Struct { fields, .. } = tree.get(*pointee) else {
            return Err(CompilerError::Internal {
                message: "a capture frame outside struct storage".to_string(),
            });
        };

        Ok(tree.get(fields[field as usize]).ty)
    }
}
