use destack_dir::{
    Argument, Block, Declaration, Declarator, DependencyItem, DynamicKey, Expression, LocalNodeId,
    MatchCase, MatchSource, Member, NodeTree, Parameter, Pattern, PatternField, Property,
    StaticKey, SymbolKind, TemplateLiteral,
};
use destack_source::StringId;
use destack_workspace::Module;

use crate::{BindError, Compiler};

const RESERVED_IDENTIFIERS: &[&str] = &[
    "implements",
    "interface",
    "let",
    "package",
    "private",
    "protected",
    "public",
    "static",
    "yield",
    "await",
    "eval",
    "arguments",
];

/// Tracks control flow context for break/continue validation.
#[derive(Debug, Clone, Default)]
struct FlowContext {
    /// Whether we're in a loop (break and continue both valid).
    in_loop: bool,
    /// Whether we're in a switch (break valid, continue not).
    in_switch: bool,
    /// Labels in scope, with whether they wrap a loop.
    labels: Vec<(StringId, bool)>,
}

impl FlowContext {
    /// Enter a loop context.
    fn enter_loop(&self) -> Self {
        Self {
            in_loop: true,
            in_switch: self.in_switch,
            labels: self.labels.clone(),
        }
    }

    /// Enter a switch context.
    fn enter_switch(&self) -> Self {
        Self {
            in_loop: self.in_loop,
            in_switch: true,
            labels: self.labels.clone(),
        }
    }

    /// Add a label to the context.
    fn with_label(&self, name: StringId, is_loop: bool) -> Self {
        let mut labels = self.labels.clone();
        labels.push((name, is_loop));
        Self {
            in_loop: self.in_loop,
            in_switch: self.in_switch,
            labels,
        }
    }

    /// Reset the context.
    fn reset() -> Self {
        Self::default()
    }

    /// Check if we can break.
    fn can_break(&self) -> bool {
        self.in_loop || self.in_switch
    }

    /// Check if we can continue.
    fn can_continue(&self) -> bool {
        self.in_loop
    }

    /// Check if we have a label.
    fn has_label(&self, name: StringId) -> bool {
        self.labels.iter().any(|(n, _)| *n == name)
    }

    /// Check if a label is a loop.
    fn label_is_loop(&self, name: StringId) -> bool {
        self.labels
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, is_loop)| *is_loop)
            .unwrap_or(false)
    }
}

impl Compiler {
    /// Check if an identifier is reserved.
    fn is_reserved_identifier(&self, name: StringId) -> bool {
        let name_str = self.program.strings.get(name);
        RESERVED_IDENTIFIERS.contains(&name_str.as_ref())
    }

    /// Check for reserved identifiers used as binding names.
    pub(super) fn validate_binding_names(&self, module: &Module) {
        let symbols = module.dir.symbols.read();
        for scope in symbols.scopes() {
            for (key, symbol_id) in scope.named_symbols.iter() {
                let symbol = symbols.get_symbol(*symbol_id);
                let Some(primary_declaration) = symbol.primary_declaration else {
                    continue;
                };
                if let StaticKey::Name(name) = key
                    && self.is_reserved_identifier(*name)
                {
                    self.error(BindError::ReservedIdentifier {
                        node: primary_declaration,
                        name: *name,
                    });
                }
            }
        }
    }

    /// Check for conflicting bindings in module scopes.
    pub(super) fn validate_binding_conflicts(&self, module: &Module) {
        let no_redeclare_locals = self
            .program
            .with_dsconfig_options(module, |opts| opts.compiler.no_redeclared_locals)
            .unwrap_or(false);
        let symbols = module.dir.symbols.read();

        for scope in symbols.scopes() {
            for (key, symbol_id) in scope.named_symbols.iter() {
                let symbol = symbols.get_symbol(*symbol_id);
                let Some(primary_declaration) = symbol.primary_declaration else {
                    continue;
                };

                for (other_key, other_symbol_id) in scope.named_symbols.iter() {
                    if *other_key != *key || *other_symbol_id == *symbol_id {
                        continue;
                    }

                    let other_symbol = symbols.get_symbol(*other_symbol_id);

                    // local conflicts are allowed unless configured otherwise
                    if symbol.kind == SymbolKind::Local
                        && other_symbol.kind == SymbolKind::Local
                        && !no_redeclare_locals
                    {
                        continue;
                    }

                    let Some(other_primary_declaration) = other_symbol.primary_declaration else {
                        continue;
                    };

                    let error = if symbol.export.is_some() && other_symbol.export.is_some() {
                        BindError::ConflictingExport {
                            node: primary_declaration,
                            other_node: other_primary_declaration,
                            module: module.id,
                            name: Some(*key),
                        }
                    } else {
                        BindError::ConflictingBinding {
                            node: primary_declaration,
                            other_node: other_primary_declaration,
                            scope: symbol.scope.0.into_global(module.id),
                            name: Some(*key),
                        }
                    };
                    self.error(error);
                }
            }
        }
    }

