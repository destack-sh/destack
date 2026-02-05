use crate::analyze::common::{NormalizationMode, RelationMode};
use crate::{AnalyzeError, Compiler};
use destack_dir::{
    Argument, BinaryOperator, BindingKind, DeclarationKind, Declarator, DependencyItem,
    DependencyKind, DependencyMode, Expression, GlobalSymbolId, LocalNodeId, LocalNodeIdAny,
    MatchCase, MatchKind, MatchSelector, Mutability, NodeTree, Path, Pattern, Property,
    RuntimeCheckKind, ScalarLiteral, StaticKey, StringId, SymbolTable, SymbolType, TemplateLiteral,
    Type, TypeBinaryOperator, TypeLiteral, TypeTable, TypeUnaryOperator, UnaryOperator,
};
use destack_workspace::{Module, ProfileId};

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
        expression_id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        match expression {
            Expression::Assign { left, .. } => {
                self.validate_assignment_target(module, profile, tree, *left);
            }
            Expression::AssignBinary { left, .. } => {
                self.validate_assignment_target(module, profile, tree, *left);
            }
            Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Index { left, .. } => {
                self.validate_instantiation_access(module, profile, tree, expression_id, *left);
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
                self.validate_must_assertion(module, profile, tree, expression_id);
            }
            Expression::TypePredicate { .. } => {
                self.validate_custom_type_guard(module, profile, expression_id);
            }
            Expression::Binary { operator, .. } => {
                self.validate_unsound_narrowing_operator(
                    module,
                    profile,
                    types,
                    expression_id,
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
                );
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
            Expression::Import { kind, items, .. }
            | Expression::UnresolvedImport { kind, items, .. } => {
                self.validate_dependency_expression(
                    module,
                    profile,
                    tree,
                    expression_id,
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
                self.validate_type_import_expression(
                    module,
                    profile,
                    types,
                    expression_id,
                    *target,
                    qualifier.as_ref(),
                );
            }
            Expression::ArrayExpression { elements } | Expression::TupleExpression { elements } => {
                self.validate_tuple_optional_order(module, profile, tree, expression_id, elements);
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
                    );
                }
                if is_declare_context {
                    let node = expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(profile));
                    self.error(AnalyzeError::InvalidDeclareInitializer { node });
                }
            }
            _ => {}
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
    ) {
        let options = self.analyze_context_options_for_module(module.id);

        if !options.no_must_assertions {
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
    ) {
        let options = self.analyze_context_options_for_module(module.id);

        if !options.no_custom_type_guards {
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
    ) {
        let options = self.analyze_context_options_for_module(module.id);

        if !options.no_unsound_narrowing {
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
    ) {
        let options = self.analyze_context_options_for_module(module.id);
        if !options.no_unsound_narrowing {
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
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
    ) {
        // skip non-instantiation receivers
        if !self.is_unparenthesized_instantiation_access_target(tree, left) {
            return;
        }

        // report invalid instantiation access
        let node = expression_id
            .into_global_any(module.id)
            .into_anchored(Some(profile));
        self.error(AnalyzeError::InvalidInstantiationAccess { node });
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
                static_arguments: Some(_),
                ..
            }
            | Expression::LocalReference {
                static_arguments: Some(_),
                ..
            }
            | Expression::ModuleReference {
                static_arguments: Some(_),
                ..
            }
            | Expression::GlobalReference {
                static_arguments: Some(_),
                ..
            }
            | Expression::Member {
                static_arguments: Some(_),
                ..
            }
            | Expression::PrivateMember {
                static_arguments: Some(_),
                ..
            } => true,
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
    ) {
        // reject non-assignable targets
        if !self.is_valid_assignment_target(tree, target) {
            let node = target
                .into_global_any(module.id)
                .into_anchored(Some(profile));
            self.error(AnalyzeError::InvalidAssignmentTarget { node });
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
        kind: DependencyKind,
        items: Option<&[LocalNodeId<DependencyItem>]>,
    ) {
        // reject type-only dependencies in JavaScript modules
        if module.language_type.is_javascript() && kind == DependencyKind::Type {
            let node = expression_id
                .into_global_any(module.id)
                .into_anchored(Some(profile));
            self.error(AnalyzeError::TypeScriptSyntaxInJavaScript { node });
        }

        // validate type-only import bindings
        if let Some(items) = items {
            self.validate_type_only_import_bindings(module, profile, tree, kind, items);
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
        // reject defaults on object literal properties
        for property_id in properties {
            let property = tree.get(*property_id);
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
        }
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

        // optional conformance debug hook
        if std::env::var("DESTACK_CONFORMANCE_DEBUG").is_ok() {
            let mut modes = Vec::new();
            for item_id in items {
                let mode = match tree.get(*item_id) {
                    DependencyItem::UnresolvedRemote { mode, .. }
                    | DependencyItem::UnresolvedLocal { mode, .. }
                    | DependencyItem::Local { mode, .. }
                    | DependencyItem::Remote { mode, .. }
                    | DependencyItem::Value { mode, .. } => *mode,
                };
                modes.push(mode);
            }
            eprintln!("conformance import bindings: kind={kind:?} modes={modes:?}");
        }

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
    ) {
        // report definite assignment assertions in variable declarators
        let declarator = tree.get(declarator_id);
        if !self.pattern_has_definite_assignment(tree, declarator.pattern) {
            return;
        }
        let node = declarator_id
            .into_global_any(module.id)
            .into_anchored(Some(profile));

        let options = self.analyze_context_options_for_module(module.id);
        if options.no_definite_assignment_assertions {
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
}
