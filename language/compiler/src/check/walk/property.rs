use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{Origin, ReceiverCapture, TypeOperand, TypeRelation, TypeTerm, WalkState};

/// Receiver type available to instance members of one declaration.
#[derive(Debug, Clone, Copy)]
pub(in crate::check) struct MemberReceiverContext {
    /// The module that owns the member declaration.
    pub(in crate::check) module: ModuleId,
    /// The nominal declaration that introduces the receiver, when any.
    pub(in crate::check) owner: Option<dir::GlobalSymbolId>,
    /// The receiver type.
    pub(in crate::check) ty: TypeOperand,
}

impl WalkState<'_, '_> {
    /// Walk one property.
    ///
    /// Example:
    /// ```ds
    /// { name: value, method() { value } }
    /// ```
    pub(in crate::check) fn walk_property(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Property>,
        property: &dir::Property,
    ) {
        if !self.push_static_guard_for(tree, id.into_any(), None) {
            return;
        }

        match property {
            // { key: value }
            dir::Property::Field { key, value, .. } => {
                // compute runtime property key
                if let dir::Key::Expression(key) = key {
                    self.walk_expression(tree, *key, tree.get(*key));
                }

                self.walk_expression(tree, *value, tree.get(*value));
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
                        self.walk_expression(tree, *key, tree.get(*key));
                    }
                }

                // walk signature before reading its term inputs
                self.walk_function_signature(tree, signature);

                let symbol = self
                    .check
                    .module(tree.module_id)
                    .declaration_symbol(id.into_any());
                let result =
                    self.function_result_operand(tree.module_id, id.into_any(), signature, *body);

                // commit method property symbol type
                if let Some(symbol) = symbol {
                    let term = self.lower_function_signature_term(
                        signature,
                        None,
                        result.map(Into::into),
                        tree,
                    );
                    let condition = self.active_static_guard();

                    self.bind_symbol_type(symbol, TypeTerm::Function(term), condition);
                }

                // walk method body after its result operand exists
                if let (Some(symbol), Some(body), Some(result)) = (symbol, body, result) {
                    self.walk_function_body(tree, symbol, signature, *body, result, None);
                }
            }
            // { ...value }
            dir::Property::Spread { value } => {
                self.walk_expression(tree, *value, tree.get(*value));
            }
            // ignore damaged syntax
            dir::Property::Error => {}
        };

        self.pop_static_guard();
    }

    /// Walk one member.
    ///
    /// Example:
    /// ```ds
    /// field: string = "value"
    /// ```
    pub(in crate::check) fn walk_member(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Member>,
        member: &dir::Member,
        receiver_context: Option<MemberReceiverContext>,
    ) {
        let static_receiver = self.member_static_guard_receiver(id, receiver_context);
        if !self.push_static_guard_for(tree, id.into_any(), static_receiver) {
            return;
        }

        match member {
            // type Item = T
            dir::Member::AssociatedType {
                generic_parameters,
                where_clauses,
                constraint,
                value,
                ..
            } => {
                // walk generic parameters
                for generic_parameter in generic_parameters {
                    self.walk_generic_parameter(
                        tree,
                        *generic_parameter,
                        tree.get(*generic_parameter),
                    );
                }

                // walk where clauses
                for where_clause in where_clauses {
                    self.walk_where_clause(tree, *where_clause, tree.get(*where_clause));
                }

                // walk associated type constraint
                if let Some(constraint) = constraint {
                    self.walk_type_expression(tree, *constraint, tree.get(*constraint));
                }

                // walk associated type value
                if let Some(value) = value {
                    self.walk_type_expression(tree, *value, tree.get(*value));
                }

                // commit associated type value
                if let Some(value) = value
                    && let Some(symbol) = self.check.module(tree.module_id).declaration_symbol(id.into_any())
                {
                    let value = self.allocate_node_type_operand(*value);
                    let condition = self.active_static_guard();

                    self.bind_symbol_type_operand(symbol, value, condition);
                }
            }
            // const item: T = value
            dir::Member::AssociatedConst {
                declared_type,
                value,
                ..
            } => {
                if value.is_some() && declared_type.is_none() {
                    self.check.report_missing_type_annotation(tree.module_id, id.into_any());
                }

                // walk associated const type
                if let Some(declared_type) = declared_type {
                    self.walk_type_expression(tree, *declared_type, tree.get(*declared_type));
                }

                // walk associated const value
                if let Some(value) = value {
                    self.walk_static_expression(tree, *value);
                }

                if let Some(symbol) = self.check.module(tree.module_id).declaration_symbol(id.into_any()) {
                    // associated const type lives in type space
                    if let Some(declared_type) = declared_type {
                        let declared_type =
                            self.allocate_node_type_operand(*declared_type);
                        let condition = self.active_static_guard();

                        self.bind_symbol_type_operand(
                            symbol,
                            declared_type,
                            condition,
                        );
                    }

                    // associated const value lives in static space
                    if let Some(value) = value {
                        let variable = self.allocate_symbol_static_variable(symbol);
                        let condition = self.active_static_guard();
                        let value = self.allocate_static_expression_variable(*value,
                            condition.clone(),
                        );
                        let origin = self.check.variable(variable).source;
                        self.check.equate_static(origin, variable, value, condition);
                    }
                }
            }
            // field: T = value
            dir::Member::Field {
                key,
                declared_type,
                default,
                ..
            } => {
                if declared_type.is_none() {
                    self.check.report_missing_type_annotation(tree.module_id, id.into_any());
                }

                // check computed member key in declaration context
                if let dir::Key::Expression(key) = key {
                    let before_key = self.fork_flow();

                    self.walk_expression(tree, *key, tree.get(*key));
                    self.restore_flow(before_key);
                }

                if let Some(declared_type) = declared_type {
                    self.walk_type_expression(tree, *declared_type, tree.get(*declared_type));
                }
                if let Some(default) = default {
                    // check field default in declaration context
                    let before_default = self.fork_flow();

                    self.walk_expression(tree, *default, tree.get(*default));
                    self.restore_flow(before_default);
                }

                if let Some(declared_type) = declared_type
                    && key.direct_static_key().is_some()
                    && let Some(symbol) =
                        self.check.module(tree.module_id).declaration_symbol(id.into_any())
                {
                    let declared_type =
                        self.allocate_node_type_operand(*declared_type);
                    let condition = self.active_static_guard();
                    let source = id.into_global_any(tree.module_id);
                    let declared_type =
                        self.induce_transparent_type_operand(source, declared_type, condition.clone());

                    if let Some(owner) = receiver_context.and_then(|receiver| receiver.owner) {
                        self.check.push_generic_induction_root(owner, declared_type);
                    }

                    self.bind_symbol_type_operand(symbol, declared_type, condition);
                }

                // defaults must fit the declared field type
                if let (Some(declared_type), Some(default)) = (declared_type, default) {
                    let origin = Origin::Node((*default).into_global_any(tree.module_id));
                    let value = self.allocate_node_type_operand(*default);
                    let declared_type =
                        self.allocate_node_type_operand(*declared_type);
                    let condition = self.active_static_guard();

                    self.check.relate_type(
                        origin,
                        TypeRelation::Assignable,
                        value,
                        declared_type,
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

                        self.walk_expression(tree, *key, tree.get(*key));
                        self.restore_flow(before_key);
                    }
                }

                let symbol = self.check.module(tree.module_id).declaration_symbol(id.into_any());

                // walk signature before reading its term inputs
                self.walk_function_signature(tree, signature);
                let receiver =
                    self.bind_member_receiver(tree, id, signature, *is_static, receiver_context);
                let result =
                    self.method_result_operand(tree.module_id, id, signature, *body, receiver);

                // bind method symbol type
                if let Some(symbol) = symbol {
                    let term = self.lower_function_signature_term(
                        signature,
                        receiver
                            .filter(|_| Self::is_receiver_visible_in_method_type(signature))
                            .map(|receiver| receiver.ty),
                        result.map(Into::into),
                        tree,
                    );
                    let condition = self.active_static_guard();

                    let operand = self.check.inference.push_term(TypeTerm::Function(term)).into();

                    if let Some(receiver) = receiver {
                        self.check.push_generic_induction_root(symbol, receiver.ty);
                    }
                    self.check.push_generic_induction_root(symbol, operand);
                    self.bind_symbol_type_operand(symbol, operand, condition);
                }

                // walk method body after its result operand exists
                if let Some(body) = body && let Some(symbol) = symbol && let Some(result) = result {
                    self.walk_function_body(tree, symbol, signature, *body, result, receiver);
                }
            }
            // static { ... }
            dir::Member::StaticBlock { body }
            // comptime { ... }
            | dir::Member::ComptimeBlock { body } => {
                // check member block in declaration context
                let before_body = self.fork_flow();

                self.walk_expression(tree, *body, tree.get(*body));
                self.restore_flow(before_body);
            }
            // ignore damaged syntax
            dir::Member::Error => {}
        };

        self.pop_static_guard();
    }

    /// Return the receiver visible to decorators on one member.
    fn member_static_guard_receiver(
        &self,
        id: dir::LocalNodeId<dir::Member>,
        receiver_context: Option<MemberReceiverContext>,
    ) -> Option<ReceiverCapture> {
        let context = receiver_context?;
        let symbol = self
            .check
            .module(context.module)
            .implicit_receiver_symbol(id.into_any())?;

        Some(ReceiverCapture {
            symbol,
            owner: context.owner,
            ty: context.ty,
        })
    }

    /// Bind the lexical receiver visible inside one method body.
    ///
    /// Example:
    /// ```ds
    /// method(this: Box): number { this.value }
    /// ```
    fn bind_member_receiver(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Member>,
        signature: &dir::FunctionSignature,
        is_static: bool,
        receiver_context: Option<MemberReceiverContext>,
    ) -> Option<ReceiverCapture> {
        // explicit `this` parameters win
        let owner = receiver_context.and_then(|context| context.owner);
        if let Some(receiver) = self.bind_explicit_receiver(signature.this_parameter, owner, tree) {
            return Some(receiver);
        }

        // skip implicit receiver for static methods
        if is_static {
            return None;
        }

        // bind the implicit instance receiver
        let Some(context) = receiver_context else {
            return None;
        };
        let Some(symbol) = self
            .check
            .module(tree.module_id)
            .implicit_receiver_symbol(id.into_any())
        else {
            return None;
        };
        if self
            .check
            .module(tree.module_id)
            .profile
            .flags
            .no_implicit_receivers
        {
            self.check
                .report_implicit_receiver(tree.module_id, id.into_any());
        }

        Some(ReceiverCapture {
            symbol,
            owner: context.owner,
            ty: context.ty,
        })
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
        receiver: Option<ReceiverCapture>,
    ) -> Option<TypeOperand> {
        // use the receiver as the constructor result
        if matches!(
            signature.role,
            Some(dir::FunctionRole::Constructor | dir::FunctionRole::New)
        ) {
            return receiver.map(|receiver| receiver.ty);
        }

        // return regular method result
        self.function_result_operand(module, id.into_any(), signature, body)
    }
}