    /// Check for break/continue used outside valid contexts.
    /// 
    /// NOTE #Architecture: walking the DIR flow in bind validate is not ideal
    ///  (Unfortunately, it's non-trivial to repurpose NodeVisitor for this due to context-dependent logic.)
    pub(super) fn validate_flow(&self, module: &Module) {
        let tree = module.dir.tree.read();
        let context = FlowContext::default();
        for root in &module.dir.roots {
            self.validate_expression(module, &tree, *root, &context);
        }
    }

    /// Validate an expression in the context of binding flow.
    fn validate_expression(
        &self,
        module: &Module,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        context: &FlowContext,
    ) {
        let expression = tree.get(expression_id);
        match expression {
            Expression::Declaration { declaration } => {
                self.validate_declaration(module, tree, *declaration, context);
            }

            Expression::Block { block } => {
                self.validate_block(module, tree, *block, context);
            }

            Expression::Statement { statement } => {
                self.validate_expression(module, tree, *statement, context);
            }

            Expression::Labelled {
                label,
                body,
                symbol: _,
            } => {
                let body_expression = tree.get(*body);
                let is_loop = matches!(
                    body_expression,
                    Expression::Loop { .. } | Expression::For { .. } | Expression::ForEach { .. }
                );
                let label_context = context.with_label(*label, is_loop);
                self.validate_expression(module, tree, *body, &label_context);
            }

            Expression::UnresolvedImport {
                kind: _,
                target: _,
                items,
                arguments,
            } => {
                for item_id in items {
                    self.validate_dependency_item(module, tree, *item_id, context);
                }
                if let Some(arguments) = arguments {
                    for argument_id in arguments {
                        self.validate_argument(module, tree, *argument_id, context);
                    }
                }
            }

            Expression::UnresolvedReExport {
                target: _,
                kind: _,
                items,
            } => {
                for item_id in items {
                    self.validate_dependency_item(module, tree, *item_id, context);
                }
            }

            Expression::Import {
                kind: _,
                target: _,
                target_module: _,
                items,
                arguments,
            } => {
                for item_id in items {
                    self.validate_dependency_item(module, tree, *item_id, context);
                }
                if let Some(arguments) = arguments {
                    for argument_id in arguments {
                        self.validate_argument(module, tree, *argument_id, context);
                    }
                }
            }

            Expression::ReExport {
                target: _,
                target_module: _,
                kind: _,
                items,
            } => {
                for item_id in items {
                    self.validate_dependency_item(module, tree, *item_id, context);
                }
            }

            Expression::Export { kind: _, items } => {
                for item_id in items {
                    self.validate_dependency_item(module, tree, *item_id, context);
                }
            }

            Expression::Let {
                descriptor: _,
                mutability: _,
                declarators,
            } => {
                for declarator_id in declarators {
                    self.validate_declarator(module, tree, *declarator_id, context);
                }
            }

            Expression::TypeUnary { operator: _, right } => {
                self.validate_expression(module, tree, *right, context);
            }

            Expression::TypeBinary {
                left,
                operator: _,
                right,
            } => {
                self.validate_expression(module, tree, *left, context);
                self.validate_expression(module, tree, *right, context);
            }

            Expression::Unary { operator: _, right } => {
                self.validate_expression(module, tree, *right, context);
            }

            Expression::ValueOf {
                mutability: _,
                variance: _,
                right,
            } => {
                self.validate_expression(module, tree, *right, context);
            }

            Expression::ReferenceOf {
                mutability: _,
                variance: _,
                right,
            } => {
                self.validate_expression(module, tree, *right, context);
            }

            Expression::Binary {
                left,
                operator: _,
                right,
            } => {
                self.validate_expression(module, tree, *left, context);
                self.validate_expression(module, tree, *right, context);
            }

            Expression::Assign { left, right } => {
                self.validate_expression(module, tree, *left, context);
                self.validate_expression(module, tree, *right, context);
            }

            Expression::AssignBinary {
                left,
                operator: _,
                right,
            } => {
                self.validate_expression(module, tree, *left, context);
                self.validate_expression(module, tree, *right, context);
            }

            Expression::Member {
                left,
                name: _,
                static_arguments,
            } => {
                self.validate_expression(module, tree, *left, context);
                if let Some(static_arguments) = static_arguments {
                    for argument_id in static_arguments {
                        self.validate_argument(module, tree, *argument_id, context);
                    }
                }
            }

            Expression::Call {
                left,
                static_arguments,
                dynamic_arguments,
            } => {
                self.validate_expression(module, tree, *left, context);
                if let Some(static_arguments) = static_arguments {
                    for argument_id in static_arguments {
                        self.validate_argument(module, tree, *argument_id, context);
                    }
                }
                for argument_id in dynamic_arguments {
                    self.validate_argument(module, tree, *argument_id, context);
                }
            }

            Expression::Index { left, right } => {
                self.validate_expression(module, tree, *left, context);
                if let Some(right) = right {
                    self.validate_expression(module, tree, *right, context);
                }
            }

            Expression::Maybe { left } => {
                self.validate_expression(module, tree, *left, context);
            }

            Expression::Must { left } => {
                self.validate_expression(module, tree, *left, context);
            }

            Expression::New {
                left,
                static_arguments,
                dynamic_arguments,
            } => {
                self.validate_expression(module, tree, *left, context);
                if let Some(static_arguments) = static_arguments {
                    for argument_id in static_arguments {
                        self.validate_argument(module, tree, *argument_id, context);
                    }
                }
                for argument_id in dynamic_arguments {
                    self.validate_argument(module, tree, *argument_id, context);
                }
            }

            Expression::Delete { value } => {
                self.validate_expression(module, tree, *value, context);
            }

            Expression::UnresolvedPath {
                path: _,
                static_arguments,
            } => {
                if let Some(static_arguments) = static_arguments {
                    for argument_id in static_arguments {
                        self.validate_argument(module, tree, *argument_id, context);
                    }
                }
            }

            Expression::LocalReference {
                path: _,
                static_arguments,
                target_symbol: _,
            } => {
                if let Some(static_arguments) = static_arguments {
                    for argument_id in static_arguments {
                        self.validate_argument(module, tree, *argument_id, context);
                    }
                }
            }

            Expression::ModuleReference {
                path: _,
                static_arguments,
                target_symbol: _,
            } => {
                if let Some(static_arguments) = static_arguments {
                    for argument_id in static_arguments {
                        self.validate_argument(module, tree, *argument_id, context);
                    }
                }
            }

            Expression::GlobalReference {
                path: _,
                static_arguments,
                target_symbol: _,
            } => {
                if let Some(static_arguments) = static_arguments {
                    for argument_id in static_arguments {
                        self.validate_argument(module, tree, *argument_id, context);
                    }
                }
            }

            Expression::ScalarLiteral { value: _ } => {}

            Expression::TypeLiteral { value: _ } => {}

            Expression::Type { value: _ } => {}

            Expression::TemplateExpression { value } => {
                if let TemplateLiteral::InterpolatedString {
                    strings: _,
                    arguments,
                } = value
                {
                    for argument_id in arguments {
                        self.validate_argument(module, tree, *argument_id, context);
                    }
                }
            }

            Expression::TaggedTemplateExpression { tag, value } => {
                self.validate_expression(module, tree, *tag, context);
                if let TemplateLiteral::InterpolatedString {
                    strings: _,
                    arguments,
                } = value
                {
                    for argument_id in arguments {
                        self.validate_argument(module, tree, *argument_id, context);
                    }
                }
            }

            Expression::RangeExpression {
                start,
                end,
                is_inclusive: _,
            } => {
                self.validate_expression(module, tree, *start, context);
                self.validate_expression(module, tree, *end, context);
            }

            Expression::ArrayExpression { elements } => {
                for element_id in elements {
                    self.validate_argument(module, tree, *element_id, context);
                }
            }

            Expression::TupleExpression { elements } => {
                for element_id in elements {
                    self.validate_argument(module, tree, *element_id, context);
                }
            }

            Expression::SequenceExpression { expressions } => {
                for child in expressions {
                    self.validate_expression(module, tree, *child, context);
                }
            }

            Expression::ObjectExpression { properties } => {
                for property_id in properties {
                    self.validate_property(module, tree, *property_id, context);
                }
            }

            Expression::TreeExpression {
                left,
                arguments,
                elements,
            } => {
                if let Some(left) = left {
                    self.validate_expression(module, tree, *left, context);
                }
                if let Some(arguments) = arguments {
                    for argument_id in arguments {
                        self.validate_argument(module, tree, *argument_id, context);
                    }
                }
                if let Some(elements) = elements {
                    for element_id in elements {
                        self.validate_argument(module, tree, *element_id, context);
                    }
                }
            }

            Expression::TaggedScalarExpression { ty, value } => {
                self.validate_expression(module, tree, *ty, context);
                self.validate_expression(module, tree, *value, context);
            }

            Expression::TaggedTupleExpression { ty, elements } => {
                self.validate_expression(module, tree, *ty, context);
                for argument_id in elements {
                    self.validate_argument(module, tree, *argument_id, context);
                }
            }

            Expression::TaggedObjectExpression { ty, properties } => {
                self.validate_expression(module, tree, *ty, context);
                for property_id in properties {
                    self.validate_property(module, tree, *property_id, context);
                }
            }

            Expression::Parenthesized { expression } => {
                self.validate_expression(module, tree, *expression, context);
            }

            Expression::If {
                kind: _,
                condition,
                then_expression,
                else_expression,
            } => {
                self.validate_expression(module, tree, *condition, context);
                self.validate_expression(module, tree, *then_expression, context);
                if let Some(else_expression) = else_expression {
                    self.validate_expression(module, tree, *else_expression, context);
                }
            }

            Expression::Loop {
                kind: _,
                condition,
                body,
                scope: _,
                symbol: _,
            } => {
                if let Some(condition) = condition {
                    self.validate_expression(module, tree, *condition, context);
                }
                let loop_context = context.enter_loop();
                self.validate_block(module, tree, *body, &loop_context);
            }

            Expression::ForEach {
                asynchrony: _,
                kind: _,
                pattern,
                iterator,
                body,
                scope: _,
                symbol: _,
            } => {
                self.validate_pattern(module, tree, *pattern, context);
                self.validate_expression(module, tree, *iterator, context);
                let loop_context = context.enter_loop();
                self.validate_block(module, tree, *body, &loop_context);
            }

            Expression::For {
                initialization,
                condition,
                increment,
                body,
                scope: _,
                symbol: _,
            } => {
                if let Some(initialization) = initialization {
                    self.validate_expression(module, tree, *initialization, context);
                }
                if let Some(condition) = condition {
                    self.validate_expression(module, tree, *condition, context);
                }
                if let Some(increment) = increment {
                    self.validate_expression(module, tree, *increment, context);
                }
                let loop_context = context.enter_loop();
                self.validate_block(module, tree, *body, &loop_context);
            }

            Expression::Try {
                try_expression,
                catch_pattern,
                catch_expression,
                finally_expression,
                scope: _,
                symbol: _,
            } => {
                self.validate_expression(module, tree, *try_expression, context);
                if let Some(catch_pattern) = catch_pattern {
                    self.validate_pattern(module, tree, *catch_pattern, context);
                }
                if let Some(catch_expression) = catch_expression {
                    self.validate_expression(module, tree, *catch_expression, context);
                }
                if let Some(finally_expression) = finally_expression {
                    self.validate_expression(module, tree, *finally_expression, context);
                }
            }

            Expression::Match {
                value,
                cases,
                source,
                scope: _,
                symbol: _,
            } if *source == MatchSource::Match => {
                self.validate_expression(module, tree, *value, context);
                let switch_context = context.enter_switch();
                for case_id in cases {
                    self.validate_match_case(module, tree, *case_id, &switch_context);
                }
            }

            Expression::Match {
                value,
                cases,
                source: _,
                scope: _,
                symbol: _,
            } => {
                self.validate_expression(module, tree, *value, context);
                for case_id in cases {
                    self.validate_match_case(module, tree, *case_id, context);
                }
            }

            Expression::UnresolvedBreak { target, value } => {
                if let Some(label) = target {
                    if !context.has_label(*label) {
                        self.error(BindError::UnknownLabel {
                            node: expression_id.into_global(module.id).into(),
                            label: *label,
                        });
                    }
                } else if !context.can_break() {
                    self.error(BindError::IllegalBreak {
                        node: expression_id.into_global(module.id).into(),
                        label: None,
                    });
                }
                if let Some(value) = value {
                    self.validate_expression(module, tree, *value, context);
                }
            }

            Expression::Break { target: _, value } => {
                if let Some(value) = value {
                    self.validate_expression(module, tree, *value, context);
                }
            }

            Expression::UnresolvedContinue { target } => {
                if let Some(label) = target {
                    if !context.has_label(*label) {
                        self.error(BindError::UnknownLabel {
                            node: expression_id.into_global(module.id).into(),
                            label: *label,
                        });
                    } else if !context.label_is_loop(*label) {
                        self.error(BindError::IllegalContinue {
                            node: expression_id.into_global(module.id).into(),
                            label: Some(*label),
                        });
                    }
                } else if !context.can_continue() {
                    self.error(BindError::IllegalContinue {
                        node: expression_id.into_global(module.id).into(),
                        label: None,
                    });
                }
            }

            Expression::Continue { target: _ } => {}

            Expression::Throw { value } => {
                if let Some(value) = value {
                    self.validate_expression(module, tree, *value, context);
                }
            }

            Expression::Await { expression } => {
                self.validate_expression(module, tree, *expression, context);
            }

            Expression::Yield {
                cardinality: _,
                value,
            } => {
                if let Some(value) = value {
                    self.validate_expression(module, tree, *value, context);
                }
            }

            Expression::Return { value } => {
                if let Some(value) = value {
                    self.validate_expression(module, tree, *value, context);
                }
            }

            Expression::Stub => {}

            Expression::Error => {}
        }
    }

