use std::str::FromStr;

use crate::{AnalyzeError, Compiler};
use destack_ast::Keyword;
use destack_base::StringId;
use destack_dir::{
    Asynchrony, BindingAnchor, BindingOperator, Declaration, DeclarationAbstraction,
    DeclarationDescriptor, DeclarationKind, DependencyKind, DependencyMode, DynamicKey, Expression,
    FunctionCardinality, FunctionKind, FunctionMode, ImportAliasTarget, LocalNodeId,
    LocalNodeIdAny, Member, Name, NodeTree, NodeType, Parameter, Path, ScalarLiteral, SymbolTable,
    TypeTable,
};
use destack_workspace::{Module, ProfileId};

const RESERVED_TYPE_NAMES: [&str; 19] = [
    "undefined",
    "unknown",
    "object",
    "null",
    "any",
    "never",
    "boolean",
    "void",
    "character",
    "string",
    "bigint",
    "number",
    "int",
    "isize",
    "uint",
    "usize",
    "float",
    "symbol",
    "unique",
];

/// The kind of declare namespace context for a node.
#[derive(Clone, Copy, PartialEq, Eq)]
enum DeclareNamespaceContext {
    /// The node is not inside a declare namespace.
    None,
    /// The node is inside a declared namespace with an identifier name.
    Namespace,
    /// The node is inside a declared module with a string name.
    Module,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Validate a declaration.
    pub(super) fn validate_declaration(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        id: LocalNodeId<Declaration>,
        declaration: &Declaration,
    ) {
        // reject typescript-only declarations in javascript modules
        if module.language_type.is_javascript()
            && matches!(
                declaration,
                Declaration::Interface { .. } | Declaration::Type { .. } | Declaration::Enum { .. }
            )
        {
            let node = id.into_global_any(module.id).into_anchored(Some(profile));
            self.error(AnalyzeError::TypeScriptSyntaxInJavaScript { node });
        }

        // validate by declaration kind
        match declaration {
            Declaration::Interface {
                descriptor,
                heritage,
                ..
            } => {
                // resolve the interface node for diagnostics
                let node = id.into_global_any(module.id).into_anchored(Some(profile));

                // interfaces cannot be abstract
                if descriptor.abstraction == DeclarationAbstraction::Abstract {
                    self.error(AnalyzeError::InvalidInterface { node });
                }

                // default export interfaces must be named
                if descriptor.export == Some(DependencyMode::Default) && descriptor.name.is_none() {
                    self.error(AnalyzeError::InvalidInterface { node });
                }

                // reject intrinsic type names as interface identifiers in typescript and destack
                if (module.language_type.is_typescript() || module.language_type.is_destack())
                    && let Some(name) = descriptor.name
                    && self.is_reserved_type_name(name)
                {
                    self.error(AnalyzeError::ReservedIdentifier {
                        node,
                        name: name.string(),
                    });
                }

                // reject empty extends clauses
                let has_empty_extends = heritage
                    .extends_types
                    .as_ref()
                    .is_some_and(|e| e.is_empty());
                if has_empty_extends {
                    self.error(AnalyzeError::InvalidLineage {
                        node,
                        extends_symbols: Vec::new(),
                        implements_symbols: Vec::new(),
                        embedded_symbols: Vec::new(),
                    });
                }
            }

            Declaration::Function {
                descriptor,
                signature,
                body,
                ..
            } => {
                // resolve the function node for diagnostics
                let node = id.into_global_any(module.id).into_anchored(Some(profile));
                let is_destack = module.language_type.is_destack();

                // preserve reserved name validation for inner-only expression bindings
                self.validate_inner_only_declaration_reserved_name(
                    module, profile, symbols, id, descriptor,
                );

                // declare functions cannot have a body
                let is_declare = descriptor.kind == DeclarationKind::Declaration
                    || self.is_in_declare_namespace(tree, id.into_any());
                if is_declare && body.is_some() {
                    self.error(AnalyzeError::InvalidFunction { node });
                }

                // arrow function values cannot declare explicit this parameters
                if signature.kind == FunctionKind::Lambda
                    && body.is_some()
                    && signature.this_parameter.is_some()
                {
                    self.error(AnalyzeError::InvalidFunction { node });
                }

                // declare functions cannot be async
                if !is_destack && is_declare && signature.asynchrony == Asynchrony::Async {
                    self.error(AnalyzeError::InvalidFunction { node });
                }

                // declare functions cannot be generators
                if !is_destack
                    && is_declare
                    && signature.cardinality == FunctionCardinality::Generator
                {
                    self.error(AnalyzeError::InvalidFunction { node });
                }

                // strict directive prologues require simple parameter lists in JS/TS modes
                if !is_destack
                    && let Some(body) = body
                    && self.has_non_simple_dynamic_parameters(tree, &signature.dynamic_parameters)
                    && self.body_declares_use_strict_directive(tree, *body)
                {
                    self.error(AnalyzeError::InvalidFunction { node });
                }
            }

            Declaration::Type {
                descriptor,
                static_parameters,
                ..
            } => {
                // resolve the type alias node for diagnostics
                let node = id.into_global_any(module.id).into_anchored(Some(profile));

                // reject intrinsic type names as type alias identifiers in typescript and destack
                if (module.language_type.is_typescript() || module.language_type.is_destack())
                    && let Some(name) = descriptor.name
                    && self.is_reserved_type_name(name)
                {
                    self.error(AnalyzeError::ReservedIdentifier {
                        node,
                        name: name.string(),
                    });
                }

                // reject invalid type parameter modifiers in type aliases
                if let Some(static_parameters) = static_parameters {
                    for parameter_id in static_parameters {
                        let parameter = tree.get(*parameter_id);
                        let modifiers = parameter.modifiers();
                        let has_const_modifier = modifiers.is_some_and(|modifiers| {
                            modifiers.operator == Some(BindingOperator::AsConst)
                        });
                        if has_const_modifier {
                            let node = parameter_id
                                .into_global_any(module.id)
                                .into_anchored(Some(profile));
                            self.error(AnalyzeError::InvalidTypeParameterModifier { node });
                        }
                    }
                }
            }

            Declaration::Struct { heritage, .. } => {
                // resolve the struct node for diagnostics
                let node = id.into_global_any(module.id).into_anchored(Some(profile));

                // structs cannot use extends
                let has_extends = heritage
                    .extends_types
                    .as_ref()
                    .is_some_and(|e| !e.is_empty());
                if has_extends {
                    let extends_symbols = types
                        .get_lineage_for_symbol(declaration.symbol().into_global(module.id))
                        .and_then(|lineage| lineage.extends)
                        .into_iter()
                        .collect();
                    self.error(AnalyzeError::InvalidLineage {
                        node,
                        extends_symbols,
                        implements_symbols: Vec::new(),
                        embedded_symbols: Vec::new(),
                    });
                }
            }

            Declaration::ImportAlias { kind, target, .. } => {
                // import aliases cannot use import type
                if *kind == DependencyKind::Type {
                    let node = id.into_global_any(module.id).into_anchored(Some(profile));
                    self.error(AnalyzeError::InvalidTypeOnlyImportAlias { node });
                }

                // import alias targets must be qualified identifier paths
                self.validate_import_alias_target(module, profile, tree, target);
            }

            Declaration::Class {
                heritage, members, ..
            } => {
                // resolve the class node for diagnostics
                let node = id.into_global_any(module.id).into_anchored(Some(profile));

                // preserve reserved name validation for inner-only expression bindings
                self.validate_inner_only_declaration_reserved_name(
                    module,
                    profile,
                    symbols,
                    id,
                    declaration.descriptor(),
                );

                // reject typescript only class syntax in javascript modules
                if module.language_type.is_javascript() {
                    let has_abstract =
                        declaration.descriptor().abstraction == DeclarationAbstraction::Abstract;
                    let has_implements = heritage.implements_types.is_some();
                    if has_abstract || has_implements {
                        self.error(AnalyzeError::TypeScriptSyntaxInJavaScript { node });
                    }
                }

                // reject intrinsic type names as class identifiers in typescript and destack
                if (module.language_type.is_typescript() || module.language_type.is_destack())
                    && let Some(name) = declaration.descriptor().name
                    && self.is_reserved_type_name(name)
                {
                    self.error(AnalyzeError::ReservedIdentifier {
                        node,
                        name: name.string(),
                    });
                }

                // classes can only extend one class
                let has_multiple_extends =
                    heritage.extends_types.as_ref().is_some_and(|e| e.len() > 1);
                if has_multiple_extends {
                    let lineage =
                        types.get_lineage_for_symbol(declaration.symbol().into_global(module.id));
                    let extends_symbols = lineage.and_then(|l| l.extends).into_iter().collect();
                    let implements_symbols =
                        lineage.map(|l| l.implements.clone()).unwrap_or_default();
                    let embedded_symbols = lineage.map(|l| l.embedded.clone()).unwrap_or_default();
                    self.error(AnalyzeError::InvalidLineage {
                        node,
                        extends_symbols,
                        implements_symbols,
                        embedded_symbols,
                    });
                }

                // reject empty extends or implements clauses
                let has_empty_extends = heritage
                    .extends_types
                    .as_ref()
                    .is_some_and(|e| e.is_empty());
                let has_empty_implements = heritage
                    .implements_types
                    .as_ref()
                    .is_some_and(|i| i.is_empty());
                if has_empty_extends || has_empty_implements {
                    self.error(AnalyzeError::InvalidLineage {
                        node,
                        extends_symbols: Vec::new(),
                        implements_symbols: Vec::new(),
                        embedded_symbols: Vec::new(),
                    });
                }

                // class bodies can contain at most one constructor definition
                self.validate_class_constructor_members(module, profile, tree, members);
            }

            Declaration::Enum { .. } => {}

            Declaration::Extension {
                heritage: _,
                members: _,
                ..
            } => {}

            _ => {}
        }
    }

