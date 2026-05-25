use std::collections::HashMap;

use destack_core::StringPool;
use destack_dir as dir;
use destack_source::ModuleId;
use destack_workspace::{ArtifactCache, ProfileId};

use crate::rules::common::glob_matches;

use super::{
    SymbolDecorator, expression_candidate_symbols, expression_unwrap_parenthesized,
    symbol_decorators_for, symbol_initializer_expression as resolve_symbol_initializer_expression,
};

/// Label set for taint tracking.
///
/// `matches_all` represents unlabeled taint and can match any sink label.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaintLabels {
    /// Whether this taint set contains an unlabeled marker.
    matches_all: bool,
    /// The explicit taint labels.
    labels: Vec<String>,
}

impl TaintLabels {
    /// Return true when this set has no labels.
    pub fn is_empty(&self) -> bool {
        !self.matches_all && self.labels.is_empty()
    }

    /// Return true when this set has at least one label.
    pub fn is_tainted(&self) -> bool {
        !self.is_empty()
    }

    /// Add an unlabeled marker.
    pub fn add_unlabeled(&mut self) {
        self.matches_all = true;
    }

    /// Add one explicit label.
    pub fn add_label(&mut self, label: String) {
        if self.labels.contains(&label) {
            return;
        }
        self.labels.push(label);
    }

    /// Merge another set into this one.
    pub fn merge(&mut self, other: &Self) {
        if other.matches_all {
            self.matches_all = true;
        }

        for label in &other.labels {
            self.add_label(label.clone());
        }
    }

    /// Return true when this source set can flow into the sink set.
    pub fn matches_sink(&self, sink: &Self) -> bool {
        if self.is_empty() || sink.is_empty() {
            return false;
        }

        if self.matches_all || sink.matches_all {
            return true;
        }

        for source_label in &self.labels {
            if sink
                .labels
                .iter()
                .any(|sink_label| glob_matches(sink_label, source_label))
            {
                return true;
            }
        }

        false
    }

    /// Remove labels covered by a sanitizer set.
    pub fn apply_sanitizer(&mut self, sanitizer: &Self) {
        if sanitizer.is_empty() {
            return;
        }

        if sanitizer.matches_all {
            *self = Self::default();
            return;
        }

        self.labels.retain(|source_label| {
            !sanitizer
                .labels
                .iter()
                .any(|sanitizer_label| glob_matches(sanitizer_label, source_label))
        });
    }
}

/// Cached taint state for one module analysis run.
#[derive(Debug, Default)]
pub struct TaintCache {
    /// Cached labels for expressions.
    expression_labels: HashMap<u32, TaintLabels>,
    /// Cached labels for symbols.
    symbol_labels: HashMap<dir::GlobalSymbolId, TaintLabels>,
}

/// One taint analysis session over a DIR module.
#[derive(Debug)]
pub struct TaintAnalysis<'a> {
    /// Cached artifact reader for this revision.
    artifacts: &'a ArtifactCache,
    /// Active profile id.
    profile_id: ProfileId,
    /// Active module id.
    module_id: ModuleId,
    /// Active module tree.
    tree: &'a dir::Tree,
    /// Active module strings.
    strings: &'a StringPool,
    /// Active module symbols.
    symbols: &'a dir::BindingTable<'a>,
    /// Active module types.
    types: &'a dir::TypeTable<'a>,
    /// Active module resolutions.
    resolutions: &'a dir::ResolutionTable<'a>,
    /// Mutable cache reused across checks.
    cache: &'a mut TaintCache,
    /// Whether heuristic taint sources should be included.
    include_heuristic_sources: bool,
}

