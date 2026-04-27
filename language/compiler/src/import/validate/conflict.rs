use crate::common::dir::{SymbolDescriptor, can_merge_declarations};
use crate::{Compiler, CompilerContext, ImportError};
use destack_dir::{
    BindingCategory, Declaration, DependencyItem, DependencyKind, EnumKind, Expression,
    GlobalNodeIdAny, LocalScopeId, LocalSymbolId, MatchCase, MatchKind, Member, NodeType, Pattern,
    Property, StaticKey, Symbol, SymbolBinding, SymbolKind, SymbolSpace, SymbolTable, SymbolType,
    Tree,
};
use destack_workspace::{DiagnosticPolicy, Module};
use std::collections::{HashMap, HashSet};
#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Check for conflicting bindings in module scopes.
    pub(super) fn validate_binding_conflicts(
        &self,
        context: &CompilerContext<'_>,
        module: &Module,
        tree: &Tree,
        symbols: &SymbolTable,
        global_augmentation_scope: LocalScopeId,
    ) {
        // resolve local redeclaration policy by language mode
        let no_redeclare_locals = self.no_redeclared_locals_enabled(context, module);
        let mut reported_conflicts = HashSet::new();
        // load symbol tables
        {
            for scope in symbols.scopes() {
                // group symbols by name and category to avoid O(n^2) scans
                let mut buckets: HashMap<StaticKey, HashMap<SymbolCategory, LocalSymbolId>> =
                    HashMap::new();
                for (key, symbol_id) in symbols.active_named_symbols(scope) {
                    let normalized_key = self.normalize_conflict_key(key);
                    let symbol = symbols.get_symbol(symbol_id);
                    let Some(primary_declaration) = symbol.primary_declaration else {
                        continue;
                    };
                    // detect enum kind mismatches within a single symbol
                    if let Some((node, other_node)) = self.enum_kind_mismatch_nodes(tree, symbol) {
                        let error = ImportError::ConflictingBinding {
                            node: node.into_anchored(None),
                            other_node: other_node.into_anchored(None),
                            scope: symbol.scope.0.into_global(module.id),
                            name: Some(key),
                            is_local: false,
                        };
                        self.error(error);
                    }
                    // compare against previously seen symbols with the same name
                    let entry = buckets.entry(normalized_key).or_default();
                    let category = SymbolCategory::from(symbol);
                    for other_symbol_id in entry.values() {
                        if *other_symbol_id == symbol_id {
                            continue;
                        }
                        let other_symbol = symbols.get_symbol(*other_symbol_id);
                        // keep global augmentations isolated from module-local duplicates
                        if symbol.origin.is_global_augmentation()
                            != other_symbol.origin.is_global_augmentation()
                        {
                            continue;
                        }
                        // check if symbols conflict based on space and merging rules
                        let import_kind_conflict =
                            self.is_type_value_import_conflict(tree, symbol, other_symbol);
                        if !symbol.space.conflicts_with(other_symbol.space) && !import_kind_conflict
                        {
                            continue;
                        }
                        let enum_kind_mismatch =
                            self.is_const_enum_mismatch(tree, symbol, other_symbol);
                        let can_merge = self.can_symbols_merge_declarations(
                            module.language_type,
                            symbol,
                            other_symbol,
                        ) && !enum_kind_mismatch;
                        if can_merge {
                            continue;
                        }
                        // local conflicts are allowed unless configured otherwise
                        let is_local_pair = symbol.kind == SymbolKind::Local
                            && other_symbol.kind == SymbolKind::Local;
                        let left_binding_category = self.symbol_binding_category(symbol);
                        let right_binding_category = self.symbol_binding_category(other_symbol);
                        let is_parameter_pair = left_binding_category == BindingCategory::Parameter
                            || right_binding_category == BindingCategory::Parameter;
                        let is_strict_local_conflict =
                            self.is_strict_local_conflict(symbol, other_symbol);
                        let is_policy_controlled_conflict =
                            is_local_pair && !is_parameter_pair && !is_strict_local_conflict;
                        // JS/TS allow duplicate runtime var declarations
                        if self.allow_runtime_var_redeclaration(module, symbol, other_symbol) {
                            continue;
                        }
                        if is_policy_controlled_conflict && !no_redeclare_locals {
                            continue;
                        }
                        // error on conflicting bindings
                        let Some(other_primary_declaration) = other_symbol.primary_declaration
                        else {
                            continue;
                        };
                        let error = if symbol.export.is_some() && other_symbol.export.is_some() {
                            ImportError::ConflictingExport {
                                node: primary_declaration.into_anchored(None),
                                other_node: other_primary_declaration.into_anchored(None),
                                module: module.id,
                                name: Some(key),
                            }
                        } else {
                            ImportError::ConflictingBinding {
                                node: primary_declaration.into_anchored(None),
                                other_node: other_primary_declaration.into_anchored(None),
                                scope: symbol.scope.0.into_global(module.id),
                                name: Some(key),
                                is_local: is_local_pair,
                            }
                        };
                        self.error(error);
                        reported_conflicts.insert(Self::conflict_pair(
                            primary_declaration,
                            other_primary_declaration,
                        ));
                        break;
                    }
                    entry.entry(category).or_insert(symbol_id);
                }
            }
            // check for local redeclarations
            if no_redeclare_locals {
                let mut context = ConflictContext::new(
                    self,
                    module,
                    tree,
                    symbols,
                    global_augmentation_scope,
                    reported_conflicts,
                );
                context.validate_switch_case_binding_conflicts();
                context.validate_ancestor_binding_conflicts();
            }
        }
    }
    /// Return true when local redeclarations should report conflicts.
    fn no_redeclared_locals_enabled(&self, context: &CompilerContext<'_>, module: &Module) -> bool {
        // JS/TS modes always enforce ecmascript redeclaration rules
        if !module.language_type.is_destack() {
            return true;
        }
        // destack modes read the configurable local redeclaration policy
        let policy = context
            .compiler_options_for_module(module)
            .map(|options| options.no_redeclared_locals)
            .unwrap_or(DiagnosticPolicy::Allow);
        !policy.is_allow()
    }
    /// Check if the symbols are a strict local conflict.
    fn is_strict_local_conflict(&self, left: &Symbol, right: &Symbol) -> bool {
        matches!(left.ty, SymbolType::TypeAlias | SymbolType::Newtype)
            || matches!(right.ty, SymbolType::TypeAlias | SymbolType::Newtype)
    }
    /// Return true when duplicate runtime `var` declarations are allowed.
    fn allow_runtime_var_redeclaration(
        &self,
        module: &Module,
        left: &Symbol,
        right: &Symbol,
    ) -> bool {
        // only JS/TS allow duplicate runtime var declarations
        if !(module.language_type.is_javascript() || module.language_type.is_typescript()) {
            return false;
        }
        self.symbol_is_runtime_var_redeclaration_candidate(left)
            && self.symbol_is_runtime_var_redeclaration_candidate(right)
    }
    /// Return true when a symbol is a runtime var-style declaration candidate.
    fn symbol_is_runtime_var_redeclaration_candidate(&self, symbol: &Symbol) -> bool {
        symbol.kind == SymbolKind::Local
            && symbol.binding == SymbolBinding::Runtime
            && symbol.binding_category == BindingCategory::FunctionScoped
            && symbol.ty == SymbolType::Void
    }
    /// Check whether two symbols can merge using declaration order.
    fn can_symbols_merge_declarations(
        &self,
        language_type: destack_source::LanguageType,
        left: &Symbol,
        right: &Symbol,
    ) -> bool {
        let left_descriptor = SymbolDescriptor::from(left);
        let right_descriptor = SymbolDescriptor::from(right);
        let Some(left_declaration) = left.primary_declaration else {
            return can_merge_declarations(language_type, left_descriptor, right_descriptor);
        };
        let Some(right_declaration) = right.primary_declaration else {
            return can_merge_declarations(language_type, left_descriptor, right_descriptor);
        };
        if left_declaration.local_id.id <= right_declaration.local_id.id {
            can_merge_declarations(language_type, left_descriptor, right_descriptor)
        } else {
            can_merge_declarations(language_type, right_descriptor, left_descriptor)
        }
    }
    /// Check if the symbols are a const enum mismatch.
    fn is_const_enum_mismatch(&self, tree: &Tree, left: &Symbol, right: &Symbol) -> bool {
        let Some(left_kind) = self.enum_kind_for_symbol(tree, left) else {
            return false;
        };
        let Some(right_kind) = self.enum_kind_for_symbol(tree, right) else {
            return false;
        };
        left_kind != right_kind
    }
    /// Get the enum kind for a symbol.
    fn enum_kind_for_symbol(&self, tree: &Tree, symbol: &Symbol) -> Option<EnumKind> {
        let primary = symbol.primary_declaration?;
        if primary.local_id.ty != NodeType::Declaration {
            return None;
        }
        let declaration_id = primary.local_id.into_typed::<Declaration>();
        match tree.get(declaration_id) {
            Declaration::Enum(declaration) => Some(declaration.kind),
            _ => None,
        }
    }
    /// Get the enum kind for a declaration.
    fn enum_kind_for_declaration(&self, tree: &Tree, node: GlobalNodeIdAny) -> Option<EnumKind> {
        if node.local_id.ty != NodeType::Declaration {
            return None;
        }
        let declaration_id = node.local_id.into_typed::<Declaration>();
        match tree.get(declaration_id) {
            Declaration::Enum(declaration) => Some(declaration.kind),
            _ => None,
        }
    }
    /// Check if the nodes are a enum kind mismatch.
    fn enum_kind_mismatch_nodes(
        &self,
        tree: &Tree,
        symbol: &Symbol,
    ) -> Option<(GlobalNodeIdAny, GlobalNodeIdAny)> {
        if symbol.ty != SymbolType::Enum {
            return None;
        }
        let primary = symbol.primary_declaration?;
        let primary_kind = self.enum_kind_for_declaration(tree, primary)?;
        let secondaries = symbol.secondary_declarations.as_deref()?;
        for secondary in secondaries {
            let Some(kind) = self.enum_kind_for_declaration(tree, *secondary) else {
                continue;
            };
            if kind != primary_kind {
                return Some((primary, *secondary));
            }
        }
        None
    }
    /// Check if the symbols are a type value import conflict.
    fn is_type_value_import_conflict(&self, tree: &Tree, left: &Symbol, right: &Symbol) -> bool {
        let Some(left_kind) = self.dependency_kind_for_symbol(tree, left) else {
            return false;
        };
        let Some(right_kind) = self.dependency_kind_for_symbol(tree, right) else {
            return false;
        };
        left_kind != right_kind
    }
    /// Get the dependency kind for a symbol.
    fn dependency_kind_for_symbol(&self, tree: &Tree, symbol: &Symbol) -> Option<DependencyKind> {
        let primary = symbol.primary_declaration?;
        if primary.local_id.ty != NodeType::DependencyItem {
            return None;
        }
        let item_id = primary.local_id.into_typed::<DependencyItem>();
        match tree.get(item_id) {
            DependencyItem::UnresolvedRemote { kind, .. }
            | DependencyItem::UnresolvedLocal { kind, .. }
            | DependencyItem::Local { kind, .. } => Some(*kind),
            _ => None,
        }
    }
    /// Validate duplicate declarations across switch case scopes.
    fn validate_switch_case_binding_conflicts(
        &self,
        module: &Module,
        tree: &Tree,
        symbols: &SymbolTable,
        reported_conflicts: &mut HashSet<(u32, u32)>,
    ) {
        for (_expression_id, expression) in tree.iter_nodes_of_type::<Expression>() {
            let Expression::Match {
                kind: MatchKind::Switch,
                cases,
                ..
            } = expression
            else {
                continue;
            };
            // collect lexical and var declarations seen across switch cases
            let mut lexical_by_name: HashMap<StaticKey, GlobalNodeIdAny> = HashMap::new();
            let mut var_by_name: HashMap<StaticKey, GlobalNodeIdAny> = HashMap::new();
            for case_id in cases {
                let case_scope = match tree.get(*case_id) {
                    MatchCase::Expression { scope, .. } | MatchCase::Block { scope, .. } => *scope,
                };
                let scope = symbols.get_scope_by_id(case_scope);
                for (key, symbol_id) in symbols.active_named_symbols(scope) {
                    let normalized_key = self.normalize_conflict_key(key);
                    let symbol = symbols.get_symbol(symbol_id);
                    let binding_category = self.symbol_binding_category(symbol);
                    let Some(primary_declaration) = symbol.primary_declaration else {
                        continue;
                    };
                    // lexical declarations conflict with lexical declarations across cases
                    if binding_category == BindingCategory::BlockScoped
                        && let Some(other_declaration) = lexical_by_name.get(&normalized_key)
                    {
                        self.report_conflicting_binding(
                            module,
                            case_scope,
                            key,
                            primary_declaration,
                            *other_declaration,
                            reported_conflicts,
                        );
                    }
                    // lexical declarations conflict with var declarations across cases
                    if binding_category == BindingCategory::BlockScoped
                        && let Some(other_declaration) = var_by_name.get(&normalized_key)
                    {
                        self.report_conflicting_binding(
                            module,
                            case_scope,
                            key,
                            primary_declaration,
                            *other_declaration,
                            reported_conflicts,
                        );
                    }
                    // var declarations conflict with lexical declarations across cases
                    if binding_category == BindingCategory::FunctionScoped
                        && let Some(other_declaration) = lexical_by_name.get(&normalized_key)
                    {
                        self.report_conflicting_binding(
                            module,
                            case_scope,
                            key,
                            primary_declaration,
                            *other_declaration,
                            reported_conflicts,
                        );
                    }
                    // keep the first declaration for each key
                    if binding_category == BindingCategory::BlockScoped {
                        lexical_by_name
                            .entry(normalized_key)
                            .or_insert(primary_declaration);
                    }
                    if binding_category == BindingCategory::FunctionScoped {
                        var_by_name
                            .entry(normalized_key)
                            .or_insert(primary_declaration);
                    }
                }
            }
        }
    }
    /// Validate conflicts between declarations in ancestor scope chains.
    /// Return true when a scope is the global augmentation scope or nested under it.
    /// (This keeps `declare global` bindings isolated from module-local redeclaration checks.)
    fn scope_is_within_global_augmentation(
        &self,
        symbols: &SymbolTable,
        scope_id: LocalScopeId,
        global_augmentation_scope: LocalScopeId,
    ) -> bool {
        let mut current = Some(scope_id);
        while let Some(current_scope_id) = current {
            if current_scope_id == global_augmentation_scope {
                return true;
            }
            current = symbols
                .get_scope_by_id(current_scope_id)
                .parent
                .map(|(id, _)| id);
        }
        false
    }
    /// Return true when the category participates in duplicate-binding checks.
    fn is_conflict_binding_category(&self, category: BindingCategory) -> bool {
        matches!(
            category,
            BindingCategory::FunctionScoped
                | BindingCategory::BlockScoped
                | BindingCategory::Parameter
        )
    }
    /// Return true when two ancestor-chain categories form a redeclaration conflict.
    fn ancestor_binding_categories_conflict(
        &self,
        tree: &Tree,
        symbols: &SymbolTable,
        current_symbol: &Symbol,
        ancestor_symbol: &Symbol,
        current: BindingCategory,
        ancestor: BindingCategory,
        current_scope_id: LocalScopeId,
        ancestor_scope_id: LocalScopeId,
    ) -> bool {
        let current_is_parameter = current == BindingCategory::Parameter;
        let ancestor_is_parameter = ancestor == BindingCategory::Parameter;
        // skip non-parameter ancestor checks for non-local declaration pairs
        let is_local_pair =
            current_symbol.kind == SymbolKind::Local && ancestor_symbol.kind == SymbolKind::Local;
        if !is_local_pair && !current_is_parameter && !ancestor_is_parameter {
            return false;
        }
        // catch parameters conflict with hoisted bindings in their body scopes
        if current == BindingCategory::Parameter
            && ancestor == BindingCategory::FunctionScoped
            && self.symbol_is_catch_parameter(tree, current_symbol)
        {
            return true;
        }
        // runtime function and catch parameters conflict with body declarations
        if ancestor == BindingCategory::Parameter
            && (self.symbol_is_runtime_function_parameter(ancestor_symbol)
                || self.symbol_is_catch_parameter(tree, ancestor_symbol))
        {
            // function scoped bindings always conflict with parameters
            if current == BindingCategory::FunctionScoped {
                return true;
            }
            // block scoped bindings only conflict in the immediate body scope
            if current == BindingCategory::BlockScoped {
                let current_scope = symbols.get_scope_by_id(current_scope_id);
                let is_immediate_body_scope = current_scope
                    .parent
                    .is_some_and(|(parent_scope_id, _)| parent_scope_id == ancestor_scope_id);
                if is_immediate_body_scope {
                    return true;
                }
            }
        }
        matches!(
            (current, ancestor),
            (
                BindingCategory::FunctionScoped,
                BindingCategory::BlockScoped
            ) | (
                BindingCategory::BlockScoped,
                BindingCategory::FunctionScoped
            )
        )
    }
    /// Return true when a symbol is a runtime function parameter.
    fn symbol_is_runtime_function_parameter(&self, symbol: &Symbol) -> bool {
        symbol.binding_category == BindingCategory::Parameter
            && symbol.binding == SymbolBinding::Runtime
    }
    /// Return true when a symbol is the catch parameter of a try expression.
    fn symbol_is_catch_parameter(&self, tree: &Tree, symbol: &Symbol) -> bool {
        let Some(primary_declaration) = symbol.primary_declaration else {
            return false;
        };
        if primary_declaration.local_id.ty != NodeType::Pattern {
            return false;
        }
        let pattern_id = primary_declaration.local_id.into_typed::<Pattern>();
        let Some(parent_id) = tree.get_parent(pattern_id.id) else {
            return false;
        };
        if parent_id.ty != NodeType::Expression {
            return false;
        }
        let parent_expression = tree.get(parent_id.into_typed::<Expression>());
        let Expression::Try {
            catch_pattern: Some(catch_pattern),
            ..
        } = parent_expression
        else {
            return false;
        };
        *catch_pattern == pattern_id
    }
    /// Return the binding category used for redeclaration checks.
    fn symbol_binding_category(&self, symbol: &Symbol) -> BindingCategory {
        if symbol.binding_category != BindingCategory::Unclassified {
            return symbol.binding_category;
        }
        BindingCategory::NonBinding
    }
    /// Return true when a scope belongs to a function or method.
    fn scope_is_function_boundary(
        &self,
        tree: &Tree,
        symbols: &SymbolTable,
        scope_id: LocalScopeId,
    ) -> bool {
        let scope = symbols.get_scope_by_id(scope_id);
        let Some(owner_symbol_id) = scope.owner_id else {
            return false;
        };
        let owner_symbol = symbols.get_symbol(owner_symbol_id);
        let Some(primary_declaration) = owner_symbol.primary_declaration else {
            return false;
        };
        match primary_declaration.local_id.ty {
            NodeType::Declaration => {
                let declaration =
                    tree.get(primary_declaration.local_id.into_typed::<Declaration>());
                matches!(declaration, Declaration::Function(..))
            }
            NodeType::Member => {
                let member = tree.get(primary_declaration.local_id.into_typed::<Member>());
                matches!(member, Member::Method { .. })
            }
            NodeType::Property => {
                let property = tree.get(primary_declaration.local_id.into_typed::<Property>());
                matches!(property, Property::Method { .. })
            }
            _ => false,
        }
    }
    /// Return the normalized key used for conflict grouping.
    fn normalize_conflict_key(&self, key: StaticKey) -> StaticKey {
        let StaticKey::Name(name_id) = key else {
            return key;
        };
        let raw_name = self.repository.strings.get(name_id).to_string();
        if !raw_name.contains('\\') {
            return key;
        }
        let Some(decoded_name) = self.decode_identifier_unicode_escapes(&raw_name) else {
            return key;
        };
        let decoded_name_id = self.repository.strings.intern(&decoded_name);
        StaticKey::Name(decoded_name_id)
    }
    /// Decode unicode escapes in an identifier name.
    fn decode_identifier_unicode_escapes(&self, raw: &str) -> Option<String> {
        let mut decoded = String::with_capacity(raw.len());
        let mut index = 0usize;
        let bytes = raw.as_bytes();
        while index < bytes.len() {
            // regular character path
            if bytes[index] != b'\\' {
                let next_char = raw[index..].chars().next()?;
                decoded.push(next_char);
                index += next_char.len_utf8();
                continue;
            }
            // only \u escapes are valid in identifier names
            if bytes.get(index + 1).copied() != Some(b'u') {
                return None;
            }
            index += 2;
            // parse \u{...} escapes
            if bytes.get(index).copied() == Some(b'{') {
                index += 1;
                let digits_start = index;
                while bytes.get(index).is_some_and(|byte| *byte != b'}') {
                    index += 1;
                }
                if bytes.get(index).copied() != Some(b'}') || digits_start == index {
                    return None;
                }
                let digits = &raw[digits_start..index];
                if digits.len() > 6 {
                    return None;
                }
                let value = u32::from_str_radix(digits, 16).ok()?;
                let character = char::from_u32(value)?;
                decoded.push(character);
                index += 1;
                continue;
            }
            // parse \uXXXX escapes
            if index + 4 > bytes.len() {
                return None;
            }
            let digits = &raw[index..index + 4];
            let value = u32::from_str_radix(digits, 16).ok()?;
            let character = char::from_u32(value)?;
            decoded.push(character);
            index += 4;
        }
        Some(decoded)
    }
    /// Report a conflicting binding pair if it has not been reported.
    fn report_conflicting_binding(
        &self,
        module: &Module,
        scope_id: LocalScopeId,
        key: StaticKey,
        declaration: GlobalNodeIdAny,
        other_declaration: GlobalNodeIdAny,
        reported_conflicts: &mut HashSet<(u32, u32)>,
    ) {
        let pair = Self::conflict_pair(declaration, other_declaration);
        if !reported_conflicts.insert(pair) {
            return;
        }
        self.error(ImportError::ConflictingBinding {
            node: declaration.into_anchored(None),
            other_node: other_declaration.into_anchored(None),
            scope: scope_id.into_global(module.id),
            name: Some(key),
            is_local: true,
        });
    }
    /// Return a stable pair key for conflict deduplication.
    fn conflict_pair(left: GlobalNodeIdAny, right: GlobalNodeIdAny) -> (u32, u32) {
        let left_id = left.local_id.id;
        let right_id = right.local_id.id;
        if left_id <= right_id {
            (left_id, right_id)
        } else {
            (right_id, left_id)
        }
    }
}
/// Stateful caches and reporting for binding conflict validation.
struct ConflictContext<'a> {
    /// The compiler driving validation.
    compiler: &'a Compiler,
    /// The module being validated.
    module: &'a Module,
    /// The module syntax tree.
    tree: &'a Tree,
    /// The module symbol table.
    symbols: &'a SymbolTable,
    /// The root scope for `declare global` isolation.
    global_augmentation_scope: LocalScopeId,
    /// The conflicts already reported for this module.
    reported_conflicts: HashSet<(u32, u32)>,
    /// The normalized active symbols by scope.
    normalized_scope_symbols: HashMap<LocalScopeId, HashMap<StaticKey, Vec<LocalSymbolId>>>,
    /// The cached global augmentation ancestry facts.
    scope_global_augmentations: HashMap<LocalScopeId, bool>,
    /// The cached function-boundary facts.
    scope_function_boundaries: HashMap<LocalScopeId, bool>,
}
impl<'a> ConflictContext<'a> {
    /// Create a new binding conflict validation context.
    fn new(
        compiler: &'a Compiler,
        module: &'a Module,
        tree: &'a Tree,
        symbols: &'a SymbolTable,
        global_augmentation_scope: LocalScopeId,
        reported_conflicts: HashSet<(u32, u32)>,
    ) -> Self {
        Self {
            compiler,
            module,
            tree,
            symbols,
            global_augmentation_scope,
            reported_conflicts,
            normalized_scope_symbols: HashMap::new(),
            scope_global_augmentations: HashMap::new(),
            scope_function_boundaries: HashMap::new(),
        }
    }
    /// Validate duplicate declarations across switch case scopes.
    fn validate_switch_case_binding_conflicts(&mut self) {
        self.compiler.validate_switch_case_binding_conflicts(
            self.module,
            self.tree,
            self.symbols,
            &mut self.reported_conflicts,
        );
    }
    /// Validate conflicts between declarations in ancestor scope chains.
    fn validate_ancestor_binding_conflicts(&mut self) {
        for scope in self.symbols.scopes() {
            for (key, symbol_id) in self.symbols.active_named_symbols(scope) {
                let normalized_key = self.compiler.normalize_conflict_key(key);
                let symbol = self.symbols.get_symbol(symbol_id);
                let scope_id = symbol.scope.0;
                let binding_category = self.compiler.symbol_binding_category(symbol);
                // skip symbols that do not participate in redeclaration checks
                if !self.compiler.is_conflict_binding_category(binding_category) {
                    continue;
                }
                // only real declarations can participate in a reported conflict
                let Some(primary_declaration) = symbol.primary_declaration else {
                    continue;
                };
                // keep global augmentations isolated from module-local ancestor checks
                let scope_is_global_augmentation =
                    self.scope_is_within_global_augmentation(scope_id);
                // walk ancestors up to the nearest function boundary
                let mut current_parent = scope.parent;
                while let Some((ancestor_scope_id, _ancestor_mark)) = current_parent {
                    let ancestor_is_global_augmentation =
                        self.scope_is_within_global_augmentation(ancestor_scope_id);
                    if scope_is_global_augmentation != ancestor_is_global_augmentation {
                        break;
                    }
                    // inspect only same-name candidates in the ancestor scope
                    let ancestor_symbol_ids = self
                        .normalized_active_named_symbols_for_scope(ancestor_scope_id)
                        .get(&normalized_key)
                        .cloned();
                    if let Some(ancestor_symbol_ids) = ancestor_symbol_ids {
                        for ancestor_symbol_id in ancestor_symbol_ids {
                            let ancestor_symbol = self.symbols.get_symbol(ancestor_symbol_id);
                            let ancestor_binding_category =
                                self.compiler.symbol_binding_category(ancestor_symbol);
                            let should_conflict =
                                self.compiler.ancestor_binding_categories_conflict(
                                    self.tree,
                                    self.symbols,
                                    symbol,
                                    ancestor_symbol,
                                    binding_category,
                                    ancestor_binding_category,
                                    scope_id,
                                    ancestor_scope_id,
                                );
                            // skip same-name ancestors that are semantically compatible
                            if !should_conflict {
                                continue;
                            }
                            // only real declarations can be reported
                            let Some(ancestor_declaration) = ancestor_symbol.primary_declaration
                            else {
                                continue;
                            };
                            self.report_conflicting_binding(
                                scope_id,
                                key,
                                primary_declaration,
                                ancestor_declaration,
                            );
                        }
                    }
                    // stop once redeclaration checks no longer cross the function boundary
                    if self.scope_is_function_boundary(ancestor_scope_id) {
                        break;
                    }
                    current_parent = self.symbols.get_scope_by_id(ancestor_scope_id).parent;
                }
            }
        }
    }
    /// Return the normalized active named symbols for a scope.
    fn normalized_active_named_symbols_for_scope(
        &mut self,
        scope_id: LocalScopeId,
    ) -> &HashMap<StaticKey, Vec<LocalSymbolId>> {
        self.normalized_scope_symbols
            .entry(scope_id)
            .or_insert_with(|| {
                let scope = self.symbols.get_scope_by_id(scope_id);
                let mut normalized_symbols = HashMap::new();
                // group active symbols by normalized name
                for (key, symbol_id) in self.symbols.active_named_symbols(scope) {
                    let normalized_key = self.compiler.normalize_conflict_key(key);
                    normalized_symbols
                        .entry(normalized_key)
                        .or_insert_with(Vec::new)
                        .push(symbol_id);
                }
                normalized_symbols
            })
    }
    /// Return true when a scope belongs to the global augmentation chain.
    fn scope_is_within_global_augmentation(&mut self, scope_id: LocalScopeId) -> bool {
        if let Some(is_within_global_augmentation) = self.scope_global_augmentations.get(&scope_id)
        {
            return *is_within_global_augmentation;
        }
        let is_within_global_augmentation = self.compiler.scope_is_within_global_augmentation(
            self.symbols,
            scope_id,
            self.global_augmentation_scope,
        );
        self.scope_global_augmentations
            .insert(scope_id, is_within_global_augmentation);
        is_within_global_augmentation
    }
    /// Return true when a scope belongs to a function or method.
    fn scope_is_function_boundary(&mut self, scope_id: LocalScopeId) -> bool {
        if let Some(is_function_boundary) = self.scope_function_boundaries.get(&scope_id) {
            return *is_function_boundary;
        }
        let is_function_boundary =
            self.compiler
                .scope_is_function_boundary(self.tree, self.symbols, scope_id);
        self.scope_function_boundaries
            .insert(scope_id, is_function_boundary);
        is_function_boundary
    }
    /// Report a conflicting binding pair if it has not been reported.
    fn report_conflicting_binding(
        &mut self,
        scope_id: LocalScopeId,
        key: StaticKey,
        declaration: GlobalNodeIdAny,
        other_declaration: GlobalNodeIdAny,
    ) {
        self.compiler.report_conflicting_binding(
            self.module,
            scope_id,
            key,
            declaration,
            other_declaration,
            &mut self.reported_conflicts,
        );
    }
}
/// Grouping key for conflict validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SymbolCategory {
    /// Symbol space for conflict grouping.
    space: SymbolSpace,
    /// Symbol type for conflict grouping.
    ty: SymbolType,
    /// Symbol binding for conflict grouping.
    binding: SymbolBinding,
    /// Symbol kind for conflict grouping.
    kind: SymbolKind,
}
impl From<&Symbol> for SymbolCategory {
    /// Create a conflict category from a symbol.
    fn from(symbol: &Symbol) -> Self {
        Self {
            space: symbol.space,
            ty: symbol.ty,
            binding: symbol.binding,
            kind: symbol.kind,
        }
    }
}
#[cfg(test)]
mod tests {
    use crate::tests::TestProgram;
    /// Report duplicate binding diagnostics from import for the given source path.
    fn assert_import_conflicting_binding_in(path: &str, source: &str) {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(path, source);
        test.import_module(module_id);
        test.compile();
        test.check_has_diagnostic("EI200");
    }
    /// Report no duplicate binding diagnostics from import for the given source path.
    fn assert_import_no_conflicting_binding_in(path: &str, source: &str) {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(path, source);
        test.import_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EI200");
    }
    /// Report duplicate binding diagnostics from import for a TypeScript module.
    fn assert_import_conflicting_binding(source: &str) {
        assert_import_conflicting_binding_in("test.ts", source);
    }
    /// Report no duplicate binding diagnostics from import for a TypeScript module.
    fn assert_import_no_conflicting_binding(source: &str) {
        assert_import_no_conflicting_binding_in("test.ts", source);
    }
    /// Catch parameters conflict with lexical declarations in catch bodies.
    #[test]
    fn test_catch_parameter_conflicts_with_lexical_binding() {
        assert_import_conflicting_binding("try {} catch(a) { let a; }");
    }
    /// JavaScript function parameter destructuring conflicts with lexical declarations.
    #[test]
    fn test_javascript_parameter_destructuring_conflicts_with_body_let() {
        assert_import_conflicting_binding_in("test.js", "function a({b}){ let b; }");
    }
    /// JavaScript method parameter destructuring conflicts with lexical declarations.
    #[test]
    fn test_javascript_method_parameter_destructuring_conflicts_with_body_let() {
        assert_import_conflicting_binding_in("test.js", "!{ a({b}){ let b; } };");
    }
    /// JavaScript arrow parameter destructuring conflicts with lexical declarations.
    #[test]
    fn test_javascript_arrow_parameter_destructuring_conflicts_with_body_const() {
        assert_import_conflicting_binding_in("test.js", "({a}) => { const a = 1; }");
    }
    /// JavaScript nested object patterns do not bind property names.
    #[test]
    fn test_javascript_parameter_nested_object_pattern_does_not_bind_property_name() {
        assert_import_no_conflicting_binding_in(
            "test.js",
            "function a({it: {gen}, it}){ it; gen; }",
        );
    }
    /// JavaScript allows nested block lexical shadowing of function parameters.
    #[test]
    fn test_javascript_parameter_allows_nested_block_lexical_shadowing() {
        assert_import_no_conflicting_binding_in(
            "test.js",
            "function a(node){ if (true) { const node = 1; } }",
        );
    }
    /// Catch parameters allow nested block lexical shadowing.
    #[test]
    fn test_catch_parameter_allows_nested_block_lexical_shadowing() {
        assert_import_no_conflicting_binding("try {} catch(a) { if (true) { let a; } }");
    }
    /// Catch parameters conflict with var declarations in catch bodies.
    #[test]
    fn test_catch_parameter_conflicts_with_for_each_var_binding() {
        assert_import_conflicting_binding("try {} catch(a) { for(var a of 1); }");
    }
    /// For each lexical headers conflict with var declarations in the body.
    #[test]
    fn test_for_each_lexical_binding_conflicts_with_body_var() {
        assert_import_conflicting_binding("for(let a in 1) { var a; }");
    }
    /// Class method lexical declarations conflict with var declarations in the same body.
    #[test]
    fn test_class_method_lexical_binding_conflicts_with_body_var_javascript() {
        assert_import_conflicting_binding_in("test.js", "class a { static b(){ let c; var c; } }");
    }
    /// Allow parameter names to shadow outer function-scoped bindings.
    #[test]
    fn test_allow_parameter_shadowing_outer_var_in_declaration_file() {
        assert_import_no_conflicting_binding_in(
            "test.d.ts",
            "declare var module: unknown;\n\
             type Loader = (module: string) => void;",
        );
    }
    /// JavaScript allows duplicate runtime var declarations in one scope.
    #[test]
    fn test_javascript_duplicate_runtime_var_declarations_are_allowed() {
        assert_import_no_conflicting_binding_in("test.js", "function f() { var a; var a; }");
    }
    /// TypeScript allows duplicate runtime var declarations in one scope.
    #[test]
    fn test_typescript_duplicate_runtime_var_declarations_are_allowed() {
        assert_import_no_conflicting_binding("function f() { var a; var a; }");
    }
    /// Switch case function and lexical declarations conflict in strict modules.
    #[test]
    fn test_switch_case_function_conflicts_with_case_lexical() {
        assert_import_conflicting_binding("switch(1) { default: function a(){} case 2: let a; }");
    }
    /// Escaped and unescaped identifier names normalize to the same key.
    #[test]
    fn test_unicode_escaped_identifier_conflicts() {
        assert_import_conflicting_binding("let \\u0061, \\u{0061};");
    }
    /// Reject ambient class and value declarations that share one name.
    #[test]
    fn test_reject_ambient_class_value_duplicate() {
        assert_import_conflicting_binding_in(
            "test.d.ts",
            "declare abstract class Iterator<T> {}\ndeclare var Iterator: { new<T>(): Iterator<T> };",
        );
    }
    /// Reject duplicate ambient class declarations.
    #[test]
    fn test_reject_ambient_class_duplicate() {
        assert_import_conflicting_binding_in(
            "test.d.ts",
            "declare class Client {}\ndeclare class Client {}",
        );
    }
    /// Allow module-local class names to coexist with global augmentations.
    #[test]
    fn test_allow_module_local_class_with_global_var_augmentation() {
        assert_import_no_conflicting_binding_in(
            "test.d.ts",
            "export {};\ndeclare abstract class Iterator<T> {}\ndeclare global { var Iterator: { new<T>(): Iterator<T> }; }",
        );
    }
    /// Allow declared module exports to coexist with nested global augmentations.
    #[test]
    fn test_allow_module_binding_class_with_global_var_augmentation() {
        assert_import_no_conflicting_binding_in(
            "test.d.ts",
            r#"
declare module "url" {
    class URL {}
    global {
        interface URL {}
        var URL: { new(): URL };
    }
}
"#,
        );
    }
    /// Allow class and runtime namespace declarations when class appears first.
    #[test]
    fn test_allow_class_then_runtime_namespace_merge() {
        assert_import_no_conflicting_binding(
            "class Client {} namespace Client { export const value = 1; }",
        );
    }
    /// Reject runtime namespace and class declarations when namespace appears first.
    #[test]
    fn test_reject_runtime_namespace_then_class_merge() {
        assert_import_conflicting_binding(
            "namespace Client { export const value = 1; } class Client {}",
        );
    }
    /// Allow type only namespace and class declarations in either order.
    #[test]
    fn test_allow_type_only_namespace_then_class_merge() {
        assert_import_no_conflicting_binding(
            "namespace Client { export interface Options {} } class Client {}",
        );
    }
    /// Allow type only namespace and function declarations in either order.
    #[test]
    fn test_allow_type_only_namespace_then_function_merge() {
        assert_import_no_conflicting_binding(
            "namespace Factory { export interface Options {} } function Factory() {}",
        );
    }
    /// Reject runtime namespace and function declarations when namespace appears first.
    #[test]
    fn test_reject_runtime_namespace_then_function_merge() {
        assert_import_conflicting_binding(
            "namespace Factory { export const value = 1; } function Factory() {}",
        );
    }
    /// Reject runtime namespace declarations that collide with runtime values.
    #[test]
    fn test_reject_runtime_namespace_with_runtime_value() {
        assert_import_conflicting_binding(
            "namespace Runtime { export const value = 1; } var Runtime = 1;",
        );
    }
    /// Reject runtime namespace declarations that collide with ambient values.
    #[test]
    fn test_reject_runtime_namespace_with_ambient_value() {
        assert_import_conflicting_binding(
            "namespace Runtime { export const value = 1; } declare var Runtime: number;",
        );
    }
    /// Allow ambient namespace declarations to coexist with runtime values.
    #[test]
    fn test_allow_ambient_namespace_with_runtime_value() {
        assert_import_no_conflicting_binding("declare namespace Runtime {} var Runtime = 1;");
    }
    /// Allow type only namespace declarations to coexist with runtime values.
    #[test]
    fn test_allow_type_only_namespace_with_runtime_value() {
        assert_import_no_conflicting_binding(
            "interface Runtime {} namespace Runtime { export type Inner = string; } const Runtime = 1;",
        );
    }
    /// Allow type only namespace declarations to coexist with ambient values.
    #[test]
    fn test_allow_type_only_namespace_with_ambient_value() {
        assert_import_no_conflicting_binding(
            "interface Runtime {} namespace Runtime { export type Inner = string; } declare var Runtime: number;",
        );
    }
    /// Allow var declarations with named function expressions that reuse the same identifier.
    #[test]
    fn test_allow_var_and_named_function_expression_same_identifier_javascript() {
        assert_import_no_conflicting_binding_in(
            "test.js",
            "var runInContext = (function runInContext(context) { return context; });",
        );
    }
}
