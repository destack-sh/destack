use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CheckError, CheckState, Condition, Decision, Obligation, Origin, PatternTerm, TermId,
    TypeOperand,
};

impl CheckState<'_> {
    /// Push an obligation for one binding pattern to cover its matched value type.
    pub(in crate::check) fn push_irrefutable_pattern_obligation(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        pattern: TermId<PatternTerm>,
        value: TypeOperand,
        condition: Condition,
    ) {
        let obligation = Obligation::IrrefutablePattern {
            source: source.into_global(module),
            pattern,
            value,
            condition,
        };

        self.push_obligation(obligation);
    }
}

impl CheckState<'_> {
    /// Check whether one pattern is irrefutable for its matched value type.
    pub(in crate::check) fn check_irrefutable_pattern(
        &mut self,
        source: dir::GlobalNodeIdAny,
        pattern: TermId<PatternTerm>,
        value: TypeOperand,
    ) -> CompilerResult<Option<CheckError>> {
        let decision = self.decide_pattern_covers_operand(Origin::Node(source), pattern, value)?;
        let diagnostic = match decision {
            Decision::Yes => return Ok(None),
            Decision::No => {
                let (module, anchor) = self.source_anchor(source);

                CheckError::RefutablePattern { anchor, module }
            }
            Decision::Undecidable => {
                let (module, anchor) = self.source_anchor(source);

                CheckError::CannotSolve { anchor, module }
            }
        };

        Ok(Some(diagnostic))
    }
}
