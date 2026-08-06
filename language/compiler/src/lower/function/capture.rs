use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::body::Binding;
use crate::{CompilerError, CompilerResult, LowerError};

impl FunctionLowerer<'_, '_, '_> {
    /// Bind the captured environment of one closure body.
    pub(in crate::lower) fn bind_captures(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        let Some(capture) = self.source().captures.capture(symbol) else {
            return Ok(());
        };
        if capture.captures.is_empty() && capture.this.is_none() {
            return Ok(());
        }

        // load the hidden environment as the lifted frame
        let capture = capture.clone();
        let frame_id = self.managed_frame(&capture)?;
        let frame = self.source().captures.get_frame(frame_id).clone();
        let carrier = self.lower_type(frame.ty)?;
        let environment = self.builder.function_environment_current(carrier);
        self.frames.insert(frame.scope, environment);

        // bind each captured symbol through its frame field
        for captured in &capture.captures {
            let symbol = captured.symbol();
            let Some(field) = frame.fields.iter().position(|entry| entry.symbol == symbol) else {
                return Err(CompilerError::Internal {
                    message: "a managed capture outside its lifted frame".to_string(),
                });
            };
            let field = field as u32;
            let ty = self.frame_field_type(carrier, field)?;
            self.values.insert(
                symbol.local_id,
                Binding::Captured {
                    frame: environment,
                    field,
                    ty,
                },
            );
        }

        Ok(())
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
        // allocate or reuse the frame this closure closes over
        let capture = capture.clone();
        let frame_id = self.managed_frame(&capture)?;
        let frame = self.source().captures.get_frame(frame_id).clone();
        let environment = self.allocate_frame_maybe(frame.scope, frame.ty)?;

        Ok(Some(environment))
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
        let carrier = self.lower_type(frame_ty)?;
        let ty = self.frame_field_type(carrier, field)?;
        let address = self.emit_field_address(frame, field, ty, mir::Access::Mutable);
        self.builder.store(address, value);
        self.values
            .insert(symbol.local_id, Binding::Captured { frame, field, ty });

        Ok(true)
    }

    /// Return the single managed frame one capture collapses onto.
    fn managed_frame(&self, capture: &dir::Capture) -> CompilerResult<dir::LocalCaptureFrameId> {
        // reject receiver captures, which have no lifted frame
        if capture.this.is_some() {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a closure capturing 'this'".to_string(),
            }
            .into());
        }
        // reject every capture mode but managed lifting
        for captured in &capture.captures {
            let mode = match captured.mode() {
                dir::CaptureMode::Manage => continue,
                dir::CaptureMode::Borrow => "borrow",
                dir::CaptureMode::Copy => "copy",
                dir::CaptureMode::Move => "move",
            };

            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: format!("a '{mode}' closure capture"),
            }
            .into());
        }

        // require exactly one frame to carry the capture
        match capture.frames.as_slice() {
            [frame] => Ok(*frame),
            _ => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a closure capturing across scopes".to_string(),
            }
            .into()),
        }
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
        let carrier = self.lower_type(ty)?;
        let mir::Type::Reference { pointee, .. } = self.builder.tree().get(carrier) else {
            return Err(CompilerError::Internal {
                message: "a capture frame outside a managed reference".to_string(),
            });
        };
        let pointee = *pointee;
        let frame = self.builder.new_zeroed(pointee, carrier);
        self.frames.insert(scope, frame);

        Ok(frame)
    }

    /// Return the stored field type at one index of a lifted frame carrier.
    fn frame_field_type(
        &self,
        carrier: mir::LocalNodeId<mir::Type>,
        field: u32,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        // peel the managed reference down to its struct storage
        let tree = self.builder.tree();
        let mir::Type::Reference { pointee, .. } = tree.get(carrier) else {
            return Err(CompilerError::Internal {
                message: "a capture frame outside a managed reference".to_string(),
            });
        };

        // read the captured bindings out of the frame's struct storage
        let mir::Type::Struct { fields, .. } = tree.get(*pointee) else {
            return Err(CompilerError::Internal {
                message: "a capture frame outside struct storage".to_string(),
            });
        };

        Ok(tree.get(fields[field as usize]).ty)
    }
}
