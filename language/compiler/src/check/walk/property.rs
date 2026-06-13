use destack_dir as dir;
use std::ptr::NonNull;

use crate::CompilerResult;
use crate::check::{
    FlowState, GenericInductionDeclaration, GenericInductionPosition, Origin, Receiver,
    ReceiverBinding, Relation, WalkState,
};

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

    /// Enter one contextual receiver scope.
    pub(in crate::check) fn enter_receiver_maybe(
        &mut self,
        receiver: Option<Receiver>,
    ) -> Option<ReceiverGuard> {
        if let Some(receiver) = receiver {
            self.flow_mut().push_receiver(receiver);
            Some(ReceiverGuard::new(self.flow_mut()))
        } else {
            None
        }
    }

    /// Walk one literal's properties and return its direct fields.
    /// Returns whether any spread property defers the literal's shape
    /// to selection.
    pub(in crate::check) fn walk_literal_properties(
        &mut self,
        properties: &[dir::LocalNodeId<dir::Property>],
    ) -> CompilerResult<(Vec<dir::TypeField>, bool)> {
        let mut fields = Vec::new();
        let mut has_spread = false;
        for property in properties {
            has_spread |= matches!(self.tree.get(*property), dir::Property::Spread { .. });
            fields.extend(self.walk_property(*property, self.tree.get(*property))?);
        }

        Ok((fields, has_spread))
    }

    /// Walk one object literal property and return its shape field.
    /// Spread and computed properties contribute no direct field.
    ///
    /// Example:
    /// ```ds
    /// { name: value, method() { value } }
    /// ```
    pub(in crate::check) fn walk_property(
        &mut self,
        id: dir::LocalNodeId<dir::Property>,
        property: &dir::Property,
    ) -> CompilerResult<Option<dir::TypeField>> {
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any())? else {
            return Ok(None);
        };

        match property {
            // { key: value }
            dir::Property::Field { key, value, .. } => {
                let (key, value) = (*key, *value);

                // compute runtime property key
                if let dir::Key::Expression(key) = key {
                    self.walk_expression(key, self.tree.get(key))?;
                }
                self.walk_expression(value, self.tree.get(value))?;

                let Some(key) = key.direct_static_key() else {
                    return Ok(None);
                };

                Ok(Some(dir::TypeField {
                    key,
                    ty: self.node_type(value)?,
                    is_optional: false,
                    is_readonly: false,
                }))
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

                // walk signature before reading its inputs
                let source = id.into_global_any(self.module);
                let template = self.signature_template(source, None, symbol, signature)?;
                self.walk_function_signature(template, signature)?;
                let result = self.function_result_type(id.into_any(), signature, body)?;

                // write the method's function type
                let method =
                    self.function_signature_type(id.into_any(), signature, template, None, result)?;
                if let Some(symbol) = symbol {
                    self.declare_symbol_type(symbol, method)?;
                }

                // walk method body after its result exists
                if let (Some(symbol), Some(body), Some(result)) = (symbol, body, result) {
                    self.walk_function_body(symbol, signature, body, result, None)?;
                }

                let Some(key) = key.and_then(dir::Key::direct_static_key) else {
                    return Ok(None);
                };

                Ok(Some(dir::TypeField {
                    key,
                    ty: method,
                    is_optional: false,
                    is_readonly: false,
                }))
            }
            // { ...value }
            dir::Property::Spread { value } => {
                let value = *value;
                self.walk_expression(value, self.tree.get(value))?;

                Ok(None)
            }
            // ignore damaged syntax
            dir::Property::Error => Ok(None),
        }
    }

    /// Walk one declaration member and return its checked row.
    ///
    /// Example:
    /// ```ds
    /// field: string = "value"
    /// ```
    pub(in crate::check) fn walk_member(
        &mut self,
        id: dir::LocalNodeId<dir::Member>,
        member: &dir::Member,
        receiver_scope: Option<Receiver>,
        induction_declaration: Option<GenericInductionDeclaration>,
    ) -> CompilerResult<Option<dir::DefinitionMember>> {
        let member_receiver = match member {
            dir::Member::StaticBlock { .. } | dir::Member::ComptimeBlock { .. } => None,
            dir::Member::Field { .. }
            | dir::Member::Method { .. }
            | dir::Member::AssociatedType { .. }
            | dir::Member::AssociatedConst { .. } => receiver_scope,
            dir::Member::Error => None,
        };

        let Some(_guard) = self.enter_decorated_static_guard(id.into_any())? else {
            return Ok(None);
        };
        let condition = self.member_condition(id.into_any())?;
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

                // tie the member symbol to its value
                if let (Some(value), Some(symbol)) = (value, symbol) {
                    let induction = GenericInductionDeclaration::new(source, parent, Some(symbol));
                    self.record_type_induction_site(induction, value);
                    self.declare_symbol_type(symbol, value)?;
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
                        condition,
                    },
                )))
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
                    .map(|value| self.lower_static_predicate(value))
                    .transpose()?;

                let symbol = self
                    .check
                    .module(self.module)
                    .declaration_symbol(id.into_any());
                if let Some(symbol) = symbol {
                    let ty = match declared {
                        Some(declared) => declared,
                        None => self.symbol_type(symbol)?,
                    };
                    self.declare_symbol_type(symbol, ty)?;

                    if let Some(written) = written {
                        // the written value flows into the declared type
                        let origin = Origin::Node(id.into_global_any(self.module));
                        self.relate_type(origin, Relation::Assignable, written, ty);
                        self.declare_symbol_value(symbol, written)?;
                    }
                }

                let (Some(symbol), Some(ty)) = (symbol, declared) else {
                    return Ok(None);
                };

                Ok(Some(dir::DefinitionMember::AssociatedConst(
                    dir::AssociatedConstDefinition {
                        symbol,
                        source: id.into_global_any(self.module),
                        key: dir::StaticKey::Name(name),
                        ty,
                        value: None,
                        condition,
                    },
                )))
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

                if declared_type.is_none() {
                    self.check
                        .report_missing_type_annotation(self.module, id.into_any());
                }

                // check computed member keys in declaration context
                if let dir::Key::Expression(key) = key {
                    let before_key = self.fork_flow();
                    self.walk_expression(key, self.tree.get(key))?;
                    self.restore_flow(before_key);
                }
                if let Some(default) = default {
                    // check field defaults in declaration context
                    let before_default = self.fork_flow();
                    self.walk_expression(default, self.tree.get(default))?;
                    self.restore_flow(before_default);
                }

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
                            self.optional_value_type(written, id.into_any())?
                        } else {
                            written
                        };

                        Some(written)
                    }
                    None => None,
                };

                // tie the field symbol to its type
                let symbol = self
                    .check
                    .module(self.module)
                    .declaration_symbol(id.into_any());
                if let (Some(field_type), Some(symbol)) = (field_type, symbol) {
                    if let Some(induction) = induction_declaration {
                        self.record_type_induction_site(induction, field_type);
                    }
                    self.declare_symbol_type(symbol, field_type)?;
                }

                // defaults must fit the declared field type
                if let (Some(field_type), Some(default)) = (field_type, default) {
                    self.expect_assignable(default, field_type)?;
                }

                let (Some(symbol), Some(ty), Some(key)) =
                    (symbol, field_type, key.direct_static_key())
                else {
                    return Ok(None);
                };

                Ok(Some(dir::DefinitionMember::Field(dir::FieldDefinition {
                    space: if is_static {
                        dir::MemberSpace::Static
                    } else {
                        dir::MemberSpace::Instance
                    },
                    symbol,
                    source: id.into_global_any(self.module),
                    key,
                    ty,
                    is_abstract,
                    is_override,
                    condition,
                })))
            }
            // method() {}
            dir::Member::Method {
                key,
                signature,
                body,
                abstraction,
                is_static,
                is_override,
                ..
            } => {
                let (key, body, is_static) = (*key, *body, *is_static);
                let (abstraction, is_override) = (*abstraction, *is_override);

                if let Some(dir::Key::Expression(key)) = key {
                    // check computed member keys in declaration context
                    let before_key = self.fork_flow();
                    self.walk_expression(key, self.tree.get(key))?;
                    self.restore_flow(before_key);
                }

                let symbol = self
                    .check
                    .module(self.module)
                    .declaration_symbol(id.into_any());
                let receiver_owner = member_receiver.and_then(|scope| scope.owner);
                let parent = self.enclosing_generic_template(receiver_scope, induction_declaration);
                let source = id.into_global_any(self.module);
                let template = self.signature_template(source, parent, symbol, signature)?;
                let captured_template = template.or(parent);

                // walk signature before reading its inputs
                self.walk_function_signature(template, signature)?;
                let implicit_receiver_scope = if is_static { None } else { receiver_scope };
                let receiver = self.method_receiver_binding(
                    id,
                    signature,
                    receiver_owner,
                    implicit_receiver_scope,
                )?;
                let result = self.method_result_type(id, signature, body, receiver)?;

                // write the method's function type
                let receiver_type = receiver
                    .filter(|_| Self::is_receiver_visible_in_method_type(signature))
                    .map(|receiver| receiver.receiver.ty);
                let method = self.function_signature_type(
                    id.into_any(),
                    signature,
                    captured_template,
                    receiver_type,
                    result,
                )?;
                let induction = GenericInductionDeclaration::new(source, parent, symbol);
                self.record_type_induction_site(induction, method);

                // tie the method symbol to its type
                if let Some(symbol) = symbol {
                    self.declare_symbol_type(symbol, method)?;
                }

                // walk method body after its result exists
                if let (Some(body), Some(symbol), Some(result)) = (body, symbol, result) {
                    self.walk_function_body(symbol, signature, body, result, receiver)?;
                }

                // place the method into its declaration slot
                let slot = match signature.role {
                    Some(dir::FunctionRole::Constructor) => dir::MemberSlot::Constructor,
                    Some(dir::FunctionRole::New) => dir::MemberSlot::New,
                    Some(dir::FunctionRole::Call) => dir::MemberSlot::Call,
                    _ => match key.and_then(dir::Key::direct_static_key) {
                        Some(key) => dir::MemberSlot::Key(key),
                        None => return Ok(None),
                    },
                };

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
                    ty: method,
                    abstraction,
                    is_override,
                    condition,
                })))
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
            // ignore damaged syntax
            dir::Member::Error => Ok(None),
        }
    }

    /// Walk one type-space member and return its checked row.
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
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any())? else {
            return Ok(None);
        };
        let condition = self.member_condition(id.into_any())?;
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
                    self.optional_value_type(declared_ty, id.into_any())?
                } else {
                    declared_ty
                };

                // tie the field symbol to its type
                let symbol = self
                    .check
                    .module(self.module)
                    .declaration_symbol(id.into_any());
                if let Some(symbol) = symbol {
                    self.declare_symbol_type(symbol, written)?;
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
                    ty: written,
                    is_abstract: false,
                    is_override: false,
                    condition,
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
                let symbol = self
                    .check
                    .module(self.module)
                    .declaration_symbol(id.into_any());
                let parent = self.enclosing_generic_template(receiver_scope, induction_declaration);
                let template = self.signature_template(source, parent, symbol, signature)?;
                let captured_template = template.or(parent);

                // walk signature before reading its inputs
                self.walk_function_signature(template, signature)?;
                let result = self.function_result_type(id.into_any(), signature, body)?;
                let receiver_type = receiver_scope
                    .filter(|_| !is_static)
                    .map(|receiver| receiver.ty);
                let method = self.function_signature_type(
                    id.into_any(),
                    signature,
                    captured_template,
                    receiver_type,
                    result,
                )?;

                // tie the method symbol to its type
                if let Some(symbol) = symbol {
                    self.declare_symbol_type(symbol, method)?;
                }

                // walk default method bodies
                if let (Some(body), Some(symbol), Some(result)) = (body, symbol, result) {
                    self.walk_function_body(symbol, signature, body, result, None)?;
                }

                let slot = match signature.role {
                    Some(dir::FunctionRole::Constructor) => dir::MemberSlot::Constructor,
                    Some(dir::FunctionRole::New) => dir::MemberSlot::New,
                    Some(dir::FunctionRole::Call) => dir::MemberSlot::Call,
                    _ => match key.direct_static_key() {
                        Some(key) => dir::MemberSlot::Key(key),
                        None => return Ok(None),
                    },
                };

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
                    ty: method,
                    abstraction: dir::MethodAbstraction::Concrete,
                    is_override: false,
                    condition,
                })))
            }
            // (value: T): U
            dir::TypeMember::CallSignature { signature } => {
                let ty = self.function_type(id.into_any(), signature, None, None)?;

                Ok(Some(dir::DefinitionMember::CallSignature(
                    dir::SignatureDefinition {
                        source,
                        ty,
                        condition,
                    },
                )))
            }
            // new (value: T): U
            dir::TypeMember::ConstructSignature { signature } => {
                let ty = self.constructor_type(id.into_any(), signature, None, None)?;

                Ok(Some(dir::DefinitionMember::ConstructSignature(
                    dir::SignatureDefinition {
                        source,
                        ty,
                        condition,
                    },
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
                        condition,
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

                // tie the member symbol to its value
                if let (Some(value), Some(symbol)) = (value, symbol) {
                    self.declare_symbol_type(symbol, value)?;
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
                        condition,
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
                    .map(|value| self.lower_static_predicate(value))
                    .transpose()?;

                let symbol = self
                    .check
                    .module(self.module)
                    .declaration_symbol(id.into_any());
                if let Some(symbol) = symbol {
                    if let Some(declared) = declared {
                        self.declare_symbol_type(symbol, declared)?;
                    }
                    if let Some(written) = written {
                        if let Some(declared) = declared {
                            let origin = Origin::Node(source);
                            self.relate_type(origin, Relation::Assignable, written, declared);
                        }
                        self.declare_symbol_value(symbol, written)?;
                    }
                }

                let (Some(symbol), Some(ty)) = (symbol, declared) else {
                    return Ok(None);
                };

                Ok(Some(dir::DefinitionMember::AssociatedConst(
                    dir::AssociatedConstDefinition {
                        symbol,
                        source,
                        key: dir::StaticKey::Name(name),
                        ty,
                        value: None,
                        condition,
                    },
                )))
            }
            // ignore damaged syntax
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
        owner: Option<dir::GlobalSymbolId>,
        implicit_receiver_scope: Option<Receiver>,
    ) -> CompilerResult<Option<ReceiverBinding>> {
        // prefer explicit `this` parameters before implicit receivers
        if let Some(parameter) = signature.this_parameter {
            let receiver = self.this_parameter_receiver_binding(parameter, owner)?;

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
        if self
            .check
            .module(self.module)
            .profile
            .flags
            .no_implicit_receivers
        {
            self.check
                .report_implicit_receiver(self.module, id.into_any());
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

    /// Return one method body or call signature result type.
    ///
    /// Example:
    /// ```ds
    /// method(): number { 1 }
    /// ```
    fn method_result_type(
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

        // return regular method result
        self.function_result_type(id.into_any(), signature, body)
    }
}
