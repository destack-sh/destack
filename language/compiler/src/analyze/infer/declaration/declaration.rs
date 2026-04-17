use std::collections::{HashMap, HashSet};

use super::expression::has_implicit_return;
use crate::analyze::common::{
    InferContext, ModuleSymbolView, SymbolTypeView, TreeSymbolView, TypeContext, TypeRewriteCache,
    TypeView, TypeWalkContext, TypeWalkKey, rewrite_type_with_cache,
};
use crate::analyze::declare::StaticConstantResolutionMode;
use crate::analyze::infer::member::MemberLookupMode;
use crate::analyze::infer::obligation::relation::UnassignableRelationFailureMode;
use crate::analyze::{
    AssociatedComptimeRequirement, AssociatedTypeRequirement, StaticMemberSymbolKind,
};
use crate::{
    AnalyzeError, AnalyzeOptions, AnalyzeResult, Assignability, CanonicalSymbolMode, Compiler,
    CompilerContext, InferState,
};
use destack_artifact::Data;
use destack_core::StringId;
use destack_dir::{
    Asynchrony, Constraint, Declaration, Declarator, DependencyItem, DependencyKind, EnumField,
    Expression, FunctionCardinality, FunctionKind, FunctionMode, FunctionSignature,
    GenericParameter, GlobalNodeIdAny, GlobalSymbolId, InferOrigin, InferScope, InferTable,
    IntType, Key, LocalNodeId, LocalNodeIdAny, LocalTypeId, Member, NodeTree, NodeType,
    NormalizationMode, Parameter, Pattern, PrimitiveType, StaticArgument, StaticExpression,
    StaticKey, SymbolSpace, SymbolTable, SymbolType, Type, TypeExpression, TypeField, TypeLiteral,
    TypeMember, TypeRewriter, TypeRewriterOptions, TypeTable, WhereClause,
};
use destack_source::ModuleId;
use destack_workspace::{Module, ModuleSource, ProfileId};

/// Describe how a declarator constrains its value type.
pub(crate) enum DeclaratorConstraint {
    /// Require assignability between value and declared types.
    Assignable,
    /// Require satisfies semantics between value and declared types.
    Satisfies,
}

/// Declaration associated type member metadata.
struct DeclarationAssociatedTypeMember<'a> {
    /// The declaration member node id.
    member_id: LocalNodeId<Member>,
    /// The declaration member symbol.
    member_symbol: GlobalSymbolId,
    /// The declaration associated type parameter nodes.
    member_parameters: &'a [LocalNodeId<GenericParameter>],
    /// The declaration associated type default expression.
    member_value: Option<LocalNodeId<TypeExpression>>,
}

/// Declaration associated comptime member metadata.
struct DeclarationAssociatedComptimeMember {
    /// The declaration member node id.
    member_id: LocalNodeId<Member>,
    /// The declaration associated comptime annotation expression.
    member_type: Option<LocalNodeId<TypeExpression>>,
    /// The declaration associated comptime value expression.
    member_value: Option<LocalNodeId<Expression>>,
}

/// Resolved contract context for associated requirement checks.
struct AssociatedContractContext {
    /// The contract declaration symbol.
    contract_symbol: GlobalSymbolId,
    /// Substitutions for contract static parameters.
    substitutions: HashMap<GlobalSymbolId, LocalTypeId>,
}

/// Rewrite associated type references from base owners into receiver owner members.
struct OverrideAssociatedTypeRewriter<'a> {
    /// The compiler instance.
    compiler: &'a Compiler,
    /// The pinned compiler context.
    compiler_context: &'a CompilerContext<'a>,
    /// The current module.
    module: &'a Module,
    /// The active profile.
    profile: ProfileId,
    /// The declaration tree.
    tree: &'a NodeTree,
    /// The declaration symbols.
    symbols: &'a SymbolTable,
    /// The concrete override receiver declaration symbol.
    receiver_symbol: GlobalSymbolId,
    /// The rewriter cache key.
    cache_key: u64,
    /// The rewriter options.
    options: TypeRewriterOptions,
    /// The local rewrite cache.
    cache: TypeRewriteCache,
}

impl<'a> OverrideAssociatedTypeRewriter<'a> {
    /// Create a rewriter for one override receiver symbol.
    fn new(
        compiler: &'a Compiler,
        compiler_context: &'a CompilerContext<'a>,
        module: &'a Module,
        profile: ProfileId,
        tree: &'a NodeTree,
        symbols: &'a SymbolTable,
        receiver_symbol: GlobalSymbolId,
    ) -> Self {
        let module_symbol_view = ModuleSymbolView::new(compiler_context, module, profile, symbols);
        let receiver_symbol = compiler.canonical_symbol_id(
            module_symbol_view,
            receiver_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        let receiver_symbol = compiler
            .declaration_symbol_id(module_symbol_view, receiver_symbol)
            .unwrap_or(receiver_symbol);

        let receiver_key = receiver_symbol.module_id.package_id.raw()
            ^ ((receiver_symbol.module_id.local_id as u64) << 32)
            ^ ((receiver_symbol.local_id.id as u64) << 1)
            ^ ((receiver_symbol.local_id.ty as u64) << 53);
        let walk_context = TypeWalkContext::new(TypeWalkKey::BASE).with_context_key(receiver_key);
        let options = walk_context.rewriter_options();
        let cache_key = options.cache_key();

        Self {
            compiler,
            compiler_context,
            module,
            profile,
            tree,
            symbols,
            receiver_symbol,
            cache_key,
            options,
            cache: TypeRewriteCache::new(),
        }
    }

    /// Borrow module and symbols as one module symbol view.
    fn module_symbol_view(&self) -> ModuleSymbolView<'_> {
        ModuleSymbolView::new(
            self.compiler_context,
            self.module,
            self.profile,
            self.symbols,
        )
    }

    /// Borrow module, tree, and symbols as one tree symbol view.
    fn tree_symbol_view(&self) -> TreeSymbolView<'_> {
        TreeSymbolView::new(
            self.compiler_context,
            self.module,
            self.profile,
            self.tree,
            self.symbols,
        )
    }

    /// Resolve one direct associated type member symbol for the receiver and member name.
    fn direct_associated_type_symbol_for_name(
        &self,
        member_name: StringId,
    ) -> Option<GlobalSymbolId> {
        let receiver_symbol = self
            .compiler
            .declaration_symbol_id(self.module_symbol_view(), self.receiver_symbol)
            .unwrap_or(self.receiver_symbol);
        self.compiler
            .with_module_tree_symbol_view_or_local_for_artifact(
                self.compiler_context,
                self.module,
                self.profile,
                receiver_symbol.module_id,
                self.tree,
                self.symbols,
                destack_artifact::ArtifactKey::dir_declared,
                |view| {
                    let symbol_entry = view.symbols.get_symbol(receiver_symbol.local_id);
                    let mut declaration_ids = Vec::new();
                    if let Some(primary_declaration) = symbol_entry.primary_declaration {
                        declaration_ids.push(primary_declaration);
                    }
                    if let Some(secondary_declarations) =
                        symbol_entry.secondary_declarations.as_deref()
                    {
                        declaration_ids.extend(secondary_declarations.iter().copied());
                    }

                    for declaration_id in declaration_ids {
                        if declaration_id.local_id.ty != NodeType::Declaration {
                            continue;
                        }
                        let declaration_id = declaration_id.local_id.into_typed::<Declaration>();
                        let declaration = view.tree.get(declaration_id);
                        let Some(members) = declaration.member_ids() else {
                            continue;
                        };

                        for member_id in members {
                            let Member::AssociatedType { name, symbol, .. } =
                                view.tree.get(*member_id)
                            else {
                                continue;
                            };
                            if *name == member_name {
                                return Some(symbol.into_global(view.module.id));
                            }
                        }
                    }

                    None
                },
            )
            .ok()
            .flatten()
    }

    /// Return true when one owner symbol belongs to the receiver extends chain.
    fn owner_is_receiver_ancestor(&self, owner_symbol: GlobalSymbolId, types: &TypeTable) -> bool {
        self.compiler.is_type_lineage_assignable(
            SymbolTypeView::new(
                self.compiler_context,
                self.module,
                self.profile,
                self.symbols,
                types,
            ),
            self.receiver_symbol,
            owner_symbol,
        )
    }
}