impl<'a> TaintAnalysis<'a> {
    /// Build a taint analysis session.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        artifacts: &'a ArtifactCache,
        profile_id: ProfileId,
        module_id: ModuleId,
        tree: &'a dir::Tree,
        strings: &'a StringPool,
        symbols: &'a dir::BindingTable<'a>,
        types: &'a dir::TypeTable<'a>,
        resolutions: &'a dir::ResolutionTable<'a>,
        cache: &'a mut TaintCache,
        include_heuristic_sources: bool,
    ) -> Self {
        Self {
            artifacts,
            profile_id,
            module_id,
            tree,
            strings,
            symbols,
            types,
            resolutions,
            cache,
            include_heuristic_sources,
        }
    }

    /// Return taint labels for one expression.
    pub fn expression_taint_labels(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> TaintLabels {
        let mut expression_stack = Vec::new();
        let mut symbol_stack = Vec::new();
        self.expression_taint_labels_inner(expression_id, &mut expression_stack, &mut symbol_stack)
    }

    /// Return true when an expression is tainted.
    pub fn expression_is_tainted(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        self.expression_taint_labels(expression_id).is_tainted()
    }

    /// Return taint labels for one assignment pattern.
    fn assign_pattern_taint_labels_inner(
        &mut self,
        assign_pattern_id: dir::LocalNodeId<dir::AssignPattern>,
        expression_stack: &mut Vec<dir::LocalNodeId<dir::Expression>>,
        symbol_stack: &mut Vec<dir::GlobalSymbolId>,
    ) -> TaintLabels {
        let assign_pattern = self.tree.get(assign_pattern_id);
        let mut labels = TaintLabels::default();

        match assign_pattern {
            // expression targets can read receiver state
            dir::AssignPattern::Expression { value } => {
                let value_labels =
                    self.expression_taint_labels_inner(*value, expression_stack, symbol_stack);
                labels.merge(&value_labels);
            }

            // defaulted targets read both the base target and fallback value
            dir::AssignPattern::Assign { pattern, value } => {
                let pattern_labels = self.assign_pattern_taint_labels_inner(
                    *pattern,
                    expression_stack,
                    symbol_stack,
                );
                labels.merge(&pattern_labels);

                let value_labels =
                    self.expression_taint_labels_inner(*value, expression_stack, symbol_stack);
                labels.merge(&value_labels);
            }

            // destructuring targets read from every nested key and target
            dir::AssignPattern::Sequence { fields } | dir::AssignPattern::Object { fields } => {
                for field_id in fields {
                    let field_labels = self.assign_pattern_field_taint_labels_inner(
                        *field_id,
                        expression_stack,
                        symbol_stack,
                    );
                    labels.merge(&field_labels);
                }
            }
        }

        labels
    }

    /// Return taint labels for one assignment pattern field.
    fn assign_pattern_field_taint_labels_inner(
        &mut self,
        assign_pattern_field_id: dir::LocalNodeId<dir::AssignPatternField>,
        expression_stack: &mut Vec<dir::LocalNodeId<dir::Expression>>,
        symbol_stack: &mut Vec<dir::GlobalSymbolId>,
    ) -> TaintLabels {
        let assign_pattern_field = self.tree.get(assign_pattern_field_id);
        let mut labels = TaintLabels::default();

        match assign_pattern_field {
            dir::AssignPatternField::Named { pattern, .. } => {
                if let Some(pattern_id) = pattern {
                    let pattern_labels = self.assign_pattern_taint_labels_inner(
                        *pattern_id,
                        expression_stack,
                        symbol_stack,
                    );
                    labels.merge(&pattern_labels);
                }
            }
            dir::AssignPatternField::Computed { key, pattern } => {
                let key_labels =
                    self.expression_taint_labels_inner(*key, expression_stack, symbol_stack);
                labels.merge(&key_labels);

                let pattern_labels = self.assign_pattern_taint_labels_inner(
                    *pattern,
                    expression_stack,
                    symbol_stack,
                );
                labels.merge(&pattern_labels);
            }
            dir::AssignPatternField::Positional { pattern } => {
                let pattern_labels = self.assign_pattern_taint_labels_inner(
                    *pattern,
                    expression_stack,
                    symbol_stack,
                );
                labels.merge(&pattern_labels);
            }
            dir::AssignPatternField::Spread { pattern } => {
                if let Some(pattern_id) = pattern {
                    let pattern_labels = self.assign_pattern_taint_labels_inner(
                        *pattern_id,
                        expression_stack,
                        symbol_stack,
                    );
                    labels.merge(&pattern_labels);
                }
            }
            dir::AssignPatternField::Elision => {}
        }

        labels
    }

    /// Resolve one expression with recursion guards.
    fn expression_taint_labels_inner(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        expression_stack: &mut Vec<dir::LocalNodeId<dir::Expression>>,
        symbol_stack: &mut Vec<dir::GlobalSymbolId>,
    ) -> TaintLabels {
        let expression_id = expression_unwrap_parenthesized(self.tree, expression_id);
        if let Some(labels) = self.cache.expression_labels.get(&expression_id.id) {
            return labels.clone();
        }
        if expression_stack.contains(&expression_id) {
            return TaintLabels::default();
        }
        expression_stack.push(expression_id);

        let expression = self.tree.get(expression_id);
        let mut labels = self.expression_attribute_taint_labels(expression_id, expression);

        if self.include_heuristic_sources
            && expression_is_heuristically_tainted(self.tree, expression_id)
        {
            labels.add_unlabeled();
        }

        match expression {
            dir::Expression::Block(block) => {
                // block expressions taint from their last expression value
                let block = self.tree.get(*block);
                if let Some(last_expression_id) = block.last_expression() {
                    let block_labels = self.expression_taint_labels_inner(
                        last_expression_id,
                        expression_stack,
                        symbol_stack,
                    );
                    labels.merge(&block_labels);
                }
            }
            dir::Expression::As {
                expression: value,
                target_type: _,
            }
            | dir::Expression::Satisfies {
                expression: value,
                target_type: _,
            } => {
                // casts do not sanitize by default
                let value_labels =
                    self.expression_taint_labels_inner(*value, expression_stack, symbol_stack);
                labels.merge(&value_labels);
            }
            dir::Expression::Unary { right, .. }
            | dir::Expression::MoveOf { right, .. }
            | dir::Expression::BorrowOf { right, .. } => {
                // unary wrappers preserve operand taint
                let right_labels =
                    self.expression_taint_labels_inner(*right, expression_stack, symbol_stack);
                labels.merge(&right_labels);
            }
            dir::Expression::Binary { left, right, .. } => {
                // binary expressions taint from both operands
                let left_labels =
                    self.expression_taint_labels_inner(*left, expression_stack, symbol_stack);
                labels.merge(&left_labels);

                let right_labels =
                    self.expression_taint_labels_inner(*right, expression_stack, symbol_stack);
                labels.merge(&right_labels);
            }
            dir::Expression::Assign {
                left,
                operator: _,
                right,
            } => {
                // assignments taint from both the target and the assigned value
                let left_labels =
                    self.assign_pattern_taint_labels_inner(*left, expression_stack, symbol_stack);
                labels.merge(&left_labels);

                let right_labels =
                    self.expression_taint_labels_inner(*right, expression_stack, symbol_stack);
                labels.merge(&right_labels);
            }
            dir::Expression::Member { left, .. }
            | dir::Expression::PrivateMember { left, .. }
            | dir::Expression::Maybe { left, .. }
            | dir::Expression::Must { left, .. }
            | dir::Expression::Instantiation { left, .. } => {
                // member and wrapper expressions preserve receiver taint
                let left_labels =
                    self.expression_taint_labels_inner(*left, expression_stack, symbol_stack);
                labels.merge(&left_labels);
            }
            dir::Expression::Index { left, index, .. } => {
                // indexing taints from receiver and optional index
                let left_labels =
                    self.expression_taint_labels_inner(*left, expression_stack, symbol_stack);
                labels.merge(&left_labels);

                if let Some(index) = index {
                    let index_labels =
                        self.expression_taint_labels_inner(*index, expression_stack, symbol_stack);
                    labels.merge(&index_labels);
                }
            }
            dir::Expression::Call {
                left, arguments, ..
            } => {
                // call results can be marked tainted by callee attributes
                let callee_expression = self.tree.get(*left);
                let callee_labels =
                    self.expression_attribute_taint_labels(*left, callee_expression);
                labels.merge(&callee_labels);

                // propagate taint from call arguments
                for argument_id in arguments.iter().copied() {
                    let argument = self.tree.get(argument_id);
                    let Some(value) = argument.value() else {
                        continue;
                    };
                    let argument_labels =
                        self.expression_taint_labels_inner(value, expression_stack, symbol_stack);
                    labels.merge(&argument_labels);
                }

                // apply opt-in sanitizer tags on the callee
                let sanitizer_labels = expression_sanitizer_taint_labels(
                    self.artifacts,
                    self.profile_id,
                    self.module_id,
                    self.tree,
                    self.strings,
                    self.symbols,
                    self.types,
                    self.resolutions,
                    *left,
                );
                labels.apply_sanitizer(&sanitizer_labels);
            }
            dir::Expression::New { arguments, .. } => {
                // propagate taint from constructor arguments
                for argument_id in arguments.iter().copied() {
                    let argument = self.tree.get(argument_id);
                    let Some(value) = argument.value() else {
                        continue;
                    };
                    let argument_labels =
                        self.expression_taint_labels_inner(value, expression_stack, symbol_stack);
                    labels.merge(&argument_labels);
                }
            }
            dir::Expression::Await { expression }
            | dir::Expression::AwaitMaybe { expression }
            | dir::Expression::AwaitMust { expression } => {
                // awaits preserve taint
                let awaited_labels =
                    self.expression_taint_labels_inner(*expression, expression_stack, symbol_stack);
                labels.merge(&awaited_labels);
            }
            dir::Expression::Throw { value } => {
                // thrown values preserve taint
                let value_labels =
                    self.expression_taint_labels_inner(*value, expression_stack, symbol_stack);
                labels.merge(&value_labels);
            }
            dir::Expression::TemplateExpression { value } => {
                // template expressions taint from interpolations
                let template_labels =
                    self.template_literal_taint_labels(value, expression_stack, symbol_stack);
                labels.merge(&template_labels);
            }
            dir::Expression::TaggedTemplateExpression {
                tag,
                value,
                generic_arguments: _,
            } => {
                // tagged templates taint from tag and interpolation values
                let tag_labels =
                    self.expression_taint_labels_inner(*tag, expression_stack, symbol_stack);
                labels.merge(&tag_labels);

                let template_labels =
                    self.template_literal_taint_labels(value, expression_stack, symbol_stack);
                labels.merge(&template_labels);
            }
            dir::Expression::ArrayExpression { elements }
            | dir::Expression::TupleExpression { elements } => {
                // aggregate values taint from their elements
                for argument_id in elements.iter().copied() {
                    let argument = self.tree.get(argument_id);
                    let Some(value) = argument.value() else {
                        continue;
                    };
                    let element_labels =
                        self.expression_taint_labels_inner(value, expression_stack, symbol_stack);
                    labels.merge(&element_labels);
                }
            }
            dir::Expression::SequenceExpression { expressions } => {
                // sequence expression result is the last value
                if let Some(last) = expressions.last() {
                    let last_labels =
                        self.expression_taint_labels_inner(*last, expression_stack, symbol_stack);
                    labels.merge(&last_labels);
                }
            }
            dir::Expression::ObjectExpression { properties }
            | dir::Expression::StructExpression { properties, .. } => {
                // object literals taint from field values and spreads
                for property_id in properties.iter().copied() {
                    let property = self.tree.get(property_id);
                    match property {
                        dir::Property::Field { value, .. } => {
                            let value_labels = self.expression_taint_labels_inner(
                                *value,
                                expression_stack,
                                symbol_stack,
                            );
                            labels.merge(&value_labels);
                        }
                        dir::Property::Method { .. } => {}
                        dir::Property::Spread { value, .. } => {
                            let value_labels = self.expression_taint_labels_inner(
                                *value,
                                expression_stack,
                                symbol_stack,
                            );
                            labels.merge(&value_labels);
                        }
                        dir::Property::Error => {}
                    }
                }
            }
            dir::Expression::TreeExpression {
                arguments,
                elements,
                generic_arguments: _,
                left: _,
            } => {
                // tree expressions taint from arguments and children
                if let Some(arguments) = arguments {
                    for argument_id in arguments.iter().copied() {
                        let argument = self.tree.get(argument_id);
                        let Some(value) = argument.value() else {
                            continue;
                        };
                        let argument_labels = self.expression_taint_labels_inner(
                            value,
                            expression_stack,
                            symbol_stack,
                        );
                        labels.merge(&argument_labels);
                    }
                }

                if let Some(elements) = elements {
                    for element_id in elements.iter().copied() {
                        let element = self.tree.get(element_id);
                        let Some(value) = element.value() else {
                            continue;
                        };
                        let element_labels = self.expression_taint_labels_inner(
                            value,
                            expression_stack,
                            symbol_stack,
                        );
                        labels.merge(&element_labels);
                    }
                }
            }
            dir::Expression::If {
                then_expression,
                else_expression,
                ..
            } => {
                // if expressions taint from branch values
                let then_labels = self.expression_taint_labels_inner(
                    *then_expression,
                    expression_stack,
                    symbol_stack,
                );
                labels.merge(&then_labels);

                if let Some(else_expression) = else_expression {
                    let else_labels = self.expression_taint_labels_inner(
                        *else_expression,
                        expression_stack,
                        symbol_stack,
                    );
                    labels.merge(&else_labels);
                }
            }
            dir::Expression::Try { body, catch, .. } => {
                // try expressions taint from try and catch branches
                let try_labels =
                    self.expression_taint_labels_inner(*body, expression_stack, symbol_stack);
                labels.merge(&try_labels);

                if let Some(catch) = catch {
                    let catch = self.tree.get(*catch);
                    let catch_labels = self.expression_taint_labels_inner(
                        catch.body,
                        expression_stack,
                        symbol_stack,
                    );
                    labels.merge(&catch_labels);
                }
            }
            dir::Expression::Match { cases, .. } => {
                // match expressions taint from case body values
                for case_id in cases.iter().copied() {
                    let case = self.tree.get(case_id);
                    match case {
                        dir::MatchCase::Expression { body, .. } => {
                            let case_labels = self.expression_taint_labels_inner(
                                *body,
                                expression_stack,
                                symbol_stack,
                            );
                            labels.merge(&case_labels);
                        }
                        dir::MatchCase::Block { body, .. } => {
                            let block = self.tree.get(*body);
                            if let Some(last_expression_id) = block.last_expression() {
                                let case_labels = self.expression_taint_labels_inner(
                                    last_expression_id,
                                    expression_stack,
                                    symbol_stack,
                                );
                                labels.merge(&case_labels);
                            }
                        }
                    }
                }
            }
            dir::Expression::Return { value } | dir::Expression::Yield { value, .. } => {
                // return and yield values preserve taint
                if let Some(value) = value {
                    let value_labels =
                        self.expression_taint_labels_inner(*value, expression_stack, symbol_stack);
                    labels.merge(&value_labels);
                }
            }
            dir::Expression::Comptime { body } => {
                // comptime wrappers preserve inner value taint
                let body_labels =
                    self.expression_taint_labels_inner(*body, expression_stack, symbol_stack);
                labels.merge(&body_labels);
            }
            _ => {}
        }

        let candidate_symbols =
            expression_candidate_symbols(self.module_id, self.resolutions, expression_id);
        for symbol_id in candidate_symbols {
            let symbol_labels =
                self.symbol_taint_labels_inner(symbol_id, expression_stack, symbol_stack);
            labels.merge(&symbol_labels);
        }

        expression_stack.pop();
        self.cache
            .expression_labels
            .insert(expression_id.id, labels.clone());
        labels
    }

    /// Resolve taint labels for one symbol with recursion guards.
    fn symbol_taint_labels_inner(
        &mut self,
        symbol_id: dir::GlobalSymbolId,
        expression_stack: &mut Vec<dir::LocalNodeId<dir::Expression>>,
        symbol_stack: &mut Vec<dir::GlobalSymbolId>,
    ) -> TaintLabels {
        if let Some(labels) = self.cache.symbol_labels.get(&symbol_id) {
            return labels.clone();
        }
        if symbol_stack.contains(&symbol_id) {
            return TaintLabels::default();
        }
        symbol_stack.push(symbol_id);

        let mut labels = symbol_taint_labels_from_decorators(
            self.artifacts,
            self.profile_id,
            self.module_id,
            self.tree,
            self.strings,
            self.symbols,
            self.types,
            self.resolutions,
            symbol_id,
        );

        if let Some(initializer) =
            self.symbol_initializer_expression(symbol_id, expression_stack, symbol_stack)
        {
            labels.merge(&initializer);
        }

        symbol_stack.pop();
        self.cache.symbol_labels.insert(symbol_id, labels.clone());
        labels
    }

    /// Resolve taint from expression target attributes.
    fn expression_attribute_taint_labels(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        _expression: &dir::Expression,
    ) -> TaintLabels {
        let candidate_symbols =
            expression_candidate_symbols(self.module_id, self.resolutions, expression_id);

        let mut labels = TaintLabels::default();
        for symbol_id in candidate_symbols {
            let symbol_labels = symbol_taint_labels_from_decorators(
                self.artifacts,
                self.profile_id,
                self.module_id,
                self.tree,
                self.strings,
                self.symbols,
                self.types,
                self.resolutions,
                symbol_id,
            );
            labels.merge(&symbol_labels);
        }

        labels
    }

    /// Resolve taint labels from a template literal payload.
    fn template_literal_taint_labels(
        &mut self,
        value: &dir::TemplateLiteral,
        expression_stack: &mut Vec<dir::LocalNodeId<dir::Expression>>,
        symbol_stack: &mut Vec<dir::GlobalSymbolId>,
    ) -> TaintLabels {
        let mut labels = TaintLabels::default();

        let dir::TemplateLiteral::InterpolatedString { arguments, .. } = value else {
            return labels;
        };
        for argument_id in arguments.iter().copied() {
            let argument = self.tree.get(argument_id);
            let Some(value) = argument.value() else {
                continue;
            };
            let argument_labels =
                self.expression_taint_labels_inner(value, expression_stack, symbol_stack);
            labels.merge(&argument_labels);
        }

        labels
    }

    /// Resolve taint from one symbol initializer when available.
    fn symbol_initializer_expression(
        &mut self,
        symbol_id: dir::GlobalSymbolId,
        expression_stack: &mut Vec<dir::LocalNodeId<dir::Expression>>,
        symbol_stack: &mut Vec<dir::GlobalSymbolId>,
    ) -> Option<TaintLabels> {
        let value_expression_id = resolve_symbol_initializer_expression(
            self.artifacts,
            self.profile_id,
            self.module_id,
            self.symbols,
            self.tree,
            symbol_id,
        )?;
        Some(self.expression_taint_labels_inner(
            value_expression_id,
            expression_stack,
            symbol_stack,
        ))
    }
}

