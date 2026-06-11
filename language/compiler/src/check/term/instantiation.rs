use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    CheckState, GenericArgument, GenericInstance, GenericTemplateId, StaticTerm, SubstitutionSet,
    TermId, TypeTerm, VariableKind,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Specialize generic arguments against one template.
    pub(in crate::check) fn specialize_generic_arguments(
        &mut self,
        template: GenericTemplateId,
        arguments: SmallVec<[GenericArgument; 2]>,
    ) -> CompilerResult<SmallVec<[GenericArgument; 2]>> {
        if arguments.is_empty() {
            return Ok(arguments);
        }
        let parameters = self
            .inference
            .generic_template_parameters(template)
            .map(|(_, generic)| generic.kind())
            .collect::<Vec<_>>();
        let mut specialized = SmallVec::new();

        // specialize supplied arguments by declared parameter
        for (index, argument) in arguments.into_iter().enumerate() {
            let Some(parameter) = parameters.get(index) else {
                specialized.push(argument);

                continue;
            };

            specialized.push(argument.specialize(*parameter, self)?);
        }

        Ok(specialized)
    }

    /// Return the generic substitution for one applied symbol.
    pub(in crate::check) fn generic_substitution(
        &mut self,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
    ) -> CompilerResult<SubstitutionSet> {
        if arguments.is_empty() {
            return Ok(SubstitutionSet::empty());
        }
        let Some(template) = self.inference.generic_template_by_symbol(symbol) else {
            return Err(CompilerError::Internal {
                message: format!("generic arguments supplied for non-generic symbol {symbol:?}"),
            });
        };
        let parameters = self
            .inference
            .generic_template_parameters(template)
            .map(|(_, generic)| {
                let parameter = generic.parameter().id;
                let kind = generic.kind();

                (parameter, kind)
            })
            .collect::<Vec<_>>();

        let mut substitution = SubstitutionSet::empty();

        // collect already resolved generic entries in declaration order
        for ((parameter, kind), argument) in parameters.into_iter().zip(arguments.iter()) {
            let argument = argument.specialize(kind, self)?;

            substitution.generic(parameter, argument);
        }

        Ok(substitution)
    }

    /// Return the generic substitution for one applied reference term.
    pub(in crate::check) fn generic_substitution_for_type_term(
        &mut self,
        symbol: dir::GlobalSymbolId,
        term: TermId<TypeTerm>,
    ) -> CompilerResult<SubstitutionSet> {
        let Some(template) = self.inference.generic_template_by_symbol(symbol) else {
            return Ok(SubstitutionSet::empty());
        };
        let parameters = self
            .inference
            .generic_template_parameters(template)
            .map(|(_, generic)| {
                let parameter = generic.parameter().id;
                let kind = generic.kind();

                (parameter, kind)
            })
            .collect::<Vec<_>>();

        let mut substitution = SubstitutionSet::empty();

        // collect stored generic entries in declaration order
        for (index, (parameter, kind)) in parameters.into_iter().enumerate() {
            let Some(argument) = self.type_term_generic_argument(term, index) else {
                break;
            };
            let argument = argument.specialize(kind, self)?;

            substitution.generic(parameter, argument);
        }

        Ok(substitution)
    }

    /// Return one generic argument from a reference term.
    fn type_term_generic_argument(
        &self,
        term: TermId<TypeTerm>,
        index: usize,
    ) -> Option<GenericArgument> {
        let TypeTerm::Reference { arguments, .. } = self.inference.term(term) else {
            return None;
        };

        arguments.get(index).copied()
    }

    /// Return the generic substitution for one generic instance.
    pub(in crate::check) fn generic_instance_substitution(
        &mut self,
        instance: &GenericInstance,
    ) -> CompilerResult<SubstitutionSet> {
        let parameters = self
            .inference
            .generic_template_parameters(instance.template)
            .map(|(_, generic)| {
                let parameter = generic.parameter().id;
                let kind = generic.kind();

                (parameter, kind)
            })
            .collect::<Vec<_>>();
        let mut substitution = SubstitutionSet::empty();

        // collect instance arguments in template order
        for ((parameter, kind), argument) in parameters.into_iter().zip(&instance.arguments) {
            let argument = argument.specialize(kind, self)?;

            substitution.generic(parameter, argument);
        }

        Ok(substitution)
    }

    /// Return the generic instance described by one substitution.
    pub(in crate::check) fn generic_instance_from_substitution(
        &mut self,
        symbol: dir::GlobalSymbolId,
        substitution: &SubstitutionSet,
    ) -> CompilerResult<Option<GenericInstance>> {
        if !substitution.has_generics() {
            return Ok(None);
        }
        let Some(template) = self.inference.generic_template_by_symbol(symbol) else {
            return Ok(None);
        };
        let parameters = self
            .inference
            .generic_template_parameters(template)
            .map(|(_, generic)| {
                let parameter = generic.parameter().id;
                let kind = generic.kind();

                (parameter, kind)
            })
            .collect::<Vec<_>>();
        let mut arguments = Vec::with_capacity(parameters.len());

        // collect all substituted parameters in template order
        for (parameter, kind) in parameters {
            let argument = match kind {
                VariableKind::Type => self
                    .substitution_type_operand(substitution, parameter)
                    .map(GenericArgument::Type)
                    .unwrap_or_else(|| {
                        let term = self.inference.push_term(TypeTerm::Parameter(parameter));

                        GenericArgument::Type(term.into())
                    }),
                VariableKind::Static => self
                    .substitution_static_operand(substitution, parameter)
                    .map(GenericArgument::Static)
                    .unwrap_or_else(|| {
                        let term = self.inference.push_term(StaticTerm::Parameter(parameter));

                        GenericArgument::Static(term.into())
                    }),
            };

            arguments.push(argument);
        }

        Ok(Some(GenericInstance::new(template, arguments.into())))
    }
}
