use destack_dir as dir;
use destack_source::NodeSpanType;

use crate::{Bindings, MatchError, Matcher, PatternNodes};

impl Matcher<'_, '_> {
    /// Match one expression node.
    pub(crate) fn match_expression(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::Expression>,
        candidate_id: dir::LocalNodeId<dir::Expression>,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        let pattern_any = pattern_id.into_any();
        if let Some(is_match) =
            self.match_metavariable(nodes, pattern_any, candidate_id.into_any(), bindings)?
        {
            return Ok(is_match);
        }
        if !self.match_decorators(nodes, pattern_any, candidate_id.into_any(), bindings)? {
            return Ok(false);
        }
        let pattern = nodes.tree().get(pattern_id);
        let candidate = self.candidate.get(candidate_id);

        match (pattern, candidate) {
            (dir::Expression::Declaration(pattern), dir::Expression::Declaration(candidate)) => {
                self.match_declaration(nodes, *pattern, *candidate, bindings)
            }
            (dir::Expression::Block(pattern), dir::Expression::Block(candidate)) => {
                self.match_block(nodes, *pattern, *candidate, bindings)
            }
            (
                dir::Expression::Import {
                    target: pattern_target,
                    items: pattern_items,
                    attributes: pattern_attributes,
                },
                dir::Expression::Import {
                    target: candidate_target,
                    items: candidate_items,
                    attributes: candidate_attributes,
                },
            ) => {
                if pattern_target != candidate_target || pattern_attributes != candidate_attributes
                {
                    return Ok(false);
                }

                self.match_optional_nodes(
                    nodes,
                    pattern_items.as_deref(),
                    candidate_items.as_deref(),
                    bindings,
                )
            }
            (
                dir::Expression::Export {
                    target: pattern_target,
                    items: pattern_items,
                    attributes: pattern_attributes,
                },
                dir::Expression::Export {
                    target: candidate_target,
                    items: candidate_items,
                    attributes: candidate_attributes,
                },
            ) => {
                if pattern_target != candidate_target || pattern_attributes != candidate_attributes
                {
                    return Ok(false);
                }

                self.match_nodes(nodes, pattern_items, candidate_items, 0, 0, bindings)
            }
            (
                dir::Expression::Let {
                    kind: pattern_kind,
                    export: pattern_export,
                    mutability: pattern_mutability,
                    declarators: pattern_declarators,
                    is_ambient: pattern_ambient,
                    is_shared: pattern_shared,
                },
                dir::Expression::Let {
                    kind: candidate_kind,
                    export: candidate_export,
                    mutability: candidate_mutability,
                    declarators: candidate_declarators,
                    is_ambient: candidate_ambient,
                    is_shared: candidate_shared,
                },
            ) => {
                if pattern_kind != candidate_kind
                    || pattern_export != candidate_export
                    || pattern_mutability != candidate_mutability
                    || pattern_ambient != candidate_ambient
                    || pattern_shared != candidate_shared
                {
                    return Ok(false);
                }

                self.match_nodes(
                    nodes,
                    pattern_declarators,
                    candidate_declarators,
                    0,
                    0,
                    bindings,
                )
            }
            (
                dir::Expression::LetElse {
                    kind: pattern_kind,
                    mutability: pattern_mutability,
                    declarator: pattern_declarator,
                    else_branch: pattern_else,
                },
                dir::Expression::LetElse {
                    kind: candidate_kind,
                    mutability: candidate_mutability,
                    declarator: candidate_declarator,
                    else_branch: candidate_else,
                },
            ) => {
                if pattern_kind != candidate_kind
                    || pattern_mutability != candidate_mutability
                    || !self.match_declarator(
                        nodes,
                        *pattern_declarator,
                        *candidate_declarator,
                        bindings,
                    )?
                {
                    return Ok(false);
                }

                self.match_expression(nodes, *pattern_else, *candidate_else, bindings)
            }
            (
                dir::Expression::Using {
                    asynchrony: pattern_asynchrony,
                    export: pattern_export,
                    declarators: pattern_declarators,
                    is_ambient: pattern_ambient,
                },
                dir::Expression::Using {
                    asynchrony: candidate_asynchrony,
                    export: candidate_export,
                    declarators: candidate_declarators,
                    is_ambient: candidate_ambient,
                },
            ) => {
                if pattern_asynchrony != candidate_asynchrony
                    || pattern_export != candidate_export
                    || pattern_ambient != candidate_ambient
                {
                    return Ok(false);
                }

                self.match_nodes(
                    nodes,
                    pattern_declarators,
                    candidate_declarators,
                    0,
                    0,
                    bindings,
                )
            }
            (
                dir::Expression::If {
                    form: pattern_form,
                    condition: pattern_condition,
                    then_expression: pattern_then,
                    else_expression: pattern_else,
                },
                dir::Expression::If {
                    form: candidate_form,
                    condition: candidate_condition,
                    then_expression: candidate_then,
                    else_expression: candidate_else,
                },
            ) => {
                if pattern_form != candidate_form
                    || !self.match_condition(
                        nodes,
                        pattern_condition,
                        candidate_condition,
                        bindings,
                    )?
                    || !self.match_expression(nodes, *pattern_then, *candidate_then, bindings)?
                {
                    return Ok(false);
                }

                self.match_optional_expression(nodes, *pattern_else, *candidate_else, bindings)
            }
            (
                dir::Expression::While {
                    label: _,
                    form: pattern_form,
                    condition: pattern_condition,
                    body: pattern_body,
                },
                dir::Expression::While {
                    label: _,
                    form: candidate_form,
                    condition: candidate_condition,
                    body: candidate_body,
                },
            ) => {
                if pattern_form != candidate_form
                    || !self.match_condition(
                        nodes,
                        pattern_condition,
                        candidate_condition,
                        bindings,
                    )?
                {
                    return Ok(false);
                }

                self.match_block(nodes, *pattern_body, *candidate_body, bindings)
            }
            (
                dir::Expression::ForEach {
                    label: _,
                    asynchrony: pattern_asynchrony,
                    binding: pattern_binding,
                    iterator: pattern_iterator,
                    body: pattern_body,
                },
                dir::Expression::ForEach {
                    label: _,
                    asynchrony: candidate_asynchrony,
                    binding: candidate_binding,
                    iterator: candidate_iterator,
                    body: candidate_body,
                },
            ) => {
                if pattern_asynchrony != candidate_asynchrony
                    || !self.match_for_each_binding(
                        nodes,
                        pattern_binding,
                        candidate_binding,
                        bindings,
                    )?
                    || !self.match_expression(
                        nodes,
                        *pattern_iterator,
                        *candidate_iterator,
                        bindings,
                    )?
                {
                    return Ok(false);
                }

                self.match_block(nodes, *pattern_body, *candidate_body, bindings)
            }
            (
                dir::Expression::For {
                    label: _,
                    initialization: pattern_initialization,
                    condition: pattern_condition,
                    increment: pattern_increment,
                    body: pattern_body,
                },
                dir::Expression::For {
                    label: _,
                    initialization: candidate_initialization,
                    condition: candidate_condition,
                    increment: candidate_increment,
                    body: candidate_body,
                },
            ) => {
                if !self.match_optional_expression(
                    nodes,
                    *pattern_initialization,
                    *candidate_initialization,
                    bindings,
                )? || !self.match_optional_expression(
                    nodes,
                    *pattern_condition,
                    *candidate_condition,
                    bindings,
                )? || !self.match_optional_expression(
                    nodes,
                    *pattern_increment,
                    *candidate_increment,
                    bindings,
                )? {
                    return Ok(false);
                }

                self.match_block(nodes, *pattern_body, *candidate_body, bindings)
            }
            (
                dir::Expression::Loop {
                    label: _,
                    body: pattern,
                },
                dir::Expression::Loop {
                    label: _,
                    body: candidate,
                },
            ) => self.match_block(nodes, *pattern, *candidate, bindings),
            (
                dir::Expression::Try {
                    body: pattern_body,
                    catch: pattern_catch,
                    finally: pattern_finally,
                },
                dir::Expression::Try {
                    body: candidate_body,
                    catch: candidate_catch,
                    finally: candidate_finally,
                },
            ) => {
                if !self.match_expression(nodes, *pattern_body, *candidate_body, bindings)?
                    || !self.match_optional_node(
                        nodes,
                        *pattern_catch,
                        *candidate_catch,
                        bindings,
                    )?
                {
                    return Ok(false);
                }

                self.match_optional_expression(
                    nodes,
                    *pattern_finally,
                    *candidate_finally,
                    bindings,
                )
            }
            (
                dir::Expression::Match {
                    value: pattern_value,
                    arms: pattern_arms,
                },
                dir::Expression::Match {
                    value: candidate_value,
                    arms: candidate_arms,
                },
            ) => {
                if !self.match_expression(nodes, *pattern_value, *candidate_value, bindings)? {
                    return Ok(false);
                }

                self.match_nodes(nodes, pattern_arms, candidate_arms, 0, 0, bindings)
            }
            (
                dir::Expression::Switch {
                    value: pattern_value,
                    cases: pattern_cases,
                },
                dir::Expression::Switch {
                    value: candidate_value,
                    cases: candidate_cases,
                },
            ) => {
                if !self.match_expression(nodes, *pattern_value, *candidate_value, bindings)? {
                    return Ok(false);
                }

                self.match_nodes(nodes, pattern_cases, candidate_cases, 0, 0, bindings)
            }
            (
                dir::Expression::Break {
                    label: pattern_label,
                    value: pattern_value,
                },
                dir::Expression::Break {
                    label: candidate_label,
                    value: candidate_value,
                },
            ) => {
                if !self.match_node_name(
                    nodes,
                    pattern_any,
                    candidate_id.into_any(),
                    *pattern_label,
                    *candidate_label,
                    bindings,
                )? {
                    return Ok(false);
                }

                self.match_optional_expression(nodes, *pattern_value, *candidate_value, bindings)
            }
            (
                dir::Expression::Continue {
                    label: pattern_label,
                },
                dir::Expression::Continue {
                    label: candidate_label,
                },
            ) => self.match_node_name(
                nodes,
                pattern_any,
                candidate_id.into_any(),
                *pattern_label,
                *candidate_label,
                bindings,
            ),
            (
                dir::Expression::Yield {
                    cardinality: pattern_cardinality,
                    value: pattern_value,
                },
                dir::Expression::Yield {
                    cardinality: candidate_cardinality,
                    value: candidate_value,
                },
            ) => {
                if pattern_cardinality != candidate_cardinality {
                    return Ok(false);
                }

                self.match_optional_expression(nodes, *pattern_value, *candidate_value, bindings)
            }
            (
                dir::Expression::Identifier { name: pattern },
                dir::Expression::Identifier { name: candidate },
            ) => Ok(pattern == candidate),
            (dir::Expression::Literal(pattern), dir::Expression::Literal(candidate)) => {
                Ok(pattern == candidate)
            }
            (dir::Expression::This, dir::Expression::This)
            | (dir::Expression::Super, dir::Expression::Super)
            | (dir::Expression::ImportMeta, dir::Expression::ImportMeta)
            | (dir::Expression::ImportSource, dir::Expression::ImportSource)
            | (dir::Expression::Debugger, dir::Expression::Debugger)
            | (dir::Expression::Missing, dir::Expression::Missing)
            | (dir::Expression::Error, dir::Expression::Error) => Ok(true),
            (
                dir::Expression::RangeExpression {
                    start: pattern_start,
                    end: pattern_end,
                    end_kind: pattern_end_kind,
                },
                dir::Expression::RangeExpression {
                    start: candidate_start,
                    end: candidate_end,
                    end_kind: candidate_end_kind,
                },
            ) => {
                if pattern_end_kind != candidate_end_kind
                    || !self.match_optional_expression(
                        nodes,
                        *pattern_start,
                        *candidate_start,
                        bindings,
                    )?
                {
                    return Ok(false);
                }

                self.match_optional_expression(nodes, *pattern_end, *candidate_end, bindings)
            }
            (
                dir::Expression::TemplateExpression { value: pattern },
                dir::Expression::TemplateExpression { value: candidate },
            ) => self.match_template_literal(nodes, pattern, candidate, bindings),
            (
                dir::Expression::TaggedTemplateExpression {
                    tag: pattern_tag,
                    generic_arguments: pattern_generics,
                    value: pattern_value,
                },
                dir::Expression::TaggedTemplateExpression {
                    tag: candidate_tag,
                    generic_arguments: candidate_generics,
                    value: candidate_value,
                },
            ) => {
                if !self.match_expression(nodes, *pattern_tag, *candidate_tag, bindings)?
                    || !self.match_nodes(
                        nodes,
                        pattern_generics,
                        candidate_generics,
                        0,
                        0,
                        bindings,
                    )?
                {
                    return Ok(false);
                }

                self.match_template_literal(nodes, pattern_value, candidate_value, bindings)
            }
            (
                dir::Expression::FixedArrayExpression {
                    value: pattern_value,
                    length: pattern_length,
                },
                dir::Expression::FixedArrayExpression {
                    value: candidate_value,
                    length: candidate_length,
                },
            ) => {
                if !self.match_expression(nodes, *pattern_value, *candidate_value, bindings)? {
                    return Ok(false);
                }

                self.match_expression(nodes, *pattern_length, *candidate_length, bindings)
            }
            (
                dir::Expression::ArrayExpression {
                    elements: pattern_elements,
                },
                dir::Expression::ArrayExpression {
                    elements: candidate_elements,
                },
            )
            | (
                dir::Expression::TupleExpression {
                    elements: pattern_elements,
                },
                dir::Expression::TupleExpression {
                    elements: candidate_elements,
                },
            ) => self.match_argument_list(
                nodes,
                pattern_elements,
                candidate_elements,
                0,
                0,
                bindings,
            ),
            (
                dir::Expression::ObjectExpression {
                    properties: pattern,
                },
                dir::Expression::ObjectExpression {
                    properties: candidate,
                },
            ) => self.match_nodes(nodes, pattern, candidate, 0, 0, bindings),
            (
                dir::Expression::StructExpression {
                    ty: pattern_type,
                    properties: pattern_properties,
                },
                dir::Expression::StructExpression {
                    ty: candidate_type,
                    properties: candidate_properties,
                },
            ) => {
                if !self.match_type_expression(nodes, *pattern_type, *candidate_type, bindings)? {
                    return Ok(false);
                }

                self.match_nodes(
                    nodes,
                    pattern_properties,
                    candidate_properties,
                    0,
                    0,
                    bindings,
                )
            }
            (
                dir::Expression::TreeExpression {
                    left: pattern_left,
                    generic_arguments: pattern_generics,
                    attributes: pattern_attributes,
                    children: pattern_children,
                },
                dir::Expression::TreeExpression {
                    left: candidate_left,
                    generic_arguments: candidate_generics,
                    attributes: candidate_attributes,
                    children: candidate_children,
                },
            ) => {
                if !self.match_optional_expression(
                    nodes,
                    *pattern_left,
                    *candidate_left,
                    bindings,
                )? || !self.match_nodes(
                    nodes,
                    pattern_generics,
                    candidate_generics,
                    0,
                    0,
                    bindings,
                )? || !self.match_optional_nodes(
                    nodes,
                    pattern_attributes.as_deref(),
                    candidate_attributes.as_deref(),
                    bindings,
                )? {
                    return Ok(false);
                }

                self.match_optional_nodes(
                    nodes,
                    pattern_children.as_deref(),
                    candidate_children.as_deref(),
                    bindings,
                )
            }
            (
                dir::Expression::Type {
                    value: pattern_value,
                },
                dir::Expression::Type {
                    value: candidate_value,
                },
            ) => self.match_type_expression(nodes, *pattern_value, *candidate_value, bindings),
            (
                dir::Expression::Const { body: pattern_body },
                dir::Expression::Const {
                    body: candidate_body,
                },
            )
            | (
                dir::Expression::Await {
                    expression: pattern_body,
                },
                dir::Expression::Await {
                    expression: candidate_body,
                },
            )
            | (
                dir::Expression::AwaitMaybe {
                    expression: pattern_body,
                },
                dir::Expression::AwaitMaybe {
                    expression: candidate_body,
                },
            )
            | (
                dir::Expression::AwaitMust {
                    expression: pattern_body,
                },
                dir::Expression::AwaitMust {
                    expression: candidate_body,
                },
            )
            | (
                dir::Expression::Chain {
                    expression: pattern_body,
                },
                dir::Expression::Chain {
                    expression: candidate_body,
                },
            ) => self.match_expression(nodes, *pattern_body, *candidate_body, bindings),
            (
                dir::Expression::Return {
                    value: pattern_value,
                },
                dir::Expression::Return {
                    value: candidate_value,
                },
            ) => self.match_optional_expression(nodes, *pattern_value, *candidate_value, bindings),
            (
                dir::Expression::As {
                    expression: pattern_expression,
                    target_type: pattern_type,
                },
                dir::Expression::As {
                    expression: candidate_expression,
                    target_type: candidate_type,
                },
            )
            | (
                dir::Expression::Satisfies {
                    expression: pattern_expression,
                    target_type: pattern_type,
                },
                dir::Expression::Satisfies {
                    expression: candidate_expression,
                    target_type: candidate_type,
                },
            )
            | (
                dir::Expression::Is {
                    value: pattern_expression,
                    target_type: pattern_type,
                },
                dir::Expression::Is {
                    value: candidate_expression,
                    target_type: candidate_type,
                },
            ) => {
                if !self.match_expression(
                    nodes,
                    *pattern_expression,
                    *candidate_expression,
                    bindings,
                )? {
                    return Ok(false);
                }

                self.match_type_expression(nodes, *pattern_type, *candidate_type, bindings)
            }
            (
                dir::Expression::InstanceOf {
                    value: pattern_value,
                    target: pattern_target,
                },
                dir::Expression::InstanceOf {
                    value: candidate_value,
                    target: candidate_target,
                },
            ) => {
                if !self.match_expression(nodes, *pattern_value, *candidate_value, bindings)? {
                    return Ok(false);
                }

                self.match_expression(nodes, *pattern_target, *candidate_target, bindings)
            }
            (
                dir::Expression::Binary {
                    left: pattern_left,
                    operator: pattern_operator,
                    right: pattern_right,
                },
                dir::Expression::Binary {
                    left: candidate_left,
                    operator: candidate_operator,
                    right: candidate_right,
                },
            ) => {
                if pattern_operator != candidate_operator {
                    return Ok(false);
                }
                if !self.match_expression(nodes, *pattern_left, *candidate_left, bindings)? {
                    return Ok(false);
                }

                self.match_expression(nodes, *pattern_right, *candidate_right, bindings)
            }
            (
                dir::Expression::Unary {
                    operator: pattern_operator,
                    right: pattern_right,
                },
                dir::Expression::Unary {
                    operator: candidate_operator,
                    right: candidate_right,
                },
            ) => {
                if pattern_operator != candidate_operator {
                    return Ok(false);
                }

                self.match_expression(nodes, *pattern_right, *candidate_right, bindings)
            }
            (
                dir::Expression::BorrowOf {
                    access: pattern_access,
                    variance: pattern_variance,
                    right: pattern_right,
                },
                dir::Expression::BorrowOf {
                    access: candidate_access,
                    variance: candidate_variance,
                    right: candidate_right,
                },
            ) => {
                if pattern_access != candidate_access || pattern_variance != candidate_variance {
                    return Ok(false);
                }

                self.match_expression(nodes, *pattern_right, *candidate_right, bindings)
            }
            (
                dir::Expression::Member {
                    left: pattern_left,
                    name: pattern_name,
                    is_optional: pattern_optional,
                },
                dir::Expression::Member {
                    left: candidate_left,
                    name: candidate_name,
                    is_optional: candidate_optional,
                },
            ) => {
                if pattern_optional != candidate_optional
                    || !self.match_expression(nodes, *pattern_left, *candidate_left, bindings)?
                {
                    return Ok(false);
                }
                let side_use = nodes.uses().get_name(pattern_any, NodeSpanType::Main);
                match side_use {
                    Some(use_entry) => self.bind_name(
                        use_entry,
                        candidate_id.into_any(),
                        *candidate_name,
                        bindings,
                    ),
                    None => Ok(pattern_name == candidate_name),
                }
            }
            (
                dir::Expression::Index {
                    position: pattern_position,
                    left: pattern_left,
                    index: pattern_index,
                    is_optional: pattern_optional,
                },
                dir::Expression::Index {
                    position: candidate_position,
                    left: candidate_left,
                    index: candidate_index,
                    is_optional: candidate_optional,
                },
            ) => {
                if pattern_position != candidate_position
                    || pattern_optional != candidate_optional
                    || !self.match_expression(nodes, *pattern_left, *candidate_left, bindings)?
                {
                    return Ok(false);
                }

                self.match_optional_expression(nodes, *pattern_index, *candidate_index, bindings)
            }
            (
                dir::Expression::Instantiation {
                    left: pattern_left,
                    generic_arguments: pattern_generics,
                },
                dir::Expression::Instantiation {
                    left: candidate_left,
                    generic_arguments: candidate_generics,
                },
            ) => {
                if !self.match_expression(nodes, *pattern_left, *candidate_left, bindings)? {
                    return Ok(false);
                }

                self.match_nodes(nodes, pattern_generics, candidate_generics, 0, 0, bindings)
            }
            (
                dir::Expression::Infer {
                    form: pattern_form,
                    name: pattern_name,
                },
                dir::Expression::Infer {
                    form: candidate_form,
                    name: candidate_name,
                },
            ) => {
                if pattern_form != candidate_form {
                    return Ok(false);
                }

                self.match_node_name(
                    nodes,
                    pattern_any,
                    candidate_id.into_any(),
                    *pattern_name,
                    *candidate_name,
                    bindings,
                )
            }
            (
                dir::Expression::Call {
                    position: pattern_position,
                    left: pattern_left,
                    generic_arguments: pattern_generics,
                    arguments: pattern_arguments,
                    is_optional: pattern_optional,
                },
                dir::Expression::Call {
                    position: candidate_position,
                    left: candidate_left,
                    generic_arguments: candidate_generics,
                    arguments: candidate_arguments,
                    is_optional: candidate_optional,
                },
            ) => {
                if pattern_position != candidate_position || pattern_optional != candidate_optional
                {
                    return Ok(false);
                }
                if !self.match_expression(nodes, *pattern_left, *candidate_left, bindings)? {
                    return Ok(false);
                }
                if !self.match_generic_arguments(
                    nodes,
                    pattern_generics,
                    candidate_generics,
                    bindings,
                )? {
                    return Ok(false);
                }

                self.match_argument_list(
                    nodes,
                    pattern_arguments,
                    candidate_arguments,
                    0,
                    0,
                    bindings,
                )
            }
            (
                dir::Expression::New {
                    left: pattern_left,
                    generic_arguments: pattern_generic_arguments,
                    arguments: pattern_arguments,
                },
                dir::Expression::New {
                    left: candidate_left,
                    generic_arguments: candidate_generic_arguments,
                    arguments: candidate_arguments,
                },
            ) => {
                if !self.match_expression(nodes, *pattern_left, *candidate_left, bindings)? {
                    return Ok(false);
                }
                if !self.match_generic_arguments(
                    nodes,
                    pattern_generic_arguments,
                    candidate_generic_arguments,
                    bindings,
                )? {
                    return Ok(false);
                }

                self.match_argument_list(
                    nodes,
                    pattern_arguments,
                    candidate_arguments,
                    0,
                    0,
                    bindings,
                )
            }
            (
                dir::Expression::Maybe {
                    position: pattern_position,
                    left: pattern_left,
                },
                dir::Expression::Maybe {
                    position: candidate_position,
                    left: candidate_left,
                },
            )
            | (
                dir::Expression::Must {
                    position: pattern_position,
                    left: pattern_left,
                },
                dir::Expression::Must {
                    position: candidate_position,
                    left: candidate_left,
                },
            ) => {
                if pattern_position != candidate_position {
                    return Ok(false);
                }

                self.match_expression(nodes, *pattern_left, *candidate_left, bindings)
            }
            (
                dir::Expression::Assign {
                    left: pattern_left,
                    operator: pattern_operator,
                    right: pattern_right,
                },
                dir::Expression::Assign {
                    left: candidate_left,
                    operator: candidate_operator,
                    right: candidate_right,
                },
            ) => {
                if pattern_operator != candidate_operator
                    || !self.match_assign_pattern(
                        nodes,
                        *pattern_left,
                        *candidate_left,
                        bindings,
                    )?
                {
                    return Ok(false);
                }

                self.match_expression(nodes, *pattern_right, *candidate_right, bindings)
            }
            _ => Ok(false),
        }
    }