    /// Validate a block in the context of binding flow.
    fn validate_block(
        &self,
        module: &Module,
        tree: &NodeTree,
        block_id: LocalNodeId<Block>,
        context: &FlowContext,
    ) {
        let block = tree.get(block_id);
        for child in &block.expressions {
            self.validate_expression(module, tree, *child, context);
        }
    }

    /// Validate a match case in the context of binding flow.
    fn validate_match_case(
        &self,
        module: &Module,
        tree: &NodeTree,
        case_id: LocalNodeId<MatchCase>,
        context: &FlowContext,
    ) {
        let case = tree.get(case_id);
        match case {
            MatchCase::Expression {
                pattern,
                body,
                guard,
                scope: _,
            } => {
                self.validate_pattern(module, tree, *pattern, context);
                self.validate_expression(module, tree, *body, context);
                if let Some(guard) = guard {
                    self.validate_expression(module, tree, *guard, context);
                }
            }
            MatchCase::Block {
                pattern,
                body,
                guard,
                scope: _,
            } => {
                self.validate_pattern(module, tree, *pattern, context);
                self.validate_block(module, tree, *body, context);
                if let Some(guard) = guard {
                    self.validate_expression(module, tree, *guard, context);
                }
            }
        }
    }

    /// Validate a declaration in the context of binding flow.
    fn validate_declaration(
        &self,
        module: &Module,
        tree: &NodeTree,
        declaration_id: LocalNodeId<Declaration>,
        context: &FlowContext,
    ) {
        let declaration = tree.get(declaration_id);
        match declaration {
            Declaration::Namespace {
                descriptor: _,
                generics: _,
                scope: _,
                expressions,
            } => {
                for expression_id in expressions {
                    self.validate_expression(module, tree, *expression_id, context);
                }
            }
            Declaration::Type {
                descriptor: _,
                kind: _,
                mutability: _,
                static_parameters: _,
                value,
            } => {
                self.validate_expression(module, tree, *value, context);
            }
            Declaration::Struct {
                descriptor: _,
                generics: _,
                heritage: _,
                scope: _,
                members,
            } => {
                for member_id in members {
                    self.validate_member(module, tree, *member_id, context);
                }
            }
            Declaration::Class {
                descriptor: _,
                generics: _,
                heritage: _,
                scope: _,
                members,
            } => {
                for member_id in members {
                    self.validate_member(module, tree, *member_id, context);
                }
            }
            Declaration::Enum {
                descriptor: _,
                generics: _,
                heritage: _,
                scope: _,
                fields: _,
                members,
            } => {
                for member_id in members {
                    self.validate_member(module, tree, *member_id, context);
                }
            }
            Declaration::Interface {
                descriptor: _,
                generics: _,
                heritage: _,
                scope: _,
                members,
            } => {
                for member_id in members {
                    self.validate_member(module, tree, *member_id, context);
                }
            }
            Declaration::Function {
                descriptor: _,
                signature,
                scope: _,
                body: Some(body),
            } => {
                for parameter_id in &signature.dynamic_parameters {
                    self.validate_parameter(module, tree, *parameter_id, context);
                }
                let reset_context = FlowContext::reset();
                self.validate_expression(module, tree, *body, &reset_context);
            }
            Declaration::Function {
                descriptor: _,
                signature,
                scope: _,
                body: None,
            } => {
                for parameter_id in &signature.dynamic_parameters {
                    self.validate_parameter(module, tree, *parameter_id, context);
                }
            }
            Declaration::Extension {
                descriptor: _,
                generics: _,
                target_type: _,
                target_symbol: _,
                heritage: _,
                scope: _,
                members,
            } => {
                for member_id in members {
                    self.validate_member(module, tree, *member_id, context);
                }
            }
        }
    }

