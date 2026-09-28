use tspp_dir as dir;
use tspp_mir as mir;

use crate::CompilerResult;
use crate::lower::FunctionLowerer;
use crate::lower::function::place::Place;

impl FunctionLowerer<'_, '_, '_> {
    /// Fake-borrow the operands of one test.
    pub(in crate::lower) fn borrow_tested_operands(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // find the tested operands
        let test = expression.into_global_any(self.source);
        if !self.source().narrowing_tests.contains(&test) {
            return Ok(());
        }
        let operands = match *self.source().tree().get(expression) {
            dir::Expression::Binary {
                left,
                operator,
                right,
            } if operator.is_equality() => vec![left, right],
            dir::Expression::Binary {
                operator: dir::BinaryOperator::In,
                right,
                ..
            } => vec![right],
            dir::Expression::Is { value, .. } | dir::Expression::InstanceOf { value, .. } => {
                vec![value]
            }
            _ => Vec::new(),
        };

        // borrow each operand
        let mut borrows = Vec::new();
        for operand in operands {
            borrows.extend(self.borrow_tested_operand(operand)?);
        }
        self.tested.push((test, borrows));

        Ok(())
    }

    /// Fake-borrow one switch scrutinee.
    pub(in crate::lower) fn borrow_tested_scrutinee(
        &mut self,
        cases: &[dir::LocalNodeId<dir::SwitchCase>],
        value: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // keep the read tests
        let tests: Vec<_> = cases
            .iter()
            .map(|case| case.into_global_any(self.source))
            .filter(|test| self.source().narrowing_tests.contains(test))
            .collect();
        if tests.is_empty() {
            return Ok(());
        }

        // borrow the scrutinee once
        let borrow = self.borrow_tested_operand(value)?;
        for test in tests {
            self.tested
                .push((test, borrow.clone().into_iter().collect()));
        }

        Ok(())
    }

    /// Fake-borrow one tested operand.
    fn borrow_tested_operand(
        &mut self,
        operand: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<(Option<dir::AccessPath>, mir::Value)>> {
        let node = operand.into_global_any(self.source);
        let Some(access) = self.source().decisions.access_resolution(node) else {
            return Ok(None);
        };
        let path = access.path().clone();
        if !self.is_place_expression(operand) || !self.names_storage(operand)? {
            return Ok(None);
        }
        let place = self.storage_place(operand)?;
        let borrow = self.fake_borrow(&place)?;

        Ok(Some((Some(path), borrow)))
    }

    /// Fake-borrow one tested place.
    pub(in crate::lower) fn borrow_tested_place(
        &mut self,
        tests: &[dir::GlobalNodeIdAny],
        place: &Place,
    ) -> CompilerResult<()> {
        // keep the read tests
        let tests: Vec<_> = tests
            .iter()
            .copied()
            .filter(|test| self.source().narrowing_tests.contains(test))
            .collect();
        if tests.is_empty() {
            return Ok(());
        }

        // borrow the place once
        let borrow = self.fake_borrow(place)?;
        for test in tests {
            self.tested.push((test, vec![(None, borrow)]));
        }

        Ok(())
    }

    /// Fake-borrow one place for a pattern.
    pub(in crate::lower) fn borrow_tested_pattern(
        &mut self,
        pattern: dir::LocalNodeId<dir::Pattern>,
        place: &Place,
    ) -> CompilerResult<()> {
        let test = pattern.into_global_any(self.source);
        self.borrow_tested_place(&[test], place)?;

        // follow the inner pattern
        match *self.source().tree().get(pattern) {
            dir::Pattern::Binding {
                pattern: Some(nested),
                ..
            }
            | dir::Pattern::Default {
                pattern: nested, ..
            } => self.borrow_tested_pattern(nested, place),
            _ => Ok(()),
        }
    }

    /// Issue one fake borrow of a place.
    fn fake_borrow(&mut self, place: &Place) -> CompilerResult<mir::Value> {
        let ty = self.place_type(place)?;
        let ty = self.insert_reference(
            mir::Reference::Borrowed,
            mir::Lifetime::frame(),
            mir::Access::Readonly,
            ty,
        );
        let place = place.lower(self)?;

        Ok(self.builder.fake_borrow(place, ty))
    }

    /// Fake-read the test borrows of one narrowed read.
    pub(in crate::lower) fn read_narrowing_tests(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // find the tests and path
        let node = expression.into_global_any(self.source);
        let Some(narrowing) = self.source().decisions.narrowing(node) else {
            return Ok(());
        };
        let path = self
            .source()
            .decisions
            .access_resolution(node)
            .map(|access| access.path().clone());

        // collect the related borrows
        let mut borrows = Vec::new();
        for test in &narrowing.tests {
            let Some((_, tested)) = self.tested.iter().rev().find(|(node, _)| node == test) else {
                continue;
            };
            for (tested, borrow) in tested {
                let is_related = match (tested, &path) {
                    (Some(tested), Some(path)) => tested.starts_with(path),
                    _ => true,
                };
                if is_related {
                    borrows.push(*borrow);
                }
            }
        }

        for borrow in borrows {
            self.builder.fake_read(borrow);
        }

        Ok(())
    }
}
