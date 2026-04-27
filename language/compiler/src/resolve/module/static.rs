use std::collections::HashSet;

use destack_artifact::{EmitFormat, Platform, Runtime};
use destack_dir::{
    BinaryOperator, Block, Declaration, Decorator, Expression, LocalNodeId, LocalNodeIdAny, Member,
    NodeType, NodeVisitor, NodeVisitorOptions, ScalarLiteral, SymbolTable, Tree, Type, TypeLiteral,
    TypeMember, TypeTable, walk_any,
};
use destack_source::ModuleId;
use destack_workspace::{ImportMeta, ProfileEnv, ProfileId};

use crate::{Compiler, ResolveError, ResolveResult};

/// A value computed by static if evaluation.
#[derive(Debug, Clone, PartialEq)]
enum StaticIfValue {
    /// A scalar literal value.
    Scalar(ScalarLiteral),
    /// The null literal value.
    Null,
    /// The undefined literal value.
    Undefined,
    /// The `import` keyword root before `.meta`.
    ImportKeyword,
    /// The import.meta object.
    ImportMeta,
    /// The import.meta.env object.
    ImportMetaEnv,
    /// The import.meta.target object.
    ImportMetaTarget,
}

impl StaticIfValue {
    /// Return the scalar literal value when present.
    fn as_scalar(&self) -> Option<&ScalarLiteral> {
        // unwrap scalar values for operators that require them
        match self {
            Self::Scalar(value) => Some(value),
            _ => None,
        }
    }
}

/// Evaluate a unary operator in a static expression.
fn evaluate_unary_scalar(
    operator: destack_dir::UnaryOperator,
    right: &ScalarLiteral,
) -> Option<ScalarLiteral> {
    match operator {
        destack_dir::UnaryOperator::Not => match right {
            ScalarLiteral::Boolean(value) => Some(ScalarLiteral::Boolean(!value)),
            _ => None,
        },
        destack_dir::UnaryOperator::Plus => Some(right.clone()),
        destack_dir::UnaryOperator::Negate => match right {
            ScalarLiteral::Integer(value) => Some(ScalarLiteral::Integer(-value)),
            ScalarLiteral::Bigint(value) => Some(ScalarLiteral::Bigint(-value)),
            ScalarLiteral::Float(value) => Some(ScalarLiteral::Float(-value)),
            _ => None,
        },
        destack_dir::UnaryOperator::WrappingNegate => match right {
            ScalarLiteral::Integer(value) => Some(ScalarLiteral::Integer(value.wrapping_neg())),
            ScalarLiteral::Bigint(value) => Some(ScalarLiteral::Bigint(value.wrapping_neg())),
            _ => None,
        },
        destack_dir::UnaryOperator::ElementwiseNot => match right {
            ScalarLiteral::Integer(value) => Some(ScalarLiteral::Integer(!value)),
            ScalarLiteral::Bigint(value) => Some(ScalarLiteral::Bigint(!value)),
            _ => None,
        },
        _ => None,
    }
}

/// Evaluate a binary operator in a static expression.
fn evaluate_binary_scalar(
    operator: BinaryOperator,
    left: &ScalarLiteral,
    right: &ScalarLiteral,
) -> Option<ScalarLiteral> {
    match operator {
        BinaryOperator::Add => evaluate_numeric_binary(left, right, |a, b| a + b, |a, b| a + b),
        BinaryOperator::Subtract => {
            evaluate_numeric_binary(left, right, |a, b| a - b, |a, b| a - b)
        }
        BinaryOperator::Multiply => {
            evaluate_numeric_binary(left, right, |a, b| a * b, |a, b| a * b)
        }
        BinaryOperator::Divide => evaluate_numeric_binary(left, right, |a, b| a / b, |a, b| a / b),
        BinaryOperator::Remainder => {
            evaluate_numeric_binary(left, right, |a, b| a % b, |a, b| a % b)
        }
        BinaryOperator::WrappingAdd => evaluate_integer_binary(left, right, i64::wrapping_add),
        BinaryOperator::WrappingSubtract => evaluate_integer_binary(left, right, i64::wrapping_sub),
        BinaryOperator::WrappingMultiply => evaluate_integer_binary(left, right, i64::wrapping_mul),
        BinaryOperator::SaturatingAdd => evaluate_integer_binary(left, right, i64::saturating_add),
        BinaryOperator::SaturatingSubtract => {
            evaluate_integer_binary(left, right, i64::saturating_sub)
        }
        BinaryOperator::SaturatingMultiply => {
            evaluate_integer_binary(left, right, i64::saturating_mul)
        }
        BinaryOperator::ShiftLeft => evaluate_shift_binary(left, right, |a, b| a << b),
        BinaryOperator::ShiftRight => evaluate_shift_binary(left, right, |a, b| a >> b),
        BinaryOperator::ElementwiseAnd => evaluate_integer_binary(left, right, |a, b| a & b),
        BinaryOperator::ElementwiseOr => evaluate_integer_binary(left, right, |a, b| a | b),
        BinaryOperator::ElementwiseXor => evaluate_integer_binary(left, right, |a, b| a ^ b),
        BinaryOperator::Equal | BinaryOperator::EqualStrict => {
            Some(ScalarLiteral::Boolean(left == right))
        }
        BinaryOperator::NotEqual | BinaryOperator::NotEqualStrict => {
            Some(ScalarLiteral::Boolean(left != right))
        }
        BinaryOperator::LessThan => evaluate_compare_binary(left, right, |a, b| a < b),
        BinaryOperator::LessThanOrEqual => evaluate_compare_binary(left, right, |a, b| a <= b),
        BinaryOperator::GreaterThan => evaluate_compare_binary(left, right, |a, b| a > b),
        BinaryOperator::GreaterThanOrEqual => evaluate_compare_binary(left, right, |a, b| a >= b),
        BinaryOperator::And => match (left, right) {
            (ScalarLiteral::Boolean(a), ScalarLiteral::Boolean(b)) => {
                Some(ScalarLiteral::Boolean(*a && *b))
            }
            _ => None,
        },
        BinaryOperator::Or => match (left, right) {
            (ScalarLiteral::Boolean(a), ScalarLiteral::Boolean(b)) => {
                Some(ScalarLiteral::Boolean(*a || *b))
            }
            _ => None,
        },
        _ => None,
    }
}

