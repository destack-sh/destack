use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CallCallee, CallTerm, CheckState, GenericArgument, Origin, Progress, Reduction,
    TypeLiteralTerm, TypeOperand, TypeTerm, VariableId,
};

/// Runtime template string term.
///
/// ```ds
/// `/${prefix}/${id}`
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct TemplateTerm {
    /// The source template expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The literal string segments.
    pub(in crate::check) strings: Vec<dir::StringId>,
    /// The interpolated expression types.
    pub(in crate::check) spans: Vec<TypeOperand>,
}

impl TemplateTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 2]> {
        self.spans
            .iter()
            .flat_map(|span| span.referenced_variables(state))
            .collect()
    }
}

impl CheckState<'_> {
    /// Reduce one runtime template string to string.
    pub(in crate::check) fn reduce_template_term(
        &self,
        template: &TemplateTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        // wait for interpolations so failed operands own their diagnostics
        for span in &template.spans {
            if self.type_operand_term(*span)?.is_none() {
                return Ok(None);
            }
        }

        Ok(Some(TypeTerm::Literal(TypeLiteralTerm::Primitive(
            dir::PrimitiveType::String,
        ))))
    }
}

/// Runtime tagged template term.
///
/// ```ds
/// sql<User>`select ${id}`
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct TaggedTemplateTerm {
    /// The source tagged template expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The tag expression type.
    pub(in crate::check) tag: TypeOperand,
    /// The explicit tag generic arguments.
    pub(in crate::check) generic_arguments: SmallVec<[GenericArgument; 2]>,
    /// The literal string segments.
    pub(in crate::check) strings: Vec<dir::StringId>,
    /// The interpolated expression types.
    pub(in crate::check) spans: Vec<TypeOperand>,
}

impl TaggedTemplateTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 2]> {
        let mut variables = SmallVec::new();

        variables.extend(self.tag.referenced_variables(state));
        variables.extend(
            self.generic_arguments
                .iter()
                .flat_map(|argument| state.argument_variables(argument)),
        );
        variables.extend(
            self.spans
                .iter()
                .flat_map(|span| span.referenced_variables(state)),
        );

        variables
    }
}

impl CheckState<'_> {
    /// Reduce one tagged template as a tag function call.
    pub(in crate::check) fn reduce_tagged_template_term(
        &mut self,
        origin: Origin,
        template: &TaggedTemplateTerm,
    ) -> CompilerResult<Reduction<TypeTerm>> {
        if self.tagged_template_has_unresolved_input(template)? {
            return Ok(Reduction::pending());
        }

        let call = self.tagged_template_call(template)?;

        self.reduce_call_term(origin, template.source.module_id, &call)
    }

    /// Expect a tagged template call to produce the expected result.
    pub(in crate::check) fn expect_tagged_template_term(
        &mut self,
        origin: Origin,
        template: &TaggedTemplateTerm,
        result: VariableId,
    ) -> CompilerResult<Progress> {
        if self.tagged_template_has_unresolved_input(template)? {
            return Ok(Progress::Unchanged);
        }

        let call = self.tagged_template_call(template)?;

        self.expect_call_term(origin, &call, result)
    }

    /// Return the lowered call shape for one tagged template.
    fn tagged_template_call(&mut self, template: &TaggedTemplateTerm) -> CompilerResult<CallTerm> {
        let string = TypeTerm::Literal(TypeLiteralTerm::Primitive(dir::PrimitiveType::String));
        let string = self.push_term(string);
        let strings = TypeTerm::Array {
            element: string.into(),
        };
        let strings = self.push_term(strings);
        let mut arguments = Vec::with_capacity(template.spans.len() + 1);

        arguments.push(strings.into());
        arguments.extend(template.spans.iter().copied());

        Ok(CallTerm {
            source: template.source,
            callee: CallCallee::Expression(template.tag),
            generic_arguments: template.generic_arguments.clone(),
            arguments: arguments.into(),
            argument_values: Default::default(),
        })
    }

    /// Return whether one tagged template still waits on local operands.
    fn tagged_template_has_unresolved_input(
        &self,
        template: &TaggedTemplateTerm,
    ) -> CompilerResult<bool> {
        for variable in template.referenced_variables(self) {
            if self.variable_solution(variable).is_none() {
                return Ok(true);
            }
        }

        Ok(false)
    }
}
