use destack_dir as dir;

use crate::check::{CheckState, Decorator};

impl CheckState<'_> {
    /// Return the capture directive attached to one function symbol.
    pub(in crate::check) fn capture_directive_for_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::CaptureDirective> {
        let module = symbol.module_id;
        let source = self.symbol_source_node(symbol);
        let decorators = self.decorators_for_owner(module, source);
        let mut directive = None;

        // use the last capture decorator in source order
        for decorator in decorators {
            let Decorator::Capture(call) = decorator else {
                continue;
            };

            directive = self.capture_directive_from_arguments(module, &call.arguments);
        }

        directive
    }

    /// Return the capture directive represented by decorator arguments.
    fn capture_directive_from_arguments(
        &self,
        module: destack_source::ModuleId,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> Option<dir::CaptureDirective> {
        let [argument] = arguments else {
            return None;
        };
        let value = self.module(module).view().get(*argument).value()?;

        match self.module(module).view().get(value) {
            dir::Expression::ScalarLiteral(dir::ScalarLiteral::String(name)) => {
                let default = self.capture_mode_from_string(module, *name)?;

                Some(dir::CaptureDirective {
                    default,
                    rules: Vec::new(),
                })
            }
            dir::Expression::ObjectExpression { properties } => {
                self.capture_directive_from_properties(module, &properties)
            }
            _ => None,
        }
    }

    /// Return the capture directive represented by an object literal.
    fn capture_directive_from_properties(
        &self,
        module: destack_source::ModuleId,
        properties: &[dir::LocalNodeId<dir::Property>],
    ) -> Option<dir::CaptureDirective> {
        let mut default = None;
        let mut rules = Vec::new();

        // read default and binding mode fields
        for property in properties {
            let dir::Property::Field { key, value, .. } = self.module(module).view().get(*property)
            else {
                return None;
            };
            let view = self.module(module).view();
            let key = key.static_key(view.tree())?;
            let dir::StaticKey::Name(name) = key else {
                return None;
            };
            let mode = self.capture_mode_from_expression(module, *value)?;

            if self.module(module).strings.get(name) == "default" {
                default = Some(mode);
            } else {
                rules.push(dir::CaptureRule { name, mode });
            }
        }

        Some(dir::CaptureDirective {
            default: default.unwrap_or(dir::CaptureMode::Manage),
            rules,
        })
    }

    /// Return the capture mode represented by one expression.
    fn capture_mode_from_expression(
        &self,
        module: destack_source::ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::CaptureMode> {
        let dir::Expression::ScalarLiteral(dir::ScalarLiteral::String(name)) =
            self.module(module).view().get(expression)
        else {
            return None;
        };

        self.capture_mode_from_string(module, *name)
    }

    /// Return the capture mode represented by one string.
    fn capture_mode_from_string(
        &self,
        module: destack_source::ModuleId,
        name: dir::StringId,
    ) -> Option<dir::CaptureMode> {
        let mode = match self.module(module).strings.get(name) {
            "manage" => dir::CaptureMode::Manage,
            "borrow" => dir::CaptureMode::Borrow,
            "copy" => dir::CaptureMode::Copy,
            "move" => dir::CaptureMode::Move,
            _ => return None,
        };

        Some(mode)
    }
}
