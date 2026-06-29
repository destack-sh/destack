use destack_artifact::DiagnosticBuilder;
use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, AssignedPlace, CheckError, CheckState, ClassInitializationObligation, Dependency,
    Origin, Relation,
};

impl CheckState<'_> {
    /// Check one class's required field initialization.
    pub(in crate::check) fn check_class_initialization(
        &mut self,
        obligation: &ClassInitializationObligation,
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        let origin = Origin::Node(obligation.source);
        let fields = self.class_initialization_fields(obligation.symbol);
        let mut errors = Vec::new();
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

            errors.push(self.field_initialization_error(&field));
        }

        // wait until every field type decision has closed
        if !blockers.is_empty() {
            return Ok(Answer::Pending(blockers));
        }

        Ok(Answer::Ready(self.report_class_initialization_errors(
            obligation.source,
            errors,
        )))
    }

    /// Return fields that need class initialization checking.
    fn class_initialization_fields(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Vec<dir::FieldDefinition> {
        let Some(dir::Definition::Class(class)) = self.definition(symbol) else {
            return Vec::new();
        };

        // collect instance fields without direct initializers
        class
            .members
            .iter()
            .filter_map(|member| match member {
                dir::DefinitionMember::Field(field)
                    if field.space == dir::MemberSpace::Instance
                        && field.initializer.is_none()
                        && !field.is_abstract =>
                {
                    Some(field.clone())
                }
                _ => None,
            })
            .collect()
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
        let undefined = self.push_type(
            field.source.module_id,
            dir::Type::Undefined,
            field.source.local_id,
        )?;

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

    /// Return one field initialization diagnostic.
    fn field_initialization_error(
        &self,
        field: &dir::FieldDefinition,
    ) -> DiagnosticBuilder<CheckError> {
        let (module, anchor) = self.source_anchor(field.source);
        let field = self.format_symbol(field.symbol);
        let error = CheckError::FieldNotDefinitelyInitialized {
            anchor,
            module,
            field,
        };

        error.into()
    }

    /// Report every class initialization error except the returned first error.
    fn report_class_initialization_errors(
        &mut self,
        source: dir::GlobalNodeIdAny,
        errors: Vec<DiagnosticBuilder<CheckError>>,
    ) -> Option<DiagnosticBuilder<CheckError>> {
        let mut errors = errors.into_iter();
        let first = errors.next();
        for error in errors {
            self.module_mut(source.module_id).diagnostics.push(error);
        }

        first
    }
}
