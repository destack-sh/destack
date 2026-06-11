use crate::CompilerResult;
use crate::check::{
    Answer, CheckEvent, CheckState, Dependency, Origin, StaticOperand, StaticRelation, TypeOperand,
    TypeRelation,
};

impl CheckState<'_> {
    /// Solve one type relation.
    pub(in crate::check) fn solve_type_relation(
        &mut self,
        origin: Origin,
        relation: TypeRelation,
        left: impl Into<TypeOperand>,
        right: impl Into<TypeOperand>,
    ) -> CompilerResult<Answer<()>> {
        let left = left.into();
        let right = right.into();

        match relation {
            TypeRelation::Equal => self.solve_type_equality(origin, left, right),
            TypeRelation::Assignable | TypeRelation::Castable => {
                self.solve_directed_type_relation(origin, relation, left, right)
            }
            TypeRelation::Satisfies | TypeRelation::Extends | TypeRelation::Implements => {
                self.constrain_type_assignability(origin, left, right)?;

                Ok(Answer::Ready(()))
            }
        }
    }

    /// Solve one type equality relation.
    pub(in crate::check) fn solve_type_equality(
        &mut self,
        origin: Origin,
        left: impl Into<TypeOperand>,
        right: impl Into<TypeOperand>,
    ) -> CompilerResult<Answer<()>> {
        let left = left.into();
        let right = right.into();

        // route computed definitions into the left result
        if let Some(variable) = left.variable()
            && let Some(answer) = self.expect_type_operand_bound(origin, variable, right)?
        {
            match answer {
                Answer::Ready(()) => return Ok(Answer::Ready(())),
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            }
        }

        // route computed definitions into the right result
        if let Some(variable) = right.variable()
            && let Some(answer) = self.expect_type_operand_bound(origin, variable, left)?
        {
            match answer {
                Answer::Ready(()) => return Ok(Answer::Ready(())),
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            }
        }

        self.bound_equal_types(origin, left, right)?;
        if self.type_operand_is_open(left) || self.type_operand_is_open(right) {
            return Ok(Answer::pending(
                left.dependencies(self)
                    .into_iter()
                    .chain(right.dependencies(self)),
            ));
        }

        self.solve_type_operands(origin, TypeRelation::Equal, left, right)
    }

    /// Solve one directed type relation.
    pub(in crate::check) fn solve_directed_type_relation(
        &mut self,
        origin: Origin,
        relation: TypeRelation,
        source: impl Into<TypeOperand>,
        target: impl Into<TypeOperand>,
    ) -> CompilerResult<Answer<()>> {
        let source = source.into();
        let target = target.into();

        // route computed definitions into the target result
        if let Some(variable) = target.variable()
            && let Some(answer) = self.expect_type_operand_bound(origin, variable, source)?
        {
            match answer {
                Answer::Ready(()) => return Ok(Answer::Ready(())),
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            }
        }

        self.bound_assignable_types(origin, source, target)?;
        if self.type_operand_is_open(source) || self.type_operand_is_open(target) {
            return Ok(Answer::pending(
                source
                    .dependencies(self)
                    .into_iter()
                    .chain(target.dependencies(self)),
            ));
        }

        self.solve_type_operands(origin, relation, source, target)
    }

    /// Constrain one type assignability relation without checking the source definition.
    pub(in crate::check) fn constrain_type_assignability(
        &mut self,
        origin: Origin,
        source: impl Into<TypeOperand>,
        target: impl Into<TypeOperand>,
    ) -> CompilerResult<()> {
        let source = source.into();
        let target = target.into();
        self.bound_assignable_types(origin, source, target)?;
        if self.type_operand_is_open(source) || self.type_operand_is_open(target) {
            return Ok(());
        }

        self.solve_type_operands(origin, TypeRelation::Assignable, source, target)?;

        Ok(())
    }

    /// Solve one type relation after its variable bounds have been written.
    fn solve_type_operands(
        &mut self,
        origin: Origin,
        relation: TypeRelation,
        left: TypeOperand,
        right: TypeOperand,
    ) -> CompilerResult<Answer<()>> {
        let left_reduction = self.reduce_type_operand(origin, left)?;
        let right_reduction = self.reduce_type_operand(origin, right)?;

        let (left, right) = match (left_reduction, right_reduction) {
            // reduce both operands
            (Answer::Ready(reduced_left), Answer::Ready(reduced_right)) => {
                if reduced_left != left || reduced_right != right {
                    self.record_event(CheckEvent::RelationReduce {
                        origin,
                        relation,
                        left,
                        right,
                        reduced_left: Some(reduced_left),
                        reduced_right: Some(reduced_right),
                    });
                }

                (reduced_left, reduced_right)
            }
            // wait for the left operand
            (Answer::Pending(blockers), Answer::Ready(_)) => {
                return Ok(Answer::Pending(blockers));
            }
            // wait for the right operand
            (Answer::Ready(_), Answer::Pending(blockers)) => {
                return Ok(Answer::Pending(blockers));
            }
            // wait for both operands
            (Answer::Pending(left), Answer::Pending(right)) => {
                return Ok(Answer::pending(left.into_iter().chain(right)));
            }
        };

        self.solve_closed_type_relation(origin, relation, left, right)
    }

    /// Solve one closed type relation.
    fn solve_closed_type_relation(
        &mut self,
        origin: Origin,
        relation: TypeRelation,
        left: TypeOperand,
        right: TypeOperand,
    ) -> CompilerResult<Answer<()>> {
        match relation {
            TypeRelation::Equal => {
                self.constrain_type_operands_equal(origin, left, right)?;

                Ok(Answer::Ready(()))
            }
            TypeRelation::Assignable
            | TypeRelation::Satisfies
            | TypeRelation::Extends
            | TypeRelation::Implements => {
                self.constrain_type_operands_assignable(origin, left, right)?;

                Ok(Answer::Ready(()))
            }
            TypeRelation::Castable => {
                let answer = self.decide_type_relation(relation, left, right)?;

                Ok(answer.map(|_| ()))
            }
        }
    }

    /// Return one reduction step for one type operand.
    pub(in crate::check) fn reduce_type_operand(
        &mut self,
        origin: Origin,
        operand: TypeOperand,
    ) -> CompilerResult<Answer<TypeOperand>> {
        let operand = match operand {
            TypeOperand::Variable(variable) => {
                let Some(operand) = self.solved_type_operand(variable) else {
                    return Ok(Answer::pending([Dependency::Variable(variable)]));
                };

                operand
            }
            TypeOperand::Term(term) => return self.reduce_type_term_by_id(origin, term),
            TypeOperand::Type(_) => operand,
        };

        Ok(Answer::Ready(operand))
    }

    /// Return whether one type operand is a direct unsolved variable.
    fn type_operand_is_open(&self, operand: TypeOperand) -> bool {
        let TypeOperand::Variable(variable) = operand else {
            return false;
        };

        self.solved_type_operand(variable).is_none()
    }

    /// Solve one static relation.
    pub(in crate::check) fn solve_static_relation(
        &mut self,
        origin: Origin,
        relation: StaticRelation,
        left: impl Into<StaticOperand>,
        right: impl Into<StaticOperand>,
    ) -> CompilerResult<Answer<()>> {
        let left = left.into();
        let right = right.into();

        match relation {
            StaticRelation::Equal => self.solve_static_equality(origin, left, right),
            StaticRelation::Assignable => self.solve_static_assignability(origin, left, right),
        }
    }

    /// Solve one static equality relation.
    pub(in crate::check) fn solve_static_equality(
        &mut self,
        origin: Origin,
        left: impl Into<StaticOperand>,
        right: impl Into<StaticOperand>,
    ) -> CompilerResult<Answer<()>> {
        let left = left.into();
        let right = right.into();

        self.bound_equal_statics(origin, left, right)?;
        self.solve_static_operands(origin, StaticRelation::Equal, left, right)
    }

    /// Solve one static assignability relation.
    pub(in crate::check) fn solve_static_assignability(
        &mut self,
        origin: Origin,
        source: impl Into<StaticOperand>,
        target: impl Into<StaticOperand>,
    ) -> CompilerResult<Answer<()>> {
        let source = source.into();
        let target = target.into();

        self.bound_assignable_statics(origin, source, target)?;
        self.solve_static_operands(origin, StaticRelation::Assignable, source, target)
    }

    /// Solve one static relation after its variable bounds have been written.
    fn solve_static_operands(
        &mut self,
        origin: Origin,
        relation: StaticRelation,
        left: StaticOperand,
        right: StaticOperand,
    ) -> CompilerResult<Answer<()>> {
        let left_reduction = self.reduce_static_operand(origin, left)?;
        let right_reduction = self.reduce_static_operand(origin, right)?;

        let (left, right) = match (left_reduction, right_reduction) {
            // reduce both operands
            (Answer::Ready(left), Answer::Ready(right)) => (left, right),
            // wait for the left operand
            (Answer::Pending(blockers), Answer::Ready(_)) => {
                return Ok(Answer::Pending(blockers));
            }
            // wait for the right operand
            (Answer::Ready(_), Answer::Pending(blockers)) => {
                return Ok(Answer::Pending(blockers));
            }
            // wait for both operands
            (Answer::Pending(left), Answer::Pending(right)) => {
                return Ok(Answer::pending(left.into_iter().chain(right)));
            }
        };

        match self.decide_static_relation(relation, left, right)? {
            Answer::Ready(_) => Ok(Answer::Ready(())),
            Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
        }
    }

    /// Bound two type operands as equal.
    fn bound_equal_types(
        &mut self,
        origin: Origin,
        left: TypeOperand,
        right: TypeOperand,
    ) -> CompilerResult<()> {
        self.bound_assignable_types(origin, left, right)?;
        self.bound_assignable_types(origin, right, left)?;

        Ok(())
    }

    /// Bound two static operands as equal.
    fn bound_equal_statics(
        &mut self,
        origin: Origin,
        left: StaticOperand,
        right: StaticOperand,
    ) -> CompilerResult<()> {
        self.bound_assignable_statics(origin, left, right)?;
        self.bound_assignable_statics(origin, right, left)?;

        Ok(())
    }

    /// Bound one type operand as assignable to another.
    fn bound_assignable_types(
        &mut self,
        _origin: Origin,
        source: TypeOperand,
        target: TypeOperand,
    ) -> CompilerResult<()> {
        if source == target {
            return Ok(());
        }

        // constrain the target variable
        if let Some(target) = target.variable() {
            self.insert_lower_type_bound(target, source)?;
        }

        // constrain the source variable
        if let Some(source) = source.variable() {
            self.insert_upper_type_bound(source, target)?;
        }

        Ok(())
    }

    /// Bound one static operand as assignable to another.
    fn bound_assignable_statics(
        &mut self,
        _origin: Origin,
        source: StaticOperand,
        target: StaticOperand,
    ) -> CompilerResult<()> {
        if source == target {
            return Ok(());
        }

        // constrain the target variable
        if let Some(target) = target.variable() {
            self.insert_lower_static_bound(target, source)?;
        }

        // constrain the source variable
        if let Some(source) = source.variable() {
            self.insert_upper_static_bound(source, target)?;
        }

        Ok(())
    }
}
