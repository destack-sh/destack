use std::collections::{HashMap, HashSet};

use destack_dir::{
    BindingCategory, Declaration, DependencyItem, DependencyKind, EnumKind, Expression,
    GlobalNodeIdAny, LocalScopeId, LocalSymbolId, MatchCase, MatchKind, Member, NodeTree, NodeType,
    Property, StaticKey, Symbol, SymbolBinding, SymbolKind, SymbolSpace, SymbolTable, SymbolType,
};
use destack_workspace::{DiagnosticPolicy, Module};

use crate::import::{SymbolDescriptor, can_merge_declarations};
use crate::{Compiler, ImportError};

impl Compiler {
    /// Check for conflicting bindings in module scopes.
    pub(super) fn validate_binding_conflicts(&self, module: &Module) {
        // resolve local redeclaration policy by language mode
        let no_redeclare_locals = self.no_redeclared_locals_enabled(module);
        let mut reported_conflicts = HashSet::new();

        // load symbol tables
        let tree = module.dir_base().tree.read();
        let symbols = module.dir_base().symbols.read();

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
                if let Some((node, other_node)) = self.enum_kind_mismatch_nodes(&tree, symbol) {
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

                    // check if symbols conflict based on space and merging rules
                    let import_kind_conflict =
                        self.is_type_value_import_conflict(&tree, symbol, other_symbol);
                    if !symbol.space.conflicts_with(other_symbol.space) && !import_kind_conflict {
                        continue;
                    }

                    let enum_kind_mismatch =
                        self.is_const_enum_mismatch(&tree, symbol, other_symbol);
                    let can_merge = can_merge_declarations(
                        module.language_type,
                        SymbolDescriptor::from(symbol),
                        SymbolDescriptor::from(other_symbol),
                    ) && !enum_kind_mismatch;
                    if can_merge {
                        continue;
                    }

                    // local conflicts are allowed unless configured otherwise
                    let is_local_pair =
                        symbol.kind == SymbolKind::Local && other_symbol.kind == SymbolKind::Local;
                    let left_binding_category = self.symbol_binding_category(symbol);
                    let right_binding_category = self.symbol_binding_category(other_symbol);
                    let is_parameter_pair = left_binding_category == BindingCategory::Parameter
                        || right_binding_category == BindingCategory::Parameter;
                    let is_strict_local_conflict =
                        self.is_strict_local_conflict(symbol, other_symbol);
                    let is_policy_controlled_conflict =
                        is_local_pair && !is_parameter_pair && !is_strict_local_conflict;
                    if is_policy_controlled_conflict && !no_redeclare_locals {
                        continue;
                    }

                    // error on conflicting bindings
                    let Some(other_primary_declaration) = other_symbol.primary_declaration else {
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
            // validate redeclaration conflicts across switch case scopes
            self.validate_switch_case_binding_conflicts(
                module,
                &tree,
                &symbols,
                &mut reported_conflicts,
            );

            // validate conflicts that require ancestor scope checks
            self.validate_ancestor_binding_conflicts(
                module,
                &tree,
                &symbols,
                &mut reported_conflicts,
            );
        }
    }

    /// Return true when local redeclarations should report conflicts.
    fn no_redeclared_locals_enabled(&self, module: &Module) -> bool {
        // JS/TS modes always enforce ecmascript redeclaration rules
        if !module.language_type.is_destack() {
            return true;
        }

        // destack modes read the configurable local redeclaration policy
        let policy = self
            .program
            .with_dsconfig_options(module, |opts| opts.compiler.no_redeclared_locals)
            .unwrap_or(DiagnosticPolicy::Allow);

        !policy.is_allow()
    }

    /// Check if the symbols are a strict local conflict.
    fn is_strict_local_conflict(&self, left: &Symbol, right: &Symbol) -> bool {
        matches!(left.ty, SymbolType::TypeAlias | SymbolType::Newtype)
            || matches!(right.ty, SymbolType::TypeAlias | SymbolType::Newtype)
    }

    /// Check if the symbols are a const enum mismatch.
    fn is_const_enum_mismatch(&self, tree: &NodeTree, left: &Symbol, right: &Symbol) -> bool {
        let Some(left_kind) = self.enum_kind_for_symbol(tree, left) else {
            return false;
        };
        let Some(right_kind) = self.enum_kind_for_symbol(tree, right) else {
            return false;
        };
        left_kind != right_kind
    }

    /// Get the enum kind for a symbol.
    fn enum_kind_for_symbol(&self, tree: &NodeTree, symbol: &Symbol) -> Option<EnumKind> {
        let primary = symbol.primary_declaration?;
        if primary.local_id.ty != NodeType::Declaration {
            return None;
        }
        let declaration_id = primary.local_id.into_typed::<Declaration>();
        match tree.get(declaration_id) {
            Declaration::Enum { kind, .. } => Some(*kind),
            _ => None,
        }
    }

    /// Get the enum kind for a declaration.
    fn enum_kind_for_declaration(
        &self,
        tree: &NodeTree,
        node: GlobalNodeIdAny,
    ) -> Option<EnumKind> {
        if node.local_id.ty != NodeType::Declaration {
            return None;
        }
        let declaration_id = node.local_id.into_typed::<Declaration>();
        match tree.get(declaration_id) {
            Declaration::Enum { kind, .. } => Some(*kind),
            _ => None,
        }
    }

    /// Check if the nodes are a enum kind mismatch.
    fn enum_kind_mismatch_nodes(
        &self,
        tree: &NodeTree,
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
    fn is_type_value_import_conflict(
        &self,
        tree: &NodeTree,
        left: &Symbol,
        right: &Symbol,
    ) -> bool {
        let Some(left_kind) = self.dependency_kind_for_symbol(tree, left) else {
            return false;
        };
        let Some(right_kind) = self.dependency_kind_for_symbol(tree, right) else {
            return false;
        };
        left_kind != right_kind
    }

    /// Get the dependency kind for a symbol.
    fn dependency_kind_for_symbol(
        &self,
        tree: &NodeTree,
        symbol: &Symbol,
    ) -> Option<DependencyKind> {
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
        tree: &NodeTree,
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
    fn validate_ancestor_binding_conflicts(
        &self,
        module: &Module,
        tree: &NodeTree,
        symbols: &SymbolTable,
        reported_conflicts: &mut HashSet<(u32, u32)>,
    ) {
        for scope in symbols.scopes() {
            for (key, symbol_id) in symbols.active_named_symbols(scope) {
                let normalized_key = self.normalize_conflict_key(key);
                let symbol = symbols.get_symbol(symbol_id);
                let binding_category = self.symbol_binding_category(symbol);
                if !self.is_conflict_binding_category(binding_category) {
                    continue;
                }
                let Some(primary_declaration) = symbol.primary_declaration else {
                    continue;
                };

                // walk ancestor scopes up to the nearest function boundary
                let mut current_parent = scope.parent;
                while let Some((ancestor_scope_id, _ancestor_mark)) = current_parent {
                    let ancestor_scope = symbols.get_scope_by_id(ancestor_scope_id);

                    for (ancestor_key, ancestor_symbol_id) in
                        symbols.active_named_symbols(ancestor_scope)
                    {
                        let ancestor_normalized_key = self.normalize_conflict_key(ancestor_key);
                        if ancestor_normalized_key != normalized_key {
                            continue;
                        }
                        let ancestor_symbol = symbols.get_symbol(ancestor_symbol_id);
                        let ancestor_binding_category =
                            self.symbol_binding_category(ancestor_symbol);
                        let should_conflict = matches!(
                            (binding_category, ancestor_binding_category),
                            (
                                BindingCategory::FunctionScoped,
                                BindingCategory::BlockScoped
                            ) | (BindingCategory::FunctionScoped, BindingCategory::Parameter)
                                | (BindingCategory::BlockScoped, BindingCategory::Parameter)
                        );
                        if !should_conflict {
                            continue;
                        }

                        let Some(ancestor_declaration) = ancestor_symbol.primary_declaration else {
                            continue;
                        };
                        self.report_conflicting_binding(
                            module,
                            symbol.scope.0,
                            key,
                            primary_declaration,
                            ancestor_declaration,
                            reported_conflicts,
                        );
                    }

                    // stop at function boundaries
                    if self.scope_is_function_boundary(tree, symbols, ancestor_scope_id) {
                        break;
                    }
                    current_parent = ancestor_scope.parent;
                }
            }
        }
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
        tree: &NodeTree,
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
                matches!(declaration, Declaration::Function { .. })
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

        let raw_name = self.program.strings.get(name_id).to_string();
        if !raw_name.contains('\\') {
            return key;
        }

        let Some(decoded_name) = self.decode_identifier_unicode_escapes(&raw_name) else {
            return key;
        };
        let decoded_name_id = self.program.strings.intern(&decoded_name);
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

    /// Report duplicate binding diagnostics from import for the given source.
    fn assert_import_conflicting_binding(source: &str) {
        // setup a ts module with the provided source
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ts", source);

        // run import and compile the queued task
        test.import_module(module_id);
        test.compile();

        // assert the duplicate binding diagnostic
        test.check_has_diagnostic("EI200");
    }

    /// Catch parameters conflict with lexical declarations in catch bodies.
    #[test]
    fn test_catch_parameter_conflicts_with_lexical_binding() {
        assert_import_conflicting_binding("try {} catch(a) { let a; }");
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
}
