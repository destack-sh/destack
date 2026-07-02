use destack_dir as dir;
use std::ptr::NonNull;

use crate::check::{
    Expectation, FlowBranch, FlowState, GenericInductionDeclaration, GenericInductionPosition,
    Origin, Receiver, ReceiverBinding, Relation, ValueUse, WalkState, Widening,
};
use crate::{CompilerError, CompilerResult};

/// Declaration member and body yielded by one member header.
pub(in crate::check) struct MemberHeader {
    /// The member installed into the containing definition.
    pub(in crate::check) definition: Option<dir::DefinitionMember>,
    /// The method body checked after the containing definition exists.
    pub(in crate::check) body: Option<MethodBody>,
}

/// Checked method body context.
#[derive(Clone, Copy)]
pub(in crate::check) struct MethodBody {
    /// The receiver binding visible inside the method body.
    pub(in crate::check) receiver: Option<ReceiverBinding>,
    /// The checked result type expected from the method body.
    pub(in crate::check) result: dir::GlobalTypeId,
}

/// One active receiver scope.
pub(in crate::check) struct ReceiverGuard {
    /// The guarded flow state.
    flow: NonNull<FlowState>,
}

impl ReceiverGuard {
    /// Return one active receiver scope.
    fn new(flow: &mut FlowState) -> Self {
        Self {
            flow: NonNull::from(flow),
        }
    }
}

impl Drop for ReceiverGuard {
    fn drop(&mut self) {
        // pop the receiver owned by this scope
        unsafe {
            self.flow.as_mut().pop_receiver();
        }
    }
}

impl WalkState<'_, '_> {
    /// Enter one explicit contextual receiver scope.
    pub(in crate::check) fn enter_receiver_scope(
        &mut self,
        receiver: Option<Receiver>,
    ) -> ReceiverGuard {
        self.flow_mut().push_receiver_scope(receiver);

        ReceiverGuard::new(self.flow_mut())
    }

    /// Walk one literal's properties.
    pub(in crate::check) fn walk_literal_properties(
        &mut self,
        properties: &[dir::LocalNodeId<dir::Property>],
    ) -> CompilerResult<()> {
        for property in properties {
            self.walk_property(*property, self.tree.get(*property))?;
        }

        Ok(())
    }

    /// Walk one object literal property.
    ///
    /// Example:
    /// ```ds
    /// { name: value, method() { value } }
    /// ```
    pub(in crate::check) fn walk_property(
        &mut self,
        id: dir::LocalNodeId<dir::Property>,
        property: &dir::Property,
    ) -> CompilerResult<()> {
        if !self.decide_decorated_presence(id.into_any())? {
            return Ok(());
        }

        match property {
            // { key: value }
            dir::Property::Field { key, value, .. } => {
                let (key, value) = (*key, *value);

                // compute runtime property key
                if let dir::Key::Expression(key) = key {
                    self.walk_expression(key, self.tree.get(key))?;
                }
                self.walk_expression(value, self.tree.get(value))?;

                Ok(())
            }
            // { method() {} }
            dir::Property::Method {
                key,
                signature,
                body,
            } => {
                let (key, body) = (*key, *body);

                if let Some(dir::Key::Expression(key)) = key {
                    // compute runtime property key
                    self.walk_expression(key, self.tree.get(key))?;
                }

                let symbol = self
                    .check
                    .module(self.module)
                    .declaration_symbol(id.into_any());

                // open signature parameters before building the method type
                let source = id.into_global_any(self.module);
                let template = self.open_signature_template(source, None, symbol, signature)?;
                let header = self.walk_function_signature(template, signature)?;
                let result = self.walk_function_result_type(id.into_any(), signature, body)?;

                // write the method's function type
                let method =
                    self.walk_function_signature_type(signature, header, None, None, result)?;
                if let Some(symbol) = symbol {
                    self.bind_symbol_type(symbol, method)?;
                }

                // walk method body after its result exists
                if let (Some(symbol), Some(body), Some(result)) = (symbol, body, result) {
                    self.walk_function_body(symbol, signature, body, result, None)?;
                }

                Ok(())
            }
            // { ...value }
            dir::Property::Spread { value } => {
                let value = *value;
                self.walk_expression(value, self.tree.get(value))?;

                Ok(())
            }
            // ignore damaged nodes
            dir::Property::Error => Ok(()),
        }
    }

