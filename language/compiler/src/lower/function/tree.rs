use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::{CompilerError, CompilerResult};

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one tree literal through its resolution.
    pub(in crate::lower) fn lower_tree(
        &mut self,
        resolution: &dir::TreeDecision,
    ) -> CompilerResult<mir::Value> {
        match &resolution.target {
            // lower an element to its tag, attributes, and children
            dir::TreeTarget::Element { call, .. } => {
                let call = self.tree_call(call)?;
                let [tag, attributes, children] = call.arguments.as_slice() else {
                    return Err(CompilerError::Internal {
                        message: "an unexpected argument count on a tree element call".to_string(),
                    });
                };
                let tag = self.lower_tree_tag(tag)?;
                let attributes =
                    self.lower_tree_attributes(attributes.argument_type, resolution)?;
                let children =
                    self.lower_tree_children(children.argument_type, &resolution.children)?;

                self.lower_tree_static_call(&call, vec![tag, attributes, children])
            }
            // lower a fragment to its children alone
            dir::TreeTarget::Fragment { call } => {
                let call = self.tree_call(call)?;
                let [children] = call.arguments.as_slice() else {
                    return Err(CompilerError::Internal {
                        message: "an unexpected argument count on a tree fragment call".to_string(),
                    });
                };
                let children =
                    self.lower_tree_children(children.argument_type, &resolution.children)?;

                self.lower_tree_static_call(&call, vec![children])
            }
            // lower a component through its selected invocation
            dir::TreeTarget::Component { invocation, .. } => match invocation {
                // call a function component with its props
                dir::TreeInvocation::Call(call) => {
                    let call = self.tree_call(call)?;
                    let [props] = call.arguments.as_slice() else {
                        return Err(CompilerError::Internal {
                            message: "an unexpected argument count on a tree component call"
                                .to_string(),
                        });
                    };
                    let props = self.lower_tree_attributes(props.argument_type, resolution)?;

                    self.lower_tree_static_call(&call, vec![props])
                }
                // construct a class component with its props
                dir::TreeInvocation::Construct(construct) => {
                    self.lower_tree_construct(construct, resolution)
                }
                // build a struct component directly from its props
                dir::TreeInvocation::Struct { ty } => self.lower_tree_aggregate(*ty, resolution),
            },
        }
    }

    /// Unwrap the single-receiver call behind one tree selection.
    fn tree_call(&self, call: &dir::CallDecision) -> CompilerResult<dir::Call> {
        let dir::OperationResolution::One(call) = call else {
            return Err(CompilerError::Internal {
                message: "a tree call over a union receiver".to_string(),
            });
        };

        Ok(call.clone())
    }

    /// Lower one resolved tree call to its selected function instance.
    fn lower_tree_static_call(
        &mut self,
        call: &dir::Call,
        values: Vec<mir::Value>,
    ) -> CompilerResult<mir::Value> {
        let dir::CallableTarget::Symbol { function, .. } = &call.target else {
            return Err(self.unsupported("an indirect tree component call"));
        };
        // call the selected function instance
        let id = self.resolve_callee(&function.key)?;
        let value = self.call(&id, values);

        value.ok_or_else(|| CompilerError::Internal {
            message: "a void tree call used as a value".to_string(),
        })
    }

    /// Lower one tree tag to its literal constant.
    fn lower_tree_tag(&mut self, tag: &dir::ArgumentBinding) -> CompilerResult<mir::Value> {
        // read the literal the tag parameter binds
        let argument_type = tag.argument_type;
        let dir::Type::Literal(literal) = self.lower.ty(argument_type)? else {
            return Err(CompilerError::Internal {
                message: "a non-literal tree tag parameter".to_string(),
            });
        };
        let representation = self.lower_type(argument_type)?;

        self.lower_constant(literal, representation)
    }

    /// Lower the attribute type of one tree literal.
    fn lower_tree_attributes(
        &mut self,
        attributes: dir::GlobalTypeId,
        resolution: &dir::TreeDecision,
    ) -> CompilerResult<mir::Value> {
        let dir::Type::Application(_) = self.lower.ty(attributes)? else {
            return Err(self.unsupported("a structural tree attribute type"));
        };

        self.lower_tree_aggregate(attributes, resolution)
    }

    /// Lower one nominal attribute type to its field aggregate.
    fn lower_tree_aggregate(
        &mut self,
        attributes: dir::GlobalTypeId,
        resolution: &dir::TreeDecision,
    ) -> CompilerResult<mir::Value> {
        // read the nominal's fields in declaration order
        let dir::Type::Application(_) = self.lower.ty(attributes)? else {
            return Err(CompilerError::Internal {
                message: "a tree aggregate outside a nominal type".to_string(),
            });
        };
        let representation = self.lower_nominal(attributes)?;
        let ty = self.lower_type(attributes)?;
        let nominal = self.lower.nominal(representation.key.symbol)?;
        let members = nominal
            .fields
            .iter()
            .map(|field| (field.key, field.symbol))
            .collect::<Vec<_>>();
        let fields = members.iter().map(|(key, _)| *key).collect::<Vec<_>>();

        // gather each attribute value under its field key
        let mut values = Vec::with_capacity(resolution.attributes.len());
        for attribute in &resolution.attributes {
            let key = dir::StaticKey::Name(attribute.key);
            let value = self.lower_tree_attribute(attribute)?;
            values.push((key, value));
        }

        // fill the children field with the children tuple
        let children = self.lower.strings.intern("children");
        let children_key = dir::StaticKey::Name(children);
        if !resolution.children.is_empty()
            && let Some((_, symbol)) = members.iter().find(|(key, _)| *key == children_key)
        {
            let ty = self.lower.symbol_type(*symbol)?;
            let value = self.lower_tree_children(ty, &resolution.children)?;
            values.push((children_key, value));
        }

        // build the aggregate in field declaration order
        let mut ordered = Vec::with_capacity(fields.len());
        for field in fields {
            let Some((_, value)) = values.iter().find(|(key, _)| *key == field) else {
                return Err(self.unsupported("a defaulted tree attribute"));
            };
            ordered.push(*value);
        }

        Ok(self.builder.aggregate(ty, ordered))
    }

    /// Lower one attribute binding to its value.
    fn lower_tree_attribute(
        &mut self,
        attribute: &dir::TreeAttributeBinding,
    ) -> CompilerResult<mir::Value> {
        // lower an attribute with an expression value
        if let Some(value) = attribute.value {
            let Ok(expression) = value.local_id.try_into_typed::<dir::Expression>() else {
                return Err(CompilerError::Internal {
                    message: "a non-expression tree attribute value".to_string(),
                });
            };

            return self.lower_value(expression);
        }

        // take the text of a written attribute
        let literal = match attribute.text {
            Some(text) => dir::Literal::String(text),
            // provide true for bare attributes
            None => dir::Literal::Boolean(true),
        };
        let representation = self.lower_type(attribute.ty)?;

        self.lower_constant(literal, representation)
    }

    /// Lower the children of one tree literal to their tuple.
    fn lower_tree_children(
        &mut self,
        tuple: dir::GlobalTypeId,
        children: &[dir::TreeChildBinding],
    ) -> CompilerResult<mir::Value> {
        // lower each child into the children tuple
        let ty = self.lower_type(tuple)?;
        let mut values = Vec::with_capacity(children.len());
        for child in children {
            match child {
                // materialize a text child as a string constant
                dir::TreeChildBinding::Text { value, ty } => {
                    let representation = self.lower_type(*ty)?;
                    values.push(self.lower_constant(dir::Literal::String(*value), representation)?);
                }
                // lower an expression child
                dir::TreeChildBinding::Expression { node, .. } => {
                    let Ok(expression) = node.local_id.try_into_typed::<dir::Expression>() else {
                        return Err(CompilerError::Internal {
                            message: "a non-expression tree child value".to_string(),
                        });
                    };
                    values.push(self.lower_value(expression)?);
                }
                // reject a spread child
                dir::TreeChildBinding::Spread { .. } => {
                    return Err(self.unsupported("a spread tree child"));
                }
            }
        }

        Ok(self.builder.aggregate(ty, values))
    }

    /// Lower one class component construction with its attribute object.
    fn lower_tree_construct(
        &mut self,
        construct: &dir::ConstructDecision,
        resolution: &dir::TreeDecision,
    ) -> CompilerResult<mir::Value> {
        // require a class target with a single props argument
        let dir::ConstructTarget::Class {
            constructor,
            arguments,
            ..
        } = &construct.target
        else {
            return Err(CompilerError::Internal {
                message: "a non-class tree component target".to_string(),
            });
        };
        let [props] = construct.arguments.as_slice() else {
            return Err(CompilerError::Internal {
                message: "an unexpected argument count on a tree component construction"
                    .to_string(),
            });
        };
        let props = self.lower_tree_attributes(props.argument_type, resolution)?;

        self.lower_class_instance(
            construct.return_type,
            constructor,
            arguments,
            &construct.regions,
            vec![props],
        )
    }
}
