use std::collections::HashMap;

use destack_builtin::LanguageSymbol;
use destack_dir::{
    Annotation, Argument, DeprecatedNotice, ExperimentalNotice, Expression, ExternBinding,
    GlobalSymbolId, IntrinsicBinding, LanguageItemBinding, LifetimeAnnotation, LocalNodeId,
    LocalNodeIdAny, NodeTree, ScalarLiteral, SymbolDecorators, SymbolTable, TagMarker, TaintMarker,
    UnrollHint, WellKnownDecorator,
};
use destack_workspace::{Module, ProfileId};

use crate::{AnalyzeError, AnalyzeResult, Compiler};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Register well-known decorator metadata on symbols.
    pub(super) fn register_symbol_decorators(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &mut SymbolTable,
    ) -> AnalyzeResult<()> {
        // map decorator marker symbols to well known ids
        let decorator_map = self.collect_well_known_decorators(profile);

        // snapshot active symbol ids to allow mutation
        let symbol_ids: Vec<_> = symbols.active_symbol_ids().collect();

        // scan every symbol for decorator annotations
        for symbol_id in symbol_ids {
            // gather declaration nodes to inspect
            let declaration_nodes = {
                let symbol = symbols.get_symbol(symbol_id);
                self.collect_symbol_declaration_nodes(tree, symbol)
            };
            if declaration_nodes.is_empty() {
                continue;
            }

            // apply decorator metadata
            let decorators = &mut symbols.get_symbol_mut(symbol_id).decorators;
            for node_id in declaration_nodes {
                self.apply_decorators_for_node(
                    module,
                    profile,
                    tree,
                    &decorator_map,
                    node_id,
                    decorators,
                );
            }
        }

        Ok(())
    }

    /// Collect decorator marker symbols for the active profile.
    pub(crate) fn collect_well_known_decorators(
        &self,
        _profile: ProfileId,
    ) -> HashMap<GlobalSymbolId, WellKnownDecorator> {
        // map language symbols to well known decorators
        let mut decorators = HashMap::new();

        decorators.insert(
            self.language_symbol(LanguageSymbol::Extern),
            WellKnownDecorator::Extern,
        );
        decorators.insert(
            self.language_symbol(LanguageSymbol::Intrinsic),
            WellKnownDecorator::Intrinsic,
        );
        decorators.insert(
            self.language_symbol(LanguageSymbol::Deprecated),
            WellKnownDecorator::Deprecated,
        );
        decorators.insert(
            self.language_symbol(LanguageSymbol::LanguageItem),
            WellKnownDecorator::LanguageItem,
        );
        decorators.insert(
            self.language_symbol(LanguageSymbol::Inline),
            WellKnownDecorator::Inline,
        );
        decorators.insert(
            self.language_symbol(LanguageSymbol::Noinline),
            WellKnownDecorator::Noinline,
        );
        decorators.insert(
            self.language_symbol(LanguageSymbol::Experimental),
            WellKnownDecorator::Experimental,
        );
        decorators.insert(
            self.language_symbol(LanguageSymbol::Allow),
            WellKnownDecorator::Allow,
        );
        decorators.insert(
            self.language_symbol(LanguageSymbol::Warn),
            WellKnownDecorator::Warn,
        );
        decorators.insert(
            self.language_symbol(LanguageSymbol::Deny),
            WellKnownDecorator::Deny,
        );
        decorators.insert(
            self.language_symbol(LanguageSymbol::Forbid),
            WellKnownDecorator::Forbid,
        );
        decorators.insert(
            self.language_symbol(LanguageSymbol::Expect),
            WellKnownDecorator::Expect,
        );
        decorators.insert(
            self.language_symbol(LanguageSymbol::Unroll),
            WellKnownDecorator::Unroll,
        );
        decorators.insert(
            self.language_symbol(LanguageSymbol::Hot),
            WellKnownDecorator::Hot,
        );
        decorators.insert(
            self.language_symbol(LanguageSymbol::Cold),
            WellKnownDecorator::Cold,
        );
        decorators.insert(
            self.language_symbol(LanguageSymbol::Likely),
            WellKnownDecorator::Likely,
        );
        decorators.insert(
            self.language_symbol(LanguageSymbol::Unlikely),
            WellKnownDecorator::Unlikely,
        );
        decorators.insert(
            self.language_symbol(LanguageSymbol::MustUse),
            WellKnownDecorator::MustUse,
        );
        decorators.insert(
            self.language_symbol(LanguageSymbol::Pure),
            WellKnownDecorator::Pure,
        );
        decorators.insert(
            self.language_symbol(LanguageSymbol::Tailcall),
            WellKnownDecorator::Tailcall,
        );
        decorators.insert(
            self.language_symbol(LanguageSymbol::Unsafe),
            WellKnownDecorator::Unsafe,
        );
        decorators.insert(
            self.language_symbol(LanguageSymbol::Transmute),
            WellKnownDecorator::Transmute,
        );
        decorators.insert(
            self.language_symbol(LanguageSymbol::NoManaged),
            WellKnownDecorator::NoManaged,
        );
        decorators.insert(
            self.language_symbol(LanguageSymbol::StackOnly),
            WellKnownDecorator::StackOnly,
        );
        decorators.insert(
            self.language_symbol(LanguageSymbol::Taint),
            WellKnownDecorator::Taint,
        );
        decorators.insert(
            self.language_symbol(LanguageSymbol::Tag),
            WellKnownDecorator::Tag,
        );
        decorators.insert(
            self.language_symbol(LanguageSymbol::Lifetime),
            WellKnownDecorator::Lifetime,
        );
        decorators.insert(
            self.language_symbol(LanguageSymbol::Addrspace),
            WellKnownDecorator::Addrspace,
        );

        decorators
    }

    /// Collect declaration nodes for a symbol and its parent nodes.
    fn collect_symbol_declaration_nodes(
        &self,
        tree: &NodeTree,
        symbol: &destack_dir::Symbol,
    ) -> Vec<LocalNodeIdAny> {
        // collect primary and secondary declarations
        let mut nodes = Vec::new();
        if let Some(declaration) = symbol.primary_declaration {
            nodes.push(declaration.local_id);
        }
        if let Some(secondary) = symbol.secondary_declarations.as_deref() {
            nodes.extend(secondary.iter().map(|node| node.local_id));
        }

        // include parent nodes for member-level decorators
        let mut expanded = Vec::new();
        for node in nodes {
            expanded.push(node);
            if let Some(parent) = tree.get_parent(node.id) {
                expanded.push(parent);
            }
        }

        expanded
    }

    /// Apply decorators attached to a node to the symbol metadata.
    fn apply_decorators_for_node(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        decorator_map: &HashMap<GlobalSymbolId, WellKnownDecorator>,
        node_id: LocalNodeIdAny,
        decorators: &mut SymbolDecorators,
    ) {
        // scan annotations for decorator markers
        let annotations = tree.get_annotations(node_id.id);
        for annotation_id in annotations {
            let annotation = tree.get(annotation_id);
            let Annotation::Decorator {
                left, arguments, ..
            } = annotation
            else {
                continue;
            };

            // resolve decorator marker symbol
            let left_expr = tree.get(*left);
            let target_symbol = match left_expr {
                Expression::LocalReference { target_symbol, .. }
                | Expression::ModuleReference { target_symbol, .. }
                | Expression::GlobalReference { target_symbol, .. } => Some(*target_symbol),
                _ => None,
            };
            let Some(target_symbol) = target_symbol else {
                continue;
            };
            let Some(marker) = decorator_map.get(&target_symbol) else {
                continue;
            };

            // apply decorator metadata
            self.apply_well_known_decorator(
                module,
                profile,
                tree,
                annotation_id,
                *marker,
                arguments.as_ref(),
                decorators,
            );
        }
    }

    /// Apply a well-known decorator to the symbol metadata.
    fn apply_well_known_decorator(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        annotation_id: LocalNodeId<Annotation>,
        marker: WellKnownDecorator,
        arguments: Option<&Vec<LocalNodeId<Argument>>>,
        decorators: &mut SymbolDecorators,
    ) {
        // collect argument values
        let decorator_name = marker.export_name();
        let Some(values) = self.decorator_argument_values(
            module,
            profile,
            tree,
            annotation_id,
            decorator_name,
            arguments,
        ) else {
            return;
        };

        // apply decorator metadata
        match marker {
            WellKnownDecorator::Extern => {
                let Some(name) = self.decorator_string_argument(
                    module,
                    profile,
                    tree,
                    annotation_id,
                    decorator_name,
                    &values,
                ) else {
                    return;
                };
                if let Some(name) = name {
                    if decorators.intrinsic_binding.is_some() {
                        self.report_invalid_well_known_decorator(
                            module,
                            profile,
                            annotation_id,
                            "extern and intrinsic decorators cannot be combined",
                        );
                        return;
                    }

                    let binding = ExternBinding { name: Some(name) };
                    self.merge_extern_binding(module, profile, annotation_id, binding, decorators);
                } else {
                    if decorators.intrinsic_binding.is_some() {
                        self.report_invalid_well_known_decorator(
                            module,
                            profile,
                            annotation_id,
                            "extern and intrinsic decorators cannot be combined",
                        );
                        return;
                    }

                    let binding = ExternBinding { name: None };
                    self.merge_extern_binding(module, profile, annotation_id, binding, decorators);
                }
            }
            WellKnownDecorator::Intrinsic => {
                let Some(name) = self.decorator_string_argument(
                    module,
                    profile,
                    tree,
                    annotation_id,
                    decorator_name,
                    &values,
                ) else {
                    return;
                };
                if let Some(name) = name {
                    if decorators.extern_binding.is_some() {
                        self.report_invalid_well_known_decorator(
                            module,
                            profile,
                            annotation_id,
                            "extern and intrinsic decorators cannot be combined",
                        );
                        return;
                    }

                    let binding = IntrinsicBinding { name: Some(name) };
                    self.merge_intrinsic_binding(
                        module,
                        profile,
                        annotation_id,
                        binding,
                        decorators,
                    );
                } else {
                    if decorators.extern_binding.is_some() {
                        self.report_invalid_well_known_decorator(
                            module,
                            profile,
                            annotation_id,
                            "extern and intrinsic decorators cannot be combined",
                        );
                        return;
                    }

                    let binding = IntrinsicBinding { name: None };
                    self.merge_intrinsic_binding(
                        module,
                        profile,
                        annotation_id,
                        binding,
                        decorators,
                    );
                }
            }
            WellKnownDecorator::LanguageItem => {
                let Some(name) = self.decorator_string_argument(
                    module,
                    profile,
                    tree,
                    annotation_id,
                    decorator_name,
                    &values,
                ) else {
                    return;
                };

                let binding = LanguageItemBinding { name };
                self.merge_language_item_binding(
                    module,
                    profile,
                    annotation_id,
                    binding,
                    decorators,
                );
            }
            WellKnownDecorator::Deprecated => {
                let Some(message) = self.decorator_string_argument(
                    module,
                    profile,
                    tree,
                    annotation_id,
                    decorator_name,
                    &values,
                ) else {
                    return;
                };

                let notice = DeprecatedNotice { message };
                self.merge_deprecated_notice(module, profile, annotation_id, notice, decorators);
            }
            WellKnownDecorator::Experimental => {
                let Some(message) = self.decorator_string_argument(
                    module,
                    profile,
                    tree,
                    annotation_id,
                    decorator_name,
                    &values,
                ) else {
                    return;
                };

                let notice = ExperimentalNotice { message };
                self.merge_experimental_notice(module, profile, annotation_id, notice, decorators);
            }
            WellKnownDecorator::NoManaged => {
                if !values.is_empty() {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        annotation_id,
                        "noManaged decorator does not accept arguments",
                    );
                    return;
                }

                decorators.is_no_managed = true;
            }
            WellKnownDecorator::StackOnly => {
                if !values.is_empty() {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        annotation_id,
                        "stackOnly decorator does not accept arguments",
                    );
                    return;
                }

                decorators.is_stack_only = true;
                decorators.is_no_managed = true;
            }
            WellKnownDecorator::Inline => {
                if !values.is_empty() {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        annotation_id,
                        "inline decorator does not accept arguments",
                    );
                    return;
                }
                if decorators.is_noinline {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        annotation_id,
                        "inline and noinline decorators cannot be combined",
                    );
                    return;
                }

                decorators.is_inline = true;
            }
            WellKnownDecorator::Noinline => {
                if !values.is_empty() {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        annotation_id,
                        "noinline decorator does not accept arguments",
                    );
                    return;
                }
                if decorators.is_inline {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        annotation_id,
                        "inline and noinline decorators cannot be combined",
                    );
                    return;
                }

                decorators.is_noinline = true;
            }
            WellKnownDecorator::Unroll => {
                let Some(factor) = self.decorator_u32_argument(
                    module,
                    profile,
                    tree,
                    annotation_id,
                    decorator_name,
                    &values,
                ) else {
                    return;
                };

                let hint = UnrollHint { factor };
                self.merge_unroll_hint(module, profile, annotation_id, hint, decorators);
            }
            WellKnownDecorator::Hot => {
                if !values.is_empty() {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        annotation_id,
                        "hot decorator does not accept arguments",
                    );
                    return;
                }
                if decorators.is_cold {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        annotation_id,
                        "hot and cold decorators cannot be combined",
                    );
                    return;
                }

                decorators.is_hot = true;
            }
            WellKnownDecorator::Cold => {
                if !values.is_empty() {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        annotation_id,
                        "cold decorator does not accept arguments",
                    );
                    return;
                }
                if decorators.is_hot {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        annotation_id,
                        "hot and cold decorators cannot be combined",
                    );
                    return;
                }

                decorators.is_cold = true;
            }
            WellKnownDecorator::Likely => {
                if !values.is_empty() {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        annotation_id,
                        "likely decorator does not accept arguments",
                    );
                    return;
                }
                if decorators.is_unlikely {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        annotation_id,
                        "likely and unlikely decorators cannot be combined",
                    );
                    return;
                }

                decorators.is_likely = true;
            }
            WellKnownDecorator::Unlikely => {
                if !values.is_empty() {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        annotation_id,
                        "unlikely decorator does not accept arguments",
                    );
                    return;
                }
                if decorators.is_likely {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        annotation_id,
                        "likely and unlikely decorators cannot be combined",
                    );
                    return;
                }

                decorators.is_unlikely = true;
            }
            WellKnownDecorator::MustUse => {
                if !values.is_empty() {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        annotation_id,
                        "mustUse decorator does not accept arguments",
                    );
                    return;
                }

                decorators.is_must_use = true;
            }
            WellKnownDecorator::Pure => {
                if !values.is_empty() {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        annotation_id,
                        "pure decorator does not accept arguments",
                    );
                    return;
                }

                decorators.is_pure = true;
            }
            WellKnownDecorator::Tailcall => {
                if !values.is_empty() {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        annotation_id,
                        "tailcall decorator does not accept arguments",
                    );
                    return;
                }

                decorators.is_tailcall = true;
            }
            WellKnownDecorator::Unsafe => {
                if !values.is_empty() {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        annotation_id,
                        "unsafe decorator does not accept arguments",
                    );
                    return;
                }

                decorators.is_unsafe = true;
            }
            WellKnownDecorator::Transmute => {
                if !values.is_empty() {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        annotation_id,
                        "transmute decorator does not accept arguments",
                    );
                    return;
                }

                decorators.is_transmute = true;
            }
            WellKnownDecorator::Taint => {
                let Some(label) = self.decorator_string_argument(
                    module,
                    profile,
                    tree,
                    annotation_id,
                    decorator_name,
                    &values,
                ) else {
                    return;
                };

                let marker = TaintMarker { label };
                decorators.taints.push(marker);
            }
            WellKnownDecorator::Tag => {
                let Some(label) = self.decorator_string_argument(
                    module,
                    profile,
                    tree,
                    annotation_id,
                    decorator_name,
                    &values,
                ) else {
                    return;
                };

                let marker = TagMarker { label };
                decorators.tags.push(marker);
            }
            WellKnownDecorator::Lifetime => {
                let Some(lifetime) = self.decorator_lifetime_annotation(
                    module,
                    profile,
                    tree,
                    annotation_id,
                    decorator_name,
                    &values,
                ) else {
                    return;
                };
                let Some(lifetime) = lifetime else {
                    return;
                };

                self.merge_lifetime_annotation(
                    module,
                    profile,
                    annotation_id,
                    lifetime,
                    decorators,
                );
            }
            WellKnownDecorator::Allow
            | WellKnownDecorator::Warn
            | WellKnownDecorator::Deny
            | WellKnownDecorator::Forbid
            | WellKnownDecorator::Expect
            | WellKnownDecorator::Addrspace => {}
        }
    }

    /// Collect positional argument values for a decorator.
    fn decorator_argument_values(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        annotation_id: LocalNodeId<Annotation>,
        decorator_name: &str,
        arguments: Option<&Vec<LocalNodeId<Argument>>>,
    ) -> Option<Vec<LocalNodeId<Expression>>> {
        // map arguments to expression values
        let mut values = Vec::new();
        let Some(arguments) = arguments else {
            return Some(values);
        };

        for argument_id in arguments {
            let argument = tree.get(*argument_id);
            let value_id = match argument {
                Argument::Positional { value, .. }
                | Argument::Named { value, .. }
                | Argument::Labeled { value, .. } => *value,
                Argument::Spread { .. } => {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        annotation_id,
                        &format!("{decorator_name} decorator does not support spread arguments"),
                    );
                    return None;
                }
            };
            values.push(value_id);
        }

        Some(values)
    }

    /// Parse a single string argument for a decorator.
    fn decorator_string_argument(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        annotation_id: LocalNodeId<Annotation>,
        decorator_name: &str,
        values: &[LocalNodeId<Expression>],
    ) -> Option<Option<destack_base::StringId>> {
        // validate arity
        if values.is_empty() {
            return Some(None);
        }
        if values.len() != 1 {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                &format!("{decorator_name} decorator expects zero or one argument"),
            );
            return None;
        }

        // extract string literal
        let expr = tree.get(values[0]);
        let Expression::ScalarLiteral {
            value: ScalarLiteral::String(string_id),
        } = expr
        else {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                &format!("{decorator_name} decorator argument must be a string literal"),
            );
            return None;
        };

        Some(Some(*string_id))
    }

    /// Parse a single integer argument for a decorator.
    fn decorator_u32_argument(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        annotation_id: LocalNodeId<Annotation>,
        decorator_name: &str,
        values: &[LocalNodeId<Expression>],
    ) -> Option<Option<u32>> {
        // validate arity
        if values.is_empty() {
            return Some(None);
        }
        if values.len() != 1 {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                &format!("{decorator_name} decorator expects zero or one argument"),
            );
            return None;
        }

        // extract integer literal
        let expr = tree.get(values[0]);
        let value = match expr {
            Expression::ScalarLiteral {
                value: ScalarLiteral::Integer(value),
            } => Some(*value),
            Expression::ScalarLiteral {
                value: ScalarLiteral::Bigint(value),
            } => Some(*value),
            _ => None,
        };
        let Some(value) = value else {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                &format!("{decorator_name} decorator argument must be an integer literal"),
            );
            return None;
        };

        // validate non-negative value
        if value < 0 {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                &format!("{decorator_name} decorator argument must be non-negative"),
            );
            return None;
        }

        Some(Some(value as u32))
    }

    /// Parse lifetime annotation arguments for a decorator.
    fn decorator_lifetime_annotation(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        annotation_id: LocalNodeId<Annotation>,
        decorator_name: &str,
        values: &[LocalNodeId<Expression>],
    ) -> Option<Option<LifetimeAnnotation>> {
        // allow empty arguments (defaults to inferred)
        if values.is_empty() {
            return Some(None);
        }

        // collect parameter names or static marker
        let mut names = Vec::new();
        let mut is_static = false;

        // scan argument values
        for value_id in values {
            let expr = tree.get(*value_id);

            // extract identifier or string literal name
            let name_id = match expr {
                Expression::ScalarLiteral {
                    value: ScalarLiteral::String(string_id),
                } => Some(*string_id),
                Expression::UnresolvedPath { path, .. }
                | Expression::LocalReference { path, .. }
                | Expression::ModuleReference { path, .. }
                | Expression::GlobalReference { path, .. } => {
                    if path.segments.len() == 1 {
                        Some(path.segments[0])
                    } else {
                        None
                    }
                }
                _ => None,
            };
            let Some(name_id) = name_id else {
                self.report_invalid_well_known_decorator(
                    module,
                    profile,
                    annotation_id,
                    &format!(
                        "{decorator_name} decorator arguments must be string literals or \
                         identifiers"
                    ),
                );
                return None;
            };

            // check for static lifetime
            let name = self.program.strings.get(name_id);
            if name == "static" {
                is_static = true;
                continue;
            }

            if !names.contains(&name_id) {
                names.push(name_id);
            }
        }

        // disallow mixing static with other names
        if is_static && !names.is_empty() {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                &format!("{decorator_name} decorator cannot mix static and parameters"),
            );
            return None;
        }

        // emit the final annotation
        if is_static {
            return Some(Some(LifetimeAnnotation::Static));
        }

        if names.is_empty() {
            return Some(None);
        }

        Some(Some(LifetimeAnnotation::Parameters(names)))
    }

    /// Merge an extern binding into the symbol metadata.
    fn merge_extern_binding(
        &self,
        module: &Module,
        profile: ProfileId,
        annotation_id: LocalNodeId<Annotation>,
        binding: ExternBinding,
        decorators: &mut SymbolDecorators,
    ) {
        let Some(existing) = decorators.extern_binding.as_ref() else {
            decorators.extern_binding = Some(binding);
            return;
        };

        if existing != &binding {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                "extern decorator is already set",
            );
        }
    }

    /// Merge a language item binding into the symbol metadata.
    fn merge_language_item_binding(
        &self,
        module: &Module,
        profile: ProfileId,
        annotation_id: LocalNodeId<Annotation>,
        binding: LanguageItemBinding,
        decorators: &mut SymbolDecorators,
    ) {
        let Some(existing) = decorators.language_item.as_ref() else {
            decorators.language_item = Some(binding);
            return;
        };

        // reject conflicting bindings
        if existing != &binding {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                "languageItem decorator is already set",
            );
        }
    }

    /// Merge an intrinsic binding into the symbol metadata.
    fn merge_intrinsic_binding(
        &self,
        module: &Module,
        profile: ProfileId,
        annotation_id: LocalNodeId<Annotation>,
        binding: IntrinsicBinding,
        decorators: &mut SymbolDecorators,
    ) {
        let Some(existing) = decorators.intrinsic_binding.as_ref() else {
            decorators.intrinsic_binding = Some(binding);
            return;
        };

        if existing != &binding {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                "intrinsic decorator is already set",
            );
        }
    }

    /// Merge a lifetime annotation into the symbol metadata.
    fn merge_lifetime_annotation(
        &self,
        module: &Module,
        profile: ProfileId,
        annotation_id: LocalNodeId<Annotation>,
        lifetime: LifetimeAnnotation,
        decorators: &mut SymbolDecorators,
    ) {
        let Some(existing) = decorators.lifetime.as_ref() else {
            decorators.lifetime = Some(lifetime);
            return;
        };

        // reject conflicting annotations
        if existing != &lifetime {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                "lifetime decorator is already set",
            );
        }
    }

    /// Merge a deprecated notice into the symbol metadata.
    fn merge_deprecated_notice(
        &self,
        module: &Module,
        profile: ProfileId,
        annotation_id: LocalNodeId<Annotation>,
        notice: DeprecatedNotice,
        decorators: &mut SymbolDecorators,
    ) {
        let Some(existing) = decorators.deprecated.as_ref() else {
            decorators.deprecated = Some(notice);
            return;
        };

        if existing != &notice {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                "deprecated decorator is already set",
            );
        }
    }

    /// Merge an experimental notice into the symbol metadata.
    fn merge_experimental_notice(
        &self,
        module: &Module,
        profile: ProfileId,
        annotation_id: LocalNodeId<Annotation>,
        notice: ExperimentalNotice,
        decorators: &mut SymbolDecorators,
    ) {
        let Some(existing) = decorators.experimental.as_ref() else {
            decorators.experimental = Some(notice);
            return;
        };

        if existing != &notice {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                "experimental decorator is already set",
            );
        }
    }

    /// Merge an unroll hint into the symbol metadata.
    fn merge_unroll_hint(
        &self,
        module: &Module,
        profile: ProfileId,
        annotation_id: LocalNodeId<Annotation>,
        hint: UnrollHint,
        decorators: &mut SymbolDecorators,
    ) {
        let Some(existing) = decorators.unroll.as_ref() else {
            decorators.unroll = Some(hint);
            return;
        };

        if existing != &hint {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                "unroll decorator is already set",
            );
        }
    }

    /// Emit a diagnostic for an invalid well-known decorator.
    fn report_invalid_well_known_decorator(
        &self,
        module: &Module,
        profile: ProfileId,
        annotation_id: LocalNodeId<Annotation>,
        message: &str,
    ) {
        let message = self.program.strings.intern(message);

        self.error(AnalyzeError::InvalidWellKnownDecorator {
            node: annotation_id
                .into_global_any(module.id)
                .into_anchored(Some(profile)),
            message,
        });
    }
}
