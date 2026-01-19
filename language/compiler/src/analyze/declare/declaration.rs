use destack_dir::{
    Asynchrony, BindingAnchor, BindingKind, BindingModifier, Block, Constraint, Declaration,
    Declarator, DynamicKey, Export, Expression, Extension, ExtensionKind, FunctionCardinality,
    FunctionMode, Generics, GlobalNodeIdAny, GlobalSymbolId, Heritage, InferOrigin, InferScope,
    InferTable, Lineage, LocalNodeId, LocalNodeIdAny, LocalSymbolId, LocalTypeId, Member,
    Mutability, NodeTree, NodeType, NodeVisitor, NodeVisitorOptions, Parameter, StaticKey,
    SymbolSpace, SymbolTable, Type, TypeField, TypeIndexSignature, TypeKind, TypeLiteral,
    TypeTable, walk_block, walk_declaration, walk_expression,
};
use destack_workspace::{Module, ProfileId};

use crate::{AnalyzeError, AnalyzeResult, AnalyzeWarning, Compiler, InferContext};

use super::super::common::{CanonicalSymbolMode, ObjectShape, ObjectShapeSet};

/// Visitor used to declare type-level constructs across a module.
#[derive(Debug)]
struct DeclareVisitor<'a> {
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

impl<'a> DeclareVisitor<'a> {
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

impl NodeVisitor for DeclareVisitor<'_> {
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
        let result = self.compiler.declare_declaration(
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
    /// Declare all declarations reachable from the module roots.
    pub(crate) fn declare_module_declarations(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // prepare the declaration visitor
        let mut visitor = DeclareVisitor::new(self, module, profile, symbols, types);

        // walk each root expression to visit all declarations
        for root_id in module.dir(profile).roots.iter() {
            // visit the root expression
            let root = tree.get(*root_id);
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
    pub(super) fn declare_declaration(
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
                self.declare_generics(module, profile, generics, tree, symbols, types)?;

                Ok(())
            }
            Declaration::Type {
                descriptor,
                kind,
                static_parameters,
                value,
                ..
            } => {
                // declare static parameters
                if let Some(parameters) = static_parameters.as_ref() {
                    for parameter_id in parameters {
                        self.declare_parameter(
                            module,
                            profile,
                            *parameter_id,
                            tree,
                            symbols,
                            types,
                        )?;
                    }
                }

                // evaluate the type alias value
                let declared_ty_id = self.try_evaluate_expression_to_type(
                    module, profile, *value, tree, symbols, types, true, true,
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
                } else {
                    let value_ty = Type::Value {
                        value: instance_ty_id,
                    };
                    let value_ty_id = types.insert_type_from(value_ty, declaration_id);
                    types.set_value_type(symbol, value_ty_id);
                }

                Ok(())
            }
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
                self.declare_generics(module, profile, generics, tree, symbols, types)?;
                self.declare_heritage(
                    module,
                    profile,
                    heritage,
                    Some(descriptor.symbol),
                    tree,
                    symbols,
                    types,
                )?;

                // nominal reference for constructors
                let symbol = descriptor.symbol.into_global(module.id);
                let nominal_reference = Type::Reference {
                    symbol,
                    static_arguments: None,
                };
                let nominal_reference_id =
                    types.insert_type_from(nominal_reference, declaration_id);

                // build instance and value shapes from members
                let shapes = self.declare_member_shapes(
                    module,
                    profile,
                    members,
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
                self.declare_generics(module, profile, generics, tree, symbols, types)?;
                self.declare_heritage(
                    module,
                    profile,
                    heritage,
                    Some(descriptor.symbol),
                    tree,
                    symbols,
                    types,
                )?;

                // prepare nominal reference for constructors
                let symbol = descriptor.symbol.into_global(module.id);
                let nominal_reference = Type::Reference {
                    symbol,
                    static_arguments: None,
                };
                let nominal_reference_id =
                    types.insert_type_from(nominal_reference, declaration_id);

                // build instance and value shapes from members
                let shapes = self.declare_member_shapes(
                    module,
                    profile,
                    members,
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
                self.declare_generics(module, profile, generics, tree, symbols, types)?;
                self.declare_heritage(
                    module,
                    profile,
                    heritage,
                    Some(descriptor.symbol),
                    tree,
                    symbols,
                    types,
                )?;

                // prepare the nominal reference for enum values
                let symbol = descriptor.symbol.into_global(module.id);
                let nominal_reference = Type::Reference {
                    symbol,
                    static_arguments: None,
                };
                let nominal_reference_id =
                    types.insert_type_from(nominal_reference, declaration_id);

                // build instance and value shapes from members
                let shapes = self.declare_member_shapes(
                    module,
                    profile,
                    members,
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
                self.declare_generics(module, profile, generics, tree, symbols, types)?;
                self.declare_heritage(
                    module,
                    profile,
                    heritage,
                    Some(descriptor.symbol),
                    tree,
                    symbols,
                    types,
                )?;

                // build instance shape from members
                let shape =
                    self.declare_member_shape(module, profile, members, tree, symbols, types)?;

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
                ..
            } => {
                // resolve declaration merge state
                let symbol_entry = symbols.get_symbol(descriptor.symbol);
                let allow_merge = module.language_type.supports_declaration_merging()
                    || symbol_entry.origin.is_global_augmentation();

                // declare generics for the signature
                if let Some(generics) = signature.generics.as_ref() {
                    self.declare_generics(module, profile, generics, tree, symbols, types)?;
                }

                // evaluate the function signature
                let ty = self.evaluate_function_signature_to_type(
                    module,
                    profile,
                    signature,
                    declaration_id.into_any(),
                    tree,
                    symbols,
                    types,
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
                    declaration_id,
                    descriptor.symbol,
                    fn_ty_id,
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
                // declare generics and heritage
                self.declare_generics(module, profile, generics, tree, symbols, types)?;
                self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    *target_type,
                    tree,
                    symbols,
                    types,
                    true,
                    true,
                )?;
                self.declare_heritage(
                    module,
                    profile,
                    heritage,
                    Some(descriptor.symbol),
                    tree,
                    symbols,
                    types,
                )?;

                // build the extension instance shape
                let shape =
                    self.declare_member_shape(module, profile, members, tree, symbols, types)?;
                let instance_ty = shape.into_object_type();
                let instance_ty_id = types.insert_type_from(instance_ty, declaration_id);
                let extension_symbol = descriptor.symbol.into_global(module.id);
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
    fn declare_generics(
        &self,
        module: &Module,
        profile: ProfileId,
        generics: &Generics,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // evaluate static parameter constraints
        if let Some(parameters) = generics.static_parameters.as_ref() {
            for parameter_id in parameters {
                self.declare_parameter(module, profile, *parameter_id, tree, symbols, types)?;
            }
        }

        Ok(())
    }

    /// Declare a parameter by evaluating its declared type.
    fn declare_parameter(
        &self,
        module: &Module,
        profile: ProfileId,
        parameter_id: LocalNodeId<Parameter>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // resolve the declared type for the parameter
        let declared_type_id = types.get_declared_type_id(parameter_id.into_global_any(module.id));
        let Some(declared_type_id) = declared_type_id else {
            return Ok(());
        };

        // evaluate the declared type
        self.evaluate_type(module, profile, declared_type_id, tree, symbols, types)?;

        Ok(())
    }

    /// Declare heritage lineages for a nominal type.
    fn declare_heritage(
        &self,
        module: &Module,
        profile: ProfileId,
        heritage: &Heritage,
        symbol: Option<LocalSymbolId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // resolve heritage targets from evaluated types when possible
        let collect_symbol = |expression_id: LocalNodeId<Expression>,
                              symbols: &SymbolTable,
                              types: &mut TypeTable|
         -> AnalyzeResult<Option<GlobalSymbolId>> {
            let ty_id = self.try_evaluate_expression_to_type(
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

    /// Override a constructor return type with a nominal reference when provided.
    fn override_constructor_return_type(
        &self,
        ty_id: LocalTypeId,
        constructor_return: Option<LocalTypeId>,
        source_id: LocalNodeIdAny,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        let Some(constructor_return) = constructor_return else {
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
            return_type: Some(constructor_return),
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

    /// Declare instance and value shapes for a list of members.
    fn declare_member_shapes(
        &self,
        module: &Module,
        profile: ProfileId,
        members: &[LocalNodeId<Member>],
        constructor_return: Option<LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<ObjectShapeSet> {
        // initialize member shapes
        let mut shapes = ObjectShapeSet::default();

        // collect member contributions
        for member_id in members {
            let member = tree.get(*member_id);

            match member {
                Member::Type { .. } => {}
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
                        let key_type = self.try_evaluate_expression_to_type(
                            module, profile, *key, tree, symbols, types, true, true,
                        )?;
                        let value_type = if let Some(value) = value {
                            self.try_evaluate_expression_to_type(
                                module, profile, *value, tree, symbols, types, true, true,
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
                        let value_ty_id = self.try_evaluate_expression_to_type(
                            module, profile, *value, tree, symbols, types, true, true,
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
                        // evaluate the signature type
                        let ty = self.evaluate_function_signature_to_type(
                            module,
                            profile,
                            signature,
                            (*member_id).into_any(),
                            tree,
                            symbols,
                            types,
                        )?;
                        let ty_id = types.insert_type_from_any(ty, (*member_id).into_any());

                        // override constructor returns when needed
                        let ty_id = if signature.mode == Some(FunctionMode::Constructor) {
                            self.override_constructor_return_type(
                                ty_id,
                                constructor_return,
                                (*member_id).into_any(),
                                types,
                            )
                        } else {
                            ty_id
                        };

                        // record the declared signature for inference
                        types.set_signature_type_for_node(
                            (*member_id).into_global_any(module.id),
                            ty_id,
                        );

                        // route the signature to the correct shape
                        match signature.mode {
                            Some(FunctionMode::Constructor) => {
                                shapes.value.construct_signatures.push(ty_id);
                            }
                            Some(FunctionMode::New) => {
                                target_shape.construct_signatures.push(ty_id);
                            }
                            _ => {
                                target_shape.call_signatures.push(ty_id);
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

                    // build the method type
                    let ty = self.evaluate_function_signature_to_type(
                        module,
                        profile,
                        signature,
                        (*member_id).into_any(),
                        tree,
                        symbols,
                        types,
                    )?;
                    let ty_id = types.insert_type_from_any(ty, (*member_id).into_any());

                    // record the declared signature for inference
                    types.set_signature_type_for_node(
                        (*member_id).into_global_any(module.id),
                        ty_id,
                    );

                    // collect field modifiers
                    let is_optional = false;
                    let is_readonly = true;

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
        if let Type::Tuple { elements } = types.get_type(declared_ty_id) {
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
    fn declare_member_shape(
        &self,
        module: &Module,
        profile: ProfileId,
        members: &[LocalNodeId<Member>],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<ObjectShape> {
        let mut shape = ObjectShape::default();

        // collect member contributions
        for member_id in members {
            let member_shape =
                self.declare_member(module, profile, *member_id, tree, symbols, types)?;
            shape.extend_from_shape(&member_shape);
        }

        Ok(shape)
    }

    /// Declare a single member into an object shape.
    fn declare_member(
        &self,
        module: &Module,
        profile: ProfileId,
        member_id: LocalNodeId<Member>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<ObjectShape> {
        let member = tree.get(member_id);
        let mut shape = ObjectShape::default();

        match member {
            Member::Type { .. } => Ok(shape),
            Member::Field {
                modifiers,
                key,
                value,
                ..
            } => {
                // handle index signatures
                if let Some(DynamicKey::NamedExpression { name, key }) = key {
                    let key_type = self.try_evaluate_expression_to_type(
                        module, profile, *key, tree, symbols, types, true, true,
                    )?;
                    let value_type = if let Some(value) = value {
                        self.try_evaluate_expression_to_type(
                            module, profile, *value, tree, symbols, types, true, true,
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
                    let value_ty_id = self.try_evaluate_expression_to_type(
                        module, profile, *value, tree, symbols, types, true, true,
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
                    self.declare_generics(module, profile, generics, tree, symbols, types)?;
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
                    let ty = self.evaluate_function_signature_to_type(
                        module,
                        profile,
                        signature,
                        member_id.into_any(),
                        tree,
                        symbols,
                        types,
                    )?;
                    let ty_id = types.insert_type_from_any(ty, member_id.into_any());

                    // record the declared signature for inference
                    types.set_signature_type_for_node(member_id.into_global_any(module.id), ty_id);

                    match signature.mode {
                        Some(FunctionMode::New) | Some(FunctionMode::Constructor) => {
                            shape.construct_signatures.push(ty_id);
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
                let ty = self.evaluate_function_signature_to_type(
                    module,
                    profile,
                    signature,
                    member_id.into_any(),
                    tree,
                    symbols,
                    types,
                )?;
                let ty_id = types.insert_type_from_any(ty, member_id.into_any());

                // record the declared signature for inference
                types.set_signature_type_for_node(member_id.into_global_any(module.id), ty_id);

                // collect field modifiers
                let is_optional = false;
                let is_readonly = true;

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

        // ensure remote module declare is ready
        self.require_analyze_module_declare(symbol.module_id, profile)?;

        // import the remote value type into this module
        let remote_module = self.program.modules.get(symbol.module_id);
        let remote_module = remote_module.read();
        let remote_types = remote_module.dir(profile).types.read();
        let Some(remote_value_id) = remote_types.get_value_type_id(symbol) else {
            return Ok(None);
        };
        let remote_value_ty = remote_types.get_type(remote_value_id);
        let local_value_id = self.import_type_from_remote_for_node(
            declaration_id.into_any(),
            remote_value_ty,
            &remote_types,
            symbol,
            types,
        );

        Ok(Some(local_value_id))
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
            let signature_id = self.struct_constructor_signature(
                module,
                profile,
                nominal_reference_id,
                declaration_id,
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
        let signature = Type::Function {
            asynchrony: Asynchrony::Sync,
            cardinality: FunctionCardinality::Scalar,
            static_parameters: Vec::new(),
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
        members: &[LocalNodeId<Member>],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
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
            let value_id = value.ok_or_else(|| AnalyzeError::MissingType {
                node: member_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile)),
            })?;
            let field_ty_id = self.try_evaluate_expression_to_type(
                module, profile, value_id, tree, symbols, types, true, true,
            )?;
            dynamic_parameters.push(field_ty_id);
        }

        // build the constructor signature
        let signature = Type::Function {
            asynchrony: Asynchrony::Sync,
            cardinality: FunctionCardinality::Scalar,
            static_parameters: Vec::new(),
            this_parameter: None,
            dynamic_parameters,
            return_type: Some(nominal_reference_id),
        };
        Ok(types.insert_type_from(signature, declaration_id))
    }

    /// Declare exported value types using local information only.
    pub(crate) fn declare_exported_value_types(
        &self,
        module: &Module,
        profile: ProfileId,
        exported_symbols: &indexmap::IndexMap<(SymbolSpace, StaticKey), Export>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        #[derive(Debug)]
        struct ExportInference {
            symbol: GlobalSymbolId,
            declarator_id: LocalNodeId<Declarator>,
            value_id: Option<LocalNodeId<Expression>>,
        }

        let options = self.analyze_context_options_for_module(module.id);
        let mut infer = InferTable::default();
        let base_ctx = InferContext::new(profile, options).for_surface_inference();
        let mut inferred_exports = Vec::new();
        let mut export_inference = Vec::new();

        // collect exported symbols that need value types
        for export in exported_symbols.values() {
            if export.space != SymbolSpace::Value {
                continue;
            }
            let Some(target_symbol) = export.target.resolved() else {
                continue;
            };
            if target_symbol.module_id != module.id {
                continue;
            }
            if types.get_value_type_id(target_symbol).is_some() {
                continue;
            }

            // resolve the primary declaration node
            let symbol_entry = symbols.get_symbol(target_symbol.local_id);
            let Some(primary_declaration) = symbol_entry.primary_declaration else {
                continue;
            };

            // resolve the declarator that owns this binding
            let declarator_id = self.declarator_for_symbol(tree, primary_declaration);
            let Some(declarator_id) = declarator_id else {
                continue;
            };

            // evaluate declared types when present
            let declared_type_id =
                types.get_declared_type_id(declarator_id.into_global_any(module.id));
            if let Some(declared_type_id) = declared_type_id {
                self.evaluate_type(module, profile, declared_type_id, tree, symbols, types)?;
                types.set_value_type(target_symbol, declared_type_id);
                continue;
            }

            // defer to surface inference for initializer only exports
            let declarator = tree.get(declarator_id);
            export_inference.push(ExportInference {
                symbol: target_symbol,
                declarator_id,
                value_id: declarator.value,
            });
        }

        // seed inference variables for export symbols
        for export in &export_inference {
            let scope = InferScope {
                owner: export.symbol,
                function_id: None,
            };
            let origin = InferOrigin::Expression(export.declarator_id.into_global_any(module.id));
            let symbol_ty_id = self.infer_var_type_for_symbol(
                &mut infer,
                types,
                export.symbol,
                export.declarator_id.into_any(),
                origin,
                scope,
            );

            types.set_value_type(export.symbol, symbol_ty_id);

            inferred_exports.push((
                symbol_ty_id,
                export.declarator_id.into_global_any(module.id),
            ));
        }

        // infer initializer types and constrain export symbols
        for export in export_inference {
            let Some(value_id) = export.value_id else {
                continue;
            };

            let Some(symbol_ty_id) = types.get_value_type_id(export.symbol) else {
                continue;
            };

            let mut ctx = base_ctx.fork().with_expected_type(Some(symbol_ty_id));
            let inferred_ty_id = self
                .infer_expression(module, value_id, tree, symbols, types, &mut infer, &mut ctx)?;

            infer.push_constraint(Constraint::Subtype {
                sub_type: inferred_ty_id,
                super_type: symbol_ty_id,
                variance: None,
            });
        }

        // solve surface inference constraints before warning
        if !infer.vars.is_empty() {
            self.solve_infer_table(module, profile, symbols, &infer, types, &base_ctx.options);
        }

        // warn when exports remain unknown after surface inference
        for (ty_id, node_id) in inferred_exports {
            if matches!(
                types.get_type(ty_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown
                }
            ) {
                self.warning(AnalyzeWarning::ExportTypeUnknown {
                    node: node_id.into_anchored(Some(profile)),
                });
            }
        }

        Ok(())
    }

    /// Find the declarator that defines a symbol.
    fn declarator_for_symbol(
        &self,
        tree: &NodeTree,
        primary_declaration: GlobalNodeIdAny,
    ) -> Option<LocalNodeId<Declarator>> {
        let mut current_id = primary_declaration.local_id;
        loop {
            if current_id.ty == NodeType::Declarator {
                return Some(current_id.into_typed());
            }

            let parent = tree.get_parent(current_id.id)?;
            current_id = parent;
        }
    }
}
