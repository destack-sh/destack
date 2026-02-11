use destack_ast::StringId;
use destack_dir::{
    DynamicKey, Export, ExportKind, ExportTarget, Expression, GlobalSymbolId, LocalNodeId,
    LocalScopeId, LocalSymbolId, ModuleTarget, NodeTree, Property, ScalarLiteral, StaticKey,
    SymbolBinding, SymbolKind, SymbolSpace, SymbolTable,
};
use destack_source::ModuleId;
use destack_workspace::ProfileId;
use indexmap::IndexMap;

use crate::{Compiler, ResolveResult};

/// A static CommonJS assignment value that can back an export target.
#[derive(Debug, Clone, Copy, PartialEq)]
enum CommonjsExportValue {
    /// A value expression from an assignment.
    Expression(LocalNodeId<Expression>),
    /// A local symbol from object literal shorthand or methods.
    Symbol(LocalSymbolId),
}

/// A classified assignment target in the CommonJS surface.
#[derive(Debug, Clone, Copy, PartialEq)]
enum CommonjsAssignmentTarget {
    /// `module.exports = value`
    ModuleExports,
    /// `exports = value`
    Exports,
    /// `module.exports.name = value`
    ModuleExportsProperty(StringId),
    /// `exports.name = value`
    ExportsProperty(StringId),
}

/// Accumulated CommonJS static export state for one module.
#[derive(Debug, Clone, PartialEq)]
struct CommonjsExportState {
    /// The latest assigned `module.exports` value expression.
    default_value: Option<LocalNodeId<Expression>>,
    /// The currently visible named exports on `module.exports`.
    named_values: IndexMap<StringId, CommonjsExportValue>,
    /// Whether `exports` still aliases the current `module.exports` object.
    is_exports_alias_linked: bool,
}

/// One assignment effect collected from a top-level assignment chain.
#[derive(Debug, Clone, Copy, PartialEq)]
struct CommonjsAssignmentEffect {
    /// The left-hand side of the assignment.
    left: LocalNodeId<Expression>,
    /// The original right-hand expression assigned to `left`.
    assigned_expression: LocalNodeId<Expression>,
    /// The runtime value produced by the assignment.
    value: LocalNodeId<Expression>,
}

impl CommonjsExportState {
    /// Create an empty CommonJS export state.
    fn new() -> Self {
        Self {
            default_value: None,
            named_values: IndexMap::new(),
            is_exports_alias_linked: true,
        }
    }
}

impl Compiler {
    /// Insert static named CommonJS exports into one module export table.
    pub(super) fn insert_commonjs_named_exports(
        &self,
        module_id: ModuleId,
        namespace_scope: LocalScopeId,
        namespace_symbol: GlobalSymbolId,
        roots: &[LocalNodeId<Expression>],
        tree: &NodeTree,
        symbols: &SymbolTable,
        exports: &mut IndexMap<(SymbolSpace, StaticKey), Export>,
    ) {
        // collect static CommonJS assignment state for this module
        let state = self.collect_commonjs_export_state(
            module_id,
            namespace_scope,
            namespace_symbol,
            roots,
            tree,
            symbols,
        );

        // synthesize named value exports from the final static state
        for (name, value) in state.named_values {
            let Some(symbol) = self.resolve_commonjs_export_value_symbol(
                tree,
                symbols,
                module_id,
                namespace_scope,
                value,
            ) else {
                continue;
            };

            let key = StaticKey::Name(name);

            // keep explicit export entries over synthesized commonjs entries
            if exports.contains_key(&(SymbolSpace::Value, key)) {
                continue;
            }

            exports.insert(
                (SymbolSpace::Value, key),
                self.commonjs_export_entry_for_symbol(module_id, key, symbol),
            );
        }
    }

    /// Resolve a CommonJS default export symbol from `module.exports = ...` assignments.
    pub(super) fn resolve_commonjs_default_export_symbol(
        &self,
        origin_module_id: ModuleId,
        target: ModuleTarget,
        profile: ProfileId,
    ) -> ResolveResult<Option<GlobalSymbolId>> {
        // handle direct module targets
        if let ModuleTarget::Module(module_id) = target {
            return self.resolve_commonjs_default_export_symbol_for_module(
                origin_module_id,
                module_id,
                profile,
            );
        }

        // handle module binding targets
        let ModuleTarget::Binding(specifier) = target else {
            return Ok(None);
        };
        let bindings = self.module_bindings_for_specifier(origin_module_id, profile, specifier)?;
        let Some(bindings) = bindings else {
            return Ok(None);
        };

        // return the first module binding that exposes a CommonJS default
        for binding in bindings {
            let symbol = self.resolve_commonjs_default_export_symbol_for_module(
                origin_module_id,
                binding.module_id,
                profile,
            )?;
            if symbol.is_some() {
                return Ok(symbol);
            }
        }

        Ok(None)
    }

