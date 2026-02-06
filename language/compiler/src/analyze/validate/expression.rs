use crate::analyze::common::{NormalizationMode, RelationMode};
use crate::{AnalyzeError, AnalyzeOptions, Compiler};
use destack_dir::{
    Argument, Asynchrony, BinaryOperator, BindingKind, Declaration, DeclarationKind, Declarator,
    DependencyItem, DependencyKind, DependencyMode, DependencySource, DynamicKey, Expression,
    ForEachBinding, ForEachKind, GlobalSymbolId, LocalNodeId, LocalNodeIdAny, MatchCase, MatchKind,
    MatchSelector, Member, Mutability, NodeTree, NodeType, Path, Pattern, Property,
    RuntimeCheckKind, ScalarLiteral, StaticKey, StringId, SymbolTable, SymbolType, TemplateLiteral,
    Type, TypeBinaryOperator, TypeLiteral, TypeTable, TypeUnaryOperator, UnaryOperator,
};
use destack_workspace::{Module, ProfileId};
use std::collections::HashSet;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Validate a single expression node.
    pub(super) fn validate_expression(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        options: AnalyzeOptions,
        expression_id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        // cache strict mode once per expression validation
        let is_strict = module.source_type.is_module() || options.always_strict;

        match expression {
            Expression::Labelled { label, .. } => {
                self.validate_duplicate_label(module, profile, tree, expression_id, *label);
            }
            Expression::Try { catch_pattern, .. } => {
                if let Some(catch_pattern_id) = catch_pattern {
                    self.validate_catch_annotation_type(
                        module,
                        profile,
                        tree,
                        expression_id,
                        *catch_pattern_id,
                    );
                }
            }
            Expression::Assign { left, .. } => {
                self.validate_assignment_target(module, profile, tree, *left, is_strict);
            }
            Expression::AssignBinary { left, .. } => {
                self.validate_assignment_target(module, profile, tree, *left, is_strict);
            }
            Expression::Call { left, .. } => {
                self.validate_super_call_expression(module, profile, tree, expression_id, *left);
                self.validate_super_property_expression(module, profile, tree, expression_id);
            }
            Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Index { left, .. } => {
                self.validate_super_property_expression(module, profile, tree, expression_id);
                self.validate_instantiation_access(
                    module,
                    profile,
                    tree,
                    types,
                    expression_id,
                    *left,
                );
            }
            Expression::Maybe { left } => {
                self.validate_super_optional_chain(module, profile, tree, expression_id, *left);
            }
            Expression::PrivateIdentifier { .. } => {
                self.validate_private_identifier_expression(module, profile, tree, expression_id);
            }
            Expression::Match {
                kind, value, cases, ..
            } => {
                self.validate_match_expression(
                    module,
                    profile,
                    tree,
                    symbols,
                    types,
                    expression_id,
                    *kind,
                    *value,
                    cases,
                );
            }
            Expression::Must { .. } => {
                self.validate_must_assertion(
                    module,
                    profile,
                    tree,
                    expression_id,
                    options.no_must_assertions,
                );
            }
            Expression::TypePredicate { .. } => {
                self.validate_custom_type_guard(
                    module,
                    profile,
                    expression_id,
                    options.no_custom_type_guards,
                );
            }
            Expression::Binary { left, operator, .. } => {
                self.validate_unsound_narrowing_operator(
                    module,
                    profile,
                    types,
                    expression_id,
                    *operator,
                    options.no_unsound_narrowing,
                );
                self.validate_exponent_left_operand(
                    module,
                    profile,
                    tree,
                    expression_id,
                    *left,
                    *operator,
                );
            }
            Expression::TypeBinary { operator, .. } => {
                self.validate_unsound_narrowing_type_operator(
                    module,
                    profile,
                    types,
                    expression_id,
                    *operator,
                    options.no_unsound_narrowing,
                );

                // reject satisfies expressions in javascript modules
                if module.language_type.is_javascript()
                    && *operator == TypeBinaryOperator::Satisfies
                {
                    let node = expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(profile));
                    self.error(AnalyzeError::TypeScriptSyntaxInJavaScript { node });
                }
            }
            Expression::ExportNamespace { .. } => {
                if !module.language_type.is_declaration() {
                    self.error(AnalyzeError::ExportNamespaceOutsideDeclaration {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(profile)),
                    });
                }
            }
            Expression::Import {
                source,
                kind,
                items,
                ..
            }
            | Expression::UnresolvedImport {
                source,
                kind,
                items,
                ..
            } => {
                self.validate_dependency_expression(
                    module,
                    profile,
                    tree,
                    expression_id,
                    *source,
                    *kind,
                    Some(items),
                );
            }
            Expression::Export { kind, .. }
            | Expression::ReExport { kind, .. }
            | Expression::UnresolvedReExport { kind, .. } => {
                self.validate_dependency_expression(
                    module,
                    profile,
                    tree,
                    expression_id,
                    DependencySource::ExportStatement,
                    *kind,
                    None,
                );
            }
            Expression::ObjectExpression { properties }
            | Expression::TaggedObjectExpression { properties, .. } => {
                self.validate_object_literal_properties(module, profile, tree, properties);
            }
            Expression::TypeUnary {
                operator: TypeUnaryOperator::Readonly,
                right,
            } => {
                self.validate_readonly_type_operator(module, profile, tree, expression_id, *right);
            }
            Expression::Unary { operator, right } => {
                self.validate_update_target(module, profile, tree, *operator, *right, is_strict);
            }
            Expression::TypeIndex { left, .. } => {
                self.validate_intrinsic_type_index(module, profile, tree, expression_id, *left);
                self.validate_type_index_access(
                    module,
                    profile,
                    tree,
                    symbols,
                    types,
                    expression_id,
                );
            }
            Expression::TypeImport {
                target, qualifier, ..
            } => {
                let target_string = self.validate_type_import_target_expression(
                    module,
                    profile,
                    tree,
                    expression_id,
                    *target,
                );
                if let Some(target_string) = target_string {
                    self.validate_type_import_expression(
                        module,
                        profile,
                        types,
                        expression_id,
                        target_string,
                        qualifier.as_ref(),
                    );
                }
            }
            Expression::ArrayExpression { elements } | Expression::TupleExpression { elements } => {
                self.validate_tuple_optional_order(module, profile, tree, expression_id, elements);
                if matches!(expression, Expression::TupleExpression { .. }) {
                    self.validate_empty_parenthesized_expression(
                        module,
                        profile,
                        expression_id,
                        elements,
                    );
                }
            }
            Expression::SequenceExpression { expressions } => {
                self.validate_empty_parenthesized_sequence(
                    module,
                    profile,
                    expression_id,
                    expressions,
                );
            }
            Expression::Delete { value } => {
                self.validate_delete_expression(
                    module,
                    profile,
                    tree,
                    expression_id,
                    *value,
                    is_strict,
                );
            }
            Expression::TaggedTemplateExpression { tag, .. } => {
                self.validate_tagged_template_expression(
                    module,
                    profile,
                    tree,
                    expression_id,
                    *tag,
                );
            }
            Expression::Let {
                descriptor,
                mutability,
                declarators,
                ..
            } => {
                let is_in_declare_namespace =
                    self.is_in_declare_namespace(tree, expression_id.into_any());
                let is_in_declare_module =
                    self.is_in_declare_module(tree, expression_id.into_any());
                let is_declare_context =
                    descriptor.kind == DeclarationKind::Declaration || is_in_declare_namespace;
                let allow_ambient_const_initializers = is_in_declare_module;
                for declarator_id in declarators {
                    self.validate_definite_assignment_declarator(
                        module,
                        profile,
                        tree,
                        *declarator_id,
                        options.no_definite_assignment_assertions,
                    );
                    self.validate_declare_binding_initializer(
                        module,
                        profile,
                        tree,
                        *declarator_id,
                        *mutability,
                        is_declare_context,
                        allow_ambient_const_initializers,
                    );
                    self.validate_const_initializer(
                        module,
                        profile,
                        tree,
                        symbols,
                        *declarator_id,
                        *mutability,
                        is_declare_context,
                        allow_ambient_const_initializers,
                    );
                    self.validate_destructuring_initializer(module, profile, tree, *declarator_id);
                }
            }
            Expression::Using {
                asynchrony,
                descriptor,
                declarators,
                ..
            } => {
                let is_declare_context = descriptor.kind == DeclarationKind::Declaration
                    || self.is_in_declare_namespace(tree, expression_id.into_any());
                for declarator_id in declarators {
                    self.validate_definite_assignment_declarator(
                        module,
                        profile,
                        tree,
                        *declarator_id,
                        options.no_definite_assignment_assertions,
                    );
                }
                if *asynchrony == Asynchrony::Async
                    && !self.can_await_in(tree, expression_id.into_any())
                {
                    let node = expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(profile));
                    self.error(AnalyzeError::InvalidAwait { node });
                }
                if is_declare_context {
                    let node = expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(profile));
                    self.error(AnalyzeError::InvalidDeclareInitializer { node });
                }
            }
            Expression::ForEach {
                asynchrony,
                kind,
                binding,
                ..
            } => {
                self.validate_for_of_binding(
                    module,
                    profile,
                    tree,
                    expression_id,
                    *asynchrony,
                    *kind,
                    binding,
                    is_strict,
                );
            }
            _ => {}
        }
    }

    /// Validate update expression targets.
    fn validate_update_target(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        operator: UnaryOperator,
        target: LocalNodeId<Expression>,
        is_strict: bool,
    ) {
        // skip non update operators
        if !matches!(
            operator,
            UnaryOperator::PostIncrement
                | UnaryOperator::PostDecrement
                | UnaryOperator::PreIncrement
                | UnaryOperator::PreDecrement
        ) {
            return;
        }

        // update operators share assignment target constraints
        self.validate_assignment_target(module, profile, tree, target, is_strict);
    }

    /// Validate for of binding constraints.
    fn validate_for_of_binding(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        asynchrony: Asynchrony,
        kind: ForEachKind,
        binding: &ForEachBinding,
        is_strict: bool,
    ) {
        // validate assignment targets for non declaration bindings
        self.validate_for_each_assignment_binding(module, profile, tree, binding, is_strict);

        // this rule applies only to sync for of loops
        if asynchrony != Asynchrony::Sync || kind != ForEachKind::Of {
            return;
        }

        // reject bindings named async
        if self.for_of_binding_is_async_identifier(tree, binding) {
            let node = expression_id
                .into_global_any(module.id)
                .into_anchored(Some(profile));
            self.error(AnalyzeError::InvalidForOfBinding { node });
        }
    }

    /// Validate assignment target rules for for each non declaration bindings.
    fn validate_for_each_assignment_binding(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        binding: &ForEachBinding,
        is_strict: bool,
    ) {
        // this rule only applies to plain pattern bindings without a declaration keyword
        let ForEachBinding::Pattern {
            pattern,
            declaration_kind: None,
        } = binding
        else {
            return;
        };

        // expression patterns in this position must be assignment targets
        let Pattern::Expression { value } = tree.get(*pattern) else {
            return;
        };

        self.validate_assignment_target(module, profile, tree, *value, is_strict);
    }

    /// Return true when a for of binding is exactly `async`.
    fn for_of_binding_is_async_identifier(
        &self,
        tree: &NodeTree,
        binding: &ForEachBinding,
    ) -> bool {
        match binding {
            // pattern and using bindings share the same pattern shape
            ForEachBinding::Pattern { pattern, .. } | ForEachBinding::Using { pattern, .. } => {
                self.pattern_is_async_identifier(tree, *pattern)
            }
        }
    }

    /// Return true when a pattern is exactly the identifier `async`.
    fn pattern_is_async_identifier(
        &self,
        tree: &NodeTree,
        pattern_id: LocalNodeId<Pattern>,
    ) -> bool {
        match tree.get(pattern_id) {
            Pattern::Binding {
                name,
                pattern: None,
                ..
            } => self.program.strings.get(*name) == "async",
            Pattern::Expression { value } => self.expression_is_async_identifier(tree, *value),
            _ => false,
        }
    }

    /// Return true when an expression is exactly the path `async`.
    fn expression_is_async_identifier(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        match tree.get(expression_id) {
            Expression::UnresolvedPath {
                path,
                static_arguments: None,
                ..
            }
            | Expression::LocalReference {
                path,
                static_arguments: None,
                ..
            }
            | Expression::ModuleReference {
                path,
                static_arguments: None,
                ..
            }
            | Expression::GlobalReference {
                path,
                static_arguments: None,
                ..
            } => path.segments.len() == 1 && self.program.strings.get(path.segments[0]) == "async",
            _ => false,
        }
    }

    /// Validate duplicate labels in nested label scopes.
    fn validate_duplicate_label(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        label: StringId,
    ) {
        // walk parent labels until a function-like boundary
        let mut current = Some(expression_id.into_any());
        while let Some(node_id) = current {
            let Some(parent) = tree.get_parent(node_id.id) else {
                break;
            };

            // duplicate labels are invalid in the same label scope chain
            if parent.ty == NodeType::Expression {
                let parent_expression = tree.get(parent.into_typed::<Expression>());
                if let Expression::Labelled {
                    label: parent_label,
                    ..
                } = parent_expression
                    && *parent_label == label
                {
                    let node = expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(profile));
                    self.error(AnalyzeError::DuplicateLabel { node });
                    return;
                }
            }

            // labels do not cross function-like boundaries
            if self.node_starts_function_scope(tree, parent) {
                break;
            }

            current = Some(parent);
        }
    }

    /// Validate catch type annotations for js and ts compatibility.
    fn validate_catch_annotation_type(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        catch_pattern_id: LocalNodeId<Pattern>,
    ) {
        // this restriction only applies to typed ts catch bindings
        if !module.language_type.is_typescript() {
            return;
        }

        // only annotated catch bindings participate in this rule
        let Pattern::Binding {
            pattern: Some(annotation_pattern_id),
            ..
        } = tree.get(catch_pattern_id)
        else {
            return;
        };

        // reject annotations that are not `any` or `unknown`
        if !self.catch_annotation_pattern_is_any_or_unknown(tree, *annotation_pattern_id) {
            let node = expression_id
                .into_global_any(module.id)
                .into_anchored(Some(profile));
            self.error(AnalyzeError::InvalidCatchAnnotationType { node });
        }
    }

    /// Return true when a catch annotation pattern is `any` or `unknown`.
    fn catch_annotation_pattern_is_any_or_unknown(
        &self,
        tree: &NodeTree,
        pattern_id: LocalNodeId<Pattern>,
    ) -> bool {
        let Pattern::Expression { value } = tree.get(pattern_id) else {
            return false;
        };

        self.catch_annotation_expression_is_any_or_unknown(tree, *value)
    }

    /// Return true when a catch annotation expression is `any` or `unknown`.
    fn catch_annotation_expression_is_any_or_unknown(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        match tree.get(expression_id) {
            Expression::TypeLiteral {
                value: TypeLiteral::Any | TypeLiteral::Unknown,
            } => true,
            Expression::Parenthesized { expression } => {
                self.catch_annotation_expression_is_any_or_unknown(tree, *expression)
            }
            _ => false,
        }
    }

    /// Return true when a node starts a fresh label scope.
    fn node_starts_function_scope(&self, tree: &NodeTree, node_id: LocalNodeIdAny) -> bool {
        match node_id.ty {
            NodeType::Declaration => {
                matches!(
                    tree.get(node_id.into_typed::<Declaration>()),
                    Declaration::Function { .. }
                )
            }
            NodeType::Member => {
                matches!(
                    tree.get(node_id.into_typed::<Member>()),
                    Member::Method { .. }
                )
            }
            NodeType::Property => {
                matches!(
                    tree.get(node_id.into_typed::<Property>()),
                    Property::Method { .. }
                )
            }
            _ => false,
        }
    }

    /// Validate left operands for exponentiation operators.
    fn validate_exponent_left_operand(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        operator: BinaryOperator,
    ) {
        // only enforce the js and ts exponentiation grammar
        if !(module.language_type.is_javascript() || module.language_type.is_typescript()) {
            return;
        }

        // skip non exponent operators
        if operator != BinaryOperator::Exponent {
            return;
        }

        // reject unparenthesized unary and delete operands
        let left_expression = tree.get(left);
        if matches!(
            left_expression,
            Expression::Unary { .. } | Expression::Delete { .. }
        ) {
            let node = expression_id
                .into_global_any(module.id)
                .into_anchored(Some(profile));
            self.error(AnalyzeError::InvalidExponentLeftUnary { node });
        }
    }

    /// Validate empty parenthesized expressions in js and ts.
    fn validate_empty_parenthesized_expression(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        elements: &[LocalNodeId<Argument>],
    ) {
        // only enforce for js and ts modules
        if !(module.language_type.is_javascript() || module.language_type.is_typescript()) {
            return;
        }

        // tuple expressions in value position represent parenthesized expressions
        if elements.is_empty() {
            let node = expression_id
                .into_global_any(module.id)
                .into_anchored(Some(profile));
            self.error(AnalyzeError::EmptyParenthesizedExpression { node });
        }
    }

    /// Validate empty sequence expressions used as parenthesized forms in js and ts.
    fn validate_empty_parenthesized_sequence(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        expressions: &[LocalNodeId<Expression>],
    ) {
        // only enforce for js and ts modules
        if !(module.language_type.is_javascript() || module.language_type.is_typescript()) {
            return;
        }

        // js and ts represent `()` as an empty sequence expression
        if expressions.is_empty() {
            let node = expression_id
                .into_global_any(module.id)
                .into_anchored(Some(profile));
            self.error(AnalyzeError::EmptyParenthesizedExpression { node });
        }
    }

    /// Validate delete expression restrictions.
    fn validate_delete_expression(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        is_strict: bool,
    ) {
        // skip non-user modules
        if !module.is_user() {
            return;
        }

        // classify the effective delete target
        let target_id = self.effective_delete_target(tree, value);

        // reject private member deletes
        if self.delete_target_contains_private_member(tree, target_id) {
            let node = expression_id
                .into_global_any(module.id)
                .into_anchored(Some(profile));
            self.error(AnalyzeError::InvalidStrictDelete { node });
            return;
        }

        // enforce strict mode delete restrictions for identifier targets
        if is_strict && self.delete_target_is_binding_reference(tree, target_id) {
            let node = expression_id
                .into_global_any(module.id)
                .into_anchored(Some(profile));
            self.error(AnalyzeError::InvalidStrictDelete { node });
        }
    }

    /// Resolve the effective target of a delete expression.
    fn effective_delete_target(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        // unwrap parenthesized wrappers
        let expression_id = self.unwrap_parenthesized_expression(expression_id, tree);

        // delete uses the last operand in sequence expressions
        if let Expression::SequenceExpression { expressions } = tree.get(expression_id)
            && let Some(last) = expressions.last()
        {
            return self.effective_delete_target(tree, *last);
        }

        expression_id
    }

    /// Return true when the delete target is a binding reference.
    fn delete_target_is_binding_reference(
        &self,
        tree: &NodeTree,
        target_id: LocalNodeId<Expression>,
    ) -> bool {
        matches!(
            tree.get(target_id),
            Expression::UnresolvedPath {
                static_arguments: None,
                ..
            } | Expression::LocalReference {
                static_arguments: None,
                ..
            } | Expression::ModuleReference {
                static_arguments: None,
                ..
            } | Expression::GlobalReference {
                static_arguments: None,
                ..
            }
        )
    }

    /// Return true when the delete target chain contains a private member access.
    fn delete_target_contains_private_member(
        &self,
        tree: &NodeTree,
        target_id: LocalNodeId<Expression>,
    ) -> bool {
        match tree.get(target_id) {
            Expression::PrivateMember { .. } => true,
            Expression::Member { left, .. } | Expression::Index { left, .. } => {
                self.delete_target_contains_private_member(tree, *left)
            }
            Expression::Maybe { left } => self.delete_target_contains_private_member(tree, *left),
            Expression::Parenthesized { expression } => {
                self.delete_target_contains_private_member(tree, *expression)
            }
            _ => false,
        }
    }

    /// Validate tagged templates after optional chains.
    fn validate_tagged_template_expression(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        tag: LocalNodeId<Expression>,
    ) {
        // optional chain tagged templates are invalid in js, ts, and destack
        if !module.language_type.is_javascript()
            && !module.language_type.is_typescript()
            && !module.language_type.is_destack()
        {
            return;
        }

        // check the tag chain for optional segments
        if self.expression_contains_optional_chain(tree, tag) {
            let node = expression_id
                .into_global_any(module.id)
                .into_anchored(Some(profile));
            self.error(AnalyzeError::InvalidOptionalChainTemplate { node });
        }
    }

    /// Validate super call expressions.
    fn validate_super_call_expression(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
    ) {
        // skip non super calls
        let left = self.unwrap_parenthesized_expression(left, tree);
        if !matches!(tree.get(left), Expression::Super) {
            return;
        }

        // allow super calls only in derived constructors
        if self.can_call_super_in_context(tree, expression_id) {
            return;
        }

        let node = expression_id
            .into_global_any(module.id)
            .into_anchored(Some(profile));
        self.error(AnalyzeError::InvalidSuperCall { node });
    }

    /// Validate optional chains rooted at super.
    fn validate_super_optional_chain(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
    ) {
        // skip non super chains
        if !self.expression_roots_in_super(tree, left) {
            return;
        }
        let node = expression_id
            .into_global_any(module.id)
            .into_anchored(Some(profile));
        self.error(AnalyzeError::InvalidSuperOptionalChain { node });
    }

    /// Validate non-call super property access.
    fn validate_super_property_expression(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) {
        // skip plain super calls, they are checked separately
        if let Expression::Call { left, .. } = tree.get(expression_id)
            && self.expression_is_super_reference(tree, *left)
        {
            return;
        }

        // skip expressions that are not rooted in super
        if !self.expression_roots_in_super(tree, expression_id) {
            return;
        }

        // allow contexts that have valid super bindings
        if self.can_access_super_in_context(tree, expression_id) {
            return;
        }

        let node = expression_id
            .into_global_any(module.id)
            .into_anchored(Some(profile));
        self.error(AnalyzeError::InvalidSuperCall { node });
    }

    /// Return true when an expression is exactly a `super` reference.
    fn expression_is_super_reference(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        let expression_id = self.unwrap_parenthesized_expression(expression_id, tree);
        matches!(tree.get(expression_id), Expression::Super)
    }

    /// Return true when an expression chain starts at a `super` reference.
    fn expression_roots_in_super(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        if self.expression_is_super_reference(tree, expression_id) {
            return true;
        }

        let expression_id = self.unwrap_parenthesized_expression(expression_id, tree);
        match tree.get(expression_id) {
            Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Call { left, .. }
            | Expression::Index { left, .. }
            | Expression::Maybe { left }
            | Expression::Must { left } => self.expression_roots_in_super(tree, *left),
            _ => false,
        }
    }

    /// Return true when the current expression can call `super(...)`.
    fn can_call_super_in_context(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        self.super_call_is_valid_context(tree, expression_id)
    }

    /// Return true when `super.x` is valid in the current lexical context.
    fn can_access_super_in_context(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        self.super_property_is_valid_context(tree, expression_id)
    }

    /// Return true when an expression chain contains optional access.
    fn expression_contains_optional_chain(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        match tree.get(expression_id) {
            Expression::Maybe { .. } => true,
            Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Call { left, .. }
            | Expression::New { left, .. }
            | Expression::TaggedTemplateExpression { tag: left, .. }
            | Expression::Instantiation { left, .. } => {
                self.expression_contains_optional_chain(tree, *left)
            }
            Expression::Parenthesized { expression } => {
                self.expression_contains_optional_chain(tree, *expression)
            }
            _ => false,
        }
    }

    /// Validate a match or switch expression.
    fn validate_match_expression(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        expression_id: LocalNodeId<Expression>,
        kind: MatchKind,
        value: LocalNodeId<Expression>,
        cases: &[LocalNodeId<MatchCase>],
    ) {
        // switch cases require expressions without guards
        if kind == MatchKind::Switch {
            for case_id in cases {
                let (selector, case_span_id) = match tree.get(*case_id) {
                    MatchCase::Expression { selector, .. } | MatchCase::Block { selector, .. } => {
                        (selector, *case_id)
                    }
                };

                if let MatchSelector::Pattern { pattern, guard } = selector {
                    // reject guarded switch cases
                    if guard.is_some() {
                        self.error(AnalyzeError::InvalidSwitchCaseGuard {
                            node: case_span_id
                                .into_global_any(module.id)
                                .into_anchored(Some(profile)),
                        });
                    }

                    // reject non expression switch cases
                    match tree.get(*pattern) {
                        Pattern::Expression { value } => {
                            if self.is_invalid_switch_case_expression(tree, *value) {
                                self.error(AnalyzeError::InvalidSwitchCasePattern {
                                    node: pattern
                                        .into_global_any(module.id)
                                        .into_anchored(Some(profile)),
                                });
                            }
                        }
                        _ => {
                            self.error(AnalyzeError::InvalidSwitchCasePattern {
                                node: pattern
                                    .into_global_any(module.id)
                                    .into_anchored(Some(profile)),
                            });
                        }
                    }
                }
            }

            return;
        }

        // match expressions require exhaustiveness checks
        self.validate_match_exhaustiveness(
            module,
            profile,
            tree,
            symbols,
            types,
            expression_id,
            value,
            cases,
        );
    }

    /// Validate must assertion usage.
    fn validate_must_assertion(
        &self,
        module: &Module,
        profile: ProfileId,
        _tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        no_must_assertions: bool,
    ) {
        if !no_must_assertions {
            return;
        }

        let node = expression_id
            .into_global_any(module.id)
            .into_anchored(Some(profile));
        self.error(AnalyzeError::MustAssertionDisabled { node });
    }

    /// Validate custom type guard usage.
    fn validate_custom_type_guard(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        no_custom_type_guards: bool,
    ) {
        if !no_custom_type_guards {
            return;
        }

        let node = expression_id
            .into_global_any(module.id)
            .into_anchored(Some(profile));
        self.error(AnalyzeError::CustomTypeGuardDisabled { node });
    }

    /// Validate narrowing guards that depend on runtime checks.
    fn validate_unsound_narrowing_operator(
        &self,
        module: &Module,
        profile: ProfileId,
        types: &TypeTable,
        expression_id: LocalNodeId<Expression>,
        operator: BinaryOperator,
        no_unsound_narrowing: bool,
    ) {
        if !no_unsound_narrowing {
            return;
        }

        if !matches!(operator, BinaryOperator::InstanceOf) {
            return;
        }

        let runtime_check = types.runtime_check_kind(expression_id.into_global_any(module.id));
        if matches!(
            runtime_check,
            Some(RuntimeCheckKind::UnionTag) | Some(RuntimeCheckKind::Constant(_))
        ) {
            return;
        }

        let node = expression_id
            .into_global_any(module.id)
            .into_anchored(Some(profile));
        self.error(AnalyzeError::UnsoundNarrowingDisabled { node });
    }

    /// Validate type guard expressions that rely on runtime narrowing.
    fn validate_unsound_narrowing_type_operator(
        &self,
        module: &Module,
        profile: ProfileId,
        types: &TypeTable,
        expression_id: LocalNodeId<Expression>,
        operator: TypeBinaryOperator,
        no_unsound_narrowing: bool,
    ) {
        if !no_unsound_narrowing {
            return;
        }
        if !matches!(
            operator,
            TypeBinaryOperator::Is | TypeBinaryOperator::InstanceOf
        ) {
            return;
        }

        let runtime_check = types.runtime_check_kind(expression_id.into_global_any(module.id));
        if matches!(
            runtime_check,
            Some(RuntimeCheckKind::UnionTag) | Some(RuntimeCheckKind::Constant(_))
        ) {
            return;
        }

        let node = expression_id
            .into_global_any(module.id)
            .into_anchored(Some(profile));
        self.error(AnalyzeError::UnsoundNarrowingDisabled { node });
    }

    /// Validate instantiation expressions followed by member or index access.
    fn validate_instantiation_access(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        types: &TypeTable,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
    ) {
        // skip non-instantiation receivers
        if !self.is_unparenthesized_instantiation_access_target(tree, left) {
            return;
        }

        // only report on value-space instantiations
        if !self.instantiation_access_target_is_value_space(module, tree, types, left) {
            return;
        }

        // report invalid instantiation access
        let node = expression_id
            .into_global_any(module.id)
            .into_anchored(Some(profile));
        self.error(AnalyzeError::InvalidInstantiationAccess { node });
    }

    /// Check whether an instantiation access target is a value-space callable.
    fn instantiation_access_target_is_value_space(
        &self,
        module: &Module,
        tree: &NodeTree,
        types: &TypeTable,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        if let Some(type_id) = types.get_inferred_type_id(expression_id.into_global_any(module.id))
        {
            let mut visited = HashSet::new();
            return self.type_is_callable_instantiation_target(type_id, types, &mut visited);
        }

        match tree.get(expression_id) {
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => {
                matches!(target_symbol.ty(), SymbolType::Function)
            }
            Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Maybe { left } => {
                self.instantiation_access_target_is_value_space(module, tree, types, *left)
            }
            _ => false,
        }
    }

    /// Check whether a receiver is an unparenthesized instantiation expression.
    fn is_unparenthesized_instantiation_access_target(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        match tree.get(expression_id) {
            Expression::Instantiation { .. } => true,
            Expression::UnresolvedPath {
                static_arguments: Some(static_arguments),
                ..
            }
            | Expression::LocalReference {
                static_arguments: Some(static_arguments),
                ..
            }
            | Expression::ModuleReference {
                static_arguments: Some(static_arguments),
                ..
            }
            | Expression::GlobalReference {
                static_arguments: Some(static_arguments),
                ..
            }
            | Expression::Member {
                static_arguments: Some(static_arguments),
                ..
            }
            | Expression::PrivateMember {
                static_arguments: Some(static_arguments),
                ..
            } => !static_arguments.is_empty(),
            Expression::Maybe { left } => {
                self.is_unparenthesized_instantiation_access_target(tree, *left)
            }
            Expression::Parenthesized { .. } => false,
            _ => false,
        }
    }

    /// Validate assignment targets for assignment expressions.
    fn validate_assignment_target(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        target: LocalNodeId<Expression>,
        is_strict: bool,
    ) {
        // reject non-assignable targets
        if !self.is_valid_assignment_target(tree, target) {
            let node = target
                .into_global_any(module.id)
                .into_anchored(Some(profile));
            self.error(AnalyzeError::InvalidAssignmentTarget { node });
            return;
        }

        // reject strict mode assignments to reserved binding names
        if module.is_user()
            && is_strict
            && let Some((reserved_target, name)) =
                self.strict_reserved_assignment_target_binding(tree, target)
        {
            let node = reserved_target
                .into_global_any(module.id)
                .into_anchored(Some(profile));
            self.error(AnalyzeError::ReservedIdentifier { node, name });
        }
    }

    /// Return reserved strict-mode assignment targets with their binding name.
    fn strict_reserved_assignment_target_binding(
        &self,
        tree: &NodeTree,
        target: LocalNodeId<Expression>,
    ) -> Option<(LocalNodeId<Expression>, StringId)> {
        let target = self.unwrap_parenthesized_expression(target, tree);
        let name = self.assignment_target_binding_name(tree, target)?;
        if !self.is_reserved_binding_name(name) {
            return None;
        }

        Some((target, name))
    }

    /// Return the binding name for assignment targets when available.
    fn assignment_target_binding_name(
        &self,
        tree: &NodeTree,
        target: LocalNodeId<Expression>,
    ) -> Option<StringId> {
        match tree.get(target) {
            Expression::UnresolvedPath {
                path,
                static_arguments: None,
                ..
            }
            | Expression::LocalReference {
                path,
                static_arguments: None,
                ..
            }
            | Expression::ModuleReference {
                path,
                static_arguments: None,
                ..
            }
            | Expression::GlobalReference {
                path,
                static_arguments: None,
                ..
            } => {
                if path.segments.len() != 1 {
                    return None;
                }

                path.last_segment()
            }
            _ => None,
        }
    }

    /// Check whether an expression is a valid assignment target.
    pub(crate) fn is_valid_assignment_target(
        &self,
        tree: &NodeTree,
        target: LocalNodeId<Expression>,
    ) -> bool {
        match tree.get(target) {
            Expression::UnresolvedPath {
                static_arguments: None,
                ..
            }
            | Expression::LocalReference {
                static_arguments: None,
                ..
            }
            | Expression::ModuleReference {
                static_arguments: None,
                ..
            }
            | Expression::GlobalReference {
                static_arguments: None,
                ..
            } => true,
            Expression::Member {
                static_arguments: None,
                ..
            } => true,
            Expression::PrivateMember {
                static_arguments: None,
                ..
            } => true,
            Expression::Index { .. } => true,
            Expression::Parenthesized { expression } => {
                self.is_valid_assignment_target(tree, *expression)
            }
            _ => false,
        }
    }

    /// Validate an import or export expression.
    fn validate_dependency_expression(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        source: DependencySource,
        kind: DependencyKind,
        items: Option<&[LocalNodeId<DependencyItem>]>,
    ) {
        // top level enforcement for static dependencies
        if self.dependency_requires_top_level(source)
            && !self.is_top_level_dependency_expression(tree, expression_id)
        {
            let node = expression_id
                .into_global_any(module.id)
                .into_anchored(Some(profile));
            if matches!(
                source,
                DependencySource::ImportStatement | DependencySource::ImportEquals
            ) {
                self.error(AnalyzeError::ImportNotTopLevel { node });
            } else {
                self.error(AnalyzeError::ExportNotTopLevel { node });
            }
        }

        // reject type-only dependencies in JavaScript modules
        if module.language_type.is_javascript() && kind == DependencyKind::Type {
            let node = expression_id
                .into_global_any(module.id)
                .into_anchored(Some(profile));
            self.error(AnalyzeError::TypeScriptSyntaxInJavaScript { node });
        }

        // validate type-only import bindings
        if let Some(items) = items
            && matches!(
                source,
                DependencySource::ImportStatement | DependencySource::ImportEquals
            )
        {
            self.validate_type_only_import_bindings(module, profile, tree, kind, items);
        }
    }

    /// Validate private identifier usage inside expressions.
    fn validate_private_identifier_expression(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) {
        // allow private identifiers only as the left operand of an `in` expression
        if self.private_identifier_is_in_expression(tree, expression_id) {
            return;
        }

        let node = expression_id
            .into_global_any(module.id)
            .into_anchored(Some(profile));
        self.error(AnalyzeError::InvalidPrivateIdentifier { node });
    }

    /// Return true when a private identifier is used in a `#name in obj` expression.
    fn private_identifier_is_in_expression(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        // walk parenthesized wrappers to locate the binary expression
        let mut current_id = expression_id;
        loop {
            let Some(parent) = tree.get_parent(current_id.id) else {
                return false;
            };
            if parent.ty != NodeType::Expression {
                return false;
            }

            let parent_id = parent.into_typed::<Expression>();
            let parent_expression = tree.get(parent_id);
            match parent_expression {
                Expression::Parenthesized { expression } => {
                    if *expression != current_id {
                        return false;
                    }
                    current_id = parent_id;
                }
                Expression::Binary {
                    operator: BinaryOperator::In,
                    left,
                    ..
                } => {
                    return *left == current_id;
                }
                _ => {
                    return false;
                }
            }
        }
    }

    /// Return true when a dependency source must be top level.
    fn dependency_requires_top_level(&self, source: DependencySource) -> bool {
        matches!(
            source,
            DependencySource::ImportStatement
                | DependencySource::ImportEquals
                | DependencySource::ExportStatement
                | DependencySource::ValueExpression
        )
    }

    /// Return true when a dependency expression is rooted at the module.
    fn is_top_level_dependency_expression(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        let mut current = expression_id.into_any();

        // unwrap statement wrappers to find the outer parent
        loop {
            let Some(parent) = tree.get_parent(current.id) else {
                return true;
            };
            if parent.ty != NodeType::Expression {
                return false;
            }

            let parent_expression = tree.get(parent.into_typed::<Expression>());
            if matches!(parent_expression, Expression::Statement { .. }) {
                current = parent;
                continue;
            }

            return false;
        }
    }

    /// Validate defaults on object literal properties.
    fn validate_object_literal_properties(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        properties: &[LocalNodeId<Property>],
    ) {
        // reserve the proto setter key once per object
        let proto_name = self.program.strings.intern("__proto__");

        // validate each property for object literal restrictions
        for property_id in properties {
            let property = tree.get(*property_id);

            // reject object literal `__proto__` setter fields
            if self.is_object_proto_setter_field(property, proto_name) {
                let node = property_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile));
                self.error(AnalyzeError::UnsupportedObjectPrototypeSetter { node });
            }

            // reject defaults on object literal properties
            if matches!(
                property,
                Property::Field {
                    default: Some(_),
                    ..
                }
            ) {
                self.error(AnalyzeError::ObjectLiteralDefault {
                    node: property_id
                        .into_global_any(module.id)
                        .into_anchored(Some(profile)),
                });
            }

            // validate accessor signatures on object literals
            if let Property::Method { signature, .. } = property {
                self.validate_accessor_signature(
                    module,
                    profile,
                    tree,
                    (*property_id).into_any(),
                    signature,
                );
            }
        }
    }

    /// Return true when a property defines an object literal `__proto__` setter.
    fn is_object_proto_setter_field(&self, property: &Property, proto_name: StringId) -> bool {
        matches!(
            property,
            Property::Field {
                key: Some(DynamicKey::Name(name)),
                value: Some(_),
                ..
            } if *name == proto_name
        )
    }

    /// Check whether await is valid in the current node context.
    fn can_await_in(&self, tree: &NodeTree, node_id: LocalNodeIdAny) -> bool {
        // walk up to find the nearest enclosing function or method
        let mut current = Some(node_id);
        while let Some(current_id) = current {
            let Some(parent) = tree.get_parent(current_id.id) else {
                break;
            };

            match parent.ty {
                NodeType::Declaration => {
                    let declaration = tree.get(parent.into_typed::<Declaration>());
                    if let Declaration::Function { signature, .. } = declaration {
                        return signature.asynchrony == Asynchrony::Async;
                    }
                }
                NodeType::Member => {
                    let member = tree.get(parent.into_typed::<Member>());
                    if let Member::Method { signature, .. } = member {
                        return signature.asynchrony == Asynchrony::Async;
                    }
                }
                NodeType::Property => {
                    let property = tree.get(parent.into_typed::<Property>());
                    if let Property::Method { signature, .. } = property {
                        return signature.asynchrony == Asynchrony::Async;
                    }
                }
                _ => {}
            }

            current = Some(parent);
        }

        // allow top level await outside declare namespaces
        !self.is_in_declare_namespace(tree, node_id)
    }

    /// Validate type-only import bindings.
    fn validate_type_only_import_bindings(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        kind: DependencyKind,
        items: &[LocalNodeId<DependencyItem>],
    ) {
        // diagnostics
        let report_error = |node: LocalNodeIdAny| {
            let node = node.into_anchored(module.id, Some(profile));
            self.error(AnalyzeError::InvalidTypeOnlyImportBindings { node });
        };

        // only enforce for type-only imports
        if kind != DependencyKind::Type {
            return;
        }

        // detect default plus named bindings
        let mut has_default = false;
        let mut named_item = None;
        for item_id in items {
            let mode = match tree.get(*item_id) {
                DependencyItem::UnresolvedRemote { mode, .. }
                | DependencyItem::UnresolvedLocal { mode, .. }
                | DependencyItem::Local { mode, .. }
                | DependencyItem::Remote { mode, .. }
                | DependencyItem::Value { mode, .. } => *mode,
            };

            if mode == DependencyMode::Default {
                has_default = true;
            } else if mode == DependencyMode::Item && named_item.is_none() {
                named_item = Some(*item_id);
            }
        }
        if has_default && let Some(named_item) = named_item {
            report_error(named_item.into_any());
        }
    }

    /// Validate readonly type operator placement.
    fn validate_readonly_type_operator(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        right: LocalNodeId<Expression>,
    ) {
        // validate readonly target shape
        if self.is_readonly_type_target(tree, right) {
            return;
        }

        // report invalid readonly usage
        let node = expression_id
            .into_global_any(module.id)
            .into_anchored(Some(profile));
        self.error(AnalyzeError::InvalidReadonlyType { node });
    }

    /// Check whether a readonly type target is valid.
    fn is_readonly_type_target(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        // unwrap parenthesized expressions
        let mut current_id = expression_id;
        loop {
            match tree.get(current_id) {
                Expression::Parenthesized { expression } => {
                    current_id = *expression;
                }
                Expression::ArrayExpression { .. } | Expression::TupleExpression { .. } => {
                    return true;
                }
                Expression::Index { right, .. } if right.is_none() => {
                    return true;
                }
                _ => {
                    return false;
                }
            }
        }
    }

    /// Validate tuple optional element ordering.
    fn validate_tuple_optional_order(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        elements: &[LocalNodeId<Argument>],
    ) {
        // skip non tuple arrays
        if !elements
            .iter()
            .any(|argument_id| self.tuple_element_is_optional(tree, *argument_id))
        {
            return;
        }

        // enforce optional element ordering
        let mut optional_seen = false;
        for argument_id in elements {
            let is_optional = self.tuple_element_is_optional(tree, *argument_id);
            let is_rest = matches!(tree.get(*argument_id), Argument::Spread { .. });

            // tuple members cannot be both optional and rest
            if is_optional && is_rest {
                let node = expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile));
                self.error(AnalyzeError::InvalidTupleElementOrder { node });
                return;
            }

            if optional_seen && !is_optional && !is_rest {
                let node = expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile));
                self.error(AnalyzeError::InvalidTupleElementOrder { node });
                return;
            }
            if is_optional {
                optional_seen = true;
            }
        }
    }

    /// Check whether a tuple element is optional.
    fn tuple_element_is_optional(
        &self,
        tree: &NodeTree,
        argument_id: LocalNodeId<Argument>,
    ) -> bool {
        match tree.get(argument_id) {
            Argument::Positional { modifiers, .. }
            | Argument::Labeled { modifiers, .. }
            | Argument::Spread { modifiers, .. } => modifiers
                .as_ref()
                .is_some_and(|modifiers| modifiers.kind == Some(BindingKind::Maybe)),
            Argument::Named { .. } => false,
        }
    }

    /// Validate intrinsic type indexing.
    fn validate_intrinsic_type_index(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
    ) {
        // reject indexing intrinsic types
        if !self.is_intrinsic_type_target(tree, left) {
            return;
        }
        let node = expression_id
            .into_global_any(module.id)
            .into_anchored(Some(profile));
        self.error(AnalyzeError::InvalidIntrinsicTypeIndex { node });
    }

    /// Check whether an expression refers to the intrinsic keyword.
    fn is_intrinsic_type_target(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        // unwrap parenthesized expressions
        let mut current_id = expression_id;
        loop {
            match tree.get(current_id) {
                Expression::Parenthesized { expression } => {
                    current_id = *expression;
                }
                Expression::TypeLiteral {
                    value: TypeLiteral::Intrinsic(_),
                } => {
                    return true;
                }
                Expression::UnresolvedPath {
                    path,
                    static_arguments,
                    ..
                }
                | Expression::LocalReference {
                    path,
                    static_arguments,
                    ..
                }
                | Expression::ModuleReference {
                    path,
                    static_arguments,
                    ..
                }
                | Expression::GlobalReference {
                    path,
                    static_arguments,
                    ..
                } => {
                    if static_arguments.is_some() || path.segments.len() != 1 {
                        return false;
                    }
                    let name = self.program.strings.get(path.segments[0]);
                    return name == "intrinsic";
                }
                _ => return false,
            }
        }
    }

    /// Validate a declare binding initializer.
    fn validate_declare_binding_initializer(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        declarator_id: LocalNodeId<Declarator>,
        mutability: Mutability,
        is_declare_context: bool,
        allow_ambient_const_initializers: bool,
    ) {
        // only enforce in declare contexts
        if !is_declare_context {
            return;
        }

        // allow ambient const initializers to handle their own rules
        if mutability == Mutability::Immutable && allow_ambient_const_initializers {
            return;
        }

        // report initializers in declare bindings
        let declarator = tree.get(declarator_id);
        if declarator.value.is_some() {
            let node = declarator_id
                .into_global_any(module.id)
                .into_anchored(Some(profile));
            self.error(AnalyzeError::InvalidDeclareInitializer { node });
        }
    }

    /// Validate a const binding initializer.
    fn validate_const_initializer(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        declarator_id: LocalNodeId<Declarator>,
        mutability: Mutability,
        is_declare_context: bool,
        allow_ambient_const_initializers: bool,
    ) {
        // only enforce for const bindings
        if mutability != Mutability::Immutable {
            return;
        }

        // report invalid ambient const initializers
        if is_declare_context {
            if !allow_ambient_const_initializers {
                return;
            }
            let declarator = tree.get(declarator_id);
            let Some(value) = declarator.value else {
                return;
            };
            if !self.is_valid_ambient_const_initializer(module, profile, tree, symbols, value) {
                let node = declarator_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile));
                self.error(AnalyzeError::InvalidAmbientConstInitializer { node });
            }
            return;
        }

        // report missing initializers in const bindings
        let declarator = tree.get(declarator_id);
        if declarator.value.is_some() {
            return;
        }

        // destructuring bindings already report a dedicated initializer error
        if self.is_destructuring_pattern(tree, declarator.pattern) {
            return;
        }

        let node = declarator_id
            .into_global_any(module.id)
            .into_anchored(Some(profile));
        self.error(AnalyzeError::MissingConstInitializer { node });
    }

    /// Validate definite assignment assertions in variable declarators.
    fn validate_definite_assignment_declarator(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        declarator_id: LocalNodeId<Declarator>,
        no_definite_assignment_assertions: bool,
    ) {
        // report definite assignment assertions in variable declarators
        let declarator = tree.get(declarator_id);
        if !self.pattern_has_definite_assignment(tree, declarator.pattern) {
            return;
        }
        let node = declarator_id
            .into_global_any(module.id)
            .into_anchored(Some(profile));

        if no_definite_assignment_assertions {
            self.error(AnalyzeError::DefiniteAssignmentAssertionDisabled { node });
            return;
        }

        if !module.language_type.is_destack() {
            self.error(AnalyzeError::InvalidDefiniteAssignmentDeclarator { node });
        }
    }

    /// Check whether an ambient const initializer is valid.
    fn is_valid_ambient_const_initializer(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        let expression = tree.get(expression_id);

        // allow scalar literals
        if let Expression::ScalarLiteral { value } = expression {
            return matches!(
                value,
                ScalarLiteral::Boolean(_)
                    | ScalarLiteral::Integer(_)
                    | ScalarLiteral::Bigint(_)
                    | ScalarLiteral::Float(_)
                    | ScalarLiteral::String(_)
                    | ScalarLiteral::Character(_)
            );
        }

        // allow template literals without interpolations
        if let Expression::TemplateExpression { value } = expression {
            return matches!(value, TemplateLiteral::String { .. });
        }

        // allow unary minus on numeric and bigint literals
        if let Expression::Unary {
            operator: UnaryOperator::Negate,
            right,
        } = expression
        {
            let right_expression = tree.get(*right);
            return matches!(
                right_expression,
                Expression::ScalarLiteral {
                    value: ScalarLiteral::Integer(_)
                        | ScalarLiteral::Bigint(_)
                        | ScalarLiteral::Float(_)
                }
            );
        }

        // allow enum member references
        if let Expression::Member {
            left,
            static_arguments: None,
            ..
        } = expression
        {
            return self.is_ambient_const_enum_reference(module, profile, tree, symbols, *left);
        }

        false
    }

    /// Validate type index access for missing members.
    fn validate_type_index_access(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        expression_id: LocalNodeId<Expression>,
    ) {
        let Expression::TypeIndex { left, index } = tree.get(expression_id) else {
            return;
        };

        // resolve operand types
        let left_ty_id = match self.try_evaluate_expression_to_type(
            module, profile, *left, tree, symbols, types, true, true,
        ) {
            Ok(type_id) => type_id,
            Err(error) => {
                self.error(error);
                return;
            }
        };
        let index_ty_id = match self.try_evaluate_expression_to_type(
            module, profile, *index, tree, symbols, types, true, true,
        ) {
            Ok(type_id) => type_id,
            Err(error) => {
                self.error(error);
                return;
            }
        };

        // skip missing checks for unresolved type parameters
        if let Some(symbol) = types.get_type(left_ty_id).symbol()
            && self.symbol_is_static_parameter(module, profile, symbol, symbols, types)
        {
            return;
        }

        // skip index validation when the type index is an array size
        let supports_index_access = match self
            .type_supports_index_access(module, profile, left_ty_id, tree, symbols, types)
        {
            Ok(value) => value,
            Err(error) => {
                self.error(error);
                return;
            }
        };
        let is_primitive_literal = self.type_is_primitive_literal(left_ty_id, types);
        let is_index_access = if module.language_type.is_declaration() {
            true
        } else {
            supports_index_access && !is_primitive_literal
        };
        if !is_index_access {
            return;
        }

        // skip missing checks for any or unknown receivers
        if matches!(
            types.get_type(left_ty_id),
            Type::TypeLiteral {
                value: TypeLiteral::Any | TypeLiteral::Unknown,
            }
        ) {
            return;
        }

        // resolve index access types to detect missing keys
        let mut visited = Vec::new();
        let resolution = self.resolve_index_access_types(
            module,
            profile,
            expression_id.into_any(),
            left_ty_id,
            index_ty_id,
            symbols,
            types,
            NormalizationMode::Flow,
            RelationMode::TYPE_OPS,
            &mut visited,
        );
        let Some(missing_key) = resolution.missing_keys.first() else {
            return;
        };

        // report missing key access
        self.error(AnalyzeError::MissingMember {
            node: expression_id
                .into_global_any(module.id)
                .into_anchored(Some(profile)),
            receiver_ty: left_ty_id.into_global(module.id),
            member_key: *missing_key,
        });
    }

    /// Validate import type accesses for missing exports.
    fn validate_type_import_expression(
        &self,
        module: &Module,
        profile: ProfileId,
        types: &mut TypeTable,
        expression_id: LocalNodeId<Expression>,
        target: StringId,
        qualifier: Option<&Path>,
    ) {
        let Some(qualifier) = qualifier else {
            return;
        };

        // only validate single-segment qualifiers
        let member_key = if qualifier.segments.len() == 1 {
            qualifier.last_segment().map(StaticKey::Name)
        } else {
            None
        };
        let Some(member_key) = member_key else {
            return;
        };

        // resolve the import type symbol
        let resolved = self.resolve_import_type_symbol(
            module,
            profile,
            expression_id.into_any(),
            target,
            Some(qualifier),
        );
        if resolved.is_some() {
            return;
        }

        // emit missing member when the export is absent
        let receiver_ty_id = types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            },
            expression_id.into_any(),
        );
        self.error(AnalyzeError::MissingMember {
            node: expression_id
                .into_global_any(module.id)
                .into_anchored(Some(profile)),
            receiver_ty: receiver_ty_id.into_global(module.id),
            member_key,
        });
    }

    /// Validate that a type import target is a string literal and return the string id.
    fn validate_type_import_target_expression(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        target: LocalNodeId<Expression>,
    ) -> Option<StringId> {
        if let Expression::ScalarLiteral {
            value: ScalarLiteral::String(target_string),
        } = tree.get(target)
        {
            return Some(*target_string);
        }

        let node = expression_id
            .into_global_any(module.id)
            .into_anchored(Some(profile));
        self.error(AnalyzeError::InvalidTypeImportTarget { node });
        None
    }

    /// Check whether an expression is an enum reference for ambient const initializers.
    fn is_ambient_const_enum_reference(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        let target_symbol = match tree.get(expression_id) {
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => *target_symbol,
            _ => return false,
        };

        self.symbol_is_enum(module, profile, symbols, target_symbol)
    }

    /// Check whether a symbol resolves to an enum declaration.
    fn symbol_is_enum(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        symbol_id: GlobalSymbolId,
    ) -> bool {
        // check symbols from the current module
        if symbol_id.module_id == module.id {
            let symbol = symbols.get_symbol(symbol_id.local_id);
            return symbol.ty == SymbolType::Enum;
        }

        // check symbols from dependent modules
        let target_module = self.program.modules.get(symbol_id.module_id);
        let target_module = target_module.read();
        let Some(target_dir) = target_module.dir_maybe(profile) else {
            return false;
        };
        let target_symbols = target_dir.symbols.read();
        let symbol = target_symbols.get_symbol(symbol_id.local_id);
        symbol.ty == SymbolType::Enum
    }

    /// Validate a destructuring declaration without an initializer.
    fn validate_destructuring_initializer(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        declarator_id: LocalNodeId<Declarator>,
    ) {
        let declarator = tree.get(declarator_id);
        if declarator.value.is_some() {
            return;
        }

        // destructuring bindings require initializers
        if self.is_destructuring_pattern(tree, declarator.pattern) {
            let node = declarator_id
                .into_global_any(module.id)
                .into_anchored(Some(profile));
            self.error(AnalyzeError::MissingDestructuringInitializer { node });
        }
    }

    // reject structural or wildcard patterns in switch cases
    pub(crate) fn is_invalid_switch_case_expression(
        &self,
        tree: &NodeTree,
        value: LocalNodeId<Expression>,
    ) -> bool {
        match tree.get(value) {
            Expression::TupleExpression { .. } => true,
            Expression::TaggedTupleExpression { .. } => true,
            Expression::UnresolvedPath { path, .. }
            | Expression::LocalReference { path, .. }
            | Expression::ModuleReference { path, .. }
            | Expression::GlobalReference { path, .. } => {
                if path.segments.len() != 1 {
                    return false;
                }
                let name = self.program.strings.get(path.segments[0]);
                name == "_"
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::TestProgram;
    use destack_dir::Expression;

    /// Reject assignments to instantiation expressions.
    #[test]
    fn test_reject_instantiation_assignment_target() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ts",
            r#"
class ConcreteClass {
    static myFunc?: <M>(instance: M) => void;
}

const cls = ConcreteClass;
cls.myFunc<ConcreteClass> = (instance) => {
    instance;
};
"#,
        );
        test.apply_dsconfig(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        // confirm assignment target shape
        test.with_dir_read(module_id, |_, _, _, tree, symbols, _| {
            for (id, expression) in tree.iter_nodes_of_type::<Expression>() {
                let Expression::Assign { left, .. } = expression else {
                    continue;
                };

                let is_active = test.compiler.is_node_active(tree, symbols, id.into_any());
                assert!(is_active, "assignment expression unexpectedly inactive");

                let is_valid = test.compiler.is_valid_assignment_target(tree, *left);
                assert!(!is_valid, "assignment target was unexpectedly valid");
                return;
            }

            panic!("expected assignment expression");
        });
        test.check_has_diagnostic("EA226");
    }

    /// Reject super member access in plain functions.
    #[test]
    fn test_reject_super_member_access_in_plain_function() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.js",
            r#"
function a() {
    super.b;
}
"#,
        );
        test.apply_dsconfig(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA244");
    }

    /// Reject super member access in nested non-lambda functions.
    #[test]
    fn test_reject_super_member_access_in_nested_function() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.js",
            r#"
class A extends B {
    m() {
        function n() {
            super.x;
        }
    }
}
"#,
        );
        test.apply_dsconfig(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA244");
    }

    /// Allow super member access in class methods.
    #[test]
    fn test_allow_super_member_access_in_method() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.js",
            r#"
