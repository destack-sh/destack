use destack_dir::{
    Asynchrony, BindingAnchor, BindingKind, BindingModifier, Block, Declaration, DynamicKey,
    Expression, Extension, ExtensionKind, FunctionCardinality, FunctionMode, Generics,
    GlobalSymbolId, Heritage, Lineage, LocalNodeId, LocalNodeIdAny, LocalSymbolId, LocalTypeId,
    Member, Mutability, NodeTree, NodeVisitor, NodeVisitorOptions, Parameter, StaticArgument,
    StaticExpression, StaticKey, SymbolTable, Timing, Type, TypeField, TypeIndexSignature,
    TypeKind, TypeLiteral, TypeTable, walk_block, walk_declaration, walk_expression,
};
use destack_workspace::{Module, ProfileId};
use std::collections::HashMap;

use crate::{AnalyzeError, AnalyzeResult, Compiler};

use crate::analyze::common::{
    AnalyzeDependencyStage, CanonicalSymbolMode, ObjectShape, ObjectShapeSet,
};

/// Visitor used to declare type-level constructs across a module.
#[derive(Debug)]
struct CollectVisitor<'a> {
    /// The compiler shared state.
    compiler: &'a Compiler,
    /// The module being declared.
    module: &'a Module,
    /// The profile id used for evaluation.
    profile: ProfileId,
    /// The symbol table for this module.
    symbols: &'a SymbolTable,
    /// The type table for this module.
    types: &'a mut TypeTable,
    /// Track the first error encountered while walking.
    result: AnalyzeResult<()>,
    /// Node visitor options (unused, but required by trait).
    options: NodeVisitorOptions,
}

