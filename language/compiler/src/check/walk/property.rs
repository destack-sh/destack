use destack_dir as dir;
use destack_source::ModuleId;
use std::ptr::NonNull;

use crate::CompilerResult;
use crate::check::{
    FlowState, GenericInductionDeclaration, GenericInductionPosition, Origin, Receiver,
    ReceiverBinding, TypeOperand, TypeRelation, TypeTerm, WalkState,
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

    /// Walk one property.
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
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any(), None)? else {
            return Ok(());
        };

        match property {
            // { key: value }
            dir::Property::Field { key, value, .. } => {
                // compute runtime property key
                if let dir::Key::Expression(key) = key {
                    self.walk_expression(*key, self.tree.get(*key))?;
                }

                self.walk_expression(*value, self.tree.get(*value))?;
            }
            // { method() {} }
            dir::Property::Method {
                key,
                signature,
                body,
            } => {
                if let Some(key) = key {
                    // compute runtime property key
                    if let dir::Key::Expression(key) = key {
                        self.walk_expression(*key, self.tree.get(*key))?;
                    }
                }

                let symbol = self
                    .check
                    .module(self.module)
                    .declaration_symbol(id.into_any());

                // walk signature before reading its term inputs
                let source = id.into_global_any(self.module);
                let template = self.signature_template(source, None, symbol, signature)?;
                self.walk_function_signature(template, signature)?;
                let result =
                    self.function_result_operand(self.module, id.into_any(), signature, *body)?;

                // constrain method property symbol type
                if let Some(symbol) = symbol {
                    let term = self.function_signature_term(
                        signature,
                        template,
                        None,
                        result.map(Into::into),
                    )?;
                    let condition = self.active_static_guard();
                    self.constrain_symbol_type_term(symbol, TypeTerm::Function(term), condition)?;
                }

                // walk method body after its result operand exists
                if let (Some(symbol), Some(body), Some(result)) = (symbol, body, result) {
                    self.walk_function_body(symbol, signature, *body, result, None)?;
                }
            }
            // { ...value }
            dir::Property::Spread { value } => {
                self.walk_expression(*value, self.tree.get(*value))?;
            }
            // ignore damaged syntax
            dir::Property::Error => {}
        };

        Ok(())
    }

    /// Walk one member.
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
    ) -> CompilerResult<()> {
        let member_receiver = match member {
            dir::Member::StaticBlock { .. } | dir::Member::ComptimeBlock { .. } => None,
            dir::Member::Field { .. }
            | dir::Member::Method { .. }
            | dir::Member::AssociatedType { .. }
            | dir::Member::AssociatedConst { .. } => receiver_scope,
            dir::Member::Error => None,
        };

        let Some(_guard) = self.enter_decorated_static_guard(id.into_any(), member_receiver)?
        else {
            return Ok(());
        };
        let _receiver = self.enter_receiver_scope(member_receiver);

        match member {
            // type Item = T
            dir::Member::AssociatedType {
                generic_parameters,
                where_clauses,
                constraint,
                value,
                ..
            } => {
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
                    self.walk_where_clause(*where_clause, self.tree.get(*where_clause))?;
                }

                // walk associated type constraint
                if let Some(constraint) = constraint {
                    self.walk_type_expression(*constraint, self.tree.get(*constraint))?;
                }

                // walk associated type value
                if let Some(value) = value {
                    self.walk_type_expression(*value, self.tree.get(*value))?;
                }

                // constrain associated type value
                if let Some(value) = value
                    && let Some(symbol) = symbol
                {
                    let value = self.node_type_operand(*value)?;
                    let condition = self.active_static_guard();
                    let induction_declaration = GenericInductionDeclaration::new(source, parent, Some(symbol));
                    self.record_generic_induction_site(induction_declaration, value);
                    self.constrain_symbol_type(symbol, value, condition)?;
                }
            }
            // const item: T = value
            dir::Member::AssociatedConst {
                declared_type,
                value,
                ..
            } => {
                if value.is_some() && declared_type.is_none() {
                    self.check.report_missing_type_annotation(self.module, id.into_any());
                }

                // walk associated const type
                if let Some(declared_type) = declared_type {
                    self.walk_type_expression(*declared_type, self.tree.get(*declared_type))?;
                }

                // walk associated const value
                if let Some(value) = value {
                    self.walk_static_expression(*value)?;
                }

                if let Some(symbol) = self
                    .check
                    .module(self.module)
                    .declaration_symbol(id.into_any())
                {
                    // associated const type lives in type space
                    if let Some(declared_type) = declared_type {
                        let declared_type = self.node_type_operand(*declared_type)?;
                        let condition = self.active_static_guard();
                        self.constrain_symbol_type(symbol, declared_type, condition)?;
                    }

                    // associated const value lives in static space
                    if let Some(value) = value {
                        let condition = self.active_static_guard();
                        let symbol = self.symbol_static_operand(symbol)?;
                        let value = self.static_expression_operand(*value, condition.clone())?;
                        let origin = Origin::Node(id.into_global_any(self.module));
                        self.check.equate_static(origin, symbol, value, condition);
                    }
                }
            }
            // field: T = value
            dir::Member::Field {
                key,
                declared_type,
                default,
                is_optional,
                ..
            } => {
                if declared_type.is_none() {
                    self.check.report_missing_type_annotation(self.module, id.into_any());
                }

                // check computed member key in declaration context
                if let dir::Key::Expression(key) = key {
                    let before_key = self.fork_flow();
                    self.walk_expression(*key, self.tree.get(*key))?;
                    self.restore_flow(before_key);
                }

                if let Some(declared_type) = declared_type {
                    self.walk_type_expression(*declared_type, self.tree.get(*declared_type))?;
                }
                if let Some(default) = default {
                    // check field default in declaration context
                    let before_default = self.fork_flow();
                    self.walk_expression(*default, self.tree.get(*default))?;
                    self.restore_flow(before_default);
                }

                // derive declared field type
                let field_type = if let Some(declared_type) = declared_type {
                    let field_type = self.node_type_operand(*declared_type)?;
                    let condition = self.active_static_guard();
                    let source = id.into_global_any(self.module);
                    let field_type = self.induce_constraint_type_operand(
                        source,
                        field_type,
                        GenericInductionPosition::Storage,
                        condition.clone(),
                    )?;
                    let field_type = if *is_optional {
                        self.optional_value_type(field_type)
                    } else {
                        field_type
                    };

                    Some(field_type)
                } else {
                    None
                };

                // constrain direct field symbol type
                if let Some(field_type) = field_type
                    && key.direct_static_key().is_some()
                    && let Some(symbol) =
                        self.check.module(self.module).declaration_symbol(id.into_any())
                {
                    let condition = self.active_static_guard();

                    if let Some(induction_declaration) = induction_declaration {
                        self.record_generic_induction_site(induction_declaration, field_type);
                    }
                    self.constrain_symbol_type(symbol, field_type, condition)?;
                }

                // defaults must fit the declared field type
                if let (Some(field_type), Some(default)) = (field_type, default) {
                    let origin = Origin::Node((*default).into_global_any(self.module));
                    let value = self.node_type_operand(*default)?;
                    let condition = self.active_static_guard();
                    self.check.constrain_type(
                        origin,
                        TypeRelation::Assignable,
                        value,
                        field_type,
                        condition,
                    );
                }
            }
            // method() {}
            dir::Member::Method {
                key,
                signature,
                body,
                is_static,
                ..
            } => {
                if let Some(key) = key {
                    // check computed member key in declaration context
                    if let dir::Key::Expression(key) = key {
                        let before_key = self.fork_flow();
                        self.walk_expression(*key, self.tree.get(*key))?;
                        self.restore_flow(before_key);
                    }
                }

                let symbol = self.check.module(self.module).declaration_symbol(id.into_any());

                let receiver_owner = member_receiver.and_then(|scope| scope.owner);
                let parent = self.enclosing_generic_template(receiver_scope, induction_declaration);
                let source = id.into_global_any(self.module);
                let template = self.signature_template(source, parent, symbol, signature)?;
                let captured_template = template.or(parent);

                // walk signature before reading its term inputs
                self.walk_function_signature(template, signature)?;
                let implicit_receiver_scope = if *is_static { None } else { receiver_scope };
                let receiver =
                    self.method_receiver_binding(id, signature, receiver_owner, implicit_receiver_scope)?;
                let result = self.method_result_operand(
                    self.module,
                    id,
                    signature,
                    *body,
                    receiver,
                )?;

                let term = self.function_signature_term(
                    signature,
                    captured_template,
                    receiver
                        .filter(|_| Self::is_receiver_visible_in_method_type(signature))
                        .map(|receiver| receiver.receiver.ty),
                    result.map(Into::into),
                )?;
                let operand = self
                    .check
                    .inference
                    .push_term(TypeTerm::Function(term))
                    .into();
                let induction_declaration = GenericInductionDeclaration::new(source, parent, symbol);

                // collect generics induced by this method type
                self.record_generic_induction_site(induction_declaration, operand);

                // constrain direct method symbol type
                if let Some(symbol) = symbol {
                    let condition = self.active_static_guard();
                    self.constrain_symbol_type(symbol, operand, condition)?;
                }

                // walk method body after its result operand exists
                if let Some(body) = body && let Some(symbol) = symbol && let Some(result) = result {
                    self.walk_function_body(symbol, signature, *body, result, receiver)?;
                }
            }
            // static { ... }
            dir::Member::StaticBlock { body }
            // comptime { ... }
            | dir::Member::ComptimeBlock { body } => {
                // check member block in declaration context
                let before_body = self.fork_flow();
                self.walk_expression(*body, self.tree.get(*body))?;
                self.restore_flow(before_body);
            }
            // ignore damaged syntax
            dir::Member::Error => {}
        };

        Ok(())
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

    /// Return one method body or call signature result operand.
    ///
    /// Example:
    /// ```ds
    /// method(): number { 1 }
    /// ```
    fn method_result_operand(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::Member>,
        signature: &dir::FunctionSignature,
        body: Option<dir::LocalNodeId<dir::Expression>>,
        receiver: Option<ReceiverBinding>,
    ) -> CompilerResult<Option<TypeOperand>> {
        // use the receiver as the constructor result
        if matches!(
            signature.role,
            Some(dir::FunctionRole::Constructor | dir::FunctionRole::New)
        ) {
            return Ok(receiver.map(|receiver| receiver.receiver.ty));
        }

        // return regular method result
        self.function_result_operand(module, id.into_any(), signature, body)
    }
}
