use std::collections::{BTreeMap, HashMap, HashSet};
use std::str::FromStr;

use destack_codegen_js as js;
use destack_source::ModuleId;

use super::super::linker::OutputModule;
use super::linker::Rewriter;
use super::source::{MinifySourceContext, OutputScopeId};
use super::statement::DeclarationDescriptorAccess;
use crate::{LinkError, LinkResult, ScriptLinker};

/// The first character alphabet for minified identifiers.
const MINIFIED_IDENTIFIER_START: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ$_";

/// The trailing character alphabet for minified identifiers.
const MINIFIED_IDENTIFIER_CONTINUE: &[u8] =
    b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ$_0123456789";

/// One collected binding entry for identifier assignment.
#[derive(Debug, Clone)]
pub(super) struct BindingEntry {
    /// The original binding name.
    original_name: String,
    /// The scope that owns the binding.
    scope_id: OutputScopeId,
    /// Whether this binding name must be preserved.
    is_preserved: bool,
    /// The accumulated usage count for name ordering.
    use_count: u32,
}

/// One stable allocator for minified identifier names.
#[derive(Debug, Clone, Default)]
struct IdentifierNameAllocator {
    /// The next name slot to assign.
    next_index: usize,
}

impl IdentifierNameAllocator {
    /// Return the next stable minified identifier name.
    fn next_name(&mut self) -> String {
        let mut index = self.next_index;
        self.next_index += 1;

        let mut name = String::new();
        let first = MINIFIED_IDENTIFIER_START[index % MINIFIED_IDENTIFIER_START.len()] as char;
        name.push(first);
        index /= MINIFIED_IDENTIFIER_START.len();

        while index > 0 {
            index -= 1;
            let next =
                MINIFIED_IDENTIFIER_CONTINUE[index % MINIFIED_IDENTIFIER_CONTINUE.len()] as char;
            name.push(next);
            index /= MINIFIED_IDENTIFIER_CONTINUE.len();
        }

        name
    }
}

