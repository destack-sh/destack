use std::collections::{HashMap, HashSet};

use destack_base::StringId;
use destack_builtin::LanguageSymbol;
use destack_dir::{
    Annotation, Argument, CaptureDirective, CaptureKind, CapturePolicy, CaptureRule, CaptureTable,
    Declaration, DeprecatedNotice, DynamicKey, ExperimentalNotice, Expression, ExternBinding,
    GlobalSymbolId, IntrinsicBinding, LanguageItemBinding, LifetimeAnnotation, LocalNodeId,
    LocalNodeIdAny, NodeTree, Property, SanitizerMarker, ScalarLiteral, SinkMarker,
    SymbolDecorators, SymbolTable, TagMarker, TaintMarker, UnrollHint, WellKnownDecorator,
};
use destack_workspace::{Module, ProfileId};

use crate::analyze::common::CanonicalSymbolMode;
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
                    module,
                    profile,
                    tree,
                    &decorator_map,
                    node_id,
                    symbol_id.into_global(module.id),
                    &mut decorators,
                    symbols,
                    captures,
                );
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
            self.language_symbol(profile, LanguageSymbol::Addrspace),
            WellKnownDecorator::Addrspace,
        );

        decorators
    }

    /// Collect declaration nodes for a symbol and its parent nodes.
    fn collect_symbol_declaration_nodes(
        &self,
        tree: &NodeTree,
        symbol: &destack_dir::Symbol,
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
            match expression {
                Expression::Declaration { declaration } => {
                    wrappers
                        .entry(declaration.into_any())
                        .or_insert_with(Vec::new)
                        .push(expression_id);
                }
                Expression::Statement { statement } => {
                    let Expression::Declaration { declaration } = tree.get(*statement) else {
                        continue;
                    };
                    wrappers
                        .entry(declaration.into_any())
                        .or_insert_with(Vec::new)
                        .push(expression_id);
                }
                _ => {}
            }
        }

        wrappers
    }

    /// Apply decorators attached to a node to the symbol metadata.
    fn apply_decorators_for_node(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        decorator_map: &HashMap<GlobalSymbolId, WellKnownDecorator>,
        node_id: LocalNodeIdAny,
        symbol_id: GlobalSymbolId,
        decorators: &mut SymbolDecorators,
        symbols: &SymbolTable,
        captures: &mut CaptureTable,
    ) {
        // scan annotations for decorator markers
        let annotations = tree.get_annotations(node_id.id);
        for annotation_id in annotations {
            let annotation = tree.get(annotation_id);
            let Annotation::Decorator { expression, .. } = annotation else {
                continue;
            };

            // resolve decorator marker symbol
            let call = self.decorator_call(tree, *expression);
            let callee_expr = tree.get(call.callee);
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
                module,
                symbols,
                profile,
                target_symbol,
                CanonicalSymbolMode::FollowAliases,
            );
            let mut marker = decorator_map.get(&target_symbol).copied();
            if marker.is_none() {
                // look for a well known decorator in the merge group
                marker = self.find_decorator_marker_in_merge_group(
                    module,
                    profile,
                    symbols,
                    decorator_map,
                    target_symbol,
                );
            }
            if marker.is_none() && self.is_builtin_decorator_module(profile, target_symbol) {
                self.report_invalid_well_known_decorator(
                    module,
                    profile,
                    annotation_id,
                    "unknown builtin decorator marker",
                );
            }
            let Some(marker) = marker else {
                continue;
            };

            // apply decorator metadata
            self.apply_well_known_decorator(
                module,
                profile,
                tree,
                annotation_id,
                node_id,
                marker,
                call.arguments,
                decorators,
                symbol_id,
                captures,
            );
        }
    }

    /// Resolve a decorator marker from a symbol's merge group.
    fn find_decorator_marker_in_merge_group(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        decorator_map: &HashMap<GlobalSymbolId, WellKnownDecorator>,
        target_symbol: GlobalSymbolId,
    ) -> Option<WellKnownDecorator> {
        self.with_module_symbols_or_local(
            module,
            profile,
            target_symbol.module_id,
            symbols,
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
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        annotation_id: LocalNodeId<Annotation>,
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
                // keep intrinsic bindings confined to builtin modules
                if !module.is_builtin() {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        annotation_id,
                        "intrinsic decorators are only supported in builtin modules",
                    );
                    return;
                }

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
            WellKnownDecorator::Capture => {
                let Some(declaration_id) =
                    self.capture_decorator_target(module, profile, tree, annotation_id, node_id)
                else {
                    return;
                };

                let Some(directive) = self.decorator_capture_directive(
                    module,
                    profile,
                    tree,
                    annotation_id,
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
                        annotation_id,
                        "capture decorator is only supported on functions",
                    );
                    return;
                }

                if let Some(existing) = captures.capture_directive(symbol_id) {
                    if existing != &directive {
                        self.report_invalid_well_known_decorator(
                            module,
                            profile,
                            annotation_id,
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
                let Some(labels) = self.decorator_string_arguments(
                    module,
                    profile,
                    tree,
                    annotation_id,
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
                    annotation_id,
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
                    annotation_id,
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
        arguments: Option<&[LocalNodeId<Argument>]>,
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
    ) -> Option<Option<StringId>> {
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

    /// Parse zero or more string arguments for a decorator.
    fn decorator_string_arguments(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        annotation_id: LocalNodeId<Annotation>,
        decorator_name: &str,
        values: &[LocalNodeId<Expression>],
    ) -> Option<Vec<StringId>> {
        // allow empty argument lists
        if values.is_empty() {
            return Some(Vec::new());
        }

        // collect unique string literal arguments
        let mut labels = Vec::new();
        for value_id in values {
            let expression = tree.get(*value_id);
            let Expression::ScalarLiteral {
                value: ScalarLiteral::String(string_id),
            } = expression
            else {
                self.report_invalid_well_known_decorator(
                    module,
                    profile,
                    annotation_id,
                    &format!("{decorator_name} decorator arguments must be string literals"),
                );
                return None;
            };

            if !labels.contains(string_id) {
                labels.push(*string_id);
            }
        }

        Some(labels)
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

    /// Resolve the declaration targeted by a capture decorator.
    fn capture_decorator_target(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        annotation_id: LocalNodeId<Annotation>,
        node_id: LocalNodeIdAny,
    ) -> Option<LocalNodeId<Declaration>> {
        // accept direct declaration nodes
        if let Ok(declaration_id) = node_id.try_into_typed::<Declaration>() {
            return Some(declaration_id);
        }

        // accept expression nodes that wrap a declaration
        if let Ok(expression_id) = node_id.try_into_typed::<Expression>()
            && let Expression::Declaration { declaration } = tree.get(expression_id)
        {
            return Some(*declaration);
        }

        self.report_invalid_well_known_decorator(
            module,
            profile,
            annotation_id,
            "capture decorator is only supported on function declarations",
        );

        None
    }

    /// Parse capture directive arguments for a decorator.
    fn decorator_capture_directive(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        annotation_id: LocalNodeId<Annotation>,
        decorator_name: &str,
        values: &[LocalNodeId<Expression>],
    ) -> Option<CaptureDirective> {
        // validate arity
        if values.is_empty() {
            return Some(CaptureDirective::default());
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

        // unwrap trivial wrappers
        let expression_id = self.unwrap_capture_argument(tree, values[0]);
        let expression = tree.get(expression_id);

        // parse string literal policy arguments
        if let Expression::ScalarLiteral {
            value: ScalarLiteral::String(string_id),
        } = expression
        {
            let policy =
                self.capture_policy_from_string(module, profile, annotation_id, *string_id)?;
            return Some(CaptureDirective {
                policy,
                rules: Vec::new(),
            });
        }

        // parse object literal overrides
        if let Expression::ObjectExpression { properties } = expression {
            return self.capture_directive_from_object_literal(
                module,
                profile,
                tree,
                annotation_id,
                decorator_name,
                properties,
            );
        }

        // reject non literal arguments
        self.report_invalid_well_known_decorator(
            module,
            profile,
            annotation_id,
            &format!("{decorator_name} decorator argument must be a string or object literal"),
        );
        None
    }

    /// Parse a capture directive from an object literal expression.
    fn capture_directive_from_object_literal(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        annotation_id: LocalNodeId<Annotation>,
        decorator_name: &str,
        properties: &[LocalNodeId<Property>],
    ) -> Option<CaptureDirective> {
        // initialize capture directive state
        let mut directive = CaptureDirective::default();
        let mut seen_names = HashSet::new();

        // validate object literal fields
        for property_id in properties {
            let property = tree.get(*property_id);
            let Property::Field { key, value, .. } = property else {
                self.report_invalid_well_known_decorator(
                    module,
                    profile,
                    annotation_id,
                    &format!("{decorator_name} decorator only supports field properties"),
                );
                return None;
            };

            // reject dynamic or invalid names
            let Some(DynamicKey::Name(name)) = key else {
                self.report_invalid_well_known_decorator(
                    module,
                    profile,
                    annotation_id,
                    &format!("{decorator_name} decorator requires string property names"),
                );
                return None;
            };
            if !seen_names.insert(*name) {
                self.report_invalid_well_known_decorator(
                    module,
                    profile,
                    annotation_id,
                    &format!("{decorator_name} decorator contains duplicate keys"),
                );
                return None;
            }
            let Some(value_id) = value else {
                self.report_invalid_well_known_decorator(
                    module,
                    profile,
                    annotation_id,
                    &format!("{decorator_name} decorator values must be string literals"),
                );
                return None;
            };

            // get capture value
            let value_id = self.unwrap_capture_argument(tree, *value_id);
            let Expression::ScalarLiteral {
                value: ScalarLiteral::String(value_id),
            } = tree.get(value_id)
            else {
                self.report_invalid_well_known_decorator(
                    module,
                    profile,
                    annotation_id,
                    &format!("{decorator_name} decorator values must be string literals"),
                );
                return None;
            };

            // set policy for that name
            if self.program.strings.get(*name) == "default" {
                let policy =
                    self.capture_policy_from_string(module, profile, annotation_id, *value_id)?;
                directive.policy = policy;
            } else {
                let kind =
                    self.capture_kind_from_string(module, profile, annotation_id, *value_id)?;
                directive.rules.push(CaptureRule { name: *name, kind });
            }
        }

        Some(directive)
    }

    /// Resolve the innermost capture argument expression.
    fn unwrap_capture_argument(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        // walk through trivial wrappers
        let mut current = expression_id;
        loop {
            let expression = tree.get(current);
            match expression {
                Expression::Parenthesized { expression } => {
                    current = *expression;
                }
                Expression::Cast { value, .. } => {
                    current = *value;
                }
                Expression::OwnershipCast { value, .. } => {
                    current = *value;
                }
                _ => return current,
            }
        }
    }

    /// Parse a capture policy from a string literal.
    fn capture_policy_from_string(
        &self,
        module: &Module,
        profile: ProfileId,
        annotation_id: LocalNodeId<Annotation>,
        value: StringId,
    ) -> Option<CapturePolicy> {
        // compare without holding the string pool lock during diagnostics
        let value = self.program.strings.get(value);
        let is_by_value = &*value == "byValue";
        let is_by_reference = &*value == "byReference";
        let is_by_move = &*value == "byMove";
        drop(value);

        let policy = if is_by_value {
            CapturePolicy::ByValue
        } else if is_by_reference {
            CapturePolicy::ByReference
        } else if is_by_move {
            CapturePolicy::ByMove
        } else {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                "capture policy must be \"byValue\", \"byReference\", or \"byMove\"",
            );
            return None;
        };

        Some(policy)
    }

    /// Parse a capture kind from a string literal.
    fn capture_kind_from_string(
        &self,
        module: &Module,
        profile: ProfileId,
        annotation_id: LocalNodeId<Annotation>,
        value: StringId,
    ) -> Option<CaptureKind> {
        // compare without holding the string pool lock during diagnostics
        let value = self.program.strings.get(value);
        let is_by_value = &*value == "byValue";
        let is_by_reference = &*value == "byReference";
        let is_by_move = &*value == "byMove";
        drop(value);

        let kind = if is_by_value {
            CaptureKind::ByValue
        } else if is_by_reference {
            CaptureKind::ByReference
        } else if is_by_move {
            CaptureKind::ByMove
        } else {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                "capture kind must be \"byValue\", \"byReference\", or \"byMove\"",
            );
            return None;
        };

        Some(kind)
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
