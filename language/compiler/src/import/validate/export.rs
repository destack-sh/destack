use std::collections::{HashMap, HashSet};

use destack_ast::Keyword;
use destack_base::StringId;
use destack_dir::{
    Declaration, DependencyItem, DependencyKind, DependencyMode, DependencySource, Expression,
    LocalNodeId, LocalNodeIdAny, NodeTree, NodeType, Pattern, PatternField, StaticKey, SymbolTable,
};
use destack_workspace::Module;
use std::str::FromStr;

use crate::import::{SymbolDescriptor, can_merge_declarations};
use crate::{Compiler, ImportError};

/// The export category used for duplicate export checks.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ExportConflictKind {
    /// A declaration export with merge metadata.
    Declaration(SymbolDescriptor),
    /// Any non-function export.
    Other,
}

impl Compiler {
    /// Validate import and export declarations appear at the module root.
    pub(super) fn validate_dependency_top_level(&self, module: &Module) {
        // declaration files allow nested ambient import and export forms
        if module.language_type.is_declaration() {
            return;
        }

        // load the module tree once for dependency scanning
        let dir = module.dir_base();
        let tree = dir.tree.read();

        // collect top level expressions after unwrapping statement wrappers
        let mut top_level_expression_ids = HashSet::new();
        for root_id in &dir.roots {
            let expression_id = self.unwrap_statement_expression_for_import(&tree, *root_id);
            top_level_expression_ids.insert(expression_id.id);
        }

        // report nested static dependencies as import errors
        for (expression_id, expression) in tree.iter_nodes_of_type::<Expression>() {
            let is_top_level = top_level_expression_ids.contains(&expression_id.id);
            if is_top_level {
                continue;
            }

            // import declarations
            if let Expression::Import { source, .. } | Expression::UnresolvedImport { source, .. } =
                expression
            {
                if self.import_dependency_requires_top_level(*source) {
                    let node = expression_id.into_global_any(module.id).into_anchored(None);
                    self.error(ImportError::ImportNotTopLevel { node });
                }
                continue;
            }

            // export declarations
            if matches!(
                expression,
                Expression::Export { .. }
                    | Expression::ReExport { .. }
                    | Expression::UnresolvedReExport { .. }
                    | Expression::ExportNamespace { .. }
            ) {
                // namespace bodies are their own declaration roots
                if self.expression_is_within_namespace_declaration(&tree, expression_id) {
                    continue;
                }

                let node = expression_id.into_global_any(module.id).into_anchored(None);
                self.error(ImportError::ExportNotTopLevel { node });
            }
        }
    }

    /// Return true when an expression is nested under a namespace declaration.
    fn expression_is_within_namespace_declaration(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        // walk parent links until a declaration boundary is found
        let mut current_id = expression_id.id;
        while let Some(parent_id) = tree.get_parent(current_id) {
            // namespace declarations own nested export declarations in TypeScript
            if parent_id.ty == NodeType::Declaration {
                let declaration = tree.get(parent_id.into_typed::<Declaration>());
                return matches!(declaration, Declaration::Namespace { .. });
            }
            current_id = parent_id.id;
        }
        false
    }

    /// Validate local export item names use binding-compatible identifiers.
    pub(super) fn validate_export_local_item_names(&self, module: &Module) {
        // load the module tree once for dependency scanning
        let dir = module.dir_base();
        let tree = dir.tree.read();

        // only direct module exports participate in local export name validation
        for root_id in &dir.roots {
            let expression_id = self.unwrap_statement_expression_for_import(&tree, *root_id);
            let Expression::Export { items, .. } = tree.get(expression_id) else {
                continue;
            };

            for item_id in items {
                let item = tree.get(*item_id);
                let (mode, kind, name) = match item {
                    DependencyItem::UnresolvedLocal {
                        mode, kind, name, ..
                    }
                    | DependencyItem::Local {
                        mode, kind, name, ..
                    } => (*mode, *kind, *name),
                    _ => continue,
                };

                // item mode carries local names that must be binding-compatible
                if mode != DependencyMode::Item || kind != DependencyKind::Value {
                    continue;
                }

                let Some(destack_dir::Name::Identifier(name)) = name else {
                    continue;
                };
                if !self.export_local_name_is_disallowed_identifier(name) {
                    continue;
                }

                let node = (*item_id).into_global_any(module.id).into_anchored(None);
                self.error(ImportError::ReservedIdentifier { node, name });
            }
        }
    }

