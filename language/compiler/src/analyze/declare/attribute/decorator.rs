use std::collections::HashMap;

use destack_builtin::LanguageSymbol;
use destack_dir::{
    Argument, CaptureTable, Declaration, Decorator, DeprecatedNotice, ExperimentalNotice,
    Expression, ExternBinding, GlobalSymbolId, IntrinsicBinding, LanguageItemBinding, LocalNodeId,
    LocalNodeIdAny, NodeTree, SanitizerMarker, SinkMarker, Symbol, SymbolDecorators, SymbolTable,
    TagMarker, TaintMarker, UnrollHint, WellKnownDecorator,
};
use destack_workspace::ProfileId;

use crate::analyze::common::{CanonicalSymbolMode, ModuleSymbolView, TreeSymbolView};
use crate::{AnalyzeError, AnalyzeResult, Compiler, CompilerContext};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Register well-known decorator metadata on symbols.
    pub(crate) fn register_symbol_decorators(
        &self,
        module: &destack_workspace::Module,
        profile: ProfileId,
        context: &CompilerContext<'_>,
        tree: &NodeTree,
        symbols: &mut SymbolTable,
        captures: &mut CaptureTable,
    ) -> AnalyzeResult<()> {
        // map decorator marker symbols to well known ids
        let decorator_map = self.collect_well_known_decorators(profile);

        // map declaration nodes to expression wrappers for nested decorators
        let declaration_wrappers = self.collect_declaration_wrappers(tree);

        // snapshot active symbol ids to allow mutation
        let symbol_ids: Vec<_> = symbols.active_symbol_ids().collect();

        // scan every symbol for decorator annotations
        for symbol_id in symbol_ids {
            // gather declaration nodes to inspect
            let declaration_nodes = {
                let symbol = symbols.get_symbol(symbol_id);
                self.collect_symbol_declaration_nodes(tree, symbol, &declaration_wrappers)
            };
            if declaration_nodes.is_empty() {
                continue;
            }

            // apply decorator metadata
            let mut decorators = {
                let symbol = symbols.get_symbol_mut(symbol_id);
                std::mem::take(&mut symbol.decorators)
            };
            for node_id in declaration_nodes {
                self.apply_decorators_for_node(
                    TreeSymbolView::new(context, module, profile, tree, symbols),
                    &decorator_map,
                    node_id,
                    symbol_id.into_global(module.id),
                    &mut decorators,
                    captures,
                )?;
            }
            symbols.get_symbol_mut(symbol_id).decorators = decorators;
        }

        Ok(())
    }

    /// Collect decorator marker symbols for the active profile.
    pub(crate) fn collect_well_known_decorators(
        &self,
        profile: ProfileId,
    ) -> HashMap<GlobalSymbolId, WellKnownDecorator> {
        // map language symbols to well known decorators
        let mut decorators = HashMap::new();

        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::Binding),
            WellKnownDecorator::Binding,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::Extern),
            WellKnownDecorator::Extern,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::Intrinsic),
            WellKnownDecorator::Intrinsic,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::Deprecated),
            WellKnownDecorator::Deprecated,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::LanguageItem),
            WellKnownDecorator::LanguageItem,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::Inline),
            WellKnownDecorator::Inline,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::Noinline),
            WellKnownDecorator::Noinline,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::Experimental),
            WellKnownDecorator::Experimental,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::Allow),
            WellKnownDecorator::Allow,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::Warn),
            WellKnownDecorator::Warn,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::Deny),
            WellKnownDecorator::Deny,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::Forbid),
            WellKnownDecorator::Forbid,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::Expect),
            WellKnownDecorator::Expect,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::Unroll),
            WellKnownDecorator::Unroll,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::Hot),
            WellKnownDecorator::Hot,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::Cold),
            WellKnownDecorator::Cold,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::Likely),
            WellKnownDecorator::Likely,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::Unlikely),
            WellKnownDecorator::Unlikely,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::MustUse),
            WellKnownDecorator::MustUse,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::Pure),
            WellKnownDecorator::Pure,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::Tailcall),
            WellKnownDecorator::Tailcall,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::Unsafe),
            WellKnownDecorator::Unsafe,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::Transmute),
            WellKnownDecorator::Transmute,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::NoManaged),
            WellKnownDecorator::NoManaged,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::StackOnly),
            WellKnownDecorator::StackOnly,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::Capture),
            WellKnownDecorator::Capture,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::Taint),
            WellKnownDecorator::Taint,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::Sink),
            WellKnownDecorator::Sink,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::Sanitizer),
            WellKnownDecorator::Sanitizer,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::Tag),
            WellKnownDecorator::Tag,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::Lifetime),
            WellKnownDecorator::Lifetime,
        );
        decorators.insert(
            self.language_symbol(profile, LanguageSymbol::Space),
            WellKnownDecorator::Space,
        );

        decorators
    }

    /// Collect declaration nodes for a symbol and its parent nodes.
    fn collect_symbol_declaration_nodes(
        &self,
        tree: &NodeTree,
        symbol: &Symbol,
        declaration_wrappers: &HashMap<LocalNodeIdAny, Vec<LocalNodeId<Expression>>>,
    ) -> Vec<LocalNodeIdAny> {
        // collect primary and secondary declarations
        let mut nodes = Vec::new();
        if let Some(declaration) = symbol.primary_declaration {
            nodes.push(declaration.local_id);
        }
        if let Some(secondary) = symbol.secondary_declarations.as_deref() {
            nodes.extend(secondary.iter().map(|node| node.local_id));
        }

        // include expression wrappers for declaration sites
        let mut expanded = Vec::new();
        for node in nodes {
            expanded.push(node);
            if let Ok(declaration_id) = node.try_into_typed::<Declaration>()
                && let Some(wrappers) = declaration_wrappers.get(&declaration_id.into_any())
            {
                expanded.extend(wrappers.iter().map(|wrapper| wrapper.into_any()));
            }

            // include parent nodes for member-level decorators
            if let Some(parent) = tree.get_parent(node.id) {
                expanded.push(parent);
            }
        }

        expanded
    }

    /// Collect expression wrappers for declaration nodes.
    fn collect_declaration_wrappers(
        &self,
        tree: &NodeTree,
    ) -> HashMap<LocalNodeIdAny, Vec<LocalNodeId<Expression>>> {
        // index expression wrappers for declaration nodes
        let mut wrappers = HashMap::new();
        for (expression_id, expression) in tree.iter_nodes_of_type::<Expression>() {
            if let Expression::Declaration(declaration) = expression {
                wrappers
                    .entry(declaration.into_any())
                    .or_insert_with(Vec::new)
                    .push(expression_id);
            }
        }

        wrappers
    }

    /// Apply decorators attached to a node to the symbol metadata.
    fn apply_decorators_for_node(
        &self,
        ctx: TreeSymbolView<'_>,
        decorator_map: &HashMap<GlobalSymbolId, WellKnownDecorator>,
        node_id: LocalNodeIdAny,
        symbol_id: GlobalSymbolId,
        decorators: &mut SymbolDecorators,
        captures: &mut CaptureTable,
    ) -> AnalyzeResult<()> {
        // scan decorators for marker calls
        let decorator_ids = ctx.tree.get_decorators(node_id.id);
        for decorator_id in decorator_ids {
            let decorator = ctx.tree.get(decorator_id);
            let expression = decorator.expression;

            // resolve decorator marker symbol
            let call = self.decorator_call(ctx.tree, expression);
            let callee_expr = ctx.tree.get(call.callee);
            let target_symbol = match callee_expr {
                Expression::LocalReference { target_symbol, .. }
                | Expression::ModuleReference { target_symbol, .. }
                | Expression::GlobalReference { target_symbol, .. } => Some(*target_symbol),
                _ => None,
            };
            let Some(target_symbol) = target_symbol else {
                continue;
            };

            // compare well-known markers using canonical symbol ids
            let target_symbol = self.canonical_symbol_id(
                ctx.module_symbol_view(),
                target_symbol,
                CanonicalSymbolMode::FollowAliases,
            );
            let mut marker = decorator_map.get(&target_symbol).copied();
            if marker.is_none() {
                // look for a well known decorator in the merge group
                marker = self.find_decorator_marker_in_merge_group(
                    ctx.module_symbol_view(),
                    decorator_map,
                    target_symbol,
                )?;
            }
            if marker.is_none() && self.is_builtin_decorator_module(ctx.profile, target_symbol) {
                self.report_invalid_well_known_decorator(
                    ctx.module,
                    ctx.profile,
                    decorator_id,
                    "unknown builtin decorator marker",
                );
            }
            let Some(marker) = marker else {
                continue;
            };

            // apply decorator metadata
            self.apply_well_known_decorator(
                ctx.module,
                ctx.profile,
                ctx.tree,
                decorator_id,
                node_id,
                marker,
                call.arguments,
                decorators,
                symbol_id,
                captures,
            );
        }

        Ok(())
    }

    /// Resolve a decorator marker from a symbol's merge group.
    fn find_decorator_marker_in_merge_group(
        &self,
        view: ModuleSymbolView<'_>,
        decorator_map: &HashMap<GlobalSymbolId, WellKnownDecorator>,
        target_symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Option<WellKnownDecorator>> {
        self.with_module_symbols_or_local_for_artifact(
            view.compiler_context,
            view.module,
            view.profile,
            target_symbol.module_id,
            view.symbols,
            destack_artifact::ArtifactKey::dir_declared,
            |owner_module, owner_symbols| {
                let symbol_entry = owner_symbols.get_symbol(target_symbol.local_id);
                let group_id = symbol_entry.merge_group?;

                // scan merge group for a well known decorator
                for symbol_id in owner_symbols.merge_group_symbols(group_id) {
                    let symbol_id = symbol_id.into_global(owner_module.id);
                    if let Some(marker) = decorator_map.get(&symbol_id) {
                        return Some(*marker);
                    }
                }

                None
            },
        )
        .map_err(AnalyzeError::from)
    }

    /// Check whether a symbol lives in the builtin decorator module.
    fn is_builtin_decorator_module(
        &self,
        profile: ProfileId,
        target_symbol: GlobalSymbolId,
    ) -> bool {
        let decorator_symbol = self.language_symbol(profile, LanguageSymbol::Extern);
        target_symbol.module_id == decorator_symbol.module_id
    }

    /// Apply a well-known decorator to the symbol metadata.
    fn apply_well_known_decorator(
        &self,
        module: &destack_workspace::Module,
        profile: ProfileId,
        tree: &NodeTree,
        decorator_id: LocalNodeId<Decorator>,
        node_id: LocalNodeIdAny,
        marker: WellKnownDecorator,
        arguments: Option<&[LocalNodeId<Argument>]>,
        decorators: &mut SymbolDecorators,
        symbol_id: GlobalSymbolId,
        captures: &mut CaptureTable,
    ) {
        // collect argument values
        let decorator_name = marker.export_name();
        let Some(values) = self.decorator_argument_values(
            module,
            profile,
            tree,
            decorator_id,
            decorator_name,
            arguments,
        ) else {
            return;
        };

        // apply decorator metadata
        match marker {
            WellKnownDecorator::Binding => {
                let Some(binding) =
                    self.decorator_binding_argument(module, profile, tree, decorator_id, &values)
                else {
                    return;
                };

                // reject conflicting binding forms
                if decorators.intrinsic_binding.is_some() || decorators.extern_binding.is_some() {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        decorator_id,
                        "binding cannot be combined with extern or intrinsic",
                    );
                    return;
                }

                self.merge_binding(module, profile, decorator_id, binding, decorators);
            }
            WellKnownDecorator::Extern => {
                let Some(name) = self.decorator_string_argument(
                    module,
                    profile,
                    tree,
                    decorator_id,
                    decorator_name,
                    &values,
                ) else {
                    return;
                };
                if let Some(name) = name {
                    if decorators.intrinsic_binding.is_some() || decorators.binding.is_some() {
                        self.report_invalid_well_known_decorator(
                            module,
                            profile,
                            decorator_id,
                            "extern cannot be combined with binding or intrinsic",
                        );
                        return;
                    }

                    let binding = ExternBinding { name: Some(name) };
                    self.merge_extern_binding(module, profile, decorator_id, binding, decorators);
                } else {
                    if decorators.intrinsic_binding.is_some() || decorators.binding.is_some() {
                        self.report_invalid_well_known_decorator(
                            module,
                            profile,
                            decorator_id,
                            "extern cannot be combined with binding or intrinsic",
                        );
                        return;
                    }

                    let binding = ExternBinding { name: None };
                    self.merge_extern_binding(module, profile, decorator_id, binding, decorators);
                }
            }
            WellKnownDecorator::Intrinsic => {
                // keep intrinsic bindings confined to builtin modules
                if !module.is_builtin() {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        decorator_id,
                        "intrinsic decorators are only supported in builtin modules",
                    );
                    return;
                }

                let Some(name) = self.decorator_string_argument(
                    module,
                    profile,
                    tree,
                    decorator_id,
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
                            decorator_id,
                            "extern and intrinsic decorators cannot be combined",
                        );
                        return;
                    }

                    let binding = IntrinsicBinding { name: Some(name) };
                    self.merge_intrinsic_binding(
                        module,
                        profile,
                        decorator_id,
                        binding,
                        decorators,
                    );
                } else {
                    if decorators.extern_binding.is_some() {
                        self.report_invalid_well_known_decorator(
                            module,
                            profile,
                            decorator_id,
                            "extern and intrinsic decorators cannot be combined",
                        );
                        return;
                    }

                    let binding = IntrinsicBinding { name: None };
                    self.merge_intrinsic_binding(
                        module,
                        profile,
                        decorator_id,
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
                    decorator_id,
                    decorator_name,
                    &values,
                ) else {
                    return;
                };

                let binding = LanguageItemBinding { name };
                self.merge_language_item_binding(
                    module,
                    profile,
                    decorator_id,
                    binding,
                    decorators,
                );
            }
            WellKnownDecorator::Deprecated => {
                let Some(message) = self.decorator_string_argument(
                    module,
                    profile,
                    tree,
                    decorator_id,
                    decorator_name,
                    &values,
                ) else {
                    return;
                };

                let notice = DeprecatedNotice { message };
                self.merge_deprecated_notice(module, profile, decorator_id, notice, decorators);
            }
            WellKnownDecorator::Experimental => {
                let Some(message) = self.decorator_string_argument(
                    module,
                    profile,
                    tree,
                    decorator_id,
                    decorator_name,
                    &values,
                ) else {
                    return;
                };

                let notice = ExperimentalNotice { message };
                self.merge_experimental_notice(module, profile, decorator_id, notice, decorators);
            }
            WellKnownDecorator::NoManaged => {
                if !values.is_empty() {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        decorator_id,
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
                        decorator_id,
                        "stackOnly decorator does not accept arguments",
                    );
                    return;
                }

                decorators.is_stack_only = true;
                decorators.is_no_managed = true;
            }
            WellKnownDecorator::Capture => {
                let Some(declaration_id) =
                    self.capture_decorator_target(module, profile, tree, decorator_id, node_id)
                else {
                    return;
                };

                let Some(directive) = self.decorator_capture_directive(
                    module,
                    profile,
                    tree,
                    decorator_id,
                    decorator_name,
                    &values,
                ) else {
                    return;
                };

                let declaration = tree.get(declaration_id);
                if !matches!(declaration, Declaration::Function { .. }) {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        decorator_id,
                        "capture decorator is only supported on functions",
                    );
                    return;
                }

                if let Some(existing) = captures.capture_directive(symbol_id) {
                    if existing != &directive {
                        self.report_invalid_well_known_decorator(
                            module,
                            profile,
                            decorator_id,
                            "capture decorator is already defined for this function",
                        );
                    }
                    return;
                }

                captures.set_capture_directive(symbol_id, directive);
            }
            WellKnownDecorator::Inline => {
                if !values.is_empty() {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        decorator_id,
                        "inline decorator does not accept arguments",
                    );
                    return;
                }
                if decorators.is_noinline {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        decorator_id,
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
                        decorator_id,
                        "noinline decorator does not accept arguments",
                    );
                    return;
                }
                if decorators.is_inline {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        decorator_id,
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
                    decorator_id,
                    decorator_name,
                    &values,
                ) else {
                    return;
                };

                let hint = UnrollHint { factor };
                self.merge_unroll_hint(module, profile, decorator_id, hint, decorators);
            }
            WellKnownDecorator::Hot => {
                if !values.is_empty() {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        decorator_id,
                        "hot decorator does not accept arguments",
                    );
                    return;
                }
                if decorators.is_cold {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        decorator_id,
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
                        decorator_id,
                        "cold decorator does not accept arguments",
                    );
                    return;
                }
                if decorators.is_hot {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        decorator_id,
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
                        decorator_id,
                        "likely decorator does not accept arguments",
                    );
                    return;
                }
                if decorators.is_unlikely {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        decorator_id,
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
                        decorator_id,
                        "unlikely decorator does not accept arguments",
                    );
                    return;
                }
                if decorators.is_likely {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        decorator_id,
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
                        decorator_id,
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
                        decorator_id,
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
                        decorator_id,
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
                        decorator_id,
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
                        decorator_id,
                        "transmute decorator does not accept arguments",
                    );
                    return;
                }

                decorators.is_transmute = true;
            }
            WellKnownDecorator::Taint => {
                let Some(labels) = self.decorator_string_arguments(
                    module,
                    profile,
                    tree,
                    decorator_id,
                    decorator_name,
                    &values,
                ) else {
                    return;
                };

                // unlabeled taint marks a generic source
                if labels.is_empty() {
                    let marker = TaintMarker { label: None };
                    if !decorators.taints.contains(&marker) {
                        decorators.taints.push(marker);
                    }

                    return;
                }

                // each label becomes one taint marker
                for label in labels {
                    let marker = TaintMarker { label: Some(label) };
                    if !decorators.taints.contains(&marker) {
                        decorators.taints.push(marker);
                    }
                }
            }
            WellKnownDecorator::Sink => {
                let Some(labels) = self.decorator_string_arguments(
                    module,
                    profile,
                    tree,
                    decorator_id,
                    decorator_name,
                    &values,
                ) else {
                    return;
                };

                // unlabeled sink accepts any taint label
                if labels.is_empty() {
                    let marker = SinkMarker { label: None };
                    if !decorators.sinks.contains(&marker) {
                        decorators.sinks.push(marker);
                    }

                    return;
                }

                // each label becomes one sink marker
                for label in labels {
                    let marker = SinkMarker { label: Some(label) };
                    if !decorators.sinks.contains(&marker) {
                        decorators.sinks.push(marker);
                    }
                }
            }
            WellKnownDecorator::Sanitizer => {
                let Some(labels) = self.decorator_string_arguments(
                    module,
                    profile,
                    tree,
                    decorator_id,
                    decorator_name,
                    &values,
                ) else {
                    return;
                };

                // unlabeled sanitizer clears all taint labels
                if labels.is_empty() {
                    let marker = SanitizerMarker { label: None };
                    if !decorators.sanitizers.contains(&marker) {
                        decorators.sanitizers.push(marker);
                    }

                    return;
                }

                // each label becomes one sanitizer marker
                for label in labels {
                    let marker = SanitizerMarker { label: Some(label) };
                    if !decorators.sanitizers.contains(&marker) {
                        decorators.sanitizers.push(marker);
                    }
                }
            }
            WellKnownDecorator::Tag => {
                let Some(label) = self.decorator_string_argument(
                    module,
                    profile,
                    tree,
                    decorator_id,
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
                    decorator_id,
                    decorator_name,
                    &values,
                ) else {
                    return;
                };
                let Some(lifetime) = lifetime else {
                    return;
                };

                self.merge_lifetime_annotation(module, profile, decorator_id, lifetime, decorators);
            }
            WellKnownDecorator::Allow
            | WellKnownDecorator::Warn
            | WellKnownDecorator::Deny
            | WellKnownDecorator::Forbid
            | WellKnownDecorator::Expect
            | WellKnownDecorator::Space => {}
        }
    }
}
