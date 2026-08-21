use std::sync::Arc;

use destack_core::FxIndexMap;
use destack_dir as dir;
use destack_dir::TypeFold;
use smallvec::SmallVec;

use crate::sema::solve::canonical::Renaming;
use crate::sema::{
    Answer, Canonical, Cause, CauseKind, Check, CheckState, Hole, Origin, Question, Relation,
    RelationCheck,
};
use crate::{CompilerError, CompilerResult};

/// One decided question's canonical output, instantiated per ask site.
#[derive(Debug, Clone)]
pub(in crate::sema) struct Response<T> {
    /// The decided value.
    pub(in crate::sema) value: T,
    /// The canonical solution per ask hole, absent while one stayed open.
    pub(in crate::sema) solutions: SmallVec<[Option<dir::GlobalTypeId>; 2]>,
    /// The fresh existentials the answer introduces, reopened per site.
    pub(in crate::sema) holes: SmallVec<[Hole; 2]>,
    /// The canonical checks the decision queued, re-registered per site.
    pub(in crate::sema) checks: SmallVec<[RelationCheck; 2]>,
}

impl<T> Response<T> {
    /// Return whether this response is exactly its value.
    pub(in crate::sema) fn is_exact(&self) -> bool {
        self.holes.is_empty()
            && self.checks.is_empty()
            && self.solutions.iter().all(Option::is_none)
    }
}

impl CheckState<'_> {
    /// Remember the first decided response for one canonical question.
    pub(in crate::sema) fn remember_answer<T: dir::TypeFold>(
        &mut self,
        question: &Question,
        canonical: &Canonical,
        checks_from: usize,
        value: T,
        answer: impl FnOnce(Arc<Response<T>>) -> Answer,
    ) -> CompilerResult<()> {
        // keep the first answer decided for this question
        if self.answers.contains_key(question) {
            return Ok(());
        }

        // store the value folded canonical over its ask
        if let Some(response) = self.canonicalize_response(canonical, checks_from, value)? {
            self.answers
                .insert(question.clone(), answer(Arc::new(response)));
        }

        Ok(())
    }

    /// Fold one decision's output canonical over its ask, growing answer holes.
    pub(in crate::sema) fn canonicalize_response<T: dir::TypeFold>(
        &mut self,
        canonical: &Canonical,
        checks_from: usize,
        mut value: T,
    ) -> CompilerResult<Option<Response<T>>> {
        let mut renaming = Renaming::seeded(canonical);

        // fold what the decision solved of the ask's own holes
        let mut solutions = SmallVec::new();
        for root in &canonical.holes {
            let solution = match self.infer.solution(*root)? {
                Some(solution) => match self.canonical_operand(solution, &mut renaming)? {
                    Some(solution) => Some(solution),
                    None => return Ok(None),
                },
                None => None,
            };
            solutions.push(solution);
        }

        // fold the payload over the growing numbering
        let mut is_refused = !self.canonicalize_response_types(&mut value, &mut renaming)?;

        // fold the relation checks the decision queued and left pending
        let mut checks = SmallVec::new();
        for index in checks_from..self.fulfill.checks.count() {
            let entry = &self.fulfill.checks.entries[index];
            if entry.result.is_some() {
                continue;
            }
            let Check::Relation(check) = entry.check else {
                continue;
            };
            let mut check = check;
            is_refused |= !self.canonicalize_response_types(&mut check, &mut renaming)?;
            checks.push(check);
        }

        // refuse a response the numbering could not fold completely
        if is_refused {
            return Ok(None);
        }

        // describe each grown root so replay sites reopen equal variables
        let Some(holes) = self.hole_contents(&mut renaming, canonical.holes.len(), false)? else {
            return Ok(None);
        };

        Ok(Some(Response {
            value,
            solutions,
            holes,
            checks,
        }))
    }

    /// Reopen one stored response at this ask site's live roots.
    pub(in crate::sema) fn instantiate_response<T: dir::TypeFold + Clone>(
        &mut self,
        origin: Origin,
        canonical: &Canonical,
        response: &Response<T>,
    ) -> CompilerResult<T> {
        // reopen each answer hole with its recorded solving policy
        let mut live_holes: SmallVec<[dir::TypeVariableId; 4]> =
            SmallVec::from_slice(&canonical.holes);
        for hole in &response.holes {
            live_holes.push(self.allocate_variable_of(origin, hole.kind, hole.role));
        }

        // complete each reopened hole with its recorded default
        let mut memo = FxIndexMap::default();
        for (hole, variable) in response
            .holes
            .iter()
            .zip(&live_holes[canonical.holes.len()..])
        {
            if let Some(default) = hole.default {
                let default =
                    self.instantiate_response_type(default, &live_holes, canonical, &mut memo)?;
                self.set_variable_default(*variable, default);
            }
        }

        // bind what the decision solved of the ask's own holes
        for (root, solution) in canonical.holes.iter().zip(&response.solutions) {
            let Some(solution) = solution else {
                continue;
            };
            let solution =
                self.instantiate_response_type(*solution, &live_holes, canonical, &mut memo)?;
            let hole = self.intern_type(dir::Type::Variable(*root))?;
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
            self.constrain_type(origin, cause, Relation::Equal, hole, solution)?;
        }

        // re-register the checks the decision queued
        for check in &response.checks {
            let mut check = *check;
            check.map_types(&mut |ty| {
                self.instantiate_response_type(ty, &live_holes, canonical, &mut memo)
            })?;
            self.push_relation(check)?;
        }

        // reopen the payload at the live roots
        let mut value = response.value.clone();
        value.map_types(&mut |ty| {
            self.instantiate_response_type(ty, &live_holes, canonical, &mut memo)
        })?;

        Ok(value)
    }

    /// Fold one carried group over the growing numbering, reporting refusals.
    fn canonicalize_response_types(
        &mut self,
        value: &mut impl dir::TypeFold,
        renaming: &mut Renaming,
    ) -> CompilerResult<bool> {
        let mut is_folded = true;
        value.map_types(&mut |ty| {
            Ok::<_, CompilerError>(match self.canonical_operand(ty, renaming)? {
                Some(canonical) => canonical,
                None => {
                    is_folded = false;

                    ty
                }
            })
        })?;

        Ok(is_folded)
    }

    /// Reopen one carried type when it reaches a hole or renamed parameter.
    fn instantiate_response_type(
        &mut self,
        ty: dir::GlobalTypeId,
        live_holes: &[dir::TypeVariableId],
        canonical: &Canonical,
        memo: &mut FxIndexMap<dir::GlobalTypeId, dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // serve repeated types from the memo
        if let Some(done) = memo.get(&ty) {
            return Ok(*done);
        }

        // reopen hole and rigid carriers, keeping settled types verbatim
        let flags = self.type_flags(ty)?;
        let done = if flags.has_hole() || flags.has_parameter() {
            let module = self.module_id;

            self.instantiate_answer_type(module, ty, live_holes, &canonical.parameters)?
        } else {
            ty
        };
        memo.insert(ty, done);

        Ok(done)
    }
}