    /// Return true when a dependency source must be top level.
    fn import_dependency_requires_top_level(&self, source: DependencySource) -> bool {
        matches!(
            source,
            DependencySource::ImportStatement
                | DependencySource::ImportEquals
                | DependencySource::ExportStatement
                | DependencySource::ValueExpression
        )
    }

    /// Validate duplicate value exports in a module.
    pub(super) fn validate_export_conflicts(&self, module: &Module) {
        // load the module tree once for export scanning
        let dir = module.dir_base();
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();
        let default_name = self.program.strings.intern("default");
        let mut exported_names = HashMap::new();

        // scan top-level roots in source order
        for root_id in &dir.roots {
            let expression_id = self.unwrap_statement_expression_for_import(&tree, *root_id);
            let expression = tree.get(expression_id);

            // collect exported names by expression kind
            match expression {
                Expression::Declaration { declaration } => {
                    let declaration_id = *declaration;
                    let declaration = tree.get(declaration_id);
                    let export_name =
                        self.value_export_name_for_declaration(declaration, default_name);
                    if let Some(export_name) = export_name {
                        let conflict_kind =
                            self.export_conflict_kind_for_declaration(declaration, &symbols);
                        self.report_conflicting_export_name_maybe(
                            module,
                            export_name,
                            declaration_id.into_any(),
                            conflict_kind,
                            &mut exported_names,
                            default_name,
                        );
                    }
                }
                Expression::Let {
                    descriptor,
                    declarators,
                    ..
                } => {
                    // only exported declarations participate
                    if descriptor.export != Some(DependencyMode::Item) {
                        continue;
                    }

                    for declarator_id in declarators {
                        let declarator = tree.get(*declarator_id);
                        let mut names = Vec::new();
                        self.collect_binding_names_from_pattern(
                            &tree,
                            declarator.pattern,
                            &mut names,
                        );

                        for name in names {
                            self.report_conflicting_export_name_maybe(
                                module,
                                name,
                                (*declarator_id).into_any(),
                                ExportConflictKind::Other,
                                &mut exported_names,
                                default_name,
                            );
                        }
                    }
                }
                Expression::Using {
                    descriptor,
                    declarators,
                    ..
                } => {
                    // only exported declarations participate
                    if descriptor.export != Some(DependencyMode::Item) {
                        continue;
                    }

                    for declarator_id in declarators {
                        let declarator = tree.get(*declarator_id);
                        let mut names = Vec::new();
                        self.collect_binding_names_from_pattern(
                            &tree,
                            declarator.pattern,
                            &mut names,
                        );

                        for name in names {
                            self.report_conflicting_export_name_maybe(
                                module,
                                name,
                                (*declarator_id).into_any(),
                                ExportConflictKind::Other,
                                &mut exported_names,
                                default_name,
                            );
                        }
                    }
                }
                Expression::Export { items, .. }
                | Expression::ReExport { items, .. }
                | Expression::UnresolvedReExport { items, .. } => {
                    for item_id in items {
                        let export_name = self.value_export_name_for_dependency_item(
                            &tree,
                            *item_id,
                            default_name,
                        );
                        if let Some(export_name) = export_name {
                            self.report_conflicting_export_name_maybe(
                                module,
                                export_name,
                                (*item_id).into_any(),
                                ExportConflictKind::Other,
                                &mut exported_names,
                                default_name,
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Unwrap statement wrappers to access the inner expression.
    fn unwrap_statement_expression_for_import(
        &self,
        tree: &destack_dir::NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        let mut current = expression_id;

        // walk through statement wrappers
        loop {
            let expression = tree.get(current);
            if let Expression::Statement { statement } = expression {
                current = *statement;
                continue;
            }

            return current;
        }
    }

    /// Return true when an export local name is disallowed as a binding identifier.
    fn export_local_name_is_disallowed_identifier(&self, name: StringId) -> bool {
        let name = self.program.strings.get(name);
        let Ok(keyword) = Keyword::from_str(name.as_ref()) else {
            return false;
        };

        matches!(
            keyword,
            Keyword::Await
                | Keyword::Break
                | Keyword::Case
                | Keyword::Catch
                | Keyword::Class
                | Keyword::Const
                | Keyword::Continue
                | Keyword::Debugger
                | Keyword::Default
                | Keyword::Delete
                | Keyword::Do
                | Keyword::Else
                | Keyword::Enum
                | Keyword::Export
                | Keyword::Extends
                | Keyword::Finally
                | Keyword::For
                | Keyword::Function
                | Keyword::If
                | Keyword::Import
                | Keyword::In
                | Keyword::InstanceOf
                | Keyword::New
                | Keyword::Return
                | Keyword::Super
                | Keyword::Switch
                | Keyword::This
                | Keyword::Throw
                | Keyword::Try
                | Keyword::Typeof
                | Keyword::Var
                | Keyword::Void
                | Keyword::While
                | Keyword::With
                | Keyword::Yield
                | Keyword::Let
                | Keyword::Static
                | Keyword::Implements
                | Keyword::Interface
                | Keyword::Package
                | Keyword::Private
                | Keyword::Protected
                | Keyword::Public
        )
    }

    /// Resolve the value export name declared by a declaration.
    fn value_export_name_for_declaration(
        &self,
        declaration: &Declaration,
        default_name: StringId,
    ) -> Option<StringId> {
        match declaration {
            Declaration::Global { descriptor, .. }
            | Declaration::Namespace { descriptor, .. }
            | Declaration::Struct { descriptor, .. }
            | Declaration::Class { descriptor, .. }
            | Declaration::Enum { descriptor, .. }
            | Declaration::Function { descriptor, .. }
            | Declaration::Extension { descriptor, .. } => match descriptor.export {
                Some(DependencyMode::Default) => Some(default_name),
                Some(DependencyMode::Item) => descriptor.name.map(|name| name.string()),
                Some(DependencyMode::Namespace) | None => None,
            },
            Declaration::Type { .. }
            | Declaration::ImportAlias { .. }
            | Declaration::Interface { .. } => None,
        }
    }

    /// Resolve the value export name declared by a dependency item.
    fn value_export_name_for_dependency_item(
        &self,
        tree: &destack_dir::NodeTree,
        item_id: LocalNodeId<DependencyItem>,
        default_name: StringId,
    ) -> Option<StringId> {
        let item = tree.get(item_id);

        // collect value-space export names from dependency items
        match item {
            DependencyItem::Value { mode, .. } => match mode {
                DependencyMode::Default => Some(default_name),
                DependencyMode::Item | DependencyMode::Namespace => None,
            },
            DependencyItem::Local {
                mode,
                kind,
                name,
                alias,
                ..
            }
            | DependencyItem::UnresolvedLocal {
                mode,
                kind,
                name,
                alias,
                ..
            }
            | DependencyItem::UnresolvedRemote {
                mode,
                kind,
                name,
                alias,
                ..
            }
            | DependencyItem::Remote {
                mode,
                kind,
                name,
                alias,
                ..
            } => {
                // type-only exports do not conflict with value-space exports
                if *kind != DependencyKind::Value {
                    return None;
                }

                // aliases define the exported name when present
                if let Some(alias) = alias {
                    return Some(*alias);
                }

                // default exports may omit explicit names
                if *mode == DependencyMode::Default {
                    return Some(default_name);
                }

                // fallback to the source item name
                name.map(|name| name.string())
            }
        }
    }

    /// Report duplicate exported names.
    fn report_conflicting_export_name_maybe(
        &self,
        module: &Module,
        export_name: StringId,
        local_node: LocalNodeIdAny,
        conflict_kind: ExportConflictKind,
        exported_names: &mut HashMap<StringId, (LocalNodeIdAny, ExportConflictKind)>,
        default_name: StringId,
    ) {
        // keep the first site as the primary declaration
        let Some((first_node, first_kind)) = exported_names.get(&export_name).copied() else {
            exported_names.insert(export_name, (local_node, conflict_kind));
            return;
        };

        // convert local ids to anchored diagnostics
        let node = local_node.into_global(module.id).into_anchored(None);
        let other_node = first_node.into_global(module.id).into_anchored(None);

        // allow mergeable declaration exports to share one exported name
        if let (ExportConflictKind::Declaration(first), ExportConflictKind::Declaration(next)) =
            (first_kind, conflict_kind)
            && can_merge_declarations(module.language_type, first, next)
        {
            return;
        }

        // report default export conflicts separately
        if export_name == default_name {
            self.error(ImportError::ConflictingDefaultExport {
                node,
                other_node,
                name: Some(export_name),
                module: module.id,
            });
            return;
        }

        self.error(ImportError::ConflictingExport {
            node,
            other_node,
            module: module.id,
            name: Some(StaticKey::Name(export_name)),
        });
    }

    /// Resolve the conflict category for a declaration export.
    fn export_conflict_kind_for_declaration(
        &self,
        declaration: &Declaration,
        symbols: &SymbolTable,
    ) -> ExportConflictKind {
        let symbol_id = declaration.symbol();
        let symbol = symbols.get_symbol(symbol_id);
        ExportConflictKind::Declaration(SymbolDescriptor::from(symbol))
    }

    /// Collect all binding names declared by a pattern.
    fn collect_binding_names_from_pattern(
        &self,
        tree: &destack_dir::NodeTree,
        pattern_id: LocalNodeId<Pattern>,
        names: &mut Vec<StringId>,
    ) {
        match tree.get(pattern_id) {
            Pattern::Wildcard | Pattern::Expression { .. } => {}
            Pattern::Must(right)
            | Pattern::ReferenceOf { right, .. }
            | Pattern::ValueOf { right, .. } => {
                self.collect_binding_names_from_pattern(tree, *right, names);
            }
            Pattern::Binding {
                name,
                pattern: inner,
                ..
            } => {
                names.push(*name);
                if let Some(inner) = inner {
                    self.collect_binding_names_from_pattern(tree, *inner, names);
                }
            }
            Pattern::Range { start, end, .. } => {
                if let Some(start) = start {
                    self.collect_binding_names_from_pattern(tree, *start, names);
                }
                if let Some(end) = end {
                    self.collect_binding_names_from_pattern(tree, *end, names);
                }
            }
            Pattern::Tuple { fields }
            | Pattern::TaggedTuple { fields, .. }
            | Pattern::Array { fields }
            | Pattern::Object { fields }
            | Pattern::TaggedObject { fields, .. } => {
                for field_id in fields {
                    self.collect_binding_names_from_pattern_field(tree, *field_id, names);
                }
            }
            Pattern::Union { patterns } => {
                for pattern_id in patterns {
                    self.collect_binding_names_from_pattern(tree, *pattern_id, names);
                }
            }
        }
    }

    /// Collect all binding names declared by a pattern field.
    fn collect_binding_names_from_pattern_field(
        &self,
        tree: &destack_dir::NodeTree,
        field_id: LocalNodeId<PatternField>,
        names: &mut Vec<StringId>,
    ) {
        match tree.get(field_id) {
            PatternField::Named { name, pattern, .. } => {
                if let Some(pattern) = pattern {
                    self.collect_binding_names_from_pattern(tree, *pattern, names);
                } else {
                    names.push(*name);
                }
            }
            PatternField::Computed { pattern, .. } => {
                if let Some(pattern) = pattern {
                    self.collect_binding_names_from_pattern(tree, *pattern, names);
                }
            }
            PatternField::Alias { alias, .. } => {
                names.push(*alias);
            }
            PatternField::Positional { pattern } => {
                self.collect_binding_names_from_pattern(tree, *pattern, names);
            }
            PatternField::Spread { pattern, .. } => {
                if let Some(pattern) = pattern {
                    self.collect_binding_names_from_pattern(tree, *pattern, names);
                }
            }
            PatternField::Elision => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::TestProgram;

    /// Reject duplicate named exports declared through export list and export declaration.
    #[test]
    fn test_reject_duplicate_named_export() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.mjs", "export {a}; export const a = 1;");
        test.import_module(module_id);
        test.compile();
        test.check_has_diagnostic("EI201");
    }

    /// Reject duplicate default exports in a single module.
    #[test]
    fn test_reject_duplicate_default_export() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.mjs", "export default 1; export default 2;");
        test.import_module(module_id);
        test.compile();
        test.check_has_diagnostic("EI202");
    }

    /// Allow default function overload declarations to share one exported name.
    #[test]
    fn test_allow_default_function_export_overloads() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ts",
            r#"
export default function convert(value: string): string;
export default function convert(value: number): number;
export default function convert(value: string | number): string | number {
    return value;
}
"#,
        );
        test.import_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EI202");
    }

    /// Allow value and type exports that share the same textual name.
    #[test]
    fn test_allow_value_and_type_export_name_overlap() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ts",
            "type a = number; export type {a}; export const a = 1;",
        );
        test.import_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EI201");
    }

    /// Allow class and namespace exports that merge under TypeScript rules.
    #[test]
    fn test_allow_class_namespace_export_merge() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ts",
            "export class Client {} export namespace Client { export interface Options {} }",
        );
        test.import_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EI201");
    }

    /// Reject namespace and class exports when namespace appears first.
    #[test]
    fn test_reject_namespace_class_export_merge_wrong_order() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ts",
            "export namespace Client {} export class Client {}",
        );
        test.import_module(module_id);
        test.compile();
        test.check_has_diagnostic("EI201");
    }

    /// Reject export declarations nested under block statements.
    #[test]
    fn test_reject_nested_export_declaration_not_top_level() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.mjs", "{ export {a}; }");
        test.import_module(module_id);
        test.compile();
        test.check_has_diagnostic("EI305");
    }

    /// Allow nested export declarations inside TypeScript namespace bodies.
    #[test]
    fn test_allow_nested_export_declaration_inside_namespace() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ts", "export namespace N { export {}; }");
        test.import_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EI305");
    }

    /// Reject export clauses that reference missing local names.
    #[test]
    fn test_reject_export_missing_local_binding() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.mjs", "export {a};");
        test.resolve_module(module_id);
        test.compile();
        test.check_has_diagnostic("ER104");
    }

    /// Reject aliased export clauses when the source local is missing.
    #[test]
    fn test_reject_export_missing_local_binding_alias() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.mjs", "let a; export {b as a};");
        test.resolve_module(module_id);
        test.compile();
        test.check_has_diagnostic("ER104");
    }

    /// Reject reserved keyword local names in export item lists.
    #[test]
    fn test_reject_export_reserved_keyword_local_name() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.mjs", "export {if};");
        test.import_module(module_id);
        test.compile();
        test.check_has_diagnostic("EI302");
    }

    /// Reject reserved keyword local names even when exported with an alias.
    #[test]
    fn test_reject_export_reserved_keyword_local_name_with_alias() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.mjs", "export {if as foo};");
        test.import_module(module_id);
        test.compile();
        test.check_has_diagnostic("EI302");
    }
}
