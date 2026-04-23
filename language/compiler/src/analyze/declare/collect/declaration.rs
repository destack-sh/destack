use destack_dir::{
    Asynchrony, Block, Declaration, Expression, Extension, ExtensionKind, FunctionCardinality,
    FunctionKind, FunctionMode, FunctionSignature, GenericParameter, GlobalSymbolId, Lineage,
    LocalNodeId, LocalNodeIdAny, LocalSymbolId, LocalTypeId, Member, NodeTree, NodeVisitor,
    NodeVisitorOptions, Parameter, StaticArgument, StaticExpression, StaticKey, SymbolType,
    TupleElement, Type, TypeExpression, TypeField, TypeIndexSignature, TypeLiteral, TypeMember,
    TypeTable, walk_block, walk_declaration, walk_expression,
};
use destack_workspace::{Module, ProfileId};
use std::collections::{HashMap, HashSet};

use crate::{AnalyzeError, AnalyzeResult, Compiler, CompilerContext};

use crate::analyze::common::{
    CanonicalSymbolMode, NormalizationMode, ObjectShape, ObjectShapeSet, TypeContext,
};

/// Visitor used to declare type-level constructs across a module.
#[derive(Debug)]
struct CollectVisitor<'a> {
    /// The compiler shared state.
    compiler: &'a Compiler,
    /// The shared type ctx context for declaration collection.
    ctx: TypeContext<'a>,
    /// Track the first error encountered while walking.
    result: AnalyzeResult<()>,
    /// Node visitor options (unused, but required by trait).
    options: NodeVisitorOptions,
}

impl<'a> CollectVisitor<'a> {
    /// Create a new declare visitor.
    fn new(compiler: &'a Compiler, ctx: TypeContext<'a>) -> Self {
        // build the visitor state
        Self {
            compiler,
            ctx,
            result: Ok(()),
            options: NodeVisitorOptions::default(),
        }
    }

    /// Check if this visitor should continue.
    fn should_continue(&self) -> bool {
        // stop once an error is recorded
        self.result.is_ok()
    }

    /// Record a declare result, preserving the first error.
    fn record_result(&mut self, result: AnalyzeResult<()>) {
        // keep the first error in the visitor
        if self.result.is_ok() && result.is_err() {
            self.result = result;
        }
    }

    /// Finish the walk and return the result.
    fn finish(self) -> AnalyzeResult<()> {
        // return the recorded result
        self.result
    }
}

