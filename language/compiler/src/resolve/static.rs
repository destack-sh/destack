use std::collections::HashSet;

use destack_dir::{
    Annotation, BinaryOperator, Block, Declaration, Expression, LocalNodeId, LocalNodeIdAny,
    NodeTree, NodeType, NodeVisitor, NodeVisitorOptions, ScalarLiteral, SymbolTable, Type,
    TypeLiteral, TypeTable, walk_any,
};
use destack_source::ModuleId;
use destack_workspace::{ImportMeta, OutputFormat, Platform, ProfileEnv, ProfileId, Runtime};

use crate::{Compiler, ResolveError, ResolveResult, evaluate_binary_scalar, evaluate_unary_scalar};

/// A value computed by static if evaluation.
#[derive(Debug, Clone, PartialEq)]
enum StaticIfValue {
    /// A scalar literal value.
    Scalar(ScalarLiteral),
    /// The null literal value.
    Null,
    /// The undefined literal value.
    Undefined,
    /// The import.meta object.
    ImportMeta,
    /// The import.meta.env object.
    ImportMetaEnv,
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
    fn collect(&mut self, tree: &NodeTree, root: LocalNodeIdAny) {
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
    fn visit_any(&mut self, _tree: &NodeTree, ty: NodeType, id: u32) {
        self.nodes.push(LocalNodeIdAny::new(id, ty));
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Apply static if decorators to the profile dir.
    pub(super) fn apply_static_if_decorators(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        dir: &mut destack_workspace::ModuleDir,
    ) -> ResolveResult<()> {
        // exit early when import meta is missing
        let Some(import_meta) = dir.import_meta.as_ref() else {
            return Ok(());
        };

        // acquire the tree, symbols, and types for this dir
        let mut tree = dir.tree.write();
        let mut symbols = dir.symbols.write();
        let types = dir.types.read();

        // validate decorator placement before applying filters
        self.validate_static_if_placement(module_id, profile_id, &tree)?;

        // collect static if targets
        let if_name = self.program.strings.intern("if");
        let mut declaration_targets = Vec::new();
        let mut expression_targets = Vec::new();
        let mut seen_declarations = HashSet::new();
        let mut seen_expressions = HashSet::new();

        // gather parent nodes for @if annotations
        for annotation_id in tree.iter_node_ids_of_type::<Annotation>() {
            if self
                .decorator_call_named(&tree, annotation_id, if_name)
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
                NodeType::Member | NodeType::EnumField => {
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
                &tree,
                import_meta,
            )?;

            // deactivate declarations that are gated out
            if matches!(condition, Some(false)) {
                self.deactivate_declaration(&mut tree, &mut symbols, &types, declaration_id);
                continue;
            }

            // filter nested members and fields
            self.filter_declaration_members(
                module_id,
                profile_id,
                &mut tree,
                &mut symbols,
                &types,
                declaration_id,
                import_meta,
            )?;
        }

        // collect expression ids that can be gated
        let mut allowed_expressions = HashSet::new();

        // include module root expressions
        for root_id in dir.roots.iter().copied() {
            allowed_expressions.insert(root_id.id);
        }

        // include block statement expressions
        for block_id in tree.iter_node_ids_of_type::<Block>() {
            let block = tree.get(block_id);

            // record each block expression id
            for expression_id in block.expressions.iter().copied() {
                allowed_expressions.insert(expression_id.id);
            }
        }

        // find expressions gated out by static if
        let mut removed_expressions = HashSet::new();
        for expression_id in expression_targets {
            // skip expressions under inactive declarations
            if !self.is_node_active(&tree, &symbols, expression_id.into_any()) {
                continue;
            }

            // evaluate the expression gate
            let condition = self.static_if_condition_for_node(
                module_id,
                profile_id,
                expression_id.into_any(),
                &tree,
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
                    Expression::Declaration { declaration } => Some(*declaration),
                    _ => None,
                };

                // mark the expression node as inactive
                self.mark_inactive_subtree(&mut tree, &types, expression_id.into_any());

                removed_expressions.insert(expression_id.id);

                // deactivate declarations gated out at the expression level
                if let Some(declaration_id) = declaration_id {
                    self.deactivate_declaration(&mut tree, &mut symbols, &types, declaration_id);
                }
            }
        }

        // filter module roots by gating and declaration activity
        let mut kept_roots = Vec::with_capacity(dir.roots.len());
        for root_id in dir.roots.iter().copied() {
            // drop expressions gated out explicitly
            if removed_expressions.contains(&root_id.id) {
                continue;
            }

            // drop inactive declaration roots
            let Expression::Declaration { declaration } = tree.get(root_id) else {
                kept_roots.push(root_id);
                continue;
            };
            let declaration = tree.get(*declaration);
            if symbols.get_active_symbol(declaration.symbol()).is_none() {
                continue;
            }

            kept_roots.push(root_id);
        }
        dir.roots = kept_roots;

        // filter block expressions by gating and declaration activity
        let block_ids: Vec<_> = tree.iter_node_ids_of_type::<Block>();
        for block_id in block_ids {
            // rebuild the block expression list
            let expression_ids = tree.get(block_id).expressions.clone();
            let mut kept = Vec::with_capacity(expression_ids.len());

            for expression_id in expression_ids {
                // skip expressions gated out explicitly
                if removed_expressions.contains(&expression_id.id) {
                    continue;
                }

                // skip inactive declaration expressions
                if let Expression::Declaration { declaration } = tree.get(expression_id) {
                    let declaration = tree.get(*declaration);
                    if symbols.get_active_symbol(declaration.symbol()).is_none() {
                        continue;
                    }
                }

                kept.push(expression_id);
            }

            tree.get_mut(block_id).expressions = kept;
        }

        // skip resolving static if annotations in later passes
        self.mark_static_if_annotations_inactive(&mut tree, &types);

        Ok(())
    }

    /// Validate that static if decorators appear on supported nodes.
    fn validate_static_if_placement(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        tree: &NodeTree,
    ) -> ResolveResult<()> {
        // cache the decorator identifier
        let if_name = self.program.strings.intern("if");

        // walk all annotations looking for @if decorators
        for annotation_id in tree.iter_node_ids_of_type::<Annotation>() {
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
                | destack_dir::NodeType::EnumField => {}
                destack_dir::NodeType::Expression => {
                    let expression_id = LocalNodeId::<Expression>::new(parent_id.id);
                    if !matches!(
                        tree.get(expression_id),
                        Expression::Statement { .. } | Expression::Declaration { .. }
                    ) {
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

    /// Mark static if annotations inactive after they are processed.
    fn mark_static_if_annotations_inactive(&self, tree: &mut NodeTree, types: &TypeTable) {
        // cache the decorator identifier
        let if_name = self.program.strings.intern("if");

        // mark @if annotations inactive
        for annotation_id in tree.iter_node_ids_of_type::<Annotation>() {
            if self
                .decorator_call_named(tree, annotation_id, if_name)
                .is_none()
            {
                continue;
            }

            self.mark_inactive_subtree(tree, types, annotation_id.into_any());
        }
    }

    /// Mark a subtree and its annotations inactive.
    fn mark_inactive_subtree(&self, tree: &mut NodeTree, types: &TypeTable, root: LocalNodeIdAny) {
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

        // collect annotation roots attached to the subtree
        let mut annotation_roots = Vec::new();
        for node_id in inactive_ids.iter().copied() {
            let annotations = tree.get_annotations(node_id);
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
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &TypeTable,
        declaration_id: LocalNodeId<Declaration>,
        import_meta: &ImportMeta,
    ) -> ResolveResult<()> {
        // capture the member and field lists
        let (members, fields) = {
            let declaration = tree.get(declaration_id);
            let members = match declaration {
                Declaration::Struct { members, .. }
                | Declaration::Class { members, .. }
                | Declaration::Enum { members, .. }
                | Declaration::Interface { members, .. }
                | Declaration::Extension { members, .. } => members.clone(),
                _ => Vec::new(),
            };
            let fields = match declaration {
                Declaration::Enum { fields, .. } => fields.clone(),
                _ => Vec::new(),
            };
            (members, fields)
        };

        // filter members based on static if
        let mut filtered_members = Vec::with_capacity(members.len());
        for member_id in members {
            // keep members without annotations
            if !tree.has_annotations(member_id.id) {
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
                    let member = tree.get(member_id);
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

        // filter enum fields based on static if
        let mut filtered_fields = Vec::with_capacity(fields.len());
        for field_id in fields {
            // keep fields without annotations
            if !tree.has_annotations(field_id.id) {
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
                let symbol_id = {
                    let field = tree.get(field_id);
                    field.symbol
                };
                let symbol = symbols.get_symbol_mut(symbol_id);
                symbol.is_active = false;

                // mark the enum field subtree inactive
                self.mark_inactive_subtree(tree, types, field_id.into_any());
                continue;
            }

            filtered_fields.push(field_id);
        }

        // update the declaration slots
        let declaration = tree.get_mut(declaration_id);
        match declaration {
            Declaration::Struct { members: slot, .. }
            | Declaration::Class { members: slot, .. }
            | Declaration::Interface { members: slot, .. }
            | Declaration::Extension { members: slot, .. }
            | Declaration::Enum { members: slot, .. } => {
                *slot = filtered_members;
            }
            _ => {}
        }

        if let Declaration::Enum { fields: slot, .. } = declaration {
            *slot = filtered_fields;
        }

        Ok(())
    }

    /// Deactivate a declaration and its members for the profile.
    fn deactivate_declaration(
        &self,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &TypeTable,
        declaration_id: LocalNodeId<Declaration>,
    ) {
        // collect declaration ids for later updates
        let (symbol_id, members, fields) = {
            let declaration = tree.get(declaration_id);
            let symbol_id = declaration.symbol();
            let members = declaration.member_ids().map(|members| members.to_vec());
            let fields = match declaration {
                Declaration::Enum { fields, .. } => Some(fields.clone()),
                _ => None,
            };
            (symbol_id, members, fields)
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

        // deactivate enum field symbols
        if let Some(fields) = fields {
            for field_id in fields {
                let field = tree.get(field_id);
                let symbol = symbols.get_symbol_mut(field.symbol);
                symbol.is_active = false;
            }
        }
    }

    /// Evaluate the static if condition for a node.
    fn static_if_condition_for_node(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        node_id: LocalNodeIdAny,
        tree: &NodeTree,
        import_meta: &ImportMeta,
    ) -> ResolveResult<Option<bool>> {
        // track whether any @if decorators were encountered
        let mut saw_if = false;
        let mut combined = true;

        // iterate annotations for the node
        let annotations = tree.get_annotations(node_id.id);
        let if_name = self.program.strings.intern("if");
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
        tree: &NodeTree,
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

                // apply member access
                self.evaluate_member_access(
                    module_id,
                    profile_id,
                    expression_id.into_any(),
                    left_value,
                    *name,
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
            StaticIfValue::ImportMeta | StaticIfValue::ImportMetaEnv
        ) || matches!(
            right,
            StaticIfValue::ImportMeta | StaticIfValue::ImportMetaEnv
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
        name: destack_base::StringId,
        import_meta: &ImportMeta,
    ) -> ResolveResult<StaticIfValue> {
        // dispatch member access by value
        match value {
            StaticIfValue::ImportMeta => {
                self.import_meta_member(module_id, profile_id, node_id, name, import_meta)
            }
            StaticIfValue::ImportMetaEnv => self.import_meta_env_member(name, import_meta),
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
        // require import.meta.env on index access
        let StaticIfValue::ImportMetaEnv = value else {
            return Err(self.invalid_static_if(
                module_id,
                profile_id,
                node_id,
                "static if index access requires import.meta.env",
            ));
        };

        // require string literal keys
        let Some(ScalarLiteral::String(key_id)) = key.as_scalar() else {
            return Err(self.invalid_static_if(
                module_id,
                profile_id,
                node_id,
                "static if index keys must be string literals",
            ));
        };

        // reuse import.meta.env lookup
        self.import_meta_env_member(*key_id, import_meta)
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
        // validate the import.meta prefix
        let import_name = self.program.strings.intern("import");
        let meta_name = self.program.strings.intern("meta");
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
        name: destack_base::StringId,
        import_meta: &ImportMeta,
    ) -> ResolveResult<StaticIfValue> {
        // resolve the import.meta property name
        let url_name = self.program.strings.intern("url");
        let path_name = self.program.strings.intern("path");
        let file_name = self.program.strings.intern("file");
        let filename_name = self.program.strings.intern("filename");
        let dir_name = self.program.strings.intern("dir");
        let dirname_name = self.program.strings.intern("dirname");
        let output_key = self.program.strings.intern("output");
        let platform_key = self.program.strings.intern("platform");
        let runtime_key = self.program.strings.intern("runtime");
        let debug_name = self.program.strings.intern("debug");
        let test_name = self.program.strings.intern("test");
        let env_name = self.program.strings.intern("env");

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
            id if id == output_key => {
                Ok(self.static_string_literal(output_name(import_meta.output)))
            }
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
        name: destack_base::StringId,
        import_meta: &ImportMeta,
    ) -> ResolveResult<StaticIfValue> {
        // resolve the import.meta.env property name
        let dev_name = self.program.strings.intern("DEV");
        let prod_name = self.program.strings.intern("PROD");
        let test_name = self.program.strings.intern("TEST");
        let node_env_name = self.program.strings.intern("NODE_ENV");
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
                Ok(self.optional_env_literal(import_meta.env.node_env.as_deref()))
            }
            _ => {
                // read the string outside the env lookup path
                let name_str = self.program.strings.get(name);
                let name_owned = name_str.as_str().to_string();
                drop(name_str);
                Ok(self.env_lookup(name_owned.as_str(), &import_meta.env))
            }
        }
    }

    /// Convert a string into a static scalar literal.
    fn static_string_literal(&self, value: &str) -> StaticIfValue {
        // intern string values for scalar literals
        let id = self.program.strings.intern(value);
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

    /// Convert an optional env string into a static string literal.
    fn optional_env_literal(&self, value: Option<&str>) -> StaticIfValue {
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

/// Return the output name used by import.meta.
fn output_name(output: OutputFormat) -> &'static str {
    // map outputs to their import.meta names
    match output {
        OutputFormat::Js => "js",
        OutputFormat::Ts => "ts",
        OutputFormat::Wasm => "wasm",
        OutputFormat::Native => "native",
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
        Platform::IOS => "ios",
        Platform::Android => "android",
        Platform::Wasi => "wasi",
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
        Runtime::NativeHosted => "native-hosted",
        Runtime::NativeFreestanding => "native-freestanding",
        Runtime::NativeEmbedded => "native-embedded",
    }
}

#[cfg(test)]
mod tests {
    use destack_dir::{FunctionMode, Member};
    use destack_workspace::OutputFormat;

    use crate::tests::TestProgram;

    fn test_program_js() -> TestProgram {
        let mut test = TestProgram::memory_sequential();
        let default_profile = test.program.profile(test.default_profile_id_for_root());
        let mut key = default_profile.key.clone();
        key.output = OutputFormat::Js;
        let profile_id = test.program.profiles.get_or_create(key);
        test.default_profile_override = Some(profile_id);
        test
    }

    /// Gate declarations based on static if conditions.
    #[test]
    fn test_static_if_gates_declaration() {
        // build the test program
        let test = test_program_js();
        let module_id = test.add_module(
            "test.ds",
            r#"
@if(import.meta.output == "native")
struct Hidden {
    value: number;
}

struct Visible {}
"#,
        );

        // resolve the module
        test.resolve_module(module_id);
        test.compile_check_clean();

        // confirm the filtered declaration symbols
        assert!(test.resolve_to_symbol("test.ds", "Hidden").is_none());
        assert!(test.resolve_to_symbol("test.ds", "Visible").is_some());
    }

    /// Gate module statements based on static if conditions.
    #[test]
    fn test_static_if_gates_statement() {
        // build the test program
        let test = test_program_js();
        let module_id = test.add_module(
            "test.ds",
            r#"
@if(import.meta.output == "native")
missing_symbol();

const value = 1;
"#,
        );

        // resolve the module
        test.resolve_module(module_id);
        test.compile_check_clean();

        // confirm other declarations still resolve
        assert!(test.resolve_to_symbol("test.ds", "value").is_some());
    }

    /// Gate block statements based on static if conditions.
    #[test]
    fn test_static_if_gates_block_statement() {
        // build the test program
        let test = test_program_js();
        let module_id = test.add_module(
            "test.ds",
            r#"
function demo(): number {
    @if(import.meta.output == "native")
    missing_symbol();

    return 1;
}
"#,
        );

        // resolve the module
        test.resolve_module(module_id);
        test.compile_check_clean();
    }

    /// Report errors when a static if condition is true.
    #[test]
    fn test_static_if_errors_when_true() {
        // build the test program
        let test = test_program_js();
        let module_id = test.add_module(
            "test.ds",
            r#"
@if(import.meta.output == "js")
missing_symbol();
"#,
        );

        // resolve the module
        test.resolve_module(module_id);
        test.compile();

        // confirm the missing symbol diagnostic
        test.check_has_diagnostic("ER101");
    }

    /// Reject non boolean static if conditions.
    #[test]
    fn test_static_if_requires_boolean() {
        // build the test program
        let test = test_program_js();
        let module_id = test.add_module(
            "test.ds",
            r#"
@if(1)
const value = 1;
"#,
        );

        // resolve the module
        test.resolve_module(module_id);
        test.compile();

        // confirm the static if diagnostic
        test.check_has_diagnostic("ER901");
    }

    /// Reject static if decorators without arguments.
    #[test]
    fn test_static_if_requires_argument() {
        // build the test program
        let test = test_program_js();
        let module_id = test.add_module(
            "test.ds",
            r#"
@if
const value = 1;
"#,
        );

        // resolve the module
        test.resolve_module(module_id);
        test.compile();

        // confirm the static if diagnostic
        test.check_has_diagnostic("ER901");
    }

    /// Gate class members based on static if conditions.
    #[test]
    fn test_static_if_gates_members() {
        // build the test program
        let test = test_program_js();
        let module_id = test.add_module(
            "test.ds",
            r#"
class Box {
    @if(import.meta.output == "native")
    missing: MissingType;

    value: number;
}
"#,
        );

        // resolve the module
        test.resolve_module(module_id);
        test.compile_check_clean();
    }

    /// Combine multiple static if decorators on a declaration.
    #[test]
    fn test_static_if_combines_conditions() {
        // build the test program
        let test = test_program_js();
        let module_id = test.add_module(
            "test.ds",
            r#"
@if(import.meta.output == "js")
@if(import.meta.output == "native")
const value = missing_symbol();
"#,
        );

        // resolve the module
        test.resolve_module(module_id);
        test.compile_check_clean();
    }

    /// Attach static if decorators to accessor members.
    #[test]
    fn test_static_if_attaches_to_accessor_members() {
        // build the test program
        let test = test_program_js();
        let module_id = test.add_module(
            "test.ds",
            r#"
class Box {
    @if(import.meta.output == "js" && import.meta.output == "native")
    get value(): MissingType {
        return missingSymbol;
    }

    @if(import.meta.output == "js" && import.meta.output == "native")
    set value(next: MissingType) {
        missingSymbol;
    }
}
"#,
        );

        // resolve the module
        test.resolve_module(module_id);
        test.compile();

        // locate accessor members in the dir
        test.with_dir_read(
            module_id,
            |_module, _profile, _dir, tree, _symbols, _types| {
                // initialize accessor ids
                let mut getter_id = None;
                let mut setter_id = None;

                // scan members for accessor nodes
                for member_id in tree.iter_node_ids_of_type::<Member>() {
                    let Member::Method { signature, .. } = tree.get(member_id) else {
                        continue;
                    };
                    // record getter and setter members
                    match signature.mode {
                        Some(FunctionMode::Getter) => getter_id = Some(member_id),
                        Some(FunctionMode::Setter) => setter_id = Some(member_id),
                        _ => {}
                    }
                }

                // confirm both accessors were parsed
                let getter_id = getter_id.expect("expected getter member");
                let setter_id = setter_id.expect("expected setter member");

                // confirm @if annotations were attached
                assert!(
                    tree.has_annotations(getter_id.id),
                    "expected getter annotations"
                );
                assert!(
                    tree.has_annotations(setter_id.id),
                    "expected setter annotations"
                );

                // confirm accessors were gated out
                assert!(tree.is_inactive(getter_id.id), "expected getter inactive");
                assert!(tree.is_inactive(setter_id.id), "expected setter inactive");
            },
        );

        // confirm no diagnostics after gating
        test.check_clean();
    }
}
