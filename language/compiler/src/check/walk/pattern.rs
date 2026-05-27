use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    AssignPatternField, AssignPatternTerm, CheckState, PatternField, PatternTerm, TermId,
};

impl CheckState<'_> {
    /// Walk one pattern.
    pub(in crate::check) fn walk_pattern(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Pattern>,
        pattern: &dir::Pattern,
    ) {
        if !self.push_static_condition_for(tree, id.into_any(), None) {
            return;
        }
        match pattern {
            // _
            dir::Pattern::Wildcard => {}
            // pattern!
            dir::Pattern::Must(pattern)
            // &pattern
            | dir::Pattern::BorrowOf { right: pattern, .. }
            // move pattern
            | dir::Pattern::MoveOf { right: pattern, .. }
            // *pattern
            | dir::Pattern::DereferenceOf { right: pattern } => {
                self.walk_pattern(tree, *pattern, tree.get(*pattern));
            }
            // pattern = value
            dir::Pattern::Assign { pattern, value } => {
                self.walk_pattern(tree, *pattern, tree.get(*pattern));

                // check pattern default in selector context
                let before_value = self.checkpoint_flow(tree.module_id);

                self.walk_expression(tree, *value, tree.get(*value));
                self.restore_flow(tree.module_id, before_value);
            }
            // name: pattern
            dir::Pattern::Binding {
                pattern: Some(pattern),
                ..
            } => {
                self.walk_pattern(tree, *pattern, tree.get(*pattern));
            }
            // name
            dir::Pattern::Binding { pattern: None, .. } => {}
            // value
            dir::Pattern::Expression { value } => {
                // check value pattern in selector context
                let before_value = self.checkpoint_flow(tree.module_id);

                self.walk_expression(tree, *value, tree.get(*value));
                self.restore_flow(tree.module_id, before_value);
            }
            // start..end
            dir::Pattern::Range { start, end, .. } => {
                if let Some(start) = start {
                    // check range bound in selector context
                    let before_start = self.checkpoint_flow(tree.module_id);

                    self.walk_expression(tree, *start, tree.get(*start));
                    self.restore_flow(tree.module_id, before_start);
                }

                if let Some(end) = end {
                    // check range bound in selector context
                    let before_end = self.checkpoint_flow(tree.module_id);

                    self.walk_expression(tree, *end, tree.get(*end));
                    self.restore_flow(tree.module_id, before_end);
                }
            }
            // value is T
            dir::Pattern::TypeExpression { value } => {
                self.walk_type_expression(tree, *value, tree.get(*value));
            }
            // [a, b]
            dir::Pattern::Tuple { fields }
            // [...items]
            | dir::Pattern::Sequence { fields }
            // { name }
            | dir::Pattern::Object { fields } => {
                for field in fields {
                    self.walk_pattern_field(tree, *field, tree.get(field.clone()));
                }
            }
            // T(a, b)
            dir::Pattern::TaggedTuple { ty, fields }
            // T { name }
            | dir::Pattern::TaggedObject { ty, fields } => {
                self.walk_type_expression(tree, *ty, tree.get(*ty));

                for field in fields {
                    self.walk_pattern_field(tree, *field, tree.get(field.clone()));
                }
            }
            // a | b
            dir::Pattern::Union { patterns } => {
                for pattern in patterns {
                    self.walk_pattern(tree, *pattern, tree.get(*pattern));
                }
            }
        };

        self.pop_static_condition(tree.module_id);
    }

    /// Walk one pattern field.
    pub(in crate::check) fn walk_pattern_field(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::PatternField>,
        pattern_field: &dir::PatternField,
    ) {
        if !self.push_static_condition_for(tree, id.into_any(), None) {
            return;
        }
        match pattern_field {
            // { name: pattern }
            dir::PatternField::Named {
                pattern: Some(pattern),
                ..
            }
            // { ...pattern }
            | dir::PatternField::Spread {
                pattern: Some(pattern),
            } => {
                self.walk_pattern(tree, *pattern, tree.get(*pattern));
            }
            // { [key]: pattern }
            dir::PatternField::Computed { key, pattern } => {
                // check computed key in selector context
                let before_key = self.checkpoint_flow(tree.module_id);

                self.walk_expression(tree, *key, tree.get(*key));
                self.restore_flow(tree.module_id, before_key);

                self.walk_pattern(tree, *pattern, tree.get(*pattern));
            }
            // [pattern]
            dir::PatternField::Positional { pattern } => {
                self.walk_pattern(tree, *pattern, tree.get(*pattern));
            }
            // { name }
            dir::PatternField::Named { pattern: None, .. }
            // { ... }
            | dir::PatternField::Spread { pattern: None }
            // [,]
            | dir::PatternField::Elision => {}
        };

        self.pop_static_condition(tree.module_id);
    }

    /// Walk one assignment pattern.
    pub(in crate::check) fn walk_assign_pattern(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::AssignPattern>,
        assign_pattern: &dir::AssignPattern,
    ) {
        if !self.push_static_condition_for(tree, id.into_any(), None) {
            return;
        }
        match assign_pattern {
            // target
            dir::AssignPattern::Expression { value } => {
                self.walk_expression(tree, *value, tree.get(*value));
            }
            // target = value
            dir::AssignPattern::Assign { pattern, value } => {
                self.walk_assign_pattern(tree, *pattern, tree.get(*pattern));

                // check destructuring default in conditional assignment context
                let before_value = self.checkpoint_flow(tree.module_id);

                self.walk_expression(tree, *value, tree.get(*value));
                self.restore_flow(tree.module_id, before_value);
            }
            // [a, b]
            dir::AssignPattern::Sequence { fields }
            // { a, b }
            | dir::AssignPattern::Object { fields } => {
                for field in fields {
                    self.walk_assign_pattern_field(tree, *field, tree.get(field.clone()));
                }
            }
        };

        self.pop_static_condition(tree.module_id);
    }

    /// Walk one assignment pattern field.
    pub(in crate::check) fn walk_assign_pattern_field(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::AssignPatternField>,
        assign_pattern_field: &dir::AssignPatternField,
    ) {
        if !self.push_static_condition_for(tree, id.into_any(), None) {
            return;
        }
        match assign_pattern_field {
            // { name: pattern }
            dir::AssignPatternField::Named {
                pattern: Some(pattern),
                ..
            }
            // { ...pattern }
            | dir::AssignPatternField::Spread {
                pattern: Some(pattern),
            } => {
                self.walk_assign_pattern(tree, *pattern, tree.get(*pattern));
            }
            // { [key]: pattern }
            dir::AssignPatternField::Computed { key, pattern } => {
                self.walk_expression(tree, *key, tree.get(*key));
                self.walk_assign_pattern(tree, *pattern, tree.get(*pattern));
            }
            // [pattern]
            dir::AssignPatternField::Positional { pattern } => {
                self.walk_assign_pattern(tree, *pattern, tree.get(*pattern));
            }
            // { name }
            dir::AssignPatternField::Named { pattern: None, .. }
            // { ... }
            | dir::AssignPatternField::Spread { pattern: None }
            // [,]
            | dir::AssignPatternField::Elision => {}
        };

        self.pop_static_condition(tree.module_id);
    }

    /// Return one pattern term from syntax.
    pub(in crate::check) fn build_pattern_term(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::Pattern>,
        tree: &dir::Tree,
    ) -> Option<TermId<PatternTerm>> {
        let term = match tree.get(id) {
            // _
            dir::Pattern::Wildcard => PatternTerm::Wildcard,
            // pattern!
            dir::Pattern::Must(pattern) => PatternTerm::Must {
                pattern: self.build_pattern_term(module, *pattern, tree)?,
            },
            // pattern = value
            dir::Pattern::Assign { pattern, value } => PatternTerm::Assign {
                pattern: self.build_pattern_term(module, *pattern, tree)?,
                value: self.intern_local_type_variable(module, *value),
            },
            // &pattern
            dir::Pattern::BorrowOf { mutability, right } => PatternTerm::BorrowOf {
                mutability: *mutability,
                pattern: self.build_pattern_term(module, *right, tree)?,
            },
            // ^pattern
            dir::Pattern::MoveOf { mutability, right } => PatternTerm::MoveOf {
                mutability: *mutability,
                pattern: self.build_pattern_term(module, *right, tree)?,
            },
            // *pattern
            dir::Pattern::DereferenceOf { right } => PatternTerm::DereferenceOf {
                pattern: self.build_pattern_term(module, *right, tree)?,
            },
            // name, name: pattern
            dir::Pattern::Binding { pattern, .. } => PatternTerm::Binding {
                symbol: self.declaration_symbol(tree.module_id, id.into_any()),
                pattern: pattern.and_then(|pattern| self.build_pattern_term(module, pattern, tree)),
            },
            // value
            dir::Pattern::Expression { value } => PatternTerm::Expression {
                value: self.intern_local_type_variable(module, *value),
            },
            // start..end
            dir::Pattern::Range {
                start,
                end,
                end_kind,
            } => PatternTerm::Range {
                start: start.map(|start| self.intern_local_type_variable(module, start)),
                end: end.map(|end| self.intern_local_type_variable(module, end)),
                end_kind: *end_kind,
            },
            // value is T
            dir::Pattern::TypeExpression { value } => PatternTerm::Type {
                ty: self.intern_local_type_variable(module, *value),
            },
            // [a, b]
            dir::Pattern::Tuple { fields } => PatternTerm::Tuple {
                fields: fields
                    .iter()
                    .filter_map(|field| self.build_pattern_field_term(module, *field, tree))
                    .collect(),
            },
            // T(a, b)
            dir::Pattern::TaggedTuple { ty, fields } => PatternTerm::TaggedTuple {
                ty: self.intern_local_type_variable(module, *ty),
                fields: fields
                    .iter()
                    .filter_map(|field| self.build_pattern_field_term(module, *field, tree))
                    .collect(),
            },
            // [...items]
            dir::Pattern::Sequence { fields } => PatternTerm::Sequence {
                fields: fields
                    .iter()
                    .filter_map(|field| self.build_pattern_field_term(module, *field, tree))
                    .collect(),
            },
            // { name }
            dir::Pattern::Object { fields } => PatternTerm::Object {
                fields: fields
                    .iter()
                    .filter_map(|field| self.build_pattern_field_term(module, *field, tree))
                    .collect(),
            },
            // T { name }
            dir::Pattern::TaggedObject { ty, fields } => PatternTerm::TaggedObject {
                ty: self.intern_local_type_variable(module, *ty),
                fields: fields
                    .iter()
                    .filter_map(|field| self.build_pattern_field_term(module, *field, tree))
                    .collect(),
            },
            // a | b
            dir::Pattern::Union { patterns } => PatternTerm::Union {
                patterns: patterns
                    .iter()
                    .filter_map(|pattern| self.build_pattern_term(module, *pattern, tree))
                    .collect(),
            },
        };

        Some(self.terms.push(term))
    }

    /// Return one pattern field term from syntax.
    fn build_pattern_field_term(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::PatternField>,
        tree: &dir::Tree,
    ) -> Option<PatternField> {
        let term = match tree.get(id) {
            // { name: pattern }
            dir::PatternField::Named { name, pattern, .. } => PatternField::Named {
                key: name.static_key(),
                pattern: pattern.and_then(|pattern| self.build_pattern_term(module, pattern, tree)),
            },
            // { [key]: pattern }
            dir::PatternField::Computed { key, pattern } => PatternField::Computed {
                key: self.intern_local_type_variable(module, *key),
                pattern: self.build_pattern_term(module, *pattern, tree)?,
            },
            // [pattern]
            dir::PatternField::Positional { pattern } => PatternField::Positional {
                pattern: self.build_pattern_term(module, *pattern, tree)?,
            },
            // { ...pattern }
            dir::PatternField::Spread { pattern } => PatternField::Spread {
                pattern: pattern.and_then(|pattern| self.build_pattern_term(module, pattern, tree)),
            },
            // [,]
            dir::PatternField::Elision => PatternField::Elision,
        };

        Some(term)
    }

    /// Return one assignment pattern term from syntax.
    pub(in crate::check) fn build_assign_pattern_term(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::AssignPattern>,
        tree: &dir::Tree,
    ) -> Option<TermId<AssignPatternTerm>> {
        let term = match tree.get(id) {
            // target
            dir::AssignPattern::Expression { value } => AssignPatternTerm::Expression {
                place: self.intern_local_type_variable(module, *value),
            },
            // target = value
            dir::AssignPattern::Assign { pattern, value } => AssignPatternTerm::Assign {
                pattern: self.build_assign_pattern_term(module, *pattern, tree)?,
                value: self.intern_local_type_variable(module, *value),
            },
            // [a, b]
            dir::AssignPattern::Sequence { fields } => AssignPatternTerm::Sequence {
                fields: fields
                    .iter()
                    .filter_map(|field| self.build_assign_pattern_field_term(module, *field, tree))
                    .collect(),
            },
            // { a, b }
            dir::AssignPattern::Object { fields } => AssignPatternTerm::Object {
                fields: fields
                    .iter()
                    .filter_map(|field| self.build_assign_pattern_field_term(module, *field, tree))
                    .collect(),
            },
        };

        Some(self.terms.push(term))
    }

    /// Return one assignment pattern field term from syntax.
    fn build_assign_pattern_field_term(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::AssignPatternField>,
        tree: &dir::Tree,
    ) -> Option<AssignPatternField> {
        let term = match tree.get(id) {
            // { name: pattern }
            dir::AssignPatternField::Named { name, pattern, .. } => AssignPatternField::Named {
                key: name.static_key(),
                pattern: pattern
                    .and_then(|pattern| self.build_assign_pattern_term(module, pattern, tree)),
            },
            // { [key]: pattern }
            dir::AssignPatternField::Computed { key, pattern } => AssignPatternField::Computed {
                key: self.intern_local_type_variable(module, *key),
                pattern: self.build_assign_pattern_term(module, *pattern, tree)?,
            },
            // [pattern]
            dir::AssignPatternField::Positional { pattern } => AssignPatternField::Positional {
                pattern: self.build_assign_pattern_term(module, *pattern, tree)?,
            },
            // { ...pattern }
            dir::AssignPatternField::Spread { pattern } => AssignPatternField::Spread {
                pattern: pattern
                    .and_then(|pattern| self.build_assign_pattern_term(module, pattern, tree)),
            },
            // [,]
            dir::AssignPatternField::Elision => AssignPatternField::Elision,
        };

        Some(term)
    }
}
