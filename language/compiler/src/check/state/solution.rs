use crate::check::{
    CheckEvent, CheckState, Dump, DumpContext, Solution, StaticOperand, TypeOperand, VariableId,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Set one complete variable solution.
    pub(in crate::check) fn set_variable_solution(
        &mut self,
        variable: VariableId,
        solution: Solution,
    ) -> CompilerResult<()> {
        if let Some(existing) = self.inference.variable_solution(variable) {
            let module = variable.module;
            let variable = self.dump_in_module(module, &variable);
            let existing = self.dump_in_module(module, &existing);
            let solution = self.dump_in_module(module, &solution);

            return Err(CompilerError::Internal {
                message: format!(
                    "check variable {variable} already has solution {existing}, got {solution}"
                ),
            });
        }

        self.inference.write_variable_solution(variable, solution);

        self.record_event(CheckEvent::SolutionSet {
            variable,
            kind: self.variable(variable).kind,
            value: solution.trace_operand(),
        });

        Ok(())
    }

    /// Return the solved type operand for one variable.
    pub(in crate::check) fn solved_type_operand(
        &self,
        variable: VariableId,
    ) -> Option<TypeOperand> {
        match self.inference.variable_solution(variable) {
            Some(Solution::Type(solution)) => Some(solution.into()),
            Some(Solution::Static(_)) | None => None,
        }
    }

    /// Return the solved static operand for one variable.
    pub(in crate::check) fn solved_static_operand(
        &self,
        variable: VariableId,
    ) -> Option<StaticOperand> {
        match self.inference.variable_solution(variable) {
            Some(Solution::Static(solution)) => Some(solution.into()),
            Some(Solution::Type(_)) | None => None,
        }
    }

    /// Return a solved variable value.
    pub(in crate::check) fn variable_solution(&self, variable: VariableId) -> Option<Solution> {
        self.inference.variable_solution(variable)
    }

    /// Return the resolved type operand for one variable.
    pub(in crate::check) fn resolved_type_variable(
        &self,
        variable: VariableId,
    ) -> Option<TypeOperand> {
        let Some(operand) = self.solved_type_operand(variable) else {
            return None;
        };

        self.resolved_type_operand(operand)
    }

    /// Return the resolved static operand for one variable.
    pub(in crate::check) fn resolved_static_variable(
        &self,
        variable: VariableId,
    ) -> Option<StaticOperand> {
        let Some(operand) = self.solved_static_operand(variable) else {
            return None;
        };

        self.resolved_static_operand(operand)
    }

    /// Return one resolved type operand.
    pub(in crate::check) fn resolved_type_operand(
        &self,
        operand: TypeOperand,
    ) -> Option<TypeOperand> {
        match operand {
            TypeOperand::Variable(variable) => self.resolved_type_variable(variable),
            TypeOperand::Term(_) | TypeOperand::Type(_) => Some(operand),
        }
    }

    /// Return one resolved static operand.
    pub(in crate::check) fn resolved_static_operand(
        &self,
        operand: StaticOperand,
    ) -> Option<StaticOperand> {
        match operand {
            StaticOperand::Variable(variable) => self.resolved_static_variable(variable),
            StaticOperand::Term(_) | StaticOperand::Static(_) => Some(operand),
        }
    }
}

impl Dump for Solution {
    /// Render this solution.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Type(solution) => TypeOperand::from(*solution).dump(context),
            Self::Static(solution) => StaticOperand::from(*solution).dump(context),
        }
    }
}