    /// Validate duplicate constructor definitions in class members.
    fn validate_class_constructor_members(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        members: &[LocalNodeId<Member>],
    ) {
        // track whether a concrete constructor implementation has been declared
        let mut has_constructor_implementation = false;

        // scan class members in source order
        for member_id in members {
            let Member::Method {
                modifiers,
                key,
                signature,
                body,
                ..
            } = tree.get(*member_id)
            else {
                continue;
            };

            // skip members that are not constructor definitions
            if !self.class_member_is_constructor_definition(
                modifiers.as_ref(),
                key.as_ref(),
                signature,
            ) {
                continue;
            }

            // overload signatures without bodies are allowed
            if body.is_none() {
                continue;
            }

            // report duplicate constructor implementations
            if has_constructor_implementation {
                let node = (*member_id)
                    .into_global_any(module.id)
                    .into_anchored(Some(profile));
                self.error(AnalyzeError::InvalidConstructor { node });
                continue;
            }

            has_constructor_implementation = true;
        }
    }

    /// Return true when a class method is a constructor definition.
    fn class_member_is_constructor_definition(
        &self,
        modifiers: Option<&destack_dir::BindingModifier>,
        key: Option<&DynamicKey>,
        signature: &destack_dir::FunctionSignature,
    ) -> bool {
        // static methods are never constructors
        let is_static =
            modifiers.is_some_and(|modifiers| modifiers.anchor == Some(BindingAnchor::Static));
        if is_static {
            return false;
        }

        // explicit constructor signatures are constructors
        if signature.mode == Some(FunctionMode::Constructor) {
            return true;
        }

        // accessor forms do not define constructors
        if matches!(
            signature.mode,
            Some(FunctionMode::Getter | FunctionMode::Setter)
        ) {
            return false;
        }

        // named methods with the key `constructor` also define constructors
        let Some(DynamicKey::Name(name)) = key else {
            return false;
        };

        self.program.strings.get(*name) == "constructor"
    }

