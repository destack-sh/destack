use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CheckError, CheckState, Condition, Decision, MatchCase, Obligation, Origin, TypeOperand,
};

impl CheckState<'_> {
    /// Push an obligation for one match expression to cover every known value.
    pub(in crate::check) fn push_exhaustive_match_obligation(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        value: TypeOperand,
        cases: Vec<MatchCase>,
        condition: Condition,
    ) {
        let obligation = Obligation::ExhaustiveMatch {
            source: source.into_global(module),
            value,
            cases,
            condition,
        };

        self.push_obligation(obligation);
    }

    /// Check whether one match covers every known selector value.
    pub(in crate::check) fn check_match_exhaustive(
        &mut self,
        source: dir::GlobalNodeIdAny,
        value: TypeOperand,
        cases: &[MatchCase],
    ) -> CompilerResult<Option<CheckError>> {
        let decision = self.decide_match_cases_cover_operand(Origin::Node(source), cases, value)?;
        let diagnostic = match decision {
            Decision::Yes => return Ok(None),
            Decision::No => {
                let (module, anchor) = self.source_anchor(source);

                CheckError::NonExhaustivePattern { anchor, module }
            }
            Decision::Undecidable => {
                let (module, anchor) = self.source_anchor(source);

                CheckError::CannotSolve { anchor, module }
            }
        };

        Ok(Some(diagnostic))
    }
}
