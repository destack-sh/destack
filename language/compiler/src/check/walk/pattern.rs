use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Origin, Relation, WalkState};

impl WalkState<'_, '_> {
    /// Walk one pattern.
    ///
    /// Every pattern node opens its own type. Matched values flow in at
    /// the match site, structural constraints carry components into the
    /// nested pattern holes, and selection records the pattern meaning
    /// once the scrutinee closes.
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
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any())? else {
            return Ok(());
        };

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
                let pattern = *pattern;
                self.walk_pattern(pattern, self.tree.get(pattern))?;

                // forward the inner pattern type through the wrapper
                let inner = self.node_type(pattern)?;
                self.declare_node_type(id, inner)?;
            }
            // pattern = value
            dir::Pattern::Assign { pattern, value } => {
                let (pattern, value) = (*pattern, *value);
                self.walk_pattern(pattern, self.tree.get(pattern))?;

                // check pattern default in selector context
                let before_value = self.fork_flow();
                self.walk_expression(value, self.tree.get(value))?;
                self.restore_flow(before_value);

                // defaults flow into the pattern hole
                let inner = self.node_type(pattern)?;
                let default = self.node_type(value)?;
                let origin = Origin::Node(value.into_global_any(self.module));
                self.relate_type(origin, Relation::Assignable, default, inner);
                self.declare_node_type(id, inner)?;
            }
            // name: pattern
            dir::Pattern::Binding {
                pattern: Some(pattern),
                ..
            } => {
                let pattern = *pattern;
                self.walk_pattern(pattern, self.tree.get(pattern))?;

                // bind the name to the inner pattern type
                let inner = self.node_type(pattern)?;
                self.declare_node_type(id, inner)?;
                self.declare_binding_pattern_symbol(id, inner)?;
            }
            // name
            dir::Pattern::Binding { pattern: None, .. } => {
                let ty = self.node_type(id)?;
                self.declare_binding_pattern_symbol(id, ty)?;
            }
            // value
            dir::Pattern::Expression { value } => {
                let value = *value;

                // check value pattern in selector context
                let before_value = self.fork_flow();
                self.walk_expression(value, self.tree.get(value))?;
                self.restore_flow(before_value);

                let expected = self.node_type(value)?;
                self.declare_node_type(id, expected)?;
            }
            // start..end
            dir::Pattern::Range { start, end, .. } => {
                let (start, end) = (*start, *end);
                // check range bound in selector context
                if let Some(start) = start {
                    let before_start = self.fork_flow();
                    self.walk_expression(start, self.tree.get(start))?;
                    self.restore_flow(before_start);
                }
                // check range bound in selector context
                if let Some(end) = end {
                    let before_end = self.fork_flow();
                    self.walk_expression(end, self.tree.get(end))?;
                    self.restore_flow(before_end);
                }
            }
            // value is T
            dir::Pattern::TypeExpression { value } => {
                let value = *value;
                let expected = self.walk_type_expression(value)?;
                self.declare_node_type(id, expected)?;
            }
            // [a, b], [...items], { name }
            dir::Pattern::Tuple { fields }
            | dir::Pattern::Sequence { fields }
            | dir::Pattern::Object { fields } => {
                let fields = fields.clone();
                for field in fields {
                    self.walk_pattern_field(field, self.tree.get(field))?;
                }
            }
            // T(a, b), T { name }
            dir::Pattern::Newtype { ty, fields } | dir::Pattern::NominalObject { ty, fields } => {
                let (ty, fields) = (*ty, fields.clone());
                let tag = self.walk_type_expression(ty)?;
                for field in fields {
                    self.walk_pattern_field(field, self.tree.get(field))?;
                }

                // the pattern matches values of its nominal tag
                self.declare_node_type(id, tag)?;
            }
            // a | b
            dir::Pattern::Union { patterns } => {
                let patterns = patterns.clone();
                for pattern in patterns {
                    self.walk_pattern(pattern, self.tree.get(pattern))?;
                }
            }
        }

        // selection resolves the pattern once the scrutinee closes
        self.node_type(id)?;
        self.queue_select(id.into_global_any(self.module));

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
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any())? else {
            return Ok(());
        };

        match field {
            // { name: pattern }, { name }
            dir::PatternField::Named { pattern, .. } => {
                let pattern = *pattern;
                if let Some(pattern) = pattern {
                    self.walk_pattern(pattern, self.tree.get(pattern))?;
                } else {
                    // shorthand fields bind their own name
                    let ty = self.node_type(id)?;
                    self.declare_binding_pattern_symbol(id, ty)?;
                }
            }
            // { [key]: pattern }, [pattern]
            dir::PatternField::Computed { pattern, .. }
            | dir::PatternField::Positional { pattern } => {
                let pattern = *pattern;
                self.walk_pattern(pattern, self.tree.get(pattern))?;
            }
            // { ...pattern }
            dir::PatternField::Spread { pattern } => {
                let pattern = *pattern;
                if let Some(pattern) = pattern {
                    self.walk_pattern(pattern, self.tree.get(pattern))?;
                }
            }
            // [,]
            dir::PatternField::Elision => {}
        }

        Ok(())
    }

    /// Declare the symbol bound at one pattern node.
    fn declare_binding_pattern_symbol<T: dir::Node>(
        &mut self,
        id: dir::LocalNodeId<T>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let Some(symbol) = self
            .check
            .module(self.module)
            .declaration_symbol(id.into_any())
        else {
            return Ok(());
        };
        self.declare_symbol_type(symbol, ty)?;

        Ok(())
    }
}
