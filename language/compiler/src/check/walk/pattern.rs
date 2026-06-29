use destack_dir as dir;

use crate::CompilerResult;
use crate::check::WalkState;

impl WalkState<'_, '_> {
    /// Walk one pattern.
    ///
    /// Pattern inputs are supplied by declarators, parameters, matches,
    /// catches, and parent pattern projections.
    ///
    /// Example:
    /// ```ds
    /// Some({ name }) if name is string
    /// ```
    pub(in crate::check) fn walk_pattern(
        &mut self,
        id: dir::LocalNodeId<dir::Pattern>,
        pattern: &dir::Pattern,
    ) -> CompilerResult<()> {
        if !self.decide_decorated_presence(id.into_any())? {
            return Ok(());
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
                self.walk_pattern(*pattern, self.tree.get(*pattern))?;
            }
            // pattern = value
            dir::Pattern::Default { pattern, value } => {
                self.walk_pattern(*pattern, self.tree.get(*pattern))?;

                // check pattern default in selector context
                let before_value = self.fork_flow();
                self.walk_expression(*value, self.tree.get(*value), None)?;
                self.restore_flow(before_value);
            }
            // name: pattern
            dir::Pattern::Binding {
                pattern: Some(pattern),
                ..
            } => {
                self.walk_pattern(*pattern, self.tree.get(*pattern))?;
            }
            // name
            dir::Pattern::Binding { pattern: None, .. } => {}
            // value
            dir::Pattern::Expression { value } => {
                // check value pattern in selector context
                let before_value = self.fork_flow();
                self.walk_expression(*value, self.tree.get(*value), None)?;
                self.restore_flow(before_value);
            }
            // start..end
            dir::Pattern::Range { start, end, .. } => {
                // check range bound in selector context
                if let Some(start) = *start {
                    let before_start = self.fork_flow();
                    self.walk_expression(start, self.tree.get(start), None)?;
                    self.restore_flow(before_start);
                }
                // check range bound in selector context
                if let Some(end) = *end {
                    let before_end = self.fork_flow();
                    self.walk_expression(end, self.tree.get(end), None)?;
                    self.restore_flow(before_end);
                }
            }
            // [a, b], [...items], { name }
            dir::Pattern::Tuple { fields } => {
                let fields = fields.clone();

                for field in fields {
                    self.walk_pattern_field(field, self.tree.get(field))?;
                }
            }
            // [a, b], [...items], { name }
            dir::Pattern::Sequence { fields } | dir::Pattern::Object { fields } => {
                let fields = fields.clone();
                for field in fields {
                    self.walk_pattern_field(field, self.tree.get(field))?;
                }
            }
            // T(a, b), T { name }
            dir::Pattern::NominalTuple { ty, fields } | dir::Pattern::NominalObject { ty, fields } => {
                let fields = fields.clone();
                self.walk_type_expression(*ty)?;
                for field in fields {
                    self.walk_pattern_field(field, self.tree.get(field))?;
                }
            }
            // a | b
            dir::Pattern::Union { patterns } => {
                let patterns = patterns.clone();
                for pattern in patterns {
                    self.walk_pattern(pattern, self.tree.get(pattern))?;
                }
            }
        }

        Ok(())
    }

    /// Walk one pattern field.
    ///
    /// Example:
    /// ```ds
    /// { name: pattern }
    /// ```
    pub(in crate::check) fn walk_pattern_field(
        &mut self,
        id: dir::LocalNodeId<dir::PatternField>,
        field: &dir::PatternField,
    ) -> CompilerResult<()> {
        if !self.decide_decorated_presence(id.into_any())? {
            return Ok(());
        }

        match field {
            // { name: pattern }, { name }
            dir::PatternField::Named { pattern, .. } => {
                if let Some(pattern) = *pattern {
                    self.walk_pattern(pattern, self.tree.get(pattern))?;
                }
            }
            // { [key]: pattern }
            dir::PatternField::Computed { key, pattern } => {
                // check computed key in selector context
                let before_key = self.fork_flow();
                self.walk_expression(*key, self.tree.get(*key), None)?;
                self.restore_flow(before_key);

                self.walk_pattern(*pattern, self.tree.get(*pattern))?;
            }
            // [pattern]
            dir::PatternField::Positional { pattern } => {
                self.walk_pattern(*pattern, self.tree.get(*pattern))?;
            }
            // { ...pattern }
            dir::PatternField::Spread { pattern } => {
                if let Some(pattern) = *pattern {
                    self.walk_pattern(pattern, self.tree.get(pattern))?;
                }
            }
            // [,]
            dir::PatternField::Elision => {}
        }

        Ok(())
    }
}
