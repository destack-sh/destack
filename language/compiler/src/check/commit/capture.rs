use destack_artifact::GlobalEnvironment;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::CheckState;

use super::CheckModuleOutput;

impl CheckState<'_> {
    /// Commit closure captures discovered while walking into the DIR capture table.
    pub(super) fn commit_capture_table(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
    ) -> dir::CaptureSegment {
        let mut table = dir::CaptureSegment::new(module);
        let captures = std::mem::take(&mut self.module_mut(module).captures);

        // write one managed frame per captured function
        for capture in captures {
            let function = capture.symbol;
            let directive = capture.directive;
            let mut captured = Vec::with_capacity(capture.symbols.len());
            let mut fields = Vec::with_capacity(capture.symbols.len());
            let mut bindings = Vec::with_capacity(capture.symbols.len());

            // commit captured binding types and split managed fields
            for symbol in capture.symbols {
                let operand = self.import_symbol_type_operand(module, symbol);
                let source = self
                    .module(symbol.module_id)
                    .symbol_declaration_node(symbol.local_id);
                let Some(ty) =
                    self.commit_type_operand(module, output, environment, operand, source)
                else {
                    self.panic_unresolved_symbol_type(module, symbol, operand);
                };
                let mode = self.capture_mode_for_symbol(directive.as_ref(), symbol);

                if mode == dir::CaptureMode::Manage {
                    fields.push(dir::CaptureFrameField { symbol, ty });
                }

                captured.push((symbol, mode, ty));
            }

            let this = capture.receiver.map(|receiver| {
                let source = self
                    .module(receiver.symbol.module_id)
                    .symbol_declaration_node(receiver.symbol.local_id);
                let Some(ty) =
                    self.commit_type_operand(module, output, environment, receiver.ty, source)
                else {
                    self.panic_unresolved_symbol_type(module, receiver.symbol, receiver.ty);
                };
                let mode = directive
                    .as_ref()
                    .map(|directive| directive.default)
                    .unwrap_or(dir::CaptureMode::Manage);

                dir::CapturedReceiver {
                    symbol: receiver.symbol,
                    mode,
                    ty,
                }
            });

            if captured.is_empty() && this.is_none() && directive.is_none() {
                continue;
            }

            let frame = if fields.is_empty() {
                None
            } else {
                let scope = self.capture_frame_scope(module, fields[0].symbol);
                let managed = fields
                    .iter()
                    .map(|field| (field.symbol, field.ty))
                    .collect::<Vec<_>>();
                let ty = self.capture_frame_type(module, output, &fields);
                let frame = table.push_frame(dir::CaptureFrame { scope, ty, fields });

                // bind every managed capture to the frame
                for (symbol, ty) in managed {
                    bindings.push(dir::CapturedBinding::Manage { symbol, ty, frame });
                }

                Some(frame)
            };

            // bind non managed captures directly to the environment
            for (symbol, mode, ty) in captured {
                let binding = match mode {
                    dir::CaptureMode::Manage => {
                        continue;
                    }
                    dir::CaptureMode::Borrow => dir::CapturedBinding::Borrow { symbol, ty },
                    dir::CaptureMode::Copy => dir::CapturedBinding::Copy { symbol, ty },
                    dir::CaptureMode::Move => dir::CapturedBinding::Move { symbol, ty },
                };

                bindings.push(binding);
            }

            // keep explicit directives visible when every capture was optimized away
            if bindings.is_empty() && this.is_none() && directive.is_none() {
                continue;
            }

            let capture = dir::Capture {
                frames: frame.into_iter().collect(),
                captures: bindings,
                this,
                directive,
            };

            table.set_capture(function, capture);
        }

        table
    }

    /// Return the capture mode for one captured symbol.
    fn capture_mode_for_symbol(
        &self,
        directive: Option<&dir::CaptureDirective>,
        symbol: dir::GlobalSymbolId,
    ) -> dir::CaptureMode {
        let Some(directive) = directive else {
            return dir::CaptureMode::Manage;
        };
        let Some(name) = self.capture_symbol_name(symbol.module_id, symbol) else {
            return directive.default;
        };

        directive.mode_for_name(name)
    }

    /// Return the source name for one captured symbol.
    fn capture_symbol_name(
        &self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::StringId> {
        let bindings = self.module(module).binding_table();
        let entry = bindings.get_symbol(symbol.local_id);

        entry.name()
    }

    /// Return the lexical scope for one managed capture frame.
    fn capture_frame_scope(
        &self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> dir::GlobalScopeId {
        let bindings = self.module(module).binding_table();
        let symbol = bindings.get_symbol(symbol.local_id);

        symbol.scope.id.into_global(module)
    }

    /// Return the managed shape type for one capture frame.
    fn capture_frame_type(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        fields: &[dir::CaptureFrameField],
    ) -> dir::GlobalTypeId {
        let fields = fields
            .iter()
            .map(|field| dir::TypeField {
                key: self.capture_field_key(module, field.symbol),
                ty: field.ty,
                is_optional: false,
                is_readonly: false,
            })
            .collect();
        let shape = dir::Type::Shape(dir::ShapeType {
            fields,
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        });
        let source = self.module(module).bound.module_node;
        let shape = self.intern_type(module, output, shape, source);
        let frame = dir::Type::Form(dir::FormType {
            form: dir::Form::Managed,
            value: shape.into_global(module),
        });

        self.intern_type(module, output, frame, source)
            .into_global(module)
    }

    /// Return the structural key for one capture frame field.
    fn capture_field_key(&self, module: ModuleId, symbol: dir::GlobalSymbolId) -> dir::StaticKey {
        let bindings = self.module(module).binding_table();
        let entry = bindings.get_symbol(symbol.local_id);

        entry
            .name()
            .map(dir::StaticKey::Name)
            .unwrap_or(dir::StaticKey::Symbol(dir::SymbolKey::Unique(symbol)))
    }
}
