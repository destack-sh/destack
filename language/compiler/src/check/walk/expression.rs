use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    AwaitTerm, CallArgument, CallCallee, CallTerm, ConditionBranch, ConstructTerm, FlowBranch,
    FlowCheckpoint, FormTerm, GenericArgument, GenericParameterId, IdentityTerm, ImportMetaTerm,
    IndexKind, IndexSetTerm, IndexTerm, InstanceCheckTerm, KeyMembershipTerm, MatchCase,
    MemberCallTerm, MemberProjectionOrigin, MemberReceiver, MemberTerm, OperatorTerm,
    OperatorTermKind, Origin, PatternRelation, RangeTerm, ShapeMember, StaticTerm, SuperTerm,
    TaggedTemplateTerm, TemplateTerm, TreeTerm, TryTerm, TryTermKind, TupleElement,
    TypeLiteralTerm, TypeOperand, TypeOperationTerm, TypeRelation, TypeTerm, TypeValueTerm,
    VariableId, WalkState, YieldTerm,
};
use crate::{CompilerError, CompilerResult};

impl WalkState<'_, '_> {
    /// Walk one expression.
    ///
    /// Example:
    /// ```ds
    /// value.member<T>(argument)
    /// ```
    pub(in crate::check) fn walk_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) -> CompilerResult<()> {
        destack_core::ensure_sufficient_stack(|| self.walk_expression_inner(id, expression))
    }

    /// Walk one expression after stack growth is handled.
    ///
    /// Example:
    /// ```ds
    /// value.member<T>(argument)
    /// ```
    fn walk_expression_inner(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) -> CompilerResult<()> {
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any(), None)? else {
            return Ok(());
        };

        match expression {
            // function f() {}
            dir::Expression::Declaration(declaration) => self.walk_declaration_expression(id, *declaration),
            // { ... }
            dir::Expression::Block(block) => self.walk_block_expression(id, *block),
            // label: body
            dir::Expression::Label { label, body } => self.walk_label_expression(id, *label, *body),
            // import { item } from "module"
            dir::Expression::Import { items, .. } => self.walk_import_expression(id, items.as_deref()),
            // export { item } from "module"
            dir::Expression::Export { items, .. } => self.walk_export_expression(id, items),
            // let x = value
            dir::Expression::Let {
                declarators,
                is_ambient,
                ..
            } => self.walk_let_expression(id, declarators, *is_ambient),
            // using x = value
            dir::Expression::Using { declarators, .. } => self.walk_using_expression(id, declarators),
            // let x = value else fallback
            dir::Expression::LetElse {
                declarator,
                else_branch,
                ..
            } => self.walk_let_else_expression(id, *declarator, *else_branch),
            // if condition { then } else { otherwise }
            dir::Expression::If {
                condition,
                then_expression,
                else_expression,
                ..
            } => self.walk_if_expression(id, condition, *then_expression, *else_expression),
            // while condition { body }
            dir::Expression::While {
                condition, body, ..
            } => self.walk_while_expression(id, None, *condition, *body),
            // for item of iterator { body }
            dir::Expression::ForEach {
                operator,
                binding,
                iterator,
                body,
                ..
            } => self.walk_for_each_expression(id, None, *operator, binding, *iterator, *body),
            // for (initialization; condition; increment) { body }
            dir::Expression::For {
                initialization,
                condition,
                increment,
                body,
            } => self.walk_for_expression(id, None, *initialization, *condition, *increment, *body),
            // loop { body }
            dir::Expression::Loop { body } => self.walk_loop_expression(id, None, *body),
            // try body catch error finally cleanup
            dir::Expression::Try {
                body,
                catch,
                finally,
            } => self.walk_try_expression(id, *body, *catch, *finally),
            // match value { case pattern => body }
            dir::Expression::Match { value, cases, .. } => self.walk_match_expression(id, *value, cases),
            // break value
            dir::Expression::Break { label, value } => self.walk_break_expression(id, *label, *value),
            // continue
            dir::Expression::Continue { label } => self.walk_continue_expression(id, *label),
            // await value
            dir::Expression::Await {
                expression: awaited,
            } => self.walk_await_expression(id, *awaited),
            // throw value
            dir::Expression::Throw { value } => self.walk_throw_expression(id, *value),
            // return value
            dir::Expression::Return { value } => self.walk_return_expression(id, *value),
            // yield value
            dir::Expression::Yield { cardinality, value } => self.walk_yield_expression(id, *cardinality, *value),
            // value
            dir::Expression::Identifier { name } => self.walk_identifier_expression(id, *name),
            // this
            dir::Expression::This => self.walk_this_expression(id),
            // 1, "text", true
            dir::Expression::ScalarLiteral(value) => self.walk_scalar_literal_expression(id, value),
            // super
            dir::Expression::Super => self.walk_super_expression(id),
            // import.meta
            dir::Expression::ImportMeta => self.walk_import_meta_expression(id),
            // #name
            dir::Expression::PrivateIdentifier { .. } => Ok(()),
            // debugger
            dir::Expression::Debugger => Ok(()),
            // missing expression
            dir::Expression::Missing => Ok(()),
            // stub expression
            dir::Expression::Stub => Ok(()),
            // ignore damaged syntax
            dir::Expression::Error => Ok(()),
            // namespace.value<T>
            dir::Expression::QualifiedReference {
                path,
                generic_arguments,
            } => self.walk_qualified_reference_expression(id, path, generic_arguments),
            // start..end
            dir::Expression::RangeExpression {
                start,
                end,
                end_kind,
            } => self.walk_range_expression(id, *start, *end, *end_kind),
            // `text ${value}`
            dir::Expression::TemplateExpression { value } => self.walk_template_expression(id, value),
            // tag<T>`text ${value}`
            dir::Expression::TaggedTemplateExpression {
                tag,
                generic_arguments,
                value,
            } => self.walk_tagged_template_expression(id, *tag, generic_arguments, value),
            // [a, b, c]
            dir::Expression::ArrayExpression { elements } => self.walk_array_expression(id, elements),
            // [value; length]
            dir::Expression::FixedArrayExpression { value, length } => self.walk_fixed_array_expression(id, *value, *length),
            // [a, label: b, ...rest]
            dir::Expression::TupleExpression { elements } => self.walk_tuple_expression(id, elements),
            // a, b, c
            dir::Expression::SequenceExpression { expressions } => self.walk_sequence_expression(id, expressions),
            // { key: value }
            dir::Expression::ObjectExpression { properties } => {
                self.walk_object_expression(id, properties)
            }
            // Type { key: value }
            dir::Expression::StructExpression { ty, properties } => self.walk_struct_expression(id, *ty, properties),
            // jsx like tree expression
            dir::Expression::TreeExpression {
                left,
                generic_arguments,
                arguments,
                elements,
            } => self.walk_tree_expression(id, *left, generic_arguments, arguments.as_deref(), elements.as_deref()),
            // (value)
            dir::Expression::Parenthesized { expression: child } => self.walk_parenthesized_expression(id, *child),
            // type T
            dir::Expression::Type { value } => self.walk_type_value_expression(id, *value),
            // comptime value
            dir::Expression::Comptime { body } => self.walk_comptime_expression(id, *body),
            // value as T
            dir::Expression::As {
                expression: child,
                target_type,
            } => self.walk_as_expression(id, *child, *target_type),
            // value satisfies T
            dir::Expression::Satisfies {
                expression: child,
                target_type,
            } => self.walk_satisfies_expression(id, *child, *target_type),
            // value is T
            dir::Expression::Is { value, target_type } => self.walk_is_expression(id, *value, *target_type),
            // value instanceof Target
            dir::Expression::InstanceOf { value, target } => self.walk_instanceof_expression(id, *value, *target),
            // !value, ++value
            dir::Expression::Unary { operator, right } => self.walk_unary_expression(id, *operator, *right),
            // ^value
            dir::Expression::MoveOf { right, .. } => self.walk_move_of_expression(id, *right),
            // &value
            dir::Expression::BorrowOf {
                mutability, right, ..
            } => self.walk_borrow_of_expression(id, *mutability, *right),
            // value.member
            dir::Expression::Member { left, name }
            // value.#member
            | dir::Expression::PrivateMember { left, name } => self.walk_member_expression(id, *left, *name),
            // value[index]
            dir::Expression::Index { left, index, .. } => self.walk_index_expression(id, *left, *index),
            // value<T>
            dir::Expression::Instantiation {
                left,
                generic_arguments,
            } => self.walk_instantiation_expression(id, *left, generic_arguments),
            // callee<T>(argument)
            dir::Expression::Call {
                left,
                generic_arguments,
                arguments,
                ..
            } => self.walk_call_expression(id, *left, generic_arguments, arguments),
            // new Type<T>(argument)
            dir::Expression::New { ty, arguments } => {
                self.walk_construct_expression(id, *ty, arguments)
            }
            // new? Type<T>(argument)
            dir::Expression::NewMaybe { ty, arguments } => {
                self.walk_maybe_construct_expression(id, *ty, arguments)
            }
            // await? value
            dir::Expression::AwaitMaybe {
                expression: awaited,
            } => self.walk_await_maybe_expression(id, *awaited),
            // await! value
            dir::Expression::AwaitMust {
                expression: awaited,
            } => self.walk_await_must_expression(id, *awaited),
            // value?
            dir::Expression::Maybe { left, .. } => self.walk_maybe_expression(id, *left),
            // value!
            dir::Expression::Must { left, .. } => self.walk_must_expression(id, *left),
            // left + right
            dir::Expression::Binary {
                left,
                operator,
                right,
            } => self.walk_binary_expression(id, *left, *operator, *right),
            // target = value
            dir::Expression::Assign {
                left,
                operator,
                right,
            } => self.walk_assign_expression(id, *left, *operator, *right),
        }
    }

    /// Walk one where clause.
    ///
    /// Example:
    /// ```ds
    /// where T: Serializable
    /// ```
    pub(in crate::check) fn walk_where_clause(
        &mut self,
        id: dir::LocalNodeId<dir::WhereClause>,
        where_clause: &dir::WhereClause,
    ) -> CompilerResult<()> {
        // walk both constraint operands
        self.walk_type_expression(where_clause.left, self.tree.get(where_clause.left))?;
        self.walk_type_expression(where_clause.right, self.tree.get(where_clause.right))?;

        // constrain the left type by the right type
        let left = self.node_type_operand(where_clause.left)?;
        let right = self.node_type_operand(where_clause.right)?;
        let origin = Origin::Node(id.into_global_any(self.module));
        let condition = self.active_static_guard();
        // attach generic bounds for member lookup
        let left_source = where_clause.left.into_global_any(self.module);
        if let Some(parameter) = self.where_clause_generic_parameter(left_source, left) {
            self.check
                .inference
                .constrain_generic_type_parameter(parameter, right)?;

            return Ok(());
        }

        self.check
            .constrain_type(origin, TypeRelation::Satisfies, left, right, condition);

        Ok(())
    }

    /// Return the generic type parameter constrained by one where-clause left side.
    fn where_clause_generic_parameter(
        &self,
        source: dir::GlobalNodeIdAny,
        operand: TypeOperand,
    ) -> Option<GenericParameterId> {
        // read resolved generic parameter names
        if let Some(symbol) = self
            .check
            .inference
            .name(source)
            .map(|resolution| resolution.symbol())
        {
            if self.check.symbol_kind(symbol) != dir::SymbolKind::GenericTypeParameter {
                return None;
            }

            return self.check.inference.generic_parameter_by_symbol(symbol);
        }

        // read direct parameter operands
        let TypeOperand::Term(term) = operand else {
            return None;
        };
        let TypeTerm::Parameter(parameter) = self.check.inference.term(term) else {
            return None;
        };

        Some(*parameter)
    }

    /// Walk one import expression.
    ///
    /// Example:
    /// ```ds
    /// import { item } from "module"
    /// ```
    fn walk_import_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        items: Option<&[dir::LocalNodeId<dir::DependencyItem>]>,
    ) -> CompilerResult<()> {
        self.constrain_node_type_term(id, TypeTerm::Literal(TypeLiteralTerm::Void))?;

        // walk imported item bindings
        let Some(items) = items else {
            return Ok(());
        };
        for item in items {
            self.walk_dependency_item(*item, self.tree.get(*item))?;
        }

        Ok(())
    }

    /// Walk one export expression.
    ///
    /// Example:
    /// ```ds
    /// export { item } from "module"
    /// ```
    fn walk_export_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        items: &[dir::LocalNodeId<dir::DependencyItem>],
    ) -> CompilerResult<()> {
        self.constrain_node_type_term(id, TypeTerm::Literal(TypeLiteralTerm::Void))?;

        // walk exported item bindings
        for item in items {
            self.walk_dependency_item(*item, self.tree.get(*item))?;
        }

        Ok(())
    }

    /// Walk one let expression.
    ///
    /// Example:
    /// ```ds
    /// let value: number = 1
    /// ```
    fn walk_let_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        declarators: &[dir::LocalNodeId<dir::Declarator>],
        is_ambient: bool,
    ) -> CompilerResult<()> {
        self.constrain_node_type_term(id, TypeTerm::Literal(TypeLiteralTerm::Void))?;

        // walk and assign each declared binding
        for declarator in declarators {
            self.walk_declarator(*declarator, self.tree.get(*declarator))?;
            if is_ambient {
                self.mark_bindings_assigned(self.tree.get(*declarator).pattern.into_any());
            } else {
                self.mark_declarator_assigned(self.tree.get(*declarator));
            }
        }

        Ok(())
    }

    /// Walk one using expression.
    ///
    /// Example:
    /// ```ds
    /// using resource = open()
    /// ```
    fn walk_using_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        declarators: &[dir::LocalNodeId<dir::Declarator>],
    ) -> CompilerResult<()> {
        self.constrain_node_type_term(id, TypeTerm::Literal(TypeLiteralTerm::Void))?;

        // walk and assign each resource binding
        for declarator in declarators {
            self.walk_declarator(*declarator, self.tree.get(*declarator))?;
            self.mark_declarator_assigned(self.tree.get(*declarator));
        }

        Ok(())
    }

    /// Walk one let else expression.
    ///
    /// Example:
    /// ```ds
    /// let Some(value) = option else return
    /// ```
    fn walk_let_else_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        declarator: dir::LocalNodeId<dir::Declarator>,
        else_branch: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        self.constrain_node_type_term(id, TypeTerm::Literal(TypeLiteralTerm::Void))?;
        self.walk_declarator(declarator, self.tree.get(declarator))?;

        // check else branch without leaking flow
        let before_else = self.fork_flow();
        self.walk_expression(else_branch, self.tree.get(else_branch))?;
        if self.expression_can_complete_normally(else_branch) {
            self.check.report_invalid_control_flow(
                self.module,
                else_branch.into_any(),
                "let else fallback must not fall through",
            );
        }
        self.restore_flow(before_else);

        // enter successful pattern flow
        self.mark_declarator_assigned(self.tree.get(declarator));
        self.narrow_declarator_pattern_success(declarator)?;

        Ok(())
    }

    /// Walk one break expression.
    ///
    /// Example:
    /// ```ds
    /// break value
    /// ```
    fn walk_break_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        label: Option<dir::StringId>,
        value: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<()> {
        self.constrain_node_type_term(id, TypeTerm::Literal(TypeLiteralTerm::Never))?;

        // walk optional break value
        let value = if let Some(value) = value {
            self.walk_expression(value, self.tree.get(value))?;

            Some(self.node_type_operand(value)?)
        } else {
            None
        };
        self.break_to_control_target(id.into_any(), label, value);

        Ok(())
    }

    /// Walk one continue expression.
    ///
    /// Example:
    /// ```ds
    /// continue
    /// ```
    fn walk_continue_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        label: Option<dir::StringId>,
    ) -> CompilerResult<()> {
        self.constrain_node_type_term(id, TypeTerm::Literal(TypeLiteralTerm::Never))?;
        self.continue_to_control_target(id.into_any(), label);

        Ok(())
    }

    /// Walk one await expression.
    ///
    /// Example:
    /// ```ds
    /// await task
    /// ```
    fn walk_await_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        awaited: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        self.validate_await_context(id.into_any());
        self.walk_expression(awaited, self.tree.get(awaited))?;

        let value = self.node_type_operand(awaited)?;
        let await_term = self.check.inference.push_term(AwaitTerm {
            source: id.into_global_any(self.module),
            value: value.into(),
        });
        let term = TypeTerm::Await(await_term);
        self.constrain_node_type_term(id, term)?;

        Ok(())
    }

    /// Walk one throw expression.
    ///
    /// Example:
    /// ```ds
    /// throw error
    /// ```
    fn walk_throw_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        self.constrain_node_type_term(id, TypeTerm::Literal(TypeLiteralTerm::Never))?;
        self.walk_expression(value, self.tree.get(value))?;

        Ok(())
    }

    /// Walk one return expression.
    ///
    /// Example:
    /// ```ds
    /// return value
    /// ```
    fn walk_return_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        value: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<()> {
        self.constrain_node_type_term(id, TypeTerm::Literal(TypeLiteralTerm::Never))?;

        // constrain explicit return value
        if let Some(value) = value {
            self.walk_expression(value, self.tree.get(value))?;

            let value = self.node_type_operand(value)?;
            self.constrain_return_value(id.into_any(), value);
        }
        // constrain implicit void return
        else {
            self.constrain_void_return(id.into_any());
        }

        Ok(())
    }

    /// Walk one yield expression.
    ///
    /// Example:
    /// ```ds
    /// yield value
    /// ```
    fn walk_yield_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        cardinality: dir::YieldCardinality,
        value: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<()> {
        if let Some(value) = value {
            self.walk_expression(value, self.tree.get(value))?;
        }

        let source = id.into_global_any(self.module);
        let origin = Origin::Node(source);
        let value_type = value
            .map(|value| self.node_type_operand(value))
            .transpose()?;
        let delegate_return_target = if cardinality == dir::YieldCardinality::Generator {
            let target = self.check.push_type_variable(self.module, origin);

            Some(TypeOperand::from(target))
        } else {
            None
        };
        let yield_term = self.check.inference.push_term(YieldTerm {
            source,
            value: value_type,
            yield_target: self.current_yield_target(),
            resume_target: self.current_resume_target(),
            delegate_return_target,
            cardinality,
        });
        let term = TypeTerm::Yield(yield_term);
        self.constrain_node_type_term(id, term)?;
        self.constrain_yield_value(
            id.into_any(),
            cardinality,
            value_type,
            delegate_return_target,
        );

        Ok(())
    }

    /// Walk one identifier expression.
    ///
    /// Example:
    /// ```ds
    /// value
    /// ```
    fn walk_identifier_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        name: dir::StringId,
    ) -> CompilerResult<()> {
        if let Some(operand) = self.walk_identifier_reference_term(id, name)? {
            self.constrain_node_type(id, operand)?;
        } else {
            self.constrain_node_type_term(id, TypeTerm::Literal(TypeLiteralTerm::Error))?;
        }

        Ok(())
    }

    /// Walk one this expression.
    ///
    /// Example:
    /// ```ds
    /// this
    /// ```
    fn walk_this_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        if let Some(term) = self.this_receiver_type_term(id.into_global_any(self.module))? {
            self.constrain_node_type_term(id, term)?;
        }

        Ok(())
    }

    /// Walk one scalar literal expression.
    ///
    /// Example:
    /// ```ds
    /// 1
    /// ```
    fn walk_scalar_literal_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        value: &dir::ScalarLiteral,
    ) -> CompilerResult<()> {
        let term = match value {
            // /pattern/
            dir::ScalarLiteral::RegexString { .. } => {
                let symbol = self.check.language_symbol(dir::LanguageItem::RegExp);

                TypeTerm::Reference {
                    origin: Origin::Node(id.into_global_any(self.module)),
                    symbol,
                    arguments: Vec::new().into(),
                }
            }
            // scalar literal
            _ => TypeTerm::Literal(TypeLiteralTerm::Scalar(value.clone())),
        };
        self.constrain_node_type_term(id, term)?;
        if !matches!(value, dir::ScalarLiteral::RegexString { .. }) {
            let term = StaticTerm::Literal(dir::StaticTerm::ScalarLiteral {
                value: value.clone(),
            });
            let condition = self.active_static_guard();
            self.constrain_node_static(id, term, condition)?;
        }

        Ok(())
    }

    /// Walk one super expression.
    ///
    /// Example:
    /// ```ds
    /// super
    /// ```
    fn walk_super_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        let source = id.into_global_any(self.module);
        let receiver = self
            .resolve_this_receiver(source)?
            .map(|receiver| receiver.ty);
        let super_term = self
            .check
            .inference
            .push_term(SuperTerm { source, receiver });
        let term = TypeTerm::Super(super_term);
        self.constrain_node_type_term(id, term)?;

        Ok(())
    }

    /// Walk one import meta expression.
    ///
    /// Example:
    /// ```ds
    /// import.meta
    /// ```
    fn walk_import_meta_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        let import_meta = self.check.inference.push_term(ImportMetaTerm {
            source: id.into_global_any(self.module),
        });
        let term = TypeTerm::ImportMeta(import_meta);
        self.constrain_node_type_term(id, term)?;

        Ok(())
    }

    /// Walk one qualified reference expression.
    ///
    /// Example:
    /// ```ds
    /// namespace.value<T>
    /// ```
    fn walk_qualified_reference_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        path: &dir::Path,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<()> {
        if let Some(operand) = self.walk_qualified_reference_term(id, path, generic_arguments)? {
            self.constrain_node_type(id, operand)?;
        }

        Ok(())
    }

    /// Walk one range expression.
    ///
    /// Example:
    /// ```ds
    /// start..end
    /// ```
    fn walk_range_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        start: Option<dir::LocalNodeId<dir::Expression>>,
        end: Option<dir::LocalNodeId<dir::Expression>>,
        end_kind: dir::RangeEnd,
    ) -> CompilerResult<()> {
        // walk range boundaries
        if let Some(start) = start {
            self.walk_expression(start, self.tree.get(start))?;
        }
        if let Some(end) = end {
            self.walk_expression(end, self.tree.get(end))?;
        }

        let start_variable = start
            .map(|start| self.node_type_operand(start))
            .transpose()?;
        let end_variable = end.map(|end| self.node_type_operand(end)).transpose()?;
        let range = self.check.inference.push_term(RangeTerm {
            source: id.into_global_any(self.module),
            start: start_variable,
            end: end_variable,
            end_kind,
        });
        let term = TypeTerm::RangeValue(range);
        self.constrain_node_type_term(id, term)?;

        Ok(())
    }

    /// Walk one template expression.
    ///
    /// Example:
    /// ```ds
    /// `hello ${name}`
    /// ```
    fn walk_template_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        value: &dir::TemplateLiteral,
    ) -> CompilerResult<()> {
        self.walk_template_literal(value)?;

        let (strings, spans) = self.template_segments(value)?;
        let template = self.check.inference.push_term(TemplateTerm {
            source: id.into_global_any(self.module),
            strings,
            spans,
        });
        let term = TypeTerm::Template(template);
        self.constrain_node_type_term(id, term)?;

        Ok(())
    }

    /// Walk one tagged template expression.
    ///
    /// Example:
    /// ```ds
    /// tag<T>`hello ${name}`
    /// ```
    fn walk_tagged_template_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        tag: dir::LocalNodeId<dir::Expression>,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        value: &dir::TemplateLiteral,
    ) -> CompilerResult<()> {
        self.walk_expression(tag, self.tree.get(tag))?;

        self.walk_template_literal(value)?;

        let (strings, spans) = self.template_segments(value)?;
        let generic_arguments_term = self.walk_generic_arguments(generic_arguments)?;
        let tag_variable = self.node_type_operand(tag)?;
        let template = self.check.inference.push_term(TaggedTemplateTerm {
            source: id.into_global_any(self.module),
            tag: tag_variable,
            generic_arguments: generic_arguments_term,
            strings,
            spans,
        });
        let term = TypeTerm::TaggedTemplate(template);
        self.constrain_node_type_term(id, term)?;

        Ok(())
    }

    /// Walk one array expression.
    ///
    /// Example:
    /// ```ds
    /// [a, b, c]
    /// ```
    fn walk_array_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        elements: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<()> {
        // walk array elements
        for element in elements {
            self.walk_argument(*element, self.tree.get(*element))?;
        }

        let elements = self.argument_value_type_operands(elements)?;
        let element = match elements.as_slice() {
            // [] starts open and is constrained by later context
            [] => {
                let source = id.into_global_any(self.module);
                let variable = self
                    .check
                    .push_type_variable(self.module, Origin::Node(source));

                variable.into()
            }
            // [value]
            [element] => *element,
            // [left, right]
            _ => {
                let term = TypeTerm::Union { elements };
                self.check.inference.push_term(term).into()
            }
        };
        let source = id.into_global_any(self.module);
        let term = self.language_type_reference(
            source,
            dir::LanguageItem::Array,
            vec![GenericArgument::Type(element)],
        );
        self.constrain_node_type_term(id, term)?;

        Ok(())
    }

    /// Walk one fixed array expression.
    ///
    /// Example:
    /// ```ds
    /// [value; 4]
    /// ```
    fn walk_fixed_array_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
        length: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        self.walk_expression(value, self.tree.get(value))?;

        // check fixed array length in static context
        let before_length = self.fork_flow();
        self.walk_static_expression(length)?;
        self.restore_flow(before_length);

        let condition = self.active_static_guard();
        let source = id.into_global_any(self.module);
        let element = self.node_type_operand(value)?;
        let length_variable = self.static_expression_operand(length, condition)?;
        let term = self.language_type_reference(
            source,
            dir::LanguageItem::FixedArray,
            vec![
                GenericArgument::Type(element),
                GenericArgument::Static(length_variable.into()),
            ],
        );
        self.constrain_node_type_term(id, term)?;

        Ok(())
    }

    /// Walk one tuple expression.
    ///
    /// Example:
    /// ```ds
    /// [name: value, ...rest]
    /// ```
    fn walk_tuple_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        elements: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<()> {
        // walk tuple elements
        for element in elements {
            self.walk_argument(*element, self.tree.get(*element))?;
        }

        let elements_term = self.tuple_elements(elements)?;
        let term = TypeTerm::Tuple {
            form: dir::TupleForm::Tuple,
            elements: elements_term.into(),
        };
        self.constrain_node_type_term(id, term)?;

        Ok(())
    }

    /// Walk one sequence expression.
    ///
    /// Example:
    /// ```ds
    /// a, b, c
    /// ```
    fn walk_sequence_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        expressions: &[dir::LocalNodeId<dir::Expression>],
    ) -> CompilerResult<()> {
        // walk sequence operands
        for expression in expressions {
            self.walk_expression(*expression, self.tree.get(*expression))?;
        }

        match expressions.last() {
            Some(expression) => {
                let operand = self.node_type_operand(*expression)?;
                self.constrain_node_type(id, operand)?;
            }
            None => {
                self.constrain_node_type_term(id, TypeTerm::Literal(TypeLiteralTerm::Void))?;
            }
        }

        Ok(())
    }

    /// Walk one struct expression.
    ///
    /// Example:
    /// ```ds
    /// Point { x: 1, y: 2 }
    /// ```
    fn walk_struct_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        properties: &[dir::LocalNodeId<dir::Property>],
    ) -> CompilerResult<()> {
        self.walk_type_expression(ty, self.tree.get(ty))?;

        // walk property values
        for property in properties {
            self.walk_property(*property, self.tree.get(*property))?;
        }

        let source = id.into_global_any(self.module);
        let owner = self.node_type_operand(ty)?;
        let members = self.shape_members(properties, false)?;
        let shape = self.check.push_shape_type(members);
        let shape = self.check.inference.push_term(shape);
        let condition = self.active_static_guard();
        self.check.constrain_type(
            Origin::Node(source),
            TypeRelation::Assignable,
            shape,
            owner,
            condition,
        );
        self.constrain_node_type(id, owner)?;

        Ok(())
    }

    /// Walk one tree expression.
    ///
    /// Example:
    /// ```ds
    /// View(title = "hello") { Text("world") }
    /// ```
    fn walk_tree_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        left: Option<dir::LocalNodeId<dir::Expression>>,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        arguments: Option<&[dir::LocalNodeId<dir::Argument>]>,
        elements: Option<&[dir::LocalNodeId<dir::Argument>]>,
    ) -> CompilerResult<()> {
        // walk tree tag and inputs
        if let Some(left) = left {
            self.walk_expression(left, self.tree.get(left))?;
        }
        if let Some(arguments) = arguments {
            for argument in arguments {
                self.walk_argument(*argument, self.tree.get(*argument))?;
            }
        }
        if let Some(elements) = elements {
            for element in elements {
                self.walk_argument(*element, self.tree.get(*element))?;
            }
        }

        let tag = left.map(|left| self.node_type_operand(left)).transpose()?;
        let generic_argument_terms = self.walk_generic_arguments(generic_arguments)?;
        let argument_types = arguments
            .map(|arguments| self.argument_value_type_operands(arguments))
            .transpose()?
            .unwrap_or_default();
        let element_types = elements
            .map(|elements| self.argument_value_type_operands(elements))
            .transpose()?
            .unwrap_or_default();
        let tree_term = self.check.inference.push_term(TreeTerm {
            source: id.into_global_any(self.module),
            tag,
            generic_arguments: generic_argument_terms,
            arguments: argument_types,
            elements: element_types,
        });
        let term = TypeTerm::Tree(tree_term);
        self.constrain_node_type_term(id, term)?;

        Ok(())
    }

    /// Walk one parenthesized expression.
    ///
    /// Example:
    /// ```ds
    /// (value)
    /// ```
    fn walk_parenthesized_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        child: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        self.walk_expression(child, self.tree.get(child))?;

        let child_type = self.node_type_operand(child)?;
        self.constrain_node_type(id, child_type)?;

        Ok(())
    }

    /// Walk one type value expression.
    ///
    /// Example:
    /// ```ds
    /// type T
    /// ```
    fn walk_type_value_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        self.walk_type_expression(value, self.tree.get(value))?;

        let ty = self.node_type_operand(value)?;
        let type_value = self.check.inference.push_term(TypeValueTerm {
            source: id.into_global_any(self.module),
            ty,
        });
        let term = TypeTerm::TypeValue(type_value);
        self.constrain_node_type_term(id, term)?;

        Ok(())
    }

    /// Walk one comptime expression.
    ///
    /// Example:
    /// ```ds
    /// comptime value
    /// ```
    fn walk_comptime_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        self.walk_expression(body, self.tree.get(body))?;

        let body_type = self.node_type_operand(body)?;
        self.constrain_node_type(id, body_type)?;

        Ok(())
    }

    /// Walk one cast expression.
    ///
    /// Example:
    /// ```ds
    /// value as T
    /// ```
    fn walk_as_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        child: dir::LocalNodeId<dir::Expression>,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        // build const assertion without contextual widening
        if matches!(self.tree.get(target_type), dir::TypeExpression::Const) {
            self.walk_const_assertion_expression(id, child)?;

            return Ok(());
        }

        self.walk_expression(child, self.tree.get(child))?;
        self.walk_type_expression(target_type, self.tree.get(target_type))?;

        let child_type = self.node_type_operand(child)?;
        let target_type = self.node_type_operand(target_type)?;
        let origin = Origin::Node(id.into_global_any(self.module));
        let condition = self.active_static_guard();
        self.constrain_node_type(id, target_type)?;
        self.check.constrain_coercion(
            origin,
            TypeRelation::Castable,
            child_type,
            target_type,
            condition,
            dir::CastOrigin::Explicit,
        );

        Ok(())
    }

    /// Walk one satisfies expression.
    ///
    /// Example:
    /// ```ds
    /// value satisfies T
    /// ```
    fn walk_satisfies_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        child: dir::LocalNodeId<dir::Expression>,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        self.walk_expression(child, self.tree.get(child))?;
        self.walk_type_expression(target_type, self.tree.get(target_type))?;

        let child_type = self.node_type_operand(child)?;
        let target_type = self.node_type_operand(target_type)?;
        let origin = Origin::Node(id.into_global_any(self.module));
        let condition = self.active_static_guard();
        self.constrain_node_type(id, child_type)?;
        self.check.constrain_type(
            origin,
            TypeRelation::Satisfies,
            child_type,
            target_type,
            condition,
        );

        Ok(())
    }

    /// Walk one is expression.
    ///
    /// Example:
    /// ```ds
    /// value is T
    /// ```
    fn walk_is_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        self.constrain_node_type_term(id, TypeTerm::Literal(TypeLiteralTerm::boolean()))?;
        self.walk_expression(value, self.tree.get(value))?;
        self.walk_type_expression(target_type, self.tree.get(target_type))?;

        Ok(())
    }

    /// Walk one instanceof expression.
    ///
    /// Example:
    /// ```ds
    /// value instanceof Target
    /// ```
    fn walk_instanceof_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
        target: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        self.walk_expression(value, self.tree.get(value))?;
        self.walk_expression(target, self.tree.get(target))?;

        let value_variable = self.node_type_operand(value)?;
        let target_variable = self.node_type_operand(target)?;
        let instance = self.check.inference.push_term(InstanceCheckTerm {
            source: id.into_global_any(self.module),
            value: value_variable,
            target: target_variable,
        });
        let term = TypeTerm::InstanceCheck(instance);
        self.constrain_node_type_term(id, term)?;

        Ok(())
    }

    /// Walk one unary expression.
    ///
    /// Example:
    /// ```ds
    /// !value
    /// ```
    fn walk_unary_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        operator: dir::UnaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        self.walk_expression(right, self.tree.get(right))?;

        let is_update = matches!(
            operator,
            dir::UnaryOperator::PostIncrement
                | dir::UnaryOperator::PostDecrement
                | dir::UnaryOperator::PreIncrement
                | dir::UnaryOperator::PreDecrement
        );
        let receiver = self.node_type_operand(right)?;
        let operator = self.check.inference.push_term(OperatorTerm {
            source: id.into_global_any(self.module),
            kind: OperatorTermKind::Unary(operator),
            receiver,
            argument: None,
        });
        let term = TypeTerm::Operator(operator);
        self.constrain_node_type_term(id, term)?;

        // write after reading the updated place
        if is_update && let Some(place) = self.place_term(right)? {
            let condition = self.active_static_guard();
            self.clear_mutated_expression_narrowings(right);
            self.mark_place_assigned(place);
            self.check.constrain_writable_place(place, condition);
        }

        Ok(())
    }

    /// Walk one move expression.
    ///
    /// Example:
    /// ```ds
    /// ^value
    /// ```
    fn walk_move_of_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        self.walk_expression(right, self.tree.get(right))?;

        let form = self.check.inference.push_term(FormTerm::Owned);
        let term = TypeTerm::Form {
            form,
            payload: self.node_type_operand(right)?.into(),
        };
        self.constrain_node_type_term(id, term)?;

        Ok(())
    }

    /// Walk one borrow expression.
    ///
    /// Example:
    /// ```ds
    /// &mut value
    /// ```
    fn walk_borrow_of_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        mutability: Option<dir::Mutability>,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        self.walk_expression(right, self.tree.get(right))?;

        let source = id.into_global_any(self.module);
        let payload = self.node_type_operand(right)?.into();
        let term = self.borrowed_form_type(self.module, source, mutability, payload)?;
        self.constrain_node_type_term(id, term)?;

        Ok(())
    }

    /// Walk one member expression.
    ///
    /// Example:
    /// ```ds
    /// value.member
    /// ```
    fn walk_member_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        name: Option<dir::StringId>,
    ) -> CompilerResult<()> {
        // walk namespace paths before runtime member access
        if let Some(path) = self.member_expression_path(id) {
            let is_namespace_path = self.walk_namespace_path_reference_term(id, &path)?;

            if is_namespace_path {
                return Ok(());
            }
        }

        self.walk_expression(left, self.tree.get(left))?;

        // set member access when the key is present
        if let Some(name) = name {
            if let Some(narrowed) = self.flow_path_narrowing(id) {
                self.constrain_node_type(id, narrowed)?;
            } else {
                let owner = self.expression_type_operand(left)?;
                let member = self.check.inference.push_term(MemberTerm {
                    origin: Origin::Node(id.into_global_any(self.module)),
                    receiver: MemberReceiver::Value(owner),
                    key: dir::StaticKey::Name(name),
                    arguments: Vec::new().into(),
                });
                self.constrain_node_type_term(id, TypeTerm::Member(member))?;
            }
        }

        Ok(())
    }

    /// Return one static member path represented by member expression syntax.
    fn member_expression_path(&self, id: dir::LocalNodeId<dir::Expression>) -> Option<dir::Path> {
        let mut suffix = SmallVec::<[dir::StringId; 1]>::new();
        let mut current = id;

        loop {
            match self.tree.get(current) {
                // collect the path root
                dir::Expression::Identifier { name } => {
                    suffix.push(*name);
                    suffix.reverse();

                    return (suffix.len() > 1).then_some(dir::Path { segments: suffix });
                }

                // collect an already path-shaped root
                dir::Expression::QualifiedReference { path, .. } => {
                    let mut segments = path.segments.clone();
                    suffix.reverse();
                    segments.extend(suffix);

                    return (segments.len() > 1).then_some(dir::Path { segments });
                }

                // extend through one member segment
                dir::Expression::Member {
                    left,
                    name: Some(name),
                } => {
                    suffix.push(*name);
                    current = *left;
                }

                // reject dynamic member syntax
                _ => return None,
            }
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
        id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        index: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<()> {
        self.walk_expression(left, self.tree.get(left))?;

        // walk index operand
        if let Some(index) = index {
            self.walk_expression(index, self.tree.get(index))?;
        }

        // set index access when the index is present
        if let Some(index) = index {
            if let Some(narrowed) = self.flow_path_narrowing(id) {
                self.constrain_node_type(id, narrowed)?;
            } else {
                let receiver = self.expression_type_operand(left)?;
                let index_type = self.node_type_operand(index)?;
                let kind = if matches!(
                    self.tree.get(index),
                    dir::Expression::RangeExpression { .. }
                ) {
                    IndexKind::Slice
                } else {
                    IndexKind::Element
                };
                let index_term = self.check.inference.push_term(IndexTerm {
                    source: id.into_global_any(self.module),
                    kind,
                    receiver,
                    index: index_type,
                    key: self.tree.get(index).static_key(),
                });
                self.constrain_node_type_term(id, TypeTerm::Index(index_term))?;
            }
        }

        Ok(())
    }

    /// Walk one generic instantiation expression.
    ///
    /// Example:
    /// ```ds
    /// value<T>
    /// ```
    fn walk_instantiation_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<()> {
        let source = id.into_global_any(self.module);
        let left_source = left.into_global_any(self.module);
        self.walk_expression(left, self.tree.get(left))?;

        let generic_owner = self
            .check
            .inference
            .name(left_source)
            .map(|resolution| resolution.symbol());
        let arguments = self.walk_generic_arguments(generic_arguments)?;
        if let Some(symbol) = generic_owner {
            let term = TypeTerm::Reference {
                origin: Origin::Node(source),
                symbol,
                arguments,
            };
            self.constrain_node_type_term(id, term)?;
        } else {
            let operand = self.node_type_operand(left)?;
            self.constrain_node_type(id, operand)?;
        }

        Ok(())
    }

    /// Walk one maybe await expression.
    ///
    /// Example:
    /// ```ds
    /// await? task
    /// ```
    fn walk_await_maybe_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        awaited: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        self.validate_await_context(id.into_any());
        self.walk_expression(awaited, self.tree.get(awaited))?;

        let source = id.into_global_any(self.module);
        let value = self.node_type_operand(awaited)?;
        let await_term = self.check.inference.push_term(AwaitTerm {
            source,
            value: value.into(),
        });
        let awaited_type = self.check.inference.push_term(TypeTerm::Await(await_term));
        let term = TryTerm {
            source,
            value: awaited_type.into(),
            kind: TryTermKind::Maybe,
        };
        let value = term.value;
        let tried = self.check.inference.push_term(term);
        self.constrain_node_type_term(id, TypeTerm::Try(tried))?;
        self.propagate_try(id.into_any(), value);

        Ok(())
    }

    /// Walk one must await expression.
    ///
    /// Example:
    /// ```ds
    /// await! task
    /// ```
    fn walk_await_must_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        awaited: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        self.validate_await_context(id.into_any());
        self.walk_expression(awaited, self.tree.get(awaited))?;

        let source = id.into_global_any(self.module);
        let value = self.node_type_operand(awaited)?;
        let await_term = self.check.inference.push_term(AwaitTerm {
            source,
            value: value.into(),
        });
        let awaited_type = self.check.inference.push_term(TypeTerm::Await(await_term));
        let term = TryTerm {
            source,
            value: awaited_type.into(),
            kind: TryTermKind::Must,
        };
        let term = self.check.inference.push_term(term);
        self.constrain_node_type_term(id, TypeTerm::Try(term))?;

        Ok(())
    }

    /// Walk one maybe propagation expression.
    ///
    /// Example:
    /// ```ds
    /// value?
    /// ```
    fn walk_maybe_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        self.walk_expression(left, self.tree.get(left))?;

        let tried = TryTerm {
            source: id.into_global_any(self.module),
            value: self.node_type_operand(left)?.into(),
            kind: TryTermKind::Maybe,
        };
        let value = tried.value;
        let term = self.check.inference.push_term(tried);
        self.constrain_node_type_term(id, TypeTerm::Try(term))?;
        self.propagate_try(id.into_any(), value);

        Ok(())
    }

    /// Walk one must propagation expression.
    ///
    /// Example:
    /// ```ds
    /// value!
    /// ```
    fn walk_must_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        self.walk_expression(left, self.tree.get(left))?;

        let value = self.node_type_operand(left)?;
        let term = self.check.inference.push_term(TryTerm {
            source: id.into_global_any(self.module),
            value: value.into(),
            kind: TryTermKind::Must,
        });
        let term = TypeTerm::Try(term);
        self.constrain_node_type_term(id, term)?;

        Ok(())
    }

    /// Walk one binary expression.
    ///
    /// Example:
    /// ```ds
    /// left + right
    /// ```
    fn walk_binary_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        match operator {
            // left && right
            dir::BinaryOperator::And => {
                self.walk_short_circuit_expression(left, right, ConditionBranch::True)?;
            }
            // left || right
            dir::BinaryOperator::Or => {
                self.walk_short_circuit_expression(left, right, ConditionBranch::False)?;
            }
            // eager binary operators
            _ => {
                self.walk_expression(left, self.tree.get(left))?;
                self.walk_expression(right, self.tree.get(right))?;
            }
        }

        let term = self.binary_expression_type(id, left, operator, right)?;
        self.constrain_node_type_term(id, term)?;

        Ok(())
    }

    /// Walk one short circuit binary expression.
    ///
    /// Example:
    /// ```ds
    /// left && right
    /// ```
    fn walk_short_circuit_expression(
        &mut self,
        left: dir::LocalNodeId<dir::Expression>,
        right: dir::LocalNodeId<dir::Expression>,
        right_branch: ConditionBranch,
    ) -> CompilerResult<()> {
        self.walk_expression(left, self.tree.get(left))?;

        let before_right = self.fork_flow();

        // walk right branch
        self.restore_flow(before_right);
        self.narrow_expression(left, right_branch)?;
        self.walk_expression(right, self.tree.get(right))?;
        let right_flow = self
            .expression_can_complete_normally(right)
            .then(|| self.collect_flow_branch(before_right));

        // collect skip branch
        self.restore_flow(before_right);
        self.narrow_expression(left, right_branch.opposite())?;
        let skip_flow = self.collect_flow_branch(before_right);

        if let Some(right_flow) = right_flow {
            self.merge_flow_branches(before_right, &skip_flow, &right_flow);
        } else {
            self.restore_flow_branch(before_right, &skip_flow);
        }

        Ok(())
    }

    /// Walk one assignment expression.
    ///
    /// Example:
    /// ```ds
    /// target += value
    /// ```
    fn walk_assign_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::AssignPattern>,
        operator: dir::AssignOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        self.walk_assign_pattern(left, self.tree.get(left))?;
        self.walk_expression(right, self.tree.get(right))?;

        // value
        let value = self.node_type_operand(right)?;
        let assigned_value = self.assigned_value_type(id, left, operator, value)?;
        if let Some(assigned_value) = assigned_value {
            // set assignment expression result
            let result = self.assignment_result_type(id, left, assigned_value)?;
            self.constrain_node_type(id, result)?;

            // constrain assigned value against target
            self.constrain_assignment_target(left, right, assigned_value)?;
        }

        // write after reading the assigned value
        self.constrain_assign_pattern(left)?;

        Ok(())
    }

    /// Return tuple elements for one expression.
    ///
    /// Example:
    /// ```ds
    /// [name: value, ...rest]
    /// ```
    fn tuple_elements(
        &mut self,
        elements: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<Vec<TupleElement>> {
        let mut terms = Vec::new();

        // keep tuple element order
        for element in elements {
            let argument = self.tree.get(*element);
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
                ty: self.node_type_operand(value)?.into(),
                is_optional: false,
                is_readonly: false,
                is_rest: matches!(argument, dir::Argument::Spread { .. }),
            });
        }

        Ok(terms)
    }

    /// Return string and span segments for one template literal.
    ///
    /// Example:
    /// ```ds
    /// `hello ${name}`
    /// ```
    fn template_segments(
        &mut self,
        value: &dir::TemplateLiteral,
    ) -> CompilerResult<(Vec<dir::StringId>, Vec<TypeOperand>)> {
        let segments = match value {
            // `text`
            dir::TemplateLiteral::String { string } => (vec![*string], Vec::new()),
            // `text ${value}`
            dir::TemplateLiteral::InterpolatedString { strings, arguments } => {
                let spans = self.argument_value_type_operands(arguments)?;

                (strings.clone(), spans)
            }
        };

        Ok(segments)
    }

    /// Return the type term for one binary expression.
    ///
    /// Example:
    /// ```ds
    /// left + right
    /// ```
    fn binary_expression_type(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<TypeTerm> {
        let source = id.into_global_any(self.module);
        let left_type = self.node_type_operand(left)?;
        let right_type = self.node_type_operand(right)?;

        // compile time equality
        if operator.is_equality()
            && let Some(term) = self.static_binary_equality_term(left, operator, right)
        {
            let condition = self.active_static_guard();
            self.constrain_node_static(id, term, condition)?;

            return Ok(TypeTerm::Literal(TypeLiteralTerm::boolean()));
        }

        let term = match operator {
            // left === right
            dir::BinaryOperator::EqualStrict | dir::BinaryOperator::NotEqualStrict => {
                let identity = self.check.inference.push_term(IdentityTerm {
                    source,
                    operator,
                    left: left_type,
                    right: right_type,
                });

                TypeTerm::Identity(identity)
            }
            // key in receiver
            dir::BinaryOperator::In => {
                let membership = self.check.inference.push_term(KeyMembershipTerm {
                    source,
                    key: left_type,
                    receiver: right_type,
                });

                TypeTerm::KeyMembership(membership)
            }
            // overloaded operators
            _ => {
                let operator = self.check.inference.push_term(OperatorTerm {
                    source,
                    kind: OperatorTermKind::Binary(operator),
                    receiver: left_type,
                    argument: Some(right_type),
                });

                TypeTerm::Operator(operator)
            }
        };

        Ok(term)
    }

    /// Return one static equality term when both operands are static values.
    fn static_binary_equality_term(
        &self,
        left: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> Option<StaticTerm> {
        let left = self
            .check
            .inputs
            .node_static(left.into_global_any(self.module))?;
        let right = self
            .check
            .inputs
            .node_static(right.into_global_any(self.module))?;
        let is_negated = operator.is_negative_equality();

        Some(StaticTerm::Equal {
            left,
            right,
            is_negated,
        })
    }

    /// Return the value operand assigned by one assignment.
    ///
    /// Example:
    /// ```ds
    /// target += value
    /// ```
    fn assigned_value_type(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::AssignPattern>,
        operator: dir::AssignOperator,
        value: TypeOperand,
    ) -> CompilerResult<Option<TypeOperand>> {
        // use the right side for direct assignment
        if operator == dir::AssignOperator::Assign {
            return Ok(Some(value));
        }

        // allocate an intermediate for compound assignment
        let result = match self.tree.get(left) {
            // target
            dir::AssignPattern::Expression { value: place } => {
                let source = id.into_global_any(self.module);
                let origin = Origin::Node(source);
                let condition = self.active_static_guard();
                let receiver = self.node_type_operand(*place)?;
                let term = if let Some(binary_operator) = operator.binary_operator() {
                    let operator = self.check.inference.push_term(OperatorTerm {
                        source,
                        kind: OperatorTermKind::Binary(binary_operator),
                        receiver,
                        argument: Some(value),
                    });

                    TypeTerm::Operator(operator)
                } else {
                    let operation = self.check.inference.push_term(TypeOperationTerm::BestCommon {
                        elements: vec![receiver.into(), value],
                    });

                    TypeTerm::Operation(operation)
                };
                let result = self.check.push_type_variable(self.module, origin);
                self.check.equate_type(result, term, condition);

                Some(result.into())
            }
            // target = value
            dir::AssignPattern::Assign { .. }
            // [a, b]
            | dir::AssignPattern::Sequence { .. }
            // { a, b }
            | dir::AssignPattern::Object { .. } => None,
        };

        Ok(result)
    }

    /// Return the expression result type for one assignment.
    ///
    /// Example:
    /// ```ds
    /// target[index] = value
    /// ```
    fn assignment_result_type(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::AssignPattern>,
        assigned_value: TypeOperand,
    ) -> CompilerResult<TypeOperand> {
        let result = match self.tree.get(left) {
            // target
            dir::AssignPattern::Expression { value: place } => {
                if let dir::Expression::Index {
                    left,
                    index: Some(index),
                    ..
                } = self.tree.get(*place)
                {
                    let receiver = self.node_type_operand(*left)?;
                    let index_type = self.node_type_operand(*index)?;
                    let set = self.check.inference.push_term(IndexSetTerm {
                        source: id.into_global_any(self.module),
                        receiver,
                        index: index_type,
                        value: assigned_value,
                        key: self.tree.get(*index).static_key(),
                    });
                    self.check.inference.push_term(TypeTerm::IndexSet(set)).into()
                } else if let Some(place) = self.place_term(*place)? {
                    place.ty
                } else {
                    assigned_value
                }
            }
            // target = value
            dir::AssignPattern::Assign { .. }
            // [a, b]
            | dir::AssignPattern::Sequence { .. }
            // { a, b }
            | dir::AssignPattern::Object { .. } => assigned_value,
        };

        Ok(result)
    }

    /// Constrain one assignment target.
    ///
    /// Example:
    /// ```ds
    /// target = value
    /// ```
    fn constrain_assignment_target(
        &mut self,
        left: dir::LocalNodeId<dir::AssignPattern>,
        right: dir::LocalNodeId<dir::Expression>,
        assigned_value: TypeOperand,
    ) -> CompilerResult<()> {
        match self.tree.get(left) {
            // target
            dir::AssignPattern::Expression { value: place } => {
                if let Some(place) = self.place_term(*place)? {
                    let target = place.ty;
                    let origin = Origin::Node(right.into_global_any(self.module));
                    let condition = self.active_static_guard();
                    self.check.constrain_type(
                        origin,
                        TypeRelation::Assignable,
                        assigned_value,
                        target,
                        condition,
                    );
                } else {
                    self.check
                        .report_not_writable(self.module, (*place).into_any());
                }
            }
            // target = value
            dir::AssignPattern::Assign { .. }
            // [a, b]
            | dir::AssignPattern::Sequence { .. }
            // { a, b }
            | dir::AssignPattern::Object { .. } => {
                if let Some(pattern) = self.assign_pattern_term(self.module, left)?
                {
                    let condition = self.active_static_guard();
                    self.check.constrain_pattern(
                        self.module,
                        PatternRelation::Assign(pattern),
                        left.into_any(),
                        assigned_value,
                        condition,
                    );
                }
            }
        }

        Ok(())
    }

    /// Walk one declaration expression.
    ///
    /// Example:
    /// ```ds
    /// (value) => value
    /// ```
    fn walk_declaration_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        declaration: dir::LocalNodeId<dir::Declaration>,
    ) -> CompilerResult<()> {
        let declaration_value = self.tree.get(declaration);
        let is_function_value = matches!(
            declaration_value,
            dir::Declaration::Function(function)
                if function.signature.form == dir::FunctionForm::Lambda || function.name.is_none()
        );

        // constrain function values
        if is_function_value {
            self.walk_function_value_expression(id, declaration)?;
        }
        // constrain declaration statements
        else {
            self.walk_declaration(declaration, declaration_value)?;
            self.constrain_node_type_term(id, TypeTerm::Literal(TypeLiteralTerm::Void))?;
        }

        Ok(())
    }

    /// Walk one function value expression.
    ///
    /// Example:
    /// ```ds
    /// function (value: string): string { value }
    /// ```
    fn walk_function_value_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        declaration: dir::LocalNodeId<dir::Declaration>,
    ) -> CompilerResult<()> {
        self.walk_declaration(declaration, self.tree.get(declaration))?;

        let symbol = self
            .check
            .module(self.module)
            .declaration_symbol(declaration.into_any())
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "function value declaration {declaration:?} in module {:?} has no symbol",
                    self.module
                ),
            })?;

        let operand = self.symbol_type_operand(symbol)?;
        self.constrain_node_type(id, operand)?;

        Ok(())
    }

    /// Walk one block expression.
    ///
    /// Example:
    /// ```ds
    /// { const value = 1; value }
    /// ```
    fn walk_block_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        block: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<()> {
        self.walk_block(block, self.tree.get(block))?;

        let operand = self.node_type_operand(block)?;
        self.constrain_node_type(id, operand)?;

        Ok(())
    }

    /// Walk one object expression.
    ///
    /// Example:
    /// ```ds
    /// { name: value, read() { value } }
    /// ```
    fn walk_object_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        properties: &[dir::LocalNodeId<dir::Property>],
    ) -> CompilerResult<()> {
        // walk property values and method bodies
        for property in properties {
            self.walk_property(*property, self.tree.get(*property))?;
        }

        let members = self.shape_members(properties, false)?;
        let term = self.check.push_shape_type(members);
        self.constrain_node_type_term(id, term)?;

        Ok(())
    }

    /// Return object-like properties as ordered shape members.
    ///
    /// Example:
    /// ```ds
    /// { ...base, name: value }
    /// ```
    fn shape_members(
        &mut self,
        properties: &[dir::LocalNodeId<dir::Property>],
        is_readonly: bool,
    ) -> CompilerResult<SmallVec<[ShapeMember; 2]>> {
        let mut members = SmallVec::new();

        // collect structural members from statically named properties
        for property in properties {
            match self.tree.get(*property) {
                // { key: value }
                dir::Property::Field { key, value, .. } => {
                    let Some(key) = key.static_key(&self.tree) else {
                        continue;
                    };

                    let variable = self.node_type_operand(*value)?;
                    let member = ShapeMember::Field {
                        key,
                        ty: variable.into(),
                        is_optional: false,
                        is_readonly,
                    };

                    members.push(member);
                }
                // { method() {} }
                dir::Property::Method { key: Some(key), .. } => {
                    let Some(key) = key.static_key(&self.tree) else {
                        continue;
                    };
                    let Some(symbol) = self
                        .check
                        .module(self.module)
                        .declaration_symbol((*property).into_any())
                    else {
                        continue;
                    };
                    let variable = self.symbol_type_operand(symbol)?;
                    let member = ShapeMember::Field {
                        key,
                        ty: variable.into(),
                        is_optional: false,
                        is_readonly,
                    };

                    members.push(member);
                }
                // { []() {} }
                dir::Property::Method { key: None, .. } => {}
                // { ...value }
                dir::Property::Spread { value } => {
                    let source = self.node_type_operand(*value)?;
                    let origin = Origin::Node((*property).into_global_any(self.module));

                    members.push(ShapeMember::Spread { origin, source });
                }
                // ignore damaged syntax
                dir::Property::Error => {}
            }
        }

        Ok(members)
    }

    /// Walk one call expression.
    ///
    /// Example:
    /// ```ds
    /// callee<T>(argument)
    /// ```
    fn walk_call_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<()> {
        let source = id.into_global_any(self.module);

        for argument in arguments {
            self.walk_argument(*argument, self.tree.get(*argument))?;
        }

        // build member call from the receiver
        let callee = if let dir::Expression::Member {
            left: receiver,
            name: Some(name),
        }
        | dir::Expression::PrivateMember {
            left: receiver,
            name: Some(name),
        } = self.tree.get(left)
        {
            let Some(receiver) = self.walk_member_call_receiver(*receiver)? else {
                self.constrain_node_type_term(id, TypeTerm::Literal(TypeLiteralTerm::Error))?;

                return Ok(());
            };
            let member = MemberCallTerm {
                origin: MemberProjectionOrigin::Expression {
                    source: left.into_global_any(self.module),
                },
                receiver,
                key: dir::StaticKey::Name(*name),
                arguments: Vec::new().into(),
            };
            let member = self.check.inference.push_term(member);

            CallCallee::Member(member)
        }
        // build call from the callee expression
        else {
            self.walk_expression(left, self.tree.get(left))?;

            let callee_source = left.into_global_any(self.module);
            let generic_owner = self
                .check
                .inference
                .name(callee_source)
                .map(|resolution| resolution.symbol());

            if let Some(symbol) = generic_owner {
                let source = left.into_global_any(self.module);
                let value = self.check.node_type_operand(source)?;

                CallCallee::Reference { value, symbol }
            } else {
                let source = left.into_global_any(self.module);
                let callee = self.check.node_type_operand(source)?;

                CallCallee::Expression(callee)
            }
        };

        let generic_arguments_term = self.walk_generic_arguments(generic_arguments)?;
        let arguments = self.call_arguments(arguments)?;
        let call = self.check.inference.push_term(CallTerm {
            source,
            callee,
            generic_arguments: generic_arguments_term,
            arguments,
        });
        let term = TypeTerm::Call(call);
        self.constrain_node_type_term(id, term)?;

        Ok(())
    }

    /// Walk one construct expression.
    ///
    /// Example:
    /// ```ds
    /// new Box(value)
    /// ```
    fn walk_construct_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<()> {
        self.walk_construct_expression_inputs(ty, arguments)?;

        let term = self.construct_expression_term(id, ty, arguments)?;
        self.constrain_node_type_term(id, term)?;

        Ok(())
    }

    /// Walk one fallible construct expression.
    ///
    /// Example:
    /// ```ds
    /// new? Box(value)
    /// ```
    fn walk_maybe_construct_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<()> {
        let source = id.into_global_any(self.module);
        self.walk_construct_expression_inputs(ty, arguments)?;

        let constructed = self.construct_expression_term(id, ty, arguments)?;
        let constructed = self.check.inference.push_term(constructed);
        let tried = TryTerm {
            source,
            value: constructed.into(),
            kind: TryTermKind::Maybe,
        };
        let value = tried.value;
        let term = self.check.inference.push_term(tried);
        self.constrain_node_type_term(id, TypeTerm::Try(term))?;
        self.propagate_try(id.into_any(), value);

        Ok(())
    }

    /// Walk construct expression inputs.
    ///
    /// Example:
    /// ```ds
    /// new Box(value)
    /// ```
    fn walk_construct_expression_inputs(
        &mut self,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<()> {
        self.walk_type_expression(ty, self.tree.get(ty))?;

        for argument in arguments {
            self.walk_argument(*argument, self.tree.get(*argument))?;
        }

        Ok(())
    }

    /// Return one construct expression term.
    ///
    /// Example:
    /// ```ds
    /// new Box(value)
    /// ```
    fn construct_expression_term(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<TypeTerm> {
        let callee = self.node_type_operand(ty)?;
        let arguments = self.call_arguments(arguments)?;
        let construct = self.check.inference.push_term(ConstructTerm {
            source: id.into_global_any(self.module),
            callee,
            generic_arguments: Default::default(),
            arguments,
        });

        Ok(TypeTerm::Construct(construct))
    }

    /// Walk one const assertion without widening aggregate literals.
    ///
    /// Example:
    /// ```ds
    /// [1, 2, 3] as const
    /// ```
    fn walk_const_assertion_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        child: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        self.walk_const_assertion_source(child)?;

        let operand = self.node_type_operand(child)?;
        self.constrain_node_type(id, operand)?;

        Ok(())
    }

    /// Return the deep readonly literal type for one const assertion.
    ///
    /// Example:
    /// ```ds
    /// { count: 0 } as const
    /// ```
    fn const_assertion_type(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<TypeOperand> {
        let term = match self.tree.get(id) {
            // 1, "text", true
            dir::Expression::ScalarLiteral(value) => {
                TypeTerm::Literal(TypeLiteralTerm::Scalar(value.clone()))
            }
            // [a, b]
            dir::Expression::ArrayExpression { elements } => {
                let elements = self.const_assertion_tuple_elements(elements)?;
                let term = TypeTerm::Tuple {
                    form: dir::TupleForm::Array,
                    elements: elements.into(),
                };
                self.readonly_type_term(term)
            }
            // (a, b)
            dir::Expression::TupleExpression { elements } => {
                let elements = self.const_assertion_tuple_elements(elements)?;
                let term = TypeTerm::Tuple {
                    form: dir::TupleForm::Tuple,
                    elements: elements.into(),
                };
                self.readonly_type_term(term)
            }
            // { key: value }
            dir::Expression::ObjectExpression { properties } => {
                let members = self.shape_members(properties, true)?;
                self.check.push_shape_type(members)
            }
            // preserve the existing source operand
            _ => {
                return self.node_type_operand(id);
            }
        };

        Ok(self.check.inference.push_term(term).into())
    }

    /// Return one readonly wrapper around a type term.
    ///
    /// Example:
    /// ```ds
    /// value as const
    /// ```
    fn readonly_type_term(&mut self, term: TypeTerm) -> TypeTerm {
        let payload = self.check.inference.push_term(term);
        let form = self.check.inference.push_term(FormTerm::Readonly);

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
        id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        match self.tree.get(id) {
            // [a, b], (a, b)
            dir::Expression::ArrayExpression { elements }
            | dir::Expression::TupleExpression { elements } => {
                // walk element values first
                for element in elements {
                    let Some(value) = self.tree.get(*element).value() else {
                        continue;
                    };
                    self.walk_const_assertion_source(value)?;
                }
                let term = self.const_assertion_type(id)?;
                self.constrain_node_type(id, term)?;
            }
            // { key: value }
            dir::Expression::ObjectExpression { properties } => {
                // walk property values first
                for property in properties {
                    match self.tree.get(*property) {
                        // { key: value }
                        dir::Property::Field { value, .. } => {
                            self.walk_const_assertion_source(*value)?;
                        }
                        // { method() {} }, { ...value }, damaged syntax
                        dir::Property::Method { .. }
                        | dir::Property::Spread { .. }
                        | dir::Property::Error => {
                            self.walk_property(*property, self.tree.get(*property))?;
                        }
                    }
                }
                let term = self.const_assertion_type(id)?;
                self.constrain_node_type(id, term)?;
            }
            // walk non aggregate source normally
            _ => {
                self.walk_expression(id, self.tree.get(id))?;
            }
        }

        Ok(())
    }

    /// Return tuple elements for one const assertion.
    ///
    /// Example:
    /// ```ds
    /// [1, label: 2] as const
    /// ```
    fn const_assertion_tuple_elements(
        &mut self,
        elements: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<Vec<TupleElement>> {
        let mut terms = Vec::new();

        // keep tuple element order
        for element in elements {
            let argument = self.tree.get(*element);
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
            let ty = self.node_type_operand(value)?;

            terms.push(TupleElement {
                label,
                ty: ty.into(),
                is_optional: false,
                is_readonly: false,
                is_rest: matches!(argument, dir::Argument::Spread { .. }),
            });
        }

        Ok(terms)
    }

    /// Walk one template literal's expression arguments.
    ///
    /// Example:
    /// ```ds
    /// `hello ${name}`
    /// ```
    fn walk_template_literal(&mut self, value: &dir::TemplateLiteral) -> CompilerResult<()> {
        match value {
            // `text`
            dir::TemplateLiteral::String { .. } => {}
            // `text ${value}`
            dir::TemplateLiteral::InterpolatedString { arguments, .. } => {
                for argument in arguments {
                    self.walk_argument(*argument, self.tree.get(*argument))?;
                }
            }
        }

        Ok(())
    }

    /// Return value type operands for expression arguments.
    ///
    /// Example:
    /// ```ds
    /// f(a, b, ...rest)
    /// ```
    pub(in crate::check) fn argument_value_type_operands(
        &mut self,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<Vec<TypeOperand>> {
        let mut operands = Vec::with_capacity(arguments.len());

        // collect argument value operands
        for argument in arguments {
            if let Some(value) = self.tree.get(*argument).value() {
                operands.push(self.node_type_operand(value)?);
            }
        }

        Ok(operands)
    }

    /// Return runtime arguments for a call-like expression.
    ///
    /// Example:
    /// ```ds
    /// f(a, b, ...rest)
    /// ```
    pub(in crate::check) fn call_arguments(
        &mut self,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<SmallVec<[CallArgument; 4]>> {
        let mut terms = SmallVec::new();

        // collect argument values with their spread marker
        for argument in arguments {
            let argument = self.tree.get(*argument);
            let Some(value) = argument.value() else {
                continue;
            };
            let term = CallArgument {
                source: value.into_global_any(self.module),
                ty: self.node_type_operand(value)?,
                is_spread: matches!(argument, dir::Argument::Spread { .. }),
            };

            terms.push(term);
        }

        Ok(terms)
    }

    /// Constrain writes through one assignment pattern.
    ///
    /// Example:
    /// ```ds
    /// { name } = value
    /// ```
    fn constrain_assign_pattern(
        &mut self,
        id: dir::LocalNodeId<dir::AssignPattern>,
    ) -> CompilerResult<()> {
        match self.tree.get(id) {
            // target
            dir::AssignPattern::Expression { value } => {
                if let Some(place) = self.place_term(*value)? {
                    let condition = self.active_static_guard();
                    self.clear_mutated_expression_narrowings(*value);
                    self.mark_place_assigned(place);
                    self.check.constrain_writable_place(place, condition);
                }
            }
            // target = value
            dir::AssignPattern::Assign { pattern, .. } => {
                self.constrain_assign_pattern(*pattern)?;
            }
            // [a, b]
            dir::AssignPattern::Sequence { fields }
            // { a, b }
            | dir::AssignPattern::Object { fields } => {
                for field in fields {
                    self.constrain_assign_pattern_field(*field)?;
                }
            }
        }

        Ok(())
    }

    /// Constrain writes through one assignment pattern field.
    ///
    /// Example:
    /// ```ds
    /// { name: target } = value
    /// ```
    fn constrain_assign_pattern_field(
        &mut self,
        id: dir::LocalNodeId<dir::AssignPatternField>,
    ) -> CompilerResult<()> {
        match self.tree.get(id) {
            // { name: pattern }
            dir::AssignPatternField::Named {
                pattern: Some(pattern),
                ..
            }
            // { ...pattern }
            | dir::AssignPatternField::Spread {
                pattern: Some(pattern),
            } => {
                self.constrain_assign_pattern(*pattern)?;
            }
            // { [key]: pattern }
            dir::AssignPatternField::Computed { pattern, .. }
            // [pattern]
            | dir::AssignPatternField::Positional { pattern } => {
                self.constrain_assign_pattern(*pattern)?;
            }
            // { name }
            dir::AssignPatternField::Named { pattern: None, .. }
            // { ... }
            | dir::AssignPatternField::Spread { pattern: None }
            // [,]
            | dir::AssignPatternField::Elision => {}
        }

        Ok(())
    }

    /// Walk one labeled expression target.
    ///
    /// Example:
    /// ```ds
    /// label: loop { break label value }
    /// ```
    fn walk_label_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        label: dir::StringId,
        body: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        match self.tree.get(body) {
            // label: while condition { body }
            dir::Expression::While {
                condition,
                body: loop_body,
                ..
            } => {
                let body_type = self.node_type_operand(body)?;
                self.constrain_node_type(id, body_type)?;
                if let Some(_guard) = self.enter_decorated_static_guard(body.into_any(), None)? {
                    self.node_type_operand(body)?;
                    self.walk_while_expression(body, Some(label), *condition, *loop_body)?;
                }

                return Ok(());
            }
            // label: for item of iterator { body }
            dir::Expression::ForEach {
                operator,
                binding,
                iterator,
                body: loop_body,
                ..
            } => {
                let body_type = self.node_type_operand(body)?;
                self.constrain_node_type(id, body_type)?;
                if let Some(_guard) = self.enter_decorated_static_guard(body.into_any(), None)? {
                    self.node_type_operand(body)?;
                    self.walk_for_each_expression(
                        body,
                        Some(label),
                        *operator,
                        binding,
                        *iterator,
                        *loop_body,
                    )?;
                }

                return Ok(());
            }
            // label: for (initialization; condition; increment) { body }
            dir::Expression::For {
                initialization,
                condition,
                increment,
                body: loop_body,
            } => {
                let body_type = self.node_type_operand(body)?;
                self.constrain_node_type(id, body_type)?;
                if let Some(_guard) = self.enter_decorated_static_guard(body.into_any(), None)? {
                    self.node_type_operand(body)?;
                    self.walk_for_expression(
                        body,
                        Some(label),
                        *initialization,
                        *condition,
                        *increment,
                        *loop_body,
                    )?;
                }

                return Ok(());
            }
            // label: loop { body }
            dir::Expression::Loop { body: loop_body } => {
                let body_type = self.node_type_operand(body)?;
                self.constrain_node_type(id, body_type)?;
                if let Some(_guard) = self.enter_decorated_static_guard(body.into_any(), None)? {
                    self.node_type_operand(body)?;
                    self.walk_loop_expression(body, Some(label), *loop_body)?;
                }

                return Ok(());
            }
            // labeled expression
            _ => {}
        }

        // enter labeled control target
        let result = self.node_type_operand(id)?;
        self.enter_control_target(Some(label), false, id, result);

        // walk body with isolated flow
        let before_body = self.fork_flow();
        self.walk_expression(body, self.tree.get(body))?;
        let fallthrough = if self.expression_can_complete_normally(body) {
            let body_type = self.node_type_operand(body)?;
            self.check.resolved_type_operand(body_type)
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

        Ok(())
    }

    /// Walk one if expression with isolated branch flow.
    ///
    /// Example:
    /// ```ds
    /// if value is T { value } else { fallback }
    /// ```
    fn walk_if_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        condition: &dir::IfCondition,
        then_expression: dir::LocalNodeId<dir::Expression>,
        else_expression: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<()> {
        // walk branches with isolated flow
        self.walk_if_condition(condition)?;
        let before = self.fork_flow();

        // walk true branch
        self.restore_flow(before);
        self.narrow_condition(condition, ConditionBranch::True)?;
        self.walk_expression(then_expression, self.tree.get(then_expression))?;
        let then_can_complete = self.expression_can_complete_normally(then_expression);
        let then_type = self.node_type_operand(then_expression)?;
        let mut branches = SmallVec::<[FlowBranch; 2]>::new();
        if then_can_complete {
            branches.push(self.collect_flow_branch(before));
        }

        if let Some(else_expression) = else_expression {
            // walk false branch
            self.restore_flow(before);
            self.narrow_condition(condition, ConditionBranch::False)?;
            self.walk_expression(else_expression, self.tree.get(else_expression))?;
            let else_can_complete = self.expression_can_complete_normally(else_expression);
            let else_type = self.node_type_operand(else_expression)?;
            let term = TypeTerm::Union {
                elements: vec![then_type.into(), else_type.into()],
            };
            self.constrain_node_type_term(id, term)?;

            // collect false completion
            if else_can_complete {
                branches.push(self.collect_flow_branch(before));
            }
        } else {
            let term = TypeTerm::Union {
                elements: vec![then_type.into(), self.void_type_operand()],
            };
            self.constrain_node_type_term(id, term)?;

            // collect implicit false completion
            self.restore_flow(before);
            self.narrow_condition(condition, ConditionBranch::False)?;
            branches.push(self.collect_flow_branch(before));
        }

        // merge normally completed branches
        self.merge_flow_branches_from(before, &branches);

        Ok(())
    }

    /// Walk one if condition.
    ///
    /// Example:
    /// ```ds
    /// if let Some(value) = option { value }
    /// ```
    fn walk_if_condition(&mut self, condition: &dir::IfCondition) -> CompilerResult<()> {
        match condition {
            // if condition
            dir::IfCondition::Expression { condition } => {
                self.walk_expression(*condition, self.tree.get(*condition))?;

                let variable = self.node_type_operand(*condition)?;
                let static_guard = self.active_static_guard();
                self.check.constrain_condition(
                    self.module,
                    (*condition).into_any(),
                    variable,
                    static_guard,
                );
            }
            // if let pattern = value
            dir::IfCondition::Let { declarator, .. } => {
                self.walk_declarator(*declarator, self.tree.get(*declarator))?;
            }
        }

        Ok(())
    }

    /// Walk one while expression.
    ///
    /// Example:
    /// ```ds
    /// while condition { body }
    /// ```
    fn walk_while_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        label: Option<dir::StringId>,
        condition: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<()> {
        // walk condition in incoming flow
        self.walk_expression(condition, self.tree.get(condition))?;
        let condition_type = self.node_type_operand(condition)?;
        let static_guard = self.active_static_guard();
        self.check.constrain_condition(
            self.module,
            condition.into_any(),
            condition_type,
            static_guard,
        );

        // enter loop control target
        let result = self.node_type_operand(id)?;
        self.enter_control_target(label, true, id, result);

        // walk body under true condition flow
        let before_body = self.fork_flow();
        self.narrow_expression(condition, ConditionBranch::True)?;
        self.walk_block(body, self.tree.get(body))?;
        self.restore_flow(before_body);

        // collect normal exit through false condition
        self.narrow_expression(condition, ConditionBranch::False)?;
        let normal_flow = self.collect_flow_branch(before_body);
        let fallthrough = self.void_type_operand();
        let mut branches = self.leave_control_target(Some(fallthrough));

        // merge break branches with normal exit
        branches.push(normal_flow);
        self.merge_flow_branches_from(before_body, &branches);

        Ok(())
    }

    /// Walk one for each expression.
    ///
    /// Example:
    /// ```ds
    /// for (item of items) {
    ///     item
    /// }
    /// ```
    fn walk_for_each_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        label: Option<dir::StringId>,
        operator: dir::ForEachOperator,
        binding: &dir::ForEachBinding,
        iterator: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<()> {
        // walk binding and iterator in incoming flow
        let pattern = match binding {
            dir::ForEachBinding::Pattern { pattern, .. }
            | dir::ForEachBinding::Using { pattern, .. } => *pattern,
        };
        self.walk_pattern(pattern, self.tree.get(pattern))?;
        self.walk_expression(iterator, self.tree.get(iterator))?;
        self.constrain_for_each_binding(id, operator, pattern, iterator)?;

        // enter loop control target
        let result = self.node_type_operand(id)?;
        self.enter_control_target(label, true, id, result);

        // walk body with iteration binding assigned
        let before_body = self.fork_flow();
        self.mark_bindings_assigned(pattern.into_any());
        self.walk_block(body, self.tree.get(body))?;
        self.restore_flow(before_body);

        // collect normal loop exit
        let normal_flow = self.collect_flow_branch(before_body);
        let fallthrough = self.void_type_operand();
        let mut branches = self.leave_control_target(Some(fallthrough));

        // merge break branches with normal exit
        branches.push(normal_flow);
        self.merge_flow_branches_from(before_body, &branches);

        Ok(())
    }

    /// Walk one traditional for expression.
    ///
    /// Example:
    /// ```ds
    /// for (let i = 0; i < n; i++) {
    ///     i
    /// }
    /// ```
    fn walk_for_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        label: Option<dir::StringId>,
        initialization: Option<dir::LocalNodeId<dir::Expression>>,
        condition: Option<dir::LocalNodeId<dir::Expression>>,
        increment: Option<dir::LocalNodeId<dir::Expression>>,
        body: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<()> {
        let result = self.node_type_operand(id)?;

        // walk initialization before loop flow splits
        if let Some(initialization) = initialization {
            self.walk_expression(initialization, self.tree.get(initialization))?;
        }

        // walk condition in incoming flow
        if let Some(condition) = condition {
            self.walk_expression(condition, self.tree.get(condition))?;

            let variable = self.node_type_operand(condition)?;
            let static_guard = self.active_static_guard();
            self.check.constrain_condition(
                self.module,
                condition.into_any(),
                variable,
                static_guard,
            );
        }

        // enter loop control target
        self.enter_control_target(label, true, id, result);

        // walk body under true condition flow
        let before_body = self.fork_flow();
        if let Some(condition) = condition {
            self.narrow_expression(condition, ConditionBranch::True)?;
        }
        self.walk_block(body, self.tree.get(body))?;
        let body_flow = self
            .block_can_complete_normally(self.tree.get(body))
            .then(|| self.collect_flow_branch(before_body));
        let continue_flows = self.take_current_continue_branches();
        if let Some(increment) = increment {
            self.walk_for_increment_expression(increment, before_body, body_flow, &continue_flows)?;
        }
        self.restore_flow(before_body);

        // collect normal exit through false condition
        let normal_flow = if let Some(condition) = condition {
            self.narrow_expression(condition, ConditionBranch::False)?;

            Some(self.collect_flow_branch(before_body))
        } else {
            None
        };
        let fallthrough = condition.map(|_| self.void_type_operand());
        let mut branches = self.leave_control_target(fallthrough);

        // merge break branches with normal exit
        if let Some(normal_flow) = normal_flow {
            branches.push(normal_flow);
        }

        self.merge_flow_branches_from(before_body, &branches);

        Ok(())
    }

    /// Walk one traditional for increment from body and continue flows.
    ///
    /// Example:
    /// ```ds
    /// for (; condition; increment) {
    ///     body
    /// }
    /// ```
    fn walk_for_increment_expression(
        &mut self,
        increment: dir::LocalNodeId<dir::Expression>,
        before_body: FlowCheckpoint,
        body_flow: Option<FlowBranch>,
        continue_flows: &[FlowBranch],
    ) -> CompilerResult<()> {
        let mut flows = Vec::with_capacity(continue_flows.len() + usize::from(body_flow.is_some()));
        flows.extend_from_slice(continue_flows);
        if let Some(body_flow) = body_flow {
            flows.push(body_flow);
        }

        // check unreachable increment once
        if flows.is_empty() {
            self.walk_expression(increment, self.tree.get(increment))?;
            self.restore_flow(before_body);

            return Ok(());
        }

        // check increment from each flow that reaches the next iteration
        for flow in flows {
            self.restore_flow_branch(before_body, &flow);
            self.walk_expression(increment, self.tree.get(increment))?;
            self.restore_flow(before_body);
        }

        Ok(())
    }

    /// Constrain one for each binding to its iterated value.
    ///
    /// Example:
    /// ```ds
    /// for (item of items) {
    ///     item
    /// }
    /// ```
    fn constrain_for_each_binding(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        operator: dir::ForEachOperator,
        pattern: dir::LocalNodeId<dir::Pattern>,
        iterator: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        let source = id.into_global_any(self.module);
        let origin = Origin::Node(source);
        let iterator_type = self.node_type_operand(iterator)?;
        let value = match operator {
            // for (const item of iterable)
            dir::ForEachOperator::Of => {
                let value = self.check.push_type_variable(self.module, origin);
                let unknown = self.check.push_type_variable(self.module, origin);
                let condition = self.active_static_guard();
                self.check.equate_type(
                    unknown,
                    TypeTerm::Literal(TypeLiteralTerm::Unknown),
                    condition.clone(),
                );
                let symbol = self.check.language_symbol(dir::LanguageItem::Iterable);
                let value_argument = GenericArgument::Type(value.into());
                let first_unknown = GenericArgument::Type(unknown.into());
                let second_unknown = GenericArgument::Type(unknown.into());
                let iterable = TypeTerm::Reference {
                    origin: Origin::Node(source),
                    symbol,
                    arguments: vec![value_argument, first_unknown, second_unknown].into(),
                };
                let iterable_variable = self.check.push_type_variable(self.module, origin);
                self.check
                    .equate_type(iterable_variable, iterable, condition.clone());

                self.check.constrain_type(
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
                let operation = self.check.inference.push_term(TypeOperationTerm::KeyOf {
                    target: iterator_type,
                });
                let term = TypeTerm::Operation(operation);
                let variable = self.check.push_type_variable(self.module, origin);
                let condition = self.active_static_guard();
                self.check.equate_type(variable, term, condition);

                variable
            }
        };

        if let Some(term) = self.pattern_term(self.module, pattern)? {
            let condition = self.active_static_guard();
            self.check.constrain_pattern(
                self.module,
                PatternRelation::Match(term),
                pattern.into_any(),
                value,
                condition,
            );
        }

        Ok(())
    }

    /// Walk one loop expression.
    ///
    /// Example:
    /// ```ds
    /// loop {
    ///     break value;
    /// }
    /// ```
    fn walk_loop_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        label: Option<dir::StringId>,
        body: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<()> {
        // enter loop control target
        let result = self.node_type_operand(id)?;
        self.enter_control_target(label, true, id, result);

        // walk body with isolated flow
        let before_body = self.fork_flow();
        self.walk_block(body, self.tree.get(body))?;
        self.restore_flow(before_body);

        // restore only branches that leave the loop
        let branches = self.leave_control_target(None);
        self.merge_flow_branches_from(before_body, &branches);

        Ok(())
    }

    /// Walk one try expression with branch flow for catch.
    ///
    /// Example:
    /// ```ds
    /// try body catch error finally cleanup
    /// ```
    fn walk_try_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Expression>,
        catch: Option<dir::LocalNodeId<dir::Catch>>,
        finally: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<()> {
        let body_type = self.node_type_operand(body)?;
        let term = match catch {
            Some(catch) => {
                let catch_type = self.node_type_operand(self.tree.get(catch).body)?;

                TypeTerm::Union {
                    elements: vec![body_type.into(), catch_type.into()],
                }
            }
            None => {
                self.constrain_node_type(id, body_type)?;

                return Ok(());
            }
        };
        self.constrain_node_type_term(id, term)?;

        // walk try branches
        let before = self.fork_flow();

        // walk the body branch from the incoming flow
        self.restore_flow(before);
        if catch.is_some() {
            self.enter_try_target(id.into_any());
        }
        self.walk_expression(body, self.tree.get(body))?;
        let catch_failure = catch.map(|_| self.leave_try_target());
        let body_flow = self.collect_flow_branch(before);
        let body_can_complete = self.expression_can_complete_normally(body);

        // merge only branches that can continue normally
        let has_normal_flow = if let Some(catch) = catch {
            let catch_can_complete =
                self.expression_can_complete_normally(self.tree.get(catch).body);
            self.restore_flow(before);
            self.walk_catch(catch, self.tree.get(catch), catch_failure)?;
            let catch_flow = self.collect_flow_branch(before);

            match (body_can_complete, catch_can_complete) {
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
        } else if body_can_complete {
            self.restore_flow_branch(before, &body_flow);

            true
        } else {
            self.restore_flow(before);

            false
        };

        // walk finally even when no normal path remains
        if let Some(finally) = finally {
            self.walk_expression(finally, self.tree.get(finally))?;

            if !has_normal_flow || !self.expression_can_complete_normally(finally) {
                self.restore_flow(before);
            }
        } else if !has_normal_flow {
            self.restore_flow(before);
        }

        Ok(())
    }

    /// Walk one match expression with isolated case flow.
    ///
    /// Example:
    /// ```ds
    /// match value { case Some(item) => item }
    /// ```
    fn walk_match_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
        cases: &[dir::LocalNodeId<dir::MatchCase>],
    ) -> CompilerResult<()> {
        // walk match discriminant
        self.walk_expression(value, self.tree.get(value))?;
        let value_type = self.node_type_operand(value)?;
        let value_path = self.flow_path(value);
        let mut active_cases = Vec::new();

        // select cases whose static guards can hold
        for case in cases {
            let condition = self.static_guard_condition(case.into_any(), None)?;
            if !condition.is_never() {
                active_cases.push((*case, condition));
            }
        }

        let before = self.fork_flow();
        let mut elements = Vec::new();
        let mut case_terms = Vec::new();
        let mut merged = None;

        for (case, condition) in &active_cases {
            self.restore_flow(before);
            {
                let _guard = self.enter_static_guard(condition.clone());
                self.walk_match_case(
                    *case,
                    self.tree.get(*case),
                    Some((value_type, value_path.clone())),
                )?;
            }

            match self.tree.get(*case) {
                // case pattern => expression
                dir::MatchCase::Expression { body, .. } => {
                    let ty = self.node_type_operand(*body)?;

                    elements.push(TypeOperand::from(ty));
                }
                // case pattern => { ... }
                dir::MatchCase::Block { body, .. } => {
                    let ty = self.node_type_operand(*body)?;

                    elements.push(TypeOperand::from(ty));
                }
            }

            match self.tree.get(*case).selector() {
                // default
                dir::MatchSelector::Default => {
                    case_terms.push(MatchCase::Default);
                }
                // case pattern if guard
                dir::MatchSelector::Pattern { pattern, guard } => {
                    if let Some(pattern) = self.pattern_term(self.module, *pattern)? {
                        let guard = guard
                            .map(|guard| self.node_type_operand(guard))
                            .transpose()?;

                        case_terms.push(MatchCase::PatternTerm { pattern, guard });
                    }
                }
            }

            if !self.match_case_can_complete_normally(self.tree.get(*case)) {
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
        self.check.constrain_exhaustive_match(
            self.module,
            id.into_any(),
            value_type,
            case_terms,
            condition,
        );
        self.constrain_node_type_term(id, TypeTerm::Union { elements })?;

        Ok(())
    }

    /// Walk one catch clause.
    ///
    /// Example:
    /// ```ds
    /// catch (error: Error) { error }
    /// ```
    pub(in crate::check) fn walk_catch(
        &mut self,
        id: dir::LocalNodeId<dir::Catch>,
        catch: &dir::Catch,
        failure: Option<VariableId>,
    ) -> CompilerResult<()> {
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any(), None)? else {
            return Ok(());
        };

        // catch (error)
        if let Some(pattern) = catch.pattern {
            self.walk_pattern(pattern, self.tree.get(pattern))?;

            if let (Some(failure), Some(ty)) = (failure, catch.ty) {
                let expected = self.node_type_operand(ty)?;
                let origin = Origin::Node(ty.into_global_any(self.module));
                let condition = self.active_static_guard();
                self.check.constrain_type(
                    origin,
                    TypeRelation::Assignable,
                    failure,
                    expected,
                    condition,
                );
            }

            let value = if let Some(ty) = catch.ty {
                Some(self.node_type_operand(ty)?)
            } else {
                failure.map(Into::into)
            };
            let pattern_term = self.pattern_term(self.module, pattern)?;

            if let (Some(value), Some(pattern_term)) = (value, pattern_term) {
                let condition = self.active_static_guard();
                self.check.constrain_pattern(
                    self.module,
                    PatternRelation::Match(pattern_term),
                    pattern.into_any(),
                    value,
                    condition,
                );
            }

            self.mark_bindings_assigned(pattern.into_any());
        }

        // catch (error: T)
        if let Some(ty) = catch.ty {
            self.walk_type_expression(ty, self.tree.get(ty))?;
        }

        // catch (...) { ... }
        self.walk_expression(catch.body, self.tree.get(catch.body))?;

        Ok(())
    }
}