    /// Validate a member in the context of binding flow.
    fn validate_member(
        &self,
        module: &Module,
        tree: &NodeTree,
        member_id: LocalNodeId<Member>,
        context: &FlowContext,
    ) {
        let member = tree.get(member_id);
        match member {
            Member::Field {
                modifiers: _,
                key,
                value,
                default,
                symbol: _,
            } => {
                if let Some(key) = key {
                    self.validate_key(module, tree, key, context);
                }
                if let Some(value) = value {
                    self.validate_expression(module, tree, *value, context);
                }
                if let Some(default) = default {
                    self.validate_expression(module, tree, *default, context);
                }
            }
            Member::Method {
                modifiers: _,
                key,
                signature,
                body,
                symbol: _,
            } => {
                if let Some(key) = key {
                    self.validate_key(module, tree, key, context);
                }
                for parameter_id in &signature.dynamic_parameters {
                    self.validate_parameter(module, tree, *parameter_id, context);
                }
                if let Some(body) = body {
                    let reset_context = FlowContext::reset();
                    self.validate_expression(module, tree, *body, &reset_context);
                }
            }
            Member::Embed {
                modifiers: _,
                value,
                symbol: _,
            } => {
                self.validate_expression(module, tree, *value, context);
            }
            Member::StaticBlock { body, symbol: _ } => {
                self.validate_expression(module, tree, *body, context);
            }
        }
    }

