use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{FunctionLowerer, GenericInstanceKey};
use crate::{CompilerError, CompilerResult, LowerError};

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one tree literal through its resolution.
    pub(in crate::lower) fn lower_tree(
        &mut self,
        resolution: &dir::TreeResolution,
    ) -> CompilerResult<mir::Value> {
        match &resolution.target {
            dir::TreeTarget::Element { call, .. } => {
                let call = self.tree_call(call)?;
                let [tag, attributes, children] = call.arguments.as_slice() else {
                    return Err(CompilerError::Internal {
                        message: "tree element call bound an unexpected argument count".to_string(),
                    });
                };
                let tag = self.lower_tree_tag(tag)?;
                let attributes = self.lower_tree_attributes(attributes.ty, resolution)?;
                let children = self.lower_tree_children(children.ty, &resolution.children)?;

                self.lower_tree_static_call(&call, vec![tag, attributes, children])
            }
            dir::TreeTarget::Fragment { call } => {
                let call = self.tree_call(call)?;
                let [children] = call.arguments.as_slice() else {
                    return Err(CompilerError::Internal {
                        message: "tree fragment call bound an unexpected argument count"
                            .to_string(),
                    });
                };
                let children = self.lower_tree_children(children.ty, &resolution.children)?;

                self.lower_tree_static_call(&call, vec![children])
            }
            dir::TreeTarget::Component { invocation, .. } => match invocation {
                dir::TreeInvocation::Call(call) => {
                    let call = self.tree_call(call)?;
                    let [props] = call.arguments.as_slice() else {
                        return Err(CompilerError::Internal {
                            message: "tree component call bound an unexpected argument count"
                                .to_string(),
                        });
                    };
                    let props = self.lower_tree_attributes(props.ty, resolution)?;

                    self.lower_tree_static_call(&call, vec![props])
                }
                dir::TreeInvocation::Construct(construct) => {
                    self.lower_tree_construct(construct, resolution)
                }
                dir::TreeInvocation::Struct { ty } => self.lower_tree_aggregate(*ty, resolution),
            },
        }
    }

    /// Unwrap the single-receiver call behind one tree selection.
    fn tree_call(&self, call: &dir::CallResolution) -> CompilerResult<dir::Call> {
        let dir::OperationResolution::One(call) = call else {
            return Err(CompilerError::Internal {
                message: "tree call resolved over a union receiver".to_string(),
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
        let dir::CallTarget::Symbol { function, .. } = &call.target else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "an indirect tree component call".to_string(),
            }
            .into());
        };
        let key = match function.generic_arguments.is_empty() {
            true => GenericInstanceKey::non_generic(function.symbol),
            false => {
                let bindings = self
                    .lowerer
                    .instance_bindings(&function.generic_arguments, &self.type_substitution)?;
                let arguments: Vec<_> = bindings.iter().map(|binding| binding.argument).collect();

                self.generic_instance_key(function.symbol, &arguments)?
            }
        };
        let id = self.function(&key)?;
        let value = self.builder.call_function(id, values);

        value.ok_or_else(|| CompilerError::Internal {
            message: "a void tree call used as a value".to_string(),
        })
    }

    /// Lower one tree tag to its literal constant.
    fn lower_tree_tag(&mut self, tag: &dir::ArgumentBinding) -> CompilerResult<mir::Value> {
        let reduced = self.lowerer.reduced_type(tag.ty)?;
        let dir::Type::Literal(literal) = self.lowerer.ty(reduced)? else {
            return Err(CompilerError::Internal {
                message: "tree tag bound a non-literal parameter".to_string(),
            });
        };
        let carrier = self.lower_type(reduced)?;
        let carrier = self.builder.tree().get(carrier).clone();

        self.lower_constant(literal, carrier)
    }

    /// Lower the attributes row of one tree literal.
    fn lower_tree_attributes(
        &mut self,
        row: dir::GlobalTypeId,
        resolution: &dir::TreeResolution,
    ) -> CompilerResult<mir::Value> {
        let row = self.lowerer.reduced_type(row)?;
        let dir::Type::Application(_) = self.lowerer.ty(row)? else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a structural tree attribute row".to_string(),
            }
            .into());
        };

        self.lower_tree_aggregate(row, resolution)
    }

    /// Lower one nominal attribute row to its field aggregate.
    fn lower_tree_aggregate(
        &mut self,
        row: dir::GlobalTypeId,
        resolution: &dir::TreeResolution,
    ) -> CompilerResult<mir::Value> {
        let row = self.lowerer.reduced_type(row)?;
        let dir::Type::Application(instance) = self.lowerer.ty(row)? else {
            return Err(CompilerError::Internal {
                message: "tree aggregate row is not nominal".to_string(),
            });
        };
        let owner = instance.symbol.module_id;
        let representation = self.lower_nominal(row)?;
        let ty = self.lower_type(row)?;
        let nominal = self.lowerer.nominal(&representation.key)?;
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
        let children = self.lowerer.strings.intern("children");
        let children_key = dir::StaticKey::Name(children);
        if !resolution.children.is_empty()
            && let Some((_, symbol)) = members.iter().find(|(key, _)| *key == children_key)
        {
            let ty = self.lowerer.symbol_type(symbol.into_global(owner))?;
            let value = self.lower_tree_children(ty, &resolution.children)?;
            values.push((children_key, value));
        }

        // build the aggregate in field declaration order
        let mut ordered = Vec::with_capacity(fields.len());
        for field in fields {
            let Some((_, value)) = values.iter().find(|(key, _)| *key == field) else {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "a defaulted tree attribute".to_string(),
                }
                .into());
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
        if let Some(value) = attribute.value {
            let Ok(expression) = value.local_id.try_into_typed::<dir::Expression>() else {
                return Err(CompilerError::Internal {
                    message: "tree attribute bound a non-expression value".to_string(),
                });
            };

            return self.lower_expression(expression);
        }
        let literal = match attribute.text {
            Some(text) => dir::ScalarLiteral::String(text),
            // provide true for bare attributes
            None => dir::ScalarLiteral::Boolean(true),
        };
        let reduced = self.lowerer.reduced_type(attribute.ty)?;
        let carrier = self.lower_type(reduced)?;
        let carrier = self.builder.tree().get(carrier).clone();

        self.lower_constant(literal, carrier)
    }

    /// Lower the children of one tree literal to their tuple.
    fn lower_tree_children(
        &mut self,
        tuple: dir::GlobalTypeId,
        children: &[dir::TreeChildBinding],
    ) -> CompilerResult<mir::Value> {
        let tuple = self.lowerer.reduced_type(tuple)?;
        let ty = self.lower_type(tuple)?;
        let mut values = Vec::with_capacity(children.len());
        for child in children {
            match child {
                dir::TreeChildBinding::Text { value, ty } => {
                    let reduced = self.lowerer.reduced_type(*ty)?;
                    let carrier = self.lower_type(reduced)?;
                    let carrier = self.builder.tree().get(carrier).clone();
                    values.push(self.lower_constant(dir::ScalarLiteral::String(*value), carrier)?);
                }
                dir::TreeChildBinding::Expression { node, .. } => {
                    let Ok(expression) = node.local_id.try_into_typed::<dir::Expression>() else {
                        return Err(CompilerError::Internal {
                            message: "tree child bound a non-expression value".to_string(),
                        });
                    };
                    values.push(self.lower_expression(expression)?);
                }
                dir::TreeChildBinding::Spread { .. } => {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a spread tree child".to_string(),
                    }
                    .into());
                }
            }
        }

        Ok(self.builder.aggregate(ty, values))
    }

    /// Lower one class component construction with its props row.
    fn lower_tree_construct(
        &mut self,
        construct: &dir::ConstructResolution,
        resolution: &dir::TreeResolution,
    ) -> CompilerResult<mir::Value> {
        let dir::ConstructTarget::Class(candidate) = &construct.target else {
            return Err(CompilerError::Internal {
                message: "tree component constructed a non-class target".to_string(),
            });
        };
        let [props] = construct.arguments.as_slice() else {
            return Err(CompilerError::Internal {
                message: "tree component construction bound an unexpected argument count"
                    .to_string(),
            });
        };
        let props = self.lower_tree_attributes(props.ty, resolution)?;

        self.lower_class_instance(construct.return_type, &candidate.constructor, vec![props])
    }
}
