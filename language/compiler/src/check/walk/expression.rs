use destack_dir as dir;

use crate::check::{
    AwaitTerm, CallCallee, CallTerm, ConstructTerm, FlowBranch, FlowCheckpoint, FormTerm,
    GenericArgument, IdentityTerm, ImportMetaTerm, IndexSetTerm, IndexTerm, InstanceCheckTerm,
    KeyMembershipTerm, MatchCase, MemberCallCallee, MemberCallTerm, MemberTerm, OperatorTerm,
    OperatorTermKind, Origin, PatternRelation, RangeValueTerm, ShapeMember, StaticTerm, SuperTerm,
    TaggedTemplateTerm, TemplateTerm, TreeTerm, TryTerm, TryTermKind, TupleElement,
    TypeLiteralTerm, TypeOperand, TypeOperationTerm, TypeRelation, TypeTerm, TypeValueTerm,
    VariableId, VariableKind, WalkState, YieldTerm,
};

/// The condition branch being entered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum ConditionBranch {
    /// The condition is known true.
    True,
    /// The condition is known false.
    False,
}

impl WalkState<'_, '_> {
    /// Walk one expression.
    ///
    /// Example:
    /// ```ds
    /// value.member<T>(argument)
    /// ```
    pub(in crate::check) fn walk_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        destack_core::ensure_sufficient_stack(|| {
            self.walk_expression_inner(tree, id, expression);
        });
    }

    /// Walk one expression after stack growth is handled.
    ///
    /// Example:
    /// ```ds
    /// value.member<T>(argument)
    /// ```
    fn walk_expression_inner(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        if !self.push_static_guard_for(tree, id.into_any(), None) {
            return;
        }
        match expression {
            // function f() {}
            dir::Expression::Declaration(declaration) => self.walk_declaration_expression(tree, id, *declaration),
            // { ... }
            dir::Expression::Block(block) => self.walk_block_expression(tree, id, *block),
            // label: body
            dir::Expression::Label { label, body } => self.walk_label_expression(tree, id, *label, *body),
            // import { item } from "module"
            dir::Expression::Import { items, .. } => self.walk_import_expression(tree, id, items.as_deref()),
            // export { item } from "module"
            dir::Expression::Export { items, .. } => self.walk_export_expression(tree, id, items),
            // let x = value
            dir::Expression::Let {
                declarators,
                is_ambient,
                ..
            } => self.walk_let_expression(tree, id, declarators, *is_ambient),
            // using x = value
            dir::Expression::Using { declarators, .. } => self.walk_using_expression(tree, id, declarators),
            // let x = value else fallback
            dir::Expression::LetElse {
                declarator,
                else_branch,
                ..
            } => self.walk_let_else_expression(tree, id, *declarator, *else_branch),
            // if condition { then } else { otherwise }
            dir::Expression::If {
                condition,
                then_expression,
                else_expression,
                ..
            } => self.walk_if_expression(tree, id, condition, *then_expression, *else_expression),
            // while condition { body }
            dir::Expression::While {
                condition, body, ..
            } => self.walk_while_expression(tree, id, None, *condition, *body),
            // for item of iterator { body }
            dir::Expression::ForEach {
                operator,
                binding,
                iterator,
                body,
                ..
            } => self.walk_for_each_expression(tree, id, None, *operator, binding, *iterator, *body),
            // for (initialization; condition; increment) { body }
            dir::Expression::For {
                initialization,
                condition,
                increment,
                body,
            } => self.walk_for_expression(tree, id, None, *initialization, *condition, *increment, *body),
            // loop { body }
            dir::Expression::Loop { body } => self.walk_loop_expression(tree, id, None, *body),
            // try body catch error finally cleanup
            dir::Expression::Try {
                body,
                catch,
                finally,
            } => self.walk_try_expression(tree, id, *body, *catch, *finally),
            // match value { case pattern => body }
            dir::Expression::Match { value, cases, .. } => self.walk_match_expression(tree, id, *value, cases),
            // break value
            dir::Expression::Break { label, value } => self.walk_break_expression(tree, id, *label, *value),
            // continue
            dir::Expression::Continue { label } => self.walk_continue_expression(tree, id, *label),
            // await value
            dir::Expression::Await {
                expression: awaited,
            } => self.walk_await_expression(tree, id, *awaited),
            // throw value
            dir::Expression::Throw { value } => self.walk_throw_expression(tree, id, *value),
            // return value
            dir::Expression::Return { value } => self.walk_return_expression(tree, id, *value),
            // yield value
            dir::Expression::Yield { cardinality, value } => self.walk_yield_expression(tree, id, *cardinality, *value),
            // value
            dir::Expression::Identifier { name } => self.walk_identifier_expression(tree, id, *name),
            // this
            dir::Expression::This => self.walk_this_expression(tree, id),
            // 1, "text", true
            dir::Expression::ScalarLiteral(value) => self.walk_scalar_literal_expression(tree, id, value),
            // super
            dir::Expression::Super => self.walk_super_expression(tree, id),
            // import.meta
            dir::Expression::ImportMeta => self.walk_import_meta_expression(tree, id),
            // #name
            dir::Expression::PrivateIdentifier { .. } => {}
            // debugger
            dir::Expression::Debugger => {}
            // missing expression
            dir::Expression::Missing => {}
            // stub expression
            dir::Expression::Stub => {}
            // ignore damaged syntax
            dir::Expression::Error => {}
            // namespace.value<T>
            dir::Expression::QualifiedReference {
                path,
                generic_arguments,
            } => self.walk_qualified_reference_expression(tree, id, path, generic_arguments),
            // start..end
            dir::Expression::RangeExpression {
                start,
                end,
                end_kind,
            } => self.walk_range_expression(tree, id, *start, *end, *end_kind),
            // `text ${value}`
            dir::Expression::TemplateExpression { value } => self.walk_template_expression(tree, id, value),
            // tag<T>`text ${value}`
            dir::Expression::TaggedTemplateExpression {
                tag,
                generic_arguments,
                value,
            } => self.walk_tagged_template_expression(tree, id, *tag, generic_arguments, value),
            // [a, b, c]
            dir::Expression::ArrayExpression { elements } => self.walk_array_expression(tree, id, elements),
            // [value; length]
            dir::Expression::FixedArrayExpression { value, length } => self.walk_fixed_array_expression(tree, id, *value, *length),
            // [a, label: b, ...rest]
            dir::Expression::TupleExpression { elements } => self.walk_tuple_expression(tree, id, elements),
            // a, b, c
            dir::Expression::SequenceExpression { expressions } => self.walk_sequence_expression(tree, id, expressions),
            // { key: value }
            dir::Expression::ObjectExpression { properties } => {
                self.walk_object_expression(tree, id, properties);
            }
            // Type { key: value }
            dir::Expression::StructExpression { ty, properties } => self.walk_struct_expression(tree, id, *ty, properties),
            // jsx like tree expression
            dir::Expression::TreeExpression {
                left,
                generic_arguments,
                arguments,
                elements,
            } => self.walk_tree_expression(tree, id, *left, generic_arguments, arguments.as_deref(), elements.as_deref()),
            // (value)
            dir::Expression::Parenthesized { expression: child } => self.walk_parenthesized_expression(tree, id, *child),
            // type T
            dir::Expression::Type { value } => self.walk_type_value_expression(tree, id, *value),
            // comptime value
            dir::Expression::Comptime { body } => self.walk_comptime_expression(tree, id, *body),
            // value as T
            dir::Expression::As {
                expression: child,
                target_type,
            } => self.walk_as_expression(tree, id, *child, *target_type),
            // value satisfies T
            dir::Expression::Satisfies {
                expression: child,
                target_type,
            } => self.walk_satisfies_expression(tree, id, *child, *target_type),
            // value is T
            dir::Expression::Is { value, target_type } => self.walk_is_expression(tree, id, *value, *target_type),
            // value instanceof Target
            dir::Expression::InstanceOf { value, target } => self.walk_instanceof_expression(tree, id, *value, *target),
            // !value, ++value
            dir::Expression::Unary { operator, right } => self.walk_unary_expression(tree, id, *operator, *right),
            // ^value
            dir::Expression::MoveOf { right, .. } => self.walk_move_of_expression(tree, id, *right),
            // &value
            dir::Expression::BorrowOf {
                mutability, right, ..
            } => self.walk_borrow_of_expression(tree, id, *mutability, *right),
            // value.member
            dir::Expression::Member { left, name }
            // value.#member
            | dir::Expression::PrivateMember { left, name } => self.walk_member_expression(tree, id, *left, *name),
            // value[index]
            dir::Expression::Index { left, index, .. } => self.walk_index_expression(tree, id, *left, *index),
            // value<T>
            dir::Expression::Instantiation {
                left,
                generic_arguments,
            } => self.walk_instantiation_expression(tree, id, *left, generic_arguments),
            // callee<T>(argument)
            dir::Expression::Call {
                left,
                generic_arguments,
                arguments,
                ..
            } => {
                self.walk_call_expression(tree, id, *left, generic_arguments, arguments);
            }
            // new Type<T>(argument)
            dir::Expression::New { ty, arguments } => {
                self.walk_construct_expression(tree, id, *ty, arguments);
            }
            // new? Type<T>(argument)
            dir::Expression::NewMaybe { ty, arguments } => {
                self.walk_maybe_construct_expression(tree, id, *ty, arguments);
            }
            // await? value
            dir::Expression::AwaitMaybe {
                expression: awaited,
            } => self.walk_await_maybe_expression(tree, id, *awaited),
            // await! value
            dir::Expression::AwaitMust {
                expression: awaited,
            } => self.walk_await_must_expression(tree, id, *awaited),
            // value?
            dir::Expression::Maybe { left, .. } => self.walk_maybe_expression(tree, id, *left),
            // value!
            dir::Expression::Must { left, .. } => self.walk_must_expression(tree, id, *left),
            // left + right
            dir::Expression::Binary {
                left,
                operator,
                right,
            } => self.walk_binary_expression(tree, id, *left, *operator, *right),
            // target = value
            dir::Expression::Assign {
                left,
                operator,
                right,
            } => self.walk_assign_expression(tree, id, *left, *operator, *right),
        }

        self.pop_static_guard();
    }

    /// Walk one where clause.
    ///
    /// Example:
    /// ```ds
    /// where T: Serializable
    /// ```
    pub(in crate::check) fn walk_where_clause(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::WhereClause>,
        where_clause: &dir::WhereClause,
    ) {
        // walk both constraint operands
        self.walk_type_expression(tree, where_clause.left, tree.get(where_clause.left));
        self.walk_type_expression(tree, where_clause.right, tree.get(where_clause.right));

        // constrain the left type by the right type
        let left = self
            .check
            .require_local_node_type(tree.module_id, where_clause.left);
        let right = self
            .check
            .require_local_node_type(tree.module_id, where_clause.right);
        let origin = Origin::Node(id.into_global_any(tree.module_id));
        let condition = self.active_static_guard();

        self.check
            .relate_type(origin, TypeRelation::Satisfies, left, right, condition);
    }

    /// Walk one import expression.
    ///
    /// Example:
    /// ```ds
    /// import { item } from "module"
    /// ```
    fn walk_import_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        items: Option<&[dir::LocalNodeId<dir::DependencyItem>]>,
    ) {
        self.output_node_type(tree.module_id, id, TypeTerm::Literal(TypeLiteralTerm::Void));

        // walk imported item bindings
        let Some(items) = items else {
            return;
        };
        for item in items {
            self.walk_dependency_item(tree, *item, tree.get(*item));
        }
    }

    /// Walk one export expression.
    ///
    /// Example:
    /// ```ds
    /// export { item } from "module"
    /// ```
    fn walk_export_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        items: &[dir::LocalNodeId<dir::DependencyItem>],
    ) {
        self.output_node_type(tree.module_id, id, TypeTerm::Literal(TypeLiteralTerm::Void));

        // walk exported item bindings
        for item in items {
            self.walk_dependency_item(tree, *item, tree.get(*item));
        }
    }

    /// Walk one let expression.
    ///
    /// Example:
    /// ```ds
    /// let value: number = 1
    /// ```
    fn walk_let_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        declarators: &[dir::LocalNodeId<dir::Declarator>],
        is_ambient: bool,
    ) {
        self.output_node_type(tree.module_id, id, TypeTerm::Literal(TypeLiteralTerm::Void));

        // walk and assign each declared binding
        for declarator in declarators {
            self.walk_declarator(tree, *declarator, tree.get(*declarator));
            if is_ambient {
                self.mark_bindings_assigned(tree, tree.get(*declarator).pattern.into_any());
            } else {
                self.mark_declarator_assigned(tree, tree.get(*declarator));
            }
        }
    }

    /// Walk one using expression.
    ///
    /// Example:
    /// ```ds
    /// using resource = open()
    /// ```
    fn walk_using_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        declarators: &[dir::LocalNodeId<dir::Declarator>],
    ) {
        self.output_node_type(tree.module_id, id, TypeTerm::Literal(TypeLiteralTerm::Void));

        // walk and assign each resource binding
        for declarator in declarators {
            self.walk_declarator(tree, *declarator, tree.get(*declarator));
            self.mark_declarator_assigned(tree, tree.get(*declarator));
        }
    }

    /// Walk one let else expression.
    ///
    /// Example:
    /// ```ds
    /// let Some(value) = option else return
    /// ```
    fn walk_let_else_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        declarator: dir::LocalNodeId<dir::Declarator>,
        else_branch: dir::LocalNodeId<dir::Expression>,
    ) {
        self.output_node_type(tree.module_id, id, TypeTerm::Literal(TypeLiteralTerm::Void));
        self.walk_declarator(tree, declarator, tree.get(declarator));

        // check else branch without leaking flow
        let before_else = self.checkpoint_flow();
        self.walk_expression(tree, else_branch, tree.get(else_branch));
        if self.can_expression_fall_through(tree, else_branch) {
            self.check.report_invalid_control_flow(
                tree.module_id,
                else_branch.into_any(),
                "let else fallback must not fall through",
            );
        }
        self.restore_flow(before_else);

        // enter successful pattern flow
        self.mark_declarator_assigned(tree, tree.get(declarator));
        self.narrow_declarator_pattern_success(tree, declarator);
    }

    /// Walk one break expression.
    ///
    /// Example:
    /// ```ds
    /// break value
    /// ```
    fn walk_break_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        label: Option<dir::StringId>,
        value: Option<dir::LocalNodeId<dir::Expression>>,
    ) {
        self.output_node_type(
            tree.module_id,
            id,
            TypeTerm::Literal(TypeLiteralTerm::Never),
        );

        // walk optional break value
        let value = if let Some(value) = value {
            self.walk_expression(tree, value, tree.get(value));

            Some(self.check.require_local_node_type(tree.module_id, value))
        } else {
            None
        };

        self.record_break(tree.module_id, id.into_any(), label, value);
    }

    /// Walk one continue expression.
    ///
    /// Example:
    /// ```ds
    /// continue
    /// ```
    fn walk_continue_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        label: Option<dir::StringId>,
    ) {
        self.output_node_type(
            tree.module_id,
            id,
            TypeTerm::Literal(TypeLiteralTerm::Never),
        );
        self.record_continue(tree.module_id, id.into_any(), label);
    }

    /// Walk one await expression.
    ///
    /// Example:
    /// ```ds
    /// await task
    /// ```
    fn walk_await_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        awaited: dir::LocalNodeId<dir::Expression>,
    ) {
        self.validate_await_context(tree.module_id, id.into_any());
        self.walk_expression(tree, awaited, tree.get(awaited));

        let value = self.check.require_local_node_type(tree.module_id, awaited);
        let await_term = self.check.inference.terms.push(AwaitTerm {
            source: id.into_global_any(tree.module_id),
            value: value.into(),
        });
        let term = TypeTerm::Await(await_term);

        self.output_node_type(tree.module_id, id, term);
    }

    /// Walk one throw expression.
    ///
    /// Example:
    /// ```ds
    /// throw error
    /// ```
    fn walk_throw_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
    ) {
        self.output_node_type(
            tree.module_id,
            id,
            TypeTerm::Literal(TypeLiteralTerm::Never),
        );
        self.walk_expression(tree, value, tree.get(value));
    }

    /// Walk one return expression.
    ///
    /// Example:
    /// ```ds
    /// return value
    /// ```
    fn walk_return_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        value: Option<dir::LocalNodeId<dir::Expression>>,
    ) {
        self.output_node_type(
            tree.module_id,
            id,
            TypeTerm::Literal(TypeLiteralTerm::Never),
        );

        // constrain explicit return value
        if let Some(value) = value {
            self.walk_expression(tree, value, tree.get(value));

            let value = self.check.require_local_node_type(tree.module_id, value);
            self.constrain_return_value(tree.module_id, id.into_any(), value);
        }
        // constrain implicit void return
        else {
            self.constrain_void_return(tree.module_id, id.into_any());
        }
    }

    /// Walk one yield expression.
    ///
    /// Example:
    /// ```ds
    /// yield value
    /// ```
    fn walk_yield_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        cardinality: dir::YieldCardinality,
        value: Option<dir::LocalNodeId<dir::Expression>>,
    ) {
        if let Some(value) = value {
            self.walk_expression(tree, value, tree.get(value));
        }

        let source = id.into_global_any(tree.module_id);
        let origin = Origin::Node(source);
        let value_type =
            value.map(|value| self.check.require_local_node_type(tree.module_id, value));
        let delegate_return_type = if cardinality == dir::YieldCardinality::Generator {
            Some(
                self.check
                    .allocate_variable(tree.module_id, VariableKind::Type, origin),
            )
        } else {
            None
        };
        let yield_term = self.check.inference.terms.push(YieldTerm {
            source,
            value: value_type,
            yield_type: self.current_yield_type(),
            resume_type: self.current_resume_type(),
            delegate_return_type,
            cardinality,
        });
        let term = TypeTerm::Yield(yield_term);

        self.output_node_type(tree.module_id, id, term);
        self.constrain_yield_value(
            tree.module_id,
            id.into_any(),
            cardinality,
            value_type,
            delegate_return_type,
        );
    }

    /// Walk one identifier expression.
    ///
    /// Example:
    /// ```ds
    /// value
    /// ```
    fn walk_identifier_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        name: dir::StringId,
    ) {
        if let Some(term) = self.bind_identifier_reference_term(id, name, tree) {
            self.output_node_type(tree.module_id, id, term);
        }
    }

    /// Walk one this expression.
    ///
    /// Example:
    /// ```ds
    /// this
    /// ```
    fn walk_this_expression(&mut self, tree: &dir::Tree, id: dir::LocalNodeId<dir::Expression>) {
        if let Some(term) = self.lower_this_receiver_type_term(id.into_global_any(tree.module_id)) {
            self.output_node_type(tree.module_id, id, term);
        }
    }

    /// Walk one scalar literal expression.
    ///
    /// Example:
    /// ```ds
    /// 1
    /// ```
    fn walk_scalar_literal_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        value: &dir::ScalarLiteral,
    ) {
        let term = match value {
            // /pattern/
            dir::ScalarLiteral::RegexString { .. } => {
                let symbol = self
                    .check
                    .language_symbol(tree.module_id, dir::LanguageItem::RegExp);

                TypeTerm::Reference {
                    origin: Origin::Node(id.into_global_any(tree.module_id)),
                    symbol,
                    arguments: Vec::new().into(),
                }
            }
            // scalar literal
            _ => TypeTerm::Literal(TypeLiteralTerm::Scalar(value.clone())),
        };

        self.output_node_type(tree.module_id, id, term);
    }

    /// Walk one super expression.
    ///
    /// Example:
    /// ```ds
    /// super
    /// ```
    fn walk_super_expression(&mut self, tree: &dir::Tree, id: dir::LocalNodeId<dir::Expression>) {
        let source = id.into_global_any(tree.module_id);
        let receiver = self
            .resolve_this_receiver(source)
            .map(|receiver| receiver.ty);
        let super_term = self
            .check
            .inference
            .terms
            .push(SuperTerm { source, receiver });
        let term = TypeTerm::Super(super_term);

        self.output_node_type(tree.module_id, id, term);
    }

    /// Walk one import meta expression.
    ///
    /// Example:
    /// ```ds
    /// import.meta
    /// ```
    fn walk_import_meta_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
    ) {
        let import_meta = self.check.inference.terms.push(ImportMetaTerm {
            source: id.into_global_any(tree.module_id),
        });
        let term = TypeTerm::ImportMeta(import_meta);

        self.output_node_type(tree.module_id, id, term);
    }

    /// Walk one qualified reference expression.
    ///
    /// Example:
    /// ```ds
    /// namespace.value<T>
    /// ```
    fn walk_qualified_reference_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        path: &dir::Path,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) {
        if let Some(term) = self.bind_qualified_reference_term(id, path, generic_arguments, tree) {
            self.output_node_type(tree.module_id, id, term);
        }
    }

    /// Walk one range expression.
    ///
    /// Example:
    /// ```ds
    /// start..end
    /// ```
    fn walk_range_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        start: Option<dir::LocalNodeId<dir::Expression>>,
        end: Option<dir::LocalNodeId<dir::Expression>>,
        end_kind: dir::RangeEnd,
    ) {
        // walk range boundaries
        if let Some(start) = start {
            self.walk_expression(tree, start, tree.get(start));
        }
        if let Some(end) = end {
            self.walk_expression(tree, end, tree.get(end));
        }

        let start_variable =
            start.map(|start| self.check.require_local_node_type(tree.module_id, start));
        let end_variable = end.map(|end| self.check.require_local_node_type(tree.module_id, end));
        let range = self.check.inference.terms.push(RangeValueTerm {
            source: id.into_global_any(tree.module_id),
            start: start_variable,
            end: end_variable,
            end_kind,
        });
        let term = TypeTerm::RangeValue(range);

        self.output_node_type(tree.module_id, id, term);
    }

    /// Walk one template expression.
    ///
    /// Example:
    /// ```ds
    /// `hello ${name}`
    /// ```
    fn walk_template_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        value: &dir::TemplateLiteral,
    ) {
        self.walk_template_literal(tree, value);

        let (strings, spans) = self.lower_template_segments(value, tree);
        let template = self.check.inference.terms.push(TemplateTerm {
            source: id.into_global_any(tree.module_id),
            strings,
            spans,
        });
        let term = TypeTerm::Template(template);

        self.output_node_type(tree.module_id, id, term);
    }

    /// Walk one tagged template expression.
    ///
    /// Example:
    /// ```ds
    /// tag<T>`hello ${name}`
    /// ```
    fn walk_tagged_template_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        tag: dir::LocalNodeId<dir::Expression>,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        value: &dir::TemplateLiteral,
    ) {
        self.walk_expression(tree, tag, tree.get(tag));

        self.walk_template_literal(tree, value);

        let tag_source = tag.into_global_any(tree.module_id);
        let generic_owner = self.check.selected_name(tag_source);
        let (strings, spans) = self.lower_template_segments(value, tree);
        let generic_arguments_term =
            self.lower_generic_arguments(generic_owner, generic_arguments, tree);
        let tag_variable = self.check.require_local_node_type(tree.module_id, tag);
        let template = self.check.inference.terms.push(TaggedTemplateTerm {
            source: id.into_global_any(tree.module_id),
            tag: tag_variable,
            generic_arguments: generic_arguments_term,
            strings,
            spans,
        });
        let term = TypeTerm::TaggedTemplate(template);

        self.output_node_type(tree.module_id, id, term);
    }

    /// Walk one array expression.
    ///
    /// Example:
    /// ```ds
    /// [a, b, c]
    /// ```
    fn walk_array_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        elements: &[dir::LocalNodeId<dir::Argument>],
    ) {
        // walk array elements
        for element in elements {
            self.walk_argument(tree, *element, tree.get(*element));
        }

        let element_types = self.lower_argument_value_type_operands(elements, tree);
        let operation = self
            .check
            .inference
            .terms
            .push(TypeOperationTerm::BestCommon {
                elements: element_types.into_iter().map(TypeOperand::from).collect(),
            });
        let element = self
            .check
            .inference
            .terms
            .push(TypeTerm::Operation(operation));
        let term = TypeTerm::Array {
            element: element.into(),
        };

        self.output_node_type(tree.module_id, id, term);
    }

    /// Walk one fixed array expression.
    ///
    /// Example:
    /// ```ds
    /// [value; 4]
    /// ```
    fn walk_fixed_array_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
        length: dir::LocalNodeId<dir::Expression>,
    ) {
        self.walk_expression(tree, value, tree.get(value));
        self.walk_expression(tree, length, tree.get(length));

        let condition = self.active_static_guard();
        let length_variable =
            self.check
                .output_static_expression_variable(tree.module_id, length, condition);
        let term = TypeTerm::FixedArray {
            element: self
                .check
                .require_local_node_type(tree.module_id, value)
                .into(),
            length: length_variable.into(),
        };

        self.output_node_type(tree.module_id, id, term);
    }

    /// Walk one tuple expression.
    ///
    /// Example:
    /// ```ds
    /// [name: value, ...rest]
    /// ```
    fn walk_tuple_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        elements: &[dir::LocalNodeId<dir::Argument>],
    ) {
        // walk tuple elements
        for element in elements {
            self.walk_argument(tree, *element, tree.get(*element));
        }

        let elements_term = self.lower_tuple_elements(tree, elements);
        let term = TypeTerm::Tuple {
            form: dir::TupleForm::Tuple,
            elements: elements_term.into(),
        };

        self.output_node_type(tree.module_id, id, term);
    }

    /// Walk one sequence expression.
    ///
    /// Example:
    /// ```ds
    /// a, b, c
    /// ```
    fn walk_sequence_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expressions: &[dir::LocalNodeId<dir::Expression>],
    ) {
        // walk sequence operands
        for expression in expressions {
            self.walk_expression(tree, *expression, tree.get(*expression));
        }

        match expressions.last() {
            Some(expression) => {
                let operand = self
                    .check
                    .require_local_node_type(tree.module_id, *expression);

                self.output_node_type_operand(tree.module_id, id, operand);
            }
            None => {
                self.output_node_type(tree.module_id, id, TypeTerm::Literal(TypeLiteralTerm::Void))
            }
        }
    }

    /// Walk one struct expression.
    ///
    /// Example:
    /// ```ds
    /// Point { x: 1, y: 2 }
    /// ```
    fn walk_struct_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        properties: &[dir::LocalNodeId<dir::Property>],
    ) {
        self.walk_type_expression(tree, ty, tree.get(ty));

        // walk property values
        for property in properties {
            self.walk_property(tree, *property, tree.get(*property));
        }

        let source = id.into_global_any(tree.module_id);
        let owner = self.check.require_local_node_type(tree.module_id, ty);

        // constrain field values against owner members
        for property in properties {
            if let dir::Property::Field { key, value, .. } = tree.get(*property)
                && let Some(key) = key.static_key(tree)
            {
                let member = self.check.inference.terms.push(MemberTerm {
                    origin: Origin::Node(source),
                    owner,
                    key,
                    arguments: Vec::new().into(),
                });
                let field = TypeTerm::Member(member);
                let field = self.check.inference.terms.push(field);
                let value = self.check.require_local_node_type(tree.module_id, *value);
                let condition = self.active_static_guard();

                self.check.relate_type(
                    Origin::Node(source),
                    TypeRelation::Assignable,
                    value,
                    field,
                    condition,
                );
            }
        }

        self.output_node_type_operand(tree.module_id, id, owner);
    }

    /// Walk one tree expression.
    ///
    /// Example:
    /// ```ds
    /// View(title = "hello") { Text("world") }
    /// ```
    fn walk_tree_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        left: Option<dir::LocalNodeId<dir::Expression>>,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        arguments: Option<&[dir::LocalNodeId<dir::Argument>]>,
        elements: Option<&[dir::LocalNodeId<dir::Argument>]>,
    ) {
        // walk tree tag and inputs
        if let Some(left) = left {
            self.walk_expression(tree, left, tree.get(left));
        }
        if let Some(arguments) = arguments {
            for argument in arguments {
                self.walk_argument(tree, *argument, tree.get(*argument));
            }
        }
        if let Some(elements) = elements {
            for element in elements {
                self.walk_argument(tree, *element, tree.get(*element));
            }
        }

        let generic_owner = left.and_then(|left| {
            self.check
                .selected_name(left.into_global_any(tree.module_id))
        });
        let tag = left.map(|left| self.check.require_local_node_type(tree.module_id, left));
        let generic_argument_terms =
            self.lower_generic_arguments(generic_owner, generic_arguments, tree);
        let argument_types = arguments
            .map(|arguments| self.lower_argument_value_type_operands(arguments, tree))
            .unwrap_or_default();
        let element_types = elements
            .map(|elements| self.lower_argument_value_type_operands(elements, tree))
            .unwrap_or_default();
        let tree_term = self.check.inference.terms.push(TreeTerm {
            source: id.into_global_any(tree.module_id),
            tag,
            generic_arguments: generic_argument_terms,
            arguments: argument_types,
            elements: element_types,
        });
        let term = TypeTerm::Tree(tree_term);

        self.output_node_type(tree.module_id, id, term);
    }

    /// Walk one parenthesized expression.
    ///
    /// Example:
    /// ```ds
    /// (value)
    /// ```
    fn walk_parenthesized_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        child: dir::LocalNodeId<dir::Expression>,
    ) {
        self.walk_expression(tree, child, tree.get(child));

        let child_type = self.check.require_local_node_type(tree.module_id, child);

        self.output_node_type_operand(tree.module_id, id, child_type);
    }

    /// Walk one type value expression.
    ///
    /// Example:
    /// ```ds
    /// type T
    /// ```
    fn walk_type_value_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::TypeExpression>,
    ) {
        self.walk_type_expression(tree, value, tree.get(value));

        let ty = self.check.require_local_node_type(tree.module_id, value);
        let type_value = self.check.inference.terms.push(TypeValueTerm {
            source: id.into_global_any(tree.module_id),
            ty,
        });
        let term = TypeTerm::TypeValue(type_value);

        self.output_node_type(tree.module_id, id, term);
    }

    /// Walk one comptime expression.
    ///
    /// Example:
    /// ```ds
    /// comptime value
    /// ```
    fn walk_comptime_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Expression>,
    ) {
        self.walk_expression(tree, body, tree.get(body));

        let body_type = self.check.require_local_node_type(tree.module_id, body);

        self.output_node_type_operand(tree.module_id, id, body_type);
    }

    /// Walk one cast expression.
    ///
    /// Example:
    /// ```ds
    /// value as T
    /// ```
    fn walk_as_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        child: dir::LocalNodeId<dir::Expression>,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
    ) {
        // lower const assertion without contextual widening
        if matches!(tree.get(target_type), dir::TypeExpression::Const) {
            self.walk_const_assertion_expression(tree, id, child);

            return;
        }

        self.walk_expression(tree, child, tree.get(child));
        self.walk_type_expression(tree, target_type, tree.get(target_type));

        let child_type = self.check.require_local_node_type(tree.module_id, child);
        let target_type = self
            .check
            .require_local_node_type(tree.module_id, target_type);
        let origin = Origin::Node(id.into_global_any(tree.module_id));
        let condition = self.active_static_guard();

        self.output_node_type_operand(tree.module_id, id, target_type);
        self.check.relate_type(
            origin,
            TypeRelation::Castable,
            child_type,
            target_type,
            condition,
        );
    }

    /// Walk one satisfies expression.
    ///
    /// Example:
    /// ```ds
    /// value satisfies T
    /// ```
    fn walk_satisfies_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        child: dir::LocalNodeId<dir::Expression>,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
    ) {
        self.walk_expression(tree, child, tree.get(child));
        self.walk_type_expression(tree, target_type, tree.get(target_type));

        let child_type = self.check.require_local_node_type(tree.module_id, child);
        let target_type = self
            .check
            .require_local_node_type(tree.module_id, target_type);
        let origin = Origin::Node(id.into_global_any(tree.module_id));
        let condition = self.active_static_guard();

        self.output_node_type_operand(tree.module_id, id, child_type);
        self.check.relate_type(
            origin,
            TypeRelation::Satisfies,
            child_type,
            target_type,
            condition,
        );
    }

    /// Walk one is expression.
    ///
    /// Example:
    /// ```ds
    /// value is T
    /// ```
    fn walk_is_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
    ) {
        self.output_node_type(
            tree.module_id,
            id,
            TypeTerm::Literal(TypeLiteralTerm::boolean()),
        );
        self.walk_expression(tree, value, tree.get(value));
        self.walk_type_expression(tree, target_type, tree.get(target_type));
    }

    /// Walk one instanceof expression.
    ///
    /// Example:
    /// ```ds
    /// value instanceof Target
    /// ```
    fn walk_instanceof_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
        target: dir::LocalNodeId<dir::Expression>,
    ) {
        self.walk_expression(tree, value, tree.get(value));
        self.walk_expression(tree, target, tree.get(target));

        let value_variable = self.check.require_local_node_type(tree.module_id, value);
        let target_variable = self.check.require_local_node_type(tree.module_id, target);
        let instance = self.check.inference.terms.push(InstanceCheckTerm {
            source: id.into_global_any(tree.module_id),
            value: value_variable,
            target: target_variable,
        });
        let term = TypeTerm::InstanceCheck(instance);

        self.output_node_type(tree.module_id, id, term);
    }

    /// Walk one unary expression.
    ///
    /// Example:
    /// ```ds
    /// !value
    /// ```
    fn walk_unary_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        operator: dir::UnaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) {
        self.walk_expression(tree, right, tree.get(right));

        let is_update = matches!(
            operator,
            dir::UnaryOperator::PostIncrement
                | dir::UnaryOperator::PostDecrement
                | dir::UnaryOperator::PreIncrement
                | dir::UnaryOperator::PreDecrement
        );
        let receiver = self.check.require_local_node_type(tree.module_id, right);
        let operator = self.check.inference.terms.push(OperatorTerm {
            source: id.into_global_any(tree.module_id),
            kind: OperatorTermKind::Unary(operator),
            receiver,
            argument: None,
        });
        let term = TypeTerm::Operator(operator);

        self.output_node_type(tree.module_id, id, term);

        // write after reading the updated place
        if is_update && let Some(place) = self.lower_place(right, tree) {
            let condition = self.active_static_guard();

            self.clear_mutated_expression_narrowings(tree, right);
            self.mark_place_assigned(place);
            self.check.require_writable_place(place, condition);
        }
    }

    /// Walk one move expression.
    ///
    /// Example:
    /// ```ds
    /// ^value
    /// ```
    fn walk_move_of_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        right: dir::LocalNodeId<dir::Expression>,
    ) {
        self.walk_expression(tree, right, tree.get(right));

        let form = self.check.inference.terms.push(FormTerm::Owned);
        let term = TypeTerm::Form {
            form,
            payload: self
                .check
                .require_local_node_type(tree.module_id, right)
                .into(),
        };

        self.output_node_type(tree.module_id, id, term);
    }

    /// Walk one borrow expression.
    ///
    /// Example:
    /// ```ds
    /// &mut value
    /// ```
    fn walk_borrow_of_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        mutability: Option<dir::Mutability>,
        right: dir::LocalNodeId<dir::Expression>,
    ) {
        self.walk_expression(tree, right, tree.get(right));

        let origin = Origin::Node(id.into_global_any(tree.module_id));
        let access = mutability
            .map(dir::Mutability::access)
            .unwrap_or(dir::Access::Mutable);
        let access = dir::StaticTerm::Access { access };
        let access = self.check.inference.terms.push(StaticTerm::Literal(access));
        let lifetime = self
            .check
            .allocate_variable(tree.module_id, VariableKind::Static, origin);
        let form = self.check.inference.terms.push(FormTerm::Borrowed {
            lifetime: lifetime.into(),
            access: access.into(),
        });
        let term = TypeTerm::Form {
            form,
            payload: self
                .check
                .require_local_node_type(tree.module_id, right)
                .into(),
        };

        self.output_node_type(tree.module_id, id, term);
    }

    /// Walk one member expression.
    ///
    /// Example:
    /// ```ds
    /// value.member
    /// ```
    fn walk_member_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        name: Option<dir::StringId>,
    ) {
        self.walk_expression(tree, left, tree.get(left));

        // set member access when the key is present
        if let Some(name) = name {
            let term = if let Some(narrowed) = self.flow_path_narrowing(tree, id) {
                narrowed.to_type_term(self.check)
            } else {
                let owner = self.check.require_local_node_type(tree.module_id, left);
                let member = self.check.inference.terms.push(MemberTerm {
                    origin: Origin::Node(id.into_global_any(tree.module_id)),
                    owner,
                    key: dir::StaticKey::Name(name),
                    arguments: Vec::new().into(),
                });

                TypeTerm::Member(member)
            };

            self.output_node_type(tree.module_id, id, term);
        }
    }

    /// Walk one index expression.
    ///
    /// Example:
    /// ```ds
    /// value[index]
    /// ```
    fn walk_index_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        index: Option<dir::LocalNodeId<dir::Expression>>,
    ) {
        self.walk_expression(tree, left, tree.get(left));

        // walk index operand
        if let Some(index) = index {
            self.walk_expression(tree, index, tree.get(index));
        }

        // set index access when the index is present
        if let Some(index) = index {
            let term = if let Some(narrowed) = self.flow_path_narrowing(tree, id) {
                narrowed.to_type_term(self.check)
            } else {
                let receiver = self.check.require_local_node_type(tree.module_id, left);
                let index_type = self.check.require_local_node_type(tree.module_id, index);
                let index_term = self.check.inference.terms.push(IndexTerm {
                    source: id.into_global_any(tree.module_id),
                    receiver,
                    index: index_type,
                    key: tree.get(index).static_key(),
                });

                TypeTerm::Index(index_term)
            };

            self.output_node_type(tree.module_id, id, term);
        }
    }

    /// Walk one generic instantiation expression.
    ///
    /// Example:
    /// ```ds
    /// value<T>
    /// ```
    fn walk_instantiation_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) {
        let source = id.into_global_any(tree.module_id);
        let left_source = left.into_global_any(tree.module_id);

        self.walk_expression(tree, left, tree.get(left));

        let generic_owner = self.check.selected_name(left_source);
        let arguments = self.lower_generic_arguments(generic_owner, generic_arguments, tree);
        if let Some(symbol) = generic_owner {
            let term = TypeTerm::Reference {
                origin: Origin::Node(source),
                symbol,
                arguments,
            };

            self.output_node_type(tree.module_id, id, term);
        } else {
            let operand = self.check.require_local_node_type(tree.module_id, left);

            self.output_node_type_operand(tree.module_id, id, operand);
        }
    }

    /// Walk one maybe await expression.
    ///
    /// Example:
    /// ```ds
    /// await? task
    /// ```
    fn walk_await_maybe_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        awaited: dir::LocalNodeId<dir::Expression>,
    ) {
        self.validate_await_context(tree.module_id, id.into_any());
        self.walk_expression(tree, awaited, tree.get(awaited));

        let source = id.into_global_any(tree.module_id);
        let value = self.check.require_local_node_type(tree.module_id, awaited);
        let await_term = self.check.inference.terms.push(AwaitTerm {
            source,
            value: value.into(),
        });
        let awaited_type = self.check.inference.terms.push(TypeTerm::Await(await_term));
        let term = TryTerm {
            source,
            value: awaited_type.into(),
            kind: TryTermKind::Maybe,
        };
        let tried = self.check.inference.terms.push(term.clone());

        self.output_node_type(tree.module_id, id, TypeTerm::Try(tried));
        self.propagate_try(tree.module_id, id.into_any(), term.value);
    }

    /// Walk one must await expression.
    ///
    /// Example:
    /// ```ds
    /// await! task
    /// ```
    fn walk_await_must_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        awaited: dir::LocalNodeId<dir::Expression>,
    ) {
        self.validate_await_context(tree.module_id, id.into_any());
        self.walk_expression(tree, awaited, tree.get(awaited));

        let source = id.into_global_any(tree.module_id);
        let value = self.check.require_local_node_type(tree.module_id, awaited);
        let await_term = self.check.inference.terms.push(AwaitTerm {
            source,
            value: value.into(),
        });
        let awaited_type = self.check.inference.terms.push(TypeTerm::Await(await_term));
        let term = TryTerm {
            source,
            value: awaited_type.into(),
            kind: TryTermKind::Must,
        };
        let term = self.check.inference.terms.push(term);

        self.output_node_type(tree.module_id, id, TypeTerm::Try(term));
    }

    /// Walk one maybe propagation expression.
    ///
    /// Example:
    /// ```ds
    /// value?
    /// ```
    fn walk_maybe_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
    ) {
        self.walk_expression(tree, left, tree.get(left));

        let tried = TryTerm {
            source: id.into_global_any(tree.module_id),
            value: self
                .check
                .require_local_node_type(tree.module_id, left)
                .into(),
            kind: TryTermKind::Maybe,
        };
        let term = self.check.inference.terms.push(tried.clone());

        self.output_node_type(tree.module_id, id, TypeTerm::Try(term));
        self.propagate_try(tree.module_id, id.into_any(), tried.value);
    }

    /// Walk one must propagation expression.
    ///
    /// Example:
    /// ```ds
    /// value!
    /// ```
    fn walk_must_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
    ) {
        self.walk_expression(tree, left, tree.get(left));

        let value = self.check.require_local_node_type(tree.module_id, left);
        let term = self.check.inference.terms.push(TryTerm {
            source: id.into_global_any(tree.module_id),
            value: value.into(),
            kind: TryTermKind::Must,
        });
        let term = TypeTerm::Try(term);

        self.output_node_type(tree.module_id, id, term);
    }

    /// Walk one binary expression.
    ///
    /// Example:
    /// ```ds
    /// left + right
    /// ```
    fn walk_binary_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) {
        match operator {
            // left && right
            dir::BinaryOperator::And => {
                self.walk_short_circuit_expression(tree, left, right, ConditionBranch::True);
            }
            // left || right
            dir::BinaryOperator::Or => {
                self.walk_short_circuit_expression(tree, left, right, ConditionBranch::False);
            }
            // eager binary operators
            _ => {
                self.walk_expression(tree, left, tree.get(left));
                self.walk_expression(tree, right, tree.get(right));
            }
        }

        let term = self.lower_binary_expression_type(tree, id, left, operator, right);

        self.output_node_type(tree.module_id, id, term);
    }

    /// Walk one short circuit binary expression.
    ///
    /// Example:
    /// ```ds
    /// left && right
    /// ```
    fn walk_short_circuit_expression(
        &mut self,
        tree: &dir::Tree,
        left: dir::LocalNodeId<dir::Expression>,
        right: dir::LocalNodeId<dir::Expression>,
        right_branch: ConditionBranch,
    ) {
        self.walk_expression(tree, left, tree.get(left));

        let before_right = self.checkpoint_flow();

        // walk right branch
        self.restore_flow(before_right);
        self.narrow_expression(tree, left, right_branch);
        self.walk_expression(tree, right, tree.get(right));
        let right_flow = self
            .can_expression_fall_through(tree, right)
            .then(|| self.collect_flow_branch(before_right));

        // collect skip branch
        self.restore_flow(before_right);
        self.narrow_expression(tree, left, right_branch.opposite());
        let skip_flow = self.collect_flow_branch(before_right);

        if let Some(right_flow) = right_flow {
            self.merge_flow_branches(before_right, &skip_flow, &right_flow);
        } else {
            self.restore_flow_branch(before_right, &skip_flow);
        }
    }

    /// Walk one assignment expression.
    ///
    /// Example:
    /// ```ds
    /// target += value
    /// ```
    fn walk_assign_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::AssignPattern>,
        operator: dir::AssignOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) {
        self.walk_assign_pattern(tree, left, tree.get(left));
        self.walk_expression(tree, right, tree.get(right));

        let value = self.check.require_local_node_type(tree.module_id, right);
        let assigned_value = self.ensure_assigned_value_type(tree, id, left, operator, value);

        // set assignment expression result
        if let Some(assigned_value) = assigned_value {
            let term = self.lower_assignment_result_type(tree, id, left, assigned_value);

            self.output_node_type(tree.module_id, id, term);
        }

        // constrain assigned value against target
        if let Some(assigned_value) = assigned_value {
            self.constrain_assignment_target(tree, left, right, assigned_value);
        }

        // write after reading the assigned value
        self.write_assign_pattern(left, tree);
    }

    /// Return tuple elements for one expression.
    ///
    /// Example:
    /// ```ds
    /// [name: value, ...rest]
    /// ```
    fn lower_tuple_elements(
        &mut self,
        tree: &dir::Tree,
        elements: &[dir::LocalNodeId<dir::Argument>],
    ) -> Vec<TupleElement> {
        let mut terms = Vec::new();

        // keep tuple element order
        for element in elements {
            let argument = tree.get(*element);
            let label = match argument {
                // [label = value]
                dir::Argument::Labeled { label, .. } => Some(*label),
                // [...label: value]
                dir::Argument::Spread { label, .. } => *label,
                // [name: value], [value]
                dir::Argument::Named { .. } | dir::Argument::Positional { .. } => None,
                // ignore damaged syntax
                dir::Argument::Error => continue,
            };
            let Some(value) = argument.value() else {
                continue;
            };

            terms.push(TupleElement {
                label,
                ty: self
                    .check
                    .require_local_node_type(tree.module_id, value)
                    .into(),
                is_optional: false,
                is_readonly: false,
                is_rest: matches!(argument, dir::Argument::Spread { .. }),
            });
        }

        terms
    }

    /// Return string and span segments for one template literal.
    ///
    /// Example:
    /// ```ds
    /// `hello ${name}`
    /// ```
    fn lower_template_segments(
        &mut self,
        value: &dir::TemplateLiteral,
        tree: &dir::Tree,
    ) -> (Vec<dir::StringId>, Vec<TypeOperand>) {
        match value {
            // `text`
            dir::TemplateLiteral::String { string } => (vec![*string], Vec::new()),
            // `text ${value}`
            dir::TemplateLiteral::InterpolatedString { strings, arguments } => {
                let spans = self.lower_argument_value_type_operands(arguments, tree);

                (strings.clone(), spans)
            }
        }
    }

    /// Return the type term for one binary expression.
    ///
    /// Example:
    /// ```ds
    /// left + right
    /// ```
    fn lower_binary_expression_type(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> TypeTerm {
        let source = id.into_global_any(tree.module_id);
        let left_type = self.check.require_local_node_type(tree.module_id, left);
        let right_type = self.check.require_local_node_type(tree.module_id, right);

        match operator {
            // left === right
            dir::BinaryOperator::EqualStrict | dir::BinaryOperator::NotEqualStrict => {
                let identity = self.check.inference.terms.push(IdentityTerm {
                    source,
                    operator,
                    left: left_type,
                    right: right_type,
                });

                TypeTerm::Identity(identity)
            }
            // key in receiver
            dir::BinaryOperator::In => {
                let membership = self.check.inference.terms.push(KeyMembershipTerm {
                    source,
                    key: left_type,
                    receiver: right_type,
                    static_key: tree.get(left).static_key(),
                });

                TypeTerm::KeyMembership(membership)
            }
            // overloaded operators
            _ => {
                let operator = self.check.inference.terms.push(OperatorTerm {
                    source,
                    kind: OperatorTermKind::Binary(operator),
                    receiver: left_type,
                    argument: Some(right_type),
                });

                TypeTerm::Operator(operator)
            }
        }
    }

    /// Ensure one assignment has a checked assigned value operand.
    ///
    /// Example:
    /// ```ds
    /// target += value
    /// ```
    fn ensure_assigned_value_type(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::AssignPattern>,
        operator: dir::AssignOperator,
        value: TypeOperand,
    ) -> Option<TypeOperand> {
        // use the right side for direct assignment
        if operator == dir::AssignOperator::Assign {
            return Some(value);
        }

        // allocate an intermediate for compound assignment
        match tree.get(left) {
            // target
            dir::AssignPattern::Expression { value: place } => {
                let source = id.into_global_any(tree.module_id);
                let origin = Origin::Node(source);
                let condition = self.active_static_guard();
                let receiver = self.check.require_local_node_type(tree.module_id, *place);
                let term = if let Some(binary_operator) = operator.binary_operator() {
                    let operator = self.check.inference.terms.push(OperatorTerm {
                        source,
                        kind: OperatorTermKind::Binary(binary_operator),
                        receiver,
                        argument: Some(value),
                    });

                    TypeTerm::Operator(operator)
                } else {
                    let operation = self.check.inference.terms.push(TypeOperationTerm::BestCommon {
                        elements: vec![receiver.into(), value],
                    });

                    TypeTerm::Operation(operation)
                };
                let result = self.check.allocate_variable(tree.module_id, VariableKind::Type, origin);

                self.check.equate_type(result, term, condition);

                Some(result.into())
            }
            // target = value
            dir::AssignPattern::Assign { .. }
            // [a, b]
            | dir::AssignPattern::Sequence { .. }
            // { a, b }
            | dir::AssignPattern::Object { .. } => None,
        }
    }

    /// Lower the expression result type for one assignment.
    ///
    /// Example:
    /// ```ds
    /// target[index] = value
    /// ```
    fn lower_assignment_result_type(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::AssignPattern>,
        assigned_value: TypeOperand,
    ) -> TypeTerm {
        match tree.get(left) {
            // target
            dir::AssignPattern::Expression { value: place } => {
                if let dir::Expression::Index {
                    left,
                    index: Some(index),
                    ..
                } = tree.get(*place)
                {
                    let receiver = self.check.require_local_node_type(tree.module_id, *left);
                    let index_type = self.check.require_local_node_type(tree.module_id, *index);
                    let set = self.check.inference.terms.push(IndexSetTerm {
                        source: id.into_global_any(tree.module_id),
                        receiver,
                        index: index_type,
                        value: assigned_value,
                        key: tree.get(*index).static_key(),
                    });

                    TypeTerm::IndexSet(set)
                } else {
                    self.lower_place(*place, tree)
                        .map(|place| place.ty.to_type_term(self.check))
                        .unwrap_or_else(|| {
                            self.check.require_local_node_type(tree.module_id, *place)
                                .to_type_term(self.check)
                        })
                }
            }
            // target = value
            dir::AssignPattern::Assign { .. }
            // [a, b]
            | dir::AssignPattern::Sequence { .. }
            // { a, b }
            | dir::AssignPattern::Object { .. } => assigned_value.to_type_term(self.check),
        }
    }

    /// Constrain one assignment target.
    ///
    /// Example:
    /// ```ds
    /// target = value
    /// ```
    fn constrain_assignment_target(
        &mut self,
        tree: &dir::Tree,
        left: dir::LocalNodeId<dir::AssignPattern>,
        right: dir::LocalNodeId<dir::Expression>,
        assigned_value: TypeOperand,
    ) {
        match tree.get(left) {
            // target
            dir::AssignPattern::Expression { value: place } => {
                if let Some(place) = self.lower_place(*place, tree) {
                    let target = place.ty;
                    let origin = Origin::Node(right.into_global_any(tree.module_id));
                    let condition = self.active_static_guard();

                    self.check.relate_type(
                        origin,
                        TypeRelation::Assignable,
                        assigned_value,
                        target,
                        condition,
                    );
                }
            }
            // target = value
            dir::AssignPattern::Assign { .. }
            // [a, b]
            | dir::AssignPattern::Sequence { .. }
            // { a, b }
            | dir::AssignPattern::Object { .. } => {
                if let Some(pattern) = self.lower_assign_pattern_term(tree.module_id, left, tree) {
                    let condition = self.active_static_guard();

                    self.check.relate_pattern(
                        tree.module_id,
                        PatternRelation::Assign(pattern),
                        left.into_any(),
                        assigned_value,
                        condition,
                    );
                }
            }
        }
    }

    /// Walk one declaration expression.
    ///
    /// Example:
    /// ```ds
    /// function value(): number { 1 }
    /// ```
    fn walk_declaration_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        declaration: dir::LocalNodeId<dir::Declaration>,
    ) {
        self.walk_declaration(tree, declaration, tree.get(declaration));

        // set declaration expression type from its symbol
        if tree.get(declaration).symbol_kind().is_some()
            && let Some(symbol) = self
                .check
                .declaration_symbol(tree.module_id, declaration.into_any())
        {
            let operand = match tree.get(declaration) {
                dir::Declaration::Type(declaration)
                    if !declaration.is_nominal
                        && self.is_intrinsic_language_item_type_declaration(
                            symbol,
                            tree.get(declaration.value),
                        ) =>
                {
                    let term = TypeTerm::Reference {
                        origin: Origin::Node(id.into_global_any(tree.module_id)),
                        symbol,
                        arguments: Default::default(),
                    };

                    self.check.inference.terms.push(term).into()
                }
                _ => {
                    if tree.get(declaration).symbol_kind().is_some_and(|kind| {
                        matches!(
                            kind,
                            dir::SymbolKind::Class
                                | dir::SymbolKind::Enum
                                | dir::SymbolKind::Interface
                                | dir::SymbolKind::Struct
                                | dir::SymbolKind::Newtype
                                | dir::SymbolKind::NewtypeInterface
                        )
                    }) {
                        let term = TypeTerm::Reference {
                            origin: Origin::Node(id.into_global_any(tree.module_id)),
                            symbol,
                            arguments: Default::default(),
                        };

                        self.check.inference.terms.push(term).into()
                    } else {
                        self.check.require_symbol_type(symbol)
                    }
                }
            };

            self.output_node_type_operand(tree.module_id, id, operand);
        }
    }

    /// Walk one block expression.
    ///
    /// Example:
    /// ```ds
    /// { const value = 1; value }
    /// ```
    fn walk_block_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        block: dir::LocalNodeId<dir::Block>,
    ) {
        self.walk_block(tree, block, tree.get(block));

        let variable = self.check.require_local_node_type(tree.module_id, block);
        let term = variable.to_type_term(self.check);

        self.output_node_type(tree.module_id, id, term);
    }

    /// Walk one object expression.
    ///
    /// Example:
    /// ```ds
    /// { name: value, read() { value } }
    /// ```
    fn walk_object_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        properties: &[dir::LocalNodeId<dir::Property>],
    ) {
        // walk property values and method bodies
        for property in properties {
            self.walk_property(tree, *property, tree.get(*property));
        }

        let mut members = Vec::new();

        // collect structural members from statically named properties
        for property in properties {
            match tree.get(*property) {
                // { key: value }
                dir::Property::Field { key, value, .. } => {
                    let Some(key) = key.static_key(tree) else {
                        continue;
                    };

                    let variable = self.check.require_local_node_type(tree.module_id, *value);
                    let member = ShapeMember::Field {
                        key,
                        ty: variable.into(),
                        is_optional: false,
                        is_readonly: false,
                    };

                    members.push(member);
                }
                // { method() {} }
                dir::Property::Method { key: Some(key), .. } => {
                    let Some(key) = key.static_key(tree) else {
                        continue;
                    };
                    let variable = self
                        .check
                        .declaration_symbol(tree.module_id, (*property).into_any())
                        .map(|symbol| self.check.require_symbol_type(symbol))
                        .unwrap_or_else(|| {
                            self.check
                                .require_local_node_type(tree.module_id, *property)
                        });
                    let member = ShapeMember::Field {
                        key,
                        ty: variable.into(),
                        is_optional: false,
                        is_readonly: false,
                    };

                    members.push(member);
                }
                // { []() {} }
                dir::Property::Method { key: None, .. } => {}
                // { ...value }
                dir::Property::Spread { .. } => {}
                // ignore damaged syntax
                dir::Property::Error => {}
            }
        }
        let term = TypeTerm::Shape {
            members: members.into(),
        };

        self.output_node_type(tree.module_id, id, term);
    }

    /// Walk one call expression.
    ///
    /// Example:
    /// ```ds
    /// callee<T>(argument)
    /// ```
    fn walk_call_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) {
        let source = id.into_global_any(tree.module_id);
        self.walk_expression(tree, left, tree.get(left));

        for argument in arguments {
            self.walk_argument(tree, *argument, tree.get(*argument));
        }

        let callee_source = left.into_global_any(tree.module_id);
        let generic_owner = self.check.selected_name(callee_source);
        let callee = self.check.require_local_node_type(tree.module_id, left);

        // classify member and reference callees
        let callee = if let dir::Expression::Member {
            left: receiver,
            name: Some(name),
        }
        | dir::Expression::PrivateMember {
            left: receiver,
            name: Some(name),
        } = tree.get(left)
        {
            let receiver = self
                .check
                .require_local_node_type(tree.module_id, *receiver);
            let member = MemberCallTerm {
                callee: MemberCallCallee::Source {
                    source: left.into_global_any(tree.module_id),
                },
                receiver,
                key: dir::StaticKey::Name(*name),
                arguments: Vec::new().into(),
            };
            let member = self.check.inference.terms.push(member);

            CallCallee::Member(member)
        } else if let Some(symbol) = generic_owner {
            CallCallee::Reference {
                value: callee,
                symbol,
            }
        } else {
            CallCallee::Value(callee)
        };

        let generic_arguments_term =
            self.lower_generic_arguments(generic_owner, generic_arguments, tree);
        let arguments_term = self.lower_argument_value_type_operands(arguments, tree);
        let call = self.check.inference.terms.push(CallTerm {
            source,
            callee,
            generic_arguments: generic_arguments_term,
            arguments: arguments_term.into_iter().map(TypeOperand::from).collect(),
        });
        let term = TypeTerm::Call(call);

        self.output_node_type(tree.module_id, id, term);
    }

    /// Walk one construct expression.
    ///
    /// Example:
    /// ```ds
    /// new Box(value)
    /// ```
    fn walk_construct_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) {
        let term = self.lower_construct_expression_term(tree, id, ty, arguments);

        self.output_node_type(tree.module_id, id, term);
    }

    /// Walk one fallible construct expression.
    ///
    /// Example:
    /// ```ds
    /// new? Box(value)
    /// ```
    fn walk_maybe_construct_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) {
        let source = id.into_global_any(tree.module_id);
        let constructed = self.lower_construct_expression_term(tree, id, ty, arguments);
        let constructed = self.check.inference.terms.push(constructed);
        let tried = TryTerm {
            source,
            value: constructed.into(),
            kind: TryTermKind::Maybe,
        };
        let term = self.check.inference.terms.push(tried.clone());

        self.output_node_type(tree.module_id, id, TypeTerm::Try(term));
        self.propagate_try(tree.module_id, id.into_any(), tried.value);
    }

    /// Return one construct expression term.
    ///
    /// Example:
    /// ```ds
    /// new Box(value)
    /// ```
    fn lower_construct_expression_term(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> TypeTerm {
        self.walk_type_expression(tree, ty, tree.get(ty));

        // walk constructor arguments
        for argument in arguments {
            self.walk_argument(tree, *argument, tree.get(*argument));
        }

        let callee = self.check.require_local_node_type(tree.module_id, ty);
        let argument_types = self.lower_argument_value_type_operands(arguments, tree);
        let construct = self.check.inference.terms.push(ConstructTerm {
            source: id.into_global_any(tree.module_id),
            callee,
            generic_arguments: Default::default(),
            arguments: argument_types.into_iter().map(TypeOperand::from).collect(),
        });

        TypeTerm::Construct(construct)
    }

    /// Walk one const assertion without widening aggregate literals.
    ///
    /// Example:
    /// ```ds
    /// [1, 2, 3] as const
    /// ```
    fn walk_const_assertion_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        child: dir::LocalNodeId<dir::Expression>,
    ) {
        let term = self.lower_const_assertion_type(tree, child);

        self.output_node_type(tree.module_id, id, term);
        self.walk_const_assertion_source(tree, child);
    }

    /// Return the deep readonly literal type for one const assertion.
    ///
    /// Example:
    /// ```ds
    /// { count: 0 } as const
    /// ```
    fn lower_const_assertion_type(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> TypeTerm {
        match tree.get(id) {
            // 1, "text", true
            dir::Expression::ScalarLiteral(value) => {
                TypeTerm::Literal(TypeLiteralTerm::Scalar(value.clone()))
            }
            // [a, b]
            dir::Expression::ArrayExpression { elements } => {
                let elements = self.lower_const_assertion_tuple_elements(tree, elements);
                let term = TypeTerm::Tuple {
                    form: dir::TupleForm::Array,
                    elements: elements.into(),
                };

                self.lower_readonly_type_term(term)
            }
            // (a, b)
            dir::Expression::TupleExpression { elements } => {
                let elements = self.lower_const_assertion_tuple_elements(tree, elements);
                let term = TypeTerm::Tuple {
                    form: dir::TupleForm::Tuple,
                    elements: elements.into(),
                };

                self.lower_readonly_type_term(term)
            }
            // { key: value }
            dir::Expression::ObjectExpression { properties } => {
                let members = self.lower_const_assertion_shape_members(tree, properties);

                TypeTerm::Shape {
                    members: members.into(),
                }
            }
            // preserve already checked source type
            _ => {
                let ty = self.check.require_local_node_type(tree.module_id, id);

                ty.to_type_term(self.check)
            }
        }
    }

    /// Lower one readonly wrapper around a type term.
    ///
    /// Example:
    /// ```ds
    /// value as const
    /// ```
    fn lower_readonly_type_term(&mut self, term: TypeTerm) -> TypeTerm {
        let payload = self.check.inference.terms.push(term);
        let form = self.check.inference.terms.push(FormTerm::Readonly);

        TypeTerm::Form {
            form,
            payload: payload.into(),
        }
    }

    /// Walk one const assertion source without widening aggregate literals.
    ///
    /// Example:
    /// ```ds
    /// [1, 2, 3] as const
    /// ```
    fn walk_const_assertion_source(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
    ) {
        match tree.get(id) {
            // [a, b], (a, b)
            dir::Expression::ArrayExpression { elements }
            | dir::Expression::TupleExpression { elements } => {
                let term = self.lower_const_assertion_type(tree, id);

                self.output_node_type(tree.module_id, id, term);

                for element in elements {
                    let Some(value) = tree.get(*element).value() else {
                        continue;
                    };

                    self.walk_const_assertion_source(tree, value);
                }
            }
            // { key: value }
            dir::Expression::ObjectExpression { properties } => {
                let term = self.lower_const_assertion_type(tree, id);

                self.output_node_type(tree.module_id, id, term);

                for property in properties {
                    match tree.get(*property) {
                        // { key: value }
                        dir::Property::Field { value, .. } => {
                            self.walk_const_assertion_source(tree, *value);
                        }
                        // { method() {} }, { ...value }, damaged syntax
                        dir::Property::Method { .. }
                        | dir::Property::Spread { .. }
                        | dir::Property::Error => {
                            self.walk_property(tree, *property, tree.get(*property));
                        }
                    }
                }
            }
            // walk non aggregate source normally
            _ => {
                self.walk_expression(tree, id, tree.get(id));
            }
        }
    }

    /// Return tuple elements for one const assertion.
    ///
    /// Example:
    /// ```ds
    /// [1, label: 2] as const
    /// ```
    fn lower_const_assertion_tuple_elements(
        &mut self,
        tree: &dir::Tree,
        elements: &[dir::LocalNodeId<dir::Argument>],
    ) -> Vec<TupleElement> {
        let mut terms = Vec::new();

        // keep tuple element order
        for element in elements {
            let argument = tree.get(*element);
            let label = match argument {
                // [label = value]
                dir::Argument::Labeled { label, .. } => Some(*label),
                // [...label: value]
                dir::Argument::Spread { label, .. } => *label,
                // [name: value], [value]
                dir::Argument::Named { .. } | dir::Argument::Positional { .. } => None,
                // ignore damaged syntax
                dir::Argument::Error => continue,
            };
            let Some(value) = argument.value() else {
                continue;
            };
            let ty = self.check.require_local_node_type(tree.module_id, value);

            terms.push(TupleElement {
                label,
                ty: ty.into(),
                is_optional: false,
                is_readonly: false,
                is_rest: matches!(argument, dir::Argument::Spread { .. }),
            });
        }

        terms
    }

    /// Return object members for one const assertion.
    ///
    /// Example:
    /// ```ds
    /// { count: 0 } as const
    /// ```
    fn lower_const_assertion_shape_members(
        &mut self,
        tree: &dir::Tree,
        properties: &[dir::LocalNodeId<dir::Property>],
    ) -> Vec<ShapeMember> {
        let mut members = Vec::new();

        // keep object member order
        for property in properties {
            match tree.get(*property) {
                // { key: value }
                dir::Property::Field { key, value, .. } => {
                    let Some(key) = key.static_key(tree) else {
                        continue;
                    };
                    let ty = self.check.require_local_node_type(tree.module_id, *value);

                    members.push(ShapeMember::Field {
                        key,
                        ty: ty.into(),
                        is_optional: false,
                        is_readonly: true,
                    });
                }
                // { method() {} }
                dir::Property::Method { key: Some(key), .. } => {
                    let Some(key) = key.static_key(tree) else {
                        continue;
                    };
                    let ty = self
                        .check
                        .declaration_symbol(tree.module_id, (*property).into_any())
                        .map(|symbol| self.check.require_symbol_type(symbol))
                        .unwrap_or_else(|| {
                            self.check
                                .require_local_node_type(tree.module_id, *property)
                        });

                    members.push(ShapeMember::Field {
                        key,
                        ty: ty.into(),
                        is_optional: false,
                        is_readonly: true,
                    });
                }
                // { []() {} }, { ...value }, damaged syntax
                dir::Property::Method { key: None, .. }
                | dir::Property::Spread { .. }
                | dir::Property::Error => {}
            }
        }

        members
    }

    /// Walk one template literal's expression arguments.
    ///
    /// Example:
    /// ```ds
    /// `hello ${name}`
    /// ```
    fn walk_template_literal(&mut self, tree: &dir::Tree, value: &dir::TemplateLiteral) {
        match value {
            // `text`
            dir::TemplateLiteral::String { .. } => {}
            // `text ${value}`
            dir::TemplateLiteral::InterpolatedString { arguments, .. } => {
                for argument in arguments {
                    self.walk_argument(tree, *argument, tree.get(*argument));
                }
            }
        }
    }

    /// Return value type operands for expression arguments.
    ///
    /// Example:
    /// ```ds
    /// f(a, b, ...rest)
    /// ```
    pub(in crate::check) fn lower_argument_value_type_operands(
        &mut self,
        arguments: &[dir::LocalNodeId<dir::Argument>],
        tree: &dir::Tree,
    ) -> Vec<TypeOperand> {
        arguments
            .iter()
            .filter_map(|argument| tree.get(*argument).value())
            .map(|value| self.check.require_local_node_type(tree.module_id, value))
            .collect()
    }

    /// Write through one assignment pattern.
    ///
    /// Example:
    /// ```ds
    /// { name } = value
    /// ```
    fn write_assign_pattern(&mut self, id: dir::LocalNodeId<dir::AssignPattern>, tree: &dir::Tree) {
        match tree.get(id) {
            // target
            dir::AssignPattern::Expression { value } => {
                if let Some(place) = self.lower_place(*value, tree) {
                    let condition = self.active_static_guard();

                    self.clear_mutated_expression_narrowings(tree, *value);
                    self.mark_place_assigned(place);
                    self.check.require_writable_place(place, condition);
                }
            }
            // target = value
            dir::AssignPattern::Assign { pattern, .. } => {
                self.write_assign_pattern(*pattern, tree);
            }
            // [a, b]
            dir::AssignPattern::Sequence { fields }
            // { a, b }
            | dir::AssignPattern::Object { fields } => {
                for field in fields {
                    self.write_assign_pattern_field(*field, tree);
                }
            }
        }
    }

    /// Write through one assignment pattern field.
    ///
    /// Example:
    /// ```ds
    /// { name: target } = value
    /// ```
    fn write_assign_pattern_field(
        &mut self,
        id: dir::LocalNodeId<dir::AssignPatternField>,
        tree: &dir::Tree,
    ) {
        match tree.get(id) {
            // { name: pattern }
            dir::AssignPatternField::Named {
                pattern: Some(pattern),
                ..
            }
            // { ...pattern }
            | dir::AssignPatternField::Spread {
                pattern: Some(pattern),
            } => {
                self.write_assign_pattern(*pattern, tree);
            }
            // { [key]: pattern }
            dir::AssignPatternField::Computed { pattern, .. }
            // [pattern]
            | dir::AssignPatternField::Positional { pattern } => {
                self.write_assign_pattern(*pattern, tree);
            }
            // { name }
            dir::AssignPatternField::Named { pattern: None, .. }
            // { ... }
            | dir::AssignPatternField::Spread { pattern: None }
            // [,]
            | dir::AssignPatternField::Elision => {}
        }
    }

    /// Walk one labeled expression target.
    ///
    /// Example:
    /// ```ds
    /// label: loop { break label value }
    /// ```
    fn walk_label_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        label: dir::StringId,
        body: dir::LocalNodeId<dir::Expression>,
    ) {
        match tree.get(body) {
            // label: while condition { body }
            dir::Expression::While {
                condition,
                body: loop_body,
                ..
            } => {
                let body_type = self.check.require_local_node_type(tree.module_id, body);

                self.output_node_type(tree.module_id, id, body_type.to_type_term(self.check));
                if self.push_static_guard_for(tree, body.into_any(), None) {
                    self.check.require_local_node_type(tree.module_id, body);
                    self.walk_while_expression(tree, body, Some(label), *condition, *loop_body);
                    self.pop_static_guard();
                }

                return;
            }
            // label: for item of iterator { body }
            dir::Expression::ForEach {
                operator,
                binding,
                iterator,
                body: loop_body,
                ..
            } => {
                let body_type = self.check.require_local_node_type(tree.module_id, body);

                self.output_node_type(tree.module_id, id, body_type.to_type_term(self.check));
                if self.push_static_guard_for(tree, body.into_any(), None) {
                    self.check.require_local_node_type(tree.module_id, body);
                    self.walk_for_each_expression(
                        tree,
                        body,
                        Some(label),
                        *operator,
                        binding,
                        *iterator,
                        *loop_body,
                    );
                    self.pop_static_guard();
                }

                return;
            }
            // label: for (initialization; condition; increment) { body }
            dir::Expression::For {
                initialization,
                condition,
                increment,
                body: loop_body,
            } => {
                let body_type = self.check.require_local_node_type(tree.module_id, body);

                self.output_node_type(tree.module_id, id, body_type.to_type_term(self.check));
                if self.push_static_guard_for(tree, body.into_any(), None) {
                    self.check.require_local_node_type(tree.module_id, body);
                    self.walk_for_expression(
                        tree,
                        body,
                        Some(label),
                        *initialization,
                        *condition,
                        *increment,
                        *loop_body,
                    );
                    self.pop_static_guard();
                }

                return;
            }
            // label: loop { body }
            dir::Expression::Loop { body: loop_body } => {
                let body_type = self.check.require_local_node_type(tree.module_id, body);

                self.output_node_type(tree.module_id, id, body_type.to_type_term(self.check));
                if self.push_static_guard_for(tree, body.into_any(), None) {
                    self.check.require_local_node_type(tree.module_id, body);
                    self.walk_loop_expression(tree, body, Some(label), *loop_body);
                    self.pop_static_guard();
                }

                return;
            }
            // labeled expression
            _ => {}
        }

        // enter labeled control target
        let checked_type = self.check.require_local_node_type(tree.module_id, id);
        let result = self.check.ensure_type_operand_variable(
            tree.module_id,
            Origin::Node(id.into_global_any(tree.module_id)),
            checked_type,
            self.active_static_guard(),
        );
        self.enter_control_target(Some(label), false, result);

        // walk body with isolated flow
        let before_body = self.checkpoint_flow();
        self.walk_expression(tree, body, tree.get(body));
        let fallthrough = if self.can_expression_fall_through(tree, body) {
            Some(
                self.check
                    .require_local_node_type(tree.module_id, body)
                    .to_type_term(self.check),
            )
        } else {
            None
        };

        // merge fallthrough and break branches
        let body_flow = fallthrough
            .is_some()
            .then(|| self.collect_flow_branch(before_body));
        let mut branches = self.leave_control_target(fallthrough);
        if let Some(body_flow) = body_flow {
            branches.push(body_flow);
        }

        self.merge_flow_branches_from(before_body, &branches);
    }

    /// Walk one if expression with isolated branch flow.
    ///
    /// Example:
    /// ```ds
    /// if value is T { value } else { fallback }
    /// ```
    fn walk_if_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        condition: &dir::IfCondition,
        then_expression: dir::LocalNodeId<dir::Expression>,
        else_expression: Option<dir::LocalNodeId<dir::Expression>>,
    ) {
        let then_type = self
            .check
            .require_local_node_type(tree.module_id, then_expression);
        let else_type = match else_expression {
            Some(else_expression) => self
                .check
                .require_local_node_type(tree.module_id, else_expression)
                .into(),
            None => self.void_type_operand(),
        };
        let term = TypeTerm::Union {
            elements: vec![then_type.into(), else_type],
        };

        self.output_node_type(tree.module_id, id, term);

        // walk branches with isolated flow
        self.walk_if_condition(tree, condition);
        let before = self.checkpoint_flow();

        self.restore_flow(before);
        self.narrow_condition(tree, condition, ConditionBranch::True);
        self.walk_expression(tree, then_expression, tree.get(then_expression));
        let then_flow = self.collect_flow_branch(before);
        let then_can_fall_through = self.can_expression_fall_through(tree, then_expression);

        if let Some(else_expression) = else_expression {
            self.restore_flow(before);
            self.narrow_condition(tree, condition, ConditionBranch::False);
            self.walk_expression(tree, else_expression, tree.get(else_expression));
            let else_flow = self.collect_flow_branch(before);
            let else_can_fall_through = self.can_expression_fall_through(tree, else_expression);

            match (then_can_fall_through, else_can_fall_through) {
                (true, true) => self.merge_flow_branches(before, &then_flow, &else_flow),
                (true, false) => self.restore_flow_branch(before, &then_flow),
                (false, true) => self.restore_flow_branch(before, &else_flow),
                (false, false) => self.restore_flow(before),
            }
        } else {
            self.restore_flow(before);
            self.narrow_condition(tree, condition, ConditionBranch::False);
            let else_flow = self.collect_flow_branch(before);

            if then_can_fall_through {
                self.merge_flow_branches(before, &else_flow, &then_flow);
            } else {
                self.restore_flow_branch(before, &else_flow);
            }
        }
    }

    /// Walk one if condition.
    ///
    /// Example:
    /// ```ds
    /// if let Some(value) = option { value }
    /// ```
    fn walk_if_condition(&mut self, tree: &dir::Tree, condition: &dir::IfCondition) {
        match condition {
            // if condition
            dir::IfCondition::Expression { condition } => {
                self.walk_expression(tree, *condition, tree.get(*condition));

                let variable = self
                    .check
                    .require_local_node_type(tree.module_id, *condition);
                let static_guard = self.active_static_guard();

                self.check.constrain_condition(
                    tree.module_id,
                    (*condition).into_any(),
                    variable,
                    static_guard,
                );
            }
            // if let pattern = value
            dir::IfCondition::Let { declarator, .. } => {
                self.walk_declarator(tree, *declarator, tree.get(*declarator));
            }
        }
    }

    /// Narrow flow from one condition.
    ///
    /// Example:
    /// ```ds
    /// if value is T { value }
    /// ```
    fn narrow_condition(
        &mut self,
        tree: &dir::Tree,
        condition: &dir::IfCondition,
        branch: ConditionBranch,
    ) {
        match condition {
            // if condition
            dir::IfCondition::Expression { condition } => {
                self.narrow_expression(tree, *condition, branch);
            }
            // if let pattern = value
            dir::IfCondition::Let { declarator, .. } if branch == ConditionBranch::True => {
                self.mark_declarator_assigned(tree, tree.get(*declarator));
                self.narrow_declarator_pattern_success(tree, *declarator);
            }
            // if let pattern = value
            dir::IfCondition::Let { .. } => {}
        }
    }

    /// Narrow flow from one expression condition.
    ///
    /// Example:
    /// ```ds
    /// if value !== undefined { value }
    /// ```
    pub(in crate::check) fn narrow_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        branch: ConditionBranch,
    ) {
        match tree.get(id) {
            // (value)
            dir::Expression::Parenthesized { expression } => {
                self.narrow_expression(tree, *expression, branch);
            }
            // !value
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Not,
                right,
            } => {
                self.narrow_expression(tree, *right, branch.opposite());
            }
            // left && right
            dir::Expression::Binary {
                left,
                operator: dir::BinaryOperator::And,
                right,
            } if branch == ConditionBranch::True => {
                self.narrow_expression(tree, *left, branch);
                self.narrow_expression(tree, *right, branch);
            }
            // left || right
            dir::Expression::Binary {
                left,
                operator: dir::BinaryOperator::Or,
                right,
            } if branch == ConditionBranch::False => {
                self.narrow_expression(tree, *left, branch);
                self.narrow_expression(tree, *right, branch);
            }
            // left === right
            dir::Expression::Binary {
                left,
                operator,
                right,
            } if Self::is_equality_operator(*operator) => {
                let branch = if Self::is_negative_equality_operator(*operator) {
                    branch.opposite()
                } else {
                    branch
                };

                self.narrow_by_equality(tree, *left, *right, branch);
                self.narrow_by_equality(tree, *right, *left, branch);
            }
            // key in value
            dir::Expression::Binary {
                left,
                operator: dir::BinaryOperator::In,
                right,
            } => {
                self.narrow_by_key_membership(tree, *left, *right, branch);
            }
            // value is T
            dir::Expression::Is { value, target_type } => {
                self.narrow_by_is(tree, *value, *target_type, branch);
            }
            // value instanceof Target
            dir::Expression::InstanceOf { value, target } => {
                self.narrow_by_instance(tree, *value, *target, branch);
            }
            // expressions without flow effects
            _ => {}
        }
    }

    /// Narrow flow from one `is` expression.
    ///
    /// Example:
    /// ```ds
    /// value is T
    /// ```
    fn narrow_by_is(
        &mut self,
        tree: &dir::Tree,
        value: dir::LocalNodeId<dir::Expression>,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
        branch: ConditionBranch,
    ) {
        let Some(path) = self.flow_path(tree, value) else {
            return;
        };
        let target = self
            .check
            .require_local_node_type(tree.module_id, target_type);

        match branch {
            ConditionBranch::True => {
                self.narrow_flow_path(path, target);
            }
            ConditionBranch::False => {
                let original = self.check.require_local_node_type(tree.module_id, value);

                self.narrow_flow_path_excluding(path, original, target);
            }
        }
    }

    /// Narrow flow from one `"key" in value` expression.
    ///
    /// Example:
    /// ```ds
    /// "name" in value
    /// ```
    fn narrow_by_key_membership(
        &mut self,
        tree: &dir::Tree,
        key: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
        branch: ConditionBranch,
    ) {
        if branch == ConditionBranch::False {
            return;
        }
        let Some(path) = self.flow_path(tree, value) else {
            return;
        };
        let Some(key) = tree.get(key).static_key() else {
            return;
        };
        let ty = self
            .check
            .inference
            .terms
            .push(TypeTerm::Literal(TypeLiteralTerm::Unknown));
        let member = ShapeMember::Field {
            key,
            ty: ty.into(),
            is_optional: false,
            is_readonly: false,
        };
        let target = self.check.inference.terms.push(TypeTerm::Shape {
            members: vec![member].into(),
        });

        self.narrow_flow_path(path, target);
    }

    /// Narrow flow from one `value instanceof Target` expression.
    ///
    /// Example:
    /// ```ds
    /// value instanceof Target
    /// ```
    fn narrow_by_instance(
        &mut self,
        tree: &dir::Tree,
        value: dir::LocalNodeId<dir::Expression>,
        target: dir::LocalNodeId<dir::Expression>,
        branch: ConditionBranch,
    ) {
        let Some(path) = self.flow_path(tree, value) else {
            return;
        };
        let target = self.check.require_local_node_type(tree.module_id, target);

        match branch {
            ConditionBranch::True => {
                self.narrow_flow_path(path, target);
            }
            ConditionBranch::False => {
                let original = self.check.require_local_node_type(tree.module_id, value);

                self.narrow_flow_path_excluding(path, original, target);
            }
        }
    }

    /// Narrow flow from one equality expression.
    ///
    /// Example:
    /// ```ds
    /// value === undefined
    /// ```
    fn narrow_by_equality(
        &mut self,
        tree: &dir::Tree,
        value: dir::LocalNodeId<dir::Expression>,
        target: dir::LocalNodeId<dir::Expression>,
        branch: ConditionBranch,
    ) {
        let Some(path) = self.flow_path(tree, value) else {
            return;
        };
        let Some(target) = self.lower_equality_target_type(tree, target) else {
            return;
        };

        match branch {
            ConditionBranch::True => {
                self.narrow_flow_path(path, target);
            }
            ConditionBranch::False => {
                let original = self.check.require_local_node_type(tree.module_id, value);

                self.narrow_flow_path_excluding(path, original, target);
            }
        }
    }

    /// Return the literal type used by one equality test.
    ///
    /// Example:
    /// ```ds
    /// value === "ready"
    /// ```
    fn lower_equality_target_type(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<TypeOperand> {
        let term = match tree.get(id) {
            // literal
            dir::Expression::ScalarLiteral(value) => {
                TypeTerm::Literal(TypeLiteralTerm::Scalar(value.clone()))
            }
            // undefined
            dir::Expression::Identifier { name }
                if *name == dir::StringId::for_text("undefined") =>
            {
                TypeTerm::Literal(TypeLiteralTerm::Undefined)
            }
            // not a literal equality target
            _ => return None,
        };

        let term = self.check.inference.terms.push(term);

        Some(term.into())
    }

    /// Return whether one binary operator tests equality.
    fn is_equality_operator(operator: dir::BinaryOperator) -> bool {
        matches!(
            operator,
            dir::BinaryOperator::Equal
                | dir::BinaryOperator::NotEqual
                | dir::BinaryOperator::EqualStrict
                | dir::BinaryOperator::NotEqualStrict
        )
    }

    /// Return whether one equality operator negates the relation.
    fn is_negative_equality_operator(operator: dir::BinaryOperator) -> bool {
        matches!(
            operator,
            dir::BinaryOperator::NotEqual | dir::BinaryOperator::NotEqualStrict
        )
    }

    /// Walk one while expression.
    ///
    /// Example:
    /// ```ds
    /// while condition { body }
    /// ```
    fn walk_while_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        label: Option<dir::StringId>,
        condition: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Block>,
    ) {
        // walk condition in incoming flow
        self.walk_expression(tree, condition, tree.get(condition));
        let condition_type = self
            .check
            .require_local_node_type(tree.module_id, condition);
        let static_guard = self.active_static_guard();

        self.check.constrain_condition(
            tree.module_id,
            condition.into_any(),
            condition_type,
            static_guard,
        );

        // enter loop control target
        let checked_type = self.check.require_local_node_type(tree.module_id, id);
        let result = self.check.ensure_type_operand_variable(
            tree.module_id,
            Origin::Node(id.into_global_any(tree.module_id)),
            checked_type,
            self.active_static_guard(),
        );
        self.enter_control_target(label, true, result);

        // walk body under true condition flow
        let before_body = self.checkpoint_flow();
        self.narrow_expression(tree, condition, ConditionBranch::True);
        self.walk_block(tree, body, tree.get(body));
        self.restore_flow(before_body);

        // collect normal exit through false condition
        self.narrow_expression(tree, condition, ConditionBranch::False);
        let normal_flow = self.collect_flow_branch(before_body);
        let mut branches =
            self.leave_control_target(Some(TypeTerm::Literal(TypeLiteralTerm::Void)));

        // merge break branches with normal exit
        branches.push(normal_flow);
        self.merge_flow_branches_from(before_body, &branches);
    }

    /// Walk one for each expression.
    ///
    /// Example:
    /// ```ds
    /// for item of items { item }
    /// ```
    fn walk_for_each_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        label: Option<dir::StringId>,
        operator: dir::ForEachOperator,
        binding: &dir::ForEachBinding,
        iterator: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Block>,
    ) {
        // walk binding and iterator in incoming flow
        let pattern = match binding {
            dir::ForEachBinding::Pattern { pattern, .. }
            | dir::ForEachBinding::Using { pattern, .. } => *pattern,
        };
        self.walk_pattern(tree, pattern, tree.get(pattern));
        self.walk_expression(tree, iterator, tree.get(iterator));
        self.constrain_for_each_binding(id, operator, pattern, iterator, tree);

        // enter loop control target
        let checked_type = self.check.require_local_node_type(tree.module_id, id);
        let result = self.check.ensure_type_operand_variable(
            tree.module_id,
            Origin::Node(id.into_global_any(tree.module_id)),
            checked_type,
            self.active_static_guard(),
        );
        self.enter_control_target(label, true, result);

        // walk body with iteration binding assigned
        let before_body = self.checkpoint_flow();
        self.mark_bindings_assigned(tree, pattern.into_any());
        self.walk_block(tree, body, tree.get(body));
        self.restore_flow(before_body);

        // collect normal loop exit
        let normal_flow = self.collect_flow_branch(before_body);
        let mut branches =
            self.leave_control_target(Some(TypeTerm::Literal(TypeLiteralTerm::Void)));

        // merge break branches with normal exit
        branches.push(normal_flow);
        self.merge_flow_branches_from(before_body, &branches);
    }

    /// Walk one traditional for expression.
    ///
    /// Example:
    /// ```ds
    /// for (let i = 0; i < n; i++) { i }
    /// ```
    fn walk_for_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        label: Option<dir::StringId>,
        initialization: Option<dir::LocalNodeId<dir::Expression>>,
        condition: Option<dir::LocalNodeId<dir::Expression>>,
        increment: Option<dir::LocalNodeId<dir::Expression>>,
        body: dir::LocalNodeId<dir::Block>,
    ) {
        // walk initialization before loop flow splits
        if let Some(initialization) = initialization {
            self.walk_expression(tree, initialization, tree.get(initialization));
        }

        // walk condition in incoming flow
        if let Some(condition) = condition {
            self.walk_expression(tree, condition, tree.get(condition));

            let variable = self
                .check
                .require_local_node_type(tree.module_id, condition);
            let static_guard = self.active_static_guard();

            self.check.constrain_condition(
                tree.module_id,
                condition.into_any(),
                variable,
                static_guard,
            );
        }

        // enter loop control target
        let checked_type = self.check.require_local_node_type(tree.module_id, id);
        let result = self.check.ensure_type_operand_variable(
            tree.module_id,
            Origin::Node(id.into_global_any(tree.module_id)),
            checked_type,
            self.active_static_guard(),
        );
        self.enter_control_target(label, true, result);

        // walk body under true condition flow
        let before_body = self.checkpoint_flow();
        if let Some(condition) = condition {
            self.narrow_expression(tree, condition, ConditionBranch::True);
        }
        self.walk_block(tree, body, tree.get(body));
        let body_flow = self
            .can_block_fall_through(tree, tree.get(body))
            .then(|| self.collect_flow_branch(before_body));
        let continue_flows = self.take_current_continue_branches();
        if let Some(increment) = increment {
            self.walk_for_increment_expression(
                tree,
                increment,
                before_body,
                body_flow,
                &continue_flows,
            );
        }
        self.restore_flow(before_body);

        // collect normal exit through false condition
        let normal_flow = condition.map(|condition| {
            self.narrow_expression(tree, condition, ConditionBranch::False);

            self.collect_flow_branch(before_body)
        });
        let fallthrough = condition.map(|_| TypeTerm::Literal(TypeLiteralTerm::Void));
        let mut branches = self.leave_control_target(fallthrough);

        // merge break branches with normal exit
        if let Some(normal_flow) = normal_flow {
            branches.push(normal_flow);
        }

        self.merge_flow_branches_from(before_body, &branches);
    }

    /// Walk one traditional for increment from body and continue flows.
    ///
    /// Example:
    /// ```ds
    /// for (; condition; increment) { body }
    /// ```
    fn walk_for_increment_expression(
        &mut self,
        tree: &dir::Tree,
        increment: dir::LocalNodeId<dir::Expression>,
        before_body: FlowCheckpoint,
        body_flow: Option<FlowBranch>,
        continue_flows: &[FlowBranch],
    ) {
        let mut flows = Vec::with_capacity(continue_flows.len() + usize::from(body_flow.is_some()));
        flows.extend_from_slice(continue_flows);
        if let Some(body_flow) = body_flow {
            flows.push(body_flow);
        }

        // check unreachable increment once
        if flows.is_empty() {
            self.walk_expression(tree, increment, tree.get(increment));
            self.restore_flow(before_body);

            return;
        }

        // check increment from each flow that reaches the next iteration
        for flow in flows {
            self.restore_flow_branch(before_body, &flow);
            self.walk_expression(tree, increment, tree.get(increment));
            self.restore_flow(before_body);
        }
    }

    /// Constrain one for each binding to its iterated value.
    ///
    /// Example:
    /// ```ds
    /// for item of items { item }
    /// ```
    fn constrain_for_each_binding(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        operator: dir::ForEachOperator,
        pattern: dir::LocalNodeId<dir::Pattern>,
        iterator: dir::LocalNodeId<dir::Expression>,
        tree: &dir::Tree,
    ) {
        let source = id.into_global_any(tree.module_id);
        let origin = Origin::Node(source);
        let iterator_type = self.check.require_local_node_type(tree.module_id, iterator);
        let value = match operator {
            // for (const item of iterable)
            dir::ForEachOperator::Of => {
                let value =
                    self.check
                        .allocate_variable(tree.module_id, VariableKind::Type, origin);
                let unknown =
                    self.check
                        .allocate_variable(tree.module_id, VariableKind::Type, origin);
                let condition = self.active_static_guard();

                self.check.equate_type(
                    unknown,
                    TypeTerm::Literal(TypeLiteralTerm::Unknown),
                    condition.clone(),
                );
                let symbol = self
                    .check
                    .language_symbol(tree.module_id, dir::LanguageItem::Iterable);
                let value_argument = GenericArgument::Type(value.into());
                let first_unknown = GenericArgument::Type(unknown.into());
                let second_unknown = GenericArgument::Type(unknown.into());
                let iterable = TypeTerm::Reference {
                    origin: Origin::Node(source),
                    symbol,
                    arguments: vec![value_argument, first_unknown, second_unknown].into(),
                };
                let iterable_variable =
                    self.check
                        .allocate_variable(tree.module_id, VariableKind::Type, origin);
                self.check
                    .equate_type(iterable_variable, iterable, condition.clone());

                self.check.relate_type(
                    origin,
                    TypeRelation::Assignable,
                    iterator_type,
                    iterable_variable,
                    condition,
                );

                value
            }
            // for (const key in object)
            dir::ForEachOperator::In => {
                let operation = self.check.inference.terms.push(TypeOperationTerm::KeyOf {
                    target: iterator_type,
                });
                let term = TypeTerm::Operation(operation);
                let variable =
                    self.check
                        .allocate_variable(tree.module_id, VariableKind::Type, origin);
                let condition = self.active_static_guard();

                self.check.equate_type(variable, term, condition);

                variable
            }
        };

        if let Some(term) = self.lower_pattern_term(tree.module_id, pattern, tree) {
            let condition = self.active_static_guard();

            self.check.relate_pattern(
                tree.module_id,
                PatternRelation::Match(term),
                pattern.into_any(),
                value,
                condition,
            );
        }
    }

    /// Walk one loop expression.
    ///
    /// Example:
    /// ```ds
    /// loop { break value }
    /// ```
    fn walk_loop_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        label: Option<dir::StringId>,
        body: dir::LocalNodeId<dir::Block>,
    ) {
        // enter loop control target
        let checked_type = self.check.require_local_node_type(tree.module_id, id);
        let result = self.check.ensure_type_operand_variable(
            tree.module_id,
            Origin::Node(id.into_global_any(tree.module_id)),
            checked_type,
            self.active_static_guard(),
        );
        self.enter_control_target(label, true, result);

        // walk body with isolated flow
        let before_body = self.checkpoint_flow();
        self.walk_block(tree, body, tree.get(body));
        self.restore_flow(before_body);

        // restore only branches that leave the loop
        let branches = self.leave_control_target(None);
        self.merge_flow_branches_from(before_body, &branches);
    }

    /// Walk one try expression with branch flow for catch.
    ///
    /// Example:
    /// ```ds
    /// try body catch error finally cleanup
    /// ```
    fn walk_try_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Expression>,
        catch: Option<dir::LocalNodeId<dir::Catch>>,
        finally: Option<dir::LocalNodeId<dir::Expression>>,
    ) {
        let body_type = self.check.require_local_node_type(tree.module_id, body);
        let term = match catch {
            Some(catch) => {
                let catch_type = self
                    .check
                    .require_local_node_type(tree.module_id, tree.get(catch).body);

                TypeTerm::Union {
                    elements: vec![body_type.into(), catch_type.into()],
                }
            }
            None => body_type.to_type_term(self.check),
        };

        self.output_node_type(tree.module_id, id, term);

        // walk try branches
        let before = self.checkpoint_flow();

        // walk the body branch from the incoming flow
        self.restore_flow(before);
        if catch.is_some() {
            self.enter_try_target(tree.module_id, id.into_any());
        }
        self.walk_expression(tree, body, tree.get(body));
        let catch_failure = catch.map(|_| self.leave_try_target());
        let body_flow = self.collect_flow_branch(before);
        let body_can_fall_through = self.can_expression_fall_through(tree, body);

        // merge only branches that can continue normally
        let has_normal_flow = if let Some(catch) = catch {
            let can_catch_fall_through = self.can_catch_fall_through(tree, tree.get(catch));

            self.restore_flow(before);
            self.walk_catch(tree, catch, tree.get(catch), catch_failure);
            let catch_flow = self.collect_flow_branch(before);

            match (body_can_fall_through, can_catch_fall_through) {
                (true, true) => {
                    self.merge_flow_branches(before, &body_flow, &catch_flow);

                    true
                }
                (true, false) => {
                    self.restore_flow_branch(before, &body_flow);

                    true
                }
                (false, true) => {
                    self.restore_flow_branch(before, &catch_flow);

                    true
                }
                (false, false) => {
                    self.restore_flow(before);

                    false
                }
            }
        } else if body_can_fall_through {
            self.restore_flow_branch(before, &body_flow);

            true
        } else {
            self.restore_flow(before);

            false
        };

        // finally is checked even when no normal path remains
        if let Some(finally) = finally {
            self.walk_expression(tree, finally, tree.get(finally));

            if !has_normal_flow || !self.can_expression_fall_through(tree, finally) {
                self.restore_flow(before);
            }
        } else if !has_normal_flow {
            self.restore_flow(before);
        }
    }

    /// Walk one match expression with isolated case flow.
    ///
    /// Example:
    /// ```ds
    /// match value { case Some(item) => item }
    /// ```
    fn walk_match_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
        cases: &[dir::LocalNodeId<dir::MatchCase>],
    ) {
        // walk match discriminant
        self.walk_expression(tree, value, tree.get(value));
        let value_type = self.check.require_local_node_type(tree.module_id, value);
        let value_path = self.flow_path(tree, value);
        let mut active_cases = Vec::new();

        // select cases whose static guards can hold
        for case in cases {
            let condition = self.evaluate_owner_static_guard(tree, case.into_any(), None);
            if !condition.is_never() {
                active_cases.push((*case, condition));
            }
        }

        let before = self.checkpoint_flow();
        let mut elements = Vec::new();
        let mut case_terms = Vec::new();
        let mut merged = None;

        for (case, condition) in &active_cases {
            self.restore_flow(before);
            self.push_static_guard(condition.clone());
            self.walk_match_case(
                tree,
                *case,
                tree.get(*case),
                Some((value_type, value_path.clone())),
            );
            self.pop_static_guard();

            match tree.get(*case) {
                // case pattern => expression
                dir::MatchCase::Expression { body, .. } => {
                    let ty = self.check.require_local_node_type(tree.module_id, *body);

                    elements.push(TypeOperand::from(ty));
                }
                // case pattern => { ... }
                dir::MatchCase::Block { body, .. } => {
                    let ty = self.check.require_local_node_type(tree.module_id, *body);

                    elements.push(TypeOperand::from(ty));
                }
            }

            match tree.get(*case).selector() {
                // default
                dir::MatchSelector::Default => {
                    case_terms.push(MatchCase::Default);
                }
                // case pattern if guard
                dir::MatchSelector::Pattern { pattern, guard } => {
                    if let Some(pattern) = self.lower_pattern_term(tree.module_id, *pattern, tree) {
                        let guard = guard
                            .map(|guard| self.check.require_local_node_type(tree.module_id, guard));

                        case_terms.push(MatchCase::PatternTerm { pattern, guard });
                    }
                }
            }

            if !self.can_match_case_fall_through(tree, tree.get(*case)) {
                continue;
            }
            let case_flow = self.collect_flow_branch(before);

            if let Some(previous) = &merged {
                self.merge_flow_branches(before, previous, &case_flow);
                merged = Some(self.collect_flow_branch(before));
            } else {
                merged = Some(case_flow);
            }
        }

        if let Some(merged) = merged {
            self.restore_flow_branch(before, &merged);
        } else {
            self.restore_flow(before);
        }

        let condition = self.active_static_guard();

        self.check.require_exhaustive_match(
            tree.module_id,
            id.into_any(),
            value_type,
            case_terms,
            condition,
        );

        self.output_node_type(tree.module_id, id, TypeTerm::Union { elements });
    }

    /// Walk one catch clause.
    ///
    /// Example:
    /// ```ds
    /// catch (error: Error) { error }
    /// ```
    pub(in crate::check) fn walk_catch(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Catch>,
        catch: &dir::Catch,
        failure: Option<VariableId>,
    ) {
        if !self.push_static_guard_for(tree, id.into_any(), None) {
            return;
        }
        // catch (error)
        if let Some(pattern) = catch.pattern {
            self.walk_pattern(tree, pattern, tree.get(pattern));

            if let (Some(failure), Some(ty)) = (failure, catch.ty) {
                let expected = self.check.require_local_node_type(tree.module_id, ty);
                let origin = Origin::Node(ty.into_global_any(tree.module_id));
                let condition = self.active_static_guard();

                self.check.relate_type(
                    origin,
                    TypeRelation::Assignable,
                    failure,
                    expected,
                    condition,
                );
            }

            if let Some(value) = catch
                .ty
                .map(|ty| self.check.require_local_node_type(tree.module_id, ty))
                .or_else(|| failure.map(Into::into))
                && let Some(pattern_term) = self.lower_pattern_term(tree.module_id, pattern, tree)
            {
                let condition = self.active_static_guard();

                self.check.relate_pattern(
                    tree.module_id,
                    PatternRelation::Match(pattern_term),
                    pattern.into_any(),
                    value,
                    condition,
                );
            }

            self.mark_bindings_assigned(tree, pattern.into_any());
        }

        // catch (error: T)
        if let Some(ty) = catch.ty {
            self.walk_type_expression(tree, ty, tree.get(ty));
        }

        // catch (...) { ... }
        self.walk_expression(tree, catch.body, tree.get(catch.body));

        self.pop_static_guard();
    }
}

impl ConditionBranch {
    /// Return the opposite control branch.
    fn opposite(self) -> Self {
        match self {
            // true branch
            Self::True => Self::False,
            // false branch
            Self::False => Self::True,
        }
    }
}
