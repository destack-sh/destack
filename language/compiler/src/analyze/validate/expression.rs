use crate::analyze::common::{NormalizationMode, RelationMode, TypeContext};
use crate::{AnalyzeError, AnalyzeOptions, Compiler};
use destack_ast::Keyword;
use destack_dir::{
    Ambientness, Argument, Asynchrony, BinaryOperator, Declaration, Declarator, DependencyItem,
    DependencyKind, DependencyMode, Expression, ForEachBinding, ForEachKind, GlobalSymbolId,
    ImportSource, Key, LocalNodeId, LocalNodeIdAny, MatchCase, MatchKind, MatchSelector, Member,
    Mutability, NodeTree, NodeType, Parameter, Path, Pattern, PatternField, Property,
    RuntimeCheckKind, ScalarLiteral, StringId, SymbolType, TemplateLiteral, Type, TypeExpression,
    TypeLiteral, UnaryOperator, WhereClause,
};
use std::str::FromStr;

/// Step result when climbing expression parents for type-position classification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TypePositionStep {
    /// The child expression inherits type context from the parent.
    Ascend,
    /// The child expression is not in a type slot.
    NotType,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Validate a single expression node.
    pub(super) fn validate_expression(
        &self,
        ctx: &mut TypeContext<'_>,
        options: AnalyzeOptions,
        expression_id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        // cache strict mode once per expression validation
        let is_strict = ctx.module.source_type.is_module() || options.always_strict;

        match expression {
            Expression::Labelled { label, .. } => {
                self.validate_duplicate_label(&mut ctx.reborrow(), expression_id, *label);
                self.validate_strict_reserved_label_identifier(
                    &ctx.reborrow(),
                    expression_id,
                    *label,
                    is_strict,
                );
            }
            Expression::Break {
                target,
                target_symbol,
                ..
            } => {
                self.validate_label_target_function_boundary(
                    &mut ctx.reborrow(),
                    expression_id,
                    *target,
                    *target_symbol,
                    true,
                );
            }
            Expression::Continue {
                target,
                target_symbol,
                ..
            } => {
                self.validate_label_target_function_boundary(
                    &mut ctx.reborrow(),
                    expression_id,
                    *target,
                    *target_symbol,
                    false,
                );
            }
            Expression::Try {
                catch_pattern,
                catch_ty,
                catch_expression,
                finally_expression,
                ..
            } => {
                self.validate_try_requires_catch_or_finally(
                    &mut ctx.reborrow(),
                    expression_id,
                    *catch_expression,
                    *finally_expression,
                );

                if let Some(catch_pattern_id) = catch_pattern {
                    self.validate_catch_binding_pattern(
                        &mut ctx.reborrow(),
                        expression_id,
                        *catch_pattern_id,
                    );
                    self.validate_catch_annotation_type(
                        &mut ctx.reborrow(),
                        expression_id,
                        *catch_ty,
                    );
                }
            }
            Expression::Assign { .. } | Expression::AssignBinary { .. } => {}
            Expression::Super => {}
            Expression::NewTarget => {
                self.validate_new_target_expression(&ctx.reborrow(), expression_id);
            }
            Expression::Call { .. } => {}
            Expression::New { .. } => {}
            Expression::Member { .. }
            | Expression::PrivateMember { .. }
            | Expression::Index { .. } => {}
            Expression::Maybe { .. } => {}
            Expression::PrivateIdentifier { .. } => {
                self.validate_private_identifier_expression(&mut ctx.reborrow(), expression_id);
            }
            Expression::UnresolvedPath { .. }
            | Expression::LocalReference { .. }
            | Expression::ModuleReference { .. }
            | Expression::GlobalReference { .. } => {}
            Expression::Match {
                kind, value, cases, ..
            } => {
                self.validate_match_expression(
                    &mut ctx.reborrow(),
                    expression_id,
                    *kind,
                    *value,
                    cases,
                );
            }
            Expression::Must { .. } => {
                self.validate_must_assertion(
                    &mut ctx.reborrow(),
                    expression_id,
                    options.no_must_assertions,
                );
            }
            Expression::Binary { left, operator, .. } => {
                self.validate_exponent_left_operand(
                    &mut ctx.reborrow(),
                    expression_id,
                    *left,
                    *operator,
                );
            }
            Expression::Is { .. } | Expression::InstanceOf { .. } => {
                self.validate_unsound_narrowing_guard(
                    &mut ctx.reborrow(),
                    expression_id,
                    options.no_unsound_narrowing,
                );
            }
            Expression::As { .. } | Expression::Satisfies { .. } => {}
            Expression::ExportNamespace { .. } => {
                if !ctx.module.language_type.is_declaration() {
                    self.error(AnalyzeError::ExportNamespaceOutsideDeclaration {
                        node: expression_id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile)),
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
                    &mut ctx.reborrow(),
                    expression_id,
                    *source,
                    *kind,
                    items.as_deref(),
                );
            }
            Expression::Export { kind, .. }
            | Expression::ReExport { kind, .. }
            | Expression::UnresolvedReExport { kind, .. } => {
                self.validate_dependency_expression(
                    &mut ctx.reborrow(),
                    expression_id,
                    ImportSource::ExportStatement,
                    *kind,
                    None,
                );
            }
            Expression::ObjectExpression { ty: _, properties }
            | Expression::TaggedObjectExpression { properties, .. } => {
                self.validate_object_literal_properties(&mut ctx.reborrow(), properties);
            }
            Expression::Unary { operator, right } => {
                self.validate_update_target(&mut ctx.reborrow(), *operator, *right, is_strict);
            }
            Expression::ArrayExpression { elements } | Expression::TupleExpression { elements } => {
                self.validate_tuple_optional_order(&mut ctx.reborrow(), expression_id, elements);
                if matches!(expression, Expression::TupleExpression { .. }) {
                    self.validate_empty_parenthesized_expression(
                        &mut ctx.reborrow(),
                        expression_id,
                        elements,
                    );
                }
            }
            Expression::SequenceExpression { expressions } => {
                self.validate_empty_parenthesized_sequence(
                    &mut ctx.reborrow(),
                    expression_id,
                    expressions,
                );
            }
            Expression::Delete { value } => {
                self.validate_delete_expression(
                    &mut ctx.reborrow(),
                    expression_id,
                    *value,
                    is_strict,
                );
            }
            Expression::TaggedTemplateExpression { tag, .. } => {
                self.validate_tagged_template_expression(&mut ctx.reborrow(), expression_id, *tag);
            }
            Expression::Let {
                ambient,
                mutability,
                declarators,
                ..
            } => {
                let is_in_declare_namespace =
                    self.is_in_declare_namespace(ctx.tree, expression_id.into_any());
                let is_in_declare_module =
                    self.is_in_declare_module(ctx.tree, expression_id.into_any());
                let is_declare_context =
                    *ambient == Ambientness::Ambient || is_in_declare_namespace;
                let allow_ambient_const_initializers = is_in_declare_module;
                for declarator_id in declarators {
                    self.validate_js_ts_compat_declarator_pattern(
                        &mut ctx.reborrow(),
                        *declarator_id,
                    );
                    self.validate_definite_assignment_declarator(
                        &mut ctx.reborrow(),
                        *declarator_id,
                        options.no_definite_assignment_assertions,
                    );
                    self.validate_declare_binding_initializer(
                        &mut ctx.reborrow(),
                        *declarator_id,
                        *mutability,
                        is_declare_context,
                        allow_ambient_const_initializers,
                    );
                    self.validate_const_initializer(
                        &mut ctx.reborrow(),
                        *declarator_id,
                        *mutability,
                        is_declare_context,
                        allow_ambient_const_initializers,
                    );
                    self.validate_destructuring_initializer(&mut ctx.reborrow(), *declarator_id);
                }
            }
            Expression::Using {
                asynchrony,
                ambient,
                declarators,
                ..
            } => {
                let is_declare_context = *ambient == Ambientness::Ambient
                    || self.is_in_declare_namespace(ctx.tree, expression_id.into_any());
                for declarator_id in declarators {
                    self.validate_js_ts_compat_declarator_pattern(
                        &mut ctx.reborrow(),
                        *declarator_id,
                    );
                    self.validate_definite_assignment_declarator(
                        &mut ctx.reborrow(),
                        *declarator_id,
                        options.no_definite_assignment_assertions,
                    );
                }
                if *asynchrony == Asynchrony::Async
                    && !self.can_await_in(ctx.tree, expression_id.into_any())
                {
                    let node = expression_id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile));
                    self.error(AnalyzeError::InvalidAwait { node });
                }
                if is_declare_context {
                    let node = expression_id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile));
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
                    &mut ctx.reborrow(),
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

    // control flow labels and strict identifier checks
    /// Validate that labelled jump targets stay in the same function lexical owner.
    fn validate_label_target_function_boundary(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
        target: Option<StringId>,
        target_symbol: Option<GlobalSymbolId>,
        is_break: bool,
    ) {
        // unlabeled jumps do not cross label scopes
        let Some(target) = target else {
            return;
        };

        // unresolved targets are reported by resolve
        let Some(target_symbol) = target_symbol else {
            return;
        };

        // label targets are always local to the current module
        if target_symbol.module_id != ctx.module.id {
            return;
        }

        // skip malformed symbols without declaration anchors
        let symbol = ctx.symbols.get_symbol(target_symbol.local_id);
        let Some(label_declaration) = symbol.primary_declaration else {
            return;
        };
        if label_declaration.local_id.ty != NodeType::Expression {
            return;
        }

        // compare lexical function owners for jump and target label
        let jump_owner = self.nearest_function_like_owner(ctx.tree, expression_id.into_any());
        let label_expression_id = LocalNodeId::<Expression>::new(label_declaration.local_id.id);
        let label_owner =
            self.nearest_function_like_owner(ctx.tree, label_expression_id.into_any());
        if jump_owner == label_owner {
            return;
        }

        let node = expression_id
            .into_global_any(ctx.module.id)
            .into_anchored(Some(ctx.profile));

        // report cross-function jumps with the appropriate control-flow diagnostic
        if is_break {
            self.error(AnalyzeError::InvalidBreak {
                node,
                label: Some(target),
            });
        } else {
            self.error(AnalyzeError::InvalidContinue {
                node,
                label: Some(target),
            });
        }
    }

    /// Return the nearest function-like owner for a node.
    fn nearest_function_like_owner(
        &self,
        tree: &NodeTree,
        node_id: LocalNodeIdAny,
    ) -> Option<LocalNodeIdAny> {
        // walk up parent nodes until a function-like owner is found
        let mut current = tree.get_parent(node_id.id);
        while let Some(parent) = current {
            if self.node_starts_function_scope(tree, parent) {
                return Some(parent);
            }
            current = tree.get_parent(parent.id);
        }

        None
    }

    /// Validate strict-mode identifier references for reserved names.
    pub(crate) fn validate_strict_reserved_identifier_reference(
        &self,
        ctx: &TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
        path: &Path,
        is_strict: bool,
    ) {
        if !ctx.module.is_user() || !is_strict {
            return;
        }

        // reserved identifier references are always single segment names
        let Some(name) = self.path_is_reserved_identifier_reference(path) else {
            return;
        };

        let node = expression_id
            .into_global_any(ctx.module.id)
            .into_anchored(Some(ctx.profile));
        self.error(AnalyzeError::ReservedIdentifier { node, name });
    }

    /// Validate strict-mode labels for reserved names.
    fn validate_strict_reserved_label_identifier(
        &self,
        ctx: &TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
        label: StringId,
        is_strict: bool,
    ) {
        if !ctx.module.is_user() || !is_strict {
            return;
        }

        // labels use identifier rules in strict mode
        if !self.strict_reserved_reference_name(label) {
            return;
        }

        let node = expression_id
            .into_global_any(ctx.module.id)
            .into_anchored(Some(ctx.profile));
        self.error(AnalyzeError::ReservedIdentifier { node, name: label });
    }

    /// Return the reserved identifier for a strict reference path when present.
    fn path_is_reserved_identifier_reference(&self, path: &Path) -> Option<StringId> {
        // only single segment references participate in strict reserved checks
        if path.segments.len() != 1 {
            return None;
        }

        let name = path.segments[0];
        if !self.strict_reserved_reference_name(name) {
            return None;
        }

        Some(name)
    }

    /// Return true when a name is reserved in strict identifier reference positions.
    fn strict_reserved_reference_name(&self, name: StringId) -> bool {
        let name_str = self.repository.strings.get(name);
        let Ok(keyword) = Keyword::from_str(name_str.as_ref()) else {
            return false;
        };

        Self::is_reserved_binding_keyword(keyword)
    }

    /// Validate `new.target` usage context.
    pub(crate) fn validate_new_target_expression(
        &self,
        ctx: &TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
    ) {
        // skip expressions that are not exactly `new.target`
        if !self.expression_is_new_target(ctx.tree, expression_id) {
            return;
        }

        // allow valid lexical owners for `new.target`
        if self.can_access_new_target_in_context(ctx.tree, expression_id) {
            return;
        }

        let node = expression_id
            .into_global_any(ctx.module.id)
            .into_anchored(Some(ctx.profile));
        self.error(AnalyzeError::InvalidNewTarget { node });
    }

    /// Validate `new` constructor expressions that use optional chaining.
    pub(crate) fn validate_new_optional_chain_expression(
        &self,
        ctx: &TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
    ) {
        if !self.expression_contains_optional_chain(ctx.tree, left) {
            return;
        }

        let node = expression_id
            .into_global_any(ctx.module.id)
            .into_anchored(Some(ctx.profile));
        self.error(AnalyzeError::InvalidNewOptionalChain { node });
    }

    // instantiation access and type position classification
    /// Validate member and index access after instantiation expressions.
    pub(crate) fn validate_instantiation_access(
        &self,
        ctx: &TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
    ) {
        if !self.has_invalid_instantiation_access_receiver(ctx.tree, expression_id) {
            return;
        }

        // type positions reuse member and index syntax for projections
        if self.is_type_position_for_instantiation_access(ctx, expression_id) {
            return;
        }

        // allow projection-style access when the instantiation receiver is type-like
        if self.expression_has_type_like_instantiation_receiver(ctx, expression_id) {
            return;
        }

        let node = expression_id
            .into_global_any(ctx.module.id)
            .into_anchored(Some(ctx.profile));
        self.error(AnalyzeError::InvalidInstantiationAccess { node });
    }

    /// Return true when a member-like expression directly follows an instantiation expression.
    fn has_invalid_instantiation_access_receiver(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        // extract the receiver for member-like expressions
        let left_expression_id = match tree.get(expression_id) {
            Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Index { left, .. } => *left,
            _ => return false,
        };

        self.is_unparenthesized_instantiation_receiver(tree, left_expression_id)
    }

    /// Return true when a member-like expression follows an instantiation of a type-like symbol.
    fn expression_has_type_like_instantiation_receiver(
        &self,
        ctx: &TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        // extract the receiver for member-like expressions
        let left_expression_id = match ctx.tree.get(expression_id) {
            Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Index { left, .. } => *left,
            _ => return false,
        };
        let receiver_expression_id = match ctx.tree.get(left_expression_id) {
            Expression::Instantiation { left, .. } => *left,
            Expression::UnresolvedPath {
                generic_arguments, ..
            }
            | Expression::LocalReference {
                generic_arguments, ..
            }
            | Expression::ModuleReference {
                generic_arguments, ..
            }
            | Expression::GlobalReference {
                generic_arguments, ..
            } if !generic_arguments.is_empty() => left_expression_id,
            _ => return false,
        };

        // resolve the instantiated base symbol
        let Some(mut lookup_symbol) = ctx.tree.get(receiver_expression_id).target_symbol() else {
            return false;
        };

        lookup_symbol = self.forwarded_symbol_id(ctx.module_symbol_view(), lookup_symbol);
        lookup_symbol = self
            .declaration_symbol_id(ctx.module_symbol_view(), lookup_symbol)
            .unwrap_or(lookup_symbol);

        matches!(
            lookup_symbol.ty(),
            SymbolType::Class
                | SymbolType::Struct
                | SymbolType::Interface
                | SymbolType::Enum
                | SymbolType::TypeAlias
                | SymbolType::Newtype
        )
    }

    /// Return true when this expression is in a type position.
    fn is_type_position_for_instantiation_access(
        &self,
        ctx: &TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        let mut current_expression_id = expression_id;
        loop {
            let Some(parent) = ctx.tree.get_parent(current_expression_id.id) else {
                return false;
            };

            match parent.ty {
                // expression parents can either be type operators or wrappers
                NodeType::Expression => {
                    let parent_expression_id = parent.into_typed::<Expression>();
                    let parent_expression = ctx.tree.get(parent_expression_id);

                    match self.type_position_step_for_parent_expression(
                        ctx.tree,
                        parent_expression,
                        current_expression_id,
                    ) {
                        TypePositionStep::Ascend => {
                            current_expression_id = parent_expression_id;
                            continue;
                        }
                        TypePositionStep::NotType => return false,
                    }
                }

                // declarator annotations are type positions
                NodeType::Declarator => {
                    let declarator = ctx.tree.get(parent.into_typed::<Declarator>());
                    return declarator.ty.is_some_and(|type_id| {
                        self.expression_is_within_type_expression_subtree(
                            ctx.tree,
                            current_expression_id,
                            type_id,
                        )
                    });
                }

                // type expression parents always mean we are inside type syntax
                NodeType::TypeExpression => return true,

                // declaration type slots carry type context
                NodeType::Declaration => {
                    let declaration = ctx.tree.get(parent.into_typed::<Declaration>());
                    return self.declaration_expression_is_type_position(
                        ctx.tree,
                        declaration,
                        current_expression_id,
                    );
                }

                // member type slots carry type context
                NodeType::Member => {
                    let member = ctx.tree.get(parent.into_typed::<Member>());
                    return self.member_expression_is_type_position(
                        ctx.tree,
                        member,
                        current_expression_id,
                    );
                }

                // property method return types carry type context
                NodeType::Property => {
                    let property = ctx.tree.get(parent.into_typed::<Property>());
                    return self.property_expression_is_type_position(
                        ctx.tree,
                        property,
                        current_expression_id,
                    );
                }

                // parameter annotations are tracked through declared types
                NodeType::Parameter => {
                    return self.parameter_expression_is_type_position(
                        ctx,
                        parent.into_typed::<Parameter>(),
                        current_expression_id,
                    );
                }

                // where clause right sides are type constraints
                NodeType::WhereClause => {
                    let where_clause = ctx.tree.get(parent.into_typed::<WhereClause>());
                    return self.expression_is_within_type_expression_subtree(
                        ctx.tree,
                        current_expression_id,
                        where_clause.right,
                    );
                }

                _ => return false,
            }
        }
    }

    /// Classify one expression parent edge for type-position traversal.
    fn type_position_step_for_parent_expression(
        &self,
        _tree: &NodeTree,
        parent_expression: &Expression,
        child_expression_id: LocalNodeId<Expression>,
    ) -> TypePositionStep {
        match parent_expression {
            Expression::Parenthesized { expression } => {
                if *expression == child_expression_id {
                    TypePositionStep::Ascend
                } else {
                    TypePositionStep::NotType
                }
            }
            _ => TypePositionStep::NotType,
        }
    }

    /// Return true when a declaration expression slot is a type position.
    fn declaration_expression_is_type_position(
        &self,
        tree: &NodeTree,
        declaration: &Declaration,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        match declaration {
            Declaration::Type(declaration) => self.expression_is_within_type_expression_subtree(
                tree,
                expression_id,
                declaration.value,
            ),
            Declaration::Function(declaration) => {
                declaration
                    .signature
                    .return_type
                    .is_some_and(|return_type| {
                        self.expression_is_within_type_expression_subtree(
                            tree,
                            expression_id,
                            return_type,
                        )
                    })
                    && declaration.body != Some(expression_id)
            }
            Declaration::Extension(declaration) => {
                self.expression_is_within_type_expression_subtree(
                    tree,
                    expression_id,
                    declaration.target_type,
                ) || self.declaration_type_slots_contain_expression(
                    tree,
                    &declaration.implements_types,
                    expression_id,
                )
            }
            Declaration::Global(_) | Declaration::Namespace(_) | Declaration::ImportAlias(_) => {
                false
            }
            Declaration::Struct(declaration) => {
                self.declaration_type_slots_contain_expression(
                    tree,
                    &declaration.implements_types,
                    expression_id,
                ) || self.declaration_type_slots_contain_expression(
                    tree,
                    &declaration.embedded_types,
                    expression_id,
                )
            }
            Declaration::Class(declaration) => self.declaration_type_slots_contain_expression(
                tree,
                &declaration.implements_types,
                expression_id,
            ),
            Declaration::Enum(declaration) => self.declaration_type_slots_contain_expression(
                tree,
                &declaration.implements_types,
                expression_id,
            ),
            Declaration::Interface(declaration) => self.declaration_type_slots_contain_expression(
                tree,
                &declaration.extends_types,
                expression_id,
            ),
        }
    }

    /// Return true when a member expression slot is a type position.
    fn member_expression_is_type_position(
        &self,
        tree: &NodeTree,
        member: &Member,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        match member {
            Member::AssociatedType {
                constraint, value, ..
            } => {
                constraint.is_some_and(|type_id| {
                    self.expression_is_within_type_expression_subtree(tree, expression_id, type_id)
                }) || value.is_some_and(|type_id| {
                    self.expression_is_within_type_expression_subtree(tree, expression_id, type_id)
                })
            }
            Member::AssociatedConst { declared_type, .. } => declared_type.is_some_and(|type_id| {
                self.expression_is_within_type_expression_subtree(tree, expression_id, type_id)
            }),
            Member::Field { declared_type, .. } => declared_type.is_some_and(|type_id| {
                self.expression_is_within_type_expression_subtree(tree, expression_id, type_id)
            }),
            Member::Method { signature, .. } => signature.return_type.is_some_and(|return_type| {
                self.expression_is_within_type_expression_subtree(tree, expression_id, return_type)
            }),
            Member::Embed { value, .. } => {
                self.expression_is_within_type_expression_subtree(tree, expression_id, *value)
            }
            Member::StaticBlock { .. } | Member::ComptimeBlock { .. } | Member::Error { .. } => {
                false
            }
        }
    }

    /// Return true when a property expression slot is a type position.
    fn property_expression_is_type_position(
        &self,
        tree: &NodeTree,
        property: &Property,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        match property {
            Property::Method { signature, .. } => {
                signature.return_type.is_some_and(|return_type| {
                    self.expression_is_within_type_expression_subtree(
                        tree,
                        expression_id,
                        return_type,
                    )
                })
            }
            Property::Field { .. } | Property::Spread { .. } | Property::Error { .. } => false,
        }
    }

    /// Return true when a parameter expression slot is a type position.
    fn parameter_expression_is_type_position(
        &self,
        ctx: &TypeContext<'_>,
        parameter_id: LocalNodeId<Parameter>,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        let Some(declared_type_id) = ctx
            .types
            .get_declared_type_id(parameter_id.into_global_any(ctx.module.id))
        else {
            return false;
        };
        let Type::Unevaluated(type_root_expression_id) = ctx.types.get_type(declared_type_id)
        else {
            return false;
        };

        self.expression_is_within_type_expression_subtree(
            ctx.tree,
            expression_id,
            *type_root_expression_id,
        )
    }

    /// Return true when any declaration type slot contains this expression.
    fn declaration_type_slots_contain_expression(
        &self,
        tree: &NodeTree,
        type_slots: &[LocalNodeId<TypeExpression>],
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        type_slots.iter().copied().any(|type_id| {
            self.expression_is_within_type_expression_subtree(tree, expression_id, type_id)
        })
    }

    /// Return true when one expression is nested inside one type-expression subtree.
    fn expression_is_within_type_expression_subtree(
        &self,
        tree: &NodeTree,
        child_expression_id: LocalNodeId<Expression>,
        ancestor_type_expression_id: LocalNodeId<TypeExpression>,
    ) -> bool {
        let mut current_id = child_expression_id.into_any();
        let ancestor_id = ancestor_type_expression_id.into_any();

        loop {
            if current_id == ancestor_id {
                return true;
            }

            let Some(parent_id) = tree.get_parent(current_id.id) else {
                return false;
            };

            current_id = parent_id;
        }
    }

    /// Return true when an expression is an instantiation receiver without parentheses.
    fn is_unparenthesized_instantiation_receiver(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        match tree.get(expression_id) {
            // direct instantiation receivers are invalid for member-like access
            Expression::Instantiation { .. } => true,

            // references with static arguments are instantiation receivers too
            Expression::UnresolvedPath {
                generic_arguments, ..
            }
            | Expression::LocalReference {
                generic_arguments, ..
            }
            | Expression::ModuleReference {
                generic_arguments, ..
            }
            | Expression::GlobalReference {
                generic_arguments, ..
            } => !generic_arguments.is_empty(),

            // optional-chain wrappers preserve the original receiver shape
            Expression::Maybe { left } => {
                self.is_unparenthesized_instantiation_receiver(tree, *left)
            }

            // parenthesized receivers are explicitly allowed
            Expression::Parenthesized { .. } => false,

            _ => false,
        }
    }

    /// Return true when an expression is exactly `new.target`.
    fn expression_is_new_target(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        // dedicated new.target nodes are already exact
        if matches!(tree.get(expression_id), Expression::NewTarget) {
            return true;
        }

        // member form covers parenthesized `new.target`
        let Expression::Member { left, name } = tree.get(expression_id) else {
            return false;
        };

        let target_name = self.repository.strings.intern("target");
        if *name != Some(target_name) {
            return false;
        }

        let left = self.unwrap_parenthesized_expression(*left, tree);
        matches!(tree.get(left), Expression::NewTarget)
    }

    /// Return true when `new.target` is valid in the current lexical context.
    fn can_access_new_target_in_context(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        // walk up to the first function-like owner
        let mut current = Some(expression_id.into_any());
        while let Some(current_id) = current {
            let Some(parent) = tree.get_parent(current_id.id) else {
                return false;
            };

            if self.new_target_is_valid_lexical_owner(tree, parent) {
                return true;
            }

            current = Some(parent);
        }

        false
    }

    /// Return true when a node introduces a valid lexical owner for `new.target`.
    fn new_target_is_valid_lexical_owner(&self, tree: &NodeTree, node_id: LocalNodeIdAny) -> bool {
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
                    Member::Method { .. } | Member::StaticBlock { .. }
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

    // assignment target validation and for each binding checks
    /// Validate update expression targets.
    fn validate_update_target(
        &self,
        ctx: &mut TypeContext<'_>,
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
        self.validate_assignment_target(ctx, target, is_strict);
    }

    /// Validate for of binding constraints.
    fn validate_for_of_binding(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
        asynchrony: Asynchrony,
        kind: ForEachKind,
        binding: &ForEachBinding,
        is_strict: bool,
    ) {
        // validate assignment targets for non declaration bindings
        self.validate_for_each_assignment_binding(&mut ctx.reborrow(), binding, is_strict);

        // this rule applies only to sync for of loops
        if asynchrony != Asynchrony::Sync || kind != ForEachKind::Of {
            return;
        }

        // reject bindings named async
        if self.for_of_binding_is_async_identifier(ctx.tree, binding) {
            let node = expression_id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
            self.error(AnalyzeError::InvalidForOfBinding { node });
        }
    }

    /// Validate assignment target rules for for each non declaration bindings.
    fn validate_for_each_assignment_binding(
        &self,
        ctx: &mut TypeContext<'_>,
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

        // recurse through the binding pattern and validate assignment leaves
        self.validate_for_each_assignment_pattern(&mut ctx.reborrow(), *pattern, is_strict);
    }

    /// Validate assignment target rules for for each binding patterns.
    pub(super) fn validate_for_each_assignment_pattern(
        &self,
        ctx: &mut TypeContext<'_>,
        pattern_id: LocalNodeId<Pattern>,
        is_strict: bool,
    ) {
        match ctx.tree.get(pattern_id) {
            Pattern::Assign { pattern, .. } => {
                self.validate_for_each_assignment_pattern(&mut ctx.reborrow(), *pattern, is_strict);
            }
            // expression patterns must be valid assignment targets
            Pattern::Expression { value } => {
                self.validate_assignment_target(ctx, *value, is_strict);
            }

            // type-space patterns are never assignment targets
            Pattern::TypeExpression { .. } => {
                let node = pattern_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile));
                self.error(AnalyzeError::InvalidAssignmentTarget { node });
            }

            // binding names in strict mode cannot use reserved identifiers
            Pattern::Binding { name, pattern, .. } => {
                if let Some(pattern) = pattern {
                    self.validate_for_each_assignment_pattern(
                        &mut ctx.reborrow(),
                        *pattern,
                        is_strict,
                    );
                    return;
                }

                if ctx.module.is_user() && is_strict && self.is_reserved_binding_name(*name) {
                    let node = pattern_id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile));
                    self.error(AnalyzeError::ReservedIdentifier { node, name: *name });
                }
            }

            // unwrap wrapper patterns and validate inner leaves
            Pattern::Must(right)
            | Pattern::ReferenceOf { right, .. }
            | Pattern::ValueOf { right, .. } => {
                self.validate_for_each_assignment_pattern(&mut ctx.reborrow(), *right, is_strict);
            }

            // recurse into tuple, array and object fields
            Pattern::Tuple { fields }
            | Pattern::TaggedTuple { fields, .. }
            | Pattern::Array { fields }
            | Pattern::Object { fields }
            | Pattern::TaggedObject { fields, .. } => {
                for field_id in fields {
                    self.validate_for_each_assignment_field(
                        &mut ctx.reborrow(),
                        *field_id,
                        is_strict,
                    );
                }
            }

            // recurse through union branches
            Pattern::Union { patterns } => {
                for pattern_id in patterns {
                    self.validate_for_each_assignment_pattern(
                        &mut ctx.reborrow(),
                        *pattern_id,
                        is_strict,
                    );
                }
            }

            // wildcard is allowed as-is for non-javascript dialects
            Pattern::Wildcard => {}
        }
    }

    /// Validate assignment target rules for pattern fields in for each bindings.
    fn validate_for_each_assignment_field(
        &self,
        ctx: &mut TypeContext<'_>,
        field_id: LocalNodeId<PatternField>,
        is_strict: bool,
    ) {
        match ctx.tree.get(field_id) {
            // shorthand named fields bind directly by field name
            PatternField::Named { name, pattern, .. } => {
                if let Some(pattern) = pattern {
                    self.validate_for_each_assignment_pattern(
                        &mut ctx.reborrow(),
                        *pattern,
                        is_strict,
                    );
                    return;
                }

                if ctx.module.is_user() && is_strict && self.is_reserved_binding_name(*name) {
                    let node = field_id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile));
                    self.error(AnalyzeError::ReservedIdentifier { node, name: *name });
                }
            }

            // computed fields delegate validation to the value pattern
            PatternField::Computed { pattern, .. } => {
                self.validate_for_each_assignment_pattern(&mut ctx.reborrow(), *pattern, is_strict);
            }

            // positional fields forward to their pattern
            PatternField::Positional { pattern, .. } => {
                self.validate_for_each_assignment_pattern(&mut ctx.reborrow(), *pattern, is_strict);
            }

            // spread fields validate the spread pattern when present
            PatternField::Spread { pattern, .. } => {
                if let Some(pattern) = pattern {
                    self.validate_for_each_assignment_pattern(
                        &mut ctx.reborrow(),
                        *pattern,
                        is_strict,
                    );
                }
            }

            // elisions introduce no assignment targets
            PatternField::Elision => {}
        }
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
            } => self.repository.strings.get(*name) == "async",
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
                generic_arguments,
                ..
            }
            | Expression::LocalReference {
                path,
                generic_arguments,
                ..
            }
            | Expression::ModuleReference {
                path,
                generic_arguments,
                ..
            }
            | Expression::GlobalReference {
                path,
                generic_arguments,
                ..
            } => {
                if !generic_arguments.is_empty() {
                    return false;
                }

                path.segments.len() == 1 && self.repository.strings.get(path.segments[0]) == "async"
            }
            _ => false,
        }
    }

    /// Validate duplicate labels in nested label scopes.
    fn validate_duplicate_label(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
        label: StringId,
    ) {
        // walk parent labels until a function-like boundary
        let mut current = Some(expression_id.into_any());
        while let Some(node_id) = current {
            let Some(parent) = ctx.tree.get_parent(node_id.id) else {
                break;
            };

            // duplicate labels are invalid in the same label scope chain
            if parent.ty == NodeType::Expression {
                let parent_expression = ctx.tree.get(parent.into_typed::<Expression>());
                if let Expression::Labelled {
                    label: parent_label,
                    ..
                } = parent_expression
                    && *parent_label == label
                {
                    let node = expression_id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile));
                    self.error(AnalyzeError::DuplicateLabel { node });
                    return;
                }
            }

            // labels do not cross function-like boundaries
            if self.node_starts_function_scope(ctx.tree, parent) {
                break;
            }

            current = Some(parent);
        }
    }

    /// Validate catch parameter shape rules for JS/TS.
    fn validate_catch_binding_pattern(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
        catch_pattern_id: LocalNodeId<Pattern>,
    ) {
        // this restriction only applies to JS/TS source forms
        if !(ctx.module.language_type.is_javascript() || ctx.module.language_type.is_typescript()) {
            return;
        }

        // JS/TS allow identifier bindings and destructuring binding patterns
        if matches!(
            ctx.tree.get(catch_pattern_id),
            Pattern::Binding { .. } | Pattern::Array { .. } | Pattern::Object { .. }
        ) {
            return;
        }

        let node = expression_id
            .into_global_any(ctx.module.id)
            .into_anchored(Some(ctx.profile));
        self.error(AnalyzeError::InvalidCatchBinding { node });
    }

    /// Validate catch type annotations for JS/TS compatibility.
    fn validate_catch_annotation_type(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
        catch_ty_id: Option<LocalNodeId<TypeExpression>>,
    ) {
        // this restriction only applies to typed ts catch bindings
        if !ctx.module.language_type.is_typescript() {
            return;
        }

        let Some(catch_ty_id) = catch_ty_id else {
            return;
        };

        // reject annotations that are not `any` or `unknown`
        if !self.catch_annotation_expression_is_any_or_unknown(ctx.tree, catch_ty_id) {
            let node = expression_id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
            self.error(AnalyzeError::InvalidCatchAnnotationType { node });
        }
    }

    /// Validate try expressions include a catch or finally clause.
    fn validate_try_requires_catch_or_finally(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
        catch_expression: Option<LocalNodeId<Expression>>,
        finally_expression: Option<LocalNodeId<Expression>>,
    ) {
        if catch_expression.is_some() || finally_expression.is_some() {
            return;
        }

        let node = expression_id
            .into_global_any(ctx.module.id)
            .into_anchored(Some(ctx.profile));
        self.error(AnalyzeError::IncompleteTry { node });
    }

    /// Return true when a catch annotation expression is `any` or `unknown`.
    fn catch_annotation_expression_is_any_or_unknown(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<TypeExpression>,
    ) -> bool {
        match tree.get(expression_id) {
            TypeExpression::Literal {
                value: TypeLiteral::Any | TypeLiteral::Unknown,
            } => true,
            TypeExpression::Parenthesized { expression } => {
                self.catch_annotation_expression_is_any_or_unknown(tree, *expression)
            }
            _ => false,
        }
    }

    /// Return true when a node starts a fresh label scope.
    fn node_starts_function_scope(&self, tree: &NodeTree, node_id: LocalNodeIdAny) -> bool {
        match node_id.ty {
            NodeType::Expression => {
                let expression = tree.get(node_id.into_typed::<Expression>());
                let Expression::Declaration(declaration) = expression else {
                    return false;
                };

                matches!(tree.get(*declaration), Declaration::Function { .. })
            }
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
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        operator: BinaryOperator,
    ) {
        // only enforce the JS/TS exponentiation grammar
        if !(ctx.module.language_type.is_javascript() || ctx.module.language_type.is_typescript()) {
            return;
        }

        // skip non exponent operators
        if operator != BinaryOperator::Exponent {
            return;
        }

        // reject unparenthesized unary and delete operands
        let left_expression = ctx.tree.get(left);
        if matches!(
            left_expression,
            Expression::Unary { .. } | Expression::Delete { .. }
        ) {
            let node = expression_id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
            self.error(AnalyzeError::InvalidExponentLeftUnary { node });
        }
    }

    /// Validate empty parenthesized expressions in JS/TS.
    fn validate_empty_parenthesized_expression(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
        elements: &[LocalNodeId<Argument>],
    ) {
        // only enforce for JS/TS modules
        if !(ctx.module.language_type.is_javascript() || ctx.module.language_type.is_typescript()) {
            return;
        }

        // tuple expressions in value position represent parenthesized expressions
        if elements.is_empty() {
            let node = expression_id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
            self.error(AnalyzeError::EmptyParenthesizedExpression { node });
        }
    }

    /// Validate empty sequence expressions used as parenthesized forms in JS/TS.
    fn validate_empty_parenthesized_sequence(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
        expressions: &[LocalNodeId<Expression>],
    ) {
        // only enforce for JS/TS modules
        if !(ctx.module.language_type.is_javascript() || ctx.module.language_type.is_typescript()) {
            return;
        }

        // JS/TS represent `()` as an empty sequence expression
        if expressions.is_empty() {
            let node = expression_id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
            self.error(AnalyzeError::EmptyParenthesizedExpression { node });
        }
    }

    /// Validate delete expression restrictions.
    fn validate_delete_expression(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        is_strict: bool,
    ) {
        // skip non-user modules
        if !ctx.module.is_user() {
            return;
        }

        // classify the effective delete target
        let target_id = self.effective_delete_target(ctx.tree, value);

        // reject private member deletes
        if self.delete_target_contains_private_member(ctx.tree, target_id) {
            let node = expression_id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
            self.error(AnalyzeError::InvalidStrictDelete { node });
            return;
        }

        // enforce strict mode delete restrictions for identifier targets
        if is_strict && self.delete_target_is_binding_reference(ctx.tree, target_id) {
            let node = expression_id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
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
        match tree.get(target_id) {
            Expression::UnresolvedPath {
                generic_arguments, ..
            }
            | Expression::LocalReference {
                generic_arguments, ..
            }
            | Expression::ModuleReference {
                generic_arguments, ..
            }
            | Expression::GlobalReference {
                generic_arguments, ..
            } => generic_arguments.is_empty(),
            _ => false,
        }
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
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
        tag: LocalNodeId<Expression>,
    ) {
        // optional chain tagged templates are invalid in js, ts, and destack
        if !ctx.module.language_type.is_javascript()
            && !ctx.module.language_type.is_typescript()
            && !ctx.module.language_type.is_destack()
        {
            return;
        }

        // check the tag chain for optional segments
        if self.expression_contains_optional_chain(ctx.tree, tag) {
            let node = expression_id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
            self.error(AnalyzeError::InvalidOptionalChainTemplate { node });
        }
    }

    /// Validate super call expressions.
    pub(crate) fn validate_super_call_expression(
        &self,
        ctx: &TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
    ) {
        // parenthesized super calls are always invalid
        if self.has_parenthesized_super_reference(ctx.tree, left) {
            let node = expression_id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
            self.error(AnalyzeError::InvalidSuperCall { node });
            return;
        }

        // skip non super calls
        let left = self.unwrap_parenthesized_expression(left, ctx.tree);
        if !matches!(ctx.tree.get(left), Expression::Super) {
            return;
        }

        // allow super calls only in derived constructors
        if self.super_call_is_valid_context(ctx.tree, expression_id) {
            return;
        }

        let node = expression_id
            .into_global_any(ctx.module.id)
            .into_anchored(Some(ctx.profile));
        self.error(AnalyzeError::InvalidSuperCall { node });
    }

    /// Validate optional chains rooted at super.
    pub(crate) fn validate_super_optional_chain(
        &self,
        ctx: &TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
    ) {
        // skip non super chains
        if !self.expression_roots_in_super(ctx.tree, left) {
            return;
        }
        let node = expression_id
            .into_global_any(ctx.module.id)
            .into_anchored(Some(ctx.profile));
        self.error(AnalyzeError::InvalidSuperOptionalChain { node });
    }

    /// Validate non-call super property access.
    pub(crate) fn validate_super_property_expression(
        &self,
        ctx: &TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
    ) {
        // skip plain super calls, they are checked separately
        if let Expression::Call { left, .. } = ctx.tree.get(expression_id)
            && self.expression_is_super_reference(ctx.tree, *left)
        {
            return;
        }

        // parenthesized super access is always invalid
        if self.has_parenthesized_super_reference(ctx.tree, expression_id) {
            let node = expression_id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
            self.error(AnalyzeError::InvalidSuperCall { node });
            return;
        }

        // reject `new super` and `new super(...)` forms in all contexts
        if let Expression::New { left, .. } = ctx.tree.get(expression_id)
            && self.expression_roots_in_super(ctx.tree, *left)
        {
            let node = expression_id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
            self.error(AnalyzeError::InvalidSuperCall { node });
            return;
        }

        // skip expressions that are not rooted in super
        if !self.expression_roots_in_super(ctx.tree, expression_id) {
            return;
        }

        // allow contexts that have valid super bindings
        if self.super_property_is_valid_context(ctx.tree, expression_id) {
            return;
        }

        let node = expression_id
            .into_global_any(ctx.module.id)
            .into_anchored(Some(ctx.profile));
        self.error(AnalyzeError::InvalidSuperCall { node });
    }

    /// Validate bare super references.
    pub(crate) fn validate_super_reference_expression(
        &self,
        ctx: &TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
    ) {
        if self.super_reference_is_part_of_expression_chain(ctx.tree, expression_id) {
            return;
        }

        let node = expression_id
            .into_global_any(ctx.module.id)
            .into_anchored(Some(ctx.profile));
        self.error(AnalyzeError::InvalidSuperCall { node });
    }

    /// Return true when `super` is consumed by a larger expression chain.
    fn super_reference_is_part_of_expression_chain(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        let Some(parent) = tree.get_parent(expression_id.id) else {
            return false;
        };
        if parent.ty != NodeType::Expression {
            return false;
        }

        let parent_expression_id = parent.into_typed::<Expression>();
        match tree.get(parent_expression_id) {
            Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Call { left, .. }
            | Expression::New { left, .. }
            | Expression::Index { left, .. }
            | Expression::Maybe { left }
            | Expression::Must { left } => *left == expression_id,
            Expression::Parenthesized { expression } => {
                if *expression != expression_id {
                    return false;
                }

                self.is_parenthesized_super_in_expression_chain(tree, parent_expression_id)
            }
            _ => false,
        }
    }

    /// Return true when `(super)` is consumed by a larger expression chain.
    fn is_parenthesized_super_in_expression_chain(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        let Some(parent) = tree.get_parent(expression_id.id) else {
            return false;
        };
        if parent.ty != NodeType::Expression {
            return false;
        }

        let parent_expression_id = parent.into_typed::<Expression>();
        match tree.get(parent_expression_id) {
            Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Call { left, .. }
            | Expression::New { left, .. }
            | Expression::Index { left, .. }
            | Expression::Maybe { left }
            | Expression::Must { left } => *left == expression_id,
            _ => false,
        }
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
            | Expression::New { left, .. }
            | Expression::Index { left, .. }
            | Expression::Maybe { left }
            | Expression::Must { left } => self.expression_roots_in_super(tree, *left),
            _ => false,
        }
    }

    /// Return true when an expression chain contains `(super)` directly.
    fn has_parenthesized_super_reference(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        match tree.get(expression_id) {
            Expression::Parenthesized { expression } => {
                self.expression_is_super_reference(tree, *expression)
                    || self.has_parenthesized_super_reference(tree, *expression)
            }
            Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Call { left, .. }
            | Expression::New { left, .. }
            | Expression::Index { left, .. }
            | Expression::Maybe { left }
            | Expression::Must { left } => self.has_parenthesized_super_reference(tree, *left),
            _ => false,
        }
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
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
        kind: MatchKind,
        value: LocalNodeId<Expression>,
        cases: &[LocalNodeId<MatchCase>],
    ) {
        // switch cases require expressions without guards
        if kind == MatchKind::Switch {
            for case_id in cases {
                let (selector, case_span_id) = match ctx.tree.get(*case_id) {
                    MatchCase::Expression { selector, .. } | MatchCase::Block { selector, .. } => {
                        (selector, *case_id)
                    }
                };

                if let MatchSelector::Pattern { pattern, guard } = selector {
                    // reject guarded switch cases
                    if guard.is_some() {
                        self.error(AnalyzeError::InvalidSwitchCaseGuard {
                            node: case_span_id
                                .into_global_any(ctx.module.id)
                                .into_anchored(Some(ctx.profile)),
                        });
                    }

                    // reject non expression switch cases
                    match ctx.tree.get(*pattern) {
                        Pattern::Expression { value } => {
                            if self.is_invalid_switch_case_expression(ctx.tree, *value) {
                                self.error(AnalyzeError::InvalidSwitchCasePattern {
                                    node: pattern
                                        .into_global_any(ctx.module.id)
                                        .into_anchored(Some(ctx.profile)),
                                });
                            }
                        }
                        _ => {
                            self.error(AnalyzeError::InvalidSwitchCasePattern {
                                node: pattern
                                    .into_global_any(ctx.module.id)
                                    .into_anchored(Some(ctx.profile)),
                            });
                        }
                    }
                }
            }

            return;
        }

        // match expressions require exhaustiveness checks
        self.validate_match_exhaustiveness(&mut ctx.reborrow(), expression_id, value, cases);
    }

    /// Validate must assertion usage.
    fn validate_must_assertion(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
        no_must_assertions: bool,
    ) {
        if !no_must_assertions {
            return;
        }

        let node = expression_id
            .into_global_any(ctx.module.id)
            .into_anchored(Some(ctx.profile));
        self.error(AnalyzeError::MustAssertionDisabled { node });
    }

    /// Validate guard expressions that rely on runtime narrowing.
    fn validate_unsound_narrowing_guard(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
        no_unsound_narrowing: bool,
    ) {
        if !no_unsound_narrowing {
            return;
        }

        let runtime_check = ctx
            .types
            .get_runtime_check_kind(expression_id.into_global_any(ctx.module.id));
        if matches!(
            runtime_check,
            Some(RuntimeCheckKind::UnionTag) | Some(RuntimeCheckKind::Constant(_))
        ) {
            return;
        }

        let node = expression_id
            .into_global_any(ctx.module.id)
            .into_anchored(Some(ctx.profile));
        self.error(AnalyzeError::UnsoundNarrowingDisabled { node });
    }

    /// Validate assignment targets for assignment expressions.
    pub(crate) fn validate_assignment_target(
        &self,
        ctx: &TypeContext<'_>,
        target: LocalNodeId<Expression>,
        is_strict: bool,
    ) {
        // reject non-assignable targets
        if !self.is_valid_assignment_target(ctx.tree, target) {
            let node = target
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
            self.error(AnalyzeError::InvalidAssignmentTarget { node });
            return;
        }

        // reject strict mode assignments to reserved binding names
        if ctx.module.is_user()
            && is_strict
            && let Some((reserved_target, name)) =
                self.strict_reserved_assignment_target_binding(ctx.tree, target)
        {
            let node = reserved_target
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
            self.error(AnalyzeError::ReservedIdentifier { node, name });
        }
    }

    /// Return reserved strict-mode assignment targets with their binding name.
    fn strict_reserved_assignment_target_binding(
        &self,
        tree: &NodeTree,
        target: LocalNodeId<Expression>,
    ) -> Option<(LocalNodeId<Expression>, StringId)> {
        let target = self.assignment_target_base(tree, target);
        let name = self.assignment_target_binding_name(tree, target)?;
        if !self.is_reserved_binding_name(name) {
            return None;
        }

        Some((target, name))
    }

    /// Return the wrapped assignment target base expression.
    fn assignment_target_base(
        &self,
        tree: &NodeTree,
        target: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        let mut target = target;

        loop {
            // strip transparent wrapper expressions used in assignment targets
            match tree.get(target) {
                Expression::Parenthesized { expression } => {
                    target = *expression;
                }
                Expression::Must { left, .. } => {
                    target = *left;
                }
                Expression::As { expression, .. } | Expression::Satisfies { expression, .. } => {
                    target = *expression;
                }
                _ => break target,
            }
        }
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
                generic_arguments,
                ..
            }
            | Expression::LocalReference {
                path,
                generic_arguments,
                ..
            }
            | Expression::ModuleReference {
                path,
                generic_arguments,
                ..
            }
            | Expression::GlobalReference {
                path,
                generic_arguments,
                ..
            } => {
                if !generic_arguments.is_empty() {
                    return None;
                }

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
        let target = self.assignment_target_base(tree, target);
        match tree.get(target) {
            Expression::UnresolvedPath {
                generic_arguments, ..
            }
            | Expression::LocalReference {
                generic_arguments, ..
            }
            | Expression::ModuleReference {
                generic_arguments, ..
            }
            | Expression::GlobalReference {
                generic_arguments, ..
            } => generic_arguments.is_empty(),
            Expression::Member { .. } | Expression::PrivateMember { .. } => true,
            Expression::Index { .. } => true,
            _ => false,
        }
    }

    /// Validate an import or export expression.
    fn validate_dependency_expression(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
        source: ImportSource,
        kind: DependencyKind,
        items: Option<&[LocalNodeId<DependencyItem>]>,
    ) {
        // reject type-only dependencies in JavaScript modules
        if ctx.module.language_type.is_javascript() && kind == DependencyKind::Type {
            let node = expression_id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
            self.error(AnalyzeError::TypeScriptSyntaxInJavaScript { node });
        }

        // validate type-only import bindings
        if let Some(items) = items
            && matches!(
                source,
                ImportSource::ImportStatement | ImportSource::ImportEquals
            )
        {
            self.validate_type_only_import_bindings(&mut ctx.reborrow(), kind, items);
        }
    }

    /// Validate private identifier usage inside expressions.
    fn validate_private_identifier_expression(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
    ) {
        // allow private identifiers only as the left operand of an `in` expression
        if self.private_identifier_is_in_expression(ctx.tree, expression_id) {
            return;
        }

        let node = expression_id
            .into_global_any(ctx.module.id)
            .into_anchored(Some(ctx.profile));
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

    /// Validate defaults on object literal properties.
    fn validate_object_literal_properties(
        &self,
        ctx: &mut TypeContext<'_>,
        properties: &[LocalNodeId<Property>],
    ) {
        // reserve the proto setter key once per object
        let proto_name = self.repository.strings.intern("__proto__");

        // validate each property for object literal restrictions
        for property_id in properties {
            let property = ctx.tree.get(*property_id);

            // reject object literal `__proto__` setter fields
            if self.is_object_proto_setter_field(property, proto_name) {
                let node = property_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile));
                self.error(AnalyzeError::UnsupportedObjectPrototypeSetter { node });
            }

            // validate accessor signatures on object literals
            if let Property::Method {
                signature, body, ..
            } = property
            {
                // strict directive prologues require simple parameter lists in JS/TS modes
                if !ctx.module.language_type.is_destack()
                    && let Some(body) = body
                    && self.has_non_simple_dynamic_parameters(ctx.tree, &signature.parameters)
                    && self.body_declares_use_strict_directive(ctx.tree, *body)
                {
                    self.error(AnalyzeError::InvalidFunction {
                        node: property_id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile)),
                    });
                }

                // object literal accessors still use accessor signature validation
                self.validate_accessor_signature(ctx, (*property_id).into_any(), signature);
            }
        }
    }

    /// Return true when a property defines an object literal `__proto__` setter.
    fn is_object_proto_setter_field(&self, property: &Property, proto_name: StringId) -> bool {
        matches!(
            property,
            Property::Field {
                key: Key::Name(name),
                ..
            } if name.string() == proto_name
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
                    if let Declaration::Function(declaration) = declaration {
                        return declaration.signature.asynchrony == Asynchrony::Async;
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
        ctx: &mut TypeContext<'_>,
        kind: DependencyKind,
        items: &[LocalNodeId<DependencyItem>],
    ) {
        // diagnostics
        let report_error = |node: LocalNodeIdAny| {
            let node = node.into_anchored(ctx.module.id, Some(ctx.profile));
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
            let mode = match ctx.tree.get(*item_id) {
                DependencyItem::UnresolvedRemote { mode, .. }
                | DependencyItem::UnresolvedLocal { mode, .. }
                | DependencyItem::Local { mode, .. }
                | DependencyItem::Remote { mode, .. }
                | DependencyItem::Value { mode, .. } => *mode,
                DependencyItem::Error => continue,
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

    /// Validate tuple optional element ordering.
    fn validate_tuple_optional_order(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
        elements: &[LocalNodeId<Argument>],
    ) {
        // skip non tuple arrays
        if !elements
            .iter()
            .any(|argument_id| self.tuple_element_is_optional(ctx.tree, *argument_id))
        {
            return;
        }

        // enforce optional element ordering
        let mut optional_seen = false;
        for argument_id in elements {
            let is_optional = self.tuple_element_is_optional(ctx.tree, *argument_id);
            let is_rest = matches!(ctx.tree.get(*argument_id), Argument::Spread { .. });

            // tuple members cannot be both optional and rest
            if is_optional && is_rest {
                let node = expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile));
                self.error(AnalyzeError::InvalidTupleElementOrder { node });
                return;
            }

            if optional_seen && !is_optional && !is_rest {
                let node = expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile));
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
        let _ = tree;
        let _ = argument_id;

        false
    }

    /// Validate a declare binding initializer.
    fn validate_declare_binding_initializer(
        &self,
        ctx: &mut TypeContext<'_>,
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
        let declarator = ctx.tree.get(declarator_id);
        if declarator.value.is_some() {
            let node = declarator_id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
            self.error(AnalyzeError::InvalidDeclareInitializer { node });
        }
    }

    /// Validate a const binding initializer.
    fn validate_const_initializer(
        &self,
        ctx: &mut TypeContext<'_>,
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
            let declarator = ctx.tree.get(declarator_id);
            let Some(value) = declarator.value else {
                return;
            };
            if !self.is_valid_ambient_const_initializer(&mut ctx.reborrow(), value) {
                let node = declarator_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile));
                self.error(AnalyzeError::InvalidAmbientConstInitializer { node });
            }
            return;
        }

        // report missing initializers in const bindings
        let declarator = ctx.tree.get(declarator_id);
        if declarator.value.is_some() {
            return;
        }

        // destructuring bindings already report a dedicated initializer error
        if self.is_destructuring_pattern(ctx.tree, declarator.pattern) {
            return;
        }

        let node = declarator_id
            .into_global_any(ctx.module.id)
            .into_anchored(Some(ctx.profile));
        self.error(AnalyzeError::MissingConstInitializer { node });
    }

    /// Validate definite assignment assertions in variable declarators.
    fn validate_definite_assignment_declarator(
        &self,
        ctx: &mut TypeContext<'_>,
        declarator_id: LocalNodeId<Declarator>,
        no_definite_assignment_assertions: bool,
    ) {
        // report definite assignment assertions in variable declarators
        let declarator = ctx.tree.get(declarator_id);
        if !self.pattern_has_definite_assignment(ctx.tree, declarator.pattern) {
            return;
        }
        let node = declarator_id
            .into_global_any(ctx.module.id)
            .into_anchored(Some(ctx.profile));

        if no_definite_assignment_assertions {
            self.error(AnalyzeError::DefiniteAssignmentAssertionDisabled { node });
            return;
        }

        if !ctx.module.language_type.is_destack() {
            self.error(AnalyzeError::InvalidDefiniteAssignmentDeclarator { node });
        }
    }

    /// Check whether an ambient const initializer is valid.
    fn is_valid_ambient_const_initializer(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        let expression = ctx.tree.get(expression_id);

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
            let right_expression = ctx.tree.get(*right);
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
        if let Expression::Member { left, .. } = expression {
            return self.is_ambient_const_enum_reference(&mut ctx.reborrow(), *left);
        }

        false
    }

    /// Validate type index access for missing members.
    pub(super) fn validate_type_index_access(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
    ) {
        let TypeExpression::Index { left, index } = ctx.tree.get(expression_id) else {
            return;
        };

        // read operand types from existing declare or infer commitments
        let Some(left_ty_id) = ctx
            .types
            .get_declared_or_inferred_type_id(left.into_global_any(ctx.module.id))
        else {
            return;
        };

        // skip missing checks for unresolved type parameters
        if let Some(symbol) = ctx.types.get_type(left_ty_id).symbol()
            && self.symbol_is_static_parameter(ctx.symbol_type_view(), symbol)
        {
            return;
        }

        // skip missing checks when the type index builds a fixed-size array
        let is_index_access =
            match self.type_index_uses_index_access(&mut ctx.reborrow(), left_ty_id, *index) {
                Ok(value) => value,
                Err(error) => {
                    self.error(error);
                    return;
                }
            };
        if !is_index_access {
            return;
        }

        // read index types only for true index-access expressions
        let Some(index_ty_id) = ctx
            .types
            .get_declared_or_inferred_type_id(index.into_global_any(ctx.module.id))
        else {
            return;
        };

        // skip missing checks for any or unknown receivers
        if matches!(
            ctx.types.get_type(left_ty_id),
            Type::TypeLiteral {
                value: TypeLiteral::Any | TypeLiteral::Unknown,
            }
        ) {
            return;
        }

        // resolve index access types to detect missing keys
        let mut visited = Vec::new();
        let resolution = self.resolve_index_access_types(
            &mut ctx.reborrow(),
            expression_id.into_any(),
            left_ty_id,
            index_ty_id,
            NormalizationMode::Flow,
            RelationMode::INDEX_ACCESS,
            &mut visited,
        );
        let Some(missing_key) = resolution.missing_keys.first() else {
            return;
        };

        // report missing key access unless a primary receiver error blocks cascades
        let blocker =
            match self.should_block_missing_member_diagnostic(ctx.type_view(), left_ty_id, false) {
                Ok(blocker) => blocker,
                Err(error) => {
                    self.error(error);
                    return;
                }
            };
        if blocker.is_some() {
            return;
        }

        let error = AnalyzeError::MissingMember {
            node: expression_id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile)),
            receiver_ty: left_ty_id.into_global(ctx.module.id),
            member_key: *missing_key,
        };
        debug_assert!(error.is_cascading_semantic_diagnostic());
        self.error(error);
    }

    /// Check whether an expression is an enum reference for ambient const initializers.
    fn is_ambient_const_enum_reference(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        let target_symbol = match ctx.tree.get(expression_id) {
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => *target_symbol,
            _ => return false,
        };

        self.symbol_is_enum(&mut ctx.reborrow(), target_symbol)
    }

    /// Check whether a symbol resolves to an enum declaration.
    fn symbol_is_enum(&self, ctx: &mut TypeContext<'_>, symbol_id: GlobalSymbolId) -> bool {
        // check symbols from the current module
        if symbol_id.module_id == ctx.module.id {
            let symbol = ctx.symbols.get_symbol(symbol_id.local_id);
            return symbol.ty == SymbolType::Enum;
        }

        // check symbols from dependent modules
        let Some(target_dir) = self.dir_declared(symbol_id.module_id, ctx.profile) else {
            return false;
        };
        let symbol = target_dir.symbols.get_symbol(symbol_id.local_id);
        symbol.ty == SymbolType::Enum
    }

    /// Validate declarator patterns against JS/TS compatibility rules.
    /// (Destack is more permissive around declarators and patterns)
    fn validate_js_ts_compat_declarator_pattern(
        &self,
        ctx: &mut TypeContext<'_>,
        declarator_id: LocalNodeId<Declarator>,
    ) {
        // only JS/TS require assignment style declarator bindings
        if !(ctx.module.language_type.is_javascript() || ctx.module.language_type.is_typescript()) {
            return;
        }

        let declarator = ctx.tree.get(declarator_id);
        let Pattern::Expression { value } = ctx.tree.get(declarator.pattern) else {
            return;
        };
        if self.is_valid_js_ts_compat_declarator_binding(ctx.tree, *value) {
            return;
        }

        let node = declarator
            .pattern
            .into_global_any(ctx.module.id)
            .into_anchored(Some(ctx.profile));
        self.error(AnalyzeError::InvalidAssignmentTarget { node });
    }

    /// Return true when an expression is valid for JS/TS declarator binding compatibility.
    fn is_valid_js_ts_compat_declarator_binding(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        match tree.get(expression_id) {
            Expression::UnresolvedPath {
                path,
                generic_arguments,
                ..
            }
            | Expression::LocalReference {
                path,
                generic_arguments,
                ..
            }
            | Expression::ModuleReference {
                path,
                generic_arguments,
                ..
            }
            | Expression::GlobalReference {
                path,
                generic_arguments,
                ..
            } => generic_arguments.is_empty() && path.segments.len() == 1,
            _ => false,
        }
    }

    /// Validate a destructuring declaration without an initializer.
    fn validate_destructuring_initializer(
        &self,
        ctx: &mut TypeContext<'_>,
        declarator_id: LocalNodeId<Declarator>,
    ) {
        let declarator = ctx.tree.get(declarator_id);
        if declarator.value.is_some() {
            return;
        }

        // destructuring bindings require initializers
        if self.is_destructuring_pattern(ctx.tree, declarator.pattern) {
            let node = declarator_id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
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
                let name = self.repository.strings.get(path.segments[0]);
                name == "_"
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Expression;
    use crate::tests::TestProgram;

    /// Allow exhaustive matches over fixed arrays with repeated element types.
    #[test]
    fn test_allow_exhaustive_match_fixed_array_with_repeated_element_type() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
declare const pair: int32[2];

match (pair) {
    [left, right] => {
        left satisfies int32;
        right satisfies int32;
    }
}
"#,
        );

        test.analyze_module(module_id);
        test.compile();

        // exhaustive fixed-array match should not raise EA400
        test.check_no_diagnostic_code("EA400");
    }

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
        test.apply_destack_config(
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

    /// Allow parenthesized cast assignment targets with assignable bases.
    #[test]
    fn test_allow_parenthesized_cast_assignment_target() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ts",
            r#"
let value = 1;
(value as number) = 2;
"#,
        );
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EA226");
    }

    /// Reject parenthesized cast assignment targets with non-assignable call bases.
    #[test]
    fn test_reject_parenthesized_cast_call_assignment_target() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ts",
            r#"