    /// Walk one declaration member header.
    ///
    /// Example:
    /// ```ds
    /// field: string = "value"
    /// ```
    pub(in crate::check) fn walk_member_declaration(
        &mut self,
        id: dir::LocalNodeId<dir::Member>,
        member: &dir::Member,
        receiver_scope: Option<Receiver>,
        induction_declaration: Option<GenericInductionDeclaration>,
        is_ambient_scope: bool,
    ) -> CompilerResult<MemberHeader> {
        let member_receiver = match member {
            dir::Member::StaticBlock { .. } | dir::Member::ComptimeBlock { .. } => None,
            dir::Member::Field { .. }
            | dir::Member::Method { .. }
            | dir::Member::AssociatedType { .. }
            | dir::Member::AssociatedConst { .. } => receiver_scope,
            dir::Member::Error => None,
        };

        if !self.decide_decorated_presence(id.into_any())? {
            return Ok(MemberHeader {
                definition: None,
                body: None,
            });
        }
        let _receiver = self.enter_receiver_scope(member_receiver);

        match member {
            // type Item = T
            dir::Member::AssociatedType {
                name,
                generic_parameters,
                where_clauses,
                constraint,
                value,
                ..
            } => {
                let (name, constraint, value) = (*name, *constraint, *value);
                let symbol = self
                    .check
                    .module(self.module)
                    .declaration_symbol(id.into_any());

                // walk generic parameters
                let source = id.into_global_any(self.module);
                let parent = self.enclosing_generic_template(receiver_scope, induction_declaration);
                if let Some(symbol) = symbol {
                    self.walk_generic_template(source, parent, Some(symbol), generic_parameters)?;
                }

                // walk where clauses
                for where_clause in where_clauses {
                    self.walk_where_clause(*where_clause)?;
                }

                // walk constraint and value
                let constraint = constraint
                    .map(|constraint| self.walk_type_expression(constraint))
                    .transpose()?;
                let value = value
                    .map(|value| self.walk_type_expression(value))
                    .transpose()?;

                // write the member symbol type
                if let (Some(value), Some(symbol)) = (value, symbol) {
                    let induction = GenericInductionDeclaration::new(source, parent, Some(symbol));
                    self.push_type_induction_site(induction, value);
                    self.bind_symbol_type(symbol, value)?;
                }

                let Some(symbol) = symbol else {
                    return Ok(MemberHeader {
                        definition: None,
                        body: None,
                    });
                };

                Ok(MemberHeader {
                    definition: Some(dir::DefinitionMember::AssociatedType(
                        dir::AssociatedTypeDefinition {
                            symbol,
                            source,
                            key: dir::StaticKey::Name(name),
                            constraint,
                            value,
                        },
                    )),
                    body: None,
                })
            }
            // const item: T = value
            dir::Member::AssociatedConst {
                name,
                declared_type,
                value,
                ..
            } => {
                let (name, declared_type, value) = (*name, *declared_type, *value);

                if value.is_some() && declared_type.is_none() {
                    self.check
                        .report_missing_type_annotation(self.module, id.into_any());
                }

                // walk the declared type
                let declared = declared_type
                    .map(|declared_type| self.walk_type_expression(declared_type))
                    .transpose()?;

                // the value is a static written form checked against the type
                let written = value
                    .map(|value| self.walk_static_term(value))
                    .transpose()?;

                let symbol = self
                    .check
                    .module(self.module)
                    .declaration_symbol(id.into_any());
                if let Some(symbol) = symbol {
                    let ty = match declared {
                        Some(declared) => declared,
                        None => self.symbol_type_slot(symbol)?,
                    };
                    self.bind_symbol_type(symbol, ty)?;

                    if let Some(written) = written {
                        // the written value constraints into the declared type
                        let origin = Origin::Node(id.into_global_any(self.module));
                        self.relate_value(
                            origin,
                            ValueUse::Store,
                            Relation::Assignable,
                            written,
                            ty,
                        );
                        self.commit_static_value(symbol, written)?;
                    }
                }

                let (Some(symbol), Some(_)) = (symbol, declared) else {
                    return Ok(MemberHeader {
                        definition: None,
                        body: None,
                    });
                };

                Ok(MemberHeader {
                    definition: Some(dir::DefinitionMember::AssociatedConst(
                        dir::AssociatedConstDefinition {
                            symbol,
                            source: id.into_global_any(self.module),
                            key: dir::StaticKey::Name(name),
                            value: None,
                        },
                    )),
                    body: None,
                })
            }
            // field: T = value
            dir::Member::Field {
                key,
                declared_type,
                default,
                is_optional,
                is_static,
                is_abstract,
                is_override,
                ..
            } => {
                let (key, declared_type, default, is_optional, is_static) =
                    (*key, *declared_type, *default, *is_optional, *is_static);
                let (is_abstract, is_override) = (*is_abstract, *is_override);
                if declared_type.is_none() && default.is_none() {
                    self.check
                        .report_missing_type_annotation(self.module, id.into_any());
                }

                // check computed member keys in declaration context
                if let dir::Key::Expression(key) = key {
                    let before_key = self.fork_flow();
                    self.walk_expression(key, self.tree.get(key))?;
                    self.restore_flow(before_key);
                }

                // resolve the field symbol before inferred storage opens its slot
                let symbol = self
                    .check
                    .module(self.module)
                    .declaration_symbol(id.into_any());

                // derive the declared field type
                let field_type = match declared_type {
                    Some(declared_type) => {
                        let written = self.walk_type_expression(declared_type)?;
                        let written = self.induce_constraint_type(
                            id.into_any(),
                            written,
                            GenericInductionPosition::Storage,
                        )?;
                        let written = if is_optional {
                            self.optional_value_type(written)?
                        } else {
                            written
                        };

                        Some(written)
                    }
                    None => {
                        if let (Some(symbol), Some(default)) = (symbol, default) {
                            let before_default = self.fork_flow();
                            self.walk_expression(default, self.tree.get(default))?;
                            self.restore_flow(before_default);

                            self.queue_bind_initializer(symbol, default, Widening::Widen)?;
                        }

                        None
                    }
                };

                // write the field symbol type
                if let (Some(field_type), Some(symbol)) = (field_type, symbol) {
                    if let Some(induction) = induction_declaration {
                        self.push_type_induction_site(induction, field_type);
                    }
                    self.bind_symbol_type(symbol, field_type)?;
                }

                // check defaults after the field type is known
                if let (Some(field_type), Some(default)) = (field_type, default) {
                    let before_default = self.fork_flow();
                    let expectation = Expectation::assignable(
                        field_type,
                        Origin::Node(default.into_global_any(self.module)),
                        ValueUse::Store,
                    );
                    self.walk_expression(default, self.tree.get(default))?;
                    self.queue_node_check(default, expectation)?;
                    self.restore_flow(before_default);
                }

                let (Some(symbol), Some(key)) = (symbol, key.direct_static_key()) else {
                    return Ok(MemberHeader {
                        definition: None,
                        body: None,
                    });
                };

                Ok(MemberHeader {
                    definition: Some(dir::DefinitionMember::Field(dir::FieldDefinition {
                        space: if is_static {
                            dir::MemberSpace::Static
                        } else {
                            dir::MemberSpace::Instance
                        },
                        symbol,
                        source: id.into_global_any(self.module),
                        key,
                        initializer: default.map(|default| default.into_global_any(self.module)),
                        is_abstract,
                        is_override,
                    })),
                    body: None,
                })
            }
            // method() {}
            dir::Member::Method {
                key,
                signature,
                body,
                abstraction,
                is_ambient,
                is_static,
                is_override,
                ..
            } => {
                if let Some(dir::Key::Expression(key)) = *key {
                    // check computed member keys in declaration context
                    let before_key = self.fork_flow();
                    self.walk_expression(key, self.tree.get(key))?;
                    self.restore_flow(before_key);
                }

                // place the method into its declaration slot
                let slot = match signature.role {
                    Some(dir::FunctionRole::Constructor) => dir::MemberSlot::Constructor,
                    Some(dir::FunctionRole::New) => dir::MemberSlot::New,
                    Some(dir::FunctionRole::Call) => dir::MemberSlot::Call,
                    _ => match (*key).and_then(dir::Key::direct_static_key) {
                        Some(key) => dir::MemberSlot::Key(key),
                        None => {
                            return Ok(MemberHeader {
                                definition: None,
                                body: None,
                            });
                        }
                    },
                };
                let Some(symbol) = self
                    .check
                    .module(self.module)
                    .declaration_symbol(id.into_any())
                else {
                    return Err(CompilerError::Internal {
                        message: format!("method member {id:?} has no declaration symbol"),
                    });
                };
                let parent = self.enclosing_generic_template(receiver_scope, induction_declaration);
                let source = id.into_global_any(self.module);
                let template =
                    self.open_signature_template(source, parent, Some(symbol), signature)?;

                // open signature parameters before building the method type
                let header = self.walk_function_signature(template, signature)?;
                let this_parameter = header.this_parameter;
                let needs_body = body.is_none()
                    && !is_ambient_scope
                    && !*is_ambient
                    && !abstraction.is_abstract();
                if needs_body {
                    let member = self.method_body_name(*key, signature);
                    let source = id.into_global_any(self.module);
                    self.check.report_missing_declaration_body(source, member);
                }
                let implicit_receiver_scope = if *is_static { None } else { receiver_scope };
                let receiver = self.method_receiver_binding(
                    id,
                    signature,
                    implicit_receiver_scope,
                    this_parameter,
                )?;
                let result = self.walk_method_result_type(id, signature, *body, receiver)?;

                // write the method's function type
                let receiver_type = receiver
                    .filter(|_| Self::is_receiver_visible_in_method_type(signature))
                    .map(|receiver| receiver.receiver.ty);
                let method = self.walk_function_signature_type(
                    signature,
                    header,
                    Some(GenericInductionDeclaration::new(
                        source,
                        parent,
                        Some(symbol),
                    )),
                    receiver_type,
                    result,
                )?;
                let induction = GenericInductionDeclaration::new(source, parent, Some(symbol));
                self.push_type_induction_site(induction, method);

                // write the method symbol type
                self.bind_symbol_type(symbol, method)?;

                let body = match (*body, result) {
                    (Some(_), Some(result))
                        if !is_ambient_scope && !*is_ambient && !abstraction.is_abstract() =>
                    {
                        Some(MethodBody { receiver, result })
                    }
                    _ => None,
                };

                Ok(MemberHeader {
                    definition: Some(dir::DefinitionMember::Method(dir::MethodDefinition {
                        space: if *is_static {
                            dir::MemberSpace::Static
                        } else {
                            dir::MemberSpace::Instance
                        },
                        symbol,
                        source,
                        slot,
                        role: signature.role,
                        abstraction: *abstraction,
                        is_override: *is_override,
                    })),
                    body,
                })
            }
            // static { ... }, comptime { ... }
            dir::Member::StaticBlock { .. } | dir::Member::ComptimeBlock { .. } => {
                Ok(MemberHeader {
                    definition: None,
                    body: None,
                })
            }
            // ignore damaged nodes
            dir::Member::Error => Ok(MemberHeader {
                definition: None,
                body: None,
            }),
        }
    }