impl ScriptLinker<'_> {
    /// Return whether one declaration name must stay stable for runtime `.name` semantics.
    fn declaration_keeps_name(&self, declaration: &js::Declaration) -> bool {
        if !self.target.minify.keep_names {
            return false;
        }

        matches!(
            declaration,
            js::Declaration::Class(_) | js::Declaration::Function(_)
        )
    }

    /// Rewrite synthetic non-code default bindings across one linked output.
    pub(in super::super) fn rewrite_module_defaults(
        &self,
        modules: &mut [OutputModule],
    ) -> LinkResult<()> {
        let mut used_names = self.collect_output_identifier_names(modules);
        let mut rename_by_symbol = HashMap::<js::ScriptSymbolId, String>::new();
        let ordered_symbols = self.collect_output_module_default_symbols(modules);

        // assign stable local names without leaking module ids into output
        for (symbol_index, symbol_id) in ordered_symbols.into_iter().enumerate() {
            let mut suffix = if symbol_index == 0 {
                None
            } else {
                Some(symbol_index + 1)
            };

            loop {
                let candidate = match suffix {
                    Some(suffix) => format!("{}{suffix}", js::MODULE_DEFAULT_NAME),
                    None => js::MODULE_DEFAULT_NAME.to_string(),
                };

                if !used_names.contains(&candidate)
                    && !Self::is_reserved_identifier_name(&candidate)
                {
                    used_names.insert(candidate.clone());
                    rename_by_symbol.insert(symbol_id, candidate);
                    break;
                }

                let next_suffix = suffix.unwrap_or(1) + 1;
                suffix = Some(next_suffix);
            }
        }

        if rename_by_symbol.is_empty() {
            return Ok(());
        }

        let source_contexts = HashMap::new();

        // apply the normalized names before printing, even without minification
        for (_, module) in modules.iter_mut() {
            self.apply_minified_identifier_names(module, &source_contexts, &rename_by_symbol)?;
        }

        Ok(())
    }

    /// Collect the synthetic non-code default symbols used in one linked output.
    fn collect_output_module_default_symbols(
        &self,
        modules: &[OutputModule],
    ) -> Vec<js::ScriptSymbolId> {
        let mut symbols = Vec::new();
        let mut seen_symbols = HashSet::new();

        // wrapper bindings and retargeted references live on these node families
        for (_, module) in modules {
            for pattern_id in module.tree.get_nodes::<js::Pattern>() {
                let Some(symbol_id) = module.tree.symbol(pattern_id) else {
                    continue;
                };

                if !matches!(symbol_id, js::ScriptSymbolId::ModuleDefault(_)) {
                    continue;
                }

                if seen_symbols.insert(symbol_id) {
                    symbols.push(symbol_id);
                }
            }

            for expression_id in module.tree.get_nodes::<js::Expression>() {
                let Some(symbol_id) = module.tree.symbol(expression_id) else {
                    continue;
                };

                if !matches!(symbol_id, js::ScriptSymbolId::ModuleDefault(_)) {
                    continue;
                }

                if seen_symbols.insert(symbol_id) {
                    symbols.push(symbol_id);
                }
            }

            for item_id in module.tree.get_nodes::<js::DependencyItem>() {
                let Some(symbol_id) = module.tree.symbol(item_id) else {
                    continue;
                };

                if !matches!(symbol_id, js::ScriptSymbolId::ModuleDefault(_)) {
                    continue;
                }

                if seen_symbols.insert(symbol_id) {
                    symbols.push(symbol_id);
                }
            }
        }

        symbols
    }

    /// Minify identifiers across one linked output.
    pub(in super::super) fn minify_output_identifiers(
        &self,
        modules: &mut [OutputModule],
    ) -> LinkResult<()> {
        let source_contexts = self.load_minify_source_contexts(modules)?;
        let mut bindings = HashMap::<js::ScriptSymbolId, BindingEntry>::new();
        let mut references = HashMap::<js::ScriptSymbolId, u32>::new();
        let mut reserved_names = self.collect_reserved_identifier_names(modules);

        // collect symbol facts from every linked module
        for (_, module) in modules.iter() {
            self.collect_module_identifier_symbols(
                module,
                &source_contexts,
                &mut bindings,
                &mut references,
                &mut reserved_names,
            )?;
        }

        // reserve names for referenced symbols that are not bound in this output
        for symbol_id in references.keys() {
            if bindings.contains_key(symbol_id) {
                continue;
            }

            if let Some(name) = self.source_symbol_name(*symbol_id, &source_contexts)? {
                reserved_names.insert(name);
            }
        }

        // strip declaration expression names before binding counts blur zero-use facts
        for (_, module) in modules.iter_mut() {
            self.strip_unused_declaration_expression_names(module, &source_contexts, &references)?;
        }

        // fold reference counts into the binding table
        for (symbol_id, count) in references {
            if let Some(binding) = bindings.get_mut(&symbol_id) {
                binding.use_count += count;
            }
        }

        let rename_by_symbol =
            self.assign_minified_identifier_names(&bindings, &source_contexts, &reserved_names)?;

        // apply the assigned names back into the linked modules
        for (_, module) in modules.iter_mut() {
            self.apply_minified_identifier_names(module, &source_contexts, &rename_by_symbol)?;

            let mut rewriter = Rewriter::new(Some(self.context), self.target, module);
            rewriter.use_object_shorthand_fields();
        }

        Ok(())
    }

    /// Collect globally reserved identifier names for one linked output.
    fn collect_reserved_identifier_names(&self, modules: &[OutputModule]) -> HashSet<String> {
        let mut reserved_names = HashSet::new();

        // keywords and direct eval names
        for keyword in [
            "await",
            "arguments",
            "eval",
            "false",
            "null",
            "this",
            "true",
        ] {
            reserved_names.insert(keyword.to_string());
        }

        // unbound path references
        for (_, module) in modules {
            for expression_id in module.tree.get_nodes::<js::Expression>() {
                let expression = module.tree.get(expression_id);
                let js::Expression::Path { path, .. } = expression else {
                    continue;
                };

                if path.segments.len() != 1 || module.tree.symbol(expression_id).is_some() {
                    continue;
                }

                let name = module.strings.get(path.segments[0]).to_string();
                reserved_names.insert(name);
            }
        }

        reserved_names
    }

    /// Collect all already-used identifier names across one linked output.
    fn collect_output_identifier_names(&self, modules: &[OutputModule]) -> HashSet<String> {
        let mut used_names = self.collect_reserved_identifier_names(modules);

        // declarations
        for (_, module) in modules {
            for declaration_id in module.tree.get_nodes::<js::Declaration>() {
                if module.tree.symbol(declaration_id).is_some_and(|symbol_id| {
                    matches!(symbol_id, js::ScriptSymbolId::ModuleDefault(_))
                }) {
                    continue;
                }

                let declaration = module.tree.get(declaration_id);
                let Some(js::Name::Identifier(name)) = declaration.descriptor().name else {
                    continue;
                };

                used_names.insert(module.strings.get(name).to_string());
            }

            // patterns
            for pattern_id in module.tree.get_nodes::<js::Pattern>() {
                if module.tree.symbol(pattern_id).is_some_and(|symbol_id| {
                    matches!(symbol_id, js::ScriptSymbolId::ModuleDefault(_))
                }) {
                    continue;
                }

                let pattern = module.tree.get(pattern_id);
                let js::Pattern::Binding { name, .. } = pattern else {
                    continue;
                };

                used_names.insert(module.strings.get(*name).to_string());
            }

            // shorthand pattern field bindings
            for pattern_field_id in module.tree.get_nodes::<js::PatternField>() {
                if module
                    .tree
                    .symbol(pattern_field_id)
                    .is_some_and(|symbol_id| {
                        matches!(symbol_id, js::ScriptSymbolId::ModuleDefault(_))
                    })
                {
                    continue;
                }

                let Some(name) =
                    Self::shorthand_pattern_field_binding_name(module, pattern_field_id)
                else {
                    continue;
                };

                used_names.insert(name);
            }

            // named parameters
            for parameter_id in module.tree.get_nodes::<js::Parameter>() {
                if module.tree.symbol(parameter_id).is_some_and(|symbol_id| {
                    matches!(symbol_id, js::ScriptSymbolId::ModuleDefault(_))
                }) {
                    continue;
                }

                let parameter = module.tree.get(parameter_id);

                match parameter {
                    js::Parameter::Named { name, .. }
                    | js::Parameter::VariadicNamed { name, .. } => {
                        used_names.insert(module.strings.get(*name).to_string());
                    }
                    js::Parameter::Pattern { .. } | js::Parameter::VariadicPattern { .. } => {}
                }
            }

            // import and export aliases
            for statement_id in module.tree.get_nodes::<js::Statement>() {
                let statement = module.tree.get(statement_id);

                match statement {
                    js::Statement::Import { items, .. } => {
                        for item_id in items.as_deref().unwrap_or(&[]) {
                            if module.tree.symbol(*item_id).is_some_and(|symbol_id| {
                                matches!(symbol_id, js::ScriptSymbolId::ModuleDefault(_))
                            }) {
                                continue;
                            }

                            let item = module.tree.get(*item_id);

                            if let Some(alias) = item.alias {
                                used_names.insert(module.strings.get(alias).to_string());
                            }

                            if let Some(js::Name::Identifier(name)) = item.name {
                                used_names.insert(module.strings.get(name).to_string());
                            }
                        }
                    }
                    js::Statement::Export { items, .. } => {
                        for item_id in items {
                            if module.tree.symbol(*item_id).is_some_and(|symbol_id| {
                                matches!(symbol_id, js::ScriptSymbolId::ModuleDefault(_))
                            }) {
                                continue;
                            }

                            let item = module.tree.get(*item_id);

                            if let Some(alias) = item.alias {
                                used_names.insert(module.strings.get(alias).to_string());
                            }

                            if let Some(js::Name::Identifier(name)) = item.name {
                                used_names.insert(module.strings.get(name).to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        used_names
    }

    /// Collect binding and reference identities from one linked module.
    fn collect_module_identifier_symbols(
        &self,
        module: &js::ScriptModule,
        source_contexts: &HashMap<ModuleId, MinifySourceContext>,
        bindings: &mut HashMap<js::ScriptSymbolId, BindingEntry>,
        references: &mut HashMap<js::ScriptSymbolId, u32>,
        reserved_names: &mut HashSet<String>,
    ) -> LinkResult<()> {
        // declaration bindings
        for declaration_id in module.tree.get_nodes::<js::Declaration>() {
            let Some(symbol_id) =
                self.declaration_name_symbol(module, declaration_id, source_contexts)?
            else {
                continue;
            };

            let declaration = module.tree.get(declaration_id);
            let Some(js::Name::Identifier(name)) = declaration.descriptor().name else {
                continue;
            };

            let name = module.strings.get(name).to_string();
            let is_preserved = declaration.descriptor().export.is_some()
                || self.declaration_keeps_name(declaration);
            self.record_binding_symbol(symbol_id, name, is_preserved, source_contexts, bindings)?;
        }

        // pattern bindings
        for pattern_id in module.tree.get_nodes::<js::Pattern>() {
            let Some(symbol_id) = module.tree.symbol(pattern_id) else {
                continue;
            };

            let pattern = module.tree.get(pattern_id);
            let js::Pattern::Binding { name, .. } = pattern else {
                continue;
            };

            let name = module.strings.get(*name).to_string();
            self.record_binding_symbol(symbol_id, name, false, source_contexts, bindings)?;
        }

        // shorthand pattern field bindings
        for pattern_field_id in module.tree.get_nodes::<js::PatternField>() {
            let Some(symbol_id) = module.tree.symbol(pattern_field_id) else {
                continue;
            };

            let Some(name) = Self::shorthand_pattern_field_binding_name(module, pattern_field_id)
            else {
                continue;
            };

            self.record_binding_symbol(symbol_id, name, false, source_contexts, bindings)?;
        }

        // named parameters
        for parameter_id in module.tree.get_nodes::<js::Parameter>() {
            let Some(symbol_id) = module.tree.symbol(parameter_id) else {
                continue;
            };

            let parameter = module.tree.get(parameter_id);
            let name = match parameter {
                js::Parameter::Named { name, .. } | js::Parameter::VariadicNamed { name, .. } => {
                    Some(module.strings.get(*name).to_string())
                }
                js::Parameter::Pattern { .. } | js::Parameter::VariadicPattern { .. } => None,
            };
            let Some(name) = name else {
                continue;
            };

            self.record_binding_symbol(symbol_id, name, false, source_contexts, bindings)?;
        }

        // import and export item bindings
        for statement_id in module.tree.get_nodes::<js::Statement>() {
            let statement = module.tree.get(statement_id);

            match statement {
                // import bindings
                js::Statement::Import { items, .. } => {
                    for item_id in items.as_deref().unwrap_or(&[]) {
                        let Some(symbol_id) = module.tree.symbol(*item_id) else {
                            continue;
                        };
                        let item = module.tree.get(*item_id);
                        let Some(name) = Self::import_binding_name(module, item) else {
                            continue;
                        };
                        self.record_binding_symbol(
                            symbol_id,
                            name,
                            false,
                            source_contexts,
                            bindings,
                        )?;
                    }
                }

                // keep export surface names stable
                js::Statement::Export { items, .. } => {
                    for item_id in items {
                        let Some(symbol_id) = module.tree.symbol(*item_id) else {
                            continue;
                        };
                        let item = module.tree.get(*item_id);
                        let Some(name) = Self::export_binding_name(module, item) else {
                            continue;
                        };
                        self.record_binding_symbol(
                            symbol_id,
                            name,
                            true,
                            source_contexts,
                            bindings,
                        )?;
                    }
                }

                // keep direct exported declarator bindings stable
                js::Statement::Let {
                    descriptor,
                    declarators,
                    ..
                }
                | js::Statement::Var {
                    descriptor,
                    declarators,
                }
                | js::Statement::Using {
                    descriptor,
                    declarators,
                    ..
                } => {
                    if descriptor.export.is_none() {
                        continue;
                    }

                    for declarator_id in declarators {
                        let declarator = module.tree.get(*declarator_id);
                        self.collect_exported_pattern_bindings(
                            module,
                            declarator.pattern,
                            source_contexts,
                            bindings,
                        )?;
                    }
                }

                _ => {}
            }
        }

        // path references
        for expression_id in module.tree.get_nodes::<js::Expression>() {
            let expression = module.tree.get(expression_id);
            let js::Expression::Path { path, .. } = expression else {
                continue;
            };

            if let Some(symbol_id) = module.tree.symbol(expression_id) {
                *references.entry(symbol_id).or_insert(0) += 1;
                continue;
            }

            if path.segments.len() == 1 {
                let name = module.strings.get(path.segments[0]).to_string();
                reserved_names.insert(name);
            }
        }

        Ok(())
    }

    /// Record one binding symbol for later renaming.
    fn record_binding_symbol(
        &self,
        symbol_id: js::ScriptSymbolId,
        original_name: String,
        is_preserved: bool,
        source_contexts: &HashMap<ModuleId, MinifySourceContext>,
        bindings: &mut HashMap<js::ScriptSymbolId, BindingEntry>,
    ) -> LinkResult<()> {
        let scope_id = self.symbol_scope_id(symbol_id, source_contexts)?;
        let entry = bindings.entry(symbol_id).or_insert_with(|| BindingEntry {
            original_name: original_name.clone(),
            scope_id,
            is_preserved,
            use_count: 1,
        });

        entry.original_name = original_name;
        entry.scope_id = scope_id;
        entry.is_preserved |= is_preserved;
        entry.use_count += 1;

        Ok(())
    }

    /// Return the local binding name for one import item.
    fn import_binding_name(module: &js::ScriptModule, item: &js::DependencyItem) -> Option<String> {
        match item.mode {
            js::DependencyMode::Default | js::DependencyMode::Namespace => item
                .alias
                .map(|alias| module.strings.get(alias).to_string()),
            js::DependencyMode::Item => {
                if let Some(alias) = item.alias {
                    return Some(module.strings.get(alias).to_string());
                }

                match item.name {
                    Some(js::Name::Identifier(name)) | Some(js::Name::String(name)) => {
                        Some(module.strings.get(name).to_string())
                    }
                    None => None,
                }
            }
        }
    }

    /// Return the local binding name for one export item.
    fn export_binding_name(module: &js::ScriptModule, item: &js::DependencyItem) -> Option<String> {
        match item.name {
            Some(js::Name::Identifier(name)) => Some(module.strings.get(name).to_string()),
            Some(js::Name::String(_)) | None => None,
        }
    }

    /// Return the bound name for one shorthand object pattern field.
    fn shorthand_pattern_field_binding_name(
        module: &js::ScriptModule,
        pattern_field_id: js::LocalNodeId<js::PatternField>,
    ) -> Option<String> {
        let pattern_field = module.tree.get(pattern_field_id);
        let js::PatternField::Named {
            name,
            is_shorthand: true,
            pattern: None,
            ..
        } = pattern_field
        else {
            return None;
        };

        Some(module.strings.get(*name).to_string())
    }

    /// Expand one shorthand object pattern field into an explicit binding pattern.
    fn expand_shorthand_pattern_field_binding(
        module: &mut js::ScriptModule,
        pattern_field_id: js::LocalNodeId<js::PatternField>,
        binding_name: &str,
    ) {
        let pattern_field = module.tree.get(pattern_field_id).clone();
        let js::PatternField::Named {
            mutability,
            is_shorthand: true,
            pattern: None,
            ..
        } = pattern_field
        else {
            return;
        };

        // explicit binding pattern
        let binding_name = module.strings.intern(binding_name);
        let binding_pattern = js::Pattern::Binding {
            mutability,
            name: binding_name,
        };
        let binding_pattern_id = module.tree.insert_from(binding_pattern, pattern_field_id);

        // rewrite the field into non-shorthand form
        let pattern_field = module.tree.get_mut(pattern_field_id);
        let js::PatternField::Named {
            is_shorthand,
            pattern,
            ..
        } = pattern_field
        else {
            unreachable!("expected shorthand named pattern field");
        };
        *is_shorthand = false;
        *pattern = Some(binding_pattern_id);
    }

    /// Collect one exported pattern tree as preserved bindings.
    fn collect_exported_pattern_bindings(
        &self,
        module: &js::ScriptModule,
        pattern_id: js::LocalNodeId<js::Pattern>,
        source_contexts: &HashMap<ModuleId, MinifySourceContext>,
        bindings: &mut HashMap<js::ScriptSymbolId, BindingEntry>,
    ) -> LinkResult<()> {
        let pattern = module.tree.get(pattern_id);

        match pattern {
            js::Pattern::Binding { name, .. } => {
                let Some(symbol_id) = module.tree.symbol(pattern_id) else {
                    return Ok(());
                };
                let name = module.strings.get(*name).to_string();
                self.record_binding_symbol(symbol_id, name, true, source_contexts, bindings)?;
            }
            js::Pattern::Assign { pattern, .. } => {
                self.collect_exported_pattern_bindings(
                    module,
                    *pattern,
                    source_contexts,
                    bindings,
                )?;
            }
            js::Pattern::Array { fields } => {
                for field_id in fields {
                    let field = module.tree.get(*field_id);

                    match field {
                        js::PatternField::Named {
                            pattern: Some(pattern),
                            ..
                        }
                        | js::PatternField::Computed { pattern, .. }
                        | js::PatternField::Positional { pattern, .. } => {
                            self.collect_exported_pattern_bindings(
                                module,
                                *pattern,
                                source_contexts,
                                bindings,
                            )?;
                        }
                        js::PatternField::Spread { .. }
                        | js::PatternField::Elision
                        | js::PatternField::Named { pattern: None, .. } => {}
                    }
                }
            }
            js::Pattern::Object { fields } => {
                for field_id in fields {
                    let field = module.tree.get(*field_id);

                    match field {
                        js::PatternField::Named {
                            pattern: Some(pattern),
                            ..
                        }
                        | js::PatternField::Computed { pattern, .. }
                        | js::PatternField::Positional { pattern, .. } => {
                            self.collect_exported_pattern_bindings(
                                module,
                                *pattern,
                                source_contexts,
                                bindings,
                            )?;
                        }
                        js::PatternField::Named {
                            pattern: None,
                            is_shorthand: true,
                            ..
                        } => {
                            let Some(symbol_id) = module.tree.symbol(*field_id) else {
                                continue;
                            };
                            let Some(name) =
                                Self::shorthand_pattern_field_binding_name(module, *field_id)
                            else {
                                continue;
                            };
                            self.record_binding_symbol(
                                symbol_id,
                                name,
                                true,
                                source_contexts,
                                bindings,
                            )?;
                        }
                        js::PatternField::Spread {
                            pattern: Some(pattern),
                            ..
                        } => {
                            self.collect_exported_pattern_bindings(
                                module,
                                *pattern,
                                source_contexts,
                                bindings,
                            )?;
                        }
                        js::PatternField::Named { pattern: None, .. }
                        | js::PatternField::Spread { pattern: None, .. }
                        | js::PatternField::Elision => {}
                    }
                }
            }
            js::Pattern::Hole => {}
        }

        Ok(())
    }

    /// Assign one final minified name to every renameable binding.
    fn assign_minified_identifier_names(
        &self,
        bindings: &HashMap<js::ScriptSymbolId, BindingEntry>,
        source_contexts: &HashMap<ModuleId, MinifySourceContext>,
        reserved_names: &HashSet<String>,
    ) -> LinkResult<HashMap<js::ScriptSymbolId, String>> {
        let mut symbols_by_scope = BTreeMap::<OutputScopeId, Vec<js::ScriptSymbolId>>::new();
        let mut children_by_scope = BTreeMap::<OutputScopeId, Vec<OutputScopeId>>::new();

        // index bindings by scope and build the scope tree
        for (symbol_id, entry) in bindings {
            symbols_by_scope
                .entry(entry.scope_id)
                .or_default()
                .push(*symbol_id);

            let mut scope_id = entry.scope_id;
            while let Some(parent_scope_id) = self.parent_scope_id(scope_id, source_contexts)? {
                let children = children_by_scope.entry(parent_scope_id).or_default();
                if !children.contains(&scope_id) {
                    children.push(scope_id);
                }

                scope_id = parent_scope_id;
            }
        }

        let mut rename_by_symbol = HashMap::new();
        let mut allocator = IdentifierNameAllocator::default();
        self.assign_minified_names_for_scope(
            OutputScopeId::TopLevel,
            bindings,
            &symbols_by_scope,
            &children_by_scope,
            reserved_names,
            &mut HashSet::new(),
            &mut allocator,
            &mut rename_by_symbol,
        )?;

        Ok(rename_by_symbol)
    }

    /// Assign minified names within one output scope and recurse into children.
    fn assign_minified_names_for_scope(
        &self,
        scope_id: OutputScopeId,
        bindings: &HashMap<js::ScriptSymbolId, BindingEntry>,
        symbols_by_scope: &BTreeMap<OutputScopeId, Vec<js::ScriptSymbolId>>,
        children_by_scope: &BTreeMap<OutputScopeId, Vec<OutputScopeId>>,
        reserved_names: &HashSet<String>,
        inherited_names: &mut HashSet<String>,
        allocator: &mut IdentifierNameAllocator,
        rename_by_symbol: &mut HashMap<js::ScriptSymbolId, String>,
    ) -> LinkResult<()> {
        let mut used_names = inherited_names.clone();
        used_names.extend(reserved_names.iter().cloned());

        let mut symbols = Vec::new();

        // collect the scope bindings up front so missing entries fail loudly
        for symbol_id in symbols_by_scope.get(&scope_id).cloned().unwrap_or_default() {
            let entry = bindings
                .get(&symbol_id)
                .cloned()
                .ok_or_else(|| LinkError::Internal {
                    package: self.package_id,
                    message: format!("missing minify binding entry for symbol {:?}", symbol_id),
                })?;

            symbols.push((symbol_id, entry));
        }

        symbols.sort_by(|(_, left_entry), (_, right_entry)| {
            if left_entry.is_preserved != right_entry.is_preserved {
                return right_entry.is_preserved.cmp(&left_entry.is_preserved);
            }

            if left_entry.use_count != right_entry.use_count {
                return right_entry.use_count.cmp(&left_entry.use_count);
            }

            left_entry.original_name.cmp(&right_entry.original_name)
        });

        // preserve boundary names before assigning new ones
        for (symbol_id, entry) in symbols.iter() {
            if !entry.is_preserved {
                continue;
            }

            used_names.insert(entry.original_name.clone());
            rename_by_symbol.insert(*symbol_id, entry.original_name.clone());
        }

        // rename the remaining symbols in frequency order
        for (symbol_id, entry) in symbols {
            if entry.is_preserved {
                continue;
            }

            let mut candidate = allocator.next_name();

            while used_names.contains(&candidate) || Self::is_reserved_identifier_name(&candidate) {
                candidate = allocator.next_name();
            }

            used_names.insert(candidate.clone());
            rename_by_symbol.insert(symbol_id, candidate);
        }

        // recurse with ancestor names reserved to avoid capture and shadowing bugs
        if let Some(children) = children_by_scope.get(&scope_id) {
            for child_scope_id in children {
                self.assign_minified_names_for_scope(
                    *child_scope_id,
                    bindings,
                    symbols_by_scope,
                    children_by_scope,
                    reserved_names,
                    &mut used_names.clone(),
                    allocator,
                    rename_by_symbol,
                )?;
            }
        }

        Ok(())
    }

    /// Apply one assigned rename table to one linked module.
    fn apply_minified_identifier_names(
        &self,
        module: &mut js::ScriptModule,
        source_contexts: &HashMap<ModuleId, MinifySourceContext>,
        rename_by_symbol: &HashMap<js::ScriptSymbolId, String>,
    ) -> LinkResult<()> {
        // declaration names
        for declaration_id in module.tree.get_nodes::<js::Declaration>() {
            let Some(symbol_id) =
                self.declaration_name_symbol(module, declaration_id, source_contexts)?
            else {
                continue;
            };
            let Some(name) = rename_by_symbol.get(&symbol_id) else {
                continue;
            };

            let declaration = module.tree.get_mut(declaration_id);
            if let Some(js::Name::Identifier(identifier)) =
                declaration.descriptor_mut().name.as_mut()
            {
                *identifier = module.strings.intern(name);
            }
        }

        // pattern names
        for pattern_id in module.tree.get_nodes::<js::Pattern>() {
            let Some(symbol_id) = module.tree.symbol(pattern_id) else {
                continue;
            };
            let Some(name) = rename_by_symbol.get(&symbol_id) else {
                continue;
            };

            let pattern = module.tree.get_mut(pattern_id);
            if let js::Pattern::Binding { name: binding, .. } = pattern {
                *binding = module.strings.intern(name);
            }
        }

        // shorthand pattern field bindings
        for pattern_field_id in module.tree.get_nodes::<js::PatternField>() {
            let Some(symbol_id) = module.tree.symbol(pattern_field_id) else {
                continue;
            };
            let Some(name) = rename_by_symbol.get(&symbol_id) else {
                continue;
            };

            Self::expand_shorthand_pattern_field_binding(module, pattern_field_id, name);
        }

        // named parameters
        for parameter_id in module.tree.get_nodes::<js::Parameter>() {
            let Some(symbol_id) = module.tree.symbol(parameter_id) else {
                continue;
            };
            let Some(name) = rename_by_symbol.get(&symbol_id) else {
                continue;
            };

            let parameter = module.tree.get_mut(parameter_id);
            match parameter {
                js::Parameter::Named {
                    name: parameter_name,
                    ..
                }
                | js::Parameter::VariadicNamed {
                    name: parameter_name,
                    ..
                } => {
                    *parameter_name = module.strings.intern(name);
                }
                js::Parameter::Pattern { .. } | js::Parameter::VariadicPattern { .. } => {}
            }
        }

        // path references
        for expression_id in module.tree.get_nodes::<js::Expression>() {
            let Some(symbol_id) = module.tree.symbol(expression_id) else {
                continue;
            };
            let Some(name) = rename_by_symbol.get(&symbol_id) else {
                continue;
            };

            let expression = module.tree.get_mut(expression_id);
            let js::Expression::Path { path, .. } = expression else {
                continue;
            };

            if let Some(first_segment) = path.segments.first_mut() {
                *first_segment = module.strings.intern(name);
            }
        }

        // import and export items
        for statement_id in module.tree.get_nodes::<js::Statement>() {
            let statement = module.tree.get(statement_id).clone();

            match statement {
                js::Statement::Import { items, .. } => {
                    for item_id in items.unwrap_or_default() {
                        let Some(symbol_id) = module.tree.symbol(item_id) else {
                            continue;
                        };
                        let Some(name) = rename_by_symbol.get(&symbol_id) else {
                            continue;
                        };

                        let item = module.tree.get_mut(item_id);
                        match item.mode {
                            js::DependencyMode::Default | js::DependencyMode::Namespace => {
                                item.alias = Some(module.strings.intern(name));
                            }
                            js::DependencyMode::Item => {
                                if item.alias.is_some() {
                                    item.alias = Some(module.strings.intern(name));
                                } else if let Some(js::Name::Identifier(_))
                                | Some(js::Name::String(_)) = item.name
                                {
                                    item.alias = Some(module.strings.intern(name));
                                }
                            }
                        }
                    }
                }
                js::Statement::Export { items, .. } => {
                    for item_id in items {
                        let Some(symbol_id) = module.tree.symbol(item_id) else {
                            continue;
                        };
                        let Some(name) = rename_by_symbol.get(&symbol_id) else {
                            continue;
                        };

                        let item = module.tree.get_mut(item_id);
                        if let Some(js::Name::Identifier(identifier)) = item.name.as_mut() {
                            *identifier = module.strings.intern(name);
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Strip unused names from function and class declaration expressions.
    fn strip_unused_declaration_expression_names(
        &self,
        module: &mut js::ScriptModule,
        source_contexts: &HashMap<ModuleId, MinifySourceContext>,
        references: &HashMap<js::ScriptSymbolId, u32>,
    ) -> LinkResult<()> {
        if self.target.minify.keep_names {
            return Ok(());
        }

        // collect declarations that are printed through expression positions
        let mut declaration_expression_ids = HashSet::new();
        for expression_id in module.tree.get_nodes::<js::Expression>() {
            let js::Expression::Declaration { declaration } = module.tree.get(expression_id) else {
                continue;
            };

            declaration_expression_ids.insert(declaration.id);
        }

        // strip only unused local names that exist just for expression self-reference
        for declaration_id in declaration_expression_ids {
            let declaration_id = js::LocalNodeId::<js::Declaration>::new(declaration_id);

            let Some(symbol_id) =
                self.declaration_name_symbol(module, declaration_id, source_contexts)?
            else {
                continue;
            };
            if references.get(&symbol_id).copied().unwrap_or(0) != 0 {
                continue;
            }
            let declaration = module.tree.get(declaration_id);
            let Some(js::Name::Identifier(_)) = declaration.descriptor().name else {
                continue;
            };

            let declaration = module.tree.get_mut(declaration_id);
            match declaration {
                js::Declaration::Class(js::ClassDeclaration { descriptor, .. })
                | js::Declaration::Function(js::FunctionDeclaration { descriptor, .. }) => {
                    descriptor.name = None;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Return whether one identifier name is reserved for minified output.
    pub(super) fn is_reserved_identifier_name(name: &str) -> bool {
        js::Keyword::from_str(name).is_ok() || matches!(name, "arguments" | "eval")
    }
}