class A extends B {
    m() {
        return super.x;
    }
}
"#,
        );
        test.apply_dsconfig(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EA244");
    }

    /// Allow super member access in lambdas nested inside methods.
    #[test]
    fn test_allow_super_member_access_in_lambda_inside_method() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.js",
            r#"
class A extends B {
    m() {
        const get = () => super.x;
        return get();
    }
}
"#,
        );
        test.apply_dsconfig(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EA244");
    }

    /// Reject strict mode updates of arguments.
    #[test]
    fn test_reject_strict_mode_update_arguments() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.js",
            r#"
function a() {
    "use strict";
    ++arguments;
}
"#,
        );
        test.apply_dsconfig(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA214");
    }

    /// Reject strict mode assignments to reserved identifier names.
    #[test]
    fn test_reject_strict_mode_assignment_to_reserved_identifier() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.js",
            r#"
function a() {
    "use strict";
    interface = 1;
}
"#,
        );
        test.apply_dsconfig(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA214");
    }

    /// Reject update expressions on non-assignable literals.
    #[test]
    fn test_reject_update_on_literal_target() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.js", "0++;");
        test.apply_dsconfig(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA226");
    }

    /// Reject object literal `__proto__` setter fields.
    #[test]
    fn test_reject_object_proto_setter() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.js", r#"({ "__proto__": null });"#);
        test.apply_dsconfig(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA249");
    }

    /// Reject non-assignable for of binding targets.
    #[test]
    fn test_reject_for_of_literal_binding_target() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.js", "for(0 of 0);");
        test.apply_dsconfig(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA226");
    }

    /// Reject object literals that mix shorthand and proto setter fields.
    #[test]
    fn test_reject_shorthand_proto_with_setter() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.js",
            r#"
const __proto__ = 1;
({ __proto__, "__proto__": null });
"#,
        );
        test.apply_dsconfig(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA249");
    }

    /// Allow plain shorthand `__proto__` bindings in object literals.
    #[test]
    fn test_allow_shorthand_proto_without_setter() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.js",
            r#"
const __proto__ = 1;
({ __proto__ });
"#,
        );
        test.apply_dsconfig(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EA249");
    }
}