    /// Check whether a name is reserved as an intrinsic type identifier.
    fn is_reserved_type_name(&self, name: Name) -> bool {
        let name_str = self.program.strings.get(name.string());
        RESERVED_TYPE_NAMES.contains(&name_str.as_ref())
    }

    /// Validate reserved identifiers for declarations that do not publish a named symbol.
    fn validate_inner_only_declaration_reserved_name(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        declaration_id: LocalNodeId<Declaration>,
        descriptor: &DeclarationDescriptor,
    ) {
        // only user code can trigger this diagnostic
        if !module.is_user() {
            return;
        }

        // only named declarations can be reserved identifier violations
        let Some(name) = descriptor.name else {
            return;
        };

        // regular declaration symbols are handled by scope-wide binding checks
        let symbol = symbols.get_symbol(descriptor.symbol);
        if symbol.name().is_some() {
            return;
        }

        // inner-only names still need reserved identifier validation
        let name = name.string();
        if !self.is_reserved_binding_name(name) {
            return;
        }

        let node = declaration_id
            .into_global_any(module.id)
            .into_anchored(Some(profile));
        self.error(AnalyzeError::ReservedIdentifier { node, name });
    }

    /// Validate an import alias target.
    fn validate_import_alias_target(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        target: &ImportAliasTarget,
    ) {
        // require targets are always valid
        let ImportAliasTarget::Path { value } = target else {
            return;
        };

        // validate qualified identifier paths
        if !self.is_valid_import_alias_expression(tree, *value) {
            let node = value
                .into_global_any(module.id)
                .into_anchored(Some(profile));
            self.error(AnalyzeError::InvalidImportAliasTarget { node });
        }
    }