/// Evaluate a numeric binary operator in a static expression.
fn evaluate_numeric_binary(
    left: &ScalarLiteral,
    right: &ScalarLiteral,
    int_op: fn(i64, i64) -> i64,
    float_op: fn(f64, f64) -> f64,
) -> Option<ScalarLiteral> {
    match (left, right) {
        (ScalarLiteral::Integer(a), ScalarLiteral::Integer(b)) => {
            Some(ScalarLiteral::Integer(int_op(*a, *b)))
        }
        (ScalarLiteral::Bigint(a), ScalarLiteral::Bigint(b)) => {
            Some(ScalarLiteral::Bigint(int_op(*a, *b)))
        }
        (ScalarLiteral::Float(a), ScalarLiteral::Float(b)) => {
            Some(ScalarLiteral::Float(float_op(*a, *b)))
        }
        (ScalarLiteral::Integer(a), ScalarLiteral::Float(b)) => {
            Some(ScalarLiteral::Float(float_op(*a as f64, *b)))
        }
        (ScalarLiteral::Float(a), ScalarLiteral::Integer(b)) => {
            Some(ScalarLiteral::Float(float_op(*a, *b as f64)))
        }
        _ => None,
    }
}

/// Evaluate an integer binary operator in a static expression.
fn evaluate_integer_binary(
    left: &ScalarLiteral,
    right: &ScalarLiteral,
    op: fn(i64, i64) -> i64,
) -> Option<ScalarLiteral> {
    match (left, right) {
        (ScalarLiteral::Integer(a), ScalarLiteral::Integer(b)) => {
            Some(ScalarLiteral::Integer(op(*a, *b)))
        }
        (ScalarLiteral::Bigint(a), ScalarLiteral::Bigint(b)) => {
            Some(ScalarLiteral::Bigint(op(*a, *b)))
        }
        _ => None,
    }
}

/// Evaluate a shift operator in a static expression.
fn evaluate_shift_binary(
    left: &ScalarLiteral,
    right: &ScalarLiteral,
    op: fn(i64, u32) -> i64,
) -> Option<ScalarLiteral> {
    let shift = match right {
        ScalarLiteral::Integer(value) => (*value).try_into().ok(),
        ScalarLiteral::Bigint(value) => (*value).try_into().ok(),
        _ => None,
    }?;

    match left {
        ScalarLiteral::Integer(value) => Some(ScalarLiteral::Integer(op(*value, shift))),
        ScalarLiteral::Bigint(value) => Some(ScalarLiteral::Bigint(op(*value, shift))),
        _ => None,
    }
}

/// Evaluate a comparison operator in a static expression.
fn evaluate_compare_binary(
    left: &ScalarLiteral,
    right: &ScalarLiteral,
    op: fn(f64, f64) -> bool,
) -> Option<ScalarLiteral> {
    let (left, right) = match (left, right) {
        (ScalarLiteral::Integer(a), ScalarLiteral::Integer(b)) => (*a as f64, *b as f64),
        (ScalarLiteral::Bigint(a), ScalarLiteral::Bigint(b)) => (*a as f64, *b as f64),
        (ScalarLiteral::Float(a), ScalarLiteral::Float(b)) => (*a, *b),
        (ScalarLiteral::Integer(a), ScalarLiteral::Float(b)) => (*a as f64, *b),
        (ScalarLiteral::Float(a), ScalarLiteral::Integer(b)) => (*a, *b as f64),
        _ => return None,
    };

    Some(ScalarLiteral::Boolean(op(left, right)))
}

