#![allow(clippy::too_many_arguments)]

use std::sync::Arc;

use destack_workspace::Ref;

pub(super) use crate::analyze::assign::Assignability;
pub(super) use crate::analyze::common::{
    AnalyzeIndex, CanonicalSymbolMode, ModuleSymbolView, ModuleTypeView, ObjectShape,
    SymbolTypeView, TypeContext,
};
pub(super) use crate::{
    AnalyzeError, AnalyzeOptions, Compiler, InferState, TestProgram, assert_string, assert_type,
    expect_let_declarator_by_name, root_expression_id, run_to_completion,
};
pub(super) use destack_core::StringId;
pub(super) use destack_dir::{
    Argument, BinaryOperator, Declaration, Declarator, EnumFieldValue, Expression, ExtensionKind,
    FlowEdgeKind, FlowGraphBuilder, GlobalNodeIdAny, GlobalSymbolId, IfCondition, IfKind, IntType,
    Key, LocalNodeId, LocalScopeMark, LocalTypeId, MatchCase, MatchSelector, Member, NodeTree,
    Pattern, PatternField, PrimitiveType, ScalarLiteral, StaticArgument, StaticExpression,
    StaticKey, SymbolKind, SymbolTable, SymbolType, Type, TypeField, TypeLiteral, TypeTable,
};
pub(super) use destack_source::ModuleId;
pub(super) use destack_workspace::{CompilerOptions, Module, ProfileId};
pub(super) use std::collections::{HashMap, HashSet};

/// Cached view of module ctx for tests.
pub(crate) struct TestModuleView<'a> {
    /// The owning test program.
    pub(super) test: &'a TestProgram,
    /// The module id under test.
    pub(super) module_id: ModuleId,
    /// The module root expressions.
    roots: Vec<LocalNodeId<Expression>>,
    /// The module node tree.
    tree: NodeTree,
    /// The module symbol table.
    symbols: SymbolTable,
    /// The module type table.
    types: TypeTable,
}

/// Resolve canonical symbol id in tests using module symbol view.
pub(super) fn canonical_symbol_id(
    compiler: &Compiler,
    module: &Module,
    symbols: &SymbolTable,
    profile: ProfileId,
    symbol: GlobalSymbolId,
    mode: CanonicalSymbolMode,
) -> GlobalSymbolId {
    let reference = Ref::for_workspace_root(compiler.repository.workspace_root());
    let revision = compiler
        .repository
        .current(&reference)
        .unwrap_or_else(|error| panic!("missing current workspace revision: {error}"));

    run_to_completion(compiler, revision, |compiler, context| {
        Ok::<_, AnalyzeError>(compiler.canonical_symbol_id(
            ModuleSymbolView::new(context, module, profile, symbols),
            symbol,
            mode,
        ))
    })
    .unwrap_or_else(|error| panic!("failed to canonicalize symbol in test harness: {error:?}"))
}

/// Check assignability in tests through a type context.
pub(super) fn is_type_assignable(
    compiler: &Compiler,
    module: &Module,
    profile: ProfileId,
    tree: &NodeTree,
    symbols: &SymbolTable,
    target_id: LocalTypeId,
    source_id: LocalTypeId,
    types: &mut TypeTable,
    options: &AnalyzeOptions,
) -> Assignability {
    let reference = Ref::for_workspace_root(compiler.repository.workspace_root());
    let revision = compiler
        .repository
        .current(&reference)
        .unwrap_or_else(|error| panic!("missing current workspace revision: {error}"));
    let compiler_context = compiler
        .context(revision)
        .unwrap_or_else(|error| panic!("{error}"));

    let mut ctx = TypeContext::new(
        &compiler_context,
        module,
        profile,
        options,
        tree,
        symbols,
        types,
        AnalyzeIndex::default(),
    );
    let revision = compiler
        .repository
        .current(&reference)
        .unwrap_or_else(|error| panic!("missing current workspace revision: {error}"));

    run_to_completion(compiler, revision, |compiler, _context| {
        Ok::<_, AnalyzeError>(compiler.is_type_assignable(&mut ctx, target_id, source_id))
    })
    .unwrap_or_else(|error| panic!("failed to compute test assignability: {error:?}"))
}

