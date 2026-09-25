use tspp_dir as dir;

use crate::sema::CheckState;
use crate::sema::derive::{Component, ComponentProjection, Composite, Derivation};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Build `Self { field: this.field.clone(), .. }`, or the defaults, at the receiver.
    pub(super) fn build_construct_body(
        &mut self,
        frame: &mut Derivation,
        shape: Composite,
        components: &[Component],
        interface: dir::AutoInterface,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        // produce every component's value
        let mut values = Vec::with_capacity(components.len());
        for component in components {
            let value = match interface {
                dir::AutoInterface::Default => {
                    let receiver = self.build_type_value(frame, component.ty)?;

                    self.build_component_call(frame, receiver, component, Vec::new())?
                }
                _ => {
                    let this = self.build_this(frame)?;
                    let read = self.build_component_read(frame, this, frame.receiver, component)?;

                    // clone a bitwise copy by reading the receiver
                    match component.call {
                        Some(_) => self.build_component_call(frame, read, component, Vec::new())?,
                        None => read,
                    }
                }
            };
            values.push(value);
        }

        // assemble the receiver from those values, a class owning the object its handle names
        let result = match shape {
            Composite::Class(_) => {
                let object = frame.receiver;

                self.intern_type(dir::Type::Form(dir::FormType {
                    form: dir::Form::Owned,
                    value: object,
                }))?
            }
            _ => frame.receiver,
        };
        let value = self.build_construction(frame, shape, components, values, result)?;

        self.build_block(frame, Vec::new(), Some(value), result)
    }

    /// Build the receiver's value from its component values.
    fn build_construction(
        &mut self,
        frame: &mut Derivation,
        shape: Composite,
        components: &[Component],
        values: Vec<dir::LocalNodeId<dir::Expression>>,
        result: dir::GlobalTypeId,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        match shape {
            // write one property per named field, a class literal owning its object inline
            Composite::Struct(_) | Composite::Class(_) => {
                let mut properties = Vec::with_capacity(values.len());
                for (component, value) in components.iter().zip(values) {
                    let ComponentProjection::Field {
                        key: dir::StaticKey::Name(name),
                        ..
                    } = component.read
                    else {
                        return Err(CompilerError::Internal {
                            message: "a struct component without a field name".to_owned(),
                        });
                    };
                    properties.push(self.build_node(
                        frame.module,
                        frame.span,
                        dir::Property::Field {
                            name: dir::Name::Identifier(name),
                            value,
                            is_shorthand: false,
                        },
                    ));
                }
                let ty = self.build_node(frame.module, frame.span, dir::TypeExpression::Intrinsic);

                self.build_expression(
                    frame,
                    dir::Expression::StructExpression { ty, properties },
                    result,
                )
            }
            // write one element per position
            Composite::Tuple => {
                let mut elements = Vec::with_capacity(values.len());
                for value in values {
                    elements.push(self.build_node(
                        frame.module,
                        frame.span,
                        dir::Argument::Positional { value },
                    ));
                }

                self.build_expression(
                    frame,
                    dir::Expression::TupleExpression { elements },
                    frame.receiver,
                )
            }
            // call the newtype over its single backing value
            Composite::Newtype(symbol) => {
                let [value] = values.as_slice() else {
                    return Err(CompilerError::Internal {
                        message: "a newtype derivation with several components".to_owned(),
                    });
                };
                let [component] = components else {
                    return Err(CompilerError::Internal {
                        message: "a newtype derivation with several components".to_owned(),
                    });
                };

                // call the newtype's own constructor over the backing value
                let argument = self.build_node(
                    frame.module,
                    frame.span,
                    dir::Argument::Positional { value: *value },
                );
                let name = self.symbol_name(symbol)?;
                let name = self.strings().intern(&name);
                let ty = self.symbol_type(symbol)?;
                let callee = self.build_name(frame, symbol, name, ty)?;
                let node = self.build_expression(
                    frame,
                    dir::Expression::Call {
                        position: dir::PostfixPosition::Direct,
                        left: callee,
                        generic_arguments: Vec::new(),
                        arguments: vec![argument],
                        is_optional: false,
                    },
                    frame.receiver,
                )?;

                // record the construction at the receiver's own instance
                let dir::Type::Application(application) = self.ty(frame.receiver)? else {
                    return Err(CompilerError::Internal {
                        message: "a newtype derivation outside its application".to_owned(),
                    });
                };
                let arguments = self.type_ids(frame.receiver.module_id, application.arguments)?;
                let arguments = self.symbol_generic_argument_bindings(symbol, arguments)?;
                let key = dir::InstanceKey::new(symbol, arguments);
                self.commit_decision(
                    node.into_global_any(frame.module),
                    dir::Decision::Construct(dir::ConstructDecision::new(
                        dir::ConstructTarget::Newtype {
                            key,
                            backing: component.ty,
                            arm: None,
                        },
                        vec![dir::ArgumentBinding {
                            coercion: None,
                            parameter_type: component.ty,
                            argument_type: component.ty,
                            source: dir::ArgumentSource::Provided(
                                argument.into_global_any(frame.module),
                            ),
                        }],
                        frame.receiver,
                        Vec::new(),
                    )),
                )?;

                Ok(node)
            }
            // a union assembles elsewhere
            Composite::Union => Err(CompilerError::Internal {
                message: "a union constructed from components".to_owned(),
            }),
        }
    }
}
