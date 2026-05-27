use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    CheckState, ConstraintOrigin, ReceiverCapture, StaticTerm, TypeRelation, TypeTerm, VariableId,
};

/// Receiver type available to instance members of one declaration.
#[derive(Debug, Clone, Copy)]
pub(in crate::check) struct MemberReceiverContext {
    /// The nominal declaration that introduces the receiver, when any.
    pub(in crate::check) owner: Option<dir::GlobalSymbolId>,
    /// The receiver type variable.
    pub(in crate::check) ty: VariableId,
}

impl CheckState<'_> {
    /// Walk one property.
    pub(in crate::check) fn walk_property(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Property>,
        property: &dir::Property,
    ) {
        if !self.push_static_condition_for(tree, id.into_any(), None) {
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

                let symbol = self.declaration_symbol(tree.module_id, id.into_any());
                let return_type = self.intern_signature_return_type_variable(
                    tree.module_id,
                    id.into_any(),
                    signature,
                    *body,
                );
                if let Some(symbol) = symbol {
                    let variable = self.intern_symbol_type_variable(tree.module_id, symbol);
                    let term = self.build_function_signature_term(signature, return_type, tree);

                    self.define_type(tree.module_id, variable, TypeTerm::Function(term));
                }

                self.walk_function_signature(tree, signature);

                if let (Some(symbol), Some(body), Some(return_type)) = (symbol, body, return_type) {
                    self.walk_function_body(tree, symbol, signature, *body, return_type, None);
                }
            }
            // { ...value }
            dir::Property::Spread { value } => {
                self.walk_expression(tree, *value, tree.get(*value));
            }
            // ignore damaged syntax
            dir::Property::Error => {}
        };

        self.pop_static_condition(tree.module_id);
    }

    /// Walk one member.
    pub(in crate::check) fn walk_member(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Member>,
        member: &dir::Member,
        receiver_context: Option<MemberReceiverContext>,
    ) {
        let static_receiver = self.member_static_receiver(id, receiver_context);
        if !self.push_static_condition_for(tree, id.into_any(), static_receiver) {
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
                // value
                if let Some(value) = value
                    && let Some(symbol) = self.declaration_symbol(tree.module_id, id.into_any())
                {
                    let variable = self.intern_symbol_type_variable(tree.module_id, symbol);
                    let value = self.intern_local_type_variable(tree.module_id, *value);
                    let term = TypeTerm::Variable(value);

                    self.define_type(tree.module_id, variable, term);
                }

                // generic parameters
                for generic_parameter in generic_parameters {
                    self.walk_generic_parameter(
                        tree,
                        *generic_parameter,
                        tree.get(*generic_parameter),
                    );
                }

                // where clauses
                for where_clause in where_clauses {
                    self.walk_where_clause(tree, *where_clause, tree.get(*where_clause));
                }

                // constraint
                if let Some(constraint) = constraint {
                    self.walk_type_expression(tree, *constraint, tree.get(*constraint));
                }

                // value
                if let Some(value) = value {
                    self.walk_type_expression(tree, *value, tree.get(*value));
                }
            }
            // const item: T = value
            dir::Member::AssociatedConst {
                declared_type,
                value,
                ..
            } => {
                if value.is_some() && declared_type.is_none() {
                    self.report_missing_type_annotation(tree.module_id, id.into_any());
                }

                if let Some(symbol) = self.declaration_symbol(tree.module_id, id.into_any()) {
                    // associated const type lives in type space
                    if let Some(declared_type) = declared_type {
                        let variable = self.intern_symbol_type_variable(tree.module_id, symbol);
                        let declared_type = self.intern_local_type_variable(tree.module_id, *declared_type);

                        self.define_type(tree.module_id, variable, TypeTerm::Variable(declared_type));
                    }

                    // associated const value lives in static space
                    if let Some(value) = value {
                        let variable = self.intern_symbol_static_variable(tree.module_id, symbol);
                        let value = self.define_static_expression_variable(tree.module_id, *value);

                        self.define_static(tree.module_id, variable, StaticTerm::Variable(value));
                    }
                }

                // type
                if let Some(declared_type) = declared_type {
                    self.walk_type_expression(tree, *declared_type, tree.get(*declared_type));
                }

                // value
                if let Some(value) = value {
                    self.walk_static_expression(tree, *value);
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
                    self.report_missing_type_annotation(tree.module_id, id.into_any());
                }

                if let Some(declared_type) = declared_type {
                    if key.direct_static_key().is_some() {
                        if let Some(symbol) = self.declaration_symbol(tree.module_id, id.into_any()) {
                            let variable = self.intern_symbol_type_variable(tree.module_id, symbol);
                            let declared_type = self.intern_local_type_variable(tree.module_id, *declared_type);

                            self.define_type(tree.module_id, variable, TypeTerm::Variable(declared_type));
                        }
                    }
                }

                // defaults must fit the declared field type
                if let (Some(declared_type), Some(default)) = (declared_type, default) {
                    let origin =
                        ConstraintOrigin::Node((*default).into_global_any(tree.module_id));
                    let value = self.intern_local_type_variable(tree.module_id, *default);
                    let declared_type = self.intern_local_type_variable(tree.module_id, *declared_type);

                    self.constrain_type(origin, TypeRelation::Assignable, value, declared_type);
                }

                // check computed member key in declaration context
                if let dir::Key::Expression(key) = key {
                    let before_key = self.checkpoint_flow(tree.module_id);

                    self.walk_expression(tree, *key, tree.get(*key));
                    self.restore_flow(tree.module_id, before_key);
                }

                if let Some(declared_type) = declared_type {
                    self.walk_type_expression(tree, *declared_type, tree.get(*declared_type));
                }
                if let Some(default) = default {
                    // check field default in declaration context
                    let before_default = self.checkpoint_flow(tree.module_id);

                    self.walk_expression(tree, *default, tree.get(*default));
                    self.restore_flow(tree.module_id, before_default);
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
                        let before_key = self.checkpoint_flow(tree.module_id);

                        self.walk_expression(tree, *key, tree.get(*key));
                        self.restore_flow(tree.module_id, before_key);
                    }
                }

                let receiver =
                    self.member_receiver(tree, id, signature, *is_static, receiver_context);
                let symbol = self.declaration_symbol(tree.module_id, id.into_any());
                if let Some(symbol) = symbol {
                    let variable = self.intern_symbol_type_variable(tree.module_id, symbol);
                    let return_type = self.method_return_type(tree.module_id, id, signature, *body, receiver);
                    let term = self.build_function_signature_term(signature, return_type, tree);
                    let function = self.terms.get_mut(term);
                    if function.this_parameter.is_none() && Self::method_type_uses_receiver(signature) {
                        function.this_parameter = receiver.map(|receiver| receiver.ty);
                    }

                    self.define_type(tree.module_id, variable, TypeTerm::Function(term));
                }

                self.walk_function_signature(tree, signature);

                // walk method body when bind provided the required symbols
                if let Some(body) = body
                    && let Some(symbol) = symbol
                    && let Some(return_type) =
                        self.method_return_type(tree.module_id, id, signature, Some(*body), receiver)
                {
                    self.walk_function_body(tree, symbol, signature, *body, return_type, receiver);
                }
            }
            // static { ... }
            dir::Member::StaticBlock { body }
            // comptime { ... }
            | dir::Member::ComptimeBlock { body } => {
                // check member block in declaration context
                let before_body = self.checkpoint_flow(tree.module_id);

                self.walk_expression(tree, *body, tree.get(*body));
                self.restore_flow(tree.module_id, before_body);
            }
            // ignore damaged syntax
            dir::Member::Error => {}
        };

        self.pop_static_condition(tree.module_id);
    }

    /// Return the receiver visible to decorators on one member.
    fn member_static_receiver(
        &self,
        id: dir::LocalNodeId<dir::Member>,
        receiver_context: Option<MemberReceiverContext>,
    ) -> Option<ReceiverCapture> {
        let context = receiver_context?;
        let symbol = self.implicit_receiver_symbol(context.ty.module, id.into_any())?;

        Some(ReceiverCapture {
            symbol,
            owner: context.owner,
            ty: context.ty,
        })
    }

    /// Return the lexical receiver visible inside one method body.
    fn member_receiver(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Member>,
        signature: &dir::FunctionSignature,
        is_static: bool,
        receiver_context: Option<MemberReceiverContext>,
    ) -> Option<ReceiverCapture> {
        // explicit `this` parameters win
        let owner = receiver_context.and_then(|context| context.owner);
        if let Some(receiver) = self.function_receiver(signature.this_parameter, owner, tree) {
            return Some(receiver);
        }

        // static methods do not receive instance `this`
        if is_static {
            return None;
        }

        // instance methods use the receiver introduced by bind
        let Some(context) = receiver_context else {
            return None;
        };
        let Some(symbol) = self.implicit_receiver_symbol(tree.module_id, id.into_any()) else {
            return None;
        };

        Some(ReceiverCapture {
            symbol,
            owner: context.owner,
            ty: context.ty,
        })
    }

    /// Return whether one method type should expose its receiver parameter.
    fn method_type_uses_receiver(signature: &dir::FunctionSignature) -> bool {
        !matches!(
            signature.role,
            Some(dir::FunctionRole::Constructor | dir::FunctionRole::New)
        )
    }

    /// Return the type produced by one method body or call signature.
    fn method_return_type(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::Member>,
        signature: &dir::FunctionSignature,
        body: Option<dir::LocalNodeId<dir::Expression>>,
        receiver: Option<ReceiverCapture>,
    ) -> Option<VariableId> {
        // constructors produce the receiver
        if matches!(
            signature.role,
            Some(dir::FunctionRole::Constructor | dir::FunctionRole::New)
        ) {
            return receiver.map(|receiver| receiver.ty);
        }

        // regular methods infer or use their declared return type
        self.intern_signature_return_type_variable(module, id.into_any(), signature, body)
    }
}