impl TypeRewriter for OverrideAssociatedTypeRewriter<'_> {
    fn options(&self) -> &TypeRewriterOptions {
        &self.options
    }

    fn rewrite_any(
        &mut self,
        types: &mut TypeTable,
        type_id: LocalTypeId,
        ty: &Type,
    ) -> Option<LocalTypeId> {
        let Type::Reference {
            symbol,
            static_arguments,
        } = ty
        else {
            return None;
        };

        let source_symbol = self.compiler.canonical_symbol_id(
            self.module_symbol_view(),
            *symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        let source_symbol = self
            .compiler
            .declaration_symbol_id(self.module_symbol_view(), source_symbol)
            .unwrap_or(source_symbol);

        if self
            .compiler
            .query_static_member_symbol_kind_for_symbol(self.tree_symbol_view(), source_symbol)
            .ok()?
            != Some(StaticMemberSymbolKind::AssociatedType)
        {
            return None;
        }

        let source_owner = self
            .compiler
            .query_owner_symbol_for_member_symbol(self.module_symbol_view(), source_symbol)
            .ok()
            .flatten()?;
        let source_owner = self
            .compiler
            .declaration_symbol_id(self.module_symbol_view(), source_owner)
            .unwrap_or(source_owner);
        if !self.owner_is_receiver_ancestor(source_owner, types) {
            return None;
        }

        let source_name = self
            .compiler
            .symbol_name_for_global_in(self.module_symbol_view(), source_symbol)?;
        let member_key = StaticKey::Name(source_name);

        let mut mapped_symbol = self
            .direct_associated_type_symbol_for_name(source_name)
            .or_else(|| {
                self.compiler.query_static_member_symbol(
                    self.compiler_context.revision(),
                    self.module,
                    self.profile,
                    self.receiver_symbol,
                    member_key,
                    self.tree,
                    self.symbols,
                )
            })?;
        if self
            .compiler
            .query_static_member_symbol_kind_for_symbol(self.tree_symbol_view(), mapped_symbol)
            .ok()?
            != Some(StaticMemberSymbolKind::AssociatedType)
        {
            return None;
        }

        mapped_symbol = self.compiler.canonical_symbol_id(
            self.module_symbol_view(),
            mapped_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        mapped_symbol = self
            .compiler
            .declaration_symbol_id(self.module_symbol_view(), mapped_symbol)
            .unwrap_or(mapped_symbol);
        if mapped_symbol == source_symbol {
            return None;
        }

        Some(types.insert_type_from_type(
            Type::Reference {
                symbol: mapped_symbol,
                static_arguments: static_arguments.clone(),
            },
            type_id,
        ))
    }

    fn rewrite_type_id(&mut self, types: &mut TypeTable, type_id: LocalTypeId) -> LocalTypeId {
        let mut cache = std::mem::take(&mut self.cache);
        let mapped = rewrite_type_with_cache(self, types, &mut cache, self.cache_key, type_id);
        self.cache = cache;
        mapped
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Decide whether an expression needs inference work.
    pub(crate) fn expression_requires_infer(
        &self,
        _module: &Module,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
    ) -> bool {
        let expression = tree.get(expression_id);
        match expression {
            Expression::Declaration(declaration) => {
                self.declaration_requires_infer(_module, *declaration, tree)
            }
            _ => true,
        }
    }

    /// Decide whether a declaration needs inference work.
    pub(crate) fn declaration_requires_infer(
        &self,
        _module: &Module,
        declaration_id: LocalNodeId<Declaration>,
        tree: &NodeTree,
    ) -> bool {
        let declaration = tree.get(declaration_id);
        let is_ambient = match declaration {
            Declaration::Global(declaration) => declaration.ambient.is_ambient(),
            Declaration::Namespace(declaration) => declaration.ambient.is_ambient(),
            Declaration::Type(declaration) => declaration.ambient.is_ambient(),
            Declaration::ImportAlias(declaration) => declaration.ambient.is_ambient(),
            Declaration::Struct(declaration) => declaration.ambient.is_ambient(),
            Declaration::Class(declaration) => declaration.ambient.is_ambient(),
            Declaration::Enum(declaration) => declaration.ambient.is_ambient(),
            Declaration::Interface(declaration) => declaration.ambient.is_ambient(),
            Declaration::Extension(declaration) => declaration.ambient.is_ambient(),
            Declaration::Function(declaration) => declaration.ambient.is_ambient(),
        };
        if is_ambient {
            return false;
        }

        match declaration {
            Declaration::Type(_) | Declaration::ImportAlias(_) => false,
            Declaration::Global(declaration) => declaration
                .expressions
                .iter()
                .copied()
                .any(|expression_id| self.expression_requires_infer(_module, expression_id, tree)),
            Declaration::Namespace(declaration) => declaration
                .expressions
                .iter()
                .copied()
                .any(|expression_id| self.expression_requires_infer(_module, expression_id, tree)),
            _ => true,
        }
    }

    /// Commit inferred return types at function boundaries.
    fn materialize_inferred_return_type(
        &self,
        ctx: &mut InferContext<'_>,
        state: &InferState,
        return_type: Option<LocalTypeId>,
        body_ty_id: LocalTypeId,
    ) -> LocalTypeId {
        let types = &mut *ctx.types;
        let should_commit =
            return_type.is_some_and(|return_ty_id| self.is_infer_var_type(return_ty_id, types));
        if should_commit {
            // preserve return literal precision for contextual generic inference variables
            if self.return_type_is_contextual_type_parameter_infer_var(
                return_type,
                ctx.infer,
                types,
            ) {
                return body_ty_id;
            }

            let materialize_ctx = state.for_widening_commit();
            self.materialize_binding_type(ctx.module, &materialize_ctx, body_ty_id, types, false)
        } else {
            body_ty_id
        }
    }

    /// Return true when a return type infer var originates from a contextual type parameter.
    fn return_type_is_contextual_type_parameter_infer_var(
        &self,
        return_type: Option<LocalTypeId>,
        infer: &InferTable,
        types: &TypeTable,
    ) -> bool {
        let Some(return_ty_id) = return_type else {
            return false;
        };
        let Type::InferVar { id } = types.get_type(return_ty_id) else {
            return false;
        };

        let Some(var) = infer.vars.get(id.0 as usize) else {
            return false;
        };
        matches!(var.origin, InferOrigin::TypeParameter(_))
    }

    /// Infer the type of a declaration.
    pub(crate) fn infer_declaration(
        &self,
        ctx: &mut InferContext<'_>,
        declaration_id: LocalNodeId<Declaration>,
        state: &mut InferState,
    ) -> AnalyzeResult<()> {
        // load the declaration node
        let declaration = ctx.tree.get(declaration_id);

        // skip inference for ambient declarations
        let is_ambient = match declaration {
            Declaration::Global(declaration) => declaration.ambient.is_ambient(),
            Declaration::Namespace(declaration) => declaration.ambient.is_ambient(),
            Declaration::Type(declaration) => declaration.ambient.is_ambient(),
            Declaration::ImportAlias(declaration) => declaration.ambient.is_ambient(),
            Declaration::Struct(declaration) => declaration.ambient.is_ambient(),
            Declaration::Class(declaration) => declaration.ambient.is_ambient(),
            Declaration::Enum(declaration) => declaration.ambient.is_ambient(),
            Declaration::Interface(declaration) => declaration.ambient.is_ambient(),
            Declaration::Extension(declaration) => declaration.ambient.is_ambient(),
            Declaration::Function(declaration) => declaration.ambient.is_ambient(),
        };
        if is_ambient {
            return Ok(());
        }

        // cache module id for dispatch helpers
        let module_id = ctx.module.id;

        // dispatch by declaration kind
        match declaration {
            // global
            Declaration::Global(declaration) => {
                self.infer_expression_list(&mut ctx.reborrow(), &declaration.expressions, state)
            }

            // namespace
            Declaration::Namespace(declaration) => self.infer_namespace_declaration(
                &mut ctx.reborrow(),
                Some(declaration.where_clauses.as_slice()),
                &declaration.expressions,
                state,
            ),

            // type alias
            Declaration::Type(_) => Ok(()),

            // import alias
            Declaration::ImportAlias(_) => Ok(()),

            // struct
            Declaration::Struct(declaration) => self.infer_struct_declaration(
                &mut ctx.reborrow(),
                declaration_id,
                declaration.symbol.into_global(module_id),
                Some(declaration.where_clauses.as_slice()),
                &declaration.members,
                &declaration.implements_types,
                state,
            ),

            // class
            Declaration::Class(declaration) => self.infer_class_declaration(
                &mut ctx.reborrow(),
                declaration_id,
                declaration.symbol.into_global(module_id),
                declaration.is_abstract,
                Some(declaration.where_clauses.as_slice()),
                &declaration.members,
                declaration.extends_expression,
                &declaration.implements_types,
                state,
            ),

            // enum
            Declaration::Enum(declaration) => self.infer_enum_declaration(
                &mut ctx.reborrow(),
                declaration_id,
                declaration.symbol.into_global(module_id),
                Some(declaration.where_clauses.as_slice()),
                &declaration.fields,
                &declaration.members,
                state,
            ),

            // extension
            Declaration::Extension(declaration) => {
                let extension_symbol = declaration.symbol.into_global(module_id);
                self.infer_extension_declaration(
                    &mut ctx.reborrow(),
                    declaration_id,
                    extension_symbol,
                    Some(declaration.where_clauses.as_slice()),
                    declaration.target_type,
                    declaration.target_symbol,
                    &declaration.members,
                    &declaration.implements_types,
                    state,
                )
            }

            // interface
            Declaration::Interface(declaration) => self.infer_interface_declaration(
                &mut ctx.reborrow(),
                declaration_id,
                declaration.symbol.into_global(module_id),
                Some(declaration.where_clauses.as_slice()),
                &declaration.members,
                state,
            ),

            // function
            Declaration::Function(declaration) => self.infer_function_declaration(
                &mut ctx.reborrow(),
                declaration_id,
                declaration.symbol,
                &declaration.signature,
                declaration.body,
                state,
            ),
        }?;

        Ok(())
    }

    /// Infer a list of declaration-scoped expressions.
    fn infer_expression_list(
        &self,
        ctx: &mut InferContext<'_>,
        expressions: &[LocalNodeId<Expression>],
        state: &mut InferState,
    ) -> AnalyzeResult<()> {
        // infer each expression that still requires work
        for expression_id in expressions {
            if self.expression_requires_infer(ctx.module, *expression_id, ctx.tree) {
                self.infer_expression(&mut ctx.reborrow(), *expression_id, state)?;
            }
        }

        Ok(())
    }

    /// Infer a namespace declaration body.
    fn infer_namespace_declaration(
        &self,
        ctx: &mut InferContext<'_>,
        where_clauses: Option<&[LocalNodeId<WhereClause>]>,
        expressions: &[LocalNodeId<Expression>],
        state: &mut InferState,
    ) -> AnalyzeResult<()> {
        // namespace bodies disallow top-level await
        state.with_namespace(|state| -> AnalyzeResult<()> {
            // infer namespace where clauses first
            self.infer_where_clauses_maybe(&mut ctx.reborrow(), where_clauses, state)?;

            // infer namespace expressions
            self.infer_expression_list(&mut ctx.reborrow(), expressions, state)?;

            Ok(())
        })
    }

    /// Infer a struct declaration.
    fn infer_struct_declaration(
        &self,
        ctx: &mut InferContext<'_>,
        declaration_id: LocalNodeId<Declaration>,
        symbol: GlobalSymbolId,
        where_clauses: Option<&[LocalNodeId<WhereClause>]>,
        members: &[LocalNodeId<Member>],
        implements_types: &[LocalNodeId<TypeExpression>],
        state: &mut InferState,
    ) -> AnalyzeResult<()> {
        // infer where clauses for member bodies
        self.infer_where_clauses_maybe(&mut ctx.reborrow(), where_clauses, state)?;

        // resolve the nominal type for `this`
        let this_ty_id = Some(ctx.types.insert_type_from(
            Type::Reference {
                symbol,
                static_arguments: None,
            },
            declaration_id,
        ));

        // infer members under nominal struct context
        let mut member_ctx = state.fork().in_nominal_symbol_maybe(Some(symbol));
        for member_id in members {
            self.infer_member(&mut ctx.reborrow(), *member_id, &mut member_ctx, this_ty_id)?;
        }

        // enforce associated type requirements
        self.infer_declaration_associated_types(
            &mut ctx.reborrow(),
            implements_types,
            members,
            false,
        )?;

        // enforce associated comptime requirements
        self.infer_declaration_associated_comptime(
            &mut ctx.reborrow(),
            implements_types,
            members,
            false,
        )?;

        Ok(())
    }

    /// Infer a class declaration.
    fn infer_class_declaration(
        &self,
        ctx: &mut InferContext<'_>,
        declaration_id: LocalNodeId<Declaration>,
        symbol: GlobalSymbolId,
        is_abstract: bool,
        where_clauses: Option<&[LocalNodeId<WhereClause>]>,
        members: &[LocalNodeId<Member>],
        extends_expression: Option<LocalNodeId<Expression>>,
        implements_types: &[LocalNodeId<TypeExpression>],
        state: &mut InferState,
    ) -> AnalyzeResult<()> {
        // infer where clauses for member bodies
        self.infer_where_clauses_maybe(&mut ctx.reborrow(), where_clauses, state)?;

        // resolve the nominal type for `this`
        let this_ty_id = Some(ctx.types.insert_type_from(
            Type::Reference {
                symbol,
                static_arguments: None,
            },
            declaration_id,
        ));

        // infer members under class context
        let mut member_ctx = state
            .fork()
            .in_abstract_class_maybe(is_abstract)
            .in_nominal_symbol_maybe(Some(symbol));
        for member_id in members {
            self.infer_member(&mut ctx.reborrow(), *member_id, &mut member_ctx, this_ty_id)?;
        }

        // collect direct inherited contracts from extends and implements
        let mut contract_types = Vec::new();
        contract_types.extend(implements_types.iter().copied());

        // enforce associated type requirements
        self.infer_declaration_associated_types(
            &mut ctx.reborrow(),
            contract_types.as_slice(),
            members,
            is_abstract,
        )?;

        // enforce associated comptime requirements
        self.infer_declaration_associated_comptime(
            &mut ctx.reborrow(),
            contract_types.as_slice(),
            members,
            is_abstract,
        )?;

        self.infer_class_override_conformance(
            &mut ctx.reborrow(),
            symbol,
            members,
            extends_expression,
        )?;

        Ok(())
    }

    /// Infer class member override type conformance against the nearest base declarations.
    fn infer_class_override_conformance(
        &self,
        ctx: &mut InferContext<'_>,
        class_symbol: GlobalSymbolId,
        members: &[LocalNodeId<Member>],
        extends_expression: Option<LocalNodeId<Expression>>,
    ) -> AnalyzeResult<()> {
        let base_symbol = ctx
            .types
            .get_lineage_for_symbol(class_symbol)
            .and_then(|lineage| lineage.extends);
        let direct_base_substitutions = self.direct_base_substitutions(
            &mut ctx.type_context_reborrow(),
            base_symbol,
            extends_expression,
        );
        let direct_base_parameter_symbols = base_symbol
            .and_then(|symbol| self.collect_static_parameter_symbols(ctx.type_view(), symbol))
            .unwrap_or_default();

        for member_id in members {
            let member = ctx.tree.get(*member_id);
            let (is_static, key) = match member {
                Member::Method {
                    key,
                    signature,
                    is_static,
                    ..
                } => {
                    if signature.mode == Some(FunctionMode::Constructor) {
                        continue;
                    }
                    (*is_static, key.as_ref())
                }
                Member::Field { key, is_static, .. } => (*is_static, Some(key)),
                _ => continue,
            };

            let Some(member_key) = key.and_then(|key| {
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

            if !self.member_overrides_base_chain(base_symbol, is_static, &member_key, ctx.types) {
                continue;
            }

            let Some(member_ty_id) =
                self.symbol_member_type_for_key(class_symbol, is_static, &member_key, ctx.types)
            else {
                continue;
            };
            let Some((base_member_owner_symbol, base_member_ty_id)) =
                self.base_chain_member_type(base_symbol, is_static, &member_key, ctx.types)
            else {
                continue;
            };

            let mut base_member_ty_id = base_member_ty_id;
            if Some(base_member_owner_symbol) == base_symbol
                && !direct_base_substitutions.is_empty()
            {
                let mut materialize_cache = TypeRewriteCache::new();
                let mut substitute_cache = HashMap::new();
                base_member_ty_id = self.instantiate_type_with_substitutions(
                    &mut ctx.type_context_reborrow(),
                    (*member_id).into_any(),
                    Some(base_member_owner_symbol),
                    base_member_ty_id,
                    &direct_base_substitutions,
                    &mut materialize_cache,
                    &mut substitute_cache,
                );
            }

            base_member_ty_id = self.rewrite_override_associated_type_references(
                &mut ctx.type_context_reborrow(),
                base_member_ty_id,
                class_symbol,
            );
            let base_member_ty_id = self.normalize_override_signature_type(
                base_member_ty_id,
                &mut ctx.type_context_reborrow(),
                direct_base_parameter_symbols.as_slice(),
            );
            let member_ty_id = self.normalize_override_signature_type(
                member_ty_id,
                &mut ctx.type_context_reborrow(),
                &[],
            );

            self.enforce_assignability_or_defer_diagnostic(
                &mut ctx.reborrow(),
                (*member_id).into_any(),
                base_member_ty_id,
                member_ty_id,
                UnassignableRelationFailureMode::ReportAndContinue,
            )?;
        }

        Ok(())
    }

    /// Rewrite associated type references in one override signature for one concrete receiver.
    fn rewrite_override_associated_type_references(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
        receiver_symbol: GlobalSymbolId,
    ) -> LocalTypeId {
        let mut rewriter = OverrideAssociatedTypeRewriter::new(
            self,
            ctx.compiler_context,
            ctx.module,
            ctx.profile,
            ctx.tree,
            ctx.symbols,
            receiver_symbol,
        );

        rewriter.rewrite_type_id(ctx.types, type_id)
    }

    /// Infer an enum declaration.
    fn infer_enum_declaration(
        &self,
        ctx: &mut InferContext<'_>,
        declaration_id: LocalNodeId<Declaration>,
        enum_symbol: GlobalSymbolId,
        where_clauses: Option<&[LocalNodeId<WhereClause>]>,
        fields: &[LocalNodeId<EnumField>],
        members: &[LocalNodeId<Member>],
        state: &mut InferState,
    ) -> AnalyzeResult<()> {
        // infer where clauses for member bodies
        self.infer_where_clauses_maybe(&mut ctx.reborrow(), where_clauses, state)?;

        // resolve and record enum field values
        let backing_type =
            self.infer_enum_field_values(&mut ctx.type_context_reborrow(), enum_symbol, fields)?;
        ctx.types.set_enum_backing_type(enum_symbol, backing_type);

        // resolve the nominal type for `this`
        let this_ty_id = Some(ctx.types.insert_type_from(
            Type::Reference {
                symbol: enum_symbol,
                static_arguments: None,
            },
            declaration_id,
        ));

        // infer members under nominal enum context
        let mut member_ctx = state.fork().in_nominal_symbol_maybe(Some(enum_symbol));
        for member_id in members {
            self.infer_member(&mut ctx.reborrow(), *member_id, &mut member_ctx, this_ty_id)?;
        }

        Ok(())
    }

    /// Infer an extension declaration.
    fn infer_extension_declaration(
        &self,
        ctx: &mut InferContext<'_>,
        declaration_id: LocalNodeId<Declaration>,
        _extension_symbol: GlobalSymbolId,
        where_clauses: Option<&[LocalNodeId<WhereClause>]>,
        target_type: LocalNodeId<TypeExpression>,
        target_symbol: Option<GlobalSymbolId>,
        members: &[LocalNodeId<Member>],
        implements_types: &[LocalNodeId<TypeExpression>],
        state: &mut InferState,
    ) -> AnalyzeResult<()> {
        // infer where clauses for member bodies
        self.infer_where_clauses_maybe(&mut ctx.reborrow(), where_clauses, state)?;

        // assign this to the nominal target type when available
        let this_ty_id = if let Some(target) = target_symbol {
            // evaluate the target type to capture static arguments
            let target_ty_id = self.resolve_declared_type_expression(
                &mut ctx.type_context_reborrow(),
                target_type,
                true,
                true,
            )?;
            let static_arguments = self.unwrap_type_symbol(ctx.types, target_ty_id).and_then(
                |(symbol, static_arguments, _)| {
                    if symbol == target {
                        static_arguments
                    } else {
                        None
                    }
                },
            );

            Some(ctx.types.insert_type_from(
                Type::Reference {
                    symbol: target,
                    static_arguments,
                },
                declaration_id,
            ))
        } else {
            None
        };

        // infer members for the extension
        for member_id in members {
            self.infer_member(&mut ctx.reborrow(), *member_id, state, this_ty_id)?;
        }

        // enforce associated type requirements
        self.infer_declaration_associated_types(
            &mut ctx.reborrow(),
            implements_types,
            members,
            false,
        )?;

        // enforce associated comptime requirements
        self.infer_declaration_associated_comptime(
            &mut ctx.reborrow(),
            implements_types,
            members,
            false,
        )?;

        Ok(())
    }

    /// Infer an interface declaration.
    fn infer_interface_declaration(
        &self,
        ctx: &mut InferContext<'_>,
        declaration_id: LocalNodeId<Declaration>,
        symbol: GlobalSymbolId,
        where_clauses: Option<&[LocalNodeId<WhereClause>]>,
        members: &[LocalNodeId<TypeMember>],
        state: &mut InferState,
    ) -> AnalyzeResult<()> {
        // infer where clauses for member bodies
        self.infer_where_clauses_maybe(&mut ctx.reborrow(), where_clauses, state)?;

        // resolve the nominal type for `this`
        let this_ty_id = Some(ctx.types.insert_type_from(
            Type::Reference {
                symbol,
                static_arguments: None,
            },
            declaration_id,
        ));

        // infer members under interface context
        for member_id in members {
            self.infer_type_member(&mut ctx.reborrow(), *member_id, state, this_ty_id)?;
        }

        Ok(())
    }

    /// Resolve one contract member symbol for a key across module boundaries.
    fn static_member_symbol_for_contract_key(
        &self,
        view: TreeSymbolView<'_>,
        contract_symbol: GlobalSymbolId,
        member_key: StaticKey,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        self.with_module_tree_symbol_view_or_local_for_artifact(
            view.compiler_context,
            view.module,
            view.profile,
            contract_symbol.module_id,
            view.tree,
            view.symbols,
            destack_artifact::ArtifactKey::dir_declared,
            |remote_view| {
                self.query_static_member_symbol(
                    view.compiler_context.revision(),
                    remote_view.module,
                    remote_view.profile,
                    contract_symbol,
                    member_key,
                    remote_view.tree,
                    remote_view.symbols,
                )
            },
        )
        .map_err(AnalyzeError::from)
    }

    /// Validate declaration implemented-contract member compatibility.
    pub(crate) fn validate_declaration_contract_conformance(
        &self,
        ctx: &mut TypeContext<'_>,
        owner_source_id: LocalNodeIdAny,
        owner_symbol: GlobalSymbolId,
        implements_types: &[LocalNodeId<TypeExpression>],
        allows_deferred_implementation: bool,
    ) -> AnalyzeResult<()> {
        // non-user modules do not run user conformance diagnostics
        if !matches!(ctx.module.source, ModuleSource::User) {
            return Ok(());
        }

        // abstract owners and declarations without implements clauses are deferred
        if allows_deferred_implementation || implements_types.is_empty() {
            return Ok(());
        }

        // resolve the owner instance type
        let Some(owner_type_id) =
            self.require_instance_type(&mut ctx.reborrow(), owner_source_id, owner_symbol)
        else {
            return Ok(());
        };
        if self.symbol_has_missing_associated_requirements(ctx.type_view(), owner_symbol)? {
            return Ok(());
        }
        let owner_type_id = ctx.types.unwrap_value_type_id(owner_type_id);
        let owner_type = ctx.types.get_type(owner_type_id).clone();

        // validate each implemented contract member against the owner member
        for contract_expression_id in implements_types {
            let Some(contract_context) = self.associated_contract_context_for_expression(
                &mut ctx.reborrow(),
                *contract_expression_id,
            )?
            else {
                continue;
            };
            let contract_symbol = contract_context.contract_symbol;
            let contract_is_user_module = self
                .with_module_types_or_local_for_artifact(
                    ctx.compiler_context,
                    ctx.module,
                    ctx.profile,
                    contract_symbol.module_id,
                    ctx.types,
                    destack_artifact::ArtifactKey::dir_declared,
                    |module, _| matches!(module.source, ModuleSource::User),
                )
                .map_err(AnalyzeError::from)?;
            if !contract_is_user_module {
                continue;
            }
            let Some(contract_type_id) = self.require_instance_type(
                &mut ctx.reborrow(),
                contract_expression_id.into_any(),
                contract_symbol,
            ) else {
                continue;
            };
            let contract_type_id = ctx.types.unwrap_value_type_id(contract_type_id);

            let contract_fields = match ctx.types.get_type(contract_type_id) {
                Type::Object { fields, .. } => fields.clone(),
                _ => continue,
            };
            for contract_field in contract_fields {
                // associated members are validated by associated requirement passes
                if let Some(member_symbol) = self.static_member_symbol_for_contract_key(
                    ctx.tree_symbol_view(),
                    contract_symbol,
                    contract_field.key,
                )? && matches!(
                    self.query_static_member_symbol_kind_for_symbol(
                        ctx.tree_symbol_view(),
                        member_symbol,
                    )?,
                    Some(
                        StaticMemberSymbolKind::AssociatedType
                            | StaticMemberSymbolKind::AssociatedComptimeConst
                    )
                ) {
                    continue;
                }

                let contract_member_ty_id = self.substitute_and_materialize_contract_type(
                    &mut ctx.reborrow(),
                    contract_field.ty,
                    &contract_context.substitutions,
                );

                // unresolved associated projections stay deferred until convergence
                if self.type_requires_static_evaluation_convergence(
                    ctx.type_view(),
                    contract_member_ty_id,
                ) {
                    continue;
                }

                let contract_member_ty_id = self.rewrite_override_associated_type_references(
                    &mut ctx.reborrow(),
                    contract_member_ty_id,
                    owner_symbol,
                );

                let mut visited = Vec::new();
                let owner_member_ty_id = self.infer_member_of_type(
                    &mut ctx.reborrow(),
                    contract_expression_id.into_any(),
                    &owner_type,
                    &contract_field.key,
                    MemberLookupMode::Instance,
                    &mut visited,
                )?;
                let Some(owner_member_ty_id) = owner_member_ty_id else {
                    let missing_member_ty_id = ctx.types.insert_type_from_any(
                        Type::TypeLiteral {
                            value: TypeLiteral::Unknown,
                        },
                        contract_expression_id.into_any(),
                    );
                    self.emit_unassignable_type_for_types(
                        ctx.module_type_view(),
                        contract_expression_id.into_any(),
                        contract_member_ty_id,
                        missing_member_ty_id,
                    );
                    continue;
                };

                let normalized_contract_ty_id = self.normalize_type(
                    &mut ctx.reborrow(),
                    contract_member_ty_id,
                    NormalizationMode::Assign,
                );
                let normalized_owner_ty_id = self.normalize_type(
                    &mut ctx.reborrow(),
                    owner_member_ty_id,
                    NormalizationMode::Assign,
                );
                let mut static_materialize_cache = TypeRewriteCache::default();
                let normalized_contract_ty_id = self.materialize_static_arguments_in_type(
                    &mut ctx.reborrow(),
                    normalized_contract_ty_id,
                    &mut static_materialize_cache,
                );
                let normalized_owner_ty_id = self.materialize_static_arguments_in_type(
                    &mut ctx.reborrow(),
                    normalized_owner_ty_id,
                    &mut static_materialize_cache,
                );

                // unresolved substituted projections stay deferred in declaration-only checks
                let relation_requires_convergence =
                    self.type_requires_static_evaluation_convergence(
                        ctx.type_view(),
                        normalized_contract_ty_id,
                    ) || self.type_requires_static_evaluation_convergence(
                        ctx.type_view(),
                        normalized_owner_ty_id,
                    );
                if relation_requires_convergence {
                    continue;
                }

                let equivalent = normalized_contract_ty_id == normalized_owner_ty_id || {
                    let left_assignable = self.is_type_assignable(
                        &mut ctx.reborrow(),
                        normalized_contract_ty_id,
                        normalized_owner_ty_id,
                    );
                    let right_assignable = self.is_type_assignable(
                        &mut ctx.reborrow(),
                        normalized_owner_ty_id,
                        normalized_contract_ty_id,
                    );
                    left_assignable.is_assignable() && right_assignable.is_assignable()
                };
                if equivalent {
                    continue;
                }

                let assignability = self.is_type_assignable(
                    &mut ctx.reborrow(),
                    normalized_contract_ty_id,
                    normalized_owner_ty_id,
                );
                if assignability == Assignability::NotAssignable {
                    self.emit_unassignable_type_for_types(
                        ctx.module_type_view(),
                        contract_expression_id.into_any(),
                        normalized_contract_ty_id,
                        normalized_owner_ty_id,
                    );
                }
            }
        }

        Ok(())
    }

    /// Load a declared type for a node, importing from remote modules when needed.
    pub(crate) fn declared_type_for_node(
        &self,
        ctx: &mut TypeContext<'_>,
        node_id: GlobalNodeIdAny,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // resolve local declared types directly
        if node_id.module_id == ctx.module.id {
            return Ok(ctx.types.get_declared_type_id(node_id));
        }

        // import declared types from remote modules
        let remote_dir = self
            .require_indexed_dir_declared(
                &ctx.index,
                ctx.compiler_context.revision(),
                node_id.module_id,
                ctx.profile,
            )
            .map_err(AnalyzeError::from)?;
        let remote_declared = remote_dir
            .types
            .get_declared_type_id(node_id)
            .map(|remote_ty_id| {
                let remote_ty = remote_dir.types.get_type(remote_ty_id).clone();
                let remote_snapshot = remote_dir.types.as_ref().clone();
                (remote_ty, remote_snapshot)
            });

        Ok(remote_declared.map(|(remote_ty, remote_snapshot)| {
            self.import_remote_type_for_node(
                node_id.local_id,
                &remote_ty,
                &remote_snapshot,
                ctx.types,
            )
        }))
    }

    /// Infer associated type contracts for declarations implementing interfaces.
    fn infer_declaration_associated_types(
        &self,
        ctx: &mut InferContext<'_>,
        contract_types: &[LocalNodeId<TypeExpression>],
        members: &[LocalNodeId<Member>],
        allows_deferred_associated_types: bool,
    ) -> AnalyzeResult<()> {
        if !matches!(ctx.module.source, ModuleSource::User) {
            return Ok(());
        }

        if contract_types.is_empty() {
            return Ok(());
        }

        let declaration_members =
            self.collect_declaration_associated_type_members(ctx.module.id, members, ctx.tree);
        let mut inherited_defaults_by_name = HashMap::new();

        for contract_expression_id in contract_types {
            self.infer_associated_type_requirements_for_contract(
                &mut ctx.type_context_reborrow(),
                *contract_expression_id,
                &declaration_members,
                &mut inherited_defaults_by_name,
                allows_deferred_associated_types,
            )?;
        }

        Ok(())
    }

    /// Collect associated type members for one declaration.
    fn collect_declaration_associated_type_members<'a>(
        &self,
        module_id: ModuleId,
        members: &'a [LocalNodeId<Member>],
        tree: &'a NodeTree,
    ) -> HashMap<StringId, DeclarationAssociatedTypeMember<'a>> {
        let mut declaration_members = HashMap::new();
        for member_id in members {
            let Member::AssociatedType {
                name,
                generic_parameters,
                value,
                symbol,
                ..
            } = tree.get(*member_id)
            else {
                continue;
            };

            declaration_members.insert(
                *name,
                DeclarationAssociatedTypeMember {
                    member_id: *member_id,
                    member_symbol: symbol.into_global(module_id),
                    member_parameters: generic_parameters.as_slice(),
                    member_value: *value,
                },
            );
        }

        declaration_members
    }

    /// Infer associated comptime contracts for declarations implementing interfaces.
    fn infer_declaration_associated_comptime(
        &self,
        ctx: &mut InferContext<'_>,
        contract_types: &[LocalNodeId<TypeExpression>],
        members: &[LocalNodeId<Member>],
        allows_deferred_associated_comptime: bool,
    ) -> AnalyzeResult<()> {
        if !matches!(ctx.module.source, ModuleSource::User) {
            return Ok(());
        }

        if contract_types.is_empty() {
            return Ok(());
        }

        let declaration_members =
            self.collect_declaration_associated_comptime_members(members, ctx.tree);
        let mut inherited_defaults_by_name = HashMap::new();

        for contract_expression_id in contract_types {
            self.infer_associated_comptime_requirements(
                &mut ctx.type_context_reborrow(),
                *contract_expression_id,
                &declaration_members,
                &mut inherited_defaults_by_name,
                allows_deferred_associated_comptime,
            )?;
        }

        Ok(())
    }

    /// Collect associated comptime members for one declaration.
    fn collect_declaration_associated_comptime_members(
        &self,
        members: &[LocalNodeId<Member>],
        tree: &NodeTree,
    ) -> HashMap<StringId, DeclarationAssociatedComptimeMember> {
        let mut declaration_members = HashMap::new();
        for member_id in members {
            let Member::AssociatedConst {
                name,
                declared_type,
                value,
                ..
            } = tree.get(*member_id)
            else {
                continue;
            };

            declaration_members.insert(
                *name,
                DeclarationAssociatedComptimeMember {
                    member_id: *member_id,
                    member_type: *declared_type,
                    member_value: *value,
                },
            );
        }

        declaration_members
    }

    // associated contracts: comptime requirements
    /// Enforce associated comptime requirements for one inherited contract.
    fn infer_associated_comptime_requirements(
        &self,
        ctx: &mut TypeContext<'_>,
        contract_expression_id: LocalNodeId<TypeExpression>,
        declaration_members: &HashMap<StringId, DeclarationAssociatedComptimeMember>,
        inherited_defaults_by_name: &mut HashMap<StringId, LocalTypeId>,
        allows_deferred_associated_comptime: bool,
    ) -> AnalyzeResult<()> {
        let Some(contract_context) = self.associated_contract_context_for_expression(
            &mut ctx.reborrow(),
            contract_expression_id,
        )?
        else {
            return Ok(());
        };
        let requirements = self.collect_contract_associated_comptime_requirements(
            ctx.tree_symbol_view(),
            contract_context.contract_symbol,
        )?;

        for requirement in requirements {
            let Some(declaration_member) = declaration_members.get(&requirement.name) else {
                // require explicit implementations for abstract members
                if requirement.requires_implementation && !allows_deferred_associated_comptime {
                    let node = contract_expression_id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile));
                    self.error(AnalyzeError::InvalidStaticArgument {
                        node,
                        message: "missing associated comptime implementation".to_string(),
                    });
                    continue;
                }

                // validate that inherited defaults do not conflict by name
                self.validate_inherited_associated_comptime_default(
                    &mut ctx.reborrow(),
                    contract_expression_id,
                    &requirement,
                    &contract_context.substitutions,
                    inherited_defaults_by_name,
                )?;
                continue;
            };

            // skip members without any concrete information
            if declaration_member.member_type.is_none() && declaration_member.member_value.is_none()
            {
                continue;
            }

            // resolve and substitute the contract requirement type
            let Some(requirement_type_node) = requirement.type_node else {
                continue;
            };
            let Some(mut requirement_type_id) =
                self.declared_type_for_node(&mut ctx.reborrow(), requirement_type_node)?
            else {
                continue;
            };
            requirement_type_id = self.substitute_and_materialize_contract_type(
                &mut ctx.reborrow(),
                requirement_type_id,
                &contract_context.substitutions,
            );

            // resolve the declaration member type from annotation or initializer
            let Some(declaration_type_id) =
                self.associated_comptime_member_type_id(&mut ctx.reborrow(), declaration_member)?
            else {
                continue;
            };

            // defer relation checks until both sides are stable enough for static evaluation
            let requirement_requires_convergence = self
                .type_requires_static_evaluation_convergence(ctx.type_view(), requirement_type_id);
            let declaration_requires_convergence = self
                .type_requires_static_evaluation_convergence(ctx.type_view(), declaration_type_id);
            if requirement_requires_convergence || declaration_requires_convergence {
                continue;
            }

            // enforce assignability from declaration member type to contract requirement
            let is_assignable = self.is_type_assignable(
                &mut ctx.reborrow(),
                requirement_type_id,
                declaration_type_id,
            );
            if is_assignable == Assignability::NotAssignable {
                self.emit_unassignable_type_for_types(
                    ctx.module_type_view(),
                    declaration_member.member_id.into_any(),
                    requirement_type_id,
                    declaration_type_id,
                );
            }
        }

        Ok(())
    }

    /// Validate that inherited associated comptime defaults agree across implemented interfaces.
    fn validate_inherited_associated_comptime_default(
        &self,
        ctx: &mut TypeContext<'_>,
        contract_expression_id: LocalNodeId<TypeExpression>,
        requirement: &AssociatedComptimeRequirement,
        interface_substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        inherited_defaults_by_name: &mut HashMap<StringId, LocalTypeId>,
    ) -> AnalyzeResult<()> {
        // skip non default requirements
        if requirement.requires_implementation {
            return Ok(());
        }

        // resolve and substitute the inherited default value
        let mut visited_symbols = HashSet::new();
        let Some(default_value) = self.resolve_static_constant_reference_for_mode(
            &mut ctx.reborrow(),
            requirement.symbol,
            Some(interface_substitutions),
            &mut visited_symbols,
            StaticConstantResolutionMode::InstantiatedInfer,
        )?
        else {
            return Ok(());
        };
        let Some(default_ty_id) = self.static_expression_type_id(
            contract_expression_id.into_any(),
            &default_value,
            ctx.types,
        ) else {
            return Ok(());
        };
        self.register_or_validate_inherited_default(
            &mut ctx.reborrow(),
            contract_expression_id,
            requirement.name,
            default_ty_id,
            inherited_defaults_by_name,
            "incompatible associated comptime defaults across inherited contracts",
        );
        Ok(())
    }

    /// Convert a static expression into a type id for relation checks.
    fn static_expression_type_id(
        &self,
        source_id: LocalNodeIdAny,
        value: &StaticExpression,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        match value {
            StaticExpression::ScalarLiteral { value } => Some(types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(value.clone()),
                },
                source_id,
            )),
            StaticExpression::TypeLiteral { value } => Some(types.insert_type_from_any(
                Type::TypeLiteral {
                    value: value.clone(),
                },
                source_id,
            )),
            StaticExpression::Type { ty } => Some(*ty),
            _ => None,
        }
    }

    // associated contracts: type requirements
    /// Enforce associated type requirements for one inherited contract.
    fn infer_associated_type_requirements_for_contract(
        &self,
        ctx: &mut TypeContext<'_>,
        contract_expression_id: LocalNodeId<TypeExpression>,
        declaration_members: &HashMap<StringId, DeclarationAssociatedTypeMember<'_>>,
        inherited_defaults_by_name: &mut HashMap<StringId, LocalTypeId>,
        allows_deferred_associated_types: bool,
    ) -> AnalyzeResult<()> {
        let Some(contract_context) = self.associated_contract_context_for_expression(
            &mut ctx.reborrow(),
            contract_expression_id,
        )?
        else {
            return Ok(());
        };
        let requirements = self.collect_contract_associated_type_requirements(
            ctx.tree_symbol_view(),
            contract_context.contract_symbol,
        )?;

        for requirement in requirements {
            let Some(declaration_member) = declaration_members.get(&requirement.name) else {
                // require explicit implementations for abstract members
                if requirement.requires_implementation && !allows_deferred_associated_types {
                    let node = contract_expression_id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile));
                    self.error(AnalyzeError::InvalidStaticArgument {
                        node,
                        message: "missing associated type implementation".to_string(),
                    });
                    continue;
                }

                // validate that inherited defaults do not conflict by name
                self.validate_inherited_associated_default(
                    &mut ctx.reborrow(),
                    contract_expression_id,
                    &requirement,
                    &contract_context.substitutions,
                    inherited_defaults_by_name,
                );
                continue;
            };

            if !self.enforce_associated_type_parameter_arity(
                ctx.module,
                ctx.profile,
                requirement.parameter_symbols.len(),
                declaration_member.member_parameters,
                declaration_member.member_id,
            ) {
                continue;
            }

            if !self.enforce_associated_type_parameter_kinds(
                &mut ctx.reborrow(),
                requirement.parameter_symbols.as_slice(),
                declaration_member.member_parameters,
                declaration_member.member_id,
            ) {
                continue;
            }

            self.enforce_associated_type_bound_assignability(
                &mut ctx.reborrow(),
                &requirement,
                declaration_member,
                &contract_context.substitutions,
            )?;
        }

        Ok(())
    }

    /// Validate that inherited associated defaults agree across implemented interfaces.
    fn validate_inherited_associated_default(
        &self,
        ctx: &mut TypeContext<'_>,
        contract_expression_id: LocalNodeId<TypeExpression>,
        requirement: &AssociatedTypeRequirement,
        interface_substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        inherited_defaults_by_name: &mut HashMap<StringId, LocalTypeId>,
    ) {
        // resolve and substitute the inherited default target
        let Some(default_ty_id) = self.alias_target_type_id_for_symbol(
            &mut ctx.reborrow(),
            requirement.symbol,
            contract_expression_id.into_any(),
        ) else {
            return;
        };
        let default_ty_id = self.substitute_and_materialize_contract_type(
            &mut ctx.reborrow(),
            default_ty_id,
            interface_substitutions,
        );
        self.register_or_validate_inherited_default(
            &mut ctx.reborrow(),
            contract_expression_id,
            requirement.name,
            default_ty_id,
            inherited_defaults_by_name,
            "incompatible associated type defaults across inherited contracts",
        );
    }

    /// Resolve one associated contract context from one heritage contract expression.
    fn associated_contract_context_for_expression(
        &self,
        ctx: &mut TypeContext<'_>,
        contract_expression_id: LocalNodeId<TypeExpression>,
    ) -> AnalyzeResult<Option<AssociatedContractContext>> {
        let contract_type_id = self.inferred_or_evaluated_type_for_expression(
            &mut ctx.reborrow(),
            contract_expression_id,
        )?;
        let Some((contract_symbol, contract_arguments)) = self
            .resolve_contract_reference_for_associated_type(
                ctx.symbol_type_view(),
                contract_type_id,
            )
        else {
            return Ok(None);
        };
        let Some(contract_symbol) =
            self.declaration_symbol_id(ctx.module_symbol_view(), contract_symbol)
        else {
            return Ok(None);
        };
        if !matches!(
            contract_symbol.ty(),
            SymbolType::Interface | SymbolType::Class
        ) {
            return Ok(None);
        }

        let substitutions = self.build_type_parameter_substitutions_for_symbol(
            &mut ctx.reborrow(),
            contract_symbol,
            contract_expression_id.into_any(),
            &contract_arguments,
        );
        Ok(Some(AssociatedContractContext {
            contract_symbol,
            substitutions,
        }))
    }

    /// Resolve one expression type from inferred, declared, or evaluated data.
    fn inferred_or_evaluated_type_for_expression(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
    ) -> AnalyzeResult<LocalTypeId> {
        if let Some(type_id) = ctx
            .types
            .get_inferred_type_id(expression_id.into_global_any(ctx.module.id))
            .or_else(|| {
                ctx.types
                    .get_declared_type_id(expression_id.into_global_any(ctx.module.id))
            })
        {
            return Ok(type_id);
        }

        self.resolve_declared_type_expression(&mut ctx.reborrow(), expression_id, true, true)
    }

    /// Apply contract substitutions and materialize static value arguments.
    fn substitute_and_materialize_contract_type(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
    ) -> LocalTypeId {
        let mut type_id = type_id;
        if !substitutions.is_empty() {
            let mut substitution_cache = HashMap::new();
            type_id = self.substitute_static_parameters(
                type_id,
                substitutions,
                ctx.types,
                &mut substitution_cache,
            );
        }

        let mut materialize_cache = TypeRewriteCache::new();
        self.materialize_static_arguments_in_type(
            &mut ctx.reborrow(),
            type_id,
            &mut materialize_cache,
        )
    }

    /// Resolve the effective type for one associated comptime declaration member.
    fn associated_comptime_member_type_id(
        &self,
        ctx: &mut TypeContext<'_>,
        declaration_member: &DeclarationAssociatedComptimeMember,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        if let Some(member_type_node) = declaration_member.member_type {
            return self
                .resolve_declared_type_expression(&mut ctx.reborrow(), member_type_node, true, true)
                .map(Some);
        }
        if let Some(member_value_node) = declaration_member.member_value {
            return Ok(ctx.types.get_declared_or_inferred_type_id(
                member_value_node.into_global_any(ctx.module.id),
            ));
        }

        Ok(None)
    }

    /// Register one inherited default or validate it against the existing default.
    fn register_or_validate_inherited_default(
        &self,
        ctx: &mut TypeContext<'_>,
        contract_expression_id: LocalNodeId<TypeExpression>,
        name: StringId,
        default_ty_id: LocalTypeId,
        inherited_defaults_by_name: &mut HashMap<StringId, LocalTypeId>,
        incompatibility_message: &str,
    ) {
        let Some(existing_default_id) = inherited_defaults_by_name.get(&name).copied() else {
            inherited_defaults_by_name.insert(name, default_ty_id);
            return;
        };
        if self.types_are_bidirectionally_assignable(
            &mut ctx.reborrow(),
            existing_default_id,
            default_ty_id,
        ) {
            return;
        }

        let node = contract_expression_id
            .into_global_any(ctx.module.id)
            .into_anchored(Some(ctx.profile));
        self.error(AnalyzeError::InvalidStaticArgument {
            node,
            message: incompatibility_message.to_string(),
        });
    }

    /// Return true when two types are assignable to each other.
    fn types_are_bidirectionally_assignable(
        &self,
        ctx: &mut TypeContext<'_>,
        left_id: LocalTypeId,
        right_id: LocalTypeId,
    ) -> bool {
        let left_to_right = self.is_type_assignable(&mut ctx.reborrow(), left_id, right_id);
        let right_to_left = self.is_type_assignable(&mut ctx.reborrow(), right_id, left_id);
        left_to_right != Assignability::NotAssignable
            && right_to_left != Assignability::NotAssignable
    }

    /// Resolve inherited contract references for associated type requirement checks.
    fn resolve_contract_reference_for_associated_type(
        &self,
        ctx: SymbolTypeView<'_>,
        type_id: LocalTypeId,
    ) -> Option<(GlobalSymbolId, Vec<StaticArgument>)> {
        let mut visited = HashSet::new();
        self.resolve_contract_reference_for_associated_type_inner(ctx, type_id, &mut visited)
    }

    /// Resolve inherited contract references for associated type requirement checks.
    fn resolve_contract_reference_for_associated_type_inner(
        &self,
        ctx: SymbolTypeView<'_>,
        type_id: LocalTypeId,
        visited: &mut HashSet<LocalTypeId>,
    ) -> Option<(GlobalSymbolId, Vec<StaticArgument>)> {
        if !visited.insert(type_id) {
            return None;
        }

        // unwrap direct type references first
        if let Some((symbol, static_arguments, _)) = self.unwrap_type_symbol(ctx.types, type_id) {
            let declaration_symbol = self
                .declaration_symbol_id(ctx.module_symbol_view(), symbol)
                .unwrap_or(symbol);

            if matches!(
                declaration_symbol.ty(),
                SymbolType::Interface | SymbolType::Class
            ) {
                return Some((declaration_symbol, static_arguments.unwrap_or_default()));
            }

            // follow alias targets when contract references are imported through aliases
            if matches!(
                declaration_symbol.ty(),
                SymbolType::TypeAlias | SymbolType::Newtype
            ) && let Some(alias_target_id) =
                ctx.types.get_alias_target_type_id(declaration_symbol)
            {
                return self.resolve_contract_reference_for_associated_type_inner(
                    ctx,
                    alias_target_id,
                    visited,
                );
            }
        }

        // unwrap type-as-value wrappers
        if let Some(contract_symbol) = self.unwrap_type_value_symbol(ctx.types, type_id) {
            let declaration_symbol = self
                .declaration_symbol_id(ctx.module_symbol_view(), contract_symbol)
                .unwrap_or(contract_symbol);

            if matches!(
                declaration_symbol.ty(),
                SymbolType::Interface | SymbolType::Class
            ) {
                return Some((declaration_symbol, Vec::new()));
            }

            if matches!(
                declaration_symbol.ty(),
                SymbolType::TypeAlias | SymbolType::Newtype
            ) && let Some(alias_target_id) =
                ctx.types.get_alias_target_type_id(declaration_symbol)
            {
                return self.resolve_contract_reference_for_associated_type_inner(
                    ctx,
                    alias_target_id,
                    visited,
                );
            }
        }

        None
    }

    /// Enforce associated type parameter arity.
    fn enforce_associated_type_parameter_arity(
        &self,
        module: &Module,
        profile: ProfileId,
        required_arity: usize,
        member_parameters: &[LocalNodeId<GenericParameter>],
        member_id: LocalNodeId<Member>,
    ) -> bool {
        let member_arity = member_parameters.len();
        if member_arity == required_arity {
            return true;
        }

        let node = member_id
            .into_global_any(module.id)
            .into_anchored(Some(profile));
        self.error(AnalyzeError::InvalidStaticArgument {
            node,
            message: "associated type parameter arity mismatch".to_string(),
        });

        false
    }

    /// Enforce associated type parameter kinds.
    fn enforce_associated_type_parameter_kinds(
        &self,
        ctx: &mut TypeContext<'_>,
        requirement_parameter_symbols: &[GlobalSymbolId],
        member_parameters: &[LocalNodeId<GenericParameter>],
        member_id: LocalNodeId<Member>,
    ) -> bool {
        let required_kinds = requirement_parameter_symbols
            .iter()
            .map(|parameter_symbol| {
                self.static_parameter_kind_for_symbol(&mut ctx.reborrow(), *parameter_symbol)
            })
            .collect::<Vec<_>>();
        let member_kinds = member_parameters
            .iter()
            .map(|parameter_id| {
                let parameter_symbol = ctx
                    .tree
                    .get(*parameter_id)
                    .symbol()
                    .into_global(ctx.module.id);
                self.static_parameter_kind_for_symbol(&mut ctx.reborrow(), parameter_symbol)
            })
            .collect::<Vec<_>>();

        let mismatch = required_kinds
            .iter()
            .zip(member_kinds.iter())
            .any(|(required_kind, member_kind)| *required_kind != *member_kind);
        if !mismatch {
            return true;
        }

        let node = member_id
            .into_global_any(ctx.module.id)
            .into_anchored(Some(ctx.profile));
        self.error(AnalyzeError::InvalidStaticArgument {
            node,
            message: "associated type parameter kind mismatch".to_string(),
        });

        false
    }

    /// Enforce associated type bound assignability.
    fn enforce_associated_type_bound_assignability(
        &self,
        ctx: &mut TypeContext<'_>,
        requirement: &AssociatedTypeRequirement,
        declaration_member: &DeclarationAssociatedTypeMember<'_>,
        interface_substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
    ) -> AnalyzeResult<()> {
        let Some(bound_node) = requirement.bound_node else {
            return Ok(());
        };

        let Some(mut bound_ty_id) = self.declared_type_for_node(&mut ctx.reborrow(), bound_node)?
        else {
            return Ok(());
        };
        if !interface_substitutions.is_empty() {
            let mut cache = HashMap::new();
            bound_ty_id = self.substitute_static_parameters(
                bound_ty_id,
                interface_substitutions,
                ctx.types,
                &mut cache,
            );
        }

        let Some(actual_ty_id) = declaration_member
            .member_value
            .and_then(|value_id| {
                ctx.types
                    .get_declared_type_id(value_id.into_global_any(ctx.module.id))
            })
            .or_else(|| {
                ctx.types
                    .get_alias_target_type_id(declaration_member.member_symbol)
            })
        else {
            return Ok(());
        };

        let assignability = self.is_type_assignable(&mut ctx.reborrow(), bound_ty_id, actual_ty_id);
        if assignability != Assignability::NotAssignable {
            return Ok(());
        }

        self.emit_unassignable_type_for_types(
            ctx.module_type_view(),
            declaration_member.member_id.into_any(),
            bound_ty_id,
            actual_ty_id,
        );

        Ok(())
    }

    /// Infer a function declaration.
    fn infer_function_declaration(
        &self,
        ctx: &mut InferContext<'_>,
        declaration_id: LocalNodeId<Declaration>,
        declaration_symbol: destack_dir::LocalSymbolId,
        signature: &FunctionSignature,
        body: Option<LocalNodeId<Expression>>,
        state: &mut InferState,
    ) -> AnalyzeResult<()> {
        // apply decorator options for this function
        let function_options = {
            let symbol = ctx.symbols.get_symbol(declaration_symbol);
            state.options.with_symbol_decorators(&symbol.decorators)
        };

        // enforce runtime constraints up front
        self.check_signature_runtime_constraints(
            ctx,
            declaration_id.into_any(),
            signature,
            function_options,
        );

        // infer the function signature
        let declared_signature_ty_id = ctx
            .types
            .get_signature_type_for_node(declaration_id.into_global_any(ctx.module.id));
        let mut signature_ctx = state.fork().with_options(function_options);
        let should_use_declared_signature = self.should_use_declared_signature(
            ctx.type_view(),
            signature,
            declared_signature_ty_id,
            state.expected_type,
        );
        let fn_ty_id = if should_use_declared_signature {
            let declared_signature_ty_id = self
                .require_declared_signature_type_for_skipped_inference(declared_signature_ty_id)?;
            self.bind_declared_signature(
                &mut ctx.reborrow(),
                declaration_id.into_any(),
                signature,
                declared_signature_ty_id,
                &mut signature_ctx,
            )?
        } else {
            let owner_symbol = declaration_symbol.into_global(ctx.module.id);
            self.infer_signature(
                &mut ctx.reborrow(),
                declaration_id.into_any(),
                owner_symbol,
                signature,
                state.expected_type,
                declared_signature_ty_id,
                &mut signature_ctx,
            )?
        };

        // merge the inferred signature into the symbol value type
        let symbol_entry = ctx.symbols.get_symbol(declaration_symbol);
        let allow_merge = ctx.module.language_type.supports_declaration_merging()
            || ctx.module.language_type.is_destack()
            || symbol_entry.origin.is_global_augmentation();
        self.merge_function_value_type(
            &mut ctx.type_context_reborrow(),
            declaration_id,
            declaration_symbol,
            fn_ty_id,
            declared_signature_ty_id,
            allow_merge,
        );

        // infer the body when needed
        let should_infer_body =
            self.should_infer_function_body(ctx.compiler_context, ctx.module, signature, body);
        if let Some(body) = body
            && should_infer_body
        {
            // prepare return type tracking for the body
            let return_type = self.function_return_type(fn_ty_id, ctx.types);
            let state = state
                .reset()
                .without_const_context()
                .with_options(function_options)
                .in_function_with_signature(declaration_id.into_any(), signature);
            let mut context_return_type = return_type;
            let mut state = if signature.cardinality == FunctionCardinality::Generator {
                let (yield_ty_id, return_ty_id, next_ty_id) = self.generator_context_types(
                    &mut ctx.type_context_reborrow(),
                    declaration_id.into_any(),
                    return_type,
                );
                context_return_type = Some(return_ty_id);
                state
                    .with_return_type(Some(return_ty_id))
                    .with_generator_types(Some(yield_ty_id), Some(next_ty_id))
            } else {
                state.with_return_type(return_type)
            };

            // adjust predicate return types for body expectations
            let mut expected_return_type = context_return_type;
            let mut constraint_return_type = context_return_type;
            if let Some(return_ty_id) = context_return_type {
                match ctx.types.get_type(return_ty_id) {
                    Type::Predicate { asserts: true, .. } => {
                        let void_ty_id = self.void_type_id(ctx.types, body.into_any());
                        expected_return_type = Some(void_ty_id);
                        constraint_return_type = Some(void_ty_id);
                    }
                    Type::Predicate { asserts: false, .. } => {
                        let boolean_ty_id = self.boolean_type_id(ctx.types, body.into_any());
                        expected_return_type = Some(boolean_ty_id);
                        constraint_return_type = Some(boolean_ty_id);
                    }
                    _ => {}
                }
            }
            if expected_return_type != context_return_type {
                state = state.with_return_type(expected_return_type);
            }

            // only propagate return type expectations into expression bodies
            let body_expression = !matches!(ctx.tree.get(body), Expression::Block(_));
            if body_expression {
                state = state.with_expected_type(expected_return_type);
            } else {
                state = state.with_expected_type(None);
            }

            // infer the function body with implicit return typing
            let body_ty_id = self.infer_body(&mut ctx.reborrow(), body, &mut state)?;

            // commit inferred return types for widening
            let committed_body_ty_id = self.materialize_inferred_return_type(
                &mut ctx.reborrow(),
                &state,
                context_return_type,
                body_ty_id,
            );

            // constrain implicit return types against the declared return type
            if let Some(return_ty_id) = constraint_return_type
                && has_implicit_return(body, ctx.tree)
            {
                ctx.infer.push_constraint(Constraint::Subtype {
                    sub_type: committed_body_ty_id,
                    super_type: return_ty_id,
                    variance: None,
                });

                let normalized_return_ty_id = self.normalize_type_for_assignability(
                    &mut ctx.type_context_reborrow(),
                    return_ty_id,
                );

                if !self.return_type_allows_fallthrough_infer(return_ty_id, ctx.types)
                    && !self.type_relation_requires_infer_convergence(
                        ctx.type_view(),
                        normalized_return_ty_id,
                        committed_body_ty_id,
                    )
                    && self.is_type_assignable(
                        &mut ctx.type_context_reborrow(),
                        normalized_return_ty_id,
                        committed_body_ty_id,
                    ) == Assignability::NotAssignable
                {
                    self.emit_unassignable_type_for_types(
                        ctx.module_type_view(),
                        body.into_any(),
                        return_ty_id,
                        body_ty_id,
                    );
                }
            }
        }

        Ok(())
    }

    /// Decide whether a function body should be inferred.
    fn should_infer_function_body(
        &self,
        context: &CompilerContext<'_>,
        module: &Module,
        signature: &FunctionSignature,
        body: Option<LocalNodeId<Expression>>,
    ) -> bool {
        let Some(_) = body else {
            return false;
        };

        if module.language_type.is_declaration() {
            return false;
        }

        let module_checks = context.module_check_options_for_module(module.id);
        if module_checks.skip_lib_check
            && matches!(module.source, ModuleSource::Builtin(_))
            && signature.return_type.is_some()
        {
            return false;
        }

        true
    }

    /// Decide whether a signature can reuse declared types.
    pub(crate) fn should_use_declared_signature(
        &self,
        ctx: TypeView<'_>,
        signature: &FunctionSignature,
        declared_signature_ty_id: Option<LocalTypeId>,
        expected_fn_ty_id: Option<LocalTypeId>,
    ) -> bool {
        if expected_fn_ty_id.is_some() {
            return false;
        }

        if declared_signature_ty_id.is_none() {
            return false;
        }

        self.signature_is_fully_declared(ctx, signature)
    }

    /// Require the declared signature type when signature inference is skipped.
    pub(crate) fn require_declared_signature_type_for_skipped_inference(
        &self,
        declared_signature_ty_id: Option<LocalTypeId>,
    ) -> AnalyzeResult<LocalTypeId> {
        declared_signature_ty_id.ok_or_else(|| AnalyzeError::Internal {
            message: "missing declared signature type for skipped signature inference".into(),
        })
    }

    /// Infer a type-surface member declaration.
    pub(crate) fn infer_type_member(
        &self,
        ctx: &mut InferContext<'_>,
        member_id: LocalNodeId<TypeMember>,
        state: &mut InferState,
        this_ty_id: Option<LocalTypeId>,
    ) -> AnalyzeResult<()> {
        // capture the member node
        let member = ctx.tree.get(member_id);

        // dispatch by member kind
        match member {
            // associated type
            TypeMember::AssociatedType {
                generic_parameters,
                where_clauses,
                constraint,
                value,
                symbol,
                ..
            } => {
                let member_symbol = symbol.into_global(ctx.module.id);

                // resolve generic parameter defaults and constraints
                for parameter_id in generic_parameters {
                    match ctx.tree.get(*parameter_id) {
                        GenericParameter::Type {
                            constraint,
                            default,
                            ..
                        } => {
                            if let Some(constraint) = constraint {
                                self.resolve_declared_type_expression(
                                    &mut ctx.type_context_reborrow(),
                                    *constraint,
                                    true,
                                    true,
                                )?;
                            }

                            if let Some(default) = default {
                                self.resolve_declared_type_expression(
                                    &mut ctx.type_context_reborrow(),
                                    *default,
                                    true,
                                    true,
                                )?;
                            }
                        }
                        GenericParameter::Value {
                            declared_type,
                            default,
                            ..
                        } => {
                            if let Some(declared_type) = declared_type {
                                self.resolve_declared_type_expression(
                                    &mut ctx.type_context_reborrow(),
                                    *declared_type,
                                    true,
                                    true,
                                )?;
                            }

                            if let Some(default) = default {
                                self.infer_expression(&mut ctx.reborrow(), *default, state)?;
                            }
                        }
                        GenericParameter::Error { .. } => {}
                    }
                }

                // infer associated type where clauses
                self.infer_where_clauses_maybe(
                    &mut ctx.reborrow(),
                    Some(where_clauses.as_slice()),
                    state,
                )?;

                // evaluate associated type constraint
                let constraint_type = if let Some(constraint) = constraint {
                    Some(self.resolve_declared_type_expression(
                        &mut ctx.type_context_reborrow(),
                        *constraint,
                        true,
                        true,
                    )?)
                } else {
                    None
                };

                // evaluate and register associated type default
                let value_type = if let Some(value) = value {
                    let value_type = self.resolve_declared_type_expression(
                        &mut ctx.type_context_reborrow(),
                        *value,
                        true,
                        true,
                    )?;

                    ctx.types
                        .set_alias_target_type_id(member_symbol, value_type);
                    ctx.types.set_instance_type(member_symbol, value_type);
                    Some(value_type)
                } else {
                    None
                };

                // validate the default against its constraint
                if let (Some(constraint_type), Some(value_type)) = (constraint_type, value_type) {
                    let is_assignable = self.is_type_assignable(
                        &mut ctx.type_context_reborrow(),
                        constraint_type,
                        value_type,
                    );
                    if is_assignable == Assignability::NotAssignable {
                        self.emit_unassignable_type_for_types(
                            ctx.module_type_view(),
                            member_id.into_any(),
                            constraint_type,
                            value_type,
                        );
                    }
                }

                Ok(())
            }

            // associated comptime
            TypeMember::AssociatedConst {
                declared_type,
                value,
                symbol,
                ..
            } => {
                // resolve the member symbol
                let member_symbol = symbol.into_global(ctx.module.id);

                // evaluate optional annotation
                let constraint_type = if let Some(declared_type) = declared_type {
                    Some(self.resolve_declared_type_expression(
                        &mut ctx.type_context_reborrow(),
                        *declared_type,
                        true,
                        true,
                    )?)
                } else {
                    None
                };

                // evaluate optional initializer
                let value_type = if let Some(value) = value {
                    let static_value = self.evaluate_static_expression_value(
                        &mut ctx.type_context_reborrow(),
                        *value,
                        None,
                    )?;

                    let mut value_type = static_value.as_ref().and_then(|value| {
                        self.static_expression_type_id(member_id.into_any(), value, ctx.types)
                    });

                    if value_type.is_none() {
                        let declared_value_type =
                            self.infer_expression(&mut ctx.reborrow(), *value, state)?;
                        let declared_type_requires_convergence = self
                            .type_requires_static_evaluation_convergence(
                                ctx.type_view(),
                                declared_value_type,
                            );
                        if static_value.is_none() && !declared_type_requires_convergence {
                            self.error(AnalyzeError::InvalidComptimeExpression {
                                node: value
                                    .into_global_any(ctx.module.id)
                                    .into_anchored(Some(ctx.profile)),
                            });

                            let error_type_id = ctx
                                .types
                                .insert_type_from_any(Type::Error, (*value).into_any());
                            ctx.types.set_value_type(member_symbol, error_type_id);
                            return Ok(());
                        }

                        value_type = Some(declared_value_type);
                    }

                    let Some(value_type) = value_type else {
                        return Ok(());
                    };
                    ctx.types.set_value_type(member_symbol, value_type);
                    Some(value_type)
                } else {
                    None
                };

                // validate initializer against annotation
                if let (Some(constraint_type), Some(value_type)) = (constraint_type, value_type) {
                    if matches!(
                        ctx.types.get_type(value_type),
                        Type::Unevaluated(_) | Type::Error
                    ) {
                        return Ok(());
                    }

                    let value_requires_convergence = self
                        .type_requires_static_evaluation_convergence(ctx.type_view(), value_type);
                    let constraint_requires_convergence = self
                        .type_requires_static_evaluation_convergence(
                            ctx.type_view(),
                            constraint_type,
                        );
                    if value_requires_convergence || constraint_requires_convergence {
                        return Ok(());
                    }

                    let is_assignable = self.is_type_assignable(
                        &mut ctx.type_context_reborrow(),
                        constraint_type,
                        value_type,
                    );
                    if is_assignable == Assignability::NotAssignable {
                        self.emit_unassignable_type_for_types(
                            ctx.module_type_view(),
                            member_id.into_any(),
                            constraint_type,
                            value_type,
                        );
                    }
                }

                Ok(())
            }

            // field
            TypeMember::Field {
                key,
                declared_type,
                symbol,
                ..
            } => {
                // resolve the member symbol
                let member_symbol = symbol.into_global(ctx.module.id);

                // infer computed keys first
                if let Key::Expression(key_expression) = key {
                    self.infer_expression(&mut ctx.reborrow(), *key_expression, state)?;
                }

                // resolve and attach the declared type
                let value_ty_id = self.resolve_declared_type_expression(
                    &mut ctx.type_context_reborrow(),
                    *declared_type,
                    true,
                    true,
                )?;
                if ctx.types.get_value_type_id(member_symbol).is_none() {
                    ctx.types.set_value_type(member_symbol, value_ty_id);
                }

                Ok(())
            }

            // method
            TypeMember::Method {
                signature,
                body,
                symbol,
                ..
            } => {
                // resolve the member symbol for value typing
                let member_symbol = symbol.into_global(ctx.module.id);

                // apply decorator options for this method
                let method_options = {
                    let symbol = ctx.symbols.get_symbol(*symbol);
                    state.options.with_symbol_decorators(&symbol.decorators)
                };

                // assign the implicit this binding type when available
                if let Some(this_ty_id) = this_ty_id {
                    let this_name = self.repository.strings.intern("this");
                    let (_scope_id, scope, _mark) = ctx.symbols.get_scope(member_id, ctx.tree);
                    if let Some(this_symbol) = ctx
                        .symbols
                        .find_active_symbol(scope, StaticKey::Name(this_name))
                    {
                        ctx.types
                            .set_value_type(this_symbol.into_global(ctx.module.id), this_ty_id);
                    }
                }

                // enforce runtime constraints up front
                self.check_signature_runtime_constraints(
                    ctx,
                    member_id.into_any(),
                    signature,
                    method_options,
                );

                // infer the method signature
                let declared_signature_ty_id = ctx
                    .types
                    .get_signature_type_for_node(member_id.into_global_any(ctx.module.id));
                let mut signature_ctx = state.fork().with_options(method_options);
                let method_ty_id = if self.should_use_declared_signature(
                    ctx.type_view(),
                    signature,
                    declared_signature_ty_id,
                    None,
                ) {
                    let declared_signature_ty_id = self
                        .require_declared_signature_type_for_skipped_inference(
                            declared_signature_ty_id,
                        )?;
                    self.bind_declared_signature(
                        &mut ctx.reborrow(),
                        member_id.into_any(),
                        signature,
                        declared_signature_ty_id,
                        &mut signature_ctx,
                    )?
                } else {
                    self.infer_signature(
                        &mut ctx.reborrow(),
                        member_id.into_any(),
                        member_symbol,
                        signature,
                        None,
                        declared_signature_ty_id,
                        &mut signature_ctx,
                    )?
                };

                // attach the method type for member symbol lookups
                if let Some(accessor_value_ty_id) =
                    self.accessor_value_type_for_signature(signature, method_ty_id, ctx.types)
                {
                    ctx.types
                        .set_value_type(member_symbol, accessor_value_ty_id);
                } else if ctx.types.get_value_type_id(member_symbol).is_none() {
                    ctx.types.set_value_type(member_symbol, method_ty_id);
                }

                // prepare the return type for body inference
                let mut return_type = self.function_return_type(method_ty_id, ctx.types);
                if let Some(this_ty_id) = this_ty_id {
                    let mut cache = HashMap::new();

                    // substitute this in the explicit this parameter
                    if let Some(this_parameter_id) = signature.this_parameter {
                        let param_symbol = ctx
                            .tree
                            .get(this_parameter_id)
                            .symbol()
                            .into_global(ctx.module.id);
                        if let Some(param_ty_id) = ctx.types.get_value_type_id(param_symbol) {
                            let mapped_ty_id = self.substitute_this_type(
                                param_ty_id,
                                this_ty_id,
                                ctx.types,
                                &mut cache,
                            );
                            ctx.types.set_value_type(param_symbol, mapped_ty_id);
                        }
                    }

                    // substitute this in dynamic parameters
                    for parameter_id in signature.parameters.iter() {
                        let param_symbol = ctx
                            .tree
                            .get(*parameter_id)
                            .symbol()
                            .into_global(ctx.module.id);
                        if let Some(param_ty_id) = ctx.types.get_value_type_id(param_symbol) {
                            let mapped_ty_id = self.substitute_this_type(
                                param_ty_id,
                                this_ty_id,
                                ctx.types,
                                &mut cache,
                            );
                            ctx.types.set_value_type(param_symbol, mapped_ty_id);
                        }
                    }

                    // substitute this in the return type
                    return_type = return_type.map(|return_type| {
                        self.substitute_this_type(return_type, this_ty_id, ctx.types, &mut cache)
                    });
                }

                // infer the body when present
                if let Some(body) = body
                    && self.should_infer_function_body(
                        ctx.compiler_context,
                        ctx.module,
                        signature,
                        Some(*body),
                    )
                {
                    let state = state
                        .reset()
                        .without_const_context()
                        .with_options(method_options)
                        .in_function_with_signature(member_id.into_any(), signature);
                    let mut context_return_type = return_type;
                    let mut state = if signature.cardinality == FunctionCardinality::Generator {
                        let (yield_ty_id, return_ty_id, next_ty_id) = self.generator_context_types(
                            &mut ctx.type_context_reborrow(),
                            member_id.into_any(),
                            return_type,
                        );
                        context_return_type = Some(return_ty_id);
                        state
                            .with_return_type(Some(return_ty_id))
                            .with_generator_types(Some(yield_ty_id), Some(next_ty_id))
                    } else {
                        state.with_return_type(return_type)
                    };

                    // adjust predicate return types for body expectations
                    let mut expected_return_type = context_return_type;
                    let mut constraint_return_type = context_return_type;
                    if let Some(return_ty_id) = context_return_type {
                        match ctx.types.get_type(return_ty_id) {
                            Type::Predicate { asserts: true, .. } => {
                                let void_ty_id = self.void_type_id(ctx.types, body.into_any());
                                expected_return_type = Some(void_ty_id);
                                constraint_return_type = Some(void_ty_id);
                            }
                            Type::Predicate { asserts: false, .. } => {
                                let boolean_ty_id =
                                    self.boolean_type_id(ctx.types, body.into_any());
                                expected_return_type = Some(boolean_ty_id);
                                constraint_return_type = Some(boolean_ty_id);
                            }
                            _ => {}
                        }
                    }
                    if expected_return_type != context_return_type {
                        state = state.with_return_type(expected_return_type);
                    }

                    // only propagate return type expectations into expression bodies
                    let body_expression = !matches!(ctx.tree.get(*body), Expression::Block(_));
                    if body_expression {
                        state = state.with_expected_type(expected_return_type);
                    } else {
                        state = state.with_expected_type(None);
                    }

                    // infer the function body with implicit return typing
                    let body_ty_id = self.infer_body(&mut ctx.reborrow(), *body, &mut state)?;

                    // commit inferred return types for widening
                    let committed_body_ty_id = self.materialize_inferred_return_type(
                        &mut ctx.reborrow(),
                        &state,
                        context_return_type,
                        body_ty_id,
                    );

                    // constrain implicit return types against the declared return type
                    if let Some(return_ty_id) = constraint_return_type
                        && has_implicit_return(*body, ctx.tree)
                    {
                        ctx.infer.push_constraint(Constraint::Subtype {
                            sub_type: committed_body_ty_id,
                            super_type: return_ty_id,
                            variance: None,
                        });

                        let normalized_return_ty_id = self.normalize_type_for_assignability(
                            &mut ctx.type_context_reborrow(),
                            return_ty_id,
                        );

                        if !self.return_type_allows_fallthrough_infer(return_ty_id, ctx.types)
                            && !self.type_relation_requires_infer_convergence(
                                ctx.type_view(),
                                normalized_return_ty_id,
                                committed_body_ty_id,
                            )
                            && self.is_type_assignable(
                                &mut ctx.type_context_reborrow(),
                                normalized_return_ty_id,
                                committed_body_ty_id,
                            ) == Assignability::NotAssignable
                        {
                            self.emit_unassignable_type_for_types(
                                ctx.module_type_view(),
                                body.into_any(),
                                return_ty_id,
                                body_ty_id,
                            );
                        }
                    }
                }

                Ok(())
            }

            // index signature
            TypeMember::IndexSignature {
                key_type,
                value_type,
                symbol,
                ..
            } => {
                self.resolve_declared_type_expression(
                    &mut ctx.type_context_reborrow(),
                    *key_type,
                    true,
                    true,
                )?;

                let value_type_id = self.resolve_declared_type_expression(
                    &mut ctx.type_context_reborrow(),
                    *value_type,
                    true,
                    true,
                )?;
                let member_symbol = symbol.into_global(ctx.module.id);
                if ctx.types.get_value_type_id(member_symbol).is_none() {
                    ctx.types.set_value_type(member_symbol, value_type_id);
                }

                Ok(())
            }

            // embed
            TypeMember::Embed { value, symbol } => {
                let value_type_id = self.resolve_declared_type_expression(
                    &mut ctx.type_context_reborrow(),
                    *value,
                    true,
                    true,
                )?;
                let member_symbol = symbol.into_global(ctx.module.id);
                if ctx.types.get_value_type_id(member_symbol).is_none() {
                    ctx.types.set_value_type(member_symbol, value_type_id);
                }

                Ok(())
            }

            // malformed nodes
            TypeMember::Error { .. } => Ok(()),
        }
    }

    /// Infer a member declaration.
    pub(crate) fn infer_member(
        &self,
        ctx: &mut InferContext<'_>,
        member_id: LocalNodeId<Member>,
        state: &mut InferState,
        this_ty_id: Option<LocalTypeId>,
    ) -> AnalyzeResult<()> {
        // capture the member node
        let member = ctx.tree.get(member_id);

        // dispatch by member kind
        match member {
            Member::AssociatedType {
                name,
                generic_parameters,
                where_clauses,
                constraint,
                value,
                symbol,
                ..
            } => {
                let _name = *name;
                let member_symbol = symbol.into_global(ctx.module.id);

                // resolve generic parameter defaults and constraints
                for parameter_id in generic_parameters {
                    match ctx.tree.get(*parameter_id) {
                        GenericParameter::Type {
                            constraint,
                            default,
                            ..
                        } => {
                            if let Some(constraint) = constraint {
                                self.resolve_declared_type_expression(
                                    &mut ctx.type_context_reborrow(),
                                    *constraint,
                                    true,
                                    true,
                                )?;
                            }

                            if let Some(default) = default {
                                self.resolve_declared_type_expression(
                                    &mut ctx.type_context_reborrow(),
                                    *default,
                                    true,
                                    true,
                                )?;
                            }
                        }
                        GenericParameter::Value {
                            declared_type,
                            default,
                            ..
                        } => {
                            if let Some(declared_type) = declared_type {
                                self.resolve_declared_type_expression(
                                    &mut ctx.type_context_reborrow(),
                                    *declared_type,
                                    true,
                                    true,
                                )?;
                            }

                            if let Some(default) = default {
                                self.infer_expression(&mut ctx.reborrow(), *default, state)?;
                            }
                        }
                        GenericParameter::Error { .. } => {}
                    }
                }

                // infer associated type where clauses
                self.infer_where_clauses_maybe(
                    &mut ctx.reborrow(),
                    Some(where_clauses.as_slice()),
                    state,
                )?;

                // evaluate associated type constraint
                let constraint_type = if let Some(constraint) = constraint {
                    Some(self.resolve_declared_type_expression(
                        &mut ctx.type_context_reborrow(),
                        *constraint,
                        true,
                        true,
                    )?)
                } else {
                    None
                };

                // evaluate and register associated type default
                let value_type = if let Some(value) = value {
                    let value_type = self.resolve_declared_type_expression(
                        &mut ctx.type_context_reborrow(),
                        *value,
                        true,
                        true,
                    )?;

                    ctx.types
                        .set_alias_target_type_id(member_symbol, value_type);
                    ctx.types.set_instance_type(member_symbol, value_type);
                    Some(value_type)
                } else {
                    None
                };

                // check associated type default against its bound
                if let (Some(constraint_type), Some(value_type)) = (constraint_type, value_type) {
                    let is_assignable = self.is_type_assignable(
                        &mut ctx.type_context_reborrow(),
                        constraint_type,
                        value_type,
                    );
                    if is_assignable == Assignability::NotAssignable {
                        self.emit_unassignable_type_for_types(
                            ctx.module_type_view(),
                            member_id.into_any(),
                            constraint_type,
                            value_type,
                        );
                    }
                }

                Ok(())
            }
            Member::AssociatedConst {
                declared_type,
                value,
                symbol,
                ..
            } => {
                // resolve the member symbol
                let member_symbol = symbol.into_global(ctx.module.id);

                // allow missing initializers only on interface members and abstract class members
                let parent_declaration = ctx
                    .tree
                    .get_parent(member_id.id)
                    .filter(|parent| parent.ty == NodeType::Declaration)
                    .map(|parent| parent.into_typed::<Declaration>());
                let is_interface_member = parent_declaration
                    .map(|declaration_id| ctx.tree.get(declaration_id))
                    .is_some_and(|declaration| matches!(declaration, Declaration::Interface(_)));
                let allows_missing_initializer = is_interface_member;

                // reject declaration only members in concrete owners
                if value.is_none() && !allows_missing_initializer {
                    self.error(AnalyzeError::InvalidStaticArgument {
                        node: member_id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile)),
                        message: "associated comptime constants require initializer".to_string(),
                    });
                }

                // evaluate optional annotation
                let constraint_type = if let Some(declared_type) = declared_type {
                    Some(self.resolve_declared_type_expression(
                        &mut ctx.type_context_reborrow(),
                        *declared_type,
                        true,
                        true,
                    )?)
                } else {
                    None
                };

                // evaluate optional initializer
                let value_type = if let Some(value) = value {
                    // evaluate static value once and reuse it for validation and typing
                    let static_value = self.evaluate_static_expression_value(
                        &mut ctx.type_context_reborrow(),
                        *value,
                        None,
                    )?;

                    // prefer static evaluation output for value typing
                    let mut value_type = static_value.as_ref().and_then(|value| {
                        self.static_expression_type_id(member_id.into_any(), value, ctx.types)
                    });

                    // resolve declaration typing when static evaluation is unavailable
                    if value_type.is_none() {
                        let declared_value_type =
                            self.infer_expression(&mut ctx.reborrow(), *value, state)?;

                        // unresolved generic projections are static but require convergence
                        let declared_type_requires_convergence = self
                            .type_requires_static_evaluation_convergence(
                                ctx.type_view(),
                                declared_value_type,
                            );
                        if static_value.is_none() && !declared_type_requires_convergence {
                            self.error(AnalyzeError::InvalidComptimeExpression {
                                node: value
                                    .into_global_any(ctx.module.id)
                                    .into_anchored(Some(ctx.profile)),
                            });

                            let error_type_id = ctx
                                .types
                                .insert_type_from_any(Type::Error, (*value).into_any());
                            ctx.types.set_value_type(member_symbol, error_type_id);
                            return Ok(());
                        }

                        value_type = Some(declared_value_type);
                    }

                    let Some(value_type) = value_type else {
                        return Ok(());
                    };
                    ctx.types.set_value_type(member_symbol, value_type);
                    Some(value_type)
                } else {
                    None
                };

                // validate initializer against annotation
                if let (Some(constraint_type), Some(value_type)) = (constraint_type, value_type) {
                    // defer relation checks until the initializer type is stable enough
                    if matches!(
                        ctx.types.get_type(value_type),
                        Type::Unevaluated(_) | Type::Error
                    ) {
                        return Ok(());
                    }

                    let value_requires_convergence = self
                        .type_requires_static_evaluation_convergence(ctx.type_view(), value_type);
                    let constraint_requires_convergence = self
                        .type_requires_static_evaluation_convergence(
                            ctx.type_view(),
                            constraint_type,
                        );
                    if value_requires_convergence || constraint_requires_convergence {
                        return Ok(());
                    }

                    let is_assignable = self.is_type_assignable(
                        &mut ctx.type_context_reborrow(),
                        constraint_type,
                        value_type,
                    );
                    if is_assignable == Assignability::NotAssignable {
                        self.emit_unassignable_type_for_types(
                            ctx.module_type_view(),
                            member_id.into_any(),
                            constraint_type,
                            value_type,
                        );
                    }
                }

                Ok(())
            }
            Member::Field {
                key,
                declared_type,
                default,
                is_optional,
                is_readonly,
                is_static,
                ..
            } => {
                // resolve the member symbol
                let member_symbol = member.symbol().into_global(ctx.module.id);

                // infer computed keys first
                if let Key::Expression(key_expression) = key {
                    self.infer_expression(&mut ctx.reborrow(), *key_expression, state)?;
                }

                // evaluate the declared field type once and reuse the declared result in infer
                let value_ty_id = if let Some(declared_type) = declared_type {
                    Some(self.resolve_declared_type_expression(
                        &mut ctx.type_context_reborrow(),
                        *declared_type,
                        true,
                        true,
                    )?)
                } else {
                    None
                };

                // infer the default value when present
                let default_ty_id = if let Some(default) = default {
                    Some(self.infer_expression(&mut ctx.reborrow(), *default, state)?)
                } else {
                    None
                };

                // update the value shape for inferred static fields
                let static_key = self.static_key_from_key(
                    ctx.compiler_context.revision(),
                    ctx.profile,
                    ctx.tree,
                    ctx.symbols,
                    ctx.types,
                    *key,
                );
                if *is_static
                    && value_ty_id.is_none()
                    && let Some(default_ty_id) = default_ty_id
                    && let Some(static_key) = static_key
                    && let Some(owner_symbol) = state.in_nominal_symbol
                {
                    let field = TypeField {
                        key: static_key,
                        ty: default_ty_id,
                        is_optional: *is_optional,
                        is_readonly: *is_readonly,
                    };
                    self.update_value_shape_with_field(
                        member_id.into_any(),
                        owner_symbol,
                        field,
                        ctx.types,
                    );
                }

                // infer member types from defaults when no annotation exists
                if value_ty_id.is_none()
                    && let Some(default_ty_id) = default_ty_id
                    && ctx.types.get_value_type_id(member_symbol).is_none()
                {
                    ctx.types.set_value_type(member_symbol, default_ty_id);
                }

                // attach declared member types when available
                if let Some(value_ty_id) = value_ty_id
                    && ctx.types.get_value_type_id(member_symbol).is_none()
                {
                    ctx.types.set_value_type(member_symbol, value_ty_id);
                }

                // seed an inference variable when no value type metadata exists
                if ctx.types.get_value_type_id(member_symbol).is_none() {
                    let scope = InferScope {
                        owner: member_symbol,
                        function_id: state
                            .in_function
                            .map(|function_id| function_id.into_global(ctx.module.id)),
                    };
                    let placeholder_ty_id = self.infer_var_type_for_symbol(
                        ctx.infer,
                        ctx.types,
                        member_symbol,
                        member_id.into_any(),
                        InferOrigin::Expression(member_id.into_global_any(ctx.module.id)),
                        scope,
                    );
                    ctx.types.set_value_type(member_symbol, placeholder_ty_id);
                }

                Ok(())
            }
            Member::Method {
                key: _,
                signature,
                body,
                ..
            } => {
                // resolve the member symbol for value typing
                let member_symbol = member.symbol().into_global(ctx.module.id);

                // apply decorator options for this method
                let method_options = {
                    let symbol = ctx.symbols.get_symbol(member.symbol());
                    state.options.with_symbol_decorators(&symbol.decorators)
                };

                // assign the implicit this binding type when available
                if let Some(this_ty_id) = this_ty_id {
                    let this_name = self.repository.strings.intern("this");
                    let (_scope_id, scope, _mark) = ctx.symbols.get_scope(member_id, ctx.tree);
                    if let Some(this_symbol) = ctx
                        .symbols
                        .find_active_symbol(scope, StaticKey::Name(this_name))
                    {
                        ctx.types
                            .set_value_type(this_symbol.into_global(ctx.module.id), this_ty_id);
                    }
                }

                // enforce runtime constraints up front
                self.check_signature_runtime_constraints(
                    ctx,
                    member_id.into_any(),
                    signature,
                    method_options,
                );

                // infer the method signature
                let declared_signature_ty_id = ctx
                    .types
                    .get_signature_type_for_node(member_id.into_global_any(ctx.module.id));
                let mut signature_ctx = state.fork().with_options(method_options);
                let method_ty_id = if self.should_use_declared_signature(
                    ctx.type_view(),
                    signature,
                    declared_signature_ty_id,
                    None,
                ) {
                    let declared_signature_ty_id = self
                        .require_declared_signature_type_for_skipped_inference(
                            declared_signature_ty_id,
                        )?;
                    self.bind_declared_signature(
                        &mut ctx.reborrow(),
                        member_id.into_any(),
                        signature,
                        declared_signature_ty_id,
                        &mut signature_ctx,
                    )?
                } else {
                    let owner_symbol = member.symbol().into_global(ctx.module.id);
                    self.infer_signature(
                        &mut ctx.reborrow(),
                        member_id.into_any(),
                        owner_symbol,
                        signature,
                        None,
                        declared_signature_ty_id,
                        &mut signature_ctx,
                    )?
                };

                // attach the method type for member symbol lookups
                if let Some(accessor_value_ty_id) =
                    self.accessor_value_type_for_signature(signature, method_ty_id, ctx.types)
                {
                    ctx.types
                        .set_value_type(member_symbol, accessor_value_ty_id);
                } else if ctx.types.get_value_type_id(member_symbol).is_none() {
                    ctx.types.set_value_type(member_symbol, method_ty_id);
                }

                // prepare the return type for body inference
                let mut return_type = self.function_return_type(method_ty_id, ctx.types);
                if let Some(this_ty_id) = this_ty_id {
                    let mut cache = HashMap::new();

                    // substitute this in the explicit this parameter
                    if let Some(this_parameter_id) = signature.this_parameter {
                        let param_symbol = ctx
                            .tree
                            .get(this_parameter_id)
                            .symbol()
                            .into_global(ctx.module.id);
                        if let Some(param_ty_id) = ctx.types.get_value_type_id(param_symbol) {
                            let mapped_ty_id = self.substitute_this_type(
                                param_ty_id,
                                this_ty_id,
                                ctx.types,
                                &mut cache,
                            );
                            ctx.types.set_value_type(param_symbol, mapped_ty_id);
                        }
                    }

                    // substitute this in dynamic parameters
                    for parameter_id in signature.parameters.iter() {
                        let param_symbol = ctx
                            .tree
                            .get(*parameter_id)
                            .symbol()
                            .into_global(ctx.module.id);
                        if let Some(param_ty_id) = ctx.types.get_value_type_id(param_symbol) {
                            let mapped_ty_id = self.substitute_this_type(
                                param_ty_id,
                                this_ty_id,
                                ctx.types,
                                &mut cache,
                            );
                            ctx.types.set_value_type(param_symbol, mapped_ty_id);
                        }
                    }

                    // substitute this in the return type
                    return_type = return_type.map(|return_type| {
                        self.substitute_this_type(return_type, this_ty_id, ctx.types, &mut cache)
                    });
                }

                // infer the body when present
                if let Some(body) = body {
                    let state = state
                        .reset()
                        .without_const_context()
                        .with_options(method_options)
                        .in_function_with_signature(member_id.into_any(), signature);
                    let mut context_return_type = return_type;
                    let mut state = if signature.cardinality == FunctionCardinality::Generator {
                        let (yield_ty_id, return_ty_id, next_ty_id) = self.generator_context_types(
                            &mut ctx.type_context_reborrow(),
                            member_id.into_any(),
                            return_type,
                        );
                        context_return_type = Some(return_ty_id);
                        state
                            .with_return_type(Some(return_ty_id))
                            .with_generator_types(Some(yield_ty_id), Some(next_ty_id))
                    } else {
                        state.with_return_type(return_type)
                    };
                    state = state.with_expected_type(context_return_type);

                    // infer the method body with implicit return typing
                    let body_ty_id =
                        self.infer_expression(&mut ctx.reborrow(), *body, &mut state)?;

                    // commit inferred return types for widening
                    let committed_body_ty_id = self.materialize_inferred_return_type(
                        &mut ctx.reborrow(),
                        &state,
                        context_return_type,
                        body_ty_id,
                    );

                    // constrain implicit return types against the declared return type
                    if let Some(return_ty_id) = context_return_type
                        && has_implicit_return(*body, ctx.tree)
                        && !matches!(
                            ctx.types.get_type(return_ty_id),
                            Type::Predicate { asserts: true, .. }
                        )
                    {
                        ctx.infer.push_constraint(Constraint::Subtype {
                            sub_type: committed_body_ty_id,
                            super_type: return_ty_id,
                            variance: None,
                        });

                        let normalized_return_ty_id = self.normalize_type_for_assignability(
                            &mut ctx.type_context_reborrow(),
                            return_ty_id,
                        );

                        if !self.return_type_allows_fallthrough_infer(return_ty_id, ctx.types)
                            && !self.type_relation_requires_infer_convergence(
                                ctx.type_view(),
                                normalized_return_ty_id,
                                committed_body_ty_id,
                            )
                            && self.is_type_assignable(
                                &mut ctx.type_context_reborrow(),
                                normalized_return_ty_id,
                                committed_body_ty_id,
                            ) == Assignability::NotAssignable
                        {
                            self.emit_unassignable_type_for_types(
                                ctx.module_type_view(),
                                body.into_any(),
                                return_ty_id,
                                body_ty_id,
                            );
                        }
                    }
                }

                Ok(())
            }
            Member::Embed { value, .. } => {
                // #Incomplete: expand embedded type into member fields?
                self.resolve_declared_type_expression(
                    &mut ctx.type_context_reborrow(),
                    *value,
                    true,
                    true,
                )?;
                Ok(())
            }
            Member::StaticBlock { body, .. } => {
                self.infer_expression(&mut ctx.reborrow(), *body, state)?;
                Ok(())
            }
            Member::ComptimeBlock { body, .. } => {
                self.infer_expression(&mut ctx.reborrow(), *body, state)?;
                Ok(())
            }
            Member::Error { .. } => Ok(()),
        }
    }

    /// Infer where clauses when present.
    pub(crate) fn infer_where_clauses_maybe(
        &self,
        ctx: &mut InferContext<'_>,
        clauses: Option<&[LocalNodeId<WhereClause>]>,
        state: &mut InferState,
    ) -> AnalyzeResult<()> {
        // infer where clause expressions
        if let Some(clauses) = clauses {
            for clause_id in clauses {
                self.infer_where_clause(&mut ctx.reborrow(), *clause_id, state)?;
            }
        }
        Ok(())
    }

    /// Infer a function signature.
    pub(crate) fn infer_signature(
        &self,
        ctx: &mut InferContext<'_>,
        node_id: LocalNodeIdAny,
        owner_symbol: GlobalSymbolId,
        signature: &FunctionSignature,
        expected_fn_ty_id: Option<LocalTypeId>,
        declared_signature_ty_id: Option<LocalTypeId>,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        let module_options = *ctx.options;
        let enforce_decorator_no_managed = state.options.no_managed && !module_options.no_managed;

        // walk generics
        self.infer_where_clauses_maybe(
            &mut ctx.reborrow(),
            Some(signature.where_clauses.as_slice()),
            state,
        )?;

        // collect static parameter placeholders
        let static_parameters = self.static_parameter_placeholders_for_signature(
            &mut ctx.type_context_reborrow(),
            signature,
        );

        // extract any contextual function signature
        let expected_signature = self.expected_function_signature(expected_fn_ty_id, ctx.types);

        // collect parameter types
        let scope = InferScope {
            owner: owner_symbol,
            function_id: Some(node_id.into_global(ctx.module.id)),
        };

        // parameter initializers run before entering async or generator execution context
        let mut parameter_ctx = state.fork().is_async_maybe(false).is_generator_maybe(false);

        // this parameter
        let expected_this_ty_id = expected_signature
            .as_ref()
            .and_then(|signature| signature.this_parameter);
        let this_parameter = if let Some(this_parameter_id) = signature.this_parameter {
            // resolve any declared type for the this parameter
            let declared_ty_id = ctx
                .types
                .get_declared_type_id(this_parameter_id.into_global_any(ctx.module.id));
            let expected_ty_id = expected_this_ty_id;

            // report implicit this when no declared type exists
            if state.options.no_implicit_this
                && declared_ty_id.is_none()
                && expected_ty_id.is_none()
                && !matches!(ctx.module.source, ModuleSource::Builtin(_))
            {
                self.error(AnalyzeError::ImplicitThis {
                    node: this_parameter_id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile)),
                });
            }

            let param_symbol = ctx
                .tree
                .get(this_parameter_id)
                .symbol()
                .into_global(ctx.module.id);
            let param_ty_id = declared_ty_id.or(expected_ty_id).unwrap_or_else(|| {
                self.infer_var_type_for_symbol(
                    ctx.infer,
                    ctx.types,
                    param_symbol,
                    this_parameter_id.into_any(),
                    InferOrigin::Parameter(this_parameter_id.into_global_any(ctx.module.id)),
                    scope,
                )
            });
            self.infer_parameter(
                &mut ctx.reborrow(),
                this_parameter_id,
                Some(param_ty_id),
                &mut parameter_ctx,
            )?;
            ctx.types.set_value_type(param_symbol, param_ty_id);
            Some(param_ty_id)
        } else if signature.kind == FunctionKind::Lambda {
            // contextual "this" for lambdas
            if let Some(expected_this_ty_id) = expected_this_ty_id {
                let this_name = self.repository.strings.intern("this");
                let (_scope_id, scope, _mark) = match node_id.ty {
                    // use the lambda declaration scope
                    NodeType::Declaration => ctx
                        .symbols
                        .get_scope(node_id.into_typed::<Declaration>(), ctx.tree),
                    // use the method scope for member lambdas
                    NodeType::Member => ctx
                        .symbols
                        .get_scope(node_id.into_typed::<Member>(), ctx.tree),
                    // use the expression scope for inline lambdas
                    NodeType::Expression => ctx
                        .symbols
                        .get_scope(node_id.into_typed::<Expression>(), ctx.tree),
                    // fall back to declaration scopes for internal nodes
                    _ => ctx
                        .symbols
                        .get_scope(LocalNodeId::<Declaration>::new(node_id.id), ctx.tree),
                };
                if let Some(this_symbol) = ctx
                    .symbols
                    .find_active_symbol(scope, StaticKey::Name(this_name))
                {
                    ctx.types.set_value_type(
                        this_symbol.into_global(ctx.module.id),
                        expected_this_ty_id,
                    );
                }

                Some(expected_this_ty_id)
            } else {
                None
            }
        } else {
            None
        };

        // dynamic parameters
        let mut dynamic_param_types = Vec::with_capacity(signature.parameters.len());
        for (index, parameter_id) in signature.parameters.iter().enumerate() {
            // resolve declared, contextual, and default metadata
            let declared_ty_id = ctx
                .types
                .get_declared_type_id(parameter_id.into_global_any(ctx.module.id));
            let expected_param_ty_id = expected_signature
                .as_ref()
                .and_then(|signature| signature.dynamic_parameters.get(index).copied());
            let has_default = match ctx.tree.get(*parameter_id) {
                Parameter::Named { default, .. } => default.is_some(),
                Parameter::Pattern { default, .. } => default.is_some(),
                Parameter::VariadicNamed { .. }
                | Parameter::VariadicPattern { .. }
                | Parameter::Error { .. } => false,
            };

            let param_symbol = ctx
                .tree
                .get(*parameter_id)
                .symbol()
                .into_global(ctx.module.id);

            // report implicit any when no type info is available
            self.report_implicit_any_for_parameter(
                &ctx.type_context_reborrow(),
                *parameter_id,
                param_symbol,
                declared_ty_id,
                expected_param_ty_id,
                has_default,
            );

            // select the parameter type or fall back to inference
            let param_ty_id = declared_ty_id.or(expected_param_ty_id).unwrap_or_else(|| {
                self.infer_var_type_for_symbol(
                    ctx.infer,
                    ctx.types,
                    param_symbol,
                    parameter_id.into_any(),
                    InferOrigin::Parameter(parameter_id.into_global_any(ctx.module.id)),
                    scope,
                )
            });

            self.infer_parameter(
                &mut ctx.reborrow(),
                *parameter_id,
                Some(param_ty_id),
                &mut parameter_ctx,
            )?;

            let resolved_param_ty_id = ctx
                .types
                .get_value_type_id(param_symbol)
                .unwrap_or(param_ty_id);
            ctx.types.set_value_type(param_symbol, resolved_param_ty_id);
            dynamic_param_types.push(resolved_param_ty_id);
        }

        // return type
        let return_type_node_id = signature.return_type;
        let declared_return_type = declared_signature_ty_id
            .and_then(|signature_id| self.function_return_type(signature_id, ctx.types));
        let has_concrete_declared_return = declared_return_type.is_some_and(|ty_id| {
            !self.is_infer_var_type(ty_id, ctx.types)
                && !matches!(
                    ctx.types.get_type(ty_id),
                    Type::TypeLiteral {
                        value: TypeLiteral::Unknown
                    }
                )
        });
        let return_type = if let Some(return_type_node_id) = return_type_node_id {
            Some(self.resolve_declared_type_expression(
                &mut ctx.type_context_reborrow(),
                return_type_node_id,
                true,
                true,
            )?)
        } else if let Some(return_type) =
            expected_signature.and_then(|signature| signature.return_type)
        {
            Some(return_type)
        } else if has_concrete_declared_return {
            declared_return_type
        } else {
            Some(self.infer_var_type_for_node(
                ctx.infer,
                ctx.types,
                node_id.into_global(ctx.module.id),
                InferOrigin::Return(node_id.into_global(ctx.module.id)),
                scope,
            ))
        };

        // enforce no-managed decorators on signature types
        if enforce_decorator_no_managed {
            self.check_no_managed_signature(
                &mut ctx.type_context_reborrow(),
                signature,
                this_parameter,
                &dynamic_param_types,
                return_type,
                return_type_node_id,
            )?;
        }

        // build the function type for this signature
        let ty = Type::Function {
            asynchrony: signature.asynchrony,
            cardinality: signature.cardinality,
            dynamic_parameters: dynamic_param_types,
            static_parameters,
            this_parameter,
            return_type,
        };
        let ty_id = if let Some(declared_ty_id) = declared_signature_ty_id {
            // update the declared signature type in place
            ctx.types.update_type(declared_ty_id, ty);
            declared_ty_id
        } else {
            ctx.types.insert_type_from_any(ty, node_id)
        };

        // record signature type for lowering
        if !state.is_surface_inference {
            ctx.infer
                .set_inferred_type_for_node(node_id.into_global(ctx.module.id), ty_id);
            ctx.types
                .set_inferred_type(node_id.into_global(ctx.module.id), ty_id);
        }

        Ok(ty_id)
    }

    /// Bind declared signature types without inference.
    pub(crate) fn bind_declared_signature(
        &self,
        ctx: &mut InferContext<'_>,
        node_id: LocalNodeIdAny,
        signature: &FunctionSignature,
        declared_signature_ty_id: LocalTypeId,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        // infer where clauses to validate constraints
        self.infer_where_clauses_maybe(
            &mut ctx.reborrow(),
            Some(signature.where_clauses.as_slice()),
            state,
        )?;

        let this_parameter = if let Some(this_parameter_id) = signature.this_parameter {
            let declared_ty_id = ctx
                .types
                .get_declared_type_id(this_parameter_id.into_global_any(ctx.module.id))
                .unwrap_or_else(|| {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    ctx.types.insert_type_from(ty, this_parameter_id)
                });
            let param_symbol = ctx
                .tree
                .get(this_parameter_id)
                .symbol()
                .into_global(ctx.module.id);
            ctx.types.set_value_type(param_symbol, declared_ty_id);
            Some(declared_ty_id)
        } else {
            None
        };

        let mut dynamic_param_types = Vec::with_capacity(signature.parameters.len());
        for parameter_id in signature.parameters.iter() {
            let declared_ty_id = ctx
                .types
                .get_declared_type_id(parameter_id.into_global_any(ctx.module.id))
                .unwrap_or_else(|| {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    ctx.types.insert_type_from(ty, *parameter_id)
                });
            let param_symbol = ctx
                .tree
                .get(*parameter_id)
                .symbol()
                .into_global(ctx.module.id);
            ctx.types.set_value_type(param_symbol, declared_ty_id);

            if matches!(ctx.tree.get(*parameter_id), Parameter::Pattern { .. }) {
                self.infer_parameter(
                    &mut ctx.reborrow(),
                    *parameter_id,
                    Some(declared_ty_id),
                    state,
                )?;
            }

            dynamic_param_types.push(declared_ty_id);
        }

        // resolve return type
        let return_type_node_id = signature.return_type;
        let return_type = return_type_node_id
            .and_then(|return_type_node_id| {
                ctx.types
                    .get_declared_type_id(return_type_node_id.into_global_any(ctx.module.id))
            })
            .or_else(|| self.function_return_type(declared_signature_ty_id, ctx.types));

        // enforce no-managed decorators on signature types
        let module_options = *ctx.options;
        let enforce_decorator_no_managed = state.options.no_managed && !module_options.no_managed;
        if enforce_decorator_no_managed {
            self.check_no_managed_signature(
                &mut ctx.type_context_reborrow(),
                signature,
                this_parameter,
                &dynamic_param_types,
                return_type,
                return_type_node_id,
            )?;
        }

        // record signature type for lowering
        if !state.is_surface_inference {
            ctx.infer.set_inferred_type_for_node(
                node_id.into_global(ctx.module.id),
                declared_signature_ty_id,
            );
            ctx.types
                .set_inferred_type(node_id.into_global(ctx.module.id), declared_signature_ty_id);
        }

        Ok(declared_signature_ty_id)
    }

    /// Enforce no-runtime constraints for a signature.
    pub(crate) fn check_signature_runtime_constraints(
        &self,
        ctx: &InferContext<'_>,
        node_id: LocalNodeIdAny,
        signature: &FunctionSignature,
        options: AnalyzeOptions,
    ) {
        if options.no_runtime
            && matches!(ctx.module.source, ModuleSource::User)
            && (signature.asynchrony == Asynchrony::Async
                || signature.cardinality == FunctionCardinality::Generator)
        {
            self.error(AnalyzeError::RuntimeDisabled {
                node: node_id
                    .into_global(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            });
        }
    }

    /// Check whether a signature is fully declared without defaults.
    fn signature_is_fully_declared(
        &self,
        ctx: TypeView<'_>,
        signature: &FunctionSignature,
    ) -> bool {
        // require explicit return type
        let Some(return_type_node_id) = signature.return_type else {
            return false;
        };

        // require declared return type
        if ctx
            .types
            .get_declared_type_id(return_type_node_id.into_global_any(ctx.module.id))
            .is_none()
        {
            return false;
        }

        // require declared `this` parameter type when present
        if let Some(this_parameter_id) = signature.this_parameter
            && ctx
                .types
                .get_declared_type_id(this_parameter_id.into_global_any(ctx.module.id))
                .is_none()
        {
            return false;
        }

        // require declared parameter types without defaults
        for parameter_id in signature.parameters.iter() {
            let parameter = ctx.tree.get(*parameter_id);
            let has_default = match parameter {
                Parameter::Named { default, .. } | Parameter::Pattern { default, .. } => {
                    default.is_some()
                }
                Parameter::VariadicNamed { .. }
                | Parameter::VariadicPattern { .. }
                | Parameter::Error { .. } => false,
            };
            if has_default {
                return false;
            }
            if ctx
                .types
                .get_declared_type_id(parameter_id.into_global_any(ctx.module.id))
                .is_none()
            {
                return false;
            }
        }

        true
    }

    // no managed enforcement
    /// Enforce no-managed decorators on function signatures.
    fn check_no_managed_signature(
        &self,
        ctx: &mut TypeContext<'_>,
        signature: &FunctionSignature,
        this_parameter: Option<LocalTypeId>,
        dynamic_param_types: &[LocalTypeId],
        return_type: Option<LocalTypeId>,
        return_type_node_id: Option<LocalNodeId<TypeExpression>>,
    ) -> AnalyzeResult<()> {
        // only enforce for user modules
        if !matches!(ctx.module.source, ModuleSource::User) {
            return Ok(());
        }
        let module_id = ctx.module.id;

        // enforce this parameter types when present
        if let Some(this_parameter_id) = signature.this_parameter
            && let Some(this_ty_id) = this_parameter
        {
            self.check_no_managed_signature_type(
                &mut ctx.reborrow(),
                this_parameter_id.into_global_any(module_id),
                this_ty_id,
            )?;
        }

        // enforce dynamic parameter types
        for (index, parameter_id) in signature.parameters.iter().enumerate() {
            let Some(param_ty_id) = dynamic_param_types.get(index).copied() else {
                continue;
            };
            self.check_no_managed_signature_type(
                &mut ctx.reborrow(),
                parameter_id.into_global_any(module_id),
                param_ty_id,
            )?;
        }

        // enforce return type when declared
        if let (Some(return_type_node_id), Some(return_type_id)) =
            (return_type_node_id, return_type)
        {
            self.check_no_managed_signature_type(
                &mut ctx.reborrow(),
                return_type_node_id.into_global_any(module_id),
                return_type_id,
            )?;
        }

        Ok(())
    }

    /// Enforce no-managed decorators on a single signature type.
    fn check_no_managed_signature_type(
        &self,
        ctx: &mut TypeContext<'_>,
        node_id: GlobalNodeIdAny,
        ty_id: LocalTypeId,
    ) -> AnalyzeResult<()> {
        // resolve unevaluated types before checking managed usage
        if matches!(ctx.types.get_type(ty_id), Type::Unevaluated(_)) {
            self.resolve_declared_type(&mut ctx.reborrow(), ty_id)?;
        }

        // report managed types in signatures
        if self.type_contains_managed(ctx.module_type_view(), ty_id) {
            self.error(AnalyzeError::ManagedMemoryDisabled {
                node: node_id.into_anchored(Some(ctx.profile)),
            });
        }

        Ok(())
    }

    /// Infer a parameter.
    pub(crate) fn infer_parameter(
        &self,
        ctx: &mut InferContext<'_>,
        parameter_id: LocalNodeId<Parameter>,
        binding_ty_id: Option<LocalTypeId>,
        state: &mut InferState,
    ) -> AnalyzeResult<()> {
        // evaluate declared parameter types before use
        if let Some(binding_ty_id) = binding_ty_id
            && matches!(ctx.types.get_type(binding_ty_id), Type::Unevaluated(_))
        {
            self.resolve_declared_type(&mut ctx.type_context_reborrow(), binding_ty_id)?;
        }

        let parameter = ctx.tree.get(parameter_id);
        match parameter {
            Parameter::Named {
                name: _,
                visibility: _,
                is_readonly: _,
                is_optional: _,
                declared_type: _,
                default,
                symbol,
            } => {
                // bind parameter symbol to its type
                if let Some(binding_ty_id) = binding_ty_id {
                    ctx.types
                        .set_value_type(symbol.into_global(ctx.module.id), binding_ty_id);
                }

                // infer default expression and constrain to parameter type
                if let Some(default) = default {
                    let default_ty_id =
                        self.infer_expression(&mut ctx.reborrow(), *default, state)?;
                    let binding_ty_id = if let Some(binding_ty_id) = binding_ty_id
                        && self.is_infer_var_type(binding_ty_id, ctx.types)
                    {
                        let committed_ty_id = self.materialize_binding_type(
                            ctx.module,
                            state,
                            default_ty_id,
                            ctx.types,
                            false,
                        );
                        ctx.types
                            .set_value_type(symbol.into_global(ctx.module.id), committed_ty_id);
                        Some(committed_ty_id)
                    } else {
                        binding_ty_id
                    };
                    if let Some(binding_ty_id) = binding_ty_id {
                        ctx.infer.push_constraint(Constraint::Subtype {
                            sub_type: default_ty_id,
                            super_type: binding_ty_id,
                            variance: None,
                        });
                    }
                }
            }
            Parameter::Pattern {
                pattern,
                is_optional: _,
                declared_type: _,
                default,
                symbol,
            } => {
                // bind parameter symbol to its type
                if let Some(binding_ty_id) = binding_ty_id {
                    ctx.types
                        .set_value_type(symbol.into_global(ctx.module.id), binding_ty_id);
                }

                // infer default expression and pick a binding type
                let default_ty_id = if let Some(default) = default {
                    Some(self.infer_expression(&mut ctx.reborrow(), *default, state)?)
                } else {
                    None
                };
                let binding_ty_id = binding_ty_id.or(default_ty_id);
                let binding_ty_id = if let Some(binding_ty_id) = binding_ty_id
                    && let Some(default_ty_id) = default_ty_id
                    && self.is_infer_var_type(binding_ty_id, ctx.types)
                {
                    let committed_ty_id = self.materialize_binding_type(
                        ctx.module,
                        state,
                        default_ty_id,
                        ctx.types,
                        false,
                    );
                    ctx.types
                        .set_value_type(symbol.into_global(ctx.module.id), committed_ty_id);
                    Some(committed_ty_id)
                } else {
                    binding_ty_id
                };

                // constrain default to the binding type
                if let (Some(default_ty_id), Some(binding_ty_id)) = (default_ty_id, binding_ty_id) {
                    ctx.infer.push_constraint(Constraint::Subtype {
                        sub_type: default_ty_id,
                        super_type: binding_ty_id,
                        variance: None,
                    });
                }

                // infer bindings within the pattern
                self.infer_pattern(&mut ctx.reborrow(), *pattern, binding_ty_id, state)?;
            }
            Parameter::VariadicNamed {
                name: _,
                visibility: _,
                is_readonly: _,
                declared_type: _,
                symbol,
            } => {
                // bind variadic parameter symbol to its type
                if let Some(binding_ty_id) = binding_ty_id {
                    ctx.types
                        .set_value_type(symbol.into_global(ctx.module.id), binding_ty_id);
                }
            }
            Parameter::VariadicPattern {
                pattern: _,
                declared_type: _,
                symbol,
            } => {
                if let Some(binding_ty_id) = binding_ty_id {
                    ctx.types
                        .set_value_type(symbol.into_global(ctx.module.id), binding_ty_id);
                }
            }
            Parameter::Error { .. } => {}
        }
        Ok(())
    }

    /// Infer a dependency item.
    ///
    /// For imports from data/text/binary modules, this infers the appropriate type
    /// for the local binding symbol.
    pub(crate) fn infer_dependency_item(
        &self,
        ctx: &mut InferContext<'_>,
        item_id: LocalNodeId<DependencyItem>,
        _state: &mut InferState,
    ) -> AnalyzeResult<()> {
        let item = ctx.tree.get(item_id);
        match item {
            DependencyItem::UnresolvedRemote { .. } => {
                // nothing to do
            }
            DependencyItem::UnresolvedLocal { .. } => {
                // nothing to do
            }
            DependencyItem::Value { .. } => {
                // nothing to do
            }
            DependencyItem::Local { .. } => {
                // nothing to do
            }
            DependencyItem::Remote {
                kind,
                target_symbol,
                symbol,
                ..
            } => {
                // infer remote value imports from non code module targets
                let target = ctx.compiler_context.module(target_symbol.module_id);
                let target = target.as_ref();

                // infer from non code module targets
                if *kind == DependencyKind::Value && !target.is_code() {
                    let ty_id =
                        self.infer_data_module_type(target, item_id.into_any(), ctx.types)?;

                    // set the type on the remote target symbol
                    ctx.types.set_value_type(*target_symbol, ty_id);

                    // mirror the type onto the local alias symbol when present
                    if let Some(symbol) = symbol {
                        ctx.types
                            .set_value_type(symbol.into_global(ctx.module.id), ty_id);
                    }
                }
            }
            DependencyItem::Error => {
                // malformed dependency items do not infer local declaration facts
            }
        }
        Ok(())
    }

    /// Infer the type for a data/text/binary module import.
    fn infer_data_module_type(
        &self,
        target_module: &Module,
        source_node: LocalNodeIdAny,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        use crate::analyze::r#type::json_value_to_type;

        if target_module.loader.is_data() {
            let data = self
                .data(target_module.id)
                .ok_or_else(|| AnalyzeError::Internal {
                    message: format!(
                        "missing data artifact for data module {:?}",
                        target_module.id
                    ),
                })?;
            let value = match data.as_ref() {
                Data::Json(value) => value,
                Data::Html(_) | Data::Css(_) => {
                    return Err(AnalyzeError::Internal {
                        message: format!(
                            "expected json data payload for module {:?}",
                            target_module.id
                        ),
                    });
                }
            };
            // infer structural type from JSON value
            return Ok(json_value_to_type(
                value,
                source_node,
                types,
                &self.repository.strings,
            ));
        }

        // otherwise text and file imports are always string
        if target_module.loader.is_text() || target_module.loader.is_file() {
            Ok(types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::String),
                },
                source_node,
            ))
        }
        // otherwise binary imports are uint8[] (Uint8Array on JS targets)
        else if target_module.loader.is_binary() {
            let element_type = types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Uint8)),
                },
                source_node,
            );
            Ok(types.insert_type_from_any(
                Type::Array {
                    element: Some(element_type),
                    is_readonly: false,
                },
                source_node,
            ))
        } else {
            debug_assert!(target_module.is_code());

            unreachable!("code modules are handled by analyze_module_infer");
        }
    }

    /// Infer where clause.
    pub(crate) fn infer_where_clause(
        &self,
        ctx: &mut InferContext<'_>,
        clause_id: LocalNodeId<WhereClause>,
        _state: &mut InferState,
    ) -> AnalyzeResult<()> {
        let clause = ctx.tree.get(clause_id);
        let constraint_ty_id = self.resolve_declared_type_expression(
            &mut ctx.type_context_reborrow(),
            clause.right,
            true,
            true,
        )?;
        let Some(parameter_symbol) =
            self.static_parameter_symbol_for_where_clause(ctx.tree_symbol_view(), clause_id)
        else {
            self.error(AnalyzeError::InvalidStaticArgument {
                node: clause_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
                message: "where clause must reference a static parameter".to_string(),
            });
            return Ok(());
        };

        // merge the constraint for static argument validation
        let constraint_ty_id = if let Some(existing_id) = ctx
            .types
            .get_static_parameter_constraint_type(parameter_symbol)
        {
            let merged_id = self.intersection_type_from_list(
                vec![existing_id, constraint_ty_id],
                existing_id,
                ctx.types,
            );
            ctx.types
                .set_static_parameter_constraint_type(parameter_symbol, merged_id);
            merged_id
        } else {
            ctx.types
                .set_static_parameter_constraint_type(parameter_symbol, constraint_ty_id);
            constraint_ty_id
        };

        // apply constraint to the parameter when no annotation exists
        let symbol_entry = ctx.symbols.get_symbol(parameter_symbol.local_id);
        if symbol_entry.is_static_parameter()
            && let Some(primary) = symbol_entry.primary_declaration
            && primary.local_id.ty == NodeType::Parameter
        {
            let existing_id = ctx.types.get_declared_type_id(primary);
            let should_override = existing_id.is_none()
                || existing_id.is_some_and(|ty_id| {
                    matches!(
                        ctx.types.get_type(ty_id),
                        Type::TypeLiteral {
                            value: TypeLiteral::Unknown | TypeLiteral::Any
                        }
                    )
                });
            if should_override {
                ctx.types.set_declared_type(primary, constraint_ty_id);
            }
        }

        // enforce the constraint during inference
        let parameter_ty_id = ctx.types.insert_type_from(
            Type::Reference {
                symbol: parameter_symbol,
                static_arguments: None,
            },
            clause_id,
        );
        ctx.infer.push_constraint(Constraint::Subtype {
            sub_type: parameter_ty_id,
            super_type: constraint_ty_id,
            variance: None,
        });

        Ok(())
    }

    /// Resolve a where clause parameter to a static parameter symbol.
    fn static_parameter_symbol_for_where_clause(
        &self,
        ctx: TreeSymbolView<'_>,
        clause_id: LocalNodeId<WhereClause>,
    ) -> Option<GlobalSymbolId> {
        // capture the parameter name as a static key
        let clause = ctx.tree.get(clause_id);
        let key = StaticKey::Name(clause.left);
        let mut scope = ctx.symbols.get_scope(clause_id, ctx.tree);

        loop {
            // search the active bindings in the current scope
            let limit = scope.2.0 as usize;
            let named_symbols = &scope.1.named_symbols;
            let limit = limit.min(named_symbols.len());
            for (candidate_key, symbol_id) in named_symbols[..limit].iter().rev() {
                if *candidate_key != key {
                    continue;
                }

                // skip inactive symbols
                let symbol = ctx.symbols.get_symbol(*symbol_id);
                if !symbol.is_active() {
                    continue;
                }

                // accept static parameters from type-capable spaces
                let is_type_space =
                    matches!(symbol.space, SymbolSpace::Type | SymbolSpace::TypeValue);
                if is_type_space && symbol.is_static_parameter() {
                    return Some(symbol_id.into_global(ctx.module.id));
                }
            }

            // walk to the parent scope when present
            let Some((parent_scope_id, parent_mark)) = scope.1.parent else {
                break;
            };
            scope = (
                parent_scope_id,
                ctx.symbols.get_scope_by_id(parent_scope_id),
                parent_mark,
            );
        }

        None
    }

    /// Infer a pattern, given an optional binding type of the pattern.
    pub(crate) fn infer_declarator(
        &self,
        ctx: &mut InferContext<'_>,
        declarator_id: LocalNodeId<Declarator>,
        _let_expression_id: LocalNodeId<Expression>,
        constraint: DeclaratorConstraint,
        state: &mut InferState,
    ) -> AnalyzeResult<()> {
        // capture the declarator node
        let declarator = ctx.tree.get(declarator_id);
        let Declarator {
            pattern,
            ty: _ty,
            value,
        } = declarator;

        // infer type from value or annotation
        // declared type is now on the declarator node, not the let expression
        let declared_annotation_ty_id = ctx
            .types
            .get_declared_type_id(declarator_id.into_global_any(ctx.module.id));
        let mut declared_relation_ty_id = declared_annotation_ty_id;

        // report implicit any when no annotation or initializer exists
        self.report_implicit_any_for_declarator(
            &ctx.type_context_reborrow(),
            declarator_id,
            declared_annotation_ty_id,
            value.is_some(),
        );

        // evaluate and prepare declared types before inference
        if let Some(initial_declared_ty_id) = declared_annotation_ty_id {
            self.resolve_declared_type(&mut ctx.type_context_reborrow(), initial_declared_ty_id)?;
            self.ensure_reference_instance_types_for_type(
                &mut ctx.type_context_reborrow(),
                declarator_id.into_any(),
                initial_declared_ty_id,
            )?;

            // register annotation-level instances on the reference type node
            if let Some((symbol, static_arguments, source_id)) =
                self.unwrap_type_symbol(ctx.types, initial_declared_ty_id)
            {
                let source_node_id = source_id.into_global(ctx.module.id);
                let _ = self.record_reference_provisional_instance(
                    &mut ctx.reborrow(),
                    source_node_id,
                    symbol,
                    static_arguments.as_deref(),
                )?;
            }

            // preserve declared relations as authored annotations
            declared_relation_ty_id = Some(initial_declared_ty_id);
        }

        let inferred_ty_id = if let Some(value) = value {
            // apply declared type as the expected type when available
            let mut value_ctx = if let Some(declared_relation_ty_id) = declared_relation_ty_id {
                state
                    .fork()
                    .with_expected_type(Some(declared_relation_ty_id))
            } else {
                state.fork()
            };
            Some(self.infer_expression(&mut ctx.reborrow(), *value, &mut value_ctx)?)
        } else {
            None
        };

        // nominal annotations require explicit conformance from tagged initializers
        if let (Some(declared_ty_id), Some(value_id)) = (declared_annotation_ty_id, value) {
            // resolve the declared nominal symbol
            let declared_symbol =
                self.unwrap_type_symbol(ctx.types, declared_ty_id)
                    .map(|(symbol, _, _)| {
                        self.canonical_symbol_id(
                            ctx.module_symbol_view(),
                            symbol,
                            CanonicalSymbolMode::PreserveAliases,
                        )
                    });

            let declared_is_nominal_interface = declared_symbol.is_some_and(|declared_symbol| {
                self.symbol_is_nominal_interface(ctx.tree_symbol_view(), declared_symbol)
            });

            // enforce explicit construction and conformance for nominal targets
            let tagged_initializer =
                self.tagged_initializer_reference(&mut ctx.type_context_reborrow(), *value_id)?;

            if let Some(declared_symbol) = declared_symbol
                && (declared_symbol.ty() == SymbolType::Newtype || declared_is_nominal_interface)
                && let Some((tagged_symbol, tagged_initializer_ty_id)) = tagged_initializer
            {
                // resolve the tagged constructor symbol for nominal comparison
                let tagged_canonical_symbol = self.canonical_symbol_id(
                    ctx.module_symbol_view(),
                    tagged_symbol,
                    CanonicalSymbolMode::PreserveAliases,
                );
                let same_nominal_symbol = tagged_canonical_symbol == declared_symbol;
                let satisfies_declared_lineage = declared_is_nominal_interface
                    && self.is_type_lineage_assignable(
                        ctx.symbol_type_view(),
                        tagged_canonical_symbol,
                        declared_symbol,
                    );
                let actual_initializer_ty_id = tagged_initializer_ty_id;

                // reject implicit wrapping through constructors outside the declared nominal symbol
                if !same_nominal_symbol
                    && !satisfies_declared_lineage
                    && let Some(error) = self.unassignable_type_error_for_types(
                        ctx.module_type_view(),
                        declarator_id.into_any(),
                        declared_ty_id,
                        actual_initializer_ty_id,
                    )
                {
                    return Err(error);
                }
            }
        }

        // commit binding types for inferred values without annotations
        let binding_ty_id = declared_annotation_ty_id.or(inferred_ty_id);
        let committed_binding_ty_id = if declared_annotation_ty_id.is_none() {
            if let (Some(binding_ty_id), Some(_)) = (binding_ty_id, value) {
                Some(self.materialize_declarator_initializer_type(
                    &mut ctx.type_context_reborrow(),
                    declarator_id,
                    binding_ty_id,
                    state,
                ))
            } else {
                binding_ty_id
            }
        } else {
            binding_ty_id
        };

        // assign direct binding value types from declared or inferred types
        if let Pattern::Binding {
            symbol, pattern, ..
        } = ctx.tree.get(*pattern)
            && pattern.is_none()
            && let Some(binding_ty_id) = committed_binding_ty_id
        {
            let binding_symbol = symbol.into_global(ctx.module.id);
            ctx.types.set_value_type(binding_symbol, binding_ty_id);

            if declared_annotation_ty_id.is_none()
                && let Some(value_id) = value
            {
                ctx.infer.upsert_direct_binding_value_commit_intent(
                    binding_symbol,
                    declarator_id,
                    *value_id,
                );
            }
        }

        // enforce explicit ownership when implicit managed values are disabled
        if let (Some(inferred_ty_id), Some(value_id)) = (inferred_ty_id, value) {
            if let Some(declared_relation_ty_id) = declared_relation_ty_id {
                self.check_no_implicit_managed_value(
                    ctx.module_type_view(),
                    *value_id,
                    declared_relation_ty_id,
                    inferred_ty_id,
                    ctx.tree,
                    &state.options,
                );
            } else {
                self.check_no_implicit_managed_inferred(
                    ctx.module_type_view(),
                    *value_id,
                    inferred_ty_id,
                    ctx.tree,
                    &state.options,
                );
            }
        }

        // type check: if both declared and inferred, check assignability
        if let (Some(declared), Some(inferred)) = (declared_relation_ty_id, inferred_ty_id) {
            let mut visited = HashSet::new();
            let skip_assignability = self.type_contains_error(declared, ctx.types, &mut visited)
                || self.type_contains_error(inferred, ctx.types, &mut visited);

            // resolve inference variables before assignability checks
            let resolved_declared = if self.is_infer_var_type(declared, ctx.types) {
                let (mut ctx, infer) = ctx.split_type_context_and_infer();
                self.resolve_infer_type_for_check(&mut ctx, declared, infer)
                    .unwrap_or(declared)
            } else {
                declared
            };
            let resolved_inferred = if self.is_infer_var_type(inferred, ctx.types) {
                let (mut ctx, infer) = ctx.split_type_context_and_infer();
                self.resolve_infer_type_for_check(&mut ctx, inferred, infer)
                    .unwrap_or(inferred)
            } else {
                inferred
            };
            // apply inference constraints when required
            if !skip_assignability && matches!(constraint, DeclaratorConstraint::Assignable) {
                ctx.infer.push_constraint(Constraint::Subtype {
                    sub_type: inferred,
                    super_type: declared,
                    variance: None,
                });
            }

            if !skip_assignability
                && !self.is_infer_var_type(resolved_declared, ctx.types)
                && !self.is_infer_var_type(resolved_inferred, ctx.types)
                && self.is_type_assignable(
                    &mut ctx.type_context_reborrow(),
                    resolved_declared,
                    resolved_inferred,
                ) == Assignability::NotAssignable
            {
                if matches!(constraint, DeclaratorConstraint::Assignable) {
                    if let Some(error) = self.unassignable_type_error_for_types(
                        ctx.module_type_view(),
                        declarator_id.into_any(),
                        resolved_declared,
                        resolved_inferred,
                    ) {
                        return Err(error);
                    }
                } else if let Some(error) = self.unsatisfied_type_error_for_types(
                    ctx.module_type_view(),
                    declarator_id.into_any(),
                    resolved_declared,
                    resolved_inferred,
                ) {
                    return Err(error);
                }
            }
        }

        // infer pattern bindings from declared or inferred type
        self.infer_pattern(
            &mut ctx.reborrow(),
            *pattern,
            committed_binding_ty_id,
            state,
        )?;

        // ensure direct bindings always record a value type
        if let Some(binding_ty_id) = binding_ty_id
            && let Pattern::Binding { symbol, .. } = ctx.tree.get(*pattern)
        {
            let binding_symbol = symbol.into_global(ctx.module.id);
            if ctx.types.get_value_type_id(binding_symbol).is_none() {
                ctx.types.set_value_type(binding_symbol, binding_ty_id);
            }
        }

        Ok(())
    }

    /// Resolve one tagged initializer authored reference from one value expression.
    fn tagged_initializer_reference(
        &self,
        ctx: &mut TypeContext<'_>,
        value_id: LocalNodeId<Expression>,
    ) -> AnalyzeResult<Option<(GlobalSymbolId, LocalTypeId)>> {
        let tag_expression_id = match ctx.tree.get(value_id) {
            Expression::TaggedScalarExpression { ty, .. }
            | Expression::TaggedTupleExpression { ty, .. }
            | Expression::TaggedObjectExpression { ty, .. } => *ty,
            _ => return Ok(None),
        };

        // prefer authored syntax references before declared type evaluation degrades them
        if let Some(target_symbol) = ctx.tree.get(tag_expression_id).target_symbol() {
            let target_symbol = self.resolve_type_reference_symbol(ctx, target_symbol);
            let generic_argument_nodes = ctx
                .tree
                .get(tag_expression_id)
                .generic_arguments()
                .map(|arguments| arguments.to_vec());
            let static_arguments = self.evaluate_generic_arguments(
                &mut ctx.reborrow(),
                generic_argument_nodes.as_deref(),
            )?;
            let authored_type = Type::Reference {
                symbol: target_symbol,
                static_arguments,
            };
            let authored_type_id = ctx
                .types
                .insert_type_from_any(authored_type, tag_expression_id.into_any());

            return Ok(Some((target_symbol, authored_type_id)));
        }

        // keep authored generic references intact for nominal diagnostics
        let declared_tag_type = self.resolve_declared_type_expression_value(
            &mut ctx.reborrow(),
            tag_expression_id,
            true,
            true,
            false,
            true,
            true,
        )?;
        let declared_tag_type_id = ctx
            .types
            .insert_type_from_any(declared_tag_type, tag_expression_id.into_any());
        let declared_tag_symbol = self
            .unwrap_type_symbol(ctx.types, declared_tag_type_id)
            .map(|(symbol, _, _)| symbol);

        Ok(declared_tag_symbol.map(|symbol| (symbol, declared_tag_type_id)))
    }

    /// Resolve the exposed property type for an accessor method signature.
    fn accessor_value_type_for_signature(
        &self,
        signature: &FunctionSignature,
        method_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        // getters expose their return type as the property value
        if signature.mode == Some(FunctionMode::Getter) {
            return self.function_return_type(method_ty_id, types);
        }

        // setters expose their first dynamic parameter type
        if signature.mode == Some(FunctionMode::Setter)
            && let Type::Function {
                dynamic_parameters, ..
            } = types.get_type(method_ty_id)
        {
            return dynamic_parameters.first().copied();
        }

        None
    }
}