/// Return sink taint labels declared on expression target symbols.
pub fn expression_sink_taint_labels(
    artifacts: &ArtifactCache,
    profile_id: ProfileId,
    module_id: ModuleId,
    tree: &dir::Tree,
    strings: &StringPool,
    symbols: &dir::BindingTable<'_>,
    types: &dir::TypeTable<'_>,
    resolutions: &dir::ResolutionTable<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> TaintLabels {
    let Some(sink_symbol) = language_item_symbol(artifacts, profile_id, dir::LanguageItem::Sink)
    else {
        return TaintLabels::default();
    };

    let candidate_symbols = expression_candidate_symbols(module_id, resolutions, expression_id);

    let mut labels = TaintLabels::default();
    for symbol_id in candidate_symbols {
        let decorators = symbol_decorators_for(
            artifacts,
            profile_id,
            module_id,
            tree,
            strings,
            symbols,
            types,
            resolutions,
            symbol_id,
            sink_symbol,
        );
        let sink_labels = decorator_taint_labels(decorators);
        labels.merge(&sink_labels);
    }

    labels
}

/// Return sanitizer taint labels declared on expression target symbols.
pub fn expression_sanitizer_taint_labels(
    artifacts: &ArtifactCache,
    profile_id: ProfileId,
    module_id: ModuleId,
    tree: &dir::Tree,
    strings: &StringPool,
    symbols: &dir::BindingTable<'_>,
    types: &dir::TypeTable<'_>,
    resolutions: &dir::ResolutionTable<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> TaintLabels {
    let Some(sanitizer_symbol) =
        language_item_symbol(artifacts, profile_id, dir::LanguageItem::Untaint)
    else {
        return TaintLabels::default();
    };

    let candidate_symbols = expression_candidate_symbols(module_id, resolutions, expression_id);

    let mut labels = TaintLabels::default();
    for symbol_id in candidate_symbols {
        let decorators = symbol_decorators_for(
            artifacts,
            profile_id,
            module_id,
            tree,
            strings,
            symbols,
            types,
            resolutions,
            symbol_id,
            sanitizer_symbol,
        );
        let sanitizer_labels = decorator_taint_labels(decorators);
        labels.merge(&sanitizer_labels);
    }

    labels
}

/// Return true when expression shape looks like user-controlled data.
fn expression_is_heuristically_tainted(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let expression_id = expression_unwrap_parenthesized(tree, expression_id);
    let expression = tree.get(expression_id);

    if matches!(expression, dir::Expression::ScalarLiteral(_)) {
        return false;
    }

    matches!(
        expression,
        dir::Expression::QualifiedReference { .. }
            | dir::Expression::Member { .. }
            | dir::Expression::Call { .. }
            | dir::Expression::Index { .. }
            | dir::Expression::Binary { .. }
            | dir::Expression::TemplateExpression { .. }
    )
}

/// Resolve taint labels from symbol decorators.
fn symbol_taint_labels_from_decorators(
    artifacts: &ArtifactCache,
    profile_id: ProfileId,
    module_id: ModuleId,
    tree: &dir::Tree,
    strings: &StringPool,
    symbols: &dir::BindingTable<'_>,
    types: &dir::TypeTable<'_>,
    resolutions: &dir::ResolutionTable<'_>,
    symbol_id: dir::GlobalSymbolId,
) -> TaintLabels {
    let Some(taint_symbol) = language_item_symbol(artifacts, profile_id, dir::LanguageItem::Taint)
    else {
        return TaintLabels::default();
    };

    let decorators = symbol_decorators_for(
        artifacts,
        profile_id,
        module_id,
        tree,
        strings,
        symbols,
        types,
        resolutions,
        symbol_id,
        taint_symbol,
    );

    decorator_taint_labels(decorators)
}

/// Resolve labels from matching decorators.
fn decorator_taint_labels(decorators: Vec<SymbolDecorator>) -> TaintLabels {
    let mut labels = TaintLabels::default();

    for decorator in decorators {
        for label in decorator.arguments {
            if let Some(label) = label {
                labels.add_label(label);
            } else {
                labels.add_unlabeled();
            }
        }
    }

    labels
}

/// Resolve a language item from the artifact cache.
fn language_item_symbol(
    artifacts: &ArtifactCache,
    profile_id: ProfileId,
    item: dir::LanguageItem,
) -> Option<dir::GlobalSymbolId> {
    let environment = artifacts.global_environment(profile_id)?;

    environment.language.symbol(item)
}