    /// Validate a property in the context of binding flow.
    fn validate_property(
        &self,
        module: &Module,
        tree: &NodeTree,
        property_id: LocalNodeId<Property>,
        context: &FlowContext,
    ) {
        let property = tree.get(property_id);
        match property {
            Property::Field {
                modifiers: _,
                key,
                value,
                default,
                symbol: _,
            } => {
                if let Some(key) = key {
                    self.validate_key(module, tree, key, context);
                }
                if let Some(value) = value {
                    self.validate_expression(module, tree, *value, context);
                }
                if let Some(default) = default {
                    self.validate_expression(module, tree, *default, context);
                }
            }
            Property::Method {
                modifiers: _,
                key,
                signature,
                body,
                symbol: _,
            } => {
                if let Some(key) = key {
                    self.validate_key(module, tree, key, context);
                }
                for parameter_id in &signature.dynamic_parameters {
                    self.validate_parameter(module, tree, *parameter_id, context);
                }
                if let Some(body) = body {
                    let reset_context = FlowContext::reset();
                    self.validate_expression(module, tree, *body, &reset_context);
                }
            }
            Property::Spread {
                modifiers: _,
                value,
                symbol: _,
            } => {
                self.validate_expression(module, tree, *value, context);
            }
        }
    }

    /// Validate a key in the context of binding flow.
    fn validate_key(
        &self,
        module: &Module,
        tree: &NodeTree,
        key: &DynamicKey,
        context: &FlowContext,
    ) {
        match key {
            DynamicKey::Name(_name) => {}
            DynamicKey::Expression(expression) => {
                self.validate_expression(module, tree, *expression, context);
            }
            DynamicKey::NamedExpression { name: _, key } => {
                self.validate_expression(module, tree, *key, context);
            }
        }
    }