    /// Resolve a CommonJS default export symbol for one module.
    fn resolve_commonjs_default_export_symbol_for_module(
        &self,
        origin_module_id: ModuleId,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> ResolveResult<Option<GlobalSymbolId>> {
        // only code modules can carry CommonJS assignments
        if !self.is_code_module(module_id) {
            return Ok(None);
        }

        // skip non commonjs modules
        let module = self.program.modules.get(module_id);
        let module = module.read();
        if !module.module_format.is_commonjs() {
            return Ok(None);
        }
        drop(module);

        // ensure the target module is ready before scanning roots
        self.require_resolve_module_prepare_if_needed(origin_module_id, module_id, profile)?;

        // load target module state
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();

        // collect static CommonJS assignment state and read the final default value
        let state = self.collect_commonjs_export_state(
            module_id,
            dir.namespace_scope,
            dir.namespace_symbol.into_global(module_id),
            &dir.roots,
            &tree,
            &symbols,
        );
        let Some(value_expression_id) = state.default_value else {
            return Ok(None);
        };

        // resolve the assignment value to a symbol when possible
        let value_symbol = self.resolve_commonjs_assignment_value_symbol(
            &tree,
            &symbols,
            module_id,
            dir.namespace_scope,
            value_expression_id,
        );

        // keep unresolved assignment values unresolved
        let value_symbol = if value_symbol.is_none()
            && self.commonjs_assignment_value_is_unresolved_path(&tree, value_expression_id)
        {
            None
        }
        // use module default fallback for dynamic runtime values
        else {
            value_symbol.or_else(|| Some(dir.default_symbol.into_global(module_id)))
        };

        Ok(value_symbol)
    }

    /// Collect static CommonJS assignment state from top-level roots.
    fn collect_commonjs_export_state(
        &self,
        module_id: ModuleId,
        namespace_scope: LocalScopeId,
        namespace_symbol: GlobalSymbolId,
        roots: &[LocalNodeId<Expression>],
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> CommonjsExportState {
        // cache common runtime identifiers
        let module_name = self.program.strings.intern("module");
        let exports_name = self.program.strings.intern("exports");
        let is_module_name_shadowed =
            self.module_name_is_shadowed(symbols, module_id, namespace_scope, module_name);
        let is_exports_name_shadowed =
            self.exports_name_is_shadowed(symbols, module_id, namespace_scope, exports_name);

        // scan top-level assignment statements in source order
        let mut state = CommonjsExportState::new();
        let mut assignments: Vec<CommonjsAssignmentEffect> = Vec::new();
        for root_id in roots {
            let statement_id = self.top_level_expression_without_statement(tree, *root_id);
            let statement_id = self.unwrap_commonjs_transparent_expression(tree, statement_id);
            assignments.clear();

            if self
                .collect_commonjs_assignment_chain(tree, statement_id, &mut assignments)
                .is_none()
            {
                continue;
            }

            // apply assignment effects in runtime order, right to left
            for assignment in assignments.iter().copied() {
                self.apply_commonjs_assignment_effect(
                    tree,
                    symbols,
                    namespace_scope,
                    module_name,
                    exports_name,
                    namespace_symbol,
                    is_module_name_shadowed,
                    is_exports_name_shadowed,
                    assignment.left,
                    assignment.assigned_expression,
                    assignment.value,
                    &mut state,
                );
            }
        }

        state
    }

    /// Collect assignment links for one top-level assignment chain.
    fn collect_commonjs_assignment_chain(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        assignments: &mut Vec<CommonjsAssignmentEffect>,
    ) -> Option<LocalNodeId<Expression>> {
        let expression_id = self.unwrap_commonjs_transparent_expression(tree, expression_id);
        let Expression::Assign { left, right } = tree.get(expression_id) else {
            return None;
        };

        // process the right side first to preserve assignment order
        let right_id = self.unwrap_commonjs_transparent_expression(tree, *right);
        let value_expression = self
            .collect_commonjs_assignment_chain(tree, right_id, assignments)
            .unwrap_or(right_id);

        assignments.push(CommonjsAssignmentEffect {
            left: *left,
            assigned_expression: right_id,
            value: value_expression,
        });

        Some(value_expression)
    }

    /// Apply one CommonJS assignment effect to the running state.
    #[allow(clippy::too_many_arguments)]
    fn apply_commonjs_assignment_effect(
        &self,
        tree: &NodeTree,
        symbols: &SymbolTable,
        namespace_scope: LocalScopeId,
        module_name: StringId,
        exports_name: StringId,
        namespace_symbol: GlobalSymbolId,
        is_module_name_shadowed: bool,
        is_exports_name_shadowed: bool,
        left: LocalNodeId<Expression>,
        assigned_expression: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        state: &mut CommonjsExportState,
    ) {
        let Some(target) = self.commonjs_assignment_target(
            tree,
            left,
            module_name,
            exports_name,
            namespace_symbol,
            is_module_name_shadowed,
            is_exports_name_shadowed,
        ) else {
            return;
        };

        // direct module replacement sets default and replaces named state
        if target == CommonjsAssignmentTarget::ModuleExports {
            state.default_value = Some(value);
            state.named_values = self
                .collect_commonjs_named_exports_from_default_value(
                    tree,
                    symbols,
                    namespace_scope,
                    value,
                )
                .unwrap_or_default();
            state.is_exports_alias_linked = false;
            return;
        }

        // direct exports assignment can relink aliases in static forms
        if target == CommonjsAssignmentTarget::Exports {
            // relink explicit `exports = module.exports` assignments
            let relinks_to_module_exports = self.expression_is_commonjs_module_exports(
                tree,
                assigned_expression,
                module_name,
                exports_name,
                namespace_symbol,
                is_module_name_shadowed,
            );

            // relink assignments that directly write module exports
            let relinks_to_module_exports_assignment = self
                .expression_assigns_commonjs_module_exports(
                    tree,
                    assigned_expression,
                    module_name,
                    exports_name,
                    namespace_symbol,
                    is_module_name_shadowed,
                    is_exports_name_shadowed,
                );

            // relink fallback when exports receives the current default value
            let relinks_to_current_default_expression = state.default_value == Some(value);

            state.is_exports_alias_linked = relinks_to_module_exports
                || relinks_to_module_exports_assignment
                || relinks_to_current_default_expression;
            return;
        }

        // module exports property writes are always on the live export object
        if let CommonjsAssignmentTarget::ModuleExportsProperty(name) = target {
            state
                .named_values
                .insert(name, CommonjsExportValue::Expression(value));
            return;
        }

        // exports property writes apply only while aliasing module exports
        if let CommonjsAssignmentTarget::ExportsProperty(name) = target
            && state.is_exports_alias_linked
        {
            state
                .named_values
                .insert(name, CommonjsExportValue::Expression(value));
        }
    }

    /// Classify one assignment left-hand side in the CommonJS static model.
    #[allow(clippy::too_many_arguments)]
    fn commonjs_assignment_target(
        &self,
        tree: &NodeTree,
        left: LocalNodeId<Expression>,
        module_name: StringId,
        exports_name: StringId,
        namespace_symbol: GlobalSymbolId,
        is_module_name_shadowed: bool,
        is_exports_name_shadowed: bool,
    ) -> Option<CommonjsAssignmentTarget> {
        let left = self.unwrap_commonjs_transparent_expression(tree, left);

        // direct module exports replacement
        if self.expression_is_commonjs_module_exports(
            tree,
            left,
            module_name,
            exports_name,
            namespace_symbol,
            is_module_name_shadowed,
        ) {
            return Some(CommonjsAssignmentTarget::ModuleExports);
        }

        // direct exports alias reassignment
        if self.expression_is_commonjs_exports_identifier(
            tree,
            left,
            exports_name,
            namespace_symbol,
            is_exports_name_shadowed,
        ) {
            return Some(CommonjsAssignmentTarget::Exports);
        }

        // nested assignment trees may encode chained assignments on the left
        if let Expression::Assign {
            left: nested_left,
            right: nested_right,
        } = tree.get(left)
        {
            let nested_left_target = self.commonjs_assignment_target(
                tree,
                *nested_left,
                module_name,
                exports_name,
                namespace_symbol,
                is_module_name_shadowed,
                is_exports_name_shadowed,
            );
            if nested_left_target.is_some() {
                return nested_left_target;
            }

            let nested_right_target = self.commonjs_assignment_target(
                tree,
                *nested_right,
                module_name,
                exports_name,
                namespace_symbol,
                is_module_name_shadowed,
                is_exports_name_shadowed,
            );
            if nested_right_target.is_some() {
                return nested_right_target;
            }
        }

        // dot property assignments
        if let Expression::Member {
            left: object,
            name,
            static_arguments,
        } = tree.get(left)
        {
            if static_arguments.is_some() {
                return None;
            }

            if self.expression_is_commonjs_module_exports(
                tree,
                *object,
                module_name,
                exports_name,
                namespace_symbol,
                is_module_name_shadowed,
            ) {
                return Some(CommonjsAssignmentTarget::ModuleExportsProperty(*name));
            }

            if self.expression_is_commonjs_exports_identifier(
                tree,
                *object,
                exports_name,
                namespace_symbol,
                is_exports_name_shadowed,
            ) {
                return Some(CommonjsAssignmentTarget::ExportsProperty(*name));
            }
        }

        // bracket property assignments
        if let Expression::Index {
            left: object,
            right,
        } = tree.get(left)
            && let Some(index_id) = right
            && let Some(name) = self.commonjs_static_property_name_from_expression(tree, *index_id)
        {
            if self.expression_is_commonjs_module_exports(
                tree,
                *object,
                module_name,
                exports_name,
                namespace_symbol,
                is_module_name_shadowed,
            ) {
                return Some(CommonjsAssignmentTarget::ModuleExportsProperty(name));
            }

            if self.expression_is_commonjs_exports_identifier(
                tree,
                *object,
                exports_name,
                namespace_symbol,
                is_exports_name_shadowed,
            ) {
                return Some(CommonjsAssignmentTarget::ExportsProperty(name));
            }
        }

        // unresolved and resolved paths may encode chained members directly
        if let Some(target) = self.commonjs_assignment_target_from_path(
            tree,
            left,
            module_name,
            exports_name,
            namespace_symbol,
            is_module_name_shadowed,
            is_exports_name_shadowed,
        ) {
            return Some(target);
        }

        None
    }

    /// Classify path-encoded CommonJS property assignments.
    #[allow(clippy::too_many_arguments)]
    fn commonjs_assignment_target_from_path(
        &self,
        tree: &NodeTree,
        left: LocalNodeId<Expression>,
        module_name: StringId,
        exports_name: StringId,
        namespace_symbol: GlobalSymbolId,
        is_module_name_shadowed: bool,
        is_exports_name_shadowed: bool,
    ) -> Option<CommonjsAssignmentTarget> {
        let (path, static_arguments, target_symbol) = match tree.get(left) {
            Expression::UnresolvedPath {
                path,
                static_arguments,
                ..
            } => (path, static_arguments, None),
            Expression::ModuleReference {
                path,
                static_arguments,
                target_symbol,
            }
            | Expression::GlobalReference {
                path,
                static_arguments,
                target_symbol,
            } => (path, static_arguments, Some(*target_symbol)),
            _ => return None,
        };

        // static arguments change runtime value, so skip synthesis
        if static_arguments.is_some() {
            return None;
        }

        // `exports.name`
        let is_exports_path = path.segments.len() == 2
            && path.segments[0] == exports_name
            && !is_exports_name_shadowed;
        let is_exports_symbol = target_symbol.is_some_and(|symbol| {
            symbol == namespace_symbol
                && path.segments.len() == 2
                && path.segments[0] == exports_name
        });
        if is_exports_path || is_exports_symbol {
            return Some(CommonjsAssignmentTarget::ExportsProperty(path.segments[1]));
        }

        // `module.exports.name`
        let is_module_exports_path = path.segments.len() == 3
            && path.segments[0] == module_name
            && path.segments[1] == exports_name
            && !is_module_name_shadowed;
        let is_module_exports_symbol = target_symbol.is_some_and(|symbol| {
            symbol == namespace_symbol
                && path.segments.len() == 3
                && path.segments[0] == module_name
                && path.segments[1] == exports_name
        });
        if is_module_exports_path || is_module_exports_symbol {
            return Some(CommonjsAssignmentTarget::ModuleExportsProperty(
                path.segments[2],
            ));
        }

        None
    }

    /// Collect named exports from a replacement `module.exports = value` expression.
    fn collect_commonjs_named_exports_from_default_value(
        &self,
        tree: &NodeTree,
        symbols: &SymbolTable,
        namespace_scope: LocalScopeId,
        value: LocalNodeId<Expression>,
    ) -> Option<IndexMap<StringId, CommonjsExportValue>> {
        let value = self.unwrap_commonjs_transparent_expression(tree, value);
        let Expression::ObjectExpression { properties } = tree.get(value) else {
            return None;
        };

        let mut named_values = IndexMap::new();
        for property_id in properties {
            let property = tree.get(*property_id);

            // object spreads are dynamic and outside the static synthesis surface
            if matches!(property, Property::Spread { .. }) {
                return None;
            }

            // collect object fields
            if let Property::Field { key, value, .. } = property {
                let Some(export_name) = self.commonjs_property_key_name(tree, *key) else {
                    return None;
                };
                let Some(export_value) = self.commonjs_export_value_for_object_field(
                    symbols,
                    namespace_scope,
                    *key,
                    *value,
                ) else {
                    return None;
                };
                named_values.insert(export_name, export_value);
                continue;
            }

            // collect object methods
            if let Property::Method { key, symbol, .. } = property {
                let Some(export_name) = self.commonjs_property_key_name(tree, *key) else {
                    return None;
                };
                named_values.insert(export_name, CommonjsExportValue::Symbol(*symbol));
            }
        }

        Some(named_values)
    }

    /// Resolve the static export value for one object literal field.
    fn commonjs_export_value_for_object_field(
        &self,
        symbols: &SymbolTable,
        namespace_scope: LocalScopeId,
        key: Option<DynamicKey>,
        value: Option<LocalNodeId<Expression>>,
    ) -> Option<CommonjsExportValue> {
        // explicit values map directly
        if let Some(value) = value {
            return Some(CommonjsExportValue::Expression(value));
        }

        // shorthand values map to symbols in the module namespace scope
        let Some(DynamicKey::Name(name)) = key else {
            return None;
        };
        let symbol = self.find_commonjs_value_symbol_in_scope(symbols, namespace_scope, name)?;

        Some(CommonjsExportValue::Symbol(symbol))
    }

    /// Resolve a static property name from a property key.
    fn commonjs_property_key_name(
        &self,
        tree: &NodeTree,
        key: Option<DynamicKey>,
    ) -> Option<StringId> {
        match key {
            Some(DynamicKey::Name(name)) => Some(name),
            Some(DynamicKey::Number(name)) => Some(name),
            Some(DynamicKey::Expression(expression_id)) => {
                self.commonjs_static_property_name_from_expression(tree, expression_id)
            }
            Some(DynamicKey::Private(_)) | Some(DynamicKey::NamedExpression { .. }) | None => None,
        }
    }

    /// Resolve a static property name from an index expression.
    fn commonjs_static_property_name_from_expression(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<StringId> {
        let expression_id = self.unwrap_commonjs_transparent_expression(tree, expression_id);
        match tree.get(expression_id) {
            Expression::ScalarLiteral {
                value: ScalarLiteral::String(name),
            } => Some(*name),
            Expression::ScalarLiteral {
                value: ScalarLiteral::Boolean(value),
            } => {
                let value = if *value { "true" } else { "false" };
                Some(self.program.strings.intern(value))
            }
            Expression::ScalarLiteral {
                value: ScalarLiteral::Integer(value),
            }
            | Expression::ScalarLiteral {
                value: ScalarLiteral::Bigint(value),
            } => {
                let value = value.to_string();
                Some(self.program.strings.intern(&value))
            }
            Expression::ScalarLiteral {
                value: ScalarLiteral::Float(value),
            } => {
                let value = value.to_string();
                Some(self.program.strings.intern(&value))
            }
            Expression::ScalarLiteral {
                value: ScalarLiteral::Character(value),
            } => {
                let value = value.to_string();
                Some(self.program.strings.intern(&value))
            }
            _ => None,
        }
    }

    /// Build one export entry from a resolved CommonJS target symbol.
    fn commonjs_export_entry_for_symbol(
        &self,
        module_id: ModuleId,
        key: StaticKey,
        symbol: GlobalSymbolId,
    ) -> Export {
        // local symbols can use the compact local export form
        if symbol.module_id == module_id {
            return Export::local(module_id, key, SymbolSpace::Value, symbol.local_id);
        }

        // remote symbols can still be addressed through a resolved target
        Export {
            key,
            space: SymbolSpace::Value,
            kind: ExportKind::Local,
            target: ExportTarget::Resolved(symbol),
            symbol: None,
            item: None,
        }
    }

    /// Resolve a symbol target for one CommonJS export value.
    fn resolve_commonjs_export_value_symbol(
        &self,
        tree: &NodeTree,
        symbols: &SymbolTable,
        module_id: ModuleId,
        scope_id: LocalScopeId,
        value: CommonjsExportValue,
    ) -> Option<GlobalSymbolId> {
        match value {
            CommonjsExportValue::Expression(expression_id) => self
                .resolve_commonjs_assignment_value_symbol(
                    tree,
                    symbols,
                    module_id,
                    scope_id,
                    expression_id,
                ),
            CommonjsExportValue::Symbol(symbol) => Some(symbol.into_global(module_id)),
        }
    }

    /// Return the contained expression when a root is wrapped in a statement node.
    fn top_level_expression_without_statement(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        match tree.get(expression_id) {
            Expression::Statement { statement } => *statement,
            _ => expression_id,
        }
    }

    /// Unwrap transparent wrappers for static CommonJS matching.
    fn unwrap_commonjs_transparent_expression(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        let mut expression_id = expression_id;

        loop {
            match tree.get(expression_id) {
                Expression::Parenthesized { expression } => {
                    expression_id = *expression;
                }
                Expression::Cast { value, .. } => {
                    expression_id = *value;
                }
                _ => return expression_id,
            }
        }
    }

    /// Check whether an expression is `module.exports`.
    fn expression_is_commonjs_module_exports(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        module_name: StringId,
        exports_name: StringId,
        module_namespace_symbol: GlobalSymbolId,
        is_module_name_shadowed: bool,
    ) -> bool {
        let expression_id = self.unwrap_commonjs_transparent_expression(tree, expression_id);

        // handle explicit member syntax: module.exports
        if let Expression::Member {
            left,
            name,
            static_arguments,
        } = tree.get(expression_id)
        {
            if static_arguments.is_some() || *name != exports_name {
                return false;
            }

            return self.expression_is_commonjs_module_identifier(
                tree,
                *left,
                module_name,
                module_namespace_symbol,
                is_module_name_shadowed,
            );
        }

        // handle bracket syntax: module["exports"]
        if let Expression::Index { left, right } = tree.get(expression_id) {
            let Some(index_id) = right else {
                return false;
            };
            if !self.expression_is_commonjs_module_identifier(
                tree,
                *left,
                module_name,
                module_namespace_symbol,
                is_module_name_shadowed,
            ) {
                return false;
            }

            return self.expression_is_commonjs_exports_literal(tree, *index_id, exports_name);
        }

        // handle encoded path syntax: module.exports
        let (path, static_arguments) = match tree.get(expression_id) {
            Expression::UnresolvedPath {
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
            } => (path, static_arguments),
            _ => return false,
        };

        if static_arguments.is_some() {
            return false;
        }

        // prefer symbol level matching for resolved commonjs runtime references
        let has_commonjs_namespace_target = match tree.get(expression_id) {
            Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => {
                *target_symbol == module_namespace_symbol
            }
            _ => false,
        };
        if has_commonjs_namespace_target
            && path.segments.len() == 2
            && path.segments[0] == module_name
            && path.segments[1] == exports_name
        {
            return true;
        }

        // fallback to unresolved path shape when runtime names were not resolved
        !is_module_name_shadowed
            && path.segments.len() == 2
            && path.segments[0] == module_name
            && path.segments[1] == exports_name
    }

    /// Check whether an expression assigns to `module.exports`.
    #[allow(clippy::too_many_arguments)]
    fn expression_assigns_commonjs_module_exports(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        module_name: StringId,
        exports_name: StringId,
        namespace_symbol: GlobalSymbolId,
        is_module_name_shadowed: bool,
        is_exports_name_shadowed: bool,
    ) -> bool {
        let expression_id = self.unwrap_commonjs_transparent_expression(tree, expression_id);
        let Expression::Assign { left, .. } = tree.get(expression_id) else {
            return false;
        };

        matches!(
            self.commonjs_assignment_target(
                tree,
                *left,
                module_name,
                exports_name,
                namespace_symbol,
                is_module_name_shadowed,
                is_exports_name_shadowed
            ),
            Some(CommonjsAssignmentTarget::ModuleExports)
        )
    }

    /// Check whether an expression is the CommonJS `module` identifier.
    fn expression_is_commonjs_module_identifier(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        module_name: StringId,
        module_namespace_symbol: GlobalSymbolId,
        is_module_name_shadowed: bool,
    ) -> bool {
        let expression_id = self.unwrap_commonjs_transparent_expression(tree, expression_id);
        let (path, static_arguments) = match tree.get(expression_id) {
            Expression::UnresolvedPath {
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
            } => (path, static_arguments),
            _ => return false,
        };

        if static_arguments.is_some() {
            return false;
        }

        // prefer symbol level matching for resolved commonjs runtime references
        let has_commonjs_namespace_target = match tree.get(expression_id) {
            Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => {
                *target_symbol == module_namespace_symbol
            }
            _ => false,
        };
        if has_commonjs_namespace_target
            && path.segments.len() == 1
            && path.first_segment() == Some(module_name)
        {
            return true;
        }

        // fallback to unresolved path shape when runtime names were not resolved
        !is_module_name_shadowed
            && path.segments.len() == 1
            && path.first_segment() == Some(module_name)
    }

    /// Check whether an expression is the CommonJS `exports` identifier.
    fn expression_is_commonjs_exports_identifier(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        exports_name: StringId,
        module_namespace_symbol: GlobalSymbolId,
        is_exports_name_shadowed: bool,
    ) -> bool {
        let expression_id = self.unwrap_commonjs_transparent_expression(tree, expression_id);
        let (path, static_arguments) = match tree.get(expression_id) {
            Expression::UnresolvedPath {
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
            } => (path, static_arguments),
            _ => return false,
        };

        if static_arguments.is_some() {
            return false;
        }

        // prefer symbol level matching for resolved commonjs runtime references
        let has_commonjs_namespace_target = match tree.get(expression_id) {
            Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => {
                *target_symbol == module_namespace_symbol
            }
            _ => false,
        };
        if has_commonjs_namespace_target
            && path.segments.len() == 1
            && path.first_segment() == Some(exports_name)
        {
            return true;
        }

        // fallback to unresolved path shape when runtime names were not resolved
        !is_exports_name_shadowed
            && path.segments.len() == 1
            && path.first_segment() == Some(exports_name)
    }

    /// Check whether the current module scope declares `module`.
    fn module_name_is_shadowed(
        &self,
        symbols: &SymbolTable,
        module_id: ModuleId,
        scope_id: LocalScopeId,
        module_name: StringId,
    ) -> bool {
        self.runtime_name_is_shadowed(symbols, module_id, scope_id, module_name)
    }

    /// Check whether the current module scope declares `exports`.
    fn exports_name_is_shadowed(
        &self,
        symbols: &SymbolTable,
        module_id: ModuleId,
        scope_id: LocalScopeId,
        exports_name: StringId,
    ) -> bool {
        self.runtime_name_is_shadowed(symbols, module_id, scope_id, exports_name)
    }

    /// Check whether the current module scope declares a runtime name.
    fn runtime_name_is_shadowed(
        &self,
        symbols: &SymbolTable,
        module_id: ModuleId,
        scope_id: LocalScopeId,
        name: StringId,
    ) -> bool {
        let scope = symbols.get_scope_by_id(scope_id);
        let Some(symbol_id) = symbols.find_active_symbol(scope, StaticKey::Name(name)) else {
            return false;
        };
        let symbol = symbols.get_symbol(symbol_id);

        symbol.module_id == module_id && symbol.binding == SymbolBinding::Runtime
    }

    /// Check whether an expression is the string literal `"exports"`.
    fn expression_is_commonjs_exports_literal(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        exports_name: StringId,
    ) -> bool {
        let expression_id = self.unwrap_commonjs_transparent_expression(tree, expression_id);
        matches!(
            tree.get(expression_id),
            Expression::ScalarLiteral {
                value: ScalarLiteral::String(name)
            } if *name == exports_name
        )
    }

    /// Resolve a symbol target for one CommonJS assignment value.
    fn resolve_commonjs_assignment_value_symbol(
        &self,
        tree: &NodeTree,
        symbols: &SymbolTable,
        module_id: ModuleId,
        scope_id: LocalScopeId,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<GlobalSymbolId> {
        // use a direct resolved target when available
        let expression_id = self.unwrap_commonjs_transparent_expression(tree, expression_id);
        let expression = tree.get(expression_id);
        if let Some(target_symbol) = expression.target_symbol() {
            return Some(target_symbol);
        }

        // resolve unresolved top level bindings by name
        let Expression::UnresolvedPath {
            path,
            static_arguments,
            ..
        } = expression
        else {
            return None;
        };
        if static_arguments.is_some() || path.segments.len() != 1 {
            return None;
        }

        let name = path.first_segment()?;
        let symbol = self.find_commonjs_value_symbol_in_scope(symbols, scope_id, name)?;

        Some(symbol.into_global(module_id))
    }

    /// Return true when an assignment value resolves to an unresolved path reference.
    fn commonjs_assignment_value_is_unresolved_path(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        let expression_id = self.unwrap_commonjs_transparent_expression(tree, expression_id);
        matches!(tree.get(expression_id), Expression::UnresolvedPath { .. })
    }

    /// Find a value symbol by name in one scope, preferring namespace merges.
    fn find_commonjs_value_symbol_in_scope(
        &self,
        symbols: &SymbolTable,
        scope_id: LocalScopeId,
        name: StringId,
    ) -> Option<LocalSymbolId> {
        let scope = symbols.get_scope_by_id(scope_id);
        let key = StaticKey::Name(name);
        let mut fallback_symbol = None;

        // prefer namespace symbols for class and namespace merges
        for (candidate_key, symbol_id) in symbols.active_named_symbols(scope) {
            if candidate_key != key {
                continue;
            }

            let symbol = symbols.get_symbol(symbol_id);
            if symbol.kind == SymbolKind::Namespace {
                return Some(symbol_id);
            }

            if fallback_symbol.is_none() {
                fallback_symbol = Some(symbol_id);
            }
        }

        fallback_symbol
    }
}

#[cfg(test)]
mod tests {
    use crate::TestProgram;

    use super::{CommonjsExportState, CommonjsExportValue};

    /// Collect the static CommonJS export state for one module.
    fn collect_commonjs_state(
        test: &TestProgram,
        module_id: destack_source::ModuleId,
    ) -> CommonjsExportState {
        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();

        test.compiler.collect_commonjs_export_state(
            module_id,
            dir.namespace_scope,
            dir.namespace_symbol.into_global(module_id),
            &dir.roots,
            &tree,
            &symbols,
        )
    }

    /// Assert a named CommonJS export points to one unresolved or resolved path name.
    fn assert_named_export_targets_name(
        test: &TestProgram,
        module_id: destack_source::ModuleId,
        state: &CommonjsExportState,
        export_name: &str,
        expected_target_name: &str,
    ) {
        let export_name_id = test.program.strings.intern(export_name);
        let expected_name_id = test.program.strings.intern(expected_target_name);
        let Some(value) = state.named_values.get(&export_name_id) else {
            panic!("expected named export '{export_name}'");
        };

        match value {
            CommonjsExportValue::Expression(expression_id) => {
                let module = test.program.modules.get(module_id);
                let module = module.read();
                let profile = test.default_profile_id(module_id);
                let tree = module.dir(profile).tree.read();
                let expression = tree.get(*expression_id);

                let path = match expression {
                    destack_dir::Expression::UnresolvedPath { path, .. }
                    | destack_dir::Expression::ModuleReference { path, .. }
                    | destack_dir::Expression::GlobalReference { path, .. } => path,
                    _ => panic!("expected path expression target, found {expression:?}"),
                };
                assert_eq!(path.segments.len(), 1);
                assert_eq!(path.segments[0], expected_name_id);
            }
            CommonjsExportValue::Symbol(symbol_id) => {
                let module = test.program.modules.get(module_id);
                let module = module.read();
                let profile = test.default_profile_id(module_id);
                let symbols = module.dir(profile).symbols.read();
                let symbol = symbols.get_symbol(*symbol_id);
                let Some(symbol_name) = symbol.name() else {
                    panic!("expected symbol name for shorthand export");
                };
                assert_eq!(symbol_name, expected_name_id);
            }
        }
    }

    /// Collect default and named state from direct and property CommonJS writes.
    #[test]
    fn test_collect_commonjs_state_for_default_and_named_writes() {
        let test = TestProgram::memory_sequential_with_prelude();
        let module_id = test.add_module(
            "main.js",
            r#"
function selected() {
    return 1;
}

function helper() {
    return 2;
}

module.exports = selected;
module.exports.helper = helper;
"#,
        );

        // enqueue resolve for module
        test.resolve_module(module_id);

        // run queued tasks
        test.compile();

        // collect static CommonJS export state
        let state = collect_commonjs_state(&test, module_id);

        // assert final default and named state
        assert!(state.default_value.is_some());
        assert_named_export_targets_name(&test, module_id, &state, "helper", "helper");
    }

    /// Capture default value from chained assignments targeting module exports.
    #[test]
    fn test_collect_commonjs_state_for_chained_module_exports_assignment() {
        let test = TestProgram::memory_sequential_with_prelude();
        let module_id = test.add_module(
            "main.js",
            r#"
const holder = {};

function buildValue() {
    return 1;
}

holder.value = module.exports = buildValue;
"#,
        );

        // enqueue resolve for module
        test.resolve_module(module_id);

        // run queued tasks
        test.compile();

        // collect static CommonJS export state
        let state = collect_commonjs_state(&test, module_id);

        // assert chained assignment sets the default value
        assert!(state.default_value.is_some());
    }

    /// Replace named export state when module exports is replaced with an object literal.
    #[test]
    fn test_collect_commonjs_state_replaces_named_values_on_module_exports_assignment() {
        let test = TestProgram::memory_sequential_with_prelude();
        let module_id = test.add_module(
            "main.js",
            r#"
function first() {
    return 1;
}

function second() {
    return 2;
}

exports.first = first;
module.exports = { second };
"#,
        );

        // enqueue resolve for module
        test.resolve_module(module_id);

        // run queued tasks
        test.compile();

        // collect static CommonJS export state
        let state = collect_commonjs_state(&test, module_id);

        // assert replacement semantics
        let first_id = test.program.strings.intern("first");
        let second_id = test.program.strings.intern("second");
        assert!(!state.named_values.contains_key(&first_id));
        assert!(state.named_values.contains_key(&second_id));
        assert_named_export_targets_name(&test, module_id, &state, "second", "second");
    }

    /// Drop exports alias writes after module exports replacement.
    #[test]
    fn test_collect_commonjs_state_drops_exports_writes_after_replacement() {
        let test = TestProgram::memory_sequential_with_prelude();
        let module_id = test.add_module(
            "main.js",
            r#"
function selected() {
    return 1;
}

function leaked() {
    return 2;
}

module.exports = selected;
exports.leaked = leaked;
"#,
        );

        // enqueue resolve for module
        test.resolve_module(module_id);

        // run queued tasks
        test.compile();

        // collect static CommonJS export state
        let state = collect_commonjs_state(&test, module_id);

        // assert alias semantics
        let leaked_id = test.program.strings.intern("leaked");
        assert!(!state.named_values.contains_key(&leaked_id));
    }

    /// Keep exports property writes when relinked through explicit module exports assignment.
    #[test]
    fn test_collect_commonjs_state_relinks_exports_alias() {
        let test = TestProgram::memory_sequential_with_prelude();
        let module_id = test.add_module(
            "main.js",
            r#"
function selected() {
    return 1;
}

function helper() {
    return 2;
}

module.exports = selected;
exports = module.exports;
exports.helper = helper;
"#,
        );

        // enqueue resolve for module
        test.resolve_module(module_id);

        // run queued tasks
        test.compile();

        // collect static CommonJS export state
        let state = collect_commonjs_state(&test, module_id);

        // assert relinked alias behavior
        let helper_id = test.program.strings.intern("helper");
        assert!(state.named_values.contains_key(&helper_id));
        assert_named_export_targets_name(&test, module_id, &state, "helper", "helper");
    }

    /// Ignore exports writes after assigning exports to a plain value expression.
    #[test]
    fn test_collect_commonjs_state_does_not_relink_exports_alias_for_value_expression() {
        let test = TestProgram::memory_sequential_with_prelude();
        let module_id = test.add_module(
            "main.js",
            r#"
function selected() {
    return 1;
}

function helper() {
    return 2;
}

module.exports = selected;
exports = selected;
exports.helper = helper;
"#,
        );

        // enqueue resolve for module
        test.resolve_module(module_id);

        // run queued tasks
        test.compile();

        // collect static CommonJS export state
        let state = collect_commonjs_state(&test, module_id);

        // assert non-relinked alias behavior
        let helper_id = test.program.strings.intern("helper");
        assert!(!state.named_values.contains_key(&helper_id));
    }

    /// Collect named exports from static numeric bracket keys.
    #[test]
    fn test_collect_commonjs_state_collects_numeric_bracket_property_name() {
        let test = TestProgram::memory_sequential_with_prelude();
        let module_id = test.add_module(
            "main.js",
            r#"
function selected() {
    return 1;
}

module.exports[1] = selected;
"#,
        );

        // enqueue resolve for module
        test.resolve_module(module_id);

        // run queued tasks
        test.compile();

        // collect static CommonJS export state
        let state = collect_commonjs_state(&test, module_id);

        // assert numeric property key was normalized
        let key_id = test.program.strings.intern("1");
        assert!(state.named_values.contains_key(&key_id));
        assert_named_export_targets_name(&test, module_id, &state, "1", "selected");
    }
}