impl TestProgram {
    /// Add, analyze, and check a module in one step.
    pub(crate) fn analyze_module_with_source(&self, name: &str, source: &str) -> ModuleId {
        // register the module
        let module_id = self.add_module(name, source);

        // analyze and check diagnostics
        self.analyze_module_and_check_clean(module_id);

        module_id
    }

    /// Resolve a canonical symbol id for a module path.
    pub(crate) fn canonical_symbol_for_path(&self, module_uri: &str, path: &str) -> GlobalSymbolId {
        // resolve the module symbol
        let symbol = self
            .resolve_to_symbol(module_uri, path)
            .unwrap_or_else(|| panic!("expected symbol for {module_uri}:{path}"));

        // resolve the module state
        let module = self.module(module_uri);
        let module = module.as_ref();
        let profile = self.default_profile_id(module.id);
        let dir = self.artifact_dir(module.id, profile);
        let symbols = &dir.symbols;

        // resolve the canonical symbol id
        canonical_symbol_id(
            &self.compiler,
            module,
            symbols,
            profile,
            symbol,
            CanonicalSymbolMode::FollowAliases,
        )
    }

    /// Create a cached module view for tests.
    pub(crate) fn view(&self, module_id: ModuleId) -> TestModuleView<'_> {
        // load module state
        let profile = self.default_profile_id(module_id);
        let dir = self.artifact_dir(module_id, profile);

        // clone module dir data
        let roots = dir.roots.clone();
        let tree = dir.tree.clone();
        let symbols = dir.symbols.clone();
        let types = dir.types.clone();

        TestModuleView {
            test: self,
            module_id,
            roots,
            tree,
            symbols,
            types,
        }
    }

    /// Create a cached declared module view for tests.
    pub(crate) fn declared_view(&self, module_id: ModuleId) -> TestModuleView<'_> {
        let dir = self.dir_declared(module_id);

        TestModuleView {
            test: self,
            module_id,
            roots: Arc::unwrap_or_clone(dir.roots),
            tree: Arc::unwrap_or_clone(dir.tree),
            symbols: Arc::unwrap_or_clone(dir.symbols),
            types: Arc::unwrap_or_clone(dir.types),
        }
    }
}

impl<'a> TestModuleView<'a> {
    /// Read the roots for this view.
    pub(crate) fn roots(&self) -> &[LocalNodeId<Expression>] {
        &self.roots
    }

    /// Read the node tree for this view.
    pub(crate) fn tree(&self) -> &NodeTree {
        &self.tree
    }

    /// Read the symbol table for this view.
    pub(crate) fn symbols(&self) -> &SymbolTable {
        &self.symbols
    }

    /// Read the type table for this view.
    pub(crate) fn types(&self) -> &TypeTable {
        &self.types
    }

    /// Resolve the default profile id for this module.
    pub(crate) fn profile_id(&self) -> ProfileId {
        self.test.default_profile_id(self.module_id)
    }

    /// Resolve the root expression for the view.
    pub(crate) fn root_expression_id(&self, index: usize) -> LocalNodeId<Expression> {
        root_expression_id(&self.roots, &self.tree, index)
    }

    /// Resolve an inferred type for a local expression.
    pub(crate) fn expect_inferred_type(&self, expression_id: LocalNodeId<Expression>) -> &Type {
        // resolve inferred type
        self.types
            .get_inferred_type(expression_id.into_global_any(self.module_id))
            .expect("expected inferred type")
    }

