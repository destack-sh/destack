use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, AssignedPlace, CheckState, ClassInitializationObligation, Dependency, ObligationCheck,
    ObligationFailure, Origin, Relation,
};

impl CheckState<'_> {
    /// Check one class's required field initialization.
    pub(in crate::check) fn check_class_initialization(
        &mut self,
        origin: Origin,
        obligation: &ClassInitializationObligation,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        let fields = self.class_initialization_fields(obligation.symbol)?;
        let mut failures = Vec::new();
        let mut blockers = SmallVec::<[Dependency; 2]>::new();

        // check each required field against every constructor completion branch
        for field in fields {
            match self.field_requires_initialization(origin, &field)? {
                Answer::Ready(true) => {}
                Answer::Ready(false) => continue,
                Answer::Pending(pending) => {
                    blockers.extend(pending);
                    continue;
                }
            }

            if self.constructors_assign_field(obligation, &field) {
                continue;
            }

            failures.push(ObligationFailure::FieldNotDefinitelyInitialized {
                source: field.source,
                field: field.symbol,
            });
        }

        // wait until every field type decision has closed
        if !blockers.is_empty() {
            return Ok(Answer::Pending(blockers));
        }

        let check = ObligationCheck::from_failures(failures);

        Ok(Answer::Ready(check))
    }

    /// Return fields that need class initialization checking.
    fn class_initialization_fields(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Vec<dir::FieldDefinition>> {
        let Some(dir::Definition::Class(class)) = self.definition(symbol)? else {
            return Ok(Vec::new());
        };

        // collect instance fields without direct initializers,
        //  skipping definite assignment assertions like `handle!: T`
        Ok(class
            .members
            .iter()
            .filter_map(|member| match member {
                dir::DefinitionMember::Field(field)
                    if field.space == dir::MemberSpace::Instance
                        && field.initializer.is_none()
                        && !field.is_optional
                        && !field.is_definite
                        && !field.is_abstract =>
                {
                    Some(field.clone())
                }
                _ => None,
            })
            .collect())
    }

    /// Return whether one field still needs constructor initialization.
    fn field_requires_initialization(
        &mut self,
        origin: Origin,
        field: &dir::FieldDefinition,
    ) -> CompilerResult<Answer<bool>> {
        let ty = match self.symbol_type(field.symbol)? {
            Answer::Ready(ty) => ty,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let undefined = self.intern_type(field.source.module_id, dir::Type::Undefined)?;

        match self.decide_relation(origin, Relation::Assignable, undefined, ty)? {
            Answer::Ready(is_assignable) => Ok(Answer::Ready(!is_assignable)),
            Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
        }
    }

    /// Return whether every constructor branch assigns one field.
    fn constructors_assign_field(
        &self,
        obligation: &ClassInitializationObligation,
        field: &dir::FieldDefinition,
    ) -> bool {
        let place = AssignedPlace::Member {
            receiver: obligation.receiver,
            key: field.key,
        };

        obligation
            .constructor_branches
            .iter()
            .all(|branch| branch.assigns(place))
    }
}
