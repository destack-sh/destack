use std::collections::HashSet;

use destack_base::StringId;
use destack_dir::AnchoredGlobalNodeId;
use destack_mir as mir;

use crate::{LowerError, LowerResult};

use crate::lower::emit::{FunctionContext, LocalBinding};
use crate::lower::r#type::StructLayout;

/// Track constructor state during function lowering.
pub(crate) struct ConstructorState {
    /// The field layout for the instance type.
    pub(crate) layout: StructLayout,
    /// The set of initialized field indices in layout order.
    pub(crate) initialized_fields: HashSet<u32>,
}

impl FunctionContext<'_> {
    /// Initialize constructor state and bind `this` to a default instance.
    pub(crate) fn initialize_constructor(
        &mut self,
        instance_type: mir::LocalNodeId<mir::Type>,
        layout: StructLayout,
        node: AnchoredGlobalNodeId,
    ) -> LowerResult<()> {
        // build the initial instance value for this
        let instance_mir_type = self.state.builder.tree().get(instance_type).clone();
        let this_value = match instance_mir_type {
            mir::Type::Reference { kind, pointee, .. } => match kind {
                mir::ReferenceKind::Managed => {
                    let pointer = self.state.builder.managed_alloc(pointee);
                    let zero = self.zero_value_for_type(pointee, node)?;
                    self.state.builder.store(pointer, zero);
                    pointer
                }
                _ => {
                    return Err(LowerError::UnsupportedConstruct {
                        node,
                        message: "unsupported constructor reference kind".to_string(),
                    });
                }
            },
            _ => self.zero_value_for_type(instance_type, node)?,
        };

        // bind this as a local variable
        let this_variable = self.state.builder.create_variable(instance_type);
        self.state
            .builder
            .define_variable(this_variable, this_value);
        self.state.bindings.this_binding = Some(LocalBinding {
            variable: this_variable,
            ty: instance_type,
        });

        // track constructor initialization state
        self.state.constructor_state = Some(ConstructorState {
            layout,
            initialized_fields: HashSet::new(),
        });

        Ok(())
    }

    /// Record that a constructor field has been initialized.
    pub(crate) fn mark_constructor_field_initialized(&mut self, field_index: u32) {
        // skip when not in a constructor body
        let Some(state) = self.state.constructor_state.as_mut() else {
            return;
        };

        state.initialized_fields.insert(field_index);
    }

    /// Ensure a constructor field was initialized before use.
    pub(crate) fn ensure_constructor_field_initialized(
        &self,
        node: AnchoredGlobalNodeId,
        field_index: u32,
        field_name: StringId,
    ) -> LowerResult<()> {
        // skip when not in a constructor body
        let Some(state) = self.state.constructor_state.as_ref() else {
            return Ok(());
        };

        // allow reads after initialization
        if state.initialized_fields.contains(&field_index) {
            return Ok(());
        }

        // report reads before initialization
        let field_name = self.env.strings.get(field_name).to_string();
        Err(LowerError::UnsupportedConstruct {
            node,
            message: format!("constructor field '{field_name}' read before initialization"),
        })
    }

    /// Ensure all constructor fields were initialized before returning.
    pub(crate) fn ensure_constructor_complete(
        &self,
        node: AnchoredGlobalNodeId,
    ) -> LowerResult<()> {
        // skip when not in a constructor body
        let Some(state) = self.state.constructor_state.as_ref() else {
            return Ok(());
        };

        // return early when all fields are initialized
        if state.initialized_fields.len() == state.layout.fields.len() {
            return Ok(());
        }

        // pick the first missing field for an error message
        let missing_field = state
            .layout
            .fields
            .iter()
            .enumerate()
            .find(|(index, _)| !state.initialized_fields.contains(&(*index as u32)))
            .map(|(_, field)| field.name);

        // build the error message
        let message = if let Some(name) = missing_field {
            let field_name = self.env.strings.get(name).to_string();
            format!("constructor field '{field_name}' not initialized")
        } else {
            "constructor fields not initialized".to_string()
        };

        Err(LowerError::UnsupportedConstruct { node, message })
    }

    /// Return the constructed value from a constructor body.
    pub(crate) fn return_constructor_value(
        &mut self,
        node: AnchoredGlobalNodeId,
    ) -> LowerResult<()> {
        // verify all fields are initialized
        self.ensure_constructor_complete(node)?;

        // return the current this value
        let binding =
            self.state
                .bindings
                .this_binding
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node,
                    message: "constructor missing this binding".to_string(),
                })?;
        let value = self.state.builder.use_variable(binding.variable);
        self.state.builder.return_(Some(value));
        Ok(())
    }
}
