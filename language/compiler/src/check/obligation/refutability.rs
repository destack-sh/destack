use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckError, CheckState, Condition, Obligation, Origin, PatternTerm, TermId, TypeOperand,
};

impl CheckState<'_> {
    /// Constrain one binding pattern to cover its matched value type.
    pub(in crate::check) fn constrain_irrefutable_pattern(
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

    /// Check whether one pattern is irrefutable for its matched value type.
    pub(in crate::check) fn check_irrefutable_pattern(
        &mut self,
        source: dir::GlobalNodeIdAny,
        pattern: TermId<PatternTerm>,
        value: TypeOperand,
    ) -> CompilerResult<Option<CheckError>> {
        let decision = self.decide_pattern_covers_operand(Origin::Node(source), pattern, value)?;
        let diagnostic = match decision {
            Answer::Ready(true) => return Ok(None),
            Answer::Ready(false) => {
                let (module, anchor) = self.source_anchor(source);

                CheckError::RefutablePattern { anchor, module }
            }
            Answer::Pending(_) => {
                let (module, anchor) = self.source_anchor(source);

                CheckError::CannotSolve { anchor, module }
            }
        };

        Ok(Some(diagnostic))
    }
}
