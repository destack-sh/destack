use destack_dir as dir;

use crate::check::{WalkState, Widening};
use crate::{CompilerError, CompilerResult};

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
        binding_widening: Option<Widening>,
    ) -> CompilerResult<()> {
        if !self.walk_decorators(id.into_any())? {
            return Ok(());
        }
        self.enter_node(id)?;

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
                self.walk_pattern(*pattern, self.tree.get(*pattern), binding_widening)?;
            }
            // pattern = value
            dir::Pattern::Default { pattern, value } => {
                self.walk_pattern(*pattern, self.tree.get(*pattern), binding_widening)?;

                // check pattern default in selector context
                let before_value = self.fork_flow();
                self.walk_expression(*value, self.tree.get(*value))?;
                self.restore_flow(before_value);
            }
            // name: pattern
            dir::Pattern::Binding {
                pattern: Some(pattern),
                ..
            } => {
                if let Some(widening) = binding_widening {
                    self.allocate_pattern_binding(id.into_any(), widening)?;
                }
                self.walk_pattern(*pattern, self.tree.get(*pattern), binding_widening)?;
            }
            // name
            dir::Pattern::Binding { pattern: None, .. } => {
                if let Some(widening) = binding_widening {
                    self.allocate_pattern_binding(id.into_any(), widening)?;
                }
            }
            // value
            dir::Pattern::Expression { value } => {
                // check value pattern in selector context
                let before_value = self.fork_flow();
                self.walk_expression(*value, self.tree.get(*value))?;
                self.restore_flow(before_value);
            }
            // start..end
            dir::Pattern::Range { start, end, .. } => {
                // check range bound in selector context
                if let Some(start) = *start {
                    let before_start = self.fork_flow();
                    self.walk_expression(start, self.tree.get(start))?;
                    self.restore_flow(before_start);
                }
                // check range bound in selector context
                if let Some(end) = *end {
                    let before_end = self.fork_flow();
                    self.walk_expression(end, self.tree.get(end))?;
                    self.restore_flow(before_end);
                }
            }
            // (a, b), [a, ...items], { name }
            dir::Pattern::Tuple { fields }
            | dir::Pattern::Sequence { fields }
            | dir::Pattern::Object { fields } => {
                let fields = fields.clone();
                for field in fields {
                    self.walk_pattern_field(field, self.tree.get(field), binding_widening)?;
                }
            }
            // T(a, b), T { name }
            dir::Pattern::NominalTuple { ty, fields } | dir::Pattern::NominalObject { ty, fields } => {
                let fields = fields.clone();
                self.walk_construct_type_expression(*ty)?;
                for field in fields {
                    self.walk_pattern_field(field, self.tree.get(field), binding_widening)?;
                }
            }
            // a | b
            dir::Pattern::Union { patterns } => {
                let patterns = patterns.clone();
                for pattern in patterns {
                    self.walk_pattern(pattern, self.tree.get(pattern), binding_widening)?;
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
        binding_widening: Option<Widening>,
    ) -> CompilerResult<()> {
        if !self.walk_decorators(id.into_any())? {
            return Ok(());
        }

        match field {
            // { name: pattern }, { name }
            dir::PatternField::Named { pattern, .. } => {
                if let Some(pattern) = *pattern {
                    self.walk_pattern(pattern, self.tree.get(pattern), binding_widening)?;
                } else if let Some(widening) = binding_widening {
                    self.allocate_pattern_binding(id.into_any(), widening)?;
                }
            }
            // { [key]: pattern }
            dir::PatternField::Computed { key, pattern } => {
                // check computed key in selector context
                let before_key = self.fork_flow();
                self.walk_expression(*key, self.tree.get(*key))?;
                self.restore_flow(before_key);

                self.walk_pattern(*pattern, self.tree.get(*pattern), binding_widening)?;
            }
            // [pattern]
            dir::PatternField::Positional { pattern } => {
                self.walk_pattern(*pattern, self.tree.get(*pattern), binding_widening)?;
            }
            // { ...pattern }
            dir::PatternField::Rest { pattern } => {
                if let Some(pattern) = *pattern {
                    self.walk_pattern(pattern, self.tree.get(pattern), binding_widening)?;
                }
            }
            // [,]
            dir::PatternField::Elision => {}
        }

        Ok(())
    }

    /// Allocate the type slot for one declaration pattern binding.
    fn allocate_pattern_binding(
        &mut self,
        source: dir::LocalNodeIdAny,
        widening: Widening,
    ) -> CompilerResult<()> {
        let symbol = self
            .check
            .module(self.module)
            .declaration_symbol(source)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("declaration pattern {source:?} has no symbol"),
            })?;
        self.binding_type_slot(symbol, widening)?;

        Ok(())
    }
}
