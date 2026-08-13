use destack_dir as dir;

use crate::sema::{
    AssignedPlace, CheckState, ClassInitializationObligation, ObligationCheck, ObligationFailure,
    Origin, Relation,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Check one class's required field initialization.
    pub(in crate::sema) fn check_class_initialization(
        &mut self,
        origin: Origin,
        obligation: &ClassInitializationObligation,
    ) -> CompilerResult<ObligationCheck> {
        let fields = self.class_initialization_fields(obligation.symbol)?;
        let mut failures = Vec::new();

        // check each required field against every constructor completion branch
        for field in fields {
            if !self.field_requires_initialization(origin, &field)? {
                continue;
            }
            if self.constructors_assign_field(obligation, &field) {
                continue;
            }

            failures.push(ObligationFailure::FieldNotDefinitelyInitialized {
                source: field.source,
                field: field.symbol,
            });
        }

        let check = ObligationCheck::from_failures(failures);

        Ok(check)
    }

    /// Return fields that need class initialization checking.
    fn class_initialization_fields(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Vec<dir::FieldDefinition>> {
        let Some(dir::Definition::Class(class)) = self.definition(symbol)? else {
            return Err(CompilerError::Internal {
                message: format!("class initialization has no class definition: {symbol:?}"),
            });
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
    ) -> CompilerResult<bool> {
        let ty = self.symbol_type(field.symbol)?;
        let undefined = self.intern_type(dir::Type::Undefined)?;
        let is_assignable = self.evaluate_relation(origin, Relation::Assignable, undefined, ty)?;

        Ok(!is_assignable)
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

        // read the exit branches constructors recorded at check; a class
        //  without any checked constructor initializes nothing
        match self.constructor_branches.get(&obligation.symbol) {
            Some(branches) => branches.iter().all(|branch| branch.assigns(place)),
            None => false,
        }
    }
}
