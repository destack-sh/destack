use std::mem::take;

use tspp_core::{FxIndexMap, FxIndexSet};
use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::sema::{Capture, CheckState, Origin};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Write captures into their DIR segment.
    pub(in crate::sema) fn write_captures(&mut self, module: ModuleId) -> CompilerResult<()> {
        let captures = take(&mut self.module_mut(module).pending_captures);
        if captures.is_empty() {
            return Ok(());
        }

        // collect managed symbols per lifted lexical scope
        let mut managed_symbols =
            FxIndexMap::<dir::LocalScopeId, FxIndexSet<dir::GlobalSymbolId>>::default();
        for capture in &captures {
            let directive = capture
                .annotation
                .as_ref()
                .map(|annotation| &annotation.directive);
            for symbol in &capture.symbols {
                if self.capture_mode(module, directive, *symbol)? != dir::CaptureMode::Manage {
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

        // write one shared frame for each managed scope
        let mut frames = FxIndexMap::default();
        for (scope, symbols) in managed_symbols {
            let frame = self.write_capture_frame(module, scope, &symbols)?;
            frames.insert(scope, frame);
        }

        // write captures for each function
        for mut capture in captures {
            let directive = capture
                .annotation
                .take()
                .map(|annotation| annotation.directive);
            let (function, capture) =
                self.write_capture(module, capture, directive.clone(), &frames)?;

            let captures = &mut self.module_mut(module).captures;
            captures.set_capture(function, capture);
            if let Some(directive) = directive {
                captures.set_capture_directive(function, directive);
            }
        }

        Ok(())
    }

    /// Write one managed capture frame for one lexical scope.
    fn write_capture_frame(
        &mut self,
        module: ModuleId,
        scope: dir::LocalScopeId,
        symbols: &FxIndexSet<dir::GlobalSymbolId>,
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
        let mut object_properties = Vec::new();

        // write one frame field per captured binding
        for (symbol, key) in frame_bindings {
            let Some(key) = key else {
                return Err(CompilerError::Internal {
                    message: format!("managed capture symbol {symbol:?} has no field key"),
                });
            };
            let ty = self.capture_symbol_type(symbol)?;

            fields.push(dir::CaptureFrameField { symbol, ty });
            object_properties.push(dir::TypeProperty {
                key,
                access: dir::PropertyAccess::ReadWrite {
                    read: ty,
                    write: ty,
                },
                is_optional: false,
            });
        }

        // build the frame object type, a handle by its default ownership
        let ty = self.intern_object(&object_properties)?;

        // store the frame in the capture segment
        let frame = dir::CaptureFrame {
            scope: scope.into_global(module),
            ty,
            fields,
        };

        Ok(self.module_mut(module).captures.push_frame(frame))
    }

    /// Write one function capture.
    fn write_capture(
        &mut self,
        module: ModuleId,
        capture: Capture,
        directive: Option<dir::CaptureDirective>,
        frames: &FxIndexMap<dir::LocalScopeId, dir::LocalCaptureFrameId>,
    ) -> CompilerResult<(dir::GlobalSymbolId, dir::Capture)> {
        let function = capture.symbol;
        let mut used_frames = FxIndexSet::default();
        let mut captured = Vec::new();

        // write captured lexical bindings
        for symbol in &capture.symbols {
            let mode = self.capture_mode(module, directive.as_ref(), *symbol)?;
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

        // write the captured receiver when present
        let this = match capture.receiver {
            Some(receiver) => {
                let mode = self.capture_mode(module, directive.as_ref(), receiver.symbol)?;
                let ty = self.shallow_resolve(receiver.receiver.ty)?;

                Some(dir::CapturedReceiver {
                    symbol: receiver.symbol,
                    mode,
                    ty,
                })
            }
            None => None,
        };

        // assemble the function's capture record
        let ownership = self.closure_environment_ownership(module, function)?;
        let capture = dir::Capture {
            frames: used_frames.into_iter().collect(),
            captures: captured,
            this,
            ownership,
        };

        Ok((function, capture))
    }

    /// Return the ownership the closure expression's callable form gives its environment.
    fn closure_environment_ownership(
        &mut self,
        module: ModuleId,
        function: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::Ownership> {
        // read the closure expression above the declaration and the form it converts into
        let declaration = self
            .module(module)
            .symbol_declaration_node(function.local_id)?;
        let Some(closure) = self.module(module).view().get_parent_any(declaration) else {
            return Ok(dir::Ownership::Managed);
        };

        // keep a declared nested function managed
        if closure.ty != dir::NodeType::Expression {
            return Ok(dir::Ownership::Managed);
        }

        // read the callable form the closure expression takes
        let node = closure.into_global(module);
        let target = match self.module(module).coercions_tail.coercion(node).cloned() {
            Some(coercion) => coercion.target(),
            None => self.node_type(node)?,
        };
        let origin = Origin::Node(node, None);
        let form = self.form_chain(origin, target)?.ownership_form();

        Ok(match form.map(|form| form.form) {
            Some(dir::Form::Owned) => dir::Ownership::Owned,
            _ => dir::Ownership::Managed,
        })
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

        let binding = self.module(module).bindings.get_symbol(symbol.local_id);
        let Some(name) = binding.name() else {
            return Ok(directive.default);
        };

        Ok(directive.mode_for_name(name))
    }

    /// Return one captured symbol's resolved type.
    fn capture_symbol_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some(ty) = self.symbol_type_maybe(symbol)? else {
            let module = self.module(symbol.module_id);
            let binding = module.bindings.get_symbol(symbol.local_id);
            let name = binding
                .name()
                .map(|name| self.strings().get(name).to_string())
                .unwrap_or_else(|| "<anonymous>".to_string());

            return Err(CompilerError::Internal {
                message: format!(
                    "captured symbol {symbol:?} named {name} has no committed type: kind={:?} role={:?} scope={:?}",
                    binding.kind, binding.role, binding.scope
                ),
            });
        };
        let ty = self.shallow_resolve(ty)?;

        Ok(ty)
    }
}