    /// Match one call argument.
    pub(crate) fn match_argument(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::Argument>,
        candidate_id: dir::LocalNodeId<dir::Argument>,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        let pattern_any = pattern_id.into_any();
        if let Some(is_match) =
            self.match_metavariable(nodes, pattern_any, candidate_id.into_any(), bindings)?
        {
            return Ok(is_match);
        }
        if !self.match_decorators(nodes, pattern_any, candidate_id.into_any(), bindings)? {
            return Ok(false);
        }
        let pattern = nodes.tree().get(pattern_id);
        let candidate = self.candidate.get(candidate_id);

        match (pattern, candidate) {
            (
                dir::Argument::Positional {
                    value: pattern_value,
                },
                dir::Argument::Positional {
                    value: candidate_value,
                },
            ) => self.match_expression(nodes, *pattern_value, *candidate_value, bindings),
            (
                dir::Argument::Spread {
                    value: pattern_value,
                },
                dir::Argument::Spread {
                    value: candidate_value,
                },
            ) => self.match_expression(nodes, *pattern_value, *candidate_value, bindings),
            (dir::Argument::Elision, dir::Argument::Elision)
            | (dir::Argument::Error, dir::Argument::Error) => Ok(true),
            _ => Ok(false),
        }
    }

    /// Match two argument lists with non-greedy sequence uses.
    fn match_argument_list(
        &self,
        nodes: &PatternNodes<'_>,
        patterns: &[dir::LocalNodeId<dir::Argument>],
        candidates: &[dir::LocalNodeId<dir::Argument>],
        pattern_index: usize,
        candidate_index: usize,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        self.match_nodes(
            nodes,
            patterns,
            candidates,
            pattern_index,
            candidate_index,
            bindings,
        )
    }
}
