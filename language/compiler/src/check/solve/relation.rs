use crate::CompilerResult;
use crate::check::{
    CheckState, Origin, StaticOperand, StaticRelation, StaticTerm, TypeOperand, TypeRelation,
    TypeTerm,
};

impl CheckState<'_> {
    /// Reduce one type relation.
    pub(in crate::check) fn reduce_type_relation(
        &mut self,
        origin: Origin,
        relation: TypeRelation,
        left: impl Into<TypeOperand>,
        right: impl Into<TypeOperand>,
    ) -> CompilerResult<()> {
        let left = left.into();
        let right = right.into();

        match relation {
            TypeRelation::Equal => self.reduce_type_equality(origin, left, right),
            TypeRelation::Assignable | TypeRelation::Castable => {
                self.reduce_contextual_type_assignability(origin, left, right)
            }
            TypeRelation::Satisfies | TypeRelation::Extends | TypeRelation::Implements => {
                self.reduce_type_assignability(origin, left, right)
            }
        }
    }

    /// Reduce one type equality relation.
    pub(in crate::check) fn reduce_type_equality(
        &mut self,
        origin: Origin,
        left: impl Into<TypeOperand>,
        right: impl Into<TypeOperand>,
    ) -> CompilerResult<()> {
        let left = left.into();
        let right = right.into();
        self.insert_equal_type_bounds(left, right)?;

        let left_value = self.reduce_type_operand(origin, left)?;
        let right_value = self.reduce_type_operand(origin, right)?;

        match (left_value, right_value) {
            // push left type into right operand
            (Some(term), None) => {
                if let Some(right) = right.variable() {
                    self.expect_type_operand(origin, right, term)?;
                }
            }
            // push right type into left operand
            (None, Some(term)) => {
                if let Some(left) = left.variable() {
                    self.expect_type_operand(origin, left, term)?;
                }
            }
            // validate reduced terms
            (Some(left), Some(right)) => {
                self.constrain_solved_type_operands_equal(origin, left, right)?
            }
            // wait for operands
            (None, None) => {}
        }

        Ok(())
    }

    /// Reduce one contextual assignability relation.
    pub(in crate::check) fn reduce_contextual_type_assignability(
        &mut self,
        origin: Origin,
        source: impl Into<TypeOperand>,
        target: impl Into<TypeOperand>,
    ) -> CompilerResult<()> {
        let source = source.into();
        let target = target.into();
        self.insert_assignable_type_bounds(source, target)?;
        let target_value = self.reduce_type_operand(origin, target)?;
        if let Some(target_value) = target_value {
            self.expect_type_operand_assignability(origin, source, target_value)?;
        }
        let source_value = self.reduce_type_operand(origin, source)?;

        match (source_value, target_value) {
            // validate reduced terms
            (Some(source_term), Some(target_term)) => {
                self.constrain_solved_type_operands_assignable(origin, source_term, target_term)?;
            }
            // wait for operands
            (Some(_), None) | (None, Some(_)) | (None, None) => {}
        }

        Ok(())
    }

    /// Reduce one assignability relation without contextual backpressure.
    pub(in crate::check) fn reduce_type_assignability(
        &mut self,
        origin: Origin,
        source: impl Into<TypeOperand>,
        target: impl Into<TypeOperand>,
    ) -> CompilerResult<()> {
        let source = source.into();
        let target = target.into();
        self.insert_assignable_type_bounds(source, target)?;
        let target_value = self.reduce_type_operand(origin, target)?;
        let source_value = self.reduce_type_operand(origin, source)?;

        match (source_value, target_value) {
            (Some(source_term), Some(target_term)) => {
                self.constrain_solved_type_operands_assignable(origin, source_term, target_term)?;
            }
            (Some(_), None) | (None, Some(_)) | (None, None) => {}
        }

        Ok(())
    }

    /// Return the reduced operand for one type operand when available.
    pub(in crate::check) fn reduce_type_operand(
        &mut self,
        origin: Origin,
        operand: TypeOperand,
    ) -> CompilerResult<Option<TypeOperand>> {
        let mut operand = operand;
        let mut seen = Vec::new();

        loop {
            let Some(next) = self.reduce_type_operand_once(origin, operand)? else {
                return Ok(None);
            };
            if next == operand {
                return Ok(Some(operand));
            }
            if seen.contains(&next) {
                return Err(self.circular_type_error(origin).into());
            }

            seen.push(operand);
            operand = next;
        }
    }

    /// Return one reduction step for one type operand.
    fn reduce_type_operand_once(
        &mut self,
        origin: Origin,
        operand: TypeOperand,
    ) -> CompilerResult<Option<TypeOperand>> {
        let operand = match operand {
            TypeOperand::Variable(variable) => {
                let Some(operand) = self.variable_type_solution_operand(variable) else {
                    return Ok(None);
                };

                operand
            }
            TypeOperand::Term(term) => return self.reduce_type_term_by_id(origin, term),
            TypeOperand::Type(_) => operand,
        };

        Ok(Some(operand))
    }

    /// Reduce one static relation.
    pub(in crate::check) fn reduce_static_relation(
        &mut self,
        relation: StaticRelation,
        left: impl Into<StaticOperand>,
        right: impl Into<StaticOperand>,
    ) -> CompilerResult<()> {
        let left = left.into();
        let right = right.into();

        match relation {
            StaticRelation::Equal => self.reduce_static_equality(left, right),
            StaticRelation::Assignable => self.reduce_static_assignability(left, right),
        }
    }

    /// Reduce one static equality relation.
    pub(in crate::check) fn reduce_static_equality(
        &mut self,
        left: impl Into<StaticOperand>,
        right: impl Into<StaticOperand>,
    ) -> CompilerResult<()> {
        let left = left.into();
        let right = right.into();

        self.insert_equal_static_bounds(left, right)
    }

    /// Reduce one static assignability relation.
    pub(in crate::check) fn reduce_static_assignability(
        &mut self,
        source: impl Into<StaticOperand>,
        target: impl Into<StaticOperand>,
    ) -> CompilerResult<()> {
        let source = source.into();
        let target = target.into();

        self.insert_assignable_static_bounds(source, target)
    }

    /// Return the solved term for one type operand when available.
    pub(in crate::check) fn type_operand_term(
        &self,
        operand: TypeOperand,
    ) -> CompilerResult<Option<TypeTerm>> {
        let term = match operand {
            TypeOperand::Variable(variable) => self.type_solution(variable)?,
            TypeOperand::Term(term) => Some(self.inference.term(term).clone()),
            TypeOperand::Type(ty) => Some(TypeTerm::Type(ty)),
        };

        Ok(term)
    }

    /// Return the solved term for one static operand when available.
    pub(in crate::check) fn static_operand_term(
        &self,
        operand: StaticOperand,
    ) -> CompilerResult<Option<StaticTerm>> {
        let term = match operand {
            StaticOperand::Variable(variable) => self.static_solution(variable)?,
            StaticOperand::Term(term) => Some(self.inference.term(term).clone()),
            StaticOperand::Static(value) => Some(StaticTerm::Static(value)),
        };

        Ok(term)
    }

    /// Insert equality bounds between two type operands.
    fn insert_equal_type_bounds(
        &mut self,
        left: TypeOperand,
        right: TypeOperand,
    ) -> CompilerResult<()> {
        self.insert_assignable_type_bounds(left, right)?;
        self.insert_assignable_type_bounds(right, left)?;

        Ok(())
    }

    /// Insert equality bounds between two static operands.
    fn insert_equal_static_bounds(
        &mut self,
        left: StaticOperand,
        right: StaticOperand,
    ) -> CompilerResult<()> {
        self.insert_assignable_static_bounds(left, right)?;
        self.insert_assignable_static_bounds(right, left)?;

        Ok(())
    }

    /// Insert assignability bounds between two type operands.
    fn insert_assignable_type_bounds(
        &mut self,
        source: TypeOperand,
        target: TypeOperand,
    ) -> CompilerResult<()> {
        if source == target {
            return Ok(());
        }

        // wake users of the constrained target
        if let Some(target) = target.variable() {
            self.insert_lower_type_bound(target, source)?;
        }

        // wake users of the constrained source
        if let Some(source) = source.variable() {
            self.insert_upper_type_bound(source, target)?;
        }

        Ok(())
    }

    /// Insert assignability bounds between two static operands.
    fn insert_assignable_static_bounds(
        &mut self,
        source: StaticOperand,
        target: StaticOperand,
    ) -> CompilerResult<()> {
        if source == target {
            return Ok(());
        }

        // wake users of the constrained target
        if let Some(target) = target.variable() {
            self.insert_lower_static_bound(target, source)?;
        }

        // wake users of the constrained source
        if let Some(source) = source.variable() {
            self.insert_upper_static_bound(source, target)?;
        }

        Ok(())
    }
}