impl<'a> CollectVisitor<'a> {
    /// Create a new declare visitor.
    fn new(
        compiler: &'a Compiler,
        module: &'a Module,
        profile: ProfileId,
        symbols: &'a SymbolTable,
        types: &'a mut TypeTable,
    ) -> Self {
        // build the visitor state
        Self {
            compiler,
            module,
            profile,
            symbols,
            types,
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
        destack_base::ensure_sufficient_stack(|| {
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
        let result = self.compiler.collect_declaration(
            self.module,
            self.profile,
            id,
            tree,
            self.symbols,
            self.types,
        );
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
    /// Decide whether declared types should be deferred for a module.
    fn should_defer_declaration_types(&self, module: &Module) -> bool {
        if !module.language_type.is_declaration() {
            return false;
        }

        let module_checks = self.module_check_options_for_module(module.id);
        module_checks.skip_lib_check || module.is_builtin()
    }

    /// Resolve or defer a type expression into a type id.
    pub(crate) fn collect_or_defer_type_expression(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        defer_type_evaluation: bool,
    ) -> AnalyzeResult<LocalTypeId> {
        if !defer_type_evaluation {
            return self.resolve_declared_type_expression(
                module,
                profile,
                expression_id,
                tree,
                symbols,
                types,
                true,
                true,
            );
        }

        let global_id = expression_id.into_global_any(module.id);
        if let Some(existing) = types.get_declared_type_id(global_id) {
            return Ok(existing);
        }

        let ty_id = types.insert_type_from(Type::Unevaluated(expression_id), expression_id);
        types.set_declared_type(global_id, ty_id);
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

    /// Declare all declarations reachable from the module roots.
    pub(crate) fn collect_module_declarations(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // prepare the declaration visitor
        let mut visitor = CollectVisitor::new(self, module, profile, symbols, types);

        // walk each root expression to visit all declarations
        for root_id in module.dir(profile).roots.iter() {
            let root = tree.get(*root_id);
            // visit the root expression
            visitor.visit_expression(tree, *root_id, root);

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
        module: &Module,
        profile: ProfileId,
        declaration_id: LocalNodeId<Declaration>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // load the declaration node
        let declaration = tree.get(declaration_id);

        // dispatch by declaration kind
        match declaration {
            Declaration::Global { .. } => Ok(()),
            Declaration::Namespace { generics, .. } => {
                // declare namespace generics
                self.collect_generics(module, profile, generics, tree, symbols, types)?;

                Ok(())
            }
            Declaration::Type {
                descriptor,
                kind,
                static_parameters,
                value,
                ..
            } => {
                // decide whether to defer declared types
                let defer_type_evaluation = self.should_defer_declaration_types(module);

                // declare static parameters
                if let Some(parameters) = static_parameters.as_ref() {
                    for parameter_id in parameters {
                        self.collect_parameter(
                            module,
                            profile,
                            *parameter_id,
                            tree,
                            symbols,
                            types,
                        )?;
                    }
                }

                // validate comptime usage for array size parameters
                if static_parameters
                    .as_ref()
                    .is_some_and(|parameters| !parameters.is_empty())
                {
                    self.validate_static_value_parameter_usage_in_type_expression(
                        module, profile, *value, tree, symbols, types, false, true,
                    )?;
                }

                // avoid eager evaluation for generic aliases
                // detect value static parameters that require deferred evaluation
                let has_comptime_parameters =
                    static_parameters.as_ref().is_some_and(|parameters| {
                        parameters.iter().any(|param_id| {
                            let parameter = tree.get(*param_id);
                            parameter
                                .modifiers()
                                .is_some_and(|modifiers| modifiers.timing == Some(Timing::Comptime))
                        })
                    });

                // resolve the declared type eagerly for type-only parameters
                let should_defer = defer_type_evaluation || has_comptime_parameters;
                let declared_ty_id = self.collect_or_defer_type_expression(
                    module,
                    profile,
                    *value,
                    tree,
                    symbols,
                    types,
                    should_defer,
                )?;
                types.set_declared_type(value.into_global_any(module.id), declared_ty_id);

                // register the instance type for this symbol
                let symbol = descriptor.symbol.into_global(module.id);
                types.set_alias_target_type_id(symbol, declared_ty_id);
                let instance_ty_id = match *kind {
                    TypeKind::Structural => declared_ty_id,
                    TypeKind::Nominal => {
                        let ty = Type::Reference {
                            symbol,
                            static_arguments: None,
                        };
                        types.insert_type_from(ty, declaration_id)
                    }
                };
                types.set_instance_type(symbol, instance_ty_id);

                // register the value type for this symbol
                if *kind == TypeKind::Nominal {
                    let static_parameters = self.static_parameter_placeholders_for_declaration(
                        module,
                        static_parameters.as_deref(),
                        tree,
                        types,
                    );
                    let constructor_id = self.newtype_constructor_signature(
                        declaration_id,
                        declared_ty_id,
                        instance_ty_id,
                        static_parameters,
                        types,
                    );
                    let mut shape = ObjectShape::default();
                    shape.call_signatures.push(constructor_id);
                    self.merge_value_shape_into_symbol(
                        module,
                        declaration_id,
                        descriptor.symbol,
                        &shape,
                        types,
                        false,
                    );
                } else if module.language_type.is_destack() {
                    let value_ty = Type::Value {
                        value: instance_ty_id,
                    };
                    let value_ty_id = types.insert_type_from(value_ty, declaration_id);
                    types.set_value_type(symbol, value_ty_id);
                }

                Ok(())
            }
            Declaration::ImportAlias { .. } => Ok(()),
            Declaration::Struct {
                descriptor,
                generics,
                heritage,
                members,
                ..
            } => {
                // resolve declaration merge state
                let symbol_entry = symbols.get_symbol(descriptor.symbol);
                let allow_merge = module.language_type.is_declaration();
                let is_primary = symbol_entry
                    .primary_declaration
                    .is_some_and(|primary| primary == declaration_id.into_global_any(module.id));

                // declare generics and heritage
                self.collect_generics(module, profile, generics, tree, symbols, types)?;
                self.collect_heritage(
                    module,
                    profile,
                    heritage,
                    Some(descriptor.symbol),
                    tree,
                    symbols,
                    types,
                )?;
                let declaration_symbol = descriptor.symbol.into_global(module.id);
                self.report_missing_declared_associated_type_requirements(
                    module,
                    profile,
                    declaration_symbol,
                    heritage,
                    members,
                    false,
                    tree,
                    symbols,
                    types,
                )?;
                self.report_missing_declared_associated_comptime_requirements(
                    module,
                    profile,
                    declaration_symbol,
                    heritage,
                    members,
                    false,
                    tree,
                    symbols,
                    types,
                )?;

                // nominal reference for constructors
                let symbol = descriptor.symbol.into_global(module.id);
                let static_arguments = self.self_type_static_arguments_for_declaration(
                    module,
                    generics.static_parameters.as_deref(),
                    tree,
                    types,
                );
                let nominal_reference = Type::Reference {
                    symbol,
                    static_arguments,
                };
                let nominal_reference_id =
                    types.insert_type_from(nominal_reference, declaration_id);

                // build instance and value shapes from members
                let shapes = self.collect_member_shapes(
                    module,
                    profile,
                    members,
                    None,
                    Some(nominal_reference_id),
                    tree,
                    symbols,
                    types,
                )?;
                let instance_shape = shapes.instance;
                let mut value_shape = shapes.value;

                // merge instance shapes for merged declarations
                self.merge_instance_shape_into_merge_group(
                    module,
                    declaration_id,
                    descriptor.symbol,
                    &instance_shape,
                    symbols,
                    types,
                    allow_merge,
                );

                // merge global augmentations once per primary declaration
                if allow_merge && is_primary {
                    self.merge_global_instance_shape_for_symbol(
                        module,
                        declaration_id,
                        descriptor.symbol,
                        symbols,
                        types,
                        profile,
                    )?;
                }

                // ensure constructors exist for the value shape
                self.ensure_constructor_signatures(
                    module,
                    profile,
                    declaration_id,
                    symbol,
                    nominal_reference_id,
                    &mut value_shape,
                    tree,
                    symbols,
                    types,
                )?;

                // register the nominal value type with static members
                self.merge_value_shape_into_symbol(
                    module,
                    declaration_id,
                    descriptor.symbol,
                    &value_shape,
                    types,
                    allow_merge,
                );

                // merge global augmentations for the value shape
                if allow_merge && is_primary {
                    self.merge_global_value_shape_for_symbol(
                        module,
                        declaration_id,
                        descriptor.symbol,
                        symbols,
                        types,
                        profile,
                    )?;
                }

                Ok(())
            }
            Declaration::Class {
                descriptor,
                generics,
                heritage,
                members,
                ..
            } => {
                // resolve declaration merge state
                let symbol_entry = symbols.get_symbol(descriptor.symbol);
                let allow_merge = module.language_type.supports_declaration_merging()
                    || symbol_entry.origin.is_global_augmentation();
                let is_primary = symbol_entry
                    .primary_declaration
                    .is_some_and(|primary| primary == declaration_id.into_global_any(module.id));

                // declare generics and heritage
                self.collect_generics(module, profile, generics, tree, symbols, types)?;
                self.collect_heritage(
                    module,
                    profile,
                    heritage,
                    Some(descriptor.symbol),
                    tree,
                    symbols,
                    types,
                )?;
                let declaration_symbol = descriptor.symbol.into_global(module.id);
                let allows_deferred_associated =
                    descriptor.abstraction == destack_dir::DeclarationAbstraction::Abstract;
                self.report_missing_declared_associated_type_requirements(
                    module,
                    profile,
                    declaration_symbol,
                    heritage,
                    members,
                    allows_deferred_associated,
                    tree,
                    symbols,
                    types,
                )?;
                self.report_missing_declared_associated_comptime_requirements(
                    module,
                    profile,
                    declaration_symbol,
                    heritage,
                    members,
                    allows_deferred_associated,
                    tree,
                    symbols,
                    types,
                )?;

                // prepare nominal reference for constructors
                let symbol = descriptor.symbol.into_global(module.id);
                let static_arguments = self.self_type_static_arguments_for_declaration(
                    module,
                    generics.static_parameters.as_deref(),
                    tree,
                    types,
                );
                let nominal_reference = Type::Reference {
                    symbol,
                    static_arguments,
                };
                let nominal_reference_id =
                    types.insert_type_from(nominal_reference, declaration_id);

                // build instance and value shapes from members
                let shapes = self.collect_member_shapes(
                    module,
                    profile,
                    members,
                    generics.static_parameters.as_deref(),
                    Some(nominal_reference_id),
                    tree,
                    symbols,
                    types,
                )?;
                let instance_shape = shapes.instance;
                let mut value_shape = shapes.value;

                // merge instance shapes for merged declarations
                self.merge_instance_shape_into_merge_group(
                    module,
                    declaration_id,
                    descriptor.symbol,
                    &instance_shape,
                    symbols,
                    types,
                    allow_merge,
                );

                // merge global augmentations once per primary declaration
                if allow_merge && is_primary {
                    self.merge_global_instance_shape_for_symbol(
                        module,
                        declaration_id,
                        descriptor.symbol,
                        symbols,
                        types,
                        profile,
                    )?;
                }

                // ensure constructors exist for the value shape
                self.ensure_constructor_signatures(
                    module,
                    profile,
                    declaration_id,
                    symbol,
                    nominal_reference_id,
                    &mut value_shape,
                    tree,
                    symbols,
                    types,
                )?;

                // register the nominal value type with static members
                self.merge_value_shape_into_symbol(
                    module,
                    declaration_id,
                    descriptor.symbol,
                    &value_shape,
                    types,
                    allow_merge,
                );

                // merge global augmentations for the value shape
                if allow_merge && is_primary {
                    self.merge_global_value_shape_for_symbol(
                        module,
                        declaration_id,
                        descriptor.symbol,
                        symbols,
                        types,
                        profile,
                    )?;
                }

                Ok(())
            }
            Declaration::Enum {
                descriptor,
                generics,
                heritage,
                fields,
                members,
                ..
            } => {
                // resolve declaration merge state
                let symbol_entry = symbols.get_symbol(descriptor.symbol);
                let allow_merge = module.language_type.supports_declaration_merging()
                    || symbol_entry.origin.is_global_augmentation();
                let is_primary = symbol_entry
                    .primary_declaration
                    .is_some_and(|primary| primary == declaration_id.into_global_any(module.id));

                // declare generics and heritage
                self.collect_generics(module, profile, generics, tree, symbols, types)?;
                self.collect_heritage(
                    module,
                    profile,
                    heritage,
                    Some(descriptor.symbol),
                    tree,
                    symbols,
                    types,
                )?;
                let declaration_symbol = descriptor.symbol.into_global(module.id);
                self.report_missing_declared_associated_type_requirements(
                    module,
                    profile,
                    declaration_symbol,
                    heritage,
                    members,
                    false,
                    tree,
                    symbols,
                    types,
                )?;
                self.report_missing_declared_associated_comptime_requirements(
                    module,
                    profile,
                    declaration_symbol,
                    heritage,
                    members,
                    false,
                    tree,
                    symbols,
                    types,
                )?;

                // prepare the nominal reference for enum values
                let symbol = descriptor.symbol.into_global(module.id);
                let static_arguments = self.self_type_static_arguments_for_declaration(
                    module,
                    generics.static_parameters.as_deref(),
                    tree,
                    types,
                );
                let nominal_reference = Type::Reference {
                    symbol,
                    static_arguments,
                };
                let nominal_reference_id =
                    types.insert_type_from(nominal_reference, declaration_id);

                // build instance and value shapes from members
                let shapes = self.collect_member_shapes(
                    module,
                    profile,
                    members,
                    generics.static_parameters.as_deref(),
                    Some(nominal_reference_id),
                    tree,
                    symbols,
                    types,
                )?;
                let instance_shape = shapes.instance;
                let mut value_shape = shapes.value;

                // merge instance shapes for merged declarations
                self.merge_instance_shape_into_merge_group(
                    module,
                    declaration_id,
                    descriptor.symbol,
                    &instance_shape,
                    symbols,
                    types,
                    allow_merge,
                );

                // merge global augmentations once per primary declaration
                if allow_merge && is_primary {
                    self.merge_global_instance_shape_for_symbol(
                        module,
                        declaration_id,
                        descriptor.symbol,
                        symbols,
                        types,
                        profile,
                    )?;
                }

                // build value fields for enum members
                for field_id in fields {
                    let field = tree.get(*field_id);
                    let field_symbol = field.symbol.into_global(module.id);
                    types.set_value_type(field_symbol, nominal_reference_id);

                    value_shape.fields.push(TypeField {
                        key: StaticKey::Name(field.name),
                        ty: nominal_reference_id,
                        is_optional: false,
                        is_readonly: true,
                    });
                }

                // register the enum value type
                self.merge_value_shape_into_symbol(
                    module,
                    declaration_id,
                    descriptor.symbol,
                    &value_shape,
                    types,
                    allow_merge,
                );

                // merge global augmentations for the value shape
                if allow_merge && is_primary {
                    self.merge_global_value_shape_for_symbol(
                        module,
                        declaration_id,
                        descriptor.symbol,
                        symbols,
                        types,
                        profile,
                    )?;
                }

                Ok(())
            }
            Declaration::Interface {
                descriptor,
                generics,
                heritage,
                members,
                ..
            } => {
                // resolve declaration merge state
                let symbol_entry = symbols.get_symbol(descriptor.symbol);
                let allow_merge = module.language_type.supports_declaration_merging()
                    || symbol_entry.origin.is_global_augmentation();
                let is_primary = symbol_entry
                    .primary_declaration
                    .is_some_and(|primary| primary == declaration_id.into_global_any(module.id));

                // declare generics and heritage
                self.collect_generics(module, profile, generics, tree, symbols, types)?;
                self.collect_heritage(
                    module,
                    profile,
                    heritage,
                    Some(descriptor.symbol),
                    tree,
                    symbols,
                    types,
                )?;

                // build instance shape from members
                let shape = self
                    .collect_member_shape(module, profile, members, None, tree, symbols, types)?;

                // merge instance shapes for merged declarations
                self.merge_instance_shape_into_merge_group(
                    module,
                    declaration_id,
                    descriptor.symbol,
                    &shape,
                    symbols,
                    types,
                    allow_merge,
                );

                // merge global augmentations once per primary declaration
                if allow_merge && is_primary {
                    self.merge_global_instance_shape_for_symbol(
                        module,
                        declaration_id,
                        descriptor.symbol,
                        symbols,
                        types,
                        profile,
                    )?;
                }

                // register the nominal type as the value type
                let nominal_ty = Type::Reference {
                    symbol: descriptor.symbol.into_global(module.id),
                    static_arguments: None,
                };
                let nominal_ty_id = types.insert_type_from(nominal_ty, declaration_id);
                let value_ty = Type::Value {
                    value: nominal_ty_id,
                };
                let value_ty_id = types.insert_type_from(value_ty, declaration_id);
                types.set_value_type(descriptor.symbol.into_global(module.id), value_ty_id);

                Ok(())
            }
            Declaration::Function {
                descriptor,
                signature,
                body,
                ..
            } => {
                // decide whether to defer declared types
                let defer_type_evaluation = self.should_defer_declaration_types(module);

                // resolve declaration merge state
                let symbol_entry = symbols.get_symbol(descriptor.symbol);
                let allow_merge = module.language_type.supports_declaration_merging()
                    || module.language_type.is_destack()
                    || symbol_entry.origin.is_global_augmentation();

                // enforce single implementation for TypeScript overloads
                if self.should_enforce_single_overload(module) && body.is_some() {
                    let mut implementation_count = 0;
                    let mut declaration_nodes = Vec::new();
                    if let Some(primary) = symbol_entry.primary_declaration {
                        declaration_nodes.push(primary);
                    }
                    if let Some(secondary) = symbol_entry.secondary_declarations.as_ref() {
                        declaration_nodes.extend(secondary.iter().copied());
                    }

                    for declaration_id in declaration_nodes {
                        if declaration_id.module_id != module.id {
                            continue;
                        }
                        let Ok(declaration_id) =
                            declaration_id.local_id.try_into_typed::<Declaration>()
                        else {
                            continue;
                        };
                        if let Declaration::Function { body: Some(_), .. } =
                            tree.get(declaration_id)
                        {
                            implementation_count += 1;
                            if implementation_count > 1 {
                                self.report_overload_implementation_error(
                                    module,
                                    profile,
                                    declaration_id.into_any(),
                                    false,
                                );
                                break;
                            }
                        }
                    }
                }

                // declare generics for the signature
                if let Some(generics) = signature.generics.as_ref() {
                    self.collect_generics(module, profile, generics, tree, symbols, types)?;
                }

                // load any previously cached signature for this declaration
                let previous_signature_id =
                    types.get_signature_type_for_node(declaration_id.into_global_any(module.id));

                // evaluate the function signature
                let ty = self.resolve_declared_function_signature_type(
                    module,
                    profile,
                    signature,
                    declaration_id.into_any(),
                    tree,
                    symbols,
                    types,
                    defer_type_evaluation,
                )?;
                let fn_ty_id = types.insert_type_from_any(ty, declaration_id.into_any());

                // record the declared signature for inference
                types.set_signature_type_for_node(
                    declaration_id.into_global_any(module.id),
                    fn_ty_id,
                );

                // merge into callable instance shape
                let mut shape = ObjectShape::default();
                shape.push_call_signature(fn_ty_id);
                self.merge_instance_shape_into_merge_group(
                    module,
                    declaration_id,
                    descriptor.symbol,
                    &shape,
                    symbols,
                    types,
                    allow_merge,
                );

                // merge the function into the value type
                self.merge_function_value_type(
                    module,
                    profile,
                    declaration_id,
                    descriptor.symbol,
                    fn_ty_id,
                    previous_signature_id,
                    symbols,
                    types,
                    allow_merge,
                );

                Ok(())
            }
            Declaration::Extension {
                descriptor,
                generics,
                target_type,
                target_symbol,
                heritage,
                members,
                ..
            } => {
                // decide whether to defer declared types
                let defer_type_evaluation = self.should_defer_declaration_types(module);

                // declare generics and heritage
                self.collect_generics(module, profile, generics, tree, symbols, types)?;
                if !defer_type_evaluation {
                    self.resolve_declared_type_expression(
                        module,
                        profile,
                        *target_type,
                        tree,
                        symbols,
                        types,
                        true,
                        true,
                    )?;
                }
                self.collect_heritage(
                    module,
                    profile,
                    heritage,
                    Some(descriptor.symbol),
                    tree,
                    symbols,
                    types,
                )?;

                // skip already declared extensions for this symbol
                let extension_symbol = descriptor.symbol.into_global(module.id);
                if types
                    .get_extension_id_for_symbol(extension_symbol)
                    .is_some()
                {
                    return Ok(());
                }

                // build the extension instance shape
                let extension_static_parameters = generics.static_parameters.as_deref();
                let shape = self.collect_member_shape(
                    module,
                    profile,
                    members,
                    extension_static_parameters,
                    tree,
                    symbols,
                    types,
                )?;
                let instance_ty = shape.into_object_type();
                let instance_ty_id = types.insert_type_from(instance_ty, declaration_id);
                types.set_instance_type(extension_symbol, instance_ty_id);

                // register the extension when a target symbol exists
                if let Some(target) = target_symbol {
                    // resolve the canonical target symbol for extension lookup
                    let canonical_target = self.canonical_symbol_id(
                        module,
                        symbols,
                        profile,
                        *target,
                        CanonicalSymbolMode::FollowAliases,
                    );
                    let kind = if module.id == canonical_target.module_id {
                        ExtensionKind::Inherent
                    } else if descriptor.name.is_some() {
                        ExtensionKind::Nominal
                    } else {
                        ExtensionKind::Local
                    };
                    let lineage = types.get_lineage_id_for_symbol(extension_symbol);
                    let extension =
                        Extension::new(extension_symbol, kind, canonical_target, lineage);
                    types.insert_extension(extension);
                }

                Ok(())
            }
        }
    }

    /// Declare generics by evaluating static parameter constraint types.
    pub(crate) fn collect_generics(
        &self,
        module: &Module,
        profile: ProfileId,
        generics: &Generics,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // defer generic constraint evaluation for declaration modules
        if self.should_defer_declaration_types(module) {
            return Ok(());
        }

        // evaluate static parameter constraints
        if let Some(parameters) = generics.static_parameters.as_ref() {
            for parameter_id in parameters {
                self.collect_parameter(module, profile, *parameter_id, tree, symbols, types)?;
            }
        }

        Ok(())
    }

    /// Declare a parameter by evaluating its declared type.
    fn collect_parameter(
        &self,
        module: &Module,
        profile: ProfileId,
        parameter_id: LocalNodeId<Parameter>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // defer parameter evaluation for declaration modules
        if self.should_defer_declaration_types(module) {
            return Ok(());
        }

        // resolve the declared type for the parameter
        let declared_type_id = types.get_declared_type_id(parameter_id.into_global_any(module.id));
        let Some(declared_type_id) = declared_type_id else {
            return Ok(());
        };

        // evaluate the declared type
        self.resolve_declared_type(module, profile, declared_type_id, tree, symbols, types)?;

        Ok(())
    }

    /// Declare heritage lineages for a nominal type.
    fn collect_heritage(
        &self,
        module: &Module,
        profile: ProfileId,
        heritage: &Heritage,
        symbol: Option<LocalSymbolId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // defer heritage evaluation for declaration modules
        let defer_type_evaluation = self.should_defer_declaration_types(module);

        // resolve heritage targets from evaluated types when possible
        let collect_symbol = |expression_id: LocalNodeId<Expression>,
                              symbols: &SymbolTable,
                              types: &mut TypeTable|
         -> AnalyzeResult<Option<GlobalSymbolId>> {
            if !defer_type_evaluation {
                let ty_id = self.resolve_declared_type_expression(
                    module,
                    profile,
                    expression_id,
                    tree,
                    symbols,
                    types,
                    true,
                    true,
                )?;
                let type_symbol = self.unwrap_type_value_symbol(types, ty_id);

                // prefer evaluated type references when available
                if let Some(type_symbol) = type_symbol {
                    return Ok(Some(self.merged_type_symbol_id(
                        module,
                        symbols,
                        profile,
                        type_symbol,
                    )));
                }
            }

            // fall back to the syntactic target symbol
            let expression = tree.get(expression_id);
            Ok(expression
                .target_symbol()
                .map(|symbol| self.merged_type_symbol_id(module, symbols, profile, symbol)))
        };

        // resolve extends symbols
        let mut extends_symbols = Vec::new();
        if let Some(extend_types) = heritage.extends_types.as_ref() {
            for expression_id in extend_types {
                if let Some(target_symbol) = collect_symbol(*expression_id, symbols, types)? {
                    let canonical_symbol = self.canonical_symbol_id(
                        module,
                        symbols,
                        profile,
                        target_symbol,
                        CanonicalSymbolMode::FollowAliases,
                    );
                    extends_symbols.push(canonical_symbol);
                }
            }
        }

        // resolve implements symbols
        let mut implements_symbols = Vec::new();
        if let Some(implements_types) = heritage.implements_types.as_ref() {
            for expression_id in implements_types {
                if let Some(target_symbol) = collect_symbol(*expression_id, symbols, types)? {
                    let canonical_symbol = self.canonical_symbol_id(
                        module,
                        symbols,
                        profile,
                        target_symbol,
                        CanonicalSymbolMode::FollowAliases,
                    );
                    implements_symbols.push(canonical_symbol);
                }
            }
        }

        // resolve embedded symbols
        let mut embedded_symbols = Vec::new();
        if let Some(embedded_types) = heritage.embedded_types.as_ref() {
            for expression_id in embedded_types {
                if let Some(target_symbol) = collect_symbol(*expression_id, symbols, types)? {
                    let canonical_symbol = self.canonical_symbol_id(
                        module,
                        symbols,
                        profile,
                        target_symbol,
                        CanonicalSymbolMode::FollowAliases,
                    );
                    embedded_symbols.push(canonical_symbol);
                }
            }
        }

        // record lineage when a symbol is provided
        if let Some(symbol) = symbol {
            let lineage = Lineage {
                extends: extends_symbols.first().copied(),
                implements: implements_symbols,
                embedded: embedded_symbols,
            };
            if !lineage.is_empty() {
                let lineage_id = types.insert_lineage(lineage);
                types.set_lineage_for_symbol(symbol.into_global(module.id), lineage_id);
            }
        }

        Ok(())
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
            static_parameters,
            this_parameter,
            dynamic_parameters,
            ..
        } = types.get_type(ty_id).clone()
        else {
            return ty_id;
        };

        let rebuilt = Type::Function {
            asynchrony,
            cardinality,
            static_parameters,
            this_parameter,
            dynamic_parameters,
            return_type: Some(return_type),
        };
        types.insert_type_from_any(rebuilt, source_id)
    }

    /// Return true when the member modifiers mark it as static.
    fn member_is_static(modifiers: Option<&BindingModifier>) -> bool {
        modifiers.is_some_and(|modifiers| modifiers.anchor == Some(BindingAnchor::Static))
    }

    /// Select the target shape for a static or instance member.
    fn member_target_shape(shapes: &mut ObjectShapeSet, is_static: bool) -> &mut ObjectShape {
        if is_static {
            &mut shapes.value
        } else {
            &mut shapes.instance
        }
    }

    /// Return optionality and readonly flags for a field.
    fn field_flags(modifiers: Option<&BindingModifier>) -> (bool, bool) {
        let is_optional =
            modifiers.is_some_and(|modifiers| matches!(modifiers.kind, Some(BindingKind::Maybe)));
        let is_readonly = modifiers
            .is_some_and(|modifiers| matches!(modifiers.mutability, Some(Mutability::Immutable)));

        (is_optional, is_readonly)
    }

    /// Add constructor parameter property fields to an instance shape.
    fn add_constructor_parameter_property_fields(
        &self,
        signature: &destack_dir::FunctionSignature,
        signature_ty_id: LocalTypeId,
        shape: &mut ObjectShape,
        tree: &NodeTree,
        types: &TypeTable,
    ) {
        let Type::Function {
            dynamic_parameters, ..
        } = types.get_type(signature_ty_id)
        else {
            return;
        };

        // map parameter property declarations into instance fields
        for (index, parameter_id) in signature.dynamic_parameters.iter().enumerate() {
            let parameter = tree.get(*parameter_id);
            let Some(modifiers) = parameter.modifiers() else {
                continue;
            };
            let is_parameter_property = modifiers.visibility.is_some()
                || modifiers.mutability == Some(Mutability::Immutable);
            if !is_parameter_property {
                continue;
            }

            let Parameter::Named { name, .. } = parameter else {
                continue;
            };
            let Some(parameter_ty_id) = dynamic_parameters.get(index).copied() else {
                continue;
            };

            // skip duplicates from explicit field declarations
            let key = StaticKey::Name(*name);
            if shape.fields.iter().any(|field| field.key == key) {
                continue;
            }

            let is_optional = modifiers.kind == Some(BindingKind::Maybe);
            let is_readonly = modifiers.mutability == Some(Mutability::Immutable);
            shape.fields.push(TypeField {
                key,
                ty: parameter_ty_id,
                is_optional,
                is_readonly,
            });
        }
    }

    /// Build static parameter placeholders for a type declaration.
    fn static_parameter_placeholders_for_declaration(
        &self,
        module: &Module,
        static_parameters: Option<&[LocalNodeId<Parameter>]>,
        tree: &NodeTree,
        types: &mut TypeTable,
    ) -> Vec<LocalTypeId> {
        // stop when no static parameters exist
        let Some(parameters) = static_parameters else {
            return Vec::new();
        };

        // map parameters to reference placeholders
        let mut placeholders = Vec::with_capacity(parameters.len());
        for parameter_id in parameters {
            let symbol = tree.get(*parameter_id).symbol().into_global(module.id);
            let ty = Type::Reference {
                symbol,
                static_arguments: None,
            };
            let type_id = types.insert_type_from(ty, *parameter_id);
            placeholders.push(type_id);
        }

        placeholders
    }

    /// Build `Self<...>` static arguments for declaration-local nominal references.
    fn self_type_static_arguments_for_declaration(
        &self,
        module: &Module,
        static_parameters: Option<&[LocalNodeId<Parameter>]>,
        tree: &NodeTree,
        types: &mut TypeTable,
    ) -> Option<Vec<StaticArgument>> {
        // build placeholder type references for static parameters
        let placeholders = self.static_parameter_placeholders_for_declaration(
            module,
            static_parameters,
            tree,
            types,
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
        module: &Module,
        owner_static_parameters: Option<&[LocalNodeId<Parameter>]>,
        ty: Type,
        tree: &NodeTree,
        types: &mut TypeTable,
    ) -> Type {
        // TODO #Cleanup: fold owner static parameters during type evaluation once instantiation boundaries are explicit
        let Some(owner_static_parameters) = owner_static_parameters else {
            return ty;
        };

        let Type::Function {
            asynchrony,
            cardinality,
            static_parameters,
            this_parameter,
            dynamic_parameters,
            return_type,
        } = ty
        else {
            return ty;
        };

        let owner_placeholders = self.static_parameter_placeholders_for_declaration(
            module,
            Some(owner_static_parameters),
            tree,
            types,
        );
        if owner_placeholders.is_empty() {
            return Type::Function {
                asynchrony,
                cardinality,
                static_parameters,
                this_parameter,
                dynamic_parameters,
                return_type,
            };
        }

        let mut combined = owner_placeholders;
        combined.extend(static_parameters);

        Type::Function {
            asynchrony,
            cardinality,
            static_parameters: combined,
            this_parameter,
            dynamic_parameters,
            return_type,
        }
    }

    /// Declare instance and value shapes for a list of members.
    fn collect_member_shapes(
        &self,
        module: &Module,
        profile: ProfileId,
        members: &[LocalNodeId<Member>],
        owner_static_parameters: Option<&[LocalNodeId<Parameter>]>,
        constructor_return: Option<LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<ObjectShapeSet> {
        // defer member type evaluation for declaration modules
        let defer_type_evaluation = self.should_defer_declaration_types(module);

        // enforce single implementations for TypeScript methods and constructors
        if self.should_enforce_single_overload(module) {
            let mut method_implementations: HashMap<StaticKey, usize> = HashMap::new();
            let mut constructor_implementations = 0;

            for member_id in members {
                let Member::Method {
                    key,
                    signature,
                    body,
                    ..
                } = tree.get(*member_id)
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
                            module,
                            profile,
                            (*member_id).into_any(),
                            true,
                        );
                    }
                    continue;
                }

                let Some(key) = key.as_ref() else {
                    continue;
                };
                let Some(static_key) =
                    self.static_key_from_dynamic_key(profile, *key, tree, symbols, types)
                else {
                    continue;
                };

                let count = method_implementations.entry(static_key).or_insert(0);
                *count += 1;
                if *count > 1 {
                    self.report_overload_implementation_error(
                        module,
                        profile,
                        (*member_id).into_any(),
                        false,
                    );
                }
            }
        }

        // initialize member shapes
        let mut shapes = ObjectShapeSet::default();

        // predeclare associated type members so later member references can resolve by symbol
        self.collect_associated_type_members(
            module,
            profile,
            members,
            tree,
            symbols,
            types,
            defer_type_evaluation,
        )?;

        // collect member contributions
        for member_id in members {
            let member = tree.get(*member_id);
            match member {
                Member::Type { .. } | Member::ComptimeConst { .. } => {}
                Member::Field {
                    modifiers,
                    key,
                    value,
                    ..
                } => {
                    // decide whether this field is static
                    let is_static = Self::member_is_static(modifiers.as_ref());

                    // select the target shape
                    let target_shape = Self::member_target_shape(&mut shapes, is_static);

                    // handle index signatures
                    if let Some(DynamicKey::NamedExpression { name, key }) = key {
                        // resolve index signature types
                        let key_type = self.collect_or_defer_type_expression(
                            module,
                            profile,
                            *key,
                            tree,
                            symbols,
                            types,
                            defer_type_evaluation,
                        )?;
                        let value_type = if let Some(value) = value {
                            self.collect_or_defer_type_expression(
                                module,
                                profile,
                                *value,
                                tree,
                                symbols,
                                types,
                                defer_type_evaluation,
                            )?
                        } else {
                            let ty = Type::TypeLiteral {
                                value: TypeLiteral::Unknown,
                            };
                            types.insert_type_from_any(ty, (*member_id).into_any())
                        };

                        // collect index signature flags
                        let is_readonly = modifiers.as_ref().is_some_and(|modifiers| {
                            modifiers.mutability == Some(Mutability::Immutable)
                        });

                        target_shape.index_signatures.push(TypeIndexSignature {
                            name: *name,
                            key_type,
                            value_type,
                            is_readonly,
                        });
                        continue;
                    }

                    // resolve a static key for the field
                    let static_key = key.and_then(|key| {
                        self.static_key_from_dynamic_key(profile, key, tree, symbols, types)
                    });

                    // resolve the field type
                    let value_ty_id = if let Some(value) = value {
                        let value_ty_id = self.collect_or_defer_type_expression(
                            module,
                            profile,
                            *value,
                            tree,
                            symbols,
                            types,
                            defer_type_evaluation,
                        )?;
                        types.set_declared_type(value.into_global_any(module.id), value_ty_id);
                        value_ty_id
                    } else {
                        let ty = Type::TypeLiteral {
                            value: TypeLiteral::Unknown,
                        };
                        types.insert_type_from_any(ty, (*member_id).into_any())
                    };

                    // collect field flags
                    let (is_optional, is_readonly) = Self::field_flags(modifiers.as_ref());

                    // build the field when a static key exists
                    if let Some(key) = static_key {
                        target_shape.fields.push(TypeField {
                            key,
                            ty: value_ty_id,
                            is_optional,
                            is_readonly,
                        });
                    }
                }
                Member::Method {
                    modifiers,
                    key,
                    signature,
                    body: _,
                    ..
                } => {
                    // decide whether this method is static
                    let is_static = Self::member_is_static(modifiers.as_ref());

                    // select the target shape
                    let target_shape = Self::member_target_shape(&mut shapes, is_static);

                    // handle call or construct signatures
                    if key.is_none()
                        && matches!(
                            signature.mode,
                            Some(FunctionMode::Call)
                                | Some(FunctionMode::New)
                                | Some(FunctionMode::Constructor)
                        )
                    {
                        // declare generics for the signature
                        if let Some(generics) = signature.generics.as_ref() {
                            self.collect_generics(module, profile, generics, tree, symbols, types)?;
                        }

                        // evaluate the signature type
                        let ty = self.resolve_declared_function_signature_type(
                            module,
                            profile,
                            signature,
                            (*member_id).into_any(),
                            tree,
                            symbols,
                            types,
                            defer_type_evaluation,
                        )?;
                        let ty = self.extend_signature_static_parameters(
                            module,
                            owner_static_parameters,
                            ty,
                            tree,
                            types,
                        );
                        let signature_ty_id =
                            types.insert_type_from_any(ty, (*member_id).into_any());

                        // override constructor returns when needed
                        let construct_signature_id =
                            if signature.mode == Some(FunctionMode::Constructor) {
                                self.replace_signature_return_type(
                                    signature_ty_id,
                                    constructor_return,
                                    (*member_id).into_any(),
                                    types,
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
                                let void_ty_id =
                                    types.insert_type_from_any(void_ty, (*member_id).into_any());
                                self.replace_signature_return_type(
                                    signature_ty_id,
                                    Some(void_ty_id),
                                    (*member_id).into_any(),
                                    types,
                                )
                            } else {
                                signature_ty_id
                            };
                        types.set_signature_type_for_node(
                            (*member_id).into_global_any(module.id),
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
                                    tree,
                                    types,
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
                    let Some(key) = key.and_then(|key| {
                        self.static_key_from_dynamic_key(profile, key, tree, symbols, types)
                    }) else {
                        continue;
                    };

                    // declare generics for the signature
                    if let Some(generics) = signature.generics.as_ref() {
                        self.collect_generics(module, profile, generics, tree, symbols, types)?;
                    }

                    // build the method type
                    let ty = self.resolve_declared_function_signature_type(
                        module,
                        profile,
                        signature,
                        (*member_id).into_any(),
                        tree,
                        symbols,
                        types,
                        defer_type_evaluation,
                    )?;
                    let ty = self.extend_signature_static_parameters(
                        module,
                        owner_static_parameters,
                        ty,
                        tree,
                        types,
                    );
                    let ty_id = types.insert_type_from_any(ty, (*member_id).into_any());

                    // record the declared signature for inference
                    types.set_signature_type_for_node(
                        (*member_id).into_global_any(module.id),
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
                    modifiers, value, ..
                } => {
                    // collect embedded fields from the target type
                    let is_static = Self::member_is_static(modifiers.as_ref());
                    let target_shape = Self::member_target_shape(&mut shapes, is_static);
                    let embed_shape =
                        self.embed_member_shape(module, profile, *value, tree, symbols, types)?;
                    target_shape.extend_from_shape(&embed_shape);
                }
                Member::StaticBlock { .. } | Member::ComptimeBlock { .. } => {}
            }
        }

        Ok(shapes)
    }

    /// Build a constructor signature for a nominal type alias.
    fn newtype_constructor_signature(
        &self,
        declaration_id: LocalNodeId<Declaration>,
        declared_ty_id: LocalTypeId,
        nominal_reference_id: LocalTypeId,
        static_parameters: Vec<LocalTypeId>,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // derive positional parameters from tuple aliases
        let mut dynamic_parameters = Vec::new();
        if let Type::Tuple { elements, .. } = types.get_type(declared_ty_id) {
            for element in elements {
                dynamic_parameters.push(element.ty);
            }
        } else {
            dynamic_parameters.push(declared_ty_id);
        }

        // build the constructor signature
        let signature = Type::Function {
            asynchrony: Asynchrony::Sync,
            cardinality: FunctionCardinality::Scalar,
            static_parameters,
            this_parameter: None,
            dynamic_parameters,
            return_type: Some(nominal_reference_id),
        };
        types.insert_type_from(signature, declaration_id)
    }

    /// Declare the instance shape for a list of members.
    fn collect_member_shape(
        &self,
        module: &Module,
        profile: ProfileId,
        members: &[LocalNodeId<Member>],
        owner_static_parameters: Option<&[LocalNodeId<Parameter>]>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<ObjectShape> {
        // defer member type evaluation for declaration modules
        let defer_type_evaluation = self.should_defer_declaration_types(module);
        let mut shape = ObjectShape::default();

        // predeclare associated type members so later member references can resolve by symbol
        self.collect_associated_type_members(
            module,
            profile,
            members,
            tree,
            symbols,
            types,
            defer_type_evaluation,
        )?;

        // collect member contributions
        for member_id in members {
            let member_shape = self.collect_member(
                module,
                profile,
                *member_id,
                owner_static_parameters,
                tree,
                symbols,
                types,
                defer_type_evaluation,
            )?;
            shape.extend_from_shape(&member_shape);
        }

        Ok(shape)
    }

    /// Declare a single member into an object shape.
    fn collect_member(
        &self,
        module: &Module,
        profile: ProfileId,
        member_id: LocalNodeId<Member>,
        owner_static_parameters: Option<&[LocalNodeId<Parameter>]>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        defer_type_evaluation: bool,
    ) -> AnalyzeResult<ObjectShape> {
        let member = tree.get(member_id);
        let mut shape = ObjectShape::default();

        match member {
            Member::Type { .. } | Member::ComptimeConst { .. } => Ok(shape),
            Member::Field {
                modifiers,
                key,
                value,
                ..
            } => {
                // handle index signatures
                if let Some(DynamicKey::NamedExpression { name, key }) = key {
                    let key_type = self.collect_or_defer_type_expression(
                        module,
                        profile,
                        *key,
                        tree,
                        symbols,
                        types,
                        defer_type_evaluation,
                    )?;
                    let value_type = if let Some(value) = value {
                        self.collect_or_defer_type_expression(
                            module,
                            profile,
                            *value,
                            tree,
                            symbols,
                            types,
                            defer_type_evaluation,
                        )?
                    } else {
                        let ty = Type::TypeLiteral {
                            value: TypeLiteral::Unknown,
                        };
                        types.insert_type_from_any(ty, member_id.into_any())
                    };
                    let is_readonly = modifiers.as_ref().is_some_and(|modifiers| {
                        modifiers.mutability == Some(Mutability::Immutable)
                    });

                    shape.index_signatures.push(TypeIndexSignature {
                        name: *name,
                        key_type,
                        value_type,
                        is_readonly,
                    });

                    return Ok(shape);
                }

                // resolve a static key for the field
                let static_key = key.and_then(|key| {
                    self.static_key_from_dynamic_key(profile, key, tree, symbols, types)
                });

                // resolve the field type
                let value_ty_id = if let Some(value) = value {
                    let value_ty_id = self.collect_or_defer_type_expression(
                        module,
                        profile,
                        *value,
                        tree,
                        symbols,
                        types,
                        defer_type_evaluation,
                    )?;
                    types.set_declared_type(value.into_global_any(module.id), value_ty_id);
                    value_ty_id
                } else {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    types.insert_type_from_any(ty, member_id.into_any())
                };

                // collect field modifiers
                let is_optional = modifiers
                    .as_ref()
                    .is_some_and(|m| matches!(m.kind, Some(BindingKind::Maybe)));
                let is_readonly = modifiers
                    .as_ref()
                    .is_some_and(|m| matches!(m.mutability, Some(Mutability::Immutable)));

                // build the field when a static key exists
                if let Some(key) = static_key {
                    shape.fields.push(TypeField {
                        key,
                        ty: value_ty_id,
                        is_optional,
                        is_readonly,
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
                // declare generics for the signature
                if let Some(generics) = signature.generics.as_ref() {
                    self.collect_generics(module, profile, generics, tree, symbols, types)?;
                }

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
                        module,
                        profile,
                        signature,
                        member_id.into_any(),
                        tree,
                        symbols,
                        types,
                        defer_type_evaluation,
                    )?;
                    let ty = self.extend_signature_static_parameters(
                        module,
                        owner_static_parameters,
                        ty,
                        tree,
                        types,
                    );
                    let ty_id = types.insert_type_from_any(ty, member_id.into_any());

                    // record the declared signature for inference
                    types.set_signature_type_for_node(member_id.into_global_any(module.id), ty_id);

                    match signature.mode {
                        Some(FunctionMode::New) | Some(FunctionMode::Constructor) => {
                            shape.construct_signatures.push(ty_id);
                            if signature.mode == Some(FunctionMode::Constructor) {
                                self.add_constructor_parameter_property_fields(
                                    signature, ty_id, &mut shape, tree, types,
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
                let Some(key) = key.and_then(|key| {
                    self.static_key_from_dynamic_key(profile, key, tree, symbols, types)
                }) else {
                    return Ok(shape);
                };

                // build the method type
                let ty = self.resolve_declared_function_signature_type(
                    module,
                    profile,
                    signature,
                    member_id.into_any(),
                    tree,
                    symbols,
                    types,
                    defer_type_evaluation,
                )?;
                let ty = self.extend_signature_static_parameters(
                    module,
                    owner_static_parameters,
                    ty,
                    tree,
                    types,
                );
                let ty_id = types.insert_type_from_any(ty, member_id.into_any());

                // record the declared signature for inference
                types.set_signature_type_for_node(member_id.into_global_any(module.id), ty_id);

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
                let embed_shape =
                    self.embed_member_shape(module, profile, *value, tree, symbols, types)?;
                shape.extend_from_shape(&embed_shape);

                Ok(shape)
            }
            Member::StaticBlock { .. } | Member::ComptimeBlock { .. } => Ok(shape),
        }
    }

    /// Resolve a local value type id for a symbol.
    fn resolve_value_type_for_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        declaration_id: LocalNodeId<Declaration>,
        symbol: GlobalSymbolId,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // use the local value type when available
        if symbol.module_id == module.id {
            return Ok(types.get_value_type_id(symbol));
        }

        self.with_module_types_at_stage(
            module,
            profile,
            symbol.module_id,
            AnalyzeDependencyStage::Declare,
            |_, remote_types| {
                let Some(remote_value_id) = remote_types.get_value_type_id(symbol) else {
                    return Ok(None);
                };
                let remote_value_ty = remote_types.get_type(remote_value_id);
                let local_value_id = self.import_type_from_remote_for_node(
                    declaration_id.into_any(),
                    remote_value_ty,
                    remote_types,
                    symbol,
                    types,
                );

                Ok(Some(local_value_id))
            },
        )
        .map_err(AnalyzeError::from)?
    }

    /// Collect constructor signatures from a symbol value type.
    fn collect_constructor_signatures_for_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        declaration_id: LocalNodeId<Declaration>,
        symbol: GlobalSymbolId,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Vec<LocalTypeId>> {
        // resolve the local value type for the symbol
        let local_value_id =
            self.resolve_value_type_for_symbol(module, profile, declaration_id, symbol, types)?;

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
            types,
            &mut shape,
            &mut extras,
            &mut visited,
        );
        Ok(shape.construct_signatures)
    }

    /// Ensure constructors exist for a nominal value shape.
    fn ensure_constructor_signatures(
        &self,
        module: &Module,
        profile: ProfileId,
        declaration_id: LocalNodeId<Declaration>,
        symbol: GlobalSymbolId,
        nominal_reference_id: LocalTypeId,
        value_shape: &mut ObjectShape,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // stop once constructors exist
        if !value_shape.construct_signatures.is_empty() {
            return Ok(());
        }

        // prefer positional constructors for nominal declarations
        if let Declaration::Struct { members, .. } | Declaration::Class { members, .. } =
            tree.get(declaration_id)
        {
            let owner_static_parameters = match tree.get(declaration_id) {
                Declaration::Struct { generics, .. } | Declaration::Class { generics, .. } => {
                    generics.static_parameters.as_deref()
                }
                _ => None,
            };
            let signature_id = self.struct_constructor_signature(
                module,
                profile,
                nominal_reference_id,
                declaration_id,
                owner_static_parameters,
                members,
                tree,
                symbols,
                types,
            )?;
            value_shape.construct_signatures.push(signature_id);
            return Ok(());
        }

        // inherit constructors from the base class when present
        if let Some(lineage) = types.get_lineage_for_symbol(symbol).cloned()
            && let Some(base_symbol) = lineage.extends
        {
            let inherited = self.collect_constructor_signatures_for_symbol(
                module,
                profile,
                declaration_id,
                base_symbol,
                types,
            )?;
            if !inherited.is_empty() {
                value_shape.construct_signatures.extend(inherited);
                return Ok(());
            }
        }

        // fall back to a default constructor
        let owner_static_parameters = match tree.get(declaration_id) {
            Declaration::Struct { generics, .. } | Declaration::Class { generics, .. } => {
                generics.static_parameters.as_deref()
            }
            _ => None,
        };
        let static_parameters = self.static_parameter_placeholders_for_declaration(
            module,
            owner_static_parameters,
            tree,
            types,
        );
        let signature = Type::Function {
            asynchrony: Asynchrony::Sync,
            cardinality: FunctionCardinality::Scalar,
            static_parameters,
            this_parameter: None,
            dynamic_parameters: Vec::new(),
            return_type: Some(nominal_reference_id),
        };
        let signature_id = types.insert_type_from(signature, declaration_id);
        value_shape.construct_signatures.push(signature_id);

        Ok(())
    }

    /// Build a positional constructor signature for nominal fields.
    fn struct_constructor_signature(
        &self,
        module: &Module,
        profile: ProfileId,
        nominal_reference_id: LocalTypeId,
        declaration_id: LocalNodeId<Declaration>,
        owner_static_parameters: Option<&[LocalNodeId<Parameter>]>,
        members: &[LocalNodeId<Member>],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        // defer field type evaluation for declaration modules
        let defer_type_evaluation = self.should_defer_declaration_types(module);

        // collect field types in source order
        let mut dynamic_parameters = Vec::new();
        for member_id in members {
            let member = tree.get(*member_id);
            let Member::Field {
                modifiers, value, ..
            } = member
            else {
                continue;
            };

            // skip static fields
            if Self::member_is_static(modifiers.as_ref()) {
                continue;
            }

            // require declared field types
            let value_id = value.ok_or_else(|| AnalyzeError::ImplicitAny {
                node: member_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile)),
            })?;
            let field_ty_id = self.collect_or_defer_type_expression(
                module,
                profile,
                value_id,
                tree,
                symbols,
                types,
                defer_type_evaluation,
            )?;
            dynamic_parameters.push(field_ty_id);
        }

        // build the constructor signature
        let static_parameters = self.static_parameter_placeholders_for_declaration(
            module,
            owner_static_parameters,
            tree,
            types,
        );
        let signature = Type::Function {
            asynchrony: Asynchrony::Sync,
            cardinality: FunctionCardinality::Scalar,
            static_parameters,
            this_parameter: None,
            dynamic_parameters,
            return_type: Some(nominal_reference_id),
        };
        Ok(types.insert_type_from(signature, declaration_id))
    }
}
