use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use destack_ast::Keyword;
use destack_core::StringId;
use destack_dir::{
    Declaration, DependencyItem, DependencyKind, DependencyMode, ExportMode, Expression,
    ImportSource, LocalNodeId, LocalNodeIdAny, NodeType, Pattern, PatternField, StaticKey,
    SymbolTable, Tree,
};

use crate::common::dir::{SymbolDescriptor, can_merge_declarations};
use crate::import::ImportState;
use crate::{Compiler, CompilerResult, ImportError};

/// The export category used for duplicate export checks.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ExportConflictKind {
    /// A declaration export with merge metadata.
    Declaration(SymbolDescriptor),
    /// Any non-function export.
    Other,
}

/// One exported binding name with its merge classification.
#[derive(Clone, Copy)]
struct BindingExport {
    /// The exported name.
    name: StringId,
    /// The merge behavior for this binding.
    conflict_kind: ExportConflictKind,
}

impl Compiler {
    /// Validate import and export declarations appear at the module root.
    pub(super) fn validate_dependency_top_level(
        &self,
        state: &ImportState<'_>,
    ) -> CompilerResult<()> {
        // declaration files allow nested ambient import and export forms
        if state.module.is_declaration() {
            return Ok(());
        }

        // collect top level expressions
        let mut top_level_expression_ids = HashSet::new();
        for root_id in state.roots {
            top_level_expression_ids.insert(root_id.id);
        }

        // report nested static dependencies as import errors
        for (expression_id, expression) in state.tree.iter_nodes_of_type::<Expression>() {
            let is_top_level = top_level_expression_ids.contains(&expression_id.id);
            if is_top_level {
                continue;
            }

            // import declarations
            if let Expression::Import { source, .. } | Expression::UnresolvedImport { source, .. } =
                expression
            {
                if self.import_dependency_requires_top_level(*source) {
                    let node = expression_id.into_global_any(state.module.id);
                    let anchor = state.anchor(node);
                    state.emit(ImportError::ImportNotTopLevel { anchor })?;
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
                if self.expression_is_within_namespace_declaration(state.tree, expression_id) {
                    continue;
                }

                let node = expression_id.into_global_any(state.module.id);
                let anchor = state.anchor(node);
                state.emit(ImportError::ExportNotTopLevel { anchor })?;
            }
        }

        Ok(())
    }

    /// Return true when an expression is nested under a namespace declaration.
    fn expression_is_within_namespace_declaration(
        &self,
        tree: &Tree,
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
    pub(super) fn validate_export_local_item_names(
        &self,
        state: &ImportState<'_>,
    ) -> CompilerResult<()> {
        // only direct module exports participate in local export name validation
        for root_id in state.roots {
            let Expression::Export { items, .. } = state.tree.get(*root_id) else {
                continue;
            };

            for item_id in items {
                let item = state.tree.get(*item_id);
                let (mode, kind, name) = match item {
                    DependencyItem::UnresolvedLocal {
                        mode, kind, name, ..
                    }
                    | DependencyItem::Local {
                        mode, kind, name, ..
                    } => (mode, kind, name),
                    _ => continue,
                };

                // item mode carries local names that must be binding-compatible
                if *mode != DependencyMode::Item || *kind != DependencyKind::Value {
                    continue;
                }

                let Some(destack_dir::Name::Identifier(name)) = name else {
                    continue;
                };
                if !self.export_local_name_is_disallowed_identifier(*name) {
                    continue;
                }

                let node = item_id.into_global_any(state.module.id);
                let anchor = state.anchor(node);
                state.emit(ImportError::ReservedIdentifier {
                    anchor,
                    name: state.string(*name),
                })?;
            }
        }

        Ok(())
    }

    /// Return true when a dependency source must be top level.
    fn import_dependency_requires_top_level(&self, source: ImportSource) -> bool {
        matches!(
            source,
            ImportSource::ImportStatement
                | ImportSource::ReferencePathDirective
                | ImportSource::ReferenceTypesDirective
                | ImportSource::ReferenceLibDirective
                | ImportSource::ReferenceNoDefaultLibDirective
                | ImportSource::ImportEquals
                | ImportSource::ExportStatement
                | ImportSource::ValueExpression
        )
    }

    /// Validate duplicate value exports in a module.
    pub(super) fn validate_export_conflicts(&self, state: &ImportState<'_>) -> CompilerResult<()> {
        let default_name = StringId::for_text("default");
        let mut exported_names = HashMap::new();

        // scan top-level roots in source order
        for root_id in state.roots {
            let expression = state.tree.get(*root_id);

            // collect exported names by expression kind
            match expression {
                Expression::Declaration(declaration_id) => {
                    let declaration_id = *declaration_id;
                    let declaration = state.tree.get(declaration_id);
                    let export_name =
                        self.value_export_name_for_declaration(declaration, default_name);
                    if let Some(export_name) = export_name {
                        let conflict_kind =
                            self.export_conflict_kind_for_declaration(declaration, state.symbols);
                        self.report_conflicting_export_name_maybe(
                            state,
                            export_name,
                            declaration_id.into_any(),
                            conflict_kind,
                            &mut exported_names,
                            default_name,
                        )?;
                    }
                }
                Expression::Let {
                    export,
                    declarators,
                    ..
                } => {
                    // only exported declarations participate
                    if *export != Some(ExportMode::Named) {
                        continue;
                    }

                    for declarator_id in declarators {
                        let declarator = state.tree.get(*declarator_id);
                        let mut bindings = Vec::new();
                        self.collect_binding_exports_from_pattern(
                            state.tree,
                            state.symbols,
                            declarator.pattern,
                            &mut bindings,
                        );

                        for binding in bindings {
                            self.report_conflicting_export_name_maybe(
                                state,
                                binding.name,
                                declarator_id.into_any(),
                                binding.conflict_kind,
                                &mut exported_names,
                                default_name,
                            )?;
                        }
                    }
                }
                Expression::Using {
                    export,
                    declarators,
                    ..
                } => {
                    // only exported declarations participate
                    if *export != Some(ExportMode::Named) {
                        continue;
                    }

                    for declarator_id in declarators {
                        let declarator = state.tree.get(*declarator_id);
                        let mut bindings = Vec::new();
                        self.collect_binding_exports_from_pattern(
                            state.tree,
                            state.symbols,
                            declarator.pattern,
                            &mut bindings,
                        );

                        for binding in bindings {
                            self.report_conflicting_export_name_maybe(
                                state,
                                binding.name,
                                declarator_id.into_any(),
                                binding.conflict_kind,
                                &mut exported_names,
                                default_name,
                            )?;
                        }
                    }
                }
                Expression::Export { items, .. }
                | Expression::ReExport { items, .. }
                | Expression::UnresolvedReExport { items, .. } => {
                    for item_id in items {
                        let export_name = self.value_export_name_for_dependency_item(
                            state.tree,
                            *item_id,
                            default_name,
                        );
                        if let Some(export_name) = export_name {
                            self.report_conflicting_export_name_maybe(
                                state,
                                export_name,
                                item_id.into_any(),
                                ExportConflictKind::Other,
                                &mut exported_names,
                                default_name,
                            )?;
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Return true when an export local name is disallowed as a binding identifier.
    fn export_local_name_is_disallowed_identifier(&self, name: StringId) -> bool {
        let name = name.to_string();
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
            Declaration::Namespace(declaration) => match declaration.export {
                Some(ExportMode::Default) => Some(default_name),
                Some(ExportMode::Named) => Some(declaration.name.string()),
                None => None,
            },
            Declaration::Struct(declaration) => match declaration.export {
                Some(ExportMode::Default) => Some(default_name),
                Some(ExportMode::Named) => Some(declaration.name.string()),
                None => None,
            },
            Declaration::Class(declaration) => match declaration.export {
                Some(ExportMode::Default) => Some(default_name),
                Some(ExportMode::Named) => declaration.name.map(|name| name.string()),
                None => None,
            },
            Declaration::Enum(declaration) => match declaration.export {
                Some(ExportMode::Default) => Some(default_name),
                Some(ExportMode::Named) => declaration.name.map(|name| name.string()),
                None => None,
            },
            Declaration::Function(declaration) => match declaration.export {
                Some(ExportMode::Default) => Some(default_name),
                Some(ExportMode::Named) => declaration.name.map(|name| name.string()),
                None => None,
            },
            Declaration::Extension(declaration) => match declaration.export {
                Some(ExportMode::Default) => Some(default_name),
                Some(ExportMode::Named) => declaration.name.map(|name| name.string()),
                None => None,
            },
            Declaration::Type(_)
            | Declaration::Global(_)
            | Declaration::ImportAlias(_)
            | Declaration::Interface(_) => None,
        }
    }

    /// Resolve the value export name declared by a dependency item.
    fn value_export_name_for_dependency_item(
        &self,
        tree: &destack_dir::Tree,
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
            DependencyItem::Error => None,
        }
    }

    /// Report duplicate exported names.
    fn report_conflicting_export_name_maybe(
        &self,
        state: &ImportState<'_>,
        export_name: StringId,
        local_node: LocalNodeIdAny,
        conflict_kind: ExportConflictKind,
        exported_names: &mut HashMap<StringId, (LocalNodeIdAny, ExportConflictKind)>,
        default_name: StringId,
    ) -> CompilerResult<()> {
        // keep the first site as the primary declaration
        let Some((first_node, first_kind)) = exported_names.get(&export_name).copied() else {
            exported_names.insert(export_name, (local_node, conflict_kind));
            return Ok(());
        };

        // convert local ids to anchored diagnostics
        let node = local_node.into_global(state.module.id);
        let other_node = first_node.into_global(state.module.id);

        // allow mergeable declaration exports to share one exported name
        if let (ExportConflictKind::Declaration(first), ExportConflictKind::Declaration(next)) =
            (first_kind, conflict_kind)
            && can_merge_declarations(state.module.code_language_type(), first, next)
        {
            return Ok(());
        }

        // report default export conflicts separately
        if export_name == default_name {
            state.emit(ImportError::ConflictingDefaultExport {
                anchor: state.anchor(node),
                other: state.anchor(other_node),
                name: state.string(export_name),
                module: state.module.id,
            })?;

            return Ok(());
        }

        state.emit(ImportError::ConflictingExport {
            anchor: state.anchor(node),
            other: state.anchor(other_node),
            module: state.module.id,
            name: state.static_key(StaticKey::Name(export_name)),
        })?;

        Ok(())
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

    /// Resolve the conflict category for a local symbol.
    fn export_conflict_kind_for_symbol(
        &self,
        symbols: &SymbolTable,
        symbol_id: destack_dir::LocalSymbolId,
    ) -> ExportConflictKind {
        let symbol = symbols.get_symbol(symbol_id);
        ExportConflictKind::Declaration(SymbolDescriptor::from(symbol))
    }

    /// Collect all exported bindings declared by a pattern.
    fn collect_binding_exports_from_pattern(
        &self,
        tree: &destack_dir::Tree,
        symbols: &SymbolTable,
        pattern_id: LocalNodeId<Pattern>,
        bindings: &mut Vec<BindingExport>,
    ) {
        match tree.get(pattern_id) {
            Pattern::Assign { pattern, .. } => {
                self.collect_binding_exports_from_pattern(tree, symbols, *pattern, bindings);
            }
            Pattern::Wildcard | Pattern::Expression { .. } | Pattern::TypeExpression { .. } => {}
            Pattern::Must(right)
            | Pattern::ReferenceOf { right, .. }
            | Pattern::ValueOf { right, .. } => {
                self.collect_binding_exports_from_pattern(tree, symbols, *right, bindings);
            }
            Pattern::Binding {
                name,
                symbol,
                pattern,
                ..
            } => {
                bindings.push(BindingExport {
                    name: *name,
                    conflict_kind: self.export_conflict_kind_for_symbol(symbols, *symbol),
                });

                if let Some(inner) = pattern {
                    self.collect_binding_exports_from_pattern(tree, symbols, *inner, bindings);
                }
            }
            Pattern::Tuple { fields }
            | Pattern::TaggedTuple { fields, .. }
            | Pattern::Array { fields }
            | Pattern::Object { fields }
            | Pattern::TaggedObject { fields, .. } => {
                for field_id in fields {
                    self.collect_binding_exports_from_pattern_field(
                        tree, symbols, *field_id, bindings,
                    );
                }
            }
            Pattern::Union { patterns } => {
                for pattern_id in patterns {
                    self.collect_binding_exports_from_pattern(tree, symbols, *pattern_id, bindings);
                }
            }
        }
    }

    /// Collect all exported bindings declared by a pattern field.
    fn collect_binding_exports_from_pattern_field(
        &self,
        tree: &destack_dir::Tree,
        symbols: &SymbolTable,
        field_id: LocalNodeId<PatternField>,
        bindings: &mut Vec<BindingExport>,
    ) {
        match tree.get(field_id) {
            PatternField::Named {
                name,
                symbol,
                pattern,
                ..
            } => {
                if let Some(pattern) = pattern {
                    self.collect_binding_exports_from_pattern(tree, symbols, *pattern, bindings);
                } else {
                    bindings.push(BindingExport {
                        name: *name,
                        conflict_kind: symbol.map_or(ExportConflictKind::Other, |symbol| {
                            self.export_conflict_kind_for_symbol(symbols, symbol)
                        }),
                    });
                }
            }
            PatternField::Computed { pattern, .. } => {
                self.collect_binding_exports_from_pattern(tree, symbols, *pattern, bindings);
            }
            PatternField::Positional { pattern, .. } => {
                self.collect_binding_exports_from_pattern(tree, symbols, *pattern, bindings);
            }
            PatternField::Spread { pattern, .. } => {
                if let Some(pattern) = pattern {
                    self.collect_binding_exports_from_pattern(tree, symbols, *pattern, bindings);
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

    /// Allow class and runtime namespace exports when class appears first.
    #[test]
    fn test_allow_class_then_runtime_namespace_export_merge() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ts",
            "export class Client {} export namespace Client { export const value = 1; }",
        );
        test.import_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EI201");
    }

    /// Reject runtime namespace and class exports when namespace appears first.
    #[test]
    fn test_reject_runtime_namespace_then_class_export_merge() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ts",
            "export namespace Client { export const value = 1; } export class Client {}",
        );
        test.import_module(module_id);
        test.compile();
        test.check_has_diagnostic("EI201");
    }

    /// Allow type only namespace and class exports in either order.
    #[test]
    fn test_allow_type_only_namespace_then_class_export_merge() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ts",
            "export namespace Client { export interface Options {} } export class Client {}",
        );
        test.import_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EI201");
    }

    /// Allow type only namespace and function exports in either order.
    #[test]
    fn test_allow_type_only_namespace_then_function_export_merge() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ts",
            "export namespace Factory { export interface Options {} } export function Factory() {}",
        );
        test.import_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EI201");
    }

    /// Allow runtime function and namespace export merges in Destack modules.
    #[test]
    fn test_allow_runtime_function_then_namespace_export_merge_in_destack() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            "export function inspect(): string { return \"ok\"; } export namespace inspect { export const custom = 1; }",
        );
        test.import_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EI201");
    }

    /// Reject runtime namespace and function exports when namespace appears first.
    #[test]
    fn test_reject_runtime_namespace_then_function_export_merge() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ts",
            "export namespace Factory { export const value = 1; } export function Factory() {}",
        );
        test.import_module(module_id);
        test.compile();
        test.check_has_diagnostic("EI201");
    }

    /// Allow ambient namespace and variable exports in either order.
    #[test]
    fn test_allow_ambient_namespace_with_variable_export_merge() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ts",
            "export const Tag = 1; export declare namespace Tag { export type A = number; }",
        );
        test.import_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EI201");
    }

    /// Allow ambient namespace and variable exports when namespace appears first.
    #[test]
    fn test_allow_ambient_namespace_then_variable_export_merge() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ts",
            "export declare namespace Tag { export type A = number; } export const Tag = 1;",
        );
        test.import_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EI201");
    }

    /// Reject runtime namespace and variable exports.
    #[test]
    fn test_reject_runtime_namespace_with_variable_export_merge() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ts",
            "export const Tag = 1; export namespace Tag { export const A = 2; }",
        );
        test.import_module(module_id);
        test.compile();
        test.check_has_diagnostic("EI201");
    }

    /// Allow runtime namespace merges when the namespace is type-only.
    #[test]
    fn test_allow_runtime_type_only_namespace_with_variable_export_merge() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ts",
            "export namespace Tag { export type A = number; } export const Tag = 1;",
        );
        test.import_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EI201");
    }

    /// Allow runtime type-only namespace merges with typed variable exports.
    #[test]
    fn test_allow_runtime_type_only_namespace_with_typed_variable_export_merge() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ts",
            "export namespace fn { export type Gen = {}; export type NonGen = {}; } export const fn: fn.Gen & fn.NonGen = {} as any;",
        );
        test.import_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EI201");
    }

    /// Allow one typed variable export with no duplicate diagnostics.
    #[test]
    fn test_allow_single_typed_variable_export() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ts", r#"export const Tag: string = "a";"#);
        test.import_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EI201");
    }

    /// Allow exported ambient namespaces to merge with exported values typed from that namespace.
    #[test]
    fn test_allow_exported_ambient_namespace_merge_with_typed_value_reference() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ts",
            r#"
export declare namespace Tag {
    export type Inner = string;
}

export const Tag: Tag.Inner = "value";
"#,
        );
        test.import_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EI201");
    }

    /// Allow exported ambient namespace merges for shorthand destructured value bindings.
    #[test]
    fn test_allow_exported_ambient_namespace_merge_with_shorthand_destructured_value() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ts",
            r#"
const source = { Tag: 1 };
export const { Tag } = source;

export declare namespace Tag {
    export type Inner = string;
}
"#,
        );
        test.import_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EI201");
    }

    /// Ignore global augmentation declarations when collecting module exports.
    #[test]
    fn test_ignore_global_augmentations_for_duplicate_module_exports() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.d.ts",
            r#"
export {};

declare global {
    interface URL {}
    var URL: { new(): URL };
}

export class URL {}
"#,
        );
        test.import_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EI201");
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