    /// Walk one declaration member body after its containing definition exists.
    pub(in crate::check) fn walk_member_body(
        &mut self,
        id: dir::LocalNodeId<dir::Member>,
        member: &dir::Member,
        receiver_scope: Option<Receiver>,
        is_ambient_scope: bool,
        method_body: Option<MethodBody>,
    ) -> CompilerResult<Option<FlowBranch>> {
        let member_receiver = match member {
            dir::Member::StaticBlock { .. } | dir::Member::ComptimeBlock { .. } => None,
            dir::Member::Field { .. }
            | dir::Member::Method { .. }
            | dir::Member::AssociatedType { .. }
            | dir::Member::AssociatedConst { .. } => receiver_scope,
            dir::Member::Error => None,
        };

        if !self.decide_decorated_presence(id.into_any())? {
            return Ok(None);
        }
        let _receiver = self.enter_receiver_scope(member_receiver);

        match member {
            // method() {}
            dir::Member::Method {
                signature,
                body,
                is_ambient,
                abstraction,
                ..
            } => {
                if body.is_none() || is_ambient_scope || *is_ambient || abstraction.is_abstract() {
                    return Ok(None);
                }
                let Some(body) = *body else {
                    return Ok(None);
                };
                let Some(symbol) = self
                    .check
                    .module(self.module)
                    .declaration_symbol(id.into_any())
                else {
                    return Err(CompilerError::Internal {
                        message: format!("method member {id:?} has no declaration symbol"),
                    });
                };
                let Some(method_body) = method_body else {
                    return Ok(None);
                };
                let branch = self.walk_function_body(
                    symbol,
                    signature,
                    body,
                    method_body.result,
                    method_body.receiver,
                )?;

                if matches!(signature.role, Some(dir::FunctionRole::Constructor)) {
                    Ok(Some(branch))
                } else {
                    Ok(None)
                }
            }
            // static { ... }, comptime { ... }
            dir::Member::StaticBlock { body } | dir::Member::ComptimeBlock { body } => {
                let body = *body;

                // check member blocks in declaration context
                let before_body = self.fork_flow();
                self.walk_expression(body, self.tree.get(body))?;
                self.restore_flow(before_body);

                Ok(None)
            }
            // members without bodies
            dir::Member::Field { .. }
            | dir::Member::AssociatedType { .. }
            | dir::Member::AssociatedConst { .. }
            | dir::Member::Error => Ok(None),
        }
    }