    /// Validate an argument in the context of binding flow.
    fn validate_argument(
        &self,
        module: &Module,
        tree: &NodeTree,
        argument_id: LocalNodeId<Argument>,
        context: &FlowContext,
    ) {
        let argument = tree.get(argument_id);
        match argument {
            Argument::Named { name: _, value } => {
                self.validate_expression(module, tree, *value, context);
            }
            Argument::Labeled { label: _, value } => {
                self.validate_expression(module, tree, *value, context);
            }
            Argument::Positional { value } => {
                self.validate_expression(module, tree, *value, context);
            }
            Argument::Spread { value } => {
                self.validate_expression(module, tree, *value, context);
            }
            Argument::Dynamic { key, value } => {
                self.validate_expression(module, tree, *key, context);
                self.validate_expression(module, tree, *value, context);
            }
        }
    }

    /// Validate a declarator in the context of binding flow.
    fn validate_declarator(
        &self,
        module: &Module,
        tree: &NodeTree,
        declarator_id: LocalNodeId<Declarator>,
        context: &FlowContext,
    ) {
        let declarator = tree.get(declarator_id);
        self.validate_pattern(module, tree, declarator.pattern, context);
        if let Some(ty) = declarator.ty {
            self.validate_expression(module, tree, ty, context);
        }
        if let Some(value) = declarator.value {
            self.validate_expression(module, tree, value, context);
        }
    }