/// Collect node ids for a subtree.
#[derive(Debug, Default)]
struct InactiveNodeCollector {
    /// The collected node ids.
    nodes: Vec<LocalNodeIdAny>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl InactiveNodeCollector {
    /// Collect node ids in a subtree.
    fn collect(&mut self, tree: &Tree, root: LocalNodeIdAny) {
        // reset the collected nodes
        self.nodes.clear();

        // walk the subtree from the root
        walk_any(self, tree, root.ty, root.id);
    }
}

impl NodeVisitor for InactiveNodeCollector {
    /// Return the visitor options.
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    /// Record each visited node id.
    fn visit_any(&mut self, _tree: &Tree, ty: NodeType, id: u32) {
        self.nodes.push(LocalNodeIdAny::new(id, ty));
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Apply static if decorators to the profile dir.
    pub(crate) fn apply_static_if_decorators(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        import_meta: Option<&ImportMeta>,
        tree: &mut Tree,
        symbols: &mut SymbolTable,
        types: &TypeTable,
        roots: &mut Vec<LocalNodeId<Expression>>,
    ) -> ResolveResult<()> {
        // exit early when import meta is missing
        let Some(import_meta) = import_meta else {
            return Ok(());
        };

        // validate decorator placement before applying filters
        self.validate_static_if_placement(module_id, profile_id, tree)?;

        // collect static if targets
        let if_name = self.repository.strings.intern("if");
        let mut declaration_targets = Vec::new();
        let mut expression_targets = Vec::new();
        let mut seen_declarations = HashSet::new();
        let mut seen_expressions = HashSet::new();

        // gather parent nodes for @if decorators
        for annotation_id in tree.iter_node_ids_of_type::<Decorator>() {
            if self
                .decorator_call_named(tree, annotation_id, if_name)
                .is_none()
            {
                continue;
            }

            let Some(parent_id) = tree.get_parent(annotation_id.id) else {
                continue;
            };

            match parent_id.ty {
                NodeType::Declaration => {
                    let declaration_id = parent_id.into_typed::<Declaration>();
                    if seen_declarations.insert(declaration_id.id) {
                        declaration_targets.push(declaration_id);
                    }
                }
                NodeType::Member | NodeType::TypeMember | NodeType::EnumField => {
                    let Some(parent_declaration) = tree.get_parent(parent_id.id) else {
                        continue;
                    };
                    if parent_declaration.ty != NodeType::Declaration {
                        continue;
                    }
                    let declaration_id = parent_declaration.into_typed::<Declaration>();
                    if seen_declarations.insert(declaration_id.id) {
                        declaration_targets.push(declaration_id);
                    }
                }
                NodeType::Expression => {
                    let expression_id = parent_id.into_typed::<Expression>();
                    if seen_expressions.insert(expression_id.id) {
                        expression_targets.push(expression_id);
                    }
                }
                _ => {}
            }
        }

        // keep targets deterministic by node id
        declaration_targets.sort_by_key(|id| id.id);
        expression_targets.sort_by_key(|id| id.id);

        // apply static if to declarations and their members
        for declaration_id in declaration_targets {
            // evaluate the declaration gate
            let condition = self.static_if_condition_for_node(
                module_id,
                profile_id,
                declaration_id.into_any(),
                tree,
                import_meta,
            )?;

            // deactivate declarations that are gated out
            if matches!(condition, Some(false)) {
                self.deactivate_declaration(tree, symbols, types, declaration_id);
                continue;
            }

            // filter nested members and fields
            self.filter_declaration_members(
                module_id,
                profile_id,
                tree,
                symbols,
                types,
                declaration_id,
                import_meta,
            )?;
        }

        // collect expression ids that can be gated
        let mut allowed_expressions = HashSet::new();

        // include module root expressions
        for root_id in roots.iter().copied() {
            allowed_expressions.insert(root_id.id);
        }

        // include block statement expressions
        for block_id in tree.iter_node_ids_of_type::<Block>() {
            let block = tree.get(block_id);

            // record each block expression id
            for expression_id in block.iter_expressions() {
                allowed_expressions.insert(expression_id.id);
            }
        }

        // find expressions gated out by static if
        let mut removed_expressions = HashSet::new();
        for expression_id in expression_targets {
            // skip expressions under inactive declarations
            if !self.is_node_active(tree, symbols, expression_id.into_any()) {
                continue;
            }

            // evaluate the expression gate
            let condition = self.static_if_condition_for_node(
                module_id,
                profile_id,
                expression_id.into_any(),
                tree,
                import_meta,
            )?;
            let Some(condition) = condition else {
                continue;
            };

            // enforce placement on roots or block statements
            if !allowed_expressions.contains(&expression_id.id) {
                return Err(self.invalid_static_if(
                    module_id,
                    profile_id,
                    expression_id.into_any(),
                    "static if is only allowed on module statements or block statements",
                ));
            }

            // collect gated out expressions
            if !condition {
                // capture declaration nodes gated out at the expression level
                let declaration_id = match tree.get(expression_id) {
                    Expression::Declaration(declaration) => Some(*declaration),
                    _ => None,
                };

                // mark the expression node as inactive
                self.mark_inactive_subtree(tree, types, expression_id.into_any());

                removed_expressions.insert(expression_id.id);

                // deactivate declarations gated out at the expression level
                if let Some(declaration_id) = declaration_id {
                    self.deactivate_declaration(tree, symbols, types, declaration_id);
                }
            }
        }

        // filter module roots by gating and declaration activity
        let mut kept_roots = Vec::with_capacity(roots.len());
        for root_id in roots.iter().copied() {
            // drop expressions gated out explicitly
            if removed_expressions.contains(&root_id.id) {
                continue;
            }

            // drop inactive declaration roots
            let Expression::Declaration(declaration) = tree.get(root_id) else {
                kept_roots.push(root_id);
                continue;
            };
            let declaration = tree.get(*declaration);
            if symbols.get_active_symbol(declaration.symbol()).is_none() {
                continue;
            }

            kept_roots.push(root_id);
        }
        *roots = kept_roots;

        // filter block expressions by gating and declaration activity
        let block_ids: Vec<_> = tree.iter_node_ids_of_type::<Block>();
        for block_id in block_ids {
            // rebuild the leading and tail expressions without collapsing block values
            let block = tree.get(block_id).clone();
            let mut kept_leading = Vec::with_capacity(block.leading_expressions.len());
            let mut kept_tail = block.tail_expression;

            for expression_id in block.leading_expressions {
                // skip expressions gated out explicitly
                if removed_expressions.contains(&expression_id.id) {
                    continue;
                }

                // skip inactive declaration expressions
                if let Expression::Declaration(declaration) = tree.get(expression_id) {
                    let declaration = tree.get(*declaration);
                    if symbols.get_active_symbol(declaration.symbol()).is_none() {
                        continue;
                    }
                }

                kept_leading.push(expression_id);
            }

            // preserve the original tail only when it remains active
            if let Some(tail_expression_id) = block.tail_expression {
                if removed_expressions.contains(&tail_expression_id.id) {
                    kept_tail = None;
                } else if let Expression::Declaration(declaration) = tree.get(tail_expression_id) {
                    let declaration = tree.get(*declaration);
                    if symbols.get_active_symbol(declaration.symbol()).is_none() {
                        kept_tail = None;
                    }
                }
            }

            let block = tree.get_mut(block_id);
            block.leading_expressions = kept_leading;
            block.tail_expression = kept_tail;
        }

        // skip resolving static if annotations in later passes
        self.mark_static_if_annotations_inactive(tree, types);

        Ok(())
    }

    /// Validate that static if decorators appear on supported nodes.
    fn validate_static_if_placement(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        tree: &Tree,
    ) -> ResolveResult<()> {
        // cache the decorator identifier
        let if_name = self.repository.strings.intern("if");

        // walk all decorators looking for @if decorators
        for annotation_id in tree.iter_node_ids_of_type::<Decorator>() {
            if self
                .decorator_call_named(tree, annotation_id, if_name)
                .is_none()
            {
                continue;
            }

            // resolve the parent node for the decorator
            let Some(parent_id) = tree.get_parent(annotation_id.id) else {
                return Err(self.invalid_static_if(
                    module_id,
                    profile_id,
                    annotation_id.into_any(),
                    "static if is missing a parent node",
                ));
            };

            // enforce placement on supported node kinds
            match parent_id.ty {
                destack_dir::NodeType::Declaration
                | destack_dir::NodeType::Member
                | destack_dir::NodeType::TypeMember
                | destack_dir::NodeType::EnumField => {}
                destack_dir::NodeType::Expression => {
                    let expression_id = LocalNodeId::<Expression>::new(parent_id.id);
                    let is_statement_position =
                        tree.expression_is_in_statement_position(expression_id);
                    if !is_statement_position
                        && !matches!(tree.get(expression_id), Expression::Declaration(_))
                    {
                        return Err(self.invalid_static_if(
                            module_id,
                            profile_id,
                            annotation_id.into_any(),
                            "static if is only allowed on declarations, members, enum fields, or statements",
                        ));
                    }
                }
                _ => {
                    return Err(self.invalid_static_if(
                        module_id,
                        profile_id,
                        annotation_id.into_any(),
                        "static if is only allowed on declarations, members, enum fields, or statements",
                    ));
                }
            }
        }

        Ok(())
    }

    /// Mark static if decorators inactive after they are processed.
    fn mark_static_if_annotations_inactive(&self, tree: &mut Tree, types: &TypeTable) {
        // cache the decorator identifier
        let if_name = self.repository.strings.intern("if");

        // mark @if decorators inactive
        for annotation_id in tree.iter_node_ids_of_type::<Decorator>() {
            if self
                .decorator_call_named(tree, annotation_id, if_name)
                .is_none()
            {
                continue;
            }

            self.mark_inactive_subtree(tree, types, annotation_id.into_any());
        }
    }

    /// Mark a subtree and its decorators inactive.
    fn mark_inactive_subtree(&self, tree: &mut Tree, types: &TypeTable, root: LocalNodeIdAny) {
        // collect nodes in the main subtree
        let mut collector = InactiveNodeCollector::default();
        let mut inactive_ids = HashSet::new();
        let mut worklist = vec![root];

        // visit subtree nodes and related type expressions
        while let Some(current) = worklist.pop() {
            // skip nodes that are already inactive
            if inactive_ids.contains(&current.id) {
                continue;
            }

            // collect nodes from the subtree
            collector.collect(tree, current);

            // queue related unevaluated type expressions
            for node_id in collector.nodes.iter().copied() {
                // skip nodes already in the inactive set
                if !inactive_ids.insert(node_id.id) {
                    continue;
                }

                // gather declared and signature type references
                let global_node_id = node_id.into_global(tree.module_id);
                let declared_type = types.get_declared_type_id(global_node_id);
                let signature_type = types.get_signature_type_for_node(global_node_id);

                // enqueue unevaluated type expressions
                for type_id in [declared_type, signature_type].into_iter().flatten() {
                    let ty = types.get_type(type_id);

                    // push unevaluated nodes onto the worklist
                    if let Type::Unevaluated(expression_id) = ty {
                        worklist.push((*expression_id).into());
                    }
                }
            }
        }

        // mark nodes in the main subtree
        for node_id in inactive_ids.iter().copied() {
            let node_type = tree.get_node_type(node_id);
            tree.mark_inactive(LocalNodeIdAny::new(node_id, node_type));
        }

        // collect decorator roots attached to the subtree
        let mut annotation_roots = Vec::new();
        for node_id in inactive_ids.iter().copied() {
            let annotations = tree.get_decorators(node_id);
            for annotation_id in annotations {
                annotation_roots.push(annotation_id.into_any());
            }
        }

        // mark annotation subtrees inactive
        for annotation_root in annotation_roots {
            collector.collect(tree, annotation_root);
            for node_id in collector.nodes.iter().copied() {
                tree.mark_inactive(node_id);
            }
        }
    }

    /// Filter declaration members and fields based on static if decorators.
    fn filter_declaration_members(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        tree: &mut Tree,
        symbols: &mut SymbolTable,
        types: &TypeTable,
        declaration_id: LocalNodeId<Declaration>,
        import_meta: &ImportMeta,
    ) -> ResolveResult<()> {
        // capture the member and field lists
        let (members, type_members, fields) = {
            let declaration = tree.get(declaration_id);
            let members = declaration
                .member_ids()
                .map(<[_]>::to_vec)
                .unwrap_or_default();
            let type_members = declaration
                .type_member_ids()
                .map(<[_]>::to_vec)
                .unwrap_or_default();
            let fields = match declaration {
                Declaration::Enum(declaration) => declaration.fields.clone(),
                _ => Vec::new(),
            };
            (members, type_members, fields)
        };

        // filter members based on static if
        let mut filtered_members = Vec::with_capacity(members.len());
        for member_id in members {
            // keep members without annotations
            if !tree.has_decorators(member_id.id) {
                filtered_members.push(member_id);
                continue;
            }

            // evaluate the member gate
            let condition = self.static_if_condition_for_node(
                module_id,
                profile_id,
                member_id.into_any(),
                tree,
                import_meta,
            )?;

            // deactivate members that are gated out
            if matches!(condition, Some(false)) {
                // mark member symbols inactive
                let symbol_id = {
                    let member: &Member = tree.get(member_id);
                    member.symbol()
                };
                let symbol = symbols.get_symbol_mut(symbol_id);
                symbol.is_active = false;

                // mark the member subtree inactive
                self.mark_inactive_subtree(tree, types, member_id.into_any());
                continue;
            }

            filtered_members.push(member_id);
        }

        // filter type members based on static if
        let mut filtered_type_members = Vec::with_capacity(type_members.len());
        for member_id in type_members {
            // keep members without annotations
            if !tree.has_decorators(member_id.id) {
                filtered_type_members.push(member_id);
                continue;
            }

            // evaluate the member gate
            let condition = self.static_if_condition_for_node(
                module_id,
                profile_id,
                member_id.into_any(),
                tree,
                import_meta,
            )?;

            // deactivate members that are gated out
            if matches!(condition, Some(false)) {
                // mark member symbols inactive
                let symbol_id = {
                    let member: &TypeMember = tree.get(member_id);
                    member.symbol()
                };
                let symbol = symbols.get_symbol_mut(symbol_id);
                symbol.is_active = false;

                // mark the member subtree inactive
                self.mark_inactive_subtree(tree, types, member_id.into_any());
                continue;
            }

            filtered_type_members.push(member_id);
        }

        // filter enum fields based on static if
        let mut filtered_fields = Vec::with_capacity(fields.len());
        for field_id in fields {
            // keep fields without annotations
            if !tree.has_decorators(field_id.id) {
                filtered_fields.push(field_id);
                continue;
            }

            // evaluate the field gate
            let condition = self.static_if_condition_for_node(
                module_id,
                profile_id,
                field_id.into_any(),
                tree,
                import_meta,
            )?;

            // deactivate fields that are gated out
            if matches!(condition, Some(false)) {
                // mark enum field symbols inactive
                if let Some(symbol_id) =
                    self.enum_field_symbol_maybe(tree, symbols, declaration_id, field_id)
                {
                    let symbol = symbols.get_symbol_mut(symbol_id);
                    symbol.is_active = false;
                }

                // mark the enum field subtree inactive
                self.mark_inactive_subtree(tree, types, field_id.into_any());
                continue;
            }

            filtered_fields.push(field_id);
        }

        // update the declaration slots
        let declaration = tree.get_mut(declaration_id);
        match declaration {
            Declaration::Struct(declaration) => declaration.members = filtered_members.clone(),
            Declaration::Class(declaration) => declaration.members = filtered_members.clone(),
            Declaration::Interface(declaration) => {
                declaration.members = filtered_type_members.clone();
            }
            Declaration::Extension(declaration) => declaration.members = filtered_members.clone(),
            Declaration::Enum(declaration) => declaration.members = filtered_members.clone(),
            _ => {}
        }

        if let Declaration::Enum(declaration) = declaration {
            declaration.fields = filtered_fields;
        }

        Ok(())
    }

    /// Deactivate a declaration and its members for the profile.
    fn deactivate_declaration(
        &self,
        tree: &mut Tree,
        symbols: &mut SymbolTable,
        types: &TypeTable,
        declaration_id: LocalNodeId<Declaration>,
    ) {
        // collect declaration ids for later updates
        let (symbol_id, members, type_members, fields) = {
            let declaration = tree.get(declaration_id);
            let symbol_id = declaration.symbol();
            let members = declaration.member_ids().map(|members| members.to_vec());
            let type_members = declaration
                .type_member_ids()
                .map(|members| members.to_vec());
            let fields = match declaration {
                Declaration::Enum(declaration) => Some(declaration.fields.clone()),
                _ => None,
            };
            (symbol_id, members, type_members, fields)
        };

        // deactivate the declaration symbol
        let symbol = symbols.get_symbol_mut(symbol_id);
        symbol.is_active = false;

        // mark the declaration subtree inactive
        self.mark_inactive_subtree(tree, types, declaration_id.into_any());

        // deactivate member symbols
        if let Some(members) = members {
            for member_id in members {
                let member = tree.get(member_id);
                let symbol = symbols.get_symbol_mut(member.symbol());
                symbol.is_active = false;
            }
        }

        // deactivate type member symbols
        if let Some(members) = type_members {
            for member_id in members {
                let member = tree.get(member_id);
                let symbol = symbols.get_symbol_mut(member.symbol());
                symbol.is_active = false;
            }
        }

        // deactivate enum field symbols
        if let Some(fields) = fields {
            for field_id in fields {
                if let Some(symbol_id) =
                    self.enum_field_symbol_maybe(tree, symbols, declaration_id, field_id)
                {
                    let symbol = symbols.get_symbol_mut(symbol_id);
                    symbol.is_active = false;
                }
            }
        }
    }

    /// Resolve the symbol declared by one enum field.
    pub(super) fn enum_field_symbol_maybe(
        &self,
        tree: &Tree,
        symbols: &SymbolTable,
        declaration_id: LocalNodeId<Declaration>,
        field_id: LocalNodeId<destack_dir::EnumField>,
    ) -> Option<destack_dir::LocalSymbolId> {
        // enum fields live in the enum declaration scope
        let Declaration::Enum(declaration) = tree.get(declaration_id) else {
            return None;
        };
        let scope = symbols.get_scope_by_id(declaration.scope);

        // match the field symbol by its primary declaration
        for (_, symbol_id) in symbols.active_named_symbols(scope) {
            let symbol = symbols.get_symbol(symbol_id);
            let is_field_symbol = symbol
                .primary_declaration
                .is_some_and(|primary_declaration| {
                    primary_declaration.local_id.ty == destack_dir::NodeType::EnumField
                        && primary_declaration.local_id.id == field_id.id
                });
            if is_field_symbol {
                return Some(symbol_id);
            }
        }

        None
    }

    /// Evaluate the static if condition for a node.
    fn static_if_condition_for_node(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        node_id: LocalNodeIdAny,
        tree: &Tree,
        import_meta: &ImportMeta,
    ) -> ResolveResult<Option<bool>> {
        // track whether any @if decorators were encountered
        let mut saw_if = false;
        let mut combined = true;

        // iterate decorators for the node
        let annotations = tree.get_decorators(node_id.id);
        let if_name = self.repository.strings.intern("if");
        for annotation_id in annotations {
            let Some(call) = self.decorator_call_named(tree, annotation_id, if_name) else {
                continue;
            };

            // record the @if usage
            saw_if = true;

            // validate the decorator arguments
            let Some(arguments) = call.arguments else {
                return Err(self.invalid_static_if(
                    module_id,
                    profile_id,
                    annotation_id.into_any(),
                    "static if requires a condition argument",
                ));
            };
            if arguments.len() != 1 {
                return Err(self.invalid_static_if(
                    module_id,
                    profile_id,
                    annotation_id.into_any(),
                    "static if requires exactly one argument",
                ));
            }

            // evaluate and combine the condition
            let argument = tree.get(arguments[0]);
            let value = self.evaluate_static_if_value(
                module_id,
                profile_id,
                tree,
                argument.value(),
                import_meta,
            )?;
            let condition = self.static_if_value_to_bool(
                module_id,
                profile_id,
                argument.value().into_any(),
                value,
            )?;
            combined &= condition;
        }

        // report the combined condition when present
        if saw_if { Ok(Some(combined)) } else { Ok(None) }
    }

    /// Convert a static if value into a boolean.
    fn static_if_value_to_bool(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        node_id: LocalNodeIdAny,
        value: StaticIfValue,
    ) -> ResolveResult<bool> {
        // accept boolean scalar literals only
        match value {
            StaticIfValue::Scalar(ScalarLiteral::Boolean(value)) => Ok(value),
            _ => Err(self.invalid_static_if(
                module_id,
                profile_id,
                node_id,
                "static if condition must be boolean",
            )),
        }
    }

    /// Evaluate a static if expression into a static value.
    fn evaluate_static_if_value(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        tree: &Tree,
        expression_id: LocalNodeId<Expression>,
        import_meta: &ImportMeta,
    ) -> ResolveResult<StaticIfValue> {
        // dispatch on expression shape
        let expression = tree.get(expression_id);
        match expression {
            Expression::ScalarLiteral { value } => Ok(StaticIfValue::Scalar(value.clone())),
            Expression::TypeLiteral { value } => match value {
                TypeLiteral::Undefined => Ok(StaticIfValue::Undefined),
                TypeLiteral::Null => Ok(StaticIfValue::Null),
                TypeLiteral::ScalarLiteral(value) => Ok(StaticIfValue::Scalar(value.clone())),
                _ => Err(self.invalid_static_if(
                    module_id,
                    profile_id,
                    expression_id.into_any(),
                    "static if does not allow type literals here",
                )),
            },
            Expression::Parenthesized { expression } => {
                self.evaluate_static_if_value(module_id, profile_id, tree, *expression, import_meta)
            }
            Expression::Unary { operator, right } => {
                // evaluate the operand
                let right_value = self.evaluate_static_if_value(
                    module_id,
                    profile_id,
                    tree,
                    *right,
                    import_meta,
                )?;
                let Some(right_literal) = right_value.as_scalar() else {
                    return Err(self.invalid_static_if(
                        module_id,
                        profile_id,
                        expression_id.into_any(),
                        "static if unary operators require scalar values",
                    ));
                };

                // evaluate the unary operator
                let Some(result) = evaluate_unary_scalar(*operator, right_literal) else {
                    return Err(self.invalid_static_if(
                        module_id,
                        profile_id,
                        expression_id.into_any(),
                        "static if unary operator is not supported",
                    ));
                };
                Ok(StaticIfValue::Scalar(result))
            }
            Expression::Binary {
                left,
                operator,
                right,
            } => {
                // evaluate both operands
                let left_value =
                    self.evaluate_static_if_value(module_id, profile_id, tree, *left, import_meta)?;
                let right_value = self.evaluate_static_if_value(
                    module_id,
                    profile_id,
                    tree,
                    *right,
                    import_meta,
                )?;

                // apply the binary operator
                self.evaluate_static_if_binary(
                    module_id,
                    profile_id,
                    expression_id.into_any(),
                    left_value,
                    *operator,
                    right_value,
                )
            }
            Expression::UnresolvedPath { path, .. } => self.evaluate_import_meta_path(
                module_id,
                profile_id,
                expression_id,
                path,
                import_meta,
            ),
            Expression::ImportMeta => Ok(StaticIfValue::ImportMeta),
            Expression::Member { left, name, .. }
            | Expression::PrivateMember { left, name, .. } => {
                // resolve the left side value
                let left_value =
                    self.evaluate_static_if_value(module_id, profile_id, tree, *left, import_meta)?;
                let Some(name) = *name else {
                    return Err(self.invalid_static_if(
                        module_id,
                        profile_id,
                        expression_id.into_any(),
                        "missing member name in static expression",
                    ));
                };

                // apply member access
                self.evaluate_member_access(
                    module_id,
                    profile_id,
                    expression_id.into_any(),
                    left_value,
                    name,
                    import_meta,
                )
            }
            Expression::Index { left, right } => {
                // resolve the left side value
                let left_value =
                    self.evaluate_static_if_value(module_id, profile_id, tree, *left, import_meta)?;

                // resolve the index value
                let right_id = right.ok_or_else(|| {
                    self.invalid_static_if(
                        module_id,
                        profile_id,
                        expression_id.into_any(),
                        "static if index expression requires a key",
                    )
                })?;
                let right_value = self.evaluate_static_if_value(
                    module_id,
                    profile_id,
                    tree,
                    right_id,
                    import_meta,
                )?;

                // apply index access
                self.evaluate_index_access(
                    module_id,
                    profile_id,
                    expression_id.into_any(),
                    left_value,
                    right_value,
                    import_meta,
                )
            }
            Expression::TemplateExpression { value } => match value {
                destack_dir::TemplateLiteral::String { string } => {
                    Ok(StaticIfValue::Scalar(ScalarLiteral::String(*string)))
                }
                _ => Err(self.invalid_static_if(
                    module_id,
                    profile_id,
                    expression_id.into_any(),
                    "static if does not allow interpolated template literals",
                )),
            },
            _ => Err(self.invalid_static_if(
                module_id,
                profile_id,
                expression_id.into_any(),
                "static if only allows import.meta, literals, and boolean operators",
            )),
        }
    }

    /// Evaluate a binary operator in a static if expression.
    fn evaluate_static_if_binary(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        node_id: LocalNodeIdAny,
        left: StaticIfValue,
        operator: BinaryOperator,
        right: StaticIfValue,
    ) -> ResolveResult<StaticIfValue> {
        // evaluate scalar binary operators
        if let (StaticIfValue::Scalar(left), StaticIfValue::Scalar(right)) = (&left, &right) {
            let Some(result) = evaluate_binary_scalar(operator, left, right) else {
                return Err(self.invalid_static_if(
                    module_id,
                    profile_id,
                    node_id,
                    "static if operator is not supported",
                ));
            };
            return Ok(StaticIfValue::Scalar(result));
        }

        // reject import.meta objects in binary expressions
        if matches!(
            left,
            StaticIfValue::ImportMeta
                | StaticIfValue::ImportMetaEnv
                | StaticIfValue::ImportMetaTarget
        ) || matches!(
            right,
            StaticIfValue::ImportMeta
                | StaticIfValue::ImportMetaEnv
                | StaticIfValue::ImportMetaTarget
        ) {
            return Err(self.invalid_static_if(
                module_id,
                profile_id,
                node_id,
                "static if does not allow import.meta values directly",
            ));
        }

        // handle null and undefined equality operators
        match operator {
            BinaryOperator::Equal | BinaryOperator::EqualStrict => {
                let is_equal = matches!(
                    (left, right),
                    (StaticIfValue::Undefined, StaticIfValue::Undefined)
                        | (StaticIfValue::Null, StaticIfValue::Null)
                );
                Ok(StaticIfValue::Scalar(ScalarLiteral::Boolean(is_equal)))
            }
            BinaryOperator::NotEqual | BinaryOperator::NotEqualStrict => {
                let is_equal = matches!(
                    (left, right),
                    (StaticIfValue::Undefined, StaticIfValue::Undefined)
                        | (StaticIfValue::Null, StaticIfValue::Null)
                );
                Ok(StaticIfValue::Scalar(ScalarLiteral::Boolean(!is_equal)))
            }
            _ => Err(self.invalid_static_if(
                module_id,
                profile_id,
                node_id,
                "static if operator is not supported",
            )),
        }
    }

    /// Evaluate a member access in a static if expression.
    fn evaluate_member_access(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        node_id: LocalNodeIdAny,
        value: StaticIfValue,
        name: destack_core::StringId,
        import_meta: &ImportMeta,
    ) -> ResolveResult<StaticIfValue> {
        // dispatch member access by value
        match value {
            StaticIfValue::ImportKeyword => {
                let meta_name = self.repository.strings.intern("meta");
                if name == meta_name {
                    Ok(StaticIfValue::ImportMeta)
                } else {
                    Err(self.invalid_static_if(
                        module_id,
                        profile_id,
                        node_id,
                        "static if only allows import.meta paths",
                    ))
                }
            }
            StaticIfValue::ImportMeta => {
                self.import_meta_member(module_id, profile_id, node_id, name, import_meta)
            }
            StaticIfValue::ImportMetaEnv => self.import_meta_env_member(name, import_meta),
            StaticIfValue::ImportMetaTarget => {
                self.import_meta_target_member(module_id, profile_id, node_id, name, import_meta)
            }
            _ => Err(self.invalid_static_if(
                module_id,
                profile_id,
                node_id,
                "static if member access requires import.meta",
            )),
        }
    }

    /// Evaluate an index access in a static if expression.
    fn evaluate_index_access(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        node_id: LocalNodeIdAny,
        value: StaticIfValue,
        key: StaticIfValue,
        import_meta: &ImportMeta,
    ) -> ResolveResult<StaticIfValue> {
        // require string literal keys
        let Some(ScalarLiteral::String(key_id)) = key.as_scalar() else {
            return Err(self.invalid_static_if(
                module_id,
                profile_id,
                node_id,
                "static if index keys must be string literals",
            ));
        };

        // route index access by object type
        match value {
            StaticIfValue::ImportMetaEnv => self.import_meta_env_member(*key_id, import_meta),
            StaticIfValue::ImportMetaTarget => {
                self.import_meta_target_member(module_id, profile_id, node_id, *key_id, import_meta)
            }
            _ => Err(self.invalid_static_if(
                module_id,
                profile_id,
                node_id,
                "static if index access requires import.meta.env or import.meta.target",
            )),
        }
    }

    /// Evaluate an import.meta path in a static if expression.
    fn evaluate_import_meta_path(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        expression_id: LocalNodeId<Expression>,
        path: &destack_dir::Path,
        import_meta: &ImportMeta,
    ) -> ResolveResult<StaticIfValue> {
        let undefined_name = self.repository.strings.intern("undefined");
        let null_name = self.repository.strings.intern("null");
        // validate the import.meta prefix
        let import_name = self.repository.strings.intern("import");
        let meta_name = self.repository.strings.intern("meta");

        // keep single-segment intrinsic values available in static conditions
        if path.segments.len() == 1 {
            let segment = path.segments[0];
            if segment == undefined_name {
                return Ok(StaticIfValue::Undefined);
            }
            if segment == null_name {
                return Ok(StaticIfValue::Null);
            }
            if segment == import_name {
                return Ok(StaticIfValue::ImportKeyword);
            }
        }
        if path.segments.len() < 2
            || path.segments[0] != import_name
            || path.segments[1] != meta_name
        {
            return Err(self.invalid_static_if(
                module_id,
                profile_id,
                expression_id.into_any(),
                "static if only allows import.meta paths",
            ));
        }

        // allow the bare import.meta object
        if path.segments.len() == 2 {
            return Ok(StaticIfValue::ImportMeta);
        }

        // walk remaining path segments as members
        let mut value = StaticIfValue::ImportMeta;
        for segment in path.segments.iter().skip(2) {
            value = self.evaluate_member_access(
                module_id,
                profile_id,
                expression_id.into_any(),
                value,
                *segment,
                import_meta,
            )?;
        }
        Ok(value)
    }

    /// Resolve an import.meta member value.
    fn import_meta_member(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        node_id: LocalNodeIdAny,
        name: destack_core::StringId,
        import_meta: &ImportMeta,
    ) -> ResolveResult<StaticIfValue> {
        // resolve the import.meta property name
        let url_name = self.repository.strings.intern("url");
        let path_name = self.repository.strings.intern("path");
        let file_name = self.repository.strings.intern("file");
        let filename_name = self.repository.strings.intern("filename");
        let dir_name = self.repository.strings.intern("dir");
        let dirname_name = self.repository.strings.intern("dirname");
        let emit_key = self.repository.strings.intern("emit");
        let platform_key = self.repository.strings.intern("platform");
        let runtime_key = self.repository.strings.intern("runtime");
        let debug_name = self.repository.strings.intern("debug");
        let test_name = self.repository.strings.intern("test");
        let env_name = self.repository.strings.intern("env");
        let target_name = self.repository.strings.intern("target");

        // match against interned names
        match name {
            id if id == url_name => Ok(self.static_string_literal(import_meta.url.as_ref())),
            id if id == path_name => Ok(self.optional_path_literal(import_meta.path.as_ref())),
            id if id == file_name => Ok(self.optional_path_literal(import_meta.file.as_ref())),
            id if id == filename_name => {
                Ok(self.optional_path_literal(import_meta.filename.as_ref()))
            }
            id if id == dir_name => Ok(self.optional_path_literal(import_meta.dir.as_ref())),
            id if id == dirname_name => {
                Ok(self.optional_path_literal(import_meta.dirname.as_ref()))
            }
            id if id == emit_key => Ok(self.static_string_literal(emit_name(import_meta.emit))),
            id if id == platform_key => {
                Ok(self.static_string_literal(platform_name(import_meta.platform)))
            }
            id if id == runtime_key => {
                Ok(self.static_string_literal(runtime_name(import_meta.runtime)))
            }
            id if id == debug_name => Ok(StaticIfValue::Scalar(ScalarLiteral::Boolean(
                import_meta.debug,
            ))),
            id if id == test_name => Ok(StaticIfValue::Scalar(ScalarLiteral::Boolean(
                import_meta.test,
            ))),
            id if id == env_name => Ok(StaticIfValue::ImportMetaEnv),
            id if id == target_name => Ok(StaticIfValue::ImportMetaTarget),
            _ => Err(self.invalid_static_if(
                module_id,
                profile_id,
                node_id,
                "static if does not support this import.meta property",
            )),
        }
    }

    /// Resolve an import.meta.env member value.
    fn import_meta_env_member(
        &self,
        name: destack_core::StringId,
        import_meta: &ImportMeta,
    ) -> ResolveResult<StaticIfValue> {
        // resolve the import.meta.env property name
        let dev_name = self.repository.strings.intern("DEV");
        let prod_name = self.repository.strings.intern("PROD");
        let test_name = self.repository.strings.intern("TEST");
        let node_env_name = self.repository.strings.intern("NODE_ENV");
        match name {
            id if id == dev_name => Ok(StaticIfValue::Scalar(ScalarLiteral::Boolean(
                import_meta.env.dev,
            ))),
            id if id == prod_name => Ok(StaticIfValue::Scalar(ScalarLiteral::Boolean(
                import_meta.env.prod,
            ))),
            id if id == test_name => Ok(StaticIfValue::Scalar(ScalarLiteral::Boolean(
                import_meta.env.test,
            ))),
            id if id == node_env_name => {
                Ok(self.optional_string_literal(import_meta.env.node_env.as_deref()))
            }
            _ => {
                // read the string outside the env lookup path
                let name_str = self.repository.strings.get(name);
                let name_owned = name_str.as_str().to_string();
                drop(name_str);
                Ok(self.env_lookup(name_owned.as_str(), &import_meta.env))
            }
        }
    }

    /// Resolve an import.meta.target member value.
    fn import_meta_target_member(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        node_id: LocalNodeIdAny,
        name: destack_core::StringId,
        import_meta: &ImportMeta,
    ) -> ResolveResult<StaticIfValue> {
        // resolve the import.meta.target property name
        let family_name = self.repository.strings.intern("family");
        let vendor_name = self.repository.strings.intern("vendor");
        let env_name = self.repository.strings.intern("env");
        let abi_name = self.repository.strings.intern("abi");
        let arch_name = self.repository.strings.intern("arch");
        match name {
            id if id == family_name => Ok(self.static_string_literal(&import_meta.target.family)),
            id if id == vendor_name => Ok(self.static_string_literal(&import_meta.target.vendor)),
            id if id == env_name => {
                Ok(self.optional_string_literal(import_meta.target.env.as_deref()))
            }
            id if id == abi_name => {
                Ok(self.optional_string_literal(import_meta.target.abi.as_deref()))
            }
            id if id == arch_name => {
                Ok(self.optional_string_literal(import_meta.target.arch.as_deref()))
            }
            _ => Err(self.invalid_static_if(
                module_id,
                profile_id,
                node_id,
                "static if does not support this import.meta.target property",
            )),
        }
    }

    /// Convert a string into a static scalar literal.
    fn static_string_literal(&self, value: &str) -> StaticIfValue {
        // intern string values for scalar literals
        let id = self.repository.strings.intern(value);
        StaticIfValue::Scalar(ScalarLiteral::String(id))
    }

    /// Convert an optional path into a static string literal.
    fn optional_path_literal(&self, value: Option<&std::path::PathBuf>) -> StaticIfValue {
        // return undefined when the path is missing
        let Some(value) = value else {
            return StaticIfValue::Undefined;
        };

        // convert the path into a string literal
        let string = value.to_string_lossy();
        self.static_string_literal(&string)
    }

    /// Convert an optional string into a static string literal.
    fn optional_string_literal(&self, value: Option<&str>) -> StaticIfValue {
        // return undefined when the env entry is missing
        let Some(value) = value else {
            return StaticIfValue::Undefined;
        };

        // convert the value into a string literal
        self.static_string_literal(value)
    }

    /// Look up a profile env value by key.
    fn env_lookup(&self, key: &str, env: &ProfileEnv) -> StaticIfValue {
        // scan env entries for a matching key
        for (entry_key, value) in &env.values {
            if entry_key == key {
                return self.static_string_literal(value);
            }
        }

        StaticIfValue::Undefined
    }

    /// Create an invalid static if error.
    fn invalid_static_if(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        node_id: LocalNodeIdAny,
        message: &str,
    ) -> ResolveError {
        // build the error payload
        ResolveError::InvalidStaticIf {
            node: node_id.into_anchored(module_id, Some(profile_id)),
            message: message.to_string(),
        }
    }
}

/// Return the emit name used by import.meta.
fn emit_name(emit: EmitFormat) -> &'static str {
    // map emit formats to their import.meta names
    match emit {
        EmitFormat::Js => "js",
        EmitFormat::Ts => "ts",
        EmitFormat::Html => "html",
        EmitFormat::Wasm => "wasm",
        EmitFormat::Native => "native",
    }
}

/// Return the platform name used by import.meta.
fn platform_name(platform: Platform) -> &'static str {
    // map platforms to their import.meta names
    match platform {
        Platform::Web => "web",
        Platform::Windows => "windows",
        Platform::MacOS => "macos",
        Platform::Linux => "linux",
        Platform::FreeBsd => "freebsd",
        Platform::OpenBsd => "openbsd",
        Platform::NetBsd => "netbsd",
        Platform::DragonFly => "dragonfly",
        Platform::Solaris => "solaris",
        Platform::Illumos => "illumos",
        Platform::Haiku => "haiku",
        Platform::Fuchsia => "fuchsia",
        Platform::Redox => "redox",
        Platform::Hermit => "hermit",
        Platform::IOS => "ios",
        Platform::Android => "android",
        Platform::Wasi => "wasi",
        Platform::Emscripten => "emscripten",
        Platform::BareMetal => "bare-metal",
        Platform::Universal => "universal",
    }
}

/// Return the runtime name used by import.meta.
fn runtime_name(runtime: Runtime) -> &'static str {
    // map runtimes to their import.meta names
    match runtime {
        Runtime::Browser => "browser",
        Runtime::Node => "node",
        Runtime::Deno => "deno",
        Runtime::Bun => "bun",
        Runtime::Worker => "worker",
        Runtime::WasmJs => "wasm-js",
        Runtime::WasmWasi => "wasm-wasi",
        Runtime::NativeManaged => "native-managed",
        Runtime::NativeFreestanding => "native-freestanding",
        Runtime::NativeEmbedded => "native-embedded",
    }
}