    /// Check whether an import alias target expression is valid.
    fn is_valid_import_alias_expression(
        &self,
        tree: &NodeTree,
        value: LocalNodeId<Expression>,
    ) -> bool {
        // match on allowed import alias expression shapes
        match tree.get(value) {
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
            } => static_arguments.is_none() && self.is_valid_import_alias_path(path),
            Expression::Member {
                left,
                name,
                static_arguments,
            } => {
                static_arguments.is_none()
                    && self.is_valid_import_alias_expression(tree, *left)
                    && self.is_valid_import_alias_segment(*name)
            }
            _ => false,
        }
    }

    /// Check whether an import alias path is valid.
    fn is_valid_import_alias_path(&self, path: &Path) -> bool {
        // require identifier segments
        !path.segments.is_empty()
            && path
                .segments
                .iter()
                .all(|segment| self.is_valid_import_alias_segment(*segment))
    }

    /// Check whether an import alias segment is valid.
    fn is_valid_import_alias_segment(&self, segment: StringId) -> bool {
        let name = self.program.strings.get(segment);
        Keyword::from_str(name.as_ref()).is_err()
    }

    /// Return true when a parameter list is non-simple.
    pub(super) fn has_non_simple_dynamic_parameters(
        &self,
        tree: &NodeTree,
        dynamic_parameters: &[LocalNodeId<Parameter>],
    ) -> bool {
        // detect defaults, patterns, and variadics in the parameter list
        dynamic_parameters.iter().any(|parameter_id| {
            let parameter = tree.get(*parameter_id);
            match parameter {
                Parameter::Named { default, .. } => default.is_some(),
                Parameter::Pattern { .. } | Parameter::VariadicPattern { .. } => true,
                Parameter::VariadicNamed { .. } => false,
            }
        })
    }

    /// Return true when a function body declares a `use strict` directive.
    pub(super) fn body_declares_use_strict_directive(
        &self,
        tree: &NodeTree,
        body_id: LocalNodeId<Expression>,
    ) -> bool {
        // intern the strict directive once for stable comparisons
        let use_strict = self.program.strings.intern("use strict");

        // only block bodies can contain directive prologues
        let Expression::Block { block } = tree.get(body_id) else {
            return false;
        };
        let block = tree.get(*block);

        // scan the directive prologue
        for expression_id in &block.expressions {
            let expression = tree.get(*expression_id);
            let Some(directive) = self.expression_directive_literal(tree, expression) else {
                break;
            };

            if directive == use_strict {
                return true;
            }
        }

        false
    }

    /// Return the directive literal id for a statement expression when present.
    fn expression_directive_literal(
        &self,
        tree: &NodeTree,
        expression: &Expression,
    ) -> Option<StringId> {
        // directives are parsed either as statement wrappers or bare literals
        let statement = match expression {
            Expression::Statement { statement } => tree.get(*statement),
            _ => expression,
        };

        let Expression::ScalarLiteral {
            value: ScalarLiteral::String(string_id),
        } = statement
        else {
            return None;
        };

        Some(*string_id)
    }

    /// Check whether a node is nested inside a declare namespace.
    pub(super) fn is_in_declare_namespace(&self, tree: &NodeTree, node_id: LocalNodeIdAny) -> bool {
        matches!(
            self.declare_namespace_context(tree, node_id),
            DeclareNamespaceContext::Namespace | DeclareNamespaceContext::Module
        )
    }

    /// Check whether a node is nested inside a declared module with a string name.
    pub(super) fn is_in_declare_module(&self, tree: &NodeTree, node_id: LocalNodeIdAny) -> bool {
        matches!(
            self.declare_namespace_context(tree, node_id),
            DeclareNamespaceContext::Module
        )
    }

    /// Classify the nearest declare namespace context for a node.
    /// NOTE #Performance: store "declaredness" on Scope/Symbol directly (from import/bind)?
    fn declare_namespace_context(
        &self,
        tree: &NodeTree,
        node_id: LocalNodeIdAny,
    ) -> DeclareNamespaceContext {
        // walk up the parent chain
        let mut saw_namespace = false;
        let mut current = tree.get_parent(node_id.id);
        while let Some(parent) = current {
            if parent.ty == NodeType::Declaration {
                let declaration = tree.get(parent.into_typed::<Declaration>());
                if let Declaration::Namespace { descriptor, .. } = declaration
                    && descriptor.kind == DeclarationKind::Declaration
                {
                    if matches!(descriptor.name, Some(Name::String(_))) {
                        return DeclareNamespaceContext::Module;
                    }
                    saw_namespace = true;
                }
            }
            current = tree.get_parent(parent.id);
        }
        if saw_namespace {
            DeclareNamespaceContext::Namespace
        } else {
            DeclareNamespaceContext::None
        }
    }
}