const value = () => 1;
(value() as number) = 2;
"#,
        );
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA226");
    }

    /// Allow non-null assertion assignment targets with assignable bases.
    #[test]
    fn test_allow_non_null_assertion_assignment_target() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ts",
            r#"
let value: number | undefined = 1;
value! = 2;
"#,
        );
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EA226");
    }

    /// Reject non-null assertion assignment targets with non-assignable call bases.
    #[test]
    fn test_reject_non_null_assertion_call_assignment_target() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ts",
            r#"
const value = () => 1;
value()! = 2;
"#,
        );
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA226");
    }

    /// Reject direct property access after instantiation expressions.
    #[test]
    fn test_reject_instantiation_property_access() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ts",
            r#"
const value = f<T>.x;
"#,
        );
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA234");
    }

    /// Allow parenthesized instantiation property access.
    #[test]
    fn test_allow_parenthesized_instantiation_property_access() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ts",
            r#"
const value = (f<T>).x;
"#,
        );
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EA234");
    }

    /// Allow instantiation property access in type annotations.
    #[test]
    fn test_allow_instantiation_property_access_in_type_annotation() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
struct Box<T> {
    type Item = T;
}

const value: Box<string>.Item = "ok";
value;
"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EA234");
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
        test.apply_destack_config(
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
        test.apply_destack_config(
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
        test.apply_destack_config(
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
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EA244");
    }

    /// Reject parenthesized super member access in methods.
    #[test]
    fn test_reject_parenthesized_super_member_access_in_method() {
        let test = TestProgram::memory_sequential();

        // source: class A extends B { m() { (super).x; } }
        let module_id = test.add_module(
            "test.js",
            r#"
class A extends B {
    m() {
        (super).x;
    }
}
"#,
        );
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA244");
    }

    /// Reject parenthesized super calls in constructors.
    #[test]
    fn test_reject_parenthesized_super_call_in_constructor() {
        let test = TestProgram::memory_sequential();

        // source: class A extends B { constructor() { (super)(); } }
        let module_id = test.add_module(
            "test.js",
            r#"
class A extends B {
    constructor() {
        (super)();
    }
}
"#,
        );
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA244");
    }

    /// Reject new super expressions in constructors.
    #[test]
    fn test_reject_new_super_in_constructor() {
        let test = TestProgram::memory_sequential();

        // source: class A extends B { constructor() { new super(); } }
        let module_id = test.add_module(
            "test.js",
            r#"
class A extends B {
    constructor() {
        new super();
    }
}
"#,
        );
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA244");
    }

    /// Reject bare super expressions.
    #[test]
    fn test_reject_bare_super_expression() {
        let test = TestProgram::memory_sequential();

        // source: class A extends B { constructor() { super; } }
        let module_id = test.add_module(
            "test.js",
            r#"
class A extends B {
    constructor() {
        super;
    }
}
"#,
        );
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA244");
    }

    /// Reject null as a function binding identifier.
    #[test]
    fn test_reject_null_as_function_binding_identifier() {
        let test = TestProgram::memory_sequential();

        // source: function null() {}
        let module_id = test.add_module("test.js", "function null() {}");
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA214");
    }

    /// Reject reserved intrinsic names as TypeScript type alias identifiers.
    #[test]
    fn test_reject_reserved_typescript_type_alias_identifiers() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ts",
            r#"
type undefined = any;
type any = any;
type string = any;
"#,
        );

        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );

        test.analyze_module(module_id);
        test.compile();

        // each reserved alias name should raise EA214
        test.check_diagnostic_count("EA214", 3);
    }

    /// Reject try expressions without catch or finally clauses.
    #[test]
    fn test_reject_try_without_catch_or_finally_in_validate() {
        let test = TestProgram::memory_sequential();

        // source: try { 1; }
        let module_id = test.add_module("test.js", "try { 1; }");
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA307");
    }

    /// Allow try expressions with catch clauses.
    #[test]
    fn test_allow_try_with_catch_in_validate() {
        let test = TestProgram::memory_sequential();

        // source: try { 1; } catch { 2; }
        let module_id = test.add_module("test.js", "try { 1; } catch { 2; }");
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EA307");
    }

    /// Allow try expressions with finally clauses.
    #[test]
    fn test_allow_try_with_finally_in_validate() {
        let test = TestProgram::memory_sequential();

        // source: try { 1; } finally { 2; }
        let module_id = test.add_module("test.js", "try { 1; } finally { 2; }");
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EA307");
    }

    /// Reject non-binding js catch parameters in Analyze.
    #[test]
    fn test_reject_js_catch_expression_parameter_in_validate() {
        let test = TestProgram::memory_sequential();

        // source: try {} catch (answer()) {}
        let module_id = test.add_module("test.js", "try {} catch (answer()) {}");
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA323");
    }

    /// Allow non-binding catch patterns in Destack.
    #[test]
    fn test_allow_destack_catch_expression_parameter_in_validate() {
        let test = TestProgram::memory_sequential();

        // source: try {} catch (answer()) {}
        let module_id = test.add_module("test.ds", "try {} catch (answer()) {}");
        test.analyze_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EA323");
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
        test.apply_destack_config(
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
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA214");
    }

    /// Reject strict named function expressions that use reserved binding names.
    #[test]
    fn test_reject_strict_named_function_expression_reserved_name() {
        let test = TestProgram::memory_sequential();

        // source: function expression named with strict reserved identifier
        let module_id = test.add_module("test.js", r#""use strict"; !function eval(){};"#);
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA214");
    }

    /// Reject strict named class expressions that use reserved binding names.
    #[test]
    fn test_reject_strict_named_class_expression_reserved_name() {
        let test = TestProgram::memory_sequential();

        // source: class expression named with strict reserved identifier
        let module_id = test.add_module("test.js", r#""use strict"; !(class arguments {});"#);
        test.apply_destack_config(
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
        test.apply_destack_config(
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
        test.apply_destack_config(
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
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA226");
    }

    /// Reject non-assignable array pattern targets in for in bindings.
    #[test]
    fn test_reject_for_in_array_pattern_literal_target() {
        let test = TestProgram::memory_sequential();

        // source: for(([0]) in 0);
        let module_id = test.add_module("test.js", "for(([0]) in 0);");
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA226");
    }

    /// Reject non-assignable object pattern targets in for of bindings.
    #[test]
    fn test_reject_for_of_object_pattern_literal_target() {
        let test = TestProgram::memory_sequential();

        // source: for({a: 0} of 0);
        let module_id = test.add_module("test.js", "for({a: 0} of 0);");
        test.apply_destack_config(
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
        test.apply_destack_config(
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
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EA249");
    }

    /// Reject strict reserved identifier references in expression position.
    #[test]
    fn test_reject_strict_reserved_identifier_reference_expression() {
        let test = TestProgram::memory_sequential();

        // source: "use strict"; +protected;
        let module_id = test.add_module("test.js", r#""use strict"; +protected;"#);
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA214");
    }

    /// Reject strict reserved `with` references in statement position.
    #[test]
    fn test_reject_strict_with_identifier_reference() {
        let test = TestProgram::memory_sequential();

        // source: "use strict"; with(1);
        let module_id = test.add_module("test.js", r#""use strict"; with(1);"#);
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA214");
    }

    /// Reject strict reserved labels.
    #[test]
    fn test_reject_strict_reserved_label_identifier() {
        let test = TestProgram::memory_sequential();

        // source: "use strict"; yield:;
        let module_id = test.add_module("test.js", r#""use strict"; yield:;"#);
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA214");
    }

    /// Reject `new.target` outside function-like contexts.
    #[test]
    fn test_reject_new_target_at_top_level() {
        let test = TestProgram::memory_sequential();

        // source: var a = new.target;
        let module_id = test.add_module("test.js", "var a = new.target;");
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA248");
    }

    /// Allow `new.target` inside function-like contexts.
    #[test]
    fn test_allow_new_target_inside_function() {
        let test = TestProgram::memory_sequential();

        // source: function f() { return new.target; }
        let module_id = test.add_module("test.js", "function f() { return new.target; }");
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EA248");
    }

    /// Reject `new` calls rooted in optional chains.
    #[test]
    fn test_reject_new_optional_chain_expression() {
        let test = TestProgram::memory_sequential();

        // source: new Test?.test();
        let module_id = test.add_module("test.ts", "new Test?.test();");
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA250");
    }

    /// Reject labelled breaks that cross function boundaries.
    #[test]
    fn test_reject_break_label_across_function_boundary() {
        let test = TestProgram::memory_sequential();

        // source: a: while (true) { (function () { break a; }); }
        let module_id =
            test.add_module("test.js", "a: while (true) { (function () { break a; }); }");
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("ER201");
    }

    /// Reject non-simple function parameters with strict directive prologues in JS/TS.
    #[test]
    fn test_reject_function_non_simple_parameters_with_use_strict() {
        let test = TestProgram::memory_sequential();

        // source: function a([]){ "use strict"; }
        let module_id = test.add_module("test.js", r#"function a([]){ "use strict"; }"#);
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA503");
    }

    /// Allow non-simple function parameters with strict directive prologues in Destack.
    #[test]
    fn test_allow_function_non_simple_parameters_with_use_strict_in_destack() {
        let test = TestProgram::memory_sequential();

        // source: function a([]){ "use strict"; }
        let module_id = test.add_module("test.ds", r#"function a([]){ "use strict"; }"#);
        test.analyze_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EA503");
    }

    /// Reject non-simple object method parameters with strict directive prologues in JS/TS.
    #[test]
    fn test_reject_object_method_non_simple_parameters_with_use_strict() {
        let test = TestProgram::memory_sequential();

        // source: ({ a([]){ "use strict"; } });
        let module_id = test.add_module("test.js", r#"({ a([]){ "use strict"; } });"#);
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA503");
    }

    /// Reject non-simple object method parameters with strict directives without semicolons.
    #[test]
    fn test_reject_object_method_non_simple_parameters_with_use_strict_no_semicolon() {
        let test = TestProgram::memory_sequential();

        // source: ({a([]){'use strict'}})
        let module_id = test.add_module("test.js", r#"({a([]){'use strict'}})"#);
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA503");
    }

    /// Allow non-simple object method parameters with strict directive prologues in Destack.
    #[test]
    fn test_allow_object_method_non_simple_parameters_with_use_strict_in_destack() {
        let test = TestProgram::memory_sequential();

        // source: ({ a([]){ "use strict"; } });
        let module_id = test.add_module("test.ds", r#"({ a([]){ "use strict"; } });"#);
        test.analyze_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EA503");
    }

    /// Reject object literal fields as arrow binding parameters in JavaScript.
    #[test]
    fn test_reject_arrow_object_literal_parameter_javascript() {
        let test = TestProgram::memory_sequential();

        // source: ({ 5 }) => {}
        let module_id = test.add_module("test.js", "({ 5 }) => {}");
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA226");
    }

    /// Reject array literal elements as arrow binding parameters in JavaScript.
    #[test]
    fn test_reject_arrow_array_literal_parameter_javascript() {
        let test = TestProgram::memory_sequential();

        // source: ([ 5 ]) => {}
        let module_id = test.add_module("test.js", "([ 5 ]) => {}");
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA226");
    }

    /// Reject yield expressions in generator parameter initializers.
    #[test]
    fn test_reject_yield_in_generator_parameter_initializer() {
        let test = TestProgram::memory_sequential();

        // source: function* a(){ function* b(c = yield d){} }
        let module_id = test.add_module("test.js", "function* a(){ function* b(c = yield d){} }");
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA303");
    }

    /// Reject delegated yield expressions in generator parameter initializers.
    #[test]
    fn test_reject_yield_star_in_generator_parameter_initializer() {
        let test = TestProgram::memory_sequential();

        // source: function* a(){ function* b(c = yield* d){} }
        let module_id = test.add_module("test.js", "function* a(){ function* b(c = yield* d){} }");
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA303");
    }

    /// Reject bare yield expressions in generator object parameter defaults.
    #[test]
    fn test_reject_bare_yield_in_generator_object_parameter_default() {
        let test = TestProgram::memory_sequential();

        // source: function* a(){ function* b({c = yield}){} }
        let module_id = test.add_module("test.js", "function* a(){ function* b({c = yield}){} }");
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA303");
    }

    /// Reject bare yield expressions in generator method object parameter defaults.
    #[test]
    fn test_reject_bare_yield_in_generator_method_object_parameter_default() {
        let test = TestProgram::memory_sequential();

        // source: function* a(){ ({ *b({c = yield}){} }); }
        let module_id = test.add_module("test.js", "function* a(){ ({ *b({c = yield}){} }); }");
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA303");
    }

    // reject infer declarations outside conditional type extends clauses
    #[test]
    fn test_reject_infer_outside_conditional_type() {
        let test = TestProgram::memory_sequential();

        // source: type Invalid = infer U;
        let module_id = test.add_module("test.ts", "type Invalid = infer U;");
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA126");
    }

    // allow infer declarations inside conditional type extends clauses
    #[test]
    fn test_allow_infer_inside_conditional_type() {
        let test = TestProgram::memory_sequential();

        // source: type Valid<T> = T extends infer U ? U : never;
        let module_id =
            test.add_module("test.ts", "type Valid<T> = T extends infer U ? U : never;");
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EA126");
    }
}