    /// Walk one object type member and return its checked definition member.
    ///
    /// Example:
    /// ```ds
    /// interface Reader { read(): string }
    /// ```
    pub(in crate::check) fn walk_type_member(
        &mut self,
        id: dir::LocalNodeId<dir::TypeMember>,
        member: &dir::TypeMember,
        receiver_scope: Option<Receiver>,
        induction_declaration: Option<GenericInductionDeclaration>,
    ) -> CompilerResult<Option<dir::DefinitionMember>> {
        if !self.decide_decorated_presence(id.into_any())? {
            return Ok(None);
        }
        let _receiver = self.enter_receiver_scope(receiver_scope);
        let source = id.into_global_any(self.module);

        match member {
            // field: T
            dir::TypeMember::Field {
                key,
                declared_type,
                is_static,
                is_optional,
                ..
            } => {
                let (key, declared_type, is_static, is_optional) =
                    (*key, *declared_type, *is_static, *is_optional);

                if declared_type.is_none() {
                    self.check
                        .report_missing_type_annotation(self.module, id.into_any());
                }
                let Some(declared_type) = declared_type else {
                    return Ok(None);
                };
                let declared_ty = self.walk_type_expression(declared_type)?;
                let written = if is_optional {
                    self.optional_value_type(declared_ty)?
                } else {
                    declared_ty
                };

                // write the field symbol type
                let symbol = self
                    .check
                    .module(self.module)
                    .declaration_symbol(id.into_any());
                if let Some(symbol) = symbol {
                    self.bind_symbol_type(symbol, written)?;
                }

                let (Some(symbol), Some(key)) = (symbol, key.direct_static_key()) else {
                    return Ok(None);
                };

                Ok(Some(dir::DefinitionMember::Field(dir::FieldDefinition {
                    space: if is_static {
                        dir::MemberSpace::Static
                    } else {
                        dir::MemberSpace::Instance
                    },
                    symbol,
                    source,
                    key,
                    initializer: None,
                    is_abstract: false,
                    is_override: false,
                })))
            }
            // method(): T
            dir::TypeMember::Method {
                key,
                signature,
                body,
                is_static,
                ..
            } => {
                let (key, body, is_static) = (*key, *body, *is_static);
                let slot = match signature.role {
                    Some(dir::FunctionRole::Constructor) => dir::MemberSlot::Constructor,
                    Some(dir::FunctionRole::New) => dir::MemberSlot::New,
                    Some(dir::FunctionRole::Call) => dir::MemberSlot::Call,
                    _ => match key.direct_static_key() {
                        Some(key) => dir::MemberSlot::Key(key),
                        None => return Ok(None),
                    },
                };
                let Some(symbol) = self
                    .check
                    .module(self.module)
                    .declaration_symbol(id.into_any())
                else {
                    return Err(CompilerError::Internal {
                        message: format!("type method member {id:?} has no declaration symbol"),
                    });
                };
                let parent = self.enclosing_generic_template(receiver_scope, induction_declaration);
                let template =
                    self.open_signature_template(source, parent, Some(symbol), signature)?;

                // open signature parameters before building the method type
                let header = self.walk_function_signature(template, signature)?;
                let result = self.walk_function_result_type(id.into_any(), signature, body)?;
                let receiver_type = receiver_scope
                    .filter(|_| !is_static)
                    .map(|receiver| receiver.ty);
                let method = self.walk_function_signature_type(
                    signature,
                    header,
                    None,
                    receiver_type,
                    result,
                )?;

                // write the method symbol type
                self.bind_symbol_type(symbol, method)?;

                // walk default method bodies
                if let (Some(body), Some(result)) = (body, result) {
                    self.walk_function_body(symbol, signature, body, result, None)?;
                }

                Ok(Some(dir::DefinitionMember::Method(dir::MethodDefinition {
                    space: if is_static {
                        dir::MemberSpace::Static
                    } else {
                        dir::MemberSpace::Instance
                    },
                    symbol,
                    source,
                    slot,
                    role: signature.role,
                    abstraction: dir::MethodAbstraction::Concrete,
                    is_override: false,
                })))
            }
            // (value: T): U
            dir::TypeMember::CallSignature { signature } => {
                let parent = self.enclosing_generic_template(receiver_scope, induction_declaration);
                let ty = self.walk_function_type(id.into_any(), signature, parent, None)?;

                Ok(Some(dir::DefinitionMember::CallSignature(
                    dir::SignatureDefinition { source, ty },
                )))
            }
            // new (value: T): U
            dir::TypeMember::ConstructSignature { signature } => {
                let parent = self.enclosing_generic_template(receiver_scope, induction_declaration);
                let ty = self.walk_constructor_type(id.into_any(), signature, parent, None)?;

                Ok(Some(dir::DefinitionMember::ConstructSignature(
                    dir::SignatureDefinition { source, ty },
                )))
            }
            // [key: K]: V
            dir::TypeMember::IndexSignature {
                key_type,
                value_type,
                ..
            } => {
                let (key_type, value_type) = (*key_type, *value_type);
                let key_type = self.walk_type_expression(key_type)?;
                let value_type = self.walk_type_expression(value_type)?;

                // index signatures write as their value function shape
                let _ = key_type;

                Ok(Some(dir::DefinitionMember::IndexSignature(
                    dir::SignatureDefinition {
                        source,
                        ty: value_type,
                    },
                )))
            }
            // type Item = T
            dir::TypeMember::AssociatedType {
                name,
                generic_parameters,
                where_clauses,
                constraint,
                value,
                ..
            } => {
                let (name, constraint, value) = (*name, *constraint, *value);
                let symbol = self
                    .check
                    .module(self.module)
                    .declaration_symbol(id.into_any());

                // walk generic parameters
                let parent = self.enclosing_generic_template(receiver_scope, induction_declaration);
                if let Some(symbol) = symbol {
                    self.walk_generic_template(source, parent, Some(symbol), generic_parameters)?;
                }
                for where_clause in where_clauses {
                    self.walk_where_clause(*where_clause)?;
                }

                let constraint = constraint
                    .map(|constraint| self.walk_type_expression(constraint))
                    .transpose()?;
                let value = value
                    .map(|value| self.walk_type_expression(value))
                    .transpose()?;

                // write the member symbol type
                if let (Some(value), Some(symbol)) = (value, symbol) {
                    self.bind_symbol_type(symbol, value)?;
                }

                let Some(symbol) = symbol else {
                    return Ok(None);
                };

                Ok(Some(dir::DefinitionMember::AssociatedType(
                    dir::AssociatedTypeDefinition {
                        symbol,
                        source,
                        key: dir::StaticKey::Name(name),
                        constraint,
                        value,
                    },
                )))
            }
            // const item: T = value
            dir::TypeMember::AssociatedConst {
                name,
                declared_type,
                value,
                ..
            } => {
                let (name, declared_type, value) = (*name, *declared_type, *value);

                if value.is_some() && declared_type.is_none() {
                    self.check
                        .report_missing_type_annotation(self.module, id.into_any());
                }

                let declared = declared_type
                    .map(|declared_type| self.walk_type_expression(declared_type))
                    .transpose()?;
                let written = value
                    .map(|value| self.walk_static_term(value))
                    .transpose()?;

                let symbol = self
                    .check
                    .module(self.module)
                    .declaration_symbol(id.into_any());
                if let Some(symbol) = symbol {
                    if let Some(declared) = declared {
                        self.bind_symbol_type(symbol, declared)?;
                    }
                    if let Some(written) = written {
                        if let Some(declared) = declared {
                            let origin = Origin::Node(source);
                            self.relate_value(
                                origin,
                                ValueUse::Store,
                                Relation::Assignable,
                                written,
                                declared,
                            );
                        }
                        self.commit_static_value(symbol, written)?;
                    }
                }

                let (Some(symbol), Some(_)) = (symbol, declared) else {
                    return Ok(None);
                };

                Ok(Some(dir::DefinitionMember::AssociatedConst(
                    dir::AssociatedConstDefinition {
                        symbol,
                        source,
                        key: dir::StaticKey::Name(name),
                        value: None,
                    },
                )))
            }
            // ignore damaged nodes
            dir::TypeMember::Error => Ok(None),
        }
    }