    /// Resolve an inferred type id for a local expression.
    pub(crate) fn expect_inferred_type_id(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> LocalTypeId {
        // resolve inferred type id
        self.types
            .get_inferred_type_id(expression_id.into_global_any(self.module_id))
            .expect("expected inferred type id")
    }

    /// Resolve the declarator for a binding name.
    pub(crate) fn expect_let_declarator(&self, name: StringId) -> LocalNodeId<Declarator> {
        expect_let_declarator_by_name(&self.roots, &self.tree, name)
    }

    /// Resolve a binding symbol for a let declarator by name.
    pub(crate) fn expect_binding_symbol(&self, name: StringId) -> GlobalSymbolId {
        // resolve the declarator
        let declarator_id = self.expect_let_declarator(name);
        let declarator = self.tree.get(declarator_id);

        // extract the binding symbol
        match self.tree.get(declarator.pattern) {
            Pattern::Binding {
                symbol, pattern, ..
            } => {
                assert!(pattern.is_none());
                symbol.into_global(self.module_id)
            }
            other => panic!("expected binding pattern, got {other:?}"),
        }
    }

    /// Resolve the initializer value for a let declarator by name.
    pub(crate) fn expect_initializer(&self, name: StringId) -> LocalNodeId<Expression> {
        // resolve the declarator
        let declarator_id = self.expect_let_declarator(name);
        let declarator = self.tree.get(declarator_id);

        // return the initializer
        declarator
            .value
            .unwrap_or_else(|| panic!("expected initializer for {name:?}"))
    }

    /// Resolve the enum field symbol for a member name.
    pub(crate) fn expect_enum_field_symbol(&self, name: StringId) -> GlobalSymbolId {
        // scan enum declarations for the field
        for declaration_id in self.tree.iter_node_ids_of_type::<Declaration>() {
            let Declaration::Enum(declaration) = self.tree.get(declaration_id) else {
                continue;
            };

            // resolve the enum member scope once
            let enum_scope = self.symbols.get_scope_by_symbol(declaration.symbol);

            for field_id in &declaration.fields {
                let field = self.tree.get(*field_id);
                if field.name.string() == name {
                    let field_key = StaticKey::Name(field.name.string());
                    let field_symbol = self
                        .symbols
                        .find_active_symbol(enum_scope, field_key)
                        .unwrap_or_else(|| {
                            panic!("expected active enum field symbol for {name:?}")
                        });

                    return field_symbol.into_global(self.module_id);
                }
            }
        }

        panic!("expected enum field for {name:?}");
    }

    /// Resolve the struct field symbol for one struct and field name.
    pub(crate) fn expect_struct_field_symbol(
        &self,
        struct_name: StringId,
        field_name: StringId,
    ) -> GlobalSymbolId {
        // scan struct declarations for the requested field
        for declaration_id in self.tree.iter_node_ids_of_type::<Declaration>() {
            let Declaration::Struct(declaration) = self.tree.get(declaration_id) else {
                continue;
            };
            if declaration.name.string() != struct_name {
                continue;
            }

            for member_id in &declaration.members {
                let Member::Field {
                    key: Key::Name(name),
                    symbol,
                    ..
                } = self.tree.get(*member_id)
                else {
                    continue;
                };
                if name.string() == field_name {
                    return symbol.into_global(self.module_id);
                }
            }
        }

        panic!("expected struct field {field_name:?} on {struct_name:?}");
    }

    /// Resolve one declaration symbol by simple name from this exact view.
    pub(crate) fn expect_declaration_symbol(&self, name: StringId) -> GlobalSymbolId {
        for declaration_id in self.tree.iter_node_ids_of_type::<Declaration>() {
            let declaration = self.tree.get(declaration_id);
            let declaration_name = match declaration {
                Declaration::Global(_) => None,
                Declaration::Namespace(declaration) => Some(declaration.name.string()),
                Declaration::Type(declaration) => Some(declaration.name.string()),
                Declaration::ImportAlias(declaration) => Some(declaration.name.string()),
                Declaration::Struct(declaration) => Some(declaration.name.string()),
                Declaration::Class(declaration) => declaration.name.map(|value| value.string()),
                Declaration::Enum(declaration) => declaration.name.map(|value| value.string()),
                Declaration::Interface(declaration) => declaration.name.map(|value| value.string()),
                Declaration::Extension(declaration) => declaration.name.map(|value| value.string()),
                Declaration::Function(declaration) => declaration.name.map(|value| value.string()),
            };
            if declaration_name != Some(name) {
                continue;
            }

            return declaration.symbol().into_global(self.module_id);
        }

        panic!("expected declaration symbol for {name:?}");
    }

    /// Read a declared type id for a node.
    pub(crate) fn expect_declared_type_id(&self, node_id: GlobalNodeIdAny) -> LocalTypeId {
        self.types
            .get_declared_type_id(node_id)
            .expect("expected declared type id")
    }

    /// Read a value type id for a symbol.
    pub(crate) fn expect_value_type_id(&self, symbol: GlobalSymbolId) -> LocalTypeId {
        self.types
            .get_value_type_id(symbol)
            .expect("expected value type id")
    }

    /// Read an instance type id for a symbol.
    pub(crate) fn expect_instance_type_id(&self, symbol: GlobalSymbolId) -> LocalTypeId {
        self.types
            .get_instance_type_id(symbol)
            .expect("expected instance type id")
    }

    /// Resolve a member expression into its receiver and member key.
    pub(crate) fn expect_member_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> (LocalNodeId<Expression>, StringId) {
        match self.tree.get(expression_id) {
            Expression::Member { left, name, .. } => {
                let name = (*name).expect("expected member name");
                (*left, name)
            }
            other => panic!("expected member expression, got {other:?}"),
        }
    }

    /// Resolve a reference symbol from a reference expression.
    pub(crate) fn expect_reference_symbol(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> GlobalSymbolId {
        match self.tree.get(expression_id) {
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => *target_symbol,
            other => panic!("expected reference expression, got {other:?}"),
        }
    }

    /// Collect the first object field list available for a type.
    pub(crate) fn object_fields_for_type(&self, ty_id: LocalTypeId) -> Vec<TypeField> {
        // prefer direct object types
        if let Type::Object { fields, .. } = self.types.get_type(ty_id) {
            return fields.to_vec();
        }

        // scan intersection elements for an object type
        if let Type::Intersection { elements } = self.types.get_type(ty_id) {
            for element_id in elements {
                if let Type::Object { fields, .. } = self.types.get_type(*element_id) {
                    return fields.to_vec();
                }
            }
        }

        panic!("expected object fields for type");
    }

    /// Resolve one object field type by field name.
    pub(crate) fn expect_object_field_type(
        &self,
        ty_id: LocalTypeId,
        field_name: StringId,
    ) -> LocalTypeId {
        let fields = self.object_fields_for_type(ty_id);
        let field = fields
            .iter()
            .find(|field| field.key.name() == Some(field_name))
            .unwrap_or_else(|| panic!("expected object field {field_name:?}"));

        field.ty
    }
}

/// Collect extension kinds for a target symbol.
pub(crate) fn extension_kinds_for_target(
    view: &TestModuleView<'_>,
    target_symbol: GlobalSymbolId,
) -> Vec<ExtensionKind> {
    // load module state for extension visibility
    let profile = view.profile_id();
    let module = view.test.program.module_descriptor(view.module_id);
    let module = module.as_ref();

    // collect visible extensions for the target symbol
    let extension_symbols = run_to_completion(
        &view.test.compiler,
        view.test.program.current_revision(),
        |compiler, context| {
            compiler.visible_extension_symbols_for_target(
                SymbolTypeView::new(context, module, profile, view.symbols(), view.types()),
                target_symbol,
            )
        },
    );
    let extension_symbols = extension_symbols.unwrap_or_default();

    // map extension symbols to kinds
    extension_symbols
        .into_iter()
        .filter_map(|symbol| {
            run_to_completion(
                &view.test.compiler,
                view.test.program.current_revision(),
                |compiler, context| {
                    compiler.extension_for_symbol_in_module(
                        ModuleTypeView::new(context, module, profile, view.types()),
                        symbol,
                    )
                },
            )
            .ok()
            .flatten()
        })
        .map(|extension| extension.kind)
        .collect()
}
