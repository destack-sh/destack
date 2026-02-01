use crate::{AnalyzeError, Compiler};
use destack_dir::{
    Argument, BindingKind, DeclarationKind, Declarator, DependencyItem, DependencyKind,
    DependencyMode, Expression, LocalNodeId, LocalNodeIdAny, MatchCase, MatchKind, MatchSelector,
    NodeTree, Pattern, PatternField, Property, TypeLiteral, TypeUnaryOperator,
};
use destack_workspace::{Module, ProfileId};

impl Compiler {
    /// Validate a single expression node.
    pub(super) fn validate_expression(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        match expression {
            Expression::Match {
                kind: MatchKind::Switch,
                cases,
                ..
            } => {
                for case_id in cases {
                    let (selector, case_span_id) = match tree.get(*case_id) {
                        MatchCase::Expression { selector, .. }
                        | MatchCase::Block { selector, .. } => (selector, *case_id),
                    };
                    if let MatchSelector::Pattern { pattern, guard } = selector {
                        if guard.is_some() {
                            self.error(AnalyzeError::InvalidSwitchCaseGuard {
                                node: case_span_id
                                    .into_global_any(module.id)
                                    .into_anchored(Some(profile)),
                            });
                        }
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
                self.validate_type_only_import_bindings(module, profile, tree, *kind, items);
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
            }
            Expression::ArrayExpression { elements } | Expression::TupleExpression { elements } => {
                self.validate_tuple_optional_order(module, profile, tree, expression_id, elements);
            }
            Expression::Let {
                descriptor,
                declarators,
                ..
            } => {
                let is_declare_context = descriptor.kind == DeclarationKind::Declaration
                    || self.is_in_declare_namespace(tree, expression_id.into_any());
                self.validate_declare_binding_initializers(
                    module,
                    profile,
                    tree,
                    declarators,
                    is_declare_context,
                );
                self.validate_destructuring_initializers(module, profile, tree, declarators);
            }
            Expression::Using { descriptor, .. } => {
                let is_declare_context = descriptor.kind == DeclarationKind::Declaration
                    || self.is_in_declare_namespace(tree, expression_id.into_any());
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

    /// Validate a single pattern node.
    pub(super) fn validate_pattern(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        pattern: &Pattern,
    ) {
        match pattern {
            Pattern::Object { fields } | Pattern::TaggedObject { fields, .. } => {
                self.validate_object_pattern_spreads(module, profile, tree, fields);
            }
            Pattern::Array { fields }
            | Pattern::Tuple { fields }
            | Pattern::TaggedTuple { fields, .. } => {
                self.validate_sequence_pattern_fields(module, profile, tree, fields);
            }
            _ => {}
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

    /// Validate object pattern spread placement.
    fn validate_object_pattern_spreads(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        fields: &[LocalNodeId<PatternField>],
    ) {
        // locate the first spread field and report duplicates
        let mut spread_index = None;
        for (index, field_id) in fields.iter().enumerate() {
            if matches!(tree.get(*field_id), PatternField::Spread { .. }) {
                let error_node = field_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile));
                if spread_index.is_some() {
                    self.error(AnalyzeError::ObjectPatternMultipleSpreads { node: error_node });
                } else {
                    spread_index = Some(index);
                }
            }
        }

        // spread must be the last field
        if let Some(index) = spread_index
            && index + 1 < fields.len()
        {
            let error_node = fields[index + 1]
                .into_global_any(module.id)
                .into_anchored(Some(profile));
            self.error(AnalyzeError::ObjectPatternSpreadNotLast { node: error_node });
        }
    }

    /// Validate named fields in array and tuple patterns.
    fn validate_sequence_pattern_fields(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        fields: &[LocalNodeId<PatternField>],
    ) {
        // Destack tuple and array patterns may use named fields
        if module.language_type.is_destack() {
            return;
        }

        // reject named or aliased fields in array and tuple patterns
        for field_id in fields {
            if matches!(
                tree.get(*field_id),
                PatternField::Named { .. } | PatternField::Alias { .. }
            ) {
                let node = field_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile));
                self.error(AnalyzeError::InvalidPatternNamedField { node });
                return;
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

    /// Validate declare binding initializers.
    fn validate_declare_binding_initializers(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        declarators: &[LocalNodeId<Declarator>],
        is_declare_context: bool,
    ) {
        // only enforce in declare contexts
        if !is_declare_context {
            return;
        }

        // report initializers in declare bindings
        for declarator_id in declarators {
            let declarator = tree.get(*declarator_id);
            if declarator.value.is_some() {
                let node = declarator_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile));
                self.error(AnalyzeError::InvalidDeclareInitializer { node });
            }
        }
    }

    /// Validate destructuring declarations without initializers.
    fn validate_destructuring_initializers(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        declarators: &[LocalNodeId<Declarator>],
    ) {
        // scan declarators without values
        for declarator_id in declarators {
            let declarator = tree.get(*declarator_id);
            if declarator.value.is_some() {
                continue;
            }

            // destructuring bindings require initializers
            if self.is_destructuring_pattern(tree, declarator.pattern) {
                let node = declarator_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile));
                self.error(AnalyzeError::MissingDestructuringInitializer { node });
            }
        }
    }

    /// Check whether a pattern is a destructuring pattern.
    fn is_destructuring_pattern(&self, tree: &NodeTree, pattern_id: LocalNodeId<Pattern>) -> bool {
        match tree.get(pattern_id) {
            Pattern::Array { .. }
            | Pattern::Object { .. }
            | Pattern::Tuple { .. }
            | Pattern::TaggedTuple { .. }
            | Pattern::TaggedObject { .. } => true,
            Pattern::Binding {
                pattern: Some(inner),
                ..
            } => self.is_destructuring_pattern(tree, *inner),
            Pattern::Must(inner)
            | Pattern::ReferenceOf { right: inner, .. }
            | Pattern::ValueOf { right: inner, .. } => self.is_destructuring_pattern(tree, *inner),
            _ => false,
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
