use destack_dir as dir;

use crate::CompilerResult;
use crate::check::CheckState;

/// One assumed static predicate value.
///
/// Assumptions carry the active @if guard context: code under a guard is
/// checked as if the guard held, so its predicate reduces to a literal
/// instead of staying symbolic. Negated guards normalize to assumed-false
/// targets when installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct Assumption {
    /// The assumed predicate type.
    pub(in crate::check) predicate: dir::GlobalTypeId,
    /// The assumed boolean value.
    pub(in crate::check) holds: bool,
}

impl CheckState<'_> {
    /// Assume one guard predicate list, returning the release mark.
    pub(in crate::check) fn assume(
        &mut self,
        predicates: &[dir::GlobalTypeId],
    ) -> CompilerResult<usize> {
        let mark = self.assumptions.len();

        for predicate in predicates {
            // normalize negated guards onto their targets
            let mut predicate = self.resolve_root(*predicate)?;
            let mut holds = true;
            while let dir::Type::Operation(dir::TypeOperation::StaticUnary(unary)) =
                self.ty(predicate)?
            {
                if unary.operator != dir::StaticUnaryOperator::Not {
                    break;
                }
                predicate = self.resolve_root(unary.target)?;
                holds = !holds;
            }

            self.assumptions.push(Assumption { predicate, holds });
        }

        Ok(mark)
    }

    /// Release every assumption installed after one mark.
    pub(in crate::check) fn release_assumptions(&mut self, mark: usize) {
        self.assumptions.truncate(mark);
    }

    /// Return the assumed value of one type root when it matches an active assumption.
    pub(in crate::check) fn assumed_value(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<bool>> {
        // only static predicate forms can match assumptions
        let (id, negated) = match self.ty(id)? {
            dir::Type::Operation(dir::TypeOperation::StaticUnary(unary))
                if unary.operator == dir::StaticUnaryOperator::Not =>
            {
                (self.resolve_root(unary.target)?, true)
            }
            dir::Type::Operation(_) | dir::Type::Reference(_) | dir::Type::Member(_) => (id, false),
            _ => return Ok(None),
        };

        for index in (0..self.assumptions.len()).rev() {
            let assumption = self.assumptions[index];
            if self.same_static_form(id, assumption.predicate)? {
                return Ok(Some(assumption.holds != negated));
            }
        }

        Ok(None)
    }

    /// Return whether two static predicate forms are structurally equal.
    fn same_static_form(
        &self,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let left = self.resolve_root(left)?;
        let right = self.resolve_root(right)?;
        if left == right {
            return Ok(true);
        }

        match (self.ty(left)?, self.ty(right)?) {
            (dir::Type::Literal(left), dir::Type::Literal(right)) => Ok(left == right),
            (dir::Type::Parameter(left), dir::Type::Parameter(right)) => Ok(left == right),
            (dir::Type::This, dir::Type::This) => Ok(true),
            (dir::Type::Static(left), dir::Type::Static(right)) => Ok(left == right),
            (dir::Type::Reference(left), dir::Type::Reference(right)) => {
                if left.symbol != right.symbol || left.arguments.len() != right.arguments.len() {
                    return Ok(false);
                }
                let pairs = left
                    .arguments
                    .iter()
                    .copied()
                    .zip(right.arguments.iter().copied())
                    .collect::<smallvec::SmallVec<[_; 4]>>();

                self.same_each_form(&pairs)
            }
            (dir::Type::Member(left), dir::Type::Member(right)) => {
                if left.key != right.key || left.arguments.len() != right.arguments.len() {
                    return Ok(false);
                }
                let mut pairs = smallvec::SmallVec::<[_; 4]>::new();
                pairs.push((left.owner, right.owner));
                pairs.extend(
                    left.arguments
                        .iter()
                        .copied()
                        .zip(right.arguments.iter().copied()),
                );

                self.same_each_form(&pairs)
            }
            (
                dir::Type::Operation(dir::TypeOperation::StaticBinary(left)),
                dir::Type::Operation(dir::TypeOperation::StaticBinary(right)),
            ) => {
                if left.operator != right.operator {
                    return Ok(false);
                }

                self.same_each_form(&[(left.left, right.left), (left.right, right.right)])
            }
            (
                dir::Type::Operation(dir::TypeOperation::StaticUnary(left)),
                dir::Type::Operation(dir::TypeOperation::StaticUnary(right)),
            ) => {
                if left.operator != right.operator {
                    return Ok(false);
                }

                self.same_static_form(left.target, right.target)
            }
            _ => Ok(false),
        }
    }

    /// Return whether every form pair is structurally equal.
    fn same_each_form(
        &self,
        pairs: &[(dir::GlobalTypeId, dir::GlobalTypeId)],
    ) -> CompilerResult<bool> {
        for (left, right) in pairs.iter().copied() {
            if !self.same_static_form(left, right)? {
                return Ok(false);
            }
        }

        Ok(true)
    }
}
