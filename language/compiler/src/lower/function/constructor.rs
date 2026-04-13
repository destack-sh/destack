use destack_dir as dir;
use std::collections::HashSet;

use destack_core::StringId;
use destack_mir as mir;

use crate::{LowerError, LowerResult};

use crate::lower::r#type::StructLayout;
use crate::lower::{FunctionLowerer, LocalBinding};

/// Track constructor state during function lowering.
pub(crate) struct ConstructorState {
    /// The field layout for the instance type.
    pub(crate) layout: StructLayout,
    /// The set of initialized field indices in layout order.
    pub(crate) initialized_fields: HashSet<u32>,
    /// The set of layout indices that require initialization.
    pub(crate) required_fields: HashSet<u32>,
}

impl FunctionLowerer<'_> {
    /// Initialize constructor state and bind `this` to a default instance.
    pub(crate) fn initialize_constructor(
        &mut self,
        instance_type: mir::LocalNodeId<mir::Type>,
        layout: StructLayout,
        node: dir::AnchoredGlobalNodeId,
        class_symbol: Option<dir::GlobalSymbolId>,
    ) -> LowerResult<()> {
        // build the initial instance value for this
        let instance_mir_type = self.state.builder.tree().get(instance_type).clone();
        let this_value = match instance_mir_type {
            mir::Type::Reference { kind, pointee, .. } => match kind {
                mir::ReferenceKind::Managed => {
                    let pointee = pointee
                        .ty()
                        .ok_or_else(|| LowerError::UnsupportedConstruct {
                            node,
                            message: "constructor pointee type is not concrete".to_string(),
                        })?;

                    let pointer = self.state.builder.managed_alloc(pointee, instance_type);
                    let default_value =
                        self.default_struct_value_for_layout(pointee, &layout, class_symbol, node)?;
                    self.state.builder.store(pointer, default_value);
                    pointer
                }
                // reject unsupported reference kinds
                _ => {
                    return Err(LowerError::UnsupportedConstruct {
                        node,
                        message: "unsupported constructor reference kind".to_string(),
                    });
                }
            },
            // initialize value typed instances
            _ => {
                self.default_struct_value_for_layout(instance_type, &layout, class_symbol, node)?
            }
        };

        // bind this as a local or variable
        let this_binding = if self.this_needs_addressable_local() {
            let local = self
                .state
                .builder
                .local(instance_type, mir::Mutability::Immutable);
            self.state.builder.local_set(local, this_value);
            LocalBinding::local(local, instance_type)
        } else {
            let variable = self.state.builder.variable(instance_type);
            self.state.builder.define_variable(variable, this_value);
            LocalBinding::variable(variable, instance_type)
        };
        self.state.bindings.this_binding = Some(this_binding);

        // track constructor initialization state
        self.state.constructor_state = Some(ConstructorState {
            layout,
            initialized_fields: HashSet::new(),
            required_fields: HashSet::new(),
        });

        // mark layout indices that need explicit initialization
        if let Some(state) = self.state.constructor_state.as_mut() {
            for (index, field) in state.layout.fields.iter().enumerate() {
                if field.source_index.is_some() {
                    state.required_fields.insert(index as u32);
                }
            }
        }

        Ok(())
    }

    /// Record that a constructor field has been initialized.
    pub(crate) fn mark_constructor_field_initialized(&mut self, field_index: u32) {
        // skip when not in a constructor body
        let Some(state) = self.state.constructor_state.as_mut() else {
            return;
        };

        // record the field initialization when required
        if state.required_fields.contains(&field_index) {
            state.initialized_fields.insert(field_index);
        }
    }

    /// Require a constructor field to be initialized before use.
    pub(crate) fn require_constructor_field_initialized(
        &self,
        node: dir::AnchoredGlobalNodeId,
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
        let field_name = self.context.strings.get(field_name).to_string();
        Err(LowerError::UnsupportedConstruct {
            node,
            message: format!("constructor field '{field_name}' read before initialization"),
        })
    }

    /// Require all constructor fields to be initialized before returning.
    pub(crate) fn require_constructor_complete(
        &self,
        node: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<()> {
        // skip when not in a constructor body
        let Some(state) = self.state.constructor_state.as_ref() else {
            return Ok(());
        };

        // return early when all fields are initialized
        if state
            .required_fields
            .iter()
            .all(|index| state.initialized_fields.contains(index))
        {
            return Ok(());
        }

        // pick the first missing field for an error message
        let missing_field = state
            .required_fields
            .iter()
            .find(|index| !state.initialized_fields.contains(index))
            .and_then(|index| state.layout.fields.get(*index as usize))
            .map(|field| field.name);

        // build the error message
        let message = if let Some(name) = missing_field {
            let field_name = self.context.strings.get(name).to_string();
            format!("constructor field '{field_name}' not initialized")
        } else {
            "constructor fields not initialized".to_string()
        };

        Err(LowerError::UnsupportedConstruct { node, message })
    }

    /// Return the constructed value from a constructor body.
    pub(crate) fn return_constructor_value(
        &mut self,
        node: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<()> {
        // verify all fields are initialized
        self.require_constructor_complete(node)?;

        // return the current this value
        let binding =
            self.state
                .bindings
                .this_binding
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node,
                    message: "constructor missing this binding".to_string(),
                })?;
        let value = self.binding_value(binding);
        self.state.builder.return_(Some(value));
        Ok(())
    }
}
