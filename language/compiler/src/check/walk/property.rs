use destack_dir as dir;
use dir::NodeVisitor as _;

use crate::check::{
    CheckModuleState, ConstraintOrigin, ReceiverCapture, StaticTerm, TypeRelation, TypeTerm,
    VariableId,
};

/// Receiver type available to instance members of one declaration.
#[derive(Debug, Clone, Copy)]
pub(in crate::check) struct MemberReceiverContext {
    /// The nominal declaration that introduces the receiver, when any.
    pub(in crate::check) owner: Option<dir::GlobalSymbolId>,
    /// The receiver type variable.
    pub(in crate::check) ty: VariableId,
}

impl CheckModuleState {
    /// Walk one property.
    pub(in crate::check) fn walk_property(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Property>,
        property: &dir::Property,
    ) {
        // apply static owner guards
        if !self.static_allows(tree, id.into_any()) {
            return;
        }

        self.visit_any(tree, dir::NodeType::Property, id.id);

        match property {
            // { key: value }
            dir::Property::Field { key, value, .. } => {
                self.walk_key(tree, key);
                self.walk_expression(tree, *value, tree.get(*value));
            }
            // { method() {} }
            dir::Property::Method {
                key,
                signature,
                body,
            } => {
                if let Some(key) = key {
                    self.walk_key(tree, key);
                }

                let symbol = self.declaration_symbol(id.into_any());
                let return_type =
                    self.intern_signature_return_type_variable(id.into_any(), signature, *body);
                if let Some(symbol) = symbol {
                    let variable = self.intern_symbol_type_variable(symbol);
                    let term = self.function_signature_term(signature, return_type, tree);

                    self.define_type_term(variable, TypeTerm::Function(term));
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
    }

    /// Walk one member.
    pub(in crate::check) fn walk_member(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Member>,
        member: &dir::Member,
        receiver_context: Option<MemberReceiverContext>,
    ) {
        // apply static owner guards
        if !self.static_allows(tree, id.into_any()) {
            return;
        }

        self.visit_any(tree, dir::NodeType::Member, id.id);

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
                    && let Some(symbol) = self.declaration_symbol(id.into_any())
                {
                    let variable = self.intern_symbol_type_variable(symbol);
                    let value = self.intern_local_type_variable(*value);
                    let term = TypeTerm::Variable(value);

                    self.define_type_term(variable, term);
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
                    self.report_missing_type_annotation(id.into_any());
                }

                if let Some(symbol) = self.declaration_symbol(id.into_any()) {
                    // associated const type lives in type space
                    if let Some(declared_type) = declared_type {
                        let variable = self.intern_symbol_type_variable(symbol);
                        let declared_type = self.intern_local_type_variable(*declared_type);

                        self.define_type_term(variable, TypeTerm::Variable(declared_type));
                    }

                    // associated const value lives in static space
                    if let Some(value) = value {
                        let variable = self.intern_symbol_static_variable(symbol);
                        let value = self.define_static_expression_variable(*value);

                        self.define_static_term(variable, StaticTerm::Variable(value));
                    }
                }

                // defaults must fit the declared type
                if let (Some(declared_type), Some(value)) = (declared_type, value) {
                    let origin =
                        ConstraintOrigin::Node((*value).into_global_any(self.input.module_id));
                    let value = self.intern_local_type_variable(*value);
                    let declared_type = self.intern_local_type_variable(*declared_type);

                    self.relate_type(origin, TypeRelation::Assignable, value, declared_type);
                }

                // type
                if let Some(declared_type) = declared_type {
                    self.walk_type_expression(tree, *declared_type, tree.get(*declared_type));
                }

                // value
                if let Some(value) = value {
                    self.walk_expression(tree, *value, tree.get(*value));
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
                    self.report_missing_type_annotation(id.into_any());
                }

                if let Some(declared_type) = declared_type {
                    if key.direct_static_key().is_some() {
                        if let Some(symbol) = self.declaration_symbol(id.into_any()) {
                            let variable = self.intern_symbol_type_variable(symbol);
                            let declared_type = self.intern_local_type_variable(*declared_type);

                            self.define_type_term(variable, TypeTerm::Variable(declared_type));
                        }
                    }
                }

                // defaults must fit the declared field type
                if let (Some(declared_type), Some(default)) = (declared_type, default) {
                    let origin =
                        ConstraintOrigin::Node((*default).into_global_any(self.input.module_id));
                    let value = self.intern_local_type_variable(*default);
                    let declared_type = self.intern_local_type_variable(*declared_type);

                    self.relate_type(origin, TypeRelation::Assignable, value, declared_type);
                }

                self.walk_key(tree, key);
                if let Some(declared_type) = declared_type {
                    self.walk_type_expression(tree, *declared_type, tree.get(*declared_type));
                }
                if let Some(default) = default {
                    self.walk_expression(tree, *default, tree.get(*default));
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
                    self.walk_key(tree, key);
                }

                let receiver =
                    self.member_receiver(tree, id, signature, *is_static, receiver_context);
                let symbol = self.declaration_symbol(id.into_any());
                if let Some(symbol) = symbol {
                    let variable = self.intern_symbol_type_variable(symbol);
                    let return_type = self.method_return_type(id, signature, *body, receiver);
                    let mut term = self.function_signature_term(signature, return_type, tree);
                    if term.this_parameter.is_none() && Self::method_type_uses_receiver(signature) {
                        term.this_parameter = receiver.map(|receiver| receiver.ty);
                    }

                    self.define_type_term(variable, TypeTerm::Function(term));
                }

                self.walk_function_signature(tree, signature);

                let Some(body) = body else {
                    return;
                };
                let Some(symbol) = symbol else {
                    return;
                };
                let Some(return_type) =
                    self.method_return_type(id, signature, Some(*body), receiver)
                else {
                    return;
                };

                self.walk_function_body(tree, symbol, signature, *body, return_type, receiver);
            }
            // static { ... }
            dir::Member::StaticBlock { body }
            // comptime { ... }
            | dir::Member::ComptimeBlock { body } => {
                self.walk_expression(tree, *body, tree.get(*body));
            }
            // ignore damaged syntax
            dir::Member::Error => {}
        };
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
        let Some(symbol) = self.implicit_receiver_symbol(id.into_any()) else {
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
        self.intern_signature_return_type_variable(id.into_any(), signature, body)
    }
}