    /// Validate a parameter in the context of binding flow.
    fn validate_parameter(
        &self,
        module: &Module,
        tree: &NodeTree,
        parameter_id: LocalNodeId<Parameter>,
        context: &FlowContext,
    ) {
        let parameter = tree.get(parameter_id);
        match parameter {
            Parameter::Named {
                modifiers: _,
                name: _,
                default,
                symbol: _,
            } => {
                if let Some(default) = default {
                    self.validate_expression(module, tree, *default, context);
                }
            }
            Parameter::Pattern {
                modifiers: _,
                pattern,
                default,
                symbol: _,
            } => {
                self.validate_pattern(module, tree, *pattern, context);
                if let Some(default) = default {
                    self.validate_expression(module, tree, *default, context);
                }
            }
            Parameter::Variadic {
                modifiers: _,
                name: _,
                symbol: _,
            } => {}
        }
    }

    /// Validate a pattern in the context of binding flow.
    fn validate_pattern(
        &self,
        module: &Module,
        tree: &NodeTree,
        pattern_id: LocalNodeId<Pattern>,
        context: &FlowContext,
    ) {
        let pattern = tree.get(pattern_id);
        match pattern {
            Pattern::Wildcard => {}
            Pattern::Maybe(inner) => {
                self.validate_pattern(module, tree, *inner, context);
            }
            Pattern::ReferenceOf {
                mutability: _,
                right,
            } => {
                self.validate_pattern(module, tree, *right, context);
            }
            Pattern::ValueOf {
                mutability: _,
                right,
            } => {
                self.validate_pattern(module, tree, *right, context);
            }
            Pattern::Binding {
                mutability: _,
                name: _,
                pattern,
                symbol: _,
            } => {
                if let Some(pattern) = pattern {
                    self.validate_pattern(module, tree, *pattern, context);
                }
            }
            Pattern::Expression { value } => {
                self.validate_expression(module, tree, *value, context);
            }
            Pattern::Range {
                start,
                end,
                is_inclusive: _,
            } => {
                if let Some(start) = start {
                    self.validate_pattern(module, tree, *start, context);
                }
                if let Some(end) = end {
                    self.validate_pattern(module, tree, *end, context);
                }
            }
            Pattern::Tuple { fields } => {
                for field_id in fields {
                    self.validate_pattern_field(module, tree, *field_id, context);
                }
            }
            Pattern::TaggedTuple { ty, fields } => {
                self.validate_expression(module, tree, *ty, context);
                for field_id in fields {
                    self.validate_pattern_field(module, tree, *field_id, context);
                }
            }
            Pattern::Array { fields } => {
                for field_id in fields {
                    self.validate_pattern_field(module, tree, *field_id, context);
                }
            }
            Pattern::Object { fields } => {
                for field_id in fields {
                    self.validate_pattern_field(module, tree, *field_id, context);
                }
            }
            Pattern::TaggedObject { ty, fields } => {
                self.validate_expression(module, tree, *ty, context);
                for field_id in fields {
                    self.validate_pattern_field(module, tree, *field_id, context);
                }
            }
            Pattern::Union { patterns } => {
                for pattern in patterns {
                    self.validate_pattern(module, tree, *pattern, context);
                }
            }
        }
    }

    /// Validate a pattern field in the context of binding flow.
    fn validate_pattern_field(
        &self,
        module: &Module,
        tree: &NodeTree,
        field_id: LocalNodeId<PatternField>,
        context: &FlowContext,
    ) {
        let field = tree.get(field_id);
        match field {
            PatternField::Named {
                mutability: _,
                name: _,
                pattern,
                default,
                symbol: _,
            } => {
                if let Some(pattern) = pattern {
                    self.validate_pattern(module, tree, *pattern, context);
                }
                if let Some(default) = default {
                    self.validate_expression(module, tree, *default, context);
                }
            }
            PatternField::Alias {
                mutability: _,
                name: _,
                alias: _,
                default,
                symbol: _,
            } => {
                if let Some(default) = default {
                    self.validate_expression(module, tree, *default, context);
                }
            }
            PatternField::Positional { pattern } => {
                self.validate_pattern(module, tree, *pattern, context);
            }
            PatternField::Spread {
                mutability: _,
                name: _,
                symbol: _,
            } => {}
            PatternField::Elision => {}
        }
    }

