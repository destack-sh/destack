use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::{IndexMap, IndexSet};

use crate::check::CheckState;
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Write checked captures into their DIR segment.
    pub(in crate::check) fn write_captures(&mut self, module: ModuleId) -> CompilerResult<()> {
        let captures = std::mem::take(&mut self.module_mut(module).captures);
        if captures.is_empty() {
            return Ok(());
        }

        // collect managed fields per lifted lexical scope
        let mut managed_symbols =
            IndexMap::<dir::LocalScopeId, IndexSet<dir::GlobalSymbolId>>::new();
        for capture in &captures {
            for symbol in &capture.symbols {
                if self.capture_mode(module, capture.directive.as_ref(), *symbol)?
                    != dir::CaptureMode::Manage
                {
                    continue;
                }

                let scope = self
                    .module(module)
                    .bindings
                    .get_symbol(symbol.local_id)
                    .scope
                    .id;
                managed_symbols.entry(scope).or_default().insert(*symbol);
            }
        }

        // materialize one shared frame for each managed scope
        let mut frames = IndexMap::new();
        for (scope, symbols) in managed_symbols {
            let frame = self.write_capture_frame(module, scope, &symbols)?;

            frames.insert(scope, frame);
        }

        // materialize captures for each function
        for capture in captures {
            let capture = self.write_capture(module, capture, &frames)?;

            self.module_mut(module)
                .capture_segment
                .set_capture(capture.0, capture.1);
        }

        Ok(())
    }

    /// Write one managed capture frame for one lexical scope.
    fn write_capture_frame(
        &mut self,
        module: ModuleId,
        scope: dir::LocalScopeId,
        symbols: &IndexSet<dir::GlobalSymbolId>,
    ) -> CompilerResult<dir::LocalCaptureFrameId> {
        let scope_bindings = &self.module(module).bindings.get_scope_by_id(scope).bindings;
        let mut frame_bindings = Vec::new();

        // preserve binding order from the lifted lexical scope
        for binding in scope_bindings {
            let symbol = binding.symbol.into_global(module);
            if symbols.contains(&symbol) {
                frame_bindings.push((symbol, binding.key));
            }
        }

        // require the managed set to be present in the scope
        if frame_bindings.len() != symbols.len() {
            return Err(CompilerError::Internal {
                message: format!(
                    "managed capture scope {scope:?} missed {} of {} symbols",
                    symbols.len() - frame_bindings.len(),
                    symbols.len()
                ),
            });
        }

        let mut fields = Vec::new();
        let mut shape_fields = Vec::new();

        // write one frame field per captured binding
        for (symbol, key) in frame_bindings {
            let Some(key) = key else {
                return Err(CompilerError::Internal {
                    message: format!("managed capture symbol {symbol:?} has no field key"),
                });
            };
            let ty = self.capture_symbol_type(symbol)?;

            fields.push(dir::CaptureFrameField { symbol, ty });
            shape_fields.push(dir::TypeField {
                key,
                ty,
                is_optional: false,
                is_readonly: false,
            });
        }

        // build the managed frame object type
        let shape_fields = self.intern_fields(module, &shape_fields)?;
        let shape = dir::ShapeType {
            fields: shape_fields,
            call_signatures: dir::TypeListId::EMPTY,
            construct_signatures: dir::TypeListId::EMPTY,
            index_signatures: dir::TypeListId::EMPTY,
        };
        let shape = self.intern_type(module, dir::Type::Shape(shape))?;
        let ty = self.intern_type(
            module,
            dir::Type::Form(dir::FormType {
                form: dir::Form::Managed,
                value: shape,
            }),
        )?;

        // store the frame in the checked capture segment
        let frame = dir::CaptureFrame {
            scope: scope.into_global(module),
            ty,
            fields,
        };

        Ok(self.module_mut(module).capture_segment.push_frame(frame))
    }

    /// Write one function capture.
    fn write_capture(
        &mut self,
        module: ModuleId,
        capture: crate::check::Capture,
        frames: &IndexMap<dir::LocalScopeId, dir::LocalCaptureFrameId>,
    ) -> CompilerResult<(dir::GlobalSymbolId, dir::Capture)> {
        let function = capture.symbol;
        let mut used_frames = IndexSet::new();
        let mut captured = Vec::new();

        // materialize captured lexical bindings
        for symbol in &capture.symbols {
            let mode = self.capture_mode(module, capture.directive.as_ref(), *symbol)?;
            let ty = self.capture_symbol_type(*symbol)?;
            let binding = match mode {
                dir::CaptureMode::Manage => {
                    let scope = self
                        .module(module)
                        .bindings
                        .get_symbol(symbol.local_id)
                        .scope
                        .id;
                    let Some(frame) = frames.get(&scope).copied() else {
                        return Err(CompilerError::Internal {
                            message: format!(
                                "managed capture symbol {symbol:?} has no capture frame"
                            ),
                        });
                    };

                    used_frames.insert(frame);
                    dir::CapturedBinding::Manage {
                        symbol: *symbol,
                        frame,
                        ty,
                    }
                }
                dir::CaptureMode::Borrow => dir::CapturedBinding::Borrow {
                    symbol: *symbol,
                    ty,
                },
                dir::CaptureMode::Copy => dir::CapturedBinding::Copy {
                    symbol: *symbol,
                    ty,
                },
                dir::CaptureMode::Move => dir::CapturedBinding::Move {
                    symbol: *symbol,
                    ty,
                },
            };

            captured.push(binding);
        }

        // materialize captured receiver when present
        let this = match capture.receiver {
            Some(receiver) => {
                let mode =
                    self.capture_mode(module, capture.directive.as_ref(), receiver.symbol)?;
                let ty = self.settled_root(receiver.receiver.ty)?;

                Some(dir::CapturedReceiver {
                    symbol: receiver.symbol,
                    mode,
                    ty,
                })
            }
            None => None,
        };

        let capture = dir::Capture {
            frames: used_frames.into_iter().collect(),
            captures: captured,
            this,
            directive: capture.directive,
        };

        Ok((function, capture))
    }

    /// Return one capture mode from an optional directive.
    fn capture_mode(
        &self,
        module: ModuleId,
        directive: Option<&dir::CaptureDirective>,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::CaptureMode> {
        let Some(directive) = directive else {
            return Ok(dir::CaptureMode::Manage);
        };

        let symbol_entry = self.module(module).bindings.get_symbol(symbol.local_id);
        let Some(name) = symbol_entry.name() else {
            return Ok(directive.default);
        };

        Ok(directive.mode_for_name(name))
    }

    /// Return one captured symbol's settled checked type.
    fn capture_symbol_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some(ty) = self.symbol_type_maybe(symbol) else {
            let module = self.module(symbol.module_id);
            let symbol_entry = module.bindings.get_symbol(symbol.local_id);
            let name = symbol_entry
                .name()
                .map(|name| module.strings.get(name).to_string())
                .unwrap_or_else(|| "<anonymous>".to_string());

            return Err(CompilerError::Internal {
                message: format!(
                    "captured symbol {symbol:?} named {name} has no checked type: kind={:?} role={:?} scope={:?}",
                    symbol_entry.kind, symbol_entry.role, symbol_entry.scope
                ),
            });
        };
        let ty = self.settled_root(ty)?;

        Ok(ty)
    }
}
