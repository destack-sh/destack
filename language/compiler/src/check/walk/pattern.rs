use destack_dir as dir;
use dir::NodeVisitor as _;

use crate::check::{
    AssignPatternFieldTerm, AssignPatternTerm, CheckModuleState, PatternFieldTerm, PatternTerm,
};

impl CheckModuleState {
    /// Walk one pattern.
    pub(in crate::check) fn walk_pattern(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Pattern>,
        pattern: &dir::Pattern,
    ) {
        // apply static owner guards
        if !self.static_allows(tree, id.into_any()) {
            return;
        }

        self.visit_any(tree, dir::NodeType::Pattern, id.id);

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
                let before_value = self.checkpoint_flow();

                self.walk_expression(tree, *value, tree.get(*value));
                self.restore_flow(before_value);
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
                let before_value = self.checkpoint_flow();

                self.walk_expression(tree, *value, tree.get(*value));
                self.restore_flow(before_value);
            }
            // start..end
            dir::Pattern::Range { start, end, .. } => {
                if let Some(start) = start {
                    // check range bound in selector context
                    let before_start = self.checkpoint_flow();

                    self.walk_expression(tree, *start, tree.get(*start));
                    self.restore_flow(before_start);
                }

                if let Some(end) = end {
                    // check range bound in selector context
                    let before_end = self.checkpoint_flow();

                    self.walk_expression(tree, *end, tree.get(*end));
                    self.restore_flow(before_end);
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
                    self.walk_pattern_field(tree, *field, tree.get(*field));
                }
            }
            // T(a, b)
            dir::Pattern::TaggedTuple { ty, fields }
            // T { name }
            | dir::Pattern::TaggedObject { ty, fields } => {
                self.walk_type_expression(tree, *ty, tree.get(*ty));

                for field in fields {
                    self.walk_pattern_field(tree, *field, tree.get(*field));
                }
            }
            // a | b
            dir::Pattern::Union { patterns } => {
                for pattern in patterns {
                    self.walk_pattern(tree, *pattern, tree.get(*pattern));
                }
            }
        };
    }

    /// Walk one pattern field.
    pub(in crate::check) fn walk_pattern_field(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::PatternField>,
        pattern_field: &dir::PatternField,
    ) {
        // apply static owner guards
        if !self.static_allows(tree, id.into_any()) {
            return;
        }

        self.visit_any(tree, dir::NodeType::PatternField, id.id);

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
                let before_key = self.checkpoint_flow();

                self.walk_expression(tree, *key, tree.get(*key));
                self.restore_flow(before_key);

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
    }

    /// Walk one assignment pattern.
    pub(in crate::check) fn walk_assign_pattern(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::AssignPattern>,
        assign_pattern: &dir::AssignPattern,
    ) {
        // apply static owner guards
        if !self.static_allows(tree, id.into_any()) {
            return;
        }

        self.visit_any(tree, dir::NodeType::AssignPattern, id.id);

        match assign_pattern {
            // target
            dir::AssignPattern::Expression { value } => {
                self.walk_expression(tree, *value, tree.get(*value));
            }
            // target = value
            dir::AssignPattern::Assign { pattern, value } => {
                self.walk_assign_pattern(tree, *pattern, tree.get(*pattern));

                // check destructuring default in conditional assignment context
                let before_value = self.checkpoint_flow();

                self.walk_expression(tree, *value, tree.get(*value));
                self.restore_flow(before_value);
            }
            // [a, b]
            dir::AssignPattern::Sequence { fields }
            // { a, b }
            | dir::AssignPattern::Object { fields } => {
                for field in fields {
                    self.walk_assign_pattern_field(tree, *field, tree.get(*field));
                }
            }
        };
    }

    /// Walk one assignment pattern field.
    pub(in crate::check) fn walk_assign_pattern_field(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::AssignPatternField>,
        assign_pattern_field: &dir::AssignPatternField,
    ) {
        // apply static owner guards
        if !self.static_allows(tree, id.into_any()) {
            return;
        }

        self.visit_any(tree, dir::NodeType::AssignPatternField, id.id);

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
    }

    /// Return one pattern term from syntax.
    pub(in crate::check) fn build_pattern_term(
        &mut self,
        id: dir::LocalNodeId<dir::Pattern>,
        tree: &dir::Tree,
    ) -> Option<PatternTerm> {
        let term = match tree.get(id) {
            // _
            dir::Pattern::Wildcard => PatternTerm::Wildcard,
            // pattern!
            dir::Pattern::Must(pattern) => PatternTerm::Must {
                pattern: Box::new(self.build_pattern_term(*pattern, tree)?),
            },
            // pattern = value
            dir::Pattern::Assign { pattern, value } => PatternTerm::Assign {
                pattern: Box::new(self.build_pattern_term(*pattern, tree)?),
                value: self.intern_local_type_variable(*value),
            },
            // &pattern
            dir::Pattern::BorrowOf { mutability, right } => PatternTerm::BorrowOf {
                mutability: *mutability,
                pattern: Box::new(self.build_pattern_term(*right, tree)?),
            },
            // ^pattern
            dir::Pattern::MoveOf { mutability, right } => PatternTerm::MoveOf {
                mutability: *mutability,
                pattern: Box::new(self.build_pattern_term(*right, tree)?),
            },
            // *pattern
            dir::Pattern::DereferenceOf { right } => PatternTerm::DereferenceOf {
                pattern: Box::new(self.build_pattern_term(*right, tree)?),
            },
            // name, name: pattern
            dir::Pattern::Binding { pattern, .. } => PatternTerm::Binding {
                symbol: self.declaration_symbol(id.into_any()),
                pattern: pattern
                    .and_then(|pattern| self.build_pattern_term(pattern, tree).map(Box::new)),
            },
            // value
            dir::Pattern::Expression { value } => PatternTerm::Expression {
                value: self.intern_local_type_variable(*value),
            },
            // start..end
            dir::Pattern::Range {
                start,
                end,
                end_kind,
            } => PatternTerm::Range {
                start: start.map(|start| self.intern_local_type_variable(start)),
                end: end.map(|end| self.intern_local_type_variable(end)),
                end_kind: *end_kind,
            },
            // value is T
            dir::Pattern::TypeExpression { value } => PatternTerm::Type {
                ty: self.intern_local_type_variable(*value),
            },
            // [a, b]
            dir::Pattern::Tuple { fields } => PatternTerm::Tuple {
                fields: fields
                    .iter()
                    .filter_map(|field| self.build_pattern_field_term(*field, tree))
                    .collect(),
            },
            // T(a, b)
            dir::Pattern::TaggedTuple { ty, fields } => PatternTerm::TaggedTuple {
                ty: self.intern_local_type_variable(*ty),
                fields: fields
                    .iter()
                    .filter_map(|field| self.build_pattern_field_term(*field, tree))
                    .collect(),
            },
            // [...items]
            dir::Pattern::Sequence { fields } => PatternTerm::Sequence {
                fields: fields
                    .iter()
                    .filter_map(|field| self.build_pattern_field_term(*field, tree))
                    .collect(),
            },
            // { name }
            dir::Pattern::Object { fields } => PatternTerm::Object {
                fields: fields
                    .iter()
                    .filter_map(|field| self.build_pattern_field_term(*field, tree))
                    .collect(),
            },
            // T { name }
            dir::Pattern::TaggedObject { ty, fields } => PatternTerm::TaggedObject {
                ty: self.intern_local_type_variable(*ty),
                fields: fields
                    .iter()
                    .filter_map(|field| self.build_pattern_field_term(*field, tree))
                    .collect(),
            },
            // a | b
            dir::Pattern::Union { patterns } => PatternTerm::Union {
                patterns: patterns
                    .iter()
                    .filter_map(|pattern| self.build_pattern_term(*pattern, tree))
                    .collect(),
            },
        };

        Some(term)
    }

    /// Return one pattern field term from syntax.
    fn build_pattern_field_term(
        &mut self,
        id: dir::LocalNodeId<dir::PatternField>,
        tree: &dir::Tree,
    ) -> Option<PatternFieldTerm> {
        let term = match tree.get(id) {
            // { name: pattern }
            dir::PatternField::Named { name, pattern, .. } => PatternFieldTerm::Named {
                key: name.static_key(),
                pattern: pattern.and_then(|pattern| self.build_pattern_term(pattern, tree)),
            },
            // { [key]: pattern }
            dir::PatternField::Computed { key, pattern } => PatternFieldTerm::Computed {
                key: self.intern_local_type_variable(*key),
                pattern: self.build_pattern_term(*pattern, tree)?,
            },
            // [pattern]
            dir::PatternField::Positional { pattern } => PatternFieldTerm::Positional {
                pattern: self.build_pattern_term(*pattern, tree)?,
            },
            // { ...pattern }
            dir::PatternField::Spread { pattern } => PatternFieldTerm::Spread {
                pattern: pattern.and_then(|pattern| self.build_pattern_term(pattern, tree)),
            },
            // [,]
            dir::PatternField::Elision => PatternFieldTerm::Elision,
        };

        Some(term)
    }

    /// Return one assignment pattern term from syntax.
    pub(in crate::check) fn build_assign_pattern_term(
        &mut self,
        id: dir::LocalNodeId<dir::AssignPattern>,
        tree: &dir::Tree,
    ) -> Option<AssignPatternTerm> {
        let term = match tree.get(id) {
            // target
            dir::AssignPattern::Expression { value } => AssignPatternTerm::Expression {
                place: self.intern_local_type_variable(*value),
            },
            // target = value
            dir::AssignPattern::Assign { pattern, value } => AssignPatternTerm::Assign {
                pattern: Box::new(self.build_assign_pattern_term(*pattern, tree)?),
                value: self.intern_local_type_variable(*value),
            },
            // [a, b]
            dir::AssignPattern::Sequence { fields } => AssignPatternTerm::Sequence {
                fields: fields
                    .iter()
                    .filter_map(|field| self.build_assign_pattern_field_term(*field, tree))
                    .collect(),
            },
            // { a, b }
            dir::AssignPattern::Object { fields } => AssignPatternTerm::Object {
                fields: fields
                    .iter()
                    .filter_map(|field| self.build_assign_pattern_field_term(*field, tree))
                    .collect(),
            },
        };

        Some(term)
    }

    /// Return one assignment pattern field term from syntax.
    fn build_assign_pattern_field_term(
        &mut self,
        id: dir::LocalNodeId<dir::AssignPatternField>,
        tree: &dir::Tree,
    ) -> Option<AssignPatternFieldTerm> {
        let term = match tree.get(id) {
            // { name: pattern }
            dir::AssignPatternField::Named { name, pattern, .. } => AssignPatternFieldTerm::Named {
                key: name.static_key(),
                pattern: pattern.and_then(|pattern| self.build_assign_pattern_term(pattern, tree)),
            },
            // { [key]: pattern }
            dir::AssignPatternField::Computed { key, pattern } => {
                AssignPatternFieldTerm::Computed {
                    key: self.intern_local_type_variable(*key),
                    pattern: self.build_assign_pattern_term(*pattern, tree)?,
                }
            }
            // [pattern]
            dir::AssignPatternField::Positional { pattern } => AssignPatternFieldTerm::Positional {
                pattern: self.build_assign_pattern_term(*pattern, tree)?,
            },
            // { ...pattern }
            dir::AssignPatternField::Spread { pattern } => AssignPatternFieldTerm::Spread {
                pattern: pattern.and_then(|pattern| self.build_assign_pattern_term(pattern, tree)),
            },
            // [,]
            dir::AssignPatternField::Elision => AssignPatternFieldTerm::Elision,
        };

        Some(term)
    }
}