    /// Validate a dependency item in the context of binding flow.
    fn validate_dependency_item(
        &self,
        module: &Module,
        tree: &NodeTree,
        item_id: LocalNodeId<DependencyItem>,
        context: &FlowContext,
    ) {
        let item = tree.get(item_id);
        match item {
            DependencyItem::UnresolvedRemote {
                source: _,
                mode: _,
                kind: _,
                name: _,
                alias: _,
                target: _,
                target_module: _,
                symbol: _,
            } => {}
            DependencyItem::UnresolvedLocal {
                mode: _,
                kind: _,
                name: _,
                alias: _,
                symbol: _,
            } => {}
            DependencyItem::Value { value } => {
                self.validate_expression(module, tree, *value, context);
            }
            DependencyItem::Local {
                mode: _,
                kind: _,
                name: _,
                alias: _,
                symbol: _,
                target_symbol: _,
            } => {}
            DependencyItem::Remote {
                mode: _,
                kind: _,
                name: _,
                alias: _,
                target: _,
                target_module: _,
                symbol: _,
                target_symbol: _,
            } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::TestProgram;

    #[test]
    fn test_redeclare_locals_allowed_by_default() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
let x = 1;
let x = 2;
"#,
        );
        test.bind_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EB004");
    }

    #[test]
    fn test_redeclare_locals_forbidden_by_dsconfig() {
        let test = TestProgram::memory_sequential();
        test.add_package("test", Some(r#""noRedeclaredLocals": true"#));
        let module_id = test.add_module(
            "test.ds",
            r#"
let x = 1;
let x = 2;
"#,
        );
        test.bind_module(module_id);
        test.compile();
        test.check_has_diagnostic("EB004");
    }

    #[test]
    fn test_reserved_identifier_yield_as_variable() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.js", "let yield = 1;");
        test.bind_module(module_id);
        test.compile();
        test.check_has_diagnostic("EB012");
    }

    #[test]
    fn test_reserved_identifier_implements_as_variable() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.js", "let implements = 1;");
        test.bind_module(module_id);
        test.compile();
        test.check_has_diagnostic("EB012");
    }

    #[test]
    fn test_reserved_identifier_static_as_variable() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.js", "let static = 1;");
        test.bind_module(module_id);
        test.compile();
        test.check_has_diagnostic("EB012");
    }

    #[test]
    fn test_reserved_identifier_await_as_variable() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.js", "let await = 1;");
        test.bind_module(module_id);
        test.compile();
        test.check_has_diagnostic("EB012");
    }

    #[test]
    fn test_reserved_identifier_eval_as_parameter() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.js", "function foo(eval) {}");
        test.bind_module(module_id);
        test.compile();
        test.check_has_diagnostic("EB012");
    }

    #[test]
    fn test_reserved_identifier_arguments_as_parameter() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.js", "function foo(arguments) {}");
        test.bind_module(module_id);
        test.compile();
        test.check_has_diagnostic("EB012");
    }

    #[test]
    fn test_normal_identifier_is_allowed() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.js", "let foo = 1;");
        test.bind_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EB012");
    }

    #[test]
    fn test_break_in_loop_allowed() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.js", "while (true) { break; }");
        test.bind_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EB007");
    }

    #[test]
    fn test_break_outside_loop_error() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.js", "break;");
        test.bind_module(module_id);
        test.compile();
        test.check_has_diagnostic("EB007");
    }

    #[test]
    fn test_continue_in_loop_allowed() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.js", "while (true) { continue; }");
        test.bind_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EB008");
    }

    #[test]
    fn test_continue_outside_loop_error() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.js", "continue;");
        test.bind_module(module_id);
        test.compile();
        test.check_has_diagnostic("EB008");
    }

    #[test]
    fn test_break_in_switch_allowed() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.js", "switch (x) { case 1: break; }");
        test.bind_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EB007");
    }

    #[test]
    fn test_break_with_label_allowed() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.js", "foo: while (true) { break foo; }");
        test.bind_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EB007");
        test.check_no_diagnostic_code("EB009");
    }

    #[test]
    fn test_break_with_unknown_label_error() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.js", "while (true) { break foo; }");
        test.bind_module(module_id);
        test.compile();
        test.check_has_diagnostic("EB009");
    }

    #[test]
    fn test_continue_with_label_to_loop_allowed() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.js", "foo: while (true) { continue foo; }");
        test.bind_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EB008");
        test.check_no_diagnostic_code("EB009");
    }

    #[test]
    fn test_break_in_function_inside_loop_error() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.js", "while (true) { function f() { break; } }");
        test.bind_module(module_id);
        test.compile();
        test.check_has_diagnostic("EB007");
    }
}
