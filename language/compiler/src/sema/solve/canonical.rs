use smallvec::SmallVec;
use tspp_dir as dir;

use crate::CompilerResult;
use crate::sema::{Callee, CheckState, Goal, GoalKey, Origin};

impl CheckState<'_> {
    /// Key one goal by its closed operands and assuming scope, none while an operand is open.
    pub(in crate::sema) fn goal_key(
        &mut self,
        origin: Origin,
        goal: Goal,
        operands: &[dir::GlobalTypeId],
    ) -> CompilerResult<Option<GoalKey>> {
        // resolve the operands, keying closed ones alone
        let mut resolved = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        let mut flags = dir::TypeFlags::EMPTY;
        for operand in operands {
            let operand = self.shallow_resolve(*operand)?;
            let operand_flags = self.type_flags(operand)?;
            if operand_flags.has_variable() {
                return Ok(None);
            }
            flags |= operand_flags;
            resolved.push(operand);
        }

        // key assumed operands under their assuming scope
        let Some(scope) = self.decision_scope(origin, &resolved)? else {
            return Ok(None);
        };
        let operands = self.intern_type_ids(&resolved)?;

        Ok(Some(GoalKey {
            goal,
            operands,
            scope,
        }))
    }

    /// Key one selection goal over its callee, expectation, and operands.
    pub(in crate::sema) fn selection_goal(
        &mut self,
        origin: Origin,
        callee: Callee,
        expected: Option<dir::GlobalTypeId>,
        operands: &[dir::GlobalTypeId],
    ) -> CompilerResult<Option<GoalKey>> {
        // lead the operand list with the expectation when one exists
        let mut keyed = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        keyed.extend(expected);
        keyed.extend_from_slice(operands);
        let goal = Goal::Selection {
            callee,
            expected: expected.is_some(),
        };

        self.goal_key(origin, goal, &keyed)
    }

    /// Return the scope closed operands decide under.
    pub(in crate::sema) fn decision_scope(
        &mut self,
        origin: Origin,
        operands: &[dir::GlobalTypeId],
    ) -> CompilerResult<Option<Option<dir::GlobalGenericTemplateId>>> {
        let mut flags = dir::TypeFlags::EMPTY;
        for operand in operands {
            flags |= self.type_flags(*operand)?;
        }
        if !flags.has_parameter() && !flags.has_this() {
            return Ok(Some(None));
        }
        if self.is_declaring() {
            return Ok(None);
        }

        Ok(Some(self.origin_scope(origin)?))
    }
}
