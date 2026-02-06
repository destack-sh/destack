use std::collections::HashMap;

use destack_base::StringId;
use destack_dir::{
    Declaration, DependencyItem, DependencyKind, DependencyMode, Expression, LocalNodeId,
    LocalNodeIdAny, Pattern, PatternField, StaticKey,
};
use destack_workspace::Module;

use crate::{Compiler, ImportError};

impl Compiler {
    /// Validate duplicate value exports in a module.
    pub(super) fn validate_export_conflicts(&self, module: &Module) {
        // load the module tree once for export scanning
        let dir = module.dir_base();
        let tree = dir.tree.read();
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
                        self.report_conflicting_export_name(
                            module,
                            export_name,
                            declaration_id.into_any(),
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
                            self.report_conflicting_export_name(
                                module,
                                name,
                                (*declarator_id).into_any(),
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
                            self.report_conflicting_export_name(
                                module,
                                name,
                                (*declarator_id).into_any(),
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
                            self.report_conflicting_export_name(
                                module,
                                export_name,
                                (*item_id).into_any(),
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
    fn report_conflicting_export_name(
        &self,
        module: &Module,
        export_name: StringId,
        local_node: LocalNodeIdAny,
        exported_names: &mut HashMap<StringId, LocalNodeIdAny>,
        default_name: StringId,
    ) {
        // keep the first site as the primary declaration
        let Some(first_node) = exported_names.get(&export_name).copied() else {
            exported_names.insert(export_name, local_node);
            return;
        };

        // convert local ids to anchored diagnostics
        let node = local_node.into_global(module.id).into_anchored(None);
        let other_node = first_node.into_global(module.id).into_anchored(None);

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
}
