use tspp_core::FxIndexSet;
use tspp_dir as dir;

use crate::sema::{
    AssignedPlace, CheckState, FieldInitializationObligation, ObligationCheck, ObligationFailure,
    Origin, Relation,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Report the fields the checked constructors leave uninitialized.
    pub(in crate::sema) fn report_field_initializations(&mut self) -> CompilerResult<()> {
        // report each declaration's uninitialized fields
        for obligation in std::mem::take(&mut self.field_initializations) {
            let origin = Origin::Node(obligation.source, obligation.scope);

            // report each constructor of a derived class reaching its end ahead of the base call
            if self.class_has_base(obligation.symbol)? {
                let undelegated = self
                    .constructor_branches
                    .get(&obligation.symbol)
                    .map(|branches| {
                        branches
                            .iter()
                            .filter(|(_, branch)| !branch.is_assigned(AssignedPlace::Delegated))
                            .map(|(constructor, _)| *constructor)
                            .collect::<FxIndexSet<_>>()
                    })
                    .unwrap_or_default();
                for constructor in undelegated {
                    let node = self
                        .module(constructor.module_id)
                        .symbol_declaration_node(constructor.local_id)?;
                    self.report_missing_super_call(constructor.module_id, node);
                }
            }
            let check = self.check_field_initialization(origin, &obligation)?;
            for failure in check.into_failures() {
                self.report_obligation_failure(failure)?;
            }
        }

        Ok(())
    }

    /// Check one declaration's required field initialization.
    fn check_field_initialization(
        &mut self,
        origin: Origin,
        obligation: &FieldInitializationObligation,
    ) -> CompilerResult<ObligationCheck> {
        // collect the fields that may require initialization
        let fields = self.initialization_fields(obligation.symbol)?;
        let mut failures = Vec::new();

        // record the fields every constructor assigns, for lowering to skip their defaults
        let assigned: Vec<_> = fields
            .iter()
            .filter(|field| self.has_constructor_assignment(obligation, field))
            .map(|field| field.symbol)
            .collect();
        self.module
            .decisions_tail
            .set_constructor_assignments(obligation.symbol, assigned);

        // check each field without a default that requires a runtime value
        for field in fields {
            if field.initializer.is_some()
                || field.is_optional
                || !self.is_initialization_required(origin, &field)?
            {
                continue;
            }

            // require static storage to initialize with its declaration
            if field.space == dir::MemberSpace::Static {
                failures.push(ObligationFailure::StaticFieldMissingInitializer {
                    source: field.source,
                    field: field.symbol,
                });
            }
            // accept instance storage assigned on every constructor completion
            else if !self.has_constructor_assignment(obligation, &field) {
                failures.push(ObligationFailure::FieldNotDefinitelyInitialized {
                    source: field.source,
                    field: field.symbol,
                });
            }
        }

        // gather the collected failures into one check
        let check = ObligationCheck::from_failures(failures);

        Ok(check)
    }

    /// Return fields whose declarations may require initialization.
    fn initialization_fields(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Vec<dir::FieldDefinition>> {
        // read the declaration's definition
        let Some(definition) = self.definition(symbol)? else {
            return Err(CompilerError::Internal {
                message: format!("field initialization has no definition: {symbol:?}"),
            });
        };
        let is_class = matches!(*definition, dir::Definition::Class(_));

        // collect the concrete storage a constructor may initialize
        Ok(definition
            .members()
            .iter()
            .filter_map(|member| match member {
                dir::DefinitionMember::Field(field)
                    if (field.space == dir::MemberSpace::Static || is_class)
                        && !field.is_abstract =>
                {
                    Some(field.clone())
                }
                _ => None,
            })
            .collect())
    }

    /// Return whether one field requires initialization.
    fn is_initialization_required(
        &mut self,
        origin: Origin,
        field: &dir::FieldDefinition,
    ) -> CompilerResult<bool> {
        // require a field whose declared type excludes undefined
        let ty = self.symbol_type(field.symbol)?;
        let undefined = self.intern_type(dir::Type::Undefined)?;
        let is_assignable = self
            .decide_relation(origin, Relation::Subtype, undefined, ty)?
            .holds();

        Ok(!is_assignable)
    }

    /// Return whether every constructor branch assigns one field.
    fn has_constructor_assignment(
        &self,
        obligation: &FieldInitializationObligation,
        field: &dir::FieldDefinition,
    ) -> bool {
        // name the place every constructor must assign
        let place = AssignedPlace::Member {
            receiver: obligation.receiver,
            key: field.key,
        };

        // require every checked constructor branch to assign the place
        match self.constructor_branches.get(&obligation.symbol) {
            Some(branches) => branches.iter().all(|(_, branch)| branch.is_assigned(place)),
            None => false,
        }
    }
}