    /// Return the lexical receiver visible inside one method body.
    ///
    /// Example:
    /// ```ds
    /// method(this: Box): number { this.value }
    /// ```
    fn method_receiver_binding(
        &mut self,
        id: dir::LocalNodeId<dir::Member>,
        signature: &dir::FunctionSignature,
        implicit_receiver_scope: Option<Receiver>,
        this_parameter: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<ReceiverBinding>> {
        // prefer explicit `this` parameters before implicit receivers
        if let (Some(parameter), Some(ty)) = (signature.this_parameter, this_parameter) {
            let receiver =
                self.this_parameter_receiver_binding(parameter, implicit_receiver_scope, ty)?;

            return Ok(Some(receiver));
        }

        // return the implicit instance receiver
        let Some(scope) = implicit_receiver_scope else {
            return Ok(None);
        };
        let Some(symbol) = self
            .check
            .module(self.module)
            .implicit_receiver_symbol(id.into_any())
        else {
            return Ok(None);
        };
        if self.check.module(self.module).profile.no_implicit_receivers {
            self.check
                .report_missing_explicit_receiver(self.module, id.into_any());
        }

        Ok(Some(ReceiverBinding {
            symbol,
            receiver: scope,
        }))
    }

    /// Return whether one method type should expose its receiver parameter.
    fn is_receiver_visible_in_method_type(signature: &dir::FunctionSignature) -> bool {
        !matches!(
            signature.role,
            Some(dir::FunctionRole::Constructor | dir::FunctionRole::New)
        )
    }

    /// Return the name used to report one method body requirement.
    fn method_body_name(
        &self,
        key: Option<dir::Key>,
        signature: &dir::FunctionSignature,
    ) -> String {
        match signature.role {
            Some(dir::FunctionRole::Constructor) => "constructor".to_string(),
            Some(dir::FunctionRole::New) => "new".to_string(),
            Some(dir::FunctionRole::Call) => "call".to_string(),
            Some(dir::FunctionRole::Getter) => "get".to_string(),
            Some(dir::FunctionRole::Setter) => "set".to_string(),
            None => key
                .and_then(dir::Key::direct_static_key)
                .map(|key| self.check.format_static_key(&key))
                .unwrap_or_else(|| "method".to_string()),
        }
    }

    /// Walk one method return annotation or return the constructor receiver.
    ///
    /// Example:
    /// ```ds
    /// method(): number { 1 }
    /// ```
    fn walk_method_result_type(
        &mut self,
        id: dir::LocalNodeId<dir::Member>,
        signature: &dir::FunctionSignature,
        body: Option<dir::LocalNodeId<dir::Expression>>,
        receiver: Option<ReceiverBinding>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // use the receiver as the constructor result
        if matches!(
            signature.role,
            Some(dir::FunctionRole::Constructor | dir::FunctionRole::New)
        ) {
            return Ok(receiver.map(|receiver| receiver.receiver.ty));
        }

        // walk regular method result
        let result = self.walk_function_result_type(id.into_any(), signature, body)?;
        let Some(result) = result else {
            return Ok(None);
        };

        // apply the implicit receiver to `this` in result position
        let result = match receiver {
            Some(receiver) => self.apply_receiver_scope(Some(receiver.receiver), result)?,
            None => result,
        };

        Ok(Some(result))
    }
}