impl NodeVisitor for CollectVisitor<'_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        // stop on first error
        if !self.should_continue() {
            return;
        }

        // walk nested expression nodes
        destack_core::ensure_sufficient_stack(|| {
            walk_expression(self, tree, id, expression);
        });
    }

    fn visit_block(&mut self, tree: &NodeTree, id: LocalNodeId<Block>, block: &Block) {
        // stop on first error
        if !self.should_continue() {
            return;
        }

        // walk nested block nodes
        walk_block(self, tree, id, block);
    }

    fn visit_declaration(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Declaration>,
        declaration: &Declaration,
    ) {
        // stop on first error
        if !self.should_continue() {
            return;
        }

        // declare this declaration before walking nested nodes
        let result = self
            .compiler
            .collect_declaration(&mut self.ctx.reborrow(), id);
        self.record_result(result);

        // stop on error after declaration work
        if !self.should_continue() {
            return;
        }

        // walk nested declaration nodes
        walk_declaration(self, tree, id, declaration);
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Return whether alias recursion should be checked later than declaration collection.
    fn defer_alias_cycle_check(expression: &TypeExpression) -> bool {
        matches!(expression, TypeExpression::Conditional { .. })
    }

    /// Report one recursive alias error and poison the declared type slot.
    fn report_recursive_declared_alias_error(
        &self,
        ctx: &mut TypeContext<'_>,
        value_expression_id: LocalNodeId<TypeExpression>,
        declared_ty_id: LocalTypeId,
    ) {
        let node = value_expression_id
            .into_global_any(ctx.module.id)
            .into_anchored(Some(ctx.profile));
        self.error(AnalyzeError::RecursiveTypeInstantiation { node });
        ctx.types.update_type(declared_ty_id, Type::Error);
    }

    /// Report one declared alias recursion error when the resolved target closes a cycle.
    pub(crate) fn report_declared_alias_cycle_if_any(
        &self,
        ctx: &mut TypeContext<'_>,
        symbol: GlobalSymbolId,
        value_expression_id: LocalNodeId<TypeExpression>,
        declared_ty_id: LocalTypeId,
    ) -> AnalyzeResult<()> {
        let value_expression = ctx.tree.get(value_expression_id);
        if Self::defer_alias_cycle_check(value_expression) {
            return Ok(());
        }

        // catch direct alias forwarding cycles from the resolved symbol graph
        {
            let view = ctx.module_symbol_view();
            let root_symbol = self
                .declaration_symbol_id_for_artifact(
                    view,
                    symbol,
                    destack_artifact::ArtifactKey::dir_resolved,
                )
                .map_err(AnalyzeError::from)?
                .unwrap_or(self.canonical_symbol_id_for_artifact(
                    view,
                    symbol,
                    CanonicalSymbolMode::PreserveAliases,
                    destack_artifact::ArtifactKey::dir_resolved,
                )?);
            let mut current_symbol = root_symbol;
            let mut visited = HashSet::new();

            loop {
                if !visited.insert(current_symbol) {
                    self.report_recursive_declared_alias_error(
                        ctx,
                        value_expression_id,
                        declared_ty_id,
                    );
                    return Ok(());
                }

                let next_symbol = self
                    .with_module_symbols_or_local_for_artifact(
                        ctx.compiler_context,
                        ctx.module,
                        ctx.profile,
                        current_symbol.module_id,
                        ctx.symbols,
                        destack_artifact::ArtifactKey::dir_resolved,
                        |_owner_module, owner_symbols| {
                            let symbol_entry = owner_symbols.get_symbol(current_symbol.local_id);
                            if !matches!(
                                symbol_entry.ty,
                                SymbolType::TypeAlias | SymbolType::Newtype
                            ) {
                                return None;
                            }

                            symbol_entry.target_symbol.or(symbol_entry.canonical_symbol)
                        },
                    )
                    .map_err(AnalyzeError::from)?;
                let Some(next_symbol) = next_symbol else {
                    break;
                };

                let next_symbol = self
                    .declaration_symbol_id_for_artifact(
                        view,
                        next_symbol,
                        destack_artifact::ArtifactKey::dir_resolved,
                    )
                    .map_err(AnalyzeError::from)?
                    .unwrap_or(self.canonical_symbol_id_for_artifact(
                        view,
                        next_symbol,
                        CanonicalSymbolMode::PreserveAliases,
                        destack_artifact::ArtifactKey::dir_resolved,
                    )?);
                if next_symbol == root_symbol {
                    self.report_recursive_declared_alias_error(
                        ctx,
                        value_expression_id,
                        declared_ty_id,
                    );
                    return Ok(());
                }

                current_symbol = next_symbol;
            }
        }

        if matches!(ctx.types.get_type(declared_ty_id), Type::Unevaluated(_)) {
            self.resolve_declared_type(&mut ctx.reborrow(), declared_ty_id)?;
        }

        if matches!(ctx.types.get_type(declared_ty_id), Type::Error) {
            return Ok(());
        }

        if matches!(ctx.types.get_type(declared_ty_id), Type::Mapped { .. }) {
            let contains_recursive_alias_reference = {
                let mut visited = HashSet::new();
                self.type_contains_reference_symbol(declared_ty_id, symbol, ctx.types, &mut visited)
            };
            if contains_recursive_alias_reference {
                self.report_recursive_declared_alias_error(
                    ctx,
                    value_expression_id,
                    declared_ty_id,
                );
                return Ok(());
            }
        }

        // force alias expansion once here so cross module cycles are checked
        let normalized_ty_id = self.normalize_type(
            &mut ctx.reborrow(),
            declared_ty_id,
            NormalizationMode::Assign,
        );
        if matches!(ctx.types.get_type(normalized_ty_id), Type::Error) {
            ctx.types.update_type(declared_ty_id, Type::Error);
            return Ok(());
        }

        let contains_recursive_alias_reference = {
            let mut visited = HashSet::new();
            self.type_contains_reference_symbol(normalized_ty_id, symbol, ctx.types, &mut visited)
        };
        if contains_recursive_alias_reference {
            self.report_recursive_declared_alias_error(ctx, value_expression_id, declared_ty_id);
            return Ok(());
        }

        Ok(())
    }

    /// Decide whether declared types should be deferred for a module.
    pub(crate) fn should_defer_declaration_types(
        &self,
        context: &CompilerContext<'_>,
        module: &Module,
    ) -> bool {
        if module.is_builtin() {
            return true;
        }

        if !module.language_type.is_declaration() {
            return false;
        }

        let module_checks = context.module_check_options_for_module(module.id);
        module_checks.skip_lib_check
    }

    /// Resolve or defer a type expression into a type id.
    pub(crate) fn collect_or_defer_type_expression(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
        defer_type_evaluation: bool,
    ) -> AnalyzeResult<LocalTypeId> {
        if !defer_type_evaluation {
            return self.resolve_declared_type_expression(
                &mut ctx.reborrow(),
                expression_id,
                true,
                true,
            );
        }

        let global_id = expression_id.into_global_any(ctx.module.id);
        if let Some(existing) = ctx.types.get_declared_type_id(global_id) {
            return Ok(existing);
        }

        // keep one stable declared slot even when deferred collection still depends on later work
        let ty = self
            .query_declared_type_expression_value(
                &mut ctx.reborrow(),
                expression_id,
                true,
                true,
                false,
            )?
            .unwrap_or(Type::Unevaluated(expression_id));
        let ty_id = ctx.types.insert_type_from(ty, expression_id);
        ctx.types.set_declared_type(global_id, ty_id);
        Ok(ty_id)
    }

    /// Check whether TypeScript overload implementations must be restricted.
    fn should_enforce_single_overload(&self, module: &Module) -> bool {
        module.language_type.is_typescript() && !module.language_type.is_declaration()
    }

    /// Report an overload implementation error for a node.
    fn report_overload_implementation_error(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        is_constructor: bool,
    ) {
        let node = node_id.into_global(module.id).into_anchored(Some(profile));
        if is_constructor {
            self.error(AnalyzeError::MultipleConstructorImplementations { node });
        } else {
            self.error(AnalyzeError::MultipleOverloadImplementations { node });
        }
    }

    /// Return one declaration symbol as a typed global symbol id.
    fn declaration_symbol(&self, ctx: &TypeContext<'_>, symbol: LocalSymbolId) -> GlobalSymbolId {
        let symbol_entry = ctx.symbols.get_symbol(symbol);
        let typed_symbol = symbol.with_type(symbol_entry.ty);

        GlobalSymbolId::new(ctx.module.id, typed_symbol)
    }

    /// Declare all declarations reachable from the module roots.
    pub(crate) fn collect_module_declarations(
        &self,
        ctx: &mut TypeContext<'_>,
        roots: &[LocalNodeId<Expression>],
    ) -> AnalyzeResult<()> {
        // prepare the declaration visitor
        let mut visitor = CollectVisitor::new(self, ctx.reborrow());

        // walk each root expression to visit all declarations
        for root_id in roots.iter().copied() {
            let root = visitor.ctx.tree.get(root_id);
            // visit the root expression
            visitor.visit_expression(visitor.ctx.tree, root_id, root);

            // stop early on errors
            if !visitor.should_continue() {
                break;
            }
        }

        // return the visitor result
        visitor.finish()
    }

    /// Declare a single declaration node.
    pub(super) fn collect_declaration(
        &self,
        ctx: &mut TypeContext<'_>,
        declaration_id: LocalNodeId<Declaration>,
    ) -> AnalyzeResult<()> {
        // load the declaration node
        let declaration = ctx.tree.get(declaration_id);

        // dispatch by declaration kind
        match declaration {
            Declaration::Global(_) => Ok(()),
            Declaration::Namespace(declaration) => {
                // declare namespace generics
                self.collect_generics(ctx, &declaration.generic_parameters)?;

                Ok(())
            }
            Declaration::Type(declaration) => {
                // declare generic parameters
                for generic_parameter_id in &declaration.generic_parameters {
                    self.collect_generic_parameter(&mut ctx.reborrow(), *generic_parameter_id)?;
                }

                // validate comptime usage for value generic defaults
                let has_static_parameters =
                    declaration
                        .generic_parameters
                        .iter()
                        .any(|generic_parameter_id| {
                            matches!(
                                ctx.tree.get(*generic_parameter_id),
                                GenericParameter::Value { .. }
                            )
                        });

                // validate comptime usage when static parameter dependencies are ready
                if has_static_parameters {
                    self.query_static_value_parameter_usage(
                        &mut ctx.reborrow(),
                        declaration.value,
                        false,
                        true,
                    )?;
                }

                let symbol = self.declaration_symbol(ctx, declaration.symbol);
                let declared_type_global_id = declaration.value.into_global_any(ctx.module.id);
                let declared_ty_id = if let Some(existing) =
                    ctx.types.get_declared_type_id(declared_type_global_id)
                {
                    existing
                } else {
                    let declared_ty_id = ctx.types.insert_type_from_any(
                        Type::Unevaluated(declaration.value),
                        declaration.value.into_any(),
                    );
                    ctx.types
                        .set_declared_type(declared_type_global_id, declared_ty_id);
                    declared_ty_id
                };

                // alias declarations always publish the declared target slot
                ctx.types.set_alias_target_type_id(symbol, declared_ty_id);

                let instance_ty_id = if declaration.is_nominal {
                    {
                        let ty = Type::Reference {
                            symbol,
                            generic_arguments: None,
                        };
                        ctx.types.insert_type_from(ty, declaration_id)
                    }
                } else {
                    declared_ty_id
                };
                ctx.types.set_instance_type(symbol, instance_ty_id);

                // register the value type for this symbol
                if declaration.is_nominal {
                    let generic_parameters = self.generic_parameter_placeholders_for_declaration(
                        &mut ctx.reborrow(),
                        Some(&declaration.generic_parameters),
                    );
                    let constructor_id = self.newtype_constructor_signature(
                        &mut ctx.reborrow(),
                        declaration_id,
                        declaration.value,
                        declared_ty_id,
                        instance_ty_id,
                        generic_parameters,
                    )?;
                    let mut shape = ObjectShape::default();
                    shape.call_signatures.push(constructor_id);
                    self.merge_value_shape_into_symbol(
                        &mut ctx.reborrow(),
                        declaration_id,
                        declaration.symbol,
                        symbol,
                        &shape,
                        false,
                    );
                } else if ctx.module.language_type.is_destack() {
                    let value_ty = Type::Value {
                        value: instance_ty_id,
                    };
                    let value_ty_id = ctx.types.insert_type_from(value_ty, declaration_id);
                    ctx.types.set_value_type(symbol, value_ty_id);
                }

                Ok(())
            }
            Declaration::ImportAlias(_) => Ok(()),
            Declaration::Struct(declaration) => {
                // resolve declaration merge state
                let symbol_entry = ctx.symbols.get_symbol(declaration.symbol);
                let allow_merge = ctx.module.language_type.is_declaration();
                let is_primary = symbol_entry.primary_declaration.is_some_and(|primary| {
                    primary == declaration_id.into_global_any(ctx.module.id)
                });

                // declare generics and heritage
                self.collect_generics(&mut ctx.reborrow(), &declaration.generic_parameters)?;
                self.collect_lineage(
                    &mut ctx.reborrow(),
                    None,
                    &[],
                    &declaration.implements_types,
                    &declaration.embedded_types,
                    Some(declaration.symbol),
                )?;
                let declaration_symbol = self.declaration_symbol(ctx, declaration.symbol);
                self.report_missing_associated_requirements(
                    &mut ctx.reborrow(),
                    declaration_symbol,
                    &[],
                    &declaration.implements_types,
                    &declaration.members,
                    false,
                )?;

                // nominal reference for constructors
                let symbol = self.declaration_symbol(ctx, declaration.symbol);
                let generic_arguments = self.self_type_static_arguments_for_declaration(
                    &mut ctx.reborrow(),
                    Some(&declaration.generic_parameters),
                );
                let nominal_reference = Type::Reference {
                    symbol,
                    generic_arguments,
                };
                let nominal_reference_id = ctx
                    .types
                    .insert_type_from(nominal_reference, declaration_id);

                // build instance and value shapes from members
                let shapes = self.collect_member_shapes(
                    &mut ctx.reborrow(),
                    &declaration.members,
                    Some(&declaration.generic_parameters),
                    Some(nominal_reference_id),
                )?;
                let instance_shape = shapes.instance;
                let mut value_shape = shapes.value;

                // merge instance shapes for merged declarations
                self.merge_instance_shape_into_merge_group(
                    &mut ctx.reborrow(),
                    declaration_id,
                    declaration.symbol,
                    &instance_shape,
                    allow_merge,
                );

                // merge global augmentations once per primary declaration
                if allow_merge && is_primary {
                    self.merge_global_instance_shape_for_symbol(
                        &mut ctx.reborrow(),
                        declaration_id,
                        declaration.symbol,
                    )?;
                }

                // ensure constructors exist for the value shape
                self.ensure_constructor_signatures(
                    &mut ctx.reborrow(),
                    declaration_id,
                    symbol,
                    nominal_reference_id,
                    &mut value_shape,
                )?;

                // register the nominal value type with static members
                self.merge_value_shape_into_symbol(
                    &mut ctx.reborrow(),
                    declaration_id,
                    declaration.symbol,
                    symbol,
                    &value_shape,
                    allow_merge,
                );

                // merge global augmentations for the value shape
                if allow_merge && is_primary {
                    self.merge_global_value_shape_for_symbol(
                        &mut ctx.reborrow(),
                        declaration_id,
                        declaration.symbol,
                    )?;
                }

                Ok(())
            }
            Declaration::Class(declaration) => {
                // resolve declaration merge state
                let symbol_entry = ctx.symbols.get_symbol(declaration.symbol);
                let allow_merge = ctx.module.language_type.supports_declaration_merging()
                    || symbol_entry.origin.is_global_augmentation();
                let is_primary = symbol_entry.primary_declaration.is_some_and(|primary| {
                    primary == declaration_id.into_global_any(ctx.module.id)
                });

                // declare generics and heritage
                self.collect_generics(&mut ctx.reborrow(), &declaration.generic_parameters)?;
                self.collect_lineage(
                    &mut ctx.reborrow(),
                    declaration.extends_expression,
                    &[],
                    &declaration.implements_types,
                    &[],
                    Some(declaration.symbol),
                )?;
                let declaration_symbol = self.declaration_symbol(ctx, declaration.symbol);
                let allows_deferred_associated = declaration.is_abstract;
                self.report_missing_associated_requirements(
                    &mut ctx.reborrow(),
                    declaration_symbol,
                    &[],
                    &declaration.implements_types,
                    &declaration.members,
                    allows_deferred_associated,
                )?;

                // prepare nominal reference for constructors
                let symbol = self.declaration_symbol(ctx, declaration.symbol);
                let generic_arguments = self.self_type_static_arguments_for_declaration(
                    &mut ctx.reborrow(),
                    Some(&declaration.generic_parameters),
                );
                let nominal_reference = Type::Reference {
                    symbol,
                    generic_arguments,
                };
                let nominal_reference_id = ctx
                    .types
                    .insert_type_from(nominal_reference, declaration_id);

                // build instance and value shapes from members
                let shapes = self.collect_member_shapes(
                    &mut ctx.reborrow(),
                    &declaration.members,
                    Some(&declaration.generic_parameters),
                    Some(nominal_reference_id),
                )?;
                let instance_shape = shapes.instance;
                let mut value_shape = shapes.value;

                // merge instance shapes for merged declarations
                self.merge_instance_shape_into_merge_group(
                    &mut ctx.reborrow(),
                    declaration_id,
                    declaration.symbol,
                    &instance_shape,
                    allow_merge,
                );

                // merge global augmentations once per primary declaration
                if allow_merge && is_primary {
                    self.merge_global_instance_shape_for_symbol(
                        &mut ctx.reborrow(),
                        declaration_id,
                        declaration.symbol,
                    )?;
                }

                // ensure constructors exist for the value shape
                self.ensure_constructor_signatures(
                    &mut ctx.reborrow(),
                    declaration_id,
                    symbol,
                    nominal_reference_id,
                    &mut value_shape,
                )?;

                // register the nominal value type with static members
                self.merge_value_shape_into_symbol(
                    &mut ctx.reborrow(),
                    declaration_id,
                    declaration.symbol,
                    symbol,
                    &value_shape,
                    allow_merge,
                );

                // merge global augmentations for the value shape
                if allow_merge && is_primary {
                    self.merge_global_value_shape_for_symbol(
                        &mut ctx.reborrow(),
                        declaration_id,
                        declaration.symbol,
                    )?;
                }

                Ok(())
            }
            Declaration::Enum(declaration) => {
                // resolve declaration merge state
                let symbol_entry = ctx.symbols.get_symbol(declaration.symbol);
                let allow_merge = ctx.module.language_type.supports_declaration_merging()
                    || symbol_entry.origin.is_global_augmentation();
                let is_primary = symbol_entry.primary_declaration.is_some_and(|primary| {
                    primary == declaration_id.into_global_any(ctx.module.id)
                });

                // declare generics and heritage
                self.collect_generics(&mut ctx.reborrow(), &declaration.generic_parameters)?;
                self.collect_lineage(
                    &mut ctx.reborrow(),
                    None,
                    &[],
                    &declaration.implements_types,
                    &[],
                    Some(declaration.symbol),
                )?;
                let declaration_symbol = self.declaration_symbol(ctx, declaration.symbol);
                self.report_missing_associated_requirements(
                    &mut ctx.reborrow(),
                    declaration_symbol,
                    &[],
                    &declaration.implements_types,
                    &declaration.members,
                    false,
                )?;

                // prepare the nominal reference for enum values
                let symbol = self.declaration_symbol(ctx, declaration.symbol);
                let generic_arguments = self.self_type_static_arguments_for_declaration(
                    &mut ctx.reborrow(),
                    Some(&declaration.generic_parameters),
                );
                let nominal_reference = Type::Reference {
                    symbol,
                    generic_arguments,
                };
                let nominal_reference_id = ctx
                    .types
                    .insert_type_from(nominal_reference, declaration_id);

                // build instance and value shapes from members
                let shapes = self.collect_member_shapes(
                    &mut ctx.reborrow(),
                    &declaration.members,
                    Some(&declaration.generic_parameters),
                    Some(nominal_reference_id),
                )?;
                let instance_shape = shapes.instance;
                let mut value_shape = shapes.value;

                // merge instance shapes for merged declarations
                self.merge_instance_shape_into_merge_group(
                    &mut ctx.reborrow(),
                    declaration_id,
                    declaration.symbol,
                    &instance_shape,
                    allow_merge,
                );

                // merge global augmentations once per primary declaration
                if allow_merge && is_primary {
                    self.merge_global_instance_shape_for_symbol(
                        &mut ctx.reborrow(),
                        declaration_id,
                        declaration.symbol,
                    )?;
                }

                // build value fields for enum members
                for field_id in &declaration.fields {
                    let field = ctx.tree.get(*field_id);
                    let Some(field_symbol) = self.query_enum_field_symbol_for_name(
                        ctx.type_view(),
                        declaration.symbol.into_global(ctx.module.id),
                        field.name.string(),
                    ) else {
                        continue;
                    };
                    ctx.types.set_value_type(field_symbol, nominal_reference_id);

                    value_shape.fields.push(TypeField {
                        key: StaticKey::Name(field.name.string()),
                        ty: nominal_reference_id,
                        is_optional: false,
                        is_readonly: true,
                    });
                }

                // register the enum value type
                self.merge_value_shape_into_symbol(
                    &mut ctx.reborrow(),
                    declaration_id,
                    declaration.symbol,
                    symbol,
                    &value_shape,
                    allow_merge,
                );

                // merge global augmentations for the value shape
                if allow_merge && is_primary {
                    self.merge_global_value_shape_for_symbol(
                        &mut ctx.reborrow(),
                        declaration_id,
                        declaration.symbol,
                    )?;
                }

                Ok(())
            }
            Declaration::Interface(declaration) => {
                // resolve declaration merge state
                let symbol_entry = ctx.symbols.get_symbol(declaration.symbol);
                let allow_merge = ctx.module.language_type.supports_declaration_merging()
                    || symbol_entry.origin.is_global_augmentation();
                let is_primary = symbol_entry.primary_declaration.is_some_and(|primary| {
                    primary == declaration_id.into_global_any(ctx.module.id)
                });

                // declare generics and heritage
                self.collect_generics(&mut ctx.reborrow(), &declaration.generic_parameters)?;
                self.collect_lineage(
                    &mut ctx.reborrow(),
                    None,
                    &declaration.extends_types,
                    &[],
                    &[],
                    Some(declaration.symbol),
                )?;

                // build instance shape from members
                let shape = self.collect_type_member_shape(
                    &mut ctx.reborrow(),
                    &declaration.members,
                    Some(&declaration.generic_parameters),
                )?;

                // merge instance shapes for merged declarations
                self.merge_instance_shape_into_merge_group(
                    &mut ctx.reborrow(),
                    declaration_id,
                    declaration.symbol,
                    &shape,
                    allow_merge,
                );

                // merge global augmentations once per primary declaration
                if allow_merge && is_primary {
                    self.merge_global_instance_shape_for_symbol(
                        &mut ctx.reborrow(),
                        declaration_id,
                        declaration.symbol,
                    )?;
                }

                // register the nominal type as the value type
                let nominal_ty = Type::Reference {
                    symbol: self.declaration_symbol(ctx, declaration.symbol),
                    generic_arguments: None,
                };
                let nominal_ty_id = ctx.types.insert_type_from(nominal_ty, declaration_id);
                let value_ty = Type::Value {
                    value: nominal_ty_id,
                };
                let value_ty_id = ctx.types.insert_type_from(value_ty, declaration_id);
                let symbol = self.declaration_symbol(ctx, declaration.symbol);
                ctx.types.set_value_type(symbol, value_ty_id);

                Ok(())
            }
            Declaration::Function(declaration) => {
                // decide whether to defer declared types
                let defer_type_evaluation =
                    self.should_defer_declaration_types(ctx.compiler_context, ctx.module);

                // resolve declaration merge state
                let symbol_entry = ctx.symbols.get_symbol(declaration.symbol);
                let allow_merge = ctx.module.language_type.supports_declaration_merging()
                    || ctx.module.language_type.is_destack()
                    || symbol_entry.origin.is_global_augmentation();

                // enforce single implementation for TypeScript overloads
                if self.should_enforce_single_overload(ctx.module) && declaration.body.is_some() {
                    let mut implementation_count = 0;
                    let mut declaration_nodes = Vec::new();
                    if let Some(primary) = symbol_entry.primary_declaration {
                        declaration_nodes.push(primary);
                    }
                    if let Some(secondary) = symbol_entry.secondary_declarations.as_ref() {
                        declaration_nodes.extend(secondary.iter().copied());
                    }

                    for declaration_id in declaration_nodes {
                        if declaration_id.module_id != ctx.module.id {
                            continue;
                        }
                        let Ok(declaration_id) =
                            declaration_id.local_id.try_into_typed::<Declaration>()
                        else {
                            continue;
                        };
                        if let Declaration::Function(declaration) = ctx.tree.get(declaration_id)
                            && declaration.body.is_some()
                        {
                            implementation_count += 1;
                            if implementation_count > 1 {
                                self.report_overload_implementation_error(
                                    ctx.module,
                                    ctx.profile,
                                    declaration_id.into_any(),
                                    false,
                                );
                                break;
                            }
                        }
                    }
                }

                // evaluate the function signature
                self.collect_generics(
                    &mut ctx.reborrow(),
                    &declaration.signature.generic_parameters,
                )?;
                let previous_signature_id = ctx
                    .types
                    .get_signature_type_for_node(declaration_id.into_global_any(ctx.module.id));
                let ty = self.resolve_declared_function_signature_type(
                    ctx,
                    &declaration.signature,
                    declaration_id.into_any(),
                    defer_type_evaluation,
                )?;
                let fn_ty_id = ctx
                    .types
                    .insert_type_from_any(ty, declaration_id.into_any());

                // record the declared signature for inference
                ctx.types.set_signature_type_for_node(
                    declaration_id.into_global_any(ctx.module.id),
                    fn_ty_id,
                );

                // merge into callable instance shape
                let mut shape = ObjectShape::default();
                shape.push_call_signature(fn_ty_id);
                self.merge_instance_shape_into_merge_group(
                    &mut ctx.reborrow(),
                    declaration_id,
                    declaration.symbol,
                    &shape,
                    allow_merge,
                );

                // merge the function into the value type
                self.merge_function_value_type(
                    &mut ctx.reborrow(),
                    declaration_id,
                    declaration.symbol,
                    fn_ty_id,
                    previous_signature_id,
                    allow_merge,
                );

                Ok(())
            }
            Declaration::Extension(declaration) => {
                // decide whether to defer declared types
                let defer_type_evaluation =
                    self.should_defer_declaration_types(ctx.compiler_context, ctx.module);

                // declare generics and heritage
                self.collect_generics(&mut ctx.reborrow(), &declaration.generic_parameters)?;
                if !defer_type_evaluation {
                    self.resolve_declared_type_expression(
                        &mut ctx.reborrow(),
                        declaration.target_type,
                        true,
                        true,
                    )?;
                }
                let extension_symbol = self.declaration_symbol(ctx, declaration.symbol);
                self.collect_lineage(
                    &mut ctx.reborrow(),
                    None,
                    &[],
                    &declaration.implements_types,
                    &[],
                    Some(declaration.symbol),
                )?;

                // skip already declared extensions for this symbol
                if ctx
                    .types
                    .get_extension_id_for_symbol(extension_symbol)
                    .is_some()
                {
                    return Ok(());
                }

                // build the extension instance shape
                let shape = self.collect_member_shape(
                    &mut ctx.reborrow(),
                    &declaration.members,
                    Some(&declaration.generic_parameters),
                )?;
                let instance_ty = shape.into_object_type();
                let instance_ty_id = ctx.types.insert_type_from(instance_ty, declaration_id);
                ctx.types
                    .set_instance_type(extension_symbol, instance_ty_id);

                // register the extension when a target symbol exists
                if let Some(target) = declaration.target_symbol {
                    // resolve the canonical target symbol for extension lookup
                    let canonical_target = self.canonical_symbol_id(
                        ctx.module_symbol_view(),
                        target,
                        CanonicalSymbolMode::FollowAliases,
                    );
                    let kind = if ctx.module.id == canonical_target.module_id {
                        ExtensionKind::Inherent
                    } else if declaration.name.is_some() {
                        ExtensionKind::Nominal
                    } else {
                        ExtensionKind::Local
                    };
                    let lineage = ctx.types.get_lineage_id_for_symbol(extension_symbol);
                    let extension =
                        Extension::new(extension_symbol, kind, canonical_target, lineage);
                    ctx.types.insert_extension(extension);
                }

                Ok(())
            }
        }
    }

    /// Declare generic parameters by evaluating their declared types in one ctx context.
    pub(crate) fn collect_generics(
        &self,
        ctx: &mut TypeContext<'_>,
        generic_parameters: &[LocalNodeId<GenericParameter>],
    ) -> AnalyzeResult<()> {
        // defer generic constraint evaluation for declaration modules
        if self.should_defer_declaration_types(ctx.compiler_context, ctx.module) {
            return Ok(());
        }

        // evaluate generic parameter constraints and declared types
        for generic_parameter_id in generic_parameters {
            self.collect_generic_parameter(&mut ctx.reborrow(), *generic_parameter_id)?;
        }

        Ok(())
    }

    /// Declare a generic parameter by evaluating its declared type.
    fn collect_generic_parameter(
        &self,
        ctx: &mut TypeContext<'_>,
        generic_parameter_id: LocalNodeId<GenericParameter>,
    ) -> AnalyzeResult<()> {
        // defer parameter evaluation for declaration modules
        if self.should_defer_declaration_types(ctx.compiler_context, ctx.module) {
            return Ok(());
        }

        // reject explicit comptime wrappers in value generic defaults
        let generic_parameter = ctx.tree.get(generic_parameter_id);
        if let GenericParameter::Value {
            default: Some(default_expression),
            is_comptime: true,
            ..
        } = generic_parameter
        {
            {
                let mut expression_id =
                    self.unwrap_parenthesized_expression(*default_expression, ctx.tree);
                let mut is_explicit_comptime = false;

                loop {
                    match ctx.tree.get(expression_id) {
                        Expression::Comptime { body } => {
                            is_explicit_comptime = true;
                            expression_id = self.unwrap_parenthesized_expression(*body, ctx.tree);
                        }
                        _ => break,
                    }
                }

                if is_explicit_comptime {
                    self.error(AnalyzeError::NonStaticArgument {
                        node: default_expression
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile)),
                    });
                }
            }
        }

        // resolve the declared type for the generic parameter
        let declared_type_id = ctx
            .types
            .get_declared_type_id(generic_parameter_id.into_global_any(ctx.module.id));
        let Some(declared_type_id) = declared_type_id else {
            return Ok(());
        };

        // evaluate the declared type
        self.resolve_declared_type(&mut ctx.reborrow(), declared_type_id)?;

        Ok(())
    }

    /// Declare one lineage from declaration type lists.
    fn collect_lineage(
        &self,
        ctx: &mut TypeContext<'_>,
        extends_expression: Option<LocalNodeId<Expression>>,
        extends_types: &[LocalNodeId<TypeExpression>],
        implements_types: &[LocalNodeId<TypeExpression>],
        embedded_types: &[LocalNodeId<TypeExpression>],
        symbol: Option<LocalSymbolId>,
    ) -> AnalyzeResult<()> {
        // defer heritage evaluation for declaration modules
        let defer_type_evaluation =
            self.should_defer_declaration_types(ctx.compiler_context, ctx.module);

        // resolve class extends symbol
        let mut extends_symbols = Vec::new();
        if let Some(expression_id) = extends_expression {
            let target_symbol = ctx.tree.get(expression_id).target_symbol();
            if let Some(target_symbol) = target_symbol {
                let canonical_symbol = self.canonical_symbol_id(
                    ctx.module_symbol_view(),
                    target_symbol,
                    CanonicalSymbolMode::FollowAliases,
                );
                extends_symbols.push(canonical_symbol);
            }
        }

        // resolve type extends symbols
        for expression_id in extends_types {
            if let Some(target_symbol) = self.collect_heritage_symbol(
                &mut ctx.reborrow(),
                *expression_id,
                defer_type_evaluation,
            )? {
                let canonical_symbol = self.canonical_symbol_id(
                    ctx.module_symbol_view(),
                    target_symbol,
                    CanonicalSymbolMode::FollowAliases,
                );
                extends_symbols.push(canonical_symbol);
            }
        }

        // resolve implements symbols
        let mut implements_symbols = Vec::new();
        for expression_id in implements_types {
            if let Some(target_symbol) = self.collect_heritage_symbol(
                &mut ctx.reborrow(),
                *expression_id,
                defer_type_evaluation,
            )? {
                let canonical_symbol = self.canonical_symbol_id(
                    ctx.module_symbol_view(),
                    target_symbol,
                    CanonicalSymbolMode::FollowAliases,
                );
                implements_symbols.push(canonical_symbol);
            }
        }

        // resolve embedded symbols
        let mut embedded_symbols = Vec::new();
        for expression_id in embedded_types {
            if let Some(target_symbol) = self.collect_heritage_symbol(
                &mut ctx.reborrow(),
                *expression_id,
                defer_type_evaluation,
            )? {
                let canonical_symbol = self.canonical_symbol_id(
                    ctx.module_symbol_view(),
                    target_symbol,
                    CanonicalSymbolMode::FollowAliases,
                );
                embedded_symbols.push(canonical_symbol);
            }
        }

        // record lineage when a symbol is provided
        if let Some(symbol) = symbol {
            // preserve additional extends symbols for multi-parent interface traversal
            let extends = extends_symbols.first().copied();
            if extends_symbols.len() > 1 {
                implements_symbols.extend(extends_symbols.iter().copied().skip(1));
            }

            let lineage = Lineage {
                extends,
                implements: implements_symbols,
                embedded: embedded_symbols,
            };
            if !lineage.is_empty() {
                let lineage_id = ctx.types.insert_lineage(lineage);
                ctx.types
                    .set_lineage_for_symbol(symbol.into_global(ctx.module.id), lineage_id);
            }
        }

        Ok(())
    }

    /// Resolve one heritage expression to one merged target symbol.
    fn collect_heritage_symbol(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
        defer_type_evaluation: bool,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // prefer the nominal target already present in syntax
        if let Some(target_symbol) = ctx.tree.get(expression_id).target_symbol() {
            return Ok(Some(self.resolve_type_reference_symbol(ctx, target_symbol)));
        }

        // prefer evaluated type references when type evaluation is enabled
        if !defer_type_evaluation {
            let ty_id = self.resolve_declared_type_expression(
                &mut ctx.reborrow(),
                expression_id,
                true,
                true,
            )?;
            let type_symbol = self.unwrap_type_value_symbol(ctx.types, ty_id);
            if let Some(type_symbol) = type_symbol {
                return Ok(Some(self.resolve_type_reference_symbol(ctx, type_symbol)));
            }
        }

        Ok(None)
    }

    /// Replace the return type of a function signature.
    fn replace_signature_return_type(
        &self,
        ty_id: LocalTypeId,
        return_type: Option<LocalTypeId>,
        source_id: LocalNodeIdAny,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        let Some(return_type) = return_type else {
            return ty_id;
        };

        let Type::Function {
            asynchrony,
            cardinality,
            generic_parameters,
            this_parameter,
            parameters,
            ..
        } = types.get_type(ty_id).clone()
        else {
            return ty_id;
        };

        let rebuilt = Type::Function {
            asynchrony,
            cardinality,
            generic_parameters,
            this_parameter,
            parameters,
            return_type: Some(return_type),
        };
        types.insert_type_from_any(rebuilt, source_id)
    }

    /// Select the target shape for a static or instance member.
    fn member_target_shape(shapes: &mut ObjectShapeSet, is_static: bool) -> &mut ObjectShape {
        if is_static {
            &mut shapes.value
        } else {
            &mut shapes.instance
        }
    }

    /// Add constructor parameter property fields to an instance shape.
    fn add_constructor_parameter_property_fields(
        &self,
        signature: &FunctionSignature,
        signature_ty_id: LocalTypeId,
        shape: &mut ObjectShape,
        tree: &NodeTree,
        types: &TypeTable,
    ) {
        let Type::Function { parameters, .. } = types.get_type(signature_ty_id) else {
            return;
        };

        // map parameter property declarations into instance fields
        for (index, parameter_id) in signature.parameters.iter().enumerate() {
            let parameter = tree.get(*parameter_id);
            let Parameter::Named {
                name,
                visibility,
                is_readonly,
                is_optional,
                ..
            } = parameter
            else {
                continue;
            };
            let is_parameter_property = visibility.is_some() || *is_readonly;
            if !is_parameter_property {
                continue;
            }

            let Some(parameter_ty_id) = parameters.get(index).copied() else {
                continue;
            };

            // skip duplicates from explicit field declarations
            let key = StaticKey::Name(*name);
            if shape.fields.iter().any(|field| field.key == key) {
                continue;
            }

            shape.fields.push(TypeField {
                key,
                ty: parameter_ty_id,
                is_optional: *is_optional,
                is_readonly: *is_readonly,
            });
        }
    }

    /// Build static parameter placeholders for a declaration.
    fn generic_parameter_placeholders_for_declaration(
        &self,
        ctx: &mut TypeContext<'_>,
        generic_parameters: Option<&[LocalNodeId<GenericParameter>]>,
    ) -> Vec<LocalTypeId> {
        // stop when no static parameters exist
        let Some(generic_parameters) = generic_parameters else {
            return Vec::new();
        };

        // map parameters to reference placeholders
        let mut placeholders = Vec::with_capacity(generic_parameters.len());
        for generic_parameter_id in generic_parameters {
            let symbol = ctx
                .tree
                .get(*generic_parameter_id)
                .symbol()
                .into_global(ctx.module.id);
            let ty = Type::Reference {
                symbol,
                generic_arguments: None,
            };
            let type_id = ctx.types.insert_type_from(ty, *generic_parameter_id);
            placeholders.push(type_id);
        }

        placeholders
    }

    /// Build `Self<...>` static arguments for declaration-local nominal references.
    fn self_type_static_arguments_for_declaration(
        &self,
        ctx: &mut TypeContext<'_>,
        generic_parameters: Option<&[LocalNodeId<GenericParameter>]>,
    ) -> Option<Vec<StaticArgument>> {
        // build placeholder type references for static parameters
        let placeholders = self.generic_parameter_placeholders_for_declaration(
            &mut ctx.reborrow(),
            generic_parameters,
        );
        if placeholders.is_empty() {
            return None;
        }

        // map placeholders into static argument expressions
        let arguments = placeholders
            .into_iter()
            .map(|placeholder| StaticArgument::Evaluated {
                name: None,
                value: StaticExpression::Type { ty: placeholder },
            })
            .collect();

        Some(arguments)
    }

    /// Prepend owner static parameters to a function signature when needed.
    fn extend_signature_static_parameters(
        &self,
        ctx: &mut TypeContext<'_>,
        owner_generic_parameters: Option<&[LocalNodeId<GenericParameter>]>,
        ty: Type,
    ) -> Type {
        // NOTE #Cleanup: owner static parameters are still prepended at this declaration step
        let Some(owner_generic_parameters) = owner_generic_parameters else {
            return ty;
        };

        let Type::Function {
            asynchrony,
            cardinality,
            generic_parameters,
            this_parameter,
            parameters,
            return_type,
        } = ty
        else {
            return ty;
        };

        let owner_placeholders = self.generic_parameter_placeholders_for_declaration(
            &mut ctx.reborrow(),
            Some(owner_generic_parameters),
        );
        if owner_placeholders.is_empty() {
            return Type::Function {
                asynchrony,
                cardinality,
                generic_parameters,
                this_parameter,
                parameters,
                return_type,
            };
        }

        let mut combined = owner_placeholders;
        combined.extend(generic_parameters);

        Type::Function {
            asynchrony,
            cardinality,
            generic_parameters: combined,
            this_parameter,
            parameters,
            return_type,
        }
    }

    /// Declare instance and value shapes for a list of members in one ctx context.
    fn collect_member_shapes(
        &self,
        ctx: &mut TypeContext<'_>,
        members: &[LocalNodeId<Member>],
        owner_generic_parameters: Option<&[LocalNodeId<GenericParameter>]>,
        constructor_return: Option<LocalTypeId>,
    ) -> AnalyzeResult<ObjectShapeSet> {
        // defer member type evaluation for declaration modules
        let defer_type_evaluation =
            self.should_defer_declaration_types(ctx.compiler_context, ctx.module);

        // enforce single implementations for TypeScript methods and constructors
        if self.should_enforce_single_overload(ctx.module) {
            let mut method_implementations: HashMap<StaticKey, usize> = HashMap::new();
            let mut constructor_implementations = 0;

            for member_id in members {
                let Member::Method {
                    key,
                    signature,
                    body,
                    ..
                } = ctx.tree.get(*member_id)
                else {
                    continue;
                };

                if body.is_none() {
                    continue;
                }

                if signature.mode == Some(FunctionMode::Constructor) {
                    constructor_implementations += 1;
                    if constructor_implementations > 1 {
                        self.report_overload_implementation_error(
                            ctx.module,
                            ctx.profile,
                            (*member_id).into_any(),
                            true,
                        );
                    }
                    continue;
                }

                let Some(key) = key.as_ref() else {
                    continue;
                };
                let Some(static_key) = self.static_key_from_key(
                    ctx.compiler_context.revision(),
                    ctx.profile,
                    ctx.tree,
                    ctx.symbols,
                    ctx.types,
                    *key,
                ) else {
                    continue;
                };

                let count = method_implementations.entry(static_key).or_insert(0);
                *count += 1;
                if *count > 1 {
                    self.report_overload_implementation_error(
                        ctx.module,
                        ctx.profile,
                        (*member_id).into_any(),
                        false,
                    );
                }
            }
        }

        // initialize member shapes
        let mut shapes = ObjectShapeSet::default();

        // predeclare associated type members so later member references can resolve by symbol
        self.collect_associated_type_members(&mut ctx.reborrow(), members, defer_type_evaluation)?;
        self.collect_member_projection_dependencies(&mut ctx.reborrow(), members);

        // collect member contributions
        for member_id in members {
            let member = ctx.tree.get(*member_id);
            match member {
                Member::AssociatedType { .. } | Member::AssociatedConst { .. } => {}
                Member::Field {
                    key,
                    declared_type,
                    is_optional,
                    is_readonly,
                    is_static,
                    ..
                } => {
                    // route fields to the static or instance shape
                    let target_shape = Self::member_target_shape(&mut shapes, *is_static);

                    // resolve a static key when the member key is structural
                    let static_key = self.static_key_from_key(
                        ctx.compiler_context.revision(),
                        ctx.profile,
                        ctx.tree,
                        ctx.symbols,
                        ctx.types,
                        *key,
                    );

                    // resolve the field type from its declared type
                    let declared_ty_id = if let Some(declared_type_id) = declared_type {
                        let declared_ty_id = self.collect_or_defer_type_expression(
                            &mut ctx.reborrow(),
                            *declared_type_id,
                            defer_type_evaluation,
                        )?;
                        ctx.types.set_declared_type(
                            declared_type_id.into_global_any(ctx.module.id),
                            declared_ty_id,
                        );
                        declared_ty_id
                    } else {
                        let ty = Type::TypeLiteral {
                            value: TypeLiteral::Unknown,
                        };
                        ctx.types.insert_type_from_any(ty, (*member_id).into_any())
                    };

                    // publish the declared member symbol type when present
                    if declared_type.is_some() {
                        let member_symbol = member.symbol().into_global(ctx.module.id);
                        ctx.types.set_value_type(member_symbol, declared_ty_id);
                    }

                    // add the structural field when the key is statically known
                    if let Some(key) = static_key {
                        target_shape.fields.push(TypeField {
                            key,
                            ty: declared_ty_id,
                            is_optional: *is_optional,
                            is_readonly: *is_readonly,
                        });
                    }
                }
                Member::Method {
                    key,
                    signature,
                    is_static,
                    body: _,
                    ..
                } => {
                    // route methods to the static or instance shape
                    let target_shape = Self::member_target_shape(&mut shapes, *is_static);

                    // handle call or construct signatures
                    if key.is_none()
                        && matches!(
                            signature.mode,
                            Some(FunctionMode::Call)
                                | Some(FunctionMode::New)
                                | Some(FunctionMode::Constructor)
                        )
                    {
                        // declare signature generic parameters
                        self.collect_generics(&mut ctx.reborrow(), &signature.generic_parameters)?;

                        // evaluate the signature type
                        let ty = self.resolve_declared_function_signature_type(
                            &mut ctx.reborrow(),
                            signature,
                            (*member_id).into_any(),
                            defer_type_evaluation,
                        )?;
                        let ty = self.extend_signature_static_parameters(
                            &mut ctx.reborrow(),
                            owner_generic_parameters,
                            ty,
                        );
                        let signature_ty_id =
                            ctx.types.insert_type_from_any(ty, (*member_id).into_any());
                        let member_symbol = member.symbol().into_global(ctx.module.id);
                        ctx.types.set_value_type(member_symbol, signature_ty_id);

                        // override constructor returns when needed
                        let construct_signature_id =
                            if signature.mode == Some(FunctionMode::Constructor) {
                                self.replace_signature_return_type(
                                    signature_ty_id,
                                    constructor_return,
                                    (*member_id).into_any(),
                                    ctx.types,
                                )
                            } else {
                                signature_ty_id
                            };

                        // record the declared signature for inference
                        let declared_signature_id =
                            if signature.mode == Some(FunctionMode::Constructor) {
                                let void_ty = Type::TypeLiteral {
                                    value: TypeLiteral::Void,
                                };
                                let void_ty_id = ctx
                                    .types
                                    .insert_type_from_any(void_ty, (*member_id).into_any());
                                self.replace_signature_return_type(
                                    signature_ty_id,
                                    Some(void_ty_id),
                                    (*member_id).into_any(),
                                    ctx.types,
                                )
                            } else {
                                signature_ty_id
                            };
                        ctx.types.set_signature_type_for_node(
                            (*member_id).into_global_any(ctx.module.id),
                            declared_signature_id,
                        );

                        // route the signature to the correct shape
                        match signature.mode {
                            Some(FunctionMode::Constructor) => {
                                shapes
                                    .value
                                    .construct_signatures
                                    .push(construct_signature_id);
                                self.add_constructor_parameter_property_fields(
                                    signature,
                                    signature_ty_id,
                                    &mut shapes.instance,
                                    ctx.tree,
                                    ctx.types,
                                );
                            }
                            Some(FunctionMode::New) => {
                                target_shape.construct_signatures.push(signature_ty_id);
                            }
                            _ => {
                                target_shape.call_signatures.push(signature_ty_id);
                            }
                        }
                        continue;
                    }

                    // resolve the method key
                    let Some(key) = key.as_ref().and_then(|key| {
                        self.static_key_from_key(
                            ctx.compiler_context.revision(),
                            ctx.profile,
                            ctx.tree,
                            ctx.symbols,
                            ctx.types,
                            *key,
                        )
                    }) else {
                        continue;
                    };

                    // declare signature generic parameters
                    self.collect_generics(&mut ctx.reborrow(), &signature.generic_parameters)?;

                    // build the method type
                    let ty = self.resolve_declared_function_signature_type(
                        &mut ctx.reborrow(),
                        signature,
                        (*member_id).into_any(),
                        defer_type_evaluation,
                    )?;
                    let ty = self.extend_signature_static_parameters(
                        &mut ctx.reborrow(),
                        owner_generic_parameters,
                        ty,
                    );
                    let ty_id = ctx.types.insert_type_from_any(ty, (*member_id).into_any());

                    // record the declared signature for inference
                    ctx.types.set_signature_type_for_node(
                        (*member_id).into_global_any(ctx.module.id),
                        ty_id,
                    );

                    // collect field modifiers
                    let is_optional = false;
                    let is_readonly = signature.mode != Some(FunctionMode::Setter);

                    target_shape.fields.push(TypeField {
                        key,
                        ty: ty_id,
                        is_optional,
                        is_readonly,
                    });
                }
                Member::Embed {
                    value, is_static, ..
                } => {
                    // collect embedded fields from the target type
                    let target_shape = Self::member_target_shape(&mut shapes, *is_static);
                    let embed_shape = self.embed_member_shape(&mut ctx.reborrow(), *value)?;
                    target_shape.extend_from_shape(&embed_shape);
                }
                Member::StaticBlock { .. }
                | Member::ComptimeBlock { .. }
                | Member::Error { .. } => {}
            }
        }

        Ok(shapes)
    }

    /// Build a constructor signature for a nominal type alias.
    fn newtype_constructor_signature(
        &self,
        ctx: &mut TypeContext<'_>,
        declaration_id: LocalNodeId<Declaration>,
        value_expression_id: LocalNodeId<TypeExpression>,
        declared_ty_id: LocalTypeId,
        nominal_reference_id: LocalTypeId,
        generic_parameters: Vec<LocalTypeId>,
    ) -> AnalyzeResult<LocalTypeId> {
        // derive positional parameters from tuple aliases
        let mut parameters = Vec::new();
        if let Type::Tuple { elements, .. } = ctx.types.get_type(declared_ty_id) {
            for element in elements {
                parameters.push(element.ty);
            }
        } else {
            let defer_type_evaluation =
                self.should_defer_declaration_types(ctx.compiler_context, ctx.module);
            match ctx.tree.get(value_expression_id) {
                TypeExpression::Tuple { elements } => {
                    for element_id in elements {
                        let Some(argument_value) =
                            ctx.tree.get::<TupleElement>(*element_id).value()
                        else {
                            continue;
                        };
                        let parameter_type_id = self.collect_or_defer_type_expression(
                            &mut ctx.reborrow(),
                            argument_value,
                            defer_type_evaluation,
                        )?;
                        parameters.push(parameter_type_id);
                    }
                }
                _ => {
                    parameters.push(declared_ty_id);
                }
            }
        }

        // build the constructor signature
        let signature = Type::Function {
            asynchrony: Asynchrony::Sync,
            cardinality: FunctionCardinality::Scalar,
            generic_parameters,
            this_parameter: None,
            parameters,
            return_type: Some(nominal_reference_id),
        };
        Ok(ctx.types.insert_type_from(signature, declaration_id))
    }

    /// Declare the instance shape for a list of members in one ctx context.
    fn collect_type_member_shape(
        &self,
        ctx: &mut TypeContext<'_>,
        members: &[LocalNodeId<TypeMember>],
        owner_generic_parameters: Option<&[LocalNodeId<GenericParameter>]>,
    ) -> AnalyzeResult<ObjectShape> {
        // defer member type evaluation for declaration modules
        let defer_type_evaluation =
            self.should_defer_declaration_types(ctx.compiler_context, ctx.module);
        let mut shape = ObjectShape::default();

        // predeclare associated type members so later member references can resolve by symbol
        self.collect_type_member_associated_types(
            &mut ctx.reborrow(),
            members,
            defer_type_evaluation,
        )?;
        self.collect_type_member_projection_dependencies(&mut ctx.reborrow(), members);

        // collect member contributions
        for member_id in members {
            let member_shape = self.collect_type_member(
                &mut ctx.reborrow(),
                *member_id,
                owner_generic_parameters,
                defer_type_evaluation,
            )?;
            shape.extend_from_shape(&member_shape);
        }

        Ok(shape)
    }

    /// Declare a single type member into an object shape.
    fn collect_type_member(
        &self,
        ctx: &mut TypeContext<'_>,
        member_id: LocalNodeId<TypeMember>,
        owner_generic_parameters: Option<&[LocalNodeId<GenericParameter>]>,
        defer_type_evaluation: bool,
    ) -> AnalyzeResult<ObjectShape> {
        // load the type member
        let member = ctx.tree.get(member_id);
        let mut shape = ObjectShape::default();

        // collect the member contribution
        match member {
            // associated members only contribute through their symbols
            TypeMember::AssociatedType { .. } => Ok(shape),
            TypeMember::AssociatedConst {
                declared_type,
                symbol,
                ..
            } => {
                // publish the declared member type when present
                if let Some(declared_type_id) = declared_type {
                    let declared_type_id = *declared_type_id;
                    let declared_ty_id = self.collect_or_defer_type_expression(
                        &mut ctx.reborrow(),
                        declared_type_id,
                        defer_type_evaluation,
                    )?;
                    ctx.types.set_declared_type(
                        declared_type_id.into_global_any(ctx.module.id),
                        declared_ty_id,
                    );

                    let member_symbol = symbol.into_global(ctx.module.id);
                    ctx.types.set_value_type(member_symbol, declared_ty_id);
                }

                Ok(shape)
            }

            // named fields become structural fields
            TypeMember::Field {
                key,
                declared_type,
                is_optional,
                is_readonly,
                symbol,
            } => {
                // resolve a static key for the field
                let static_key = self.static_key_from_key(
                    ctx.compiler_context.revision(),
                    ctx.profile,
                    ctx.tree,
                    ctx.symbols,
                    ctx.types,
                    *key,
                );

                // resolve the field type
                let declared_ty_id = if let Some(declared_type_id) = declared_type {
                    let declared_ty_id = self.collect_or_defer_type_expression(
                        &mut ctx.reborrow(),
                        *declared_type_id,
                        defer_type_evaluation,
                    )?;
                    ctx.types.set_declared_type(
                        declared_type_id.into_global_any(ctx.module.id),
                        declared_ty_id,
                    );
                    declared_ty_id
                } else {
                    self.error(AnalyzeError::MissingType {
                        node: member_id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile)),
                    });
                    ctx.types
                        .insert_type_from_any(Type::Error, member_id.into_any())
                };

                // publish the declared member symbol type
                let member_symbol = symbol.into_global(ctx.module.id);
                ctx.types.set_value_type(member_symbol, declared_ty_id);

                // build the field when a static key exists
                if let Some(key) = static_key {
                    shape.fields.push(TypeField {
                        key,
                        ty: declared_ty_id,
                        is_optional: *is_optional,
                        is_readonly: *is_readonly,
                    });
                }

                Ok(shape)
            }

            // call signatures contribute callable members
            TypeMember::CallSignature { signature, symbol } => {
                self.collect_generics(&mut ctx.reborrow(), &signature.generic_parameters)?;

                let signature = FunctionSignature {
                    is_abstract: false,
                    is_override: false,
                    asynchrony: Asynchrony::Sync,
                    cardinality: FunctionCardinality::Scalar,
                    mode: None,
                    kind: FunctionKind::Lambda,
                    generic_parameters: signature.generic_parameters.clone(),
                    where_clauses: signature.where_clauses.clone(),
                    this_parameter: signature.this_parameter,
                    parameters: signature.parameters.clone(),
                    return_type: signature.return_type,
                };
                let ty = self.resolve_declared_function_signature_type(
                    &mut ctx.reborrow(),
                    &signature,
                    member_id.into_any(),
                    defer_type_evaluation,
                )?;
                let ty = self.extend_signature_static_parameters(
                    &mut ctx.reborrow(),
                    owner_generic_parameters,
                    ty,
                );
                let signature_ty_id = ctx.types.insert_type_from_any(ty, member_id.into_any());
                let member_symbol = symbol.into_global(ctx.module.id);
                ctx.types.set_value_type(member_symbol, signature_ty_id);
                ctx.types.set_signature_type_for_node(
                    member_id.into_global_any(ctx.module.id),
                    signature_ty_id,
                );
                shape.call_signatures.push(signature_ty_id);

                Ok(shape)
            }

            // construct signatures contribute callable members
            TypeMember::ConstructSignature { signature, symbol } => {
                self.collect_generics(&mut ctx.reborrow(), &signature.generic_parameters)?;

                let signature = FunctionSignature {
                    is_abstract: signature.is_abstract,
                    is_override: false,
                    asynchrony: Asynchrony::Sync,
                    cardinality: FunctionCardinality::Scalar,
                    mode: Some(FunctionMode::New),
                    kind: FunctionKind::Lambda,
                    generic_parameters: signature.generic_parameters.clone(),
                    where_clauses: signature.where_clauses.clone(),
                    this_parameter: None,
                    parameters: signature.parameters.clone(),
                    return_type: signature.return_type,
                };
                let ty = self.resolve_declared_function_signature_type(
                    &mut ctx.reborrow(),
                    &signature,
                    member_id.into_any(),
                    defer_type_evaluation,
                )?;
                let ty = self.extend_signature_static_parameters(
                    &mut ctx.reborrow(),
                    owner_generic_parameters,
                    ty,
                );
                let signature_ty_id = ctx.types.insert_type_from_any(ty, member_id.into_any());
                let member_symbol = symbol.into_global(ctx.module.id);
                ctx.types.set_value_type(member_symbol, signature_ty_id);
                ctx.types.set_signature_type_for_node(
                    member_id.into_global_any(ctx.module.id),
                    signature_ty_id,
                );
                shape.construct_signatures.push(signature_ty_id);

                Ok(shape)
            }

            // methods contribute named callable members
            TypeMember::Method {
                key,
                signature,
                is_optional,
                symbol,
                ..
            } => {
                // declare signature generic parameters
                self.collect_generics(&mut ctx.reborrow(), &signature.generic_parameters)?;

                // resolve the method key
                let Some(key) = self.static_key_from_key(
                    ctx.compiler_context.revision(),
                    ctx.profile,
                    ctx.tree,
                    ctx.symbols,
                    ctx.types,
                    *key,
                ) else {
                    return Ok(shape);
                };

                // build the method type
                let ty = self.resolve_declared_function_signature_type(
                    &mut ctx.reborrow(),
                    signature,
                    member_id.into_any(),
                    defer_type_evaluation,
                )?;
                let ty = self.extend_signature_static_parameters(
                    &mut ctx.reborrow(),
                    owner_generic_parameters,
                    ty,
                );
                let ty_id = ctx.types.insert_type_from_any(ty, member_id.into_any());

                // publish the declared member symbol type
                let member_symbol = symbol.into_global(ctx.module.id);
                ctx.types.set_value_type(member_symbol, ty_id);
                ctx.types
                    .set_signature_type_for_node(member_id.into_global_any(ctx.module.id), ty_id);

                // setters are write only, everything else is readable
                let is_readonly = signature.mode != Some(FunctionMode::Setter);

                shape.fields.push(TypeField {
                    key,
                    ty: ty_id,
                    is_optional: *is_optional,
                    is_readonly,
                });

                Ok(shape)
            }

            // index signatures contribute structural index signatures
            TypeMember::IndexSignature {
                name,
                key_type,
                value_type,
                is_optional,
                is_readonly,
                ..
            } => {
                let key_type_id = *key_type;
                let key_type = self.collect_or_defer_type_expression(
                    &mut ctx.reborrow(),
                    key_type_id,
                    defer_type_evaluation,
                )?;
                ctx.types
                    .set_declared_type(key_type_id.into_global_any(ctx.module.id), key_type);

                let value_type_id = *value_type;
                let value_type = self.collect_or_defer_type_expression(
                    &mut ctx.reborrow(),
                    value_type_id,
                    defer_type_evaluation,
                )?;
                ctx.types
                    .set_declared_type(value_type_id.into_global_any(ctx.module.id), value_type);

                shape.index_signatures.push(TypeIndexSignature {
                    name: *name,
                    key_type,
                    value_type,
                    is_optional: *is_optional,
                    is_readonly: *is_readonly,
                });

                Ok(shape)
            }

            // embeds contribute the embedded shape directly
            TypeMember::Embed { value, .. } => {
                let embed_shape = self.embed_member_shape(&mut ctx.reborrow(), *value)?;
                shape.extend_from_shape(&embed_shape);

                Ok(shape)
            }

            // malformed nodes do not contribute shape
            TypeMember::Error { .. } => Ok(shape),
        }
    }

    /// Declare the instance shape for a list of members in one ctx context.
    fn collect_member_shape(
        &self,
        ctx: &mut TypeContext<'_>,
        members: &[LocalNodeId<Member>],
        owner_generic_parameters: Option<&[LocalNodeId<GenericParameter>]>,
    ) -> AnalyzeResult<ObjectShape> {
        // defer member type evaluation for declaration modules
        let defer_type_evaluation =
            self.should_defer_declaration_types(ctx.compiler_context, ctx.module);
        let mut shape = ObjectShape::default();

        // predeclare associated type members so later member references can resolve by symbol
        self.collect_associated_type_members(&mut ctx.reborrow(), members, defer_type_evaluation)?;
        self.collect_member_projection_dependencies(&mut ctx.reborrow(), members);

        // collect member contributions
        for member_id in members {
            let member_shape = self.collect_member(
                &mut ctx.reborrow(),
                *member_id,
                owner_generic_parameters,
                defer_type_evaluation,
            )?;
            shape.extend_from_shape(&member_shape);
        }

        Ok(shape)
    }

    /// Declare a single member into an object shape.
    fn collect_member(
        &self,
        ctx: &mut TypeContext<'_>,
        member_id: LocalNodeId<Member>,
        owner_generic_parameters: Option<&[LocalNodeId<GenericParameter>]>,
        defer_type_evaluation: bool,
    ) -> AnalyzeResult<ObjectShape> {
        let member = ctx.tree.get(member_id);
        let mut shape = ObjectShape::default();

        match member {
            Member::AssociatedType { .. } | Member::AssociatedConst { .. } => Ok(shape),
            Member::Field {
                key,
                declared_type,
                is_optional,
                is_readonly,
                ..
            } => {
                // resolve a static key for the field
                let static_key = self.static_key_from_key(
                    ctx.compiler_context.revision(),
                    ctx.profile,
                    ctx.tree,
                    ctx.symbols,
                    ctx.types,
                    *key,
                );

                // resolve the field type
                let declared_ty_id = if let Some(declared_type_id) = declared_type {
                    let declared_ty_id = self.collect_or_defer_type_expression(
                        &mut ctx.reborrow(),
                        *declared_type_id,
                        defer_type_evaluation,
                    )?;
                    ctx.types.set_declared_type(
                        declared_type_id.into_global_any(ctx.module.id),
                        declared_ty_id,
                    );
                    declared_ty_id
                } else {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    ctx.types.insert_type_from_any(ty, member_id.into_any())
                };

                // publish the declared member symbol type when the field has an explicit type
                if declared_type.is_some() {
                    let member_symbol = member.symbol().into_global(ctx.module.id);
                    ctx.types.set_value_type(member_symbol, declared_ty_id);
                }

                // build the field when a static key exists
                if let Some(key) = static_key {
                    shape.fields.push(TypeField {
                        key,
                        ty: declared_ty_id,
                        is_optional: *is_optional,
                        is_readonly: *is_readonly,
                    });
                }

                Ok(shape)
            }
            Member::Method {
                key,
                signature,
                body,
                ..
            } => {
                // declare signature generic parameters
                self.collect_generics(&mut ctx.reborrow(), &signature.generic_parameters)?;

                // handle call or construct signatures
                if key.is_none()
                    && body.is_none()
                    && matches!(
                        signature.mode,
                        Some(FunctionMode::Call)
                            | Some(FunctionMode::New)
                            | Some(FunctionMode::Constructor)
                    )
                {
                    let ty = self.resolve_declared_function_signature_type(
                        &mut ctx.reborrow(),
                        signature,
                        member_id.into_any(),
                        defer_type_evaluation,
                    )?;
                    let ty = self.extend_signature_static_parameters(
                        &mut ctx.reborrow(),
                        owner_generic_parameters,
                        ty,
                    );
                    let ty_id = ctx.types.insert_type_from_any(ty, member_id.into_any());
                    let member_symbol = member.symbol().into_global(ctx.module.id);
                    ctx.types.set_value_type(member_symbol, ty_id);

                    // record the declared signature for inference
                    ctx.types.set_signature_type_for_node(
                        member_id.into_global_any(ctx.module.id),
                        ty_id,
                    );

                    match signature.mode {
                        Some(FunctionMode::New) | Some(FunctionMode::Constructor) => {
                            shape.construct_signatures.push(ty_id);
                            if signature.mode == Some(FunctionMode::Constructor) {
                                self.add_constructor_parameter_property_fields(
                                    signature, ty_id, &mut shape, ctx.tree, ctx.types,
                                );
                            }
                        }
                        _ => {
                            shape.call_signatures.push(ty_id);
                        }
                    }

                    return Ok(shape);
                }

                // resolve the method key
                let Some(key) = key.as_ref().and_then(|key| {
                    self.static_key_from_key(
                        ctx.compiler_context.revision(),
                        ctx.profile,
                        ctx.tree,
                        ctx.symbols,
                        ctx.types,
                        *key,
                    )
                }) else {
                    return Ok(shape);
                };

                // build the method type
                let ty = self.resolve_declared_function_signature_type(
                    &mut ctx.reborrow(),
                    signature,
                    member_id.into_any(),
                    defer_type_evaluation,
                )?;
                let ty = self.extend_signature_static_parameters(
                    &mut ctx.reborrow(),
                    owner_generic_parameters,
                    ty,
                );
                let ty_id = ctx.types.insert_type_from_any(ty, member_id.into_any());
                let member_symbol = member.symbol().into_global(ctx.module.id);
                ctx.types.set_value_type(member_symbol, ty_id);

                // record the declared signature for inference
                ctx.types
                    .set_signature_type_for_node(member_id.into_global_any(ctx.module.id), ty_id);

                // collect field modifiers
                let is_optional = false;
                let is_readonly = signature.mode != Some(FunctionMode::Setter);

                shape.fields.push(TypeField {
                    key,
                    ty: ty_id,
                    is_optional,
                    is_readonly,
                });

                Ok(shape)
            }
            Member::Embed { value, .. } => {
                // collect embedded fields from the target type
                let embed_shape = self.embed_member_shape(&mut ctx.reborrow(), *value)?;
                shape.extend_from_shape(&embed_shape);

                Ok(shape)
            }
            Member::StaticBlock { .. } | Member::ComptimeBlock { .. } | Member::Error { .. } => {
                Ok(shape)
            }
        }
    }

    /// Resolve a local value type id for a symbol.
    fn resolve_value_type_for_symbol(
        &self,
        ctx: &mut TypeContext<'_>,
        declaration_id: LocalNodeId<Declaration>,
        symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // use the local value type when available
        if symbol.module_id == ctx.module.id {
            return Ok(ctx.types.get_value_type_id(symbol));
        }

        let remote_dir = self
            .require_artifact_dir_declared(
                ctx.compiler_context.revision(),
                symbol.module_id,
                ctx.profile,
            )
            .map_err(AnalyzeError::from)?;
        let remote_value = remote_dir
            .types
            .get_value_type_id(symbol)
            .map(|remote_value_id| {
                let remote_value_ty = remote_dir.types.get_type(remote_value_id).clone();
                let remote_snapshot = remote_dir.types.as_ref().clone();
                (remote_value_ty, remote_snapshot)
            });

        Ok(remote_value.map(|(remote_value_ty, remote_snapshot)| {
            self.import_remote_type_for_node(
                declaration_id.into_any(),
                &remote_value_ty,
                &remote_snapshot,
                ctx.types,
            )
        }))
    }

    /// Collect constructor signatures from a symbol value type.
    fn collect_constructor_signatures_for_symbol(
        &self,
        ctx: &mut TypeContext<'_>,
        declaration_id: LocalNodeId<Declaration>,
        symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Vec<LocalTypeId>> {
        // resolve the local value type for the symbol
        let local_value_id =
            self.resolve_value_type_for_symbol(&mut ctx.reborrow(), declaration_id, symbol)?;

        // stop when no value type is available
        let Some(local_value_id) = local_value_id else {
            return Ok(Vec::new());
        };

        // collect construct signatures from the value type
        let mut shape = ObjectShape::default();
        let mut extras = Vec::new();
        let mut visited = Vec::new();
        self.collect_value_shape_from_type(
            local_value_id,
            ctx.types,
            &mut shape,
            &mut extras,
            &mut visited,
        );
        Ok(shape.construct_signatures)
    }

    /// Ensure constructors exist for a nominal value shape in one ctx context.
    fn ensure_constructor_signatures(
        &self,
        ctx: &mut TypeContext<'_>,
        declaration_id: LocalNodeId<Declaration>,
        symbol: GlobalSymbolId,
        nominal_reference_id: LocalTypeId,
        value_shape: &mut ObjectShape,
    ) -> AnalyzeResult<()> {
        // stop once constructors exist
        if !value_shape.construct_signatures.is_empty() {
            return Ok(());
        }

        // prefer positional constructors for nominal declarations
        let declaration = ctx.tree.get(declaration_id);
        if let Some(members) = declaration.member_ids() {
            let owner_generic_parameters = declaration.generic_parameters();
            let signature_id = self.struct_constructor_signature(
                &mut ctx.reborrow(),
                nominal_reference_id,
                declaration_id,
                owner_generic_parameters,
                members,
            )?;
            value_shape.construct_signatures.push(signature_id);
            return Ok(());
        }

        // inherit constructors from the base class when present
        if let Some(lineage) = ctx.types.get_lineage_for_symbol(symbol).cloned()
            && let Some(base_symbol) = lineage.extends
        {
            let inherited = self.collect_constructor_signatures_for_symbol(
                &mut ctx.reborrow(),
                declaration_id,
                base_symbol,
            )?;
            if !inherited.is_empty() {
                value_shape.construct_signatures.extend(inherited);
                return Ok(());
            }
        }

        // fall back to a default constructor
        let owner_generic_parameters = ctx.tree.get(declaration_id).generic_parameters();
        let generic_parameters = self.generic_parameter_placeholders_for_declaration(
            &mut ctx.reborrow(),
            owner_generic_parameters,
        );
        let signature = Type::Function {
            asynchrony: Asynchrony::Sync,
            cardinality: FunctionCardinality::Scalar,
            generic_parameters,
            this_parameter: None,
            parameters: Vec::new(),
            return_type: Some(nominal_reference_id),
        };
        let signature_id = ctx.types.insert_type_from(signature, declaration_id);
        value_shape.construct_signatures.push(signature_id);

        Ok(())
    }

    /// Report one collected type diagnostic and return an error type id for the source node.
    fn report_collected_type_error(
        &self,
        ctx: &mut TypeContext<'_>,
        node_id: LocalNodeIdAny,
        error: AnalyzeError,
    ) -> LocalTypeId {
        self.error(error);
        ctx.types.insert_type_from_any(Type::Error, node_id)
    }

    /// Build a positional constructor signature for nominal fields in one ctx context.
    fn struct_constructor_signature(
        &self,
        ctx: &mut TypeContext<'_>,
        nominal_reference_id: LocalTypeId,
        declaration_id: LocalNodeId<Declaration>,
        owner_generic_parameters: Option<&[LocalNodeId<GenericParameter>]>,
        members: &[LocalNodeId<Member>],
    ) -> AnalyzeResult<LocalTypeId> {
        // defer field type evaluation for declaration modules
        let defer_type_evaluation =
            self.should_defer_declaration_types(ctx.compiler_context, ctx.module);

        // collect field types in source order
        let mut parameters = Vec::new();
        for member_id in members {
            let member = ctx.tree.get(*member_id);
            let Member::Field {
                declared_type,
                is_static,
                ..
            } = member
            else {
                continue;
            };

            // skip static fields
            if *is_static {
                continue;
            }

            // resolve the field type or poison the constructor slot
            let field_ty_id = if let Some(declared_type_id) = declared_type {
                self.collect_or_defer_type_expression(
                    &mut ctx.reborrow(),
                    *declared_type_id,
                    defer_type_evaluation,
                )?
            } else {
                let error = AnalyzeError::ImplicitAny {
                    node: member_id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile)),
                };
                self.report_collected_type_error(
                    &mut ctx.reborrow(),
                    (*member_id).into_any(),
                    error,
                )
            };
            parameters.push(field_ty_id);
        }

        // build the constructor signature
        let generic_parameters = self.generic_parameter_placeholders_for_declaration(
            &mut ctx.reborrow(),
            owner_generic_parameters,
        );
        let signature = Type::Function {
            asynchrony: Asynchrony::Sync,
            cardinality: FunctionCardinality::Scalar,
            generic_parameters,
            this_parameter: None,
            parameters,
            return_type: Some(nominal_reference_id),
        };
        Ok(ctx.types.insert_type_from(signature, declaration_id))
    }
}
