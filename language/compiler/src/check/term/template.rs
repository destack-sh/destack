use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    ArgumentTerm, CallTerm, CheckState, ConstraintOrigin, Progress, Reduction, TypeLiteralTerm,
    TermId, TypeTerm, VariableId,
};

/// Runtime template string term.
///
/// ```ts
/// `/${prefix}/${id}`
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct TemplateTerm {
    /// The source template expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The literal string segments.
    pub(in crate::check) strings: Vec<dir::StringId>,
    /// The interpolated expression types.
    pub(in crate::check) spans: Vec<VariableId>,
}

impl TemplateTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        self.spans.iter().copied().collect()
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
            if self.solved_type_term(*span)?.is_none() {
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
/// ```ts
/// sql<User>`select ${id}`
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct TaggedTemplateTerm {
    /// The source tagged template expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The tag expression type.
    pub(in crate::check) tag: VariableId,
    /// The explicit tag generic arguments.
    pub(in crate::check) generic_arguments: Vec<TermId<ArgumentTerm>>,
    /// The literal string segments.
    pub(in crate::check) strings: Vec<dir::StringId>,
    /// The interpolated expression types.
    pub(in crate::check) spans: Vec<VariableId>,
}

impl TaggedTemplateTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        variables.push(self.tag);
        variables.extend(
            self.generic_arguments
                .iter()
                .flat_map(|argument| state.argument_variables(*argument)),
        );
        variables.extend(self.spans.iter().copied());

        variables
    }
}

impl CheckState<'_> {
    /// Reduce one tagged template as a tag function call.
    pub(in crate::check) fn reduce_tagged_template_term(
        &mut self,
        template: &TaggedTemplateTerm,
    ) -> CompilerResult<Reduction<TypeTerm>> {
        if self.tagged_template_has_unresolved_input(template)? {
            return Ok(Reduction::pending());
        }

        let call = self.tagged_template_call(template)?;

        self.reduce_call_term(template.tag.module, &call)
    }

    /// Expect a tagged template call to produce the expected result.
    pub(in crate::check) fn expect_tagged_template_term(
        &mut self,
        template: &TaggedTemplateTerm,
        result: VariableId,
    ) -> CompilerResult<Progress> {
        if self.tagged_template_has_unresolved_input(template)? {
            return Ok(Progress::Unchanged);
        }

        let call = self.tagged_template_call(template)?;

        self.expect_call_term(&call, result)
    }

    /// Return the lowered call shape for one tagged template.
    fn tagged_template_call(&mut self, template: &TaggedTemplateTerm) -> CompilerResult<CallTerm> {
        let origin = ConstraintOrigin::Node(template.source);
        let string = TypeTerm::Literal(TypeLiteralTerm::Primitive(dir::PrimitiveType::String));
        let string = self.solve_anonymous_type(template.tag.module, origin, string)?;
        let strings = TypeTerm::Array { element: string };
        let strings = self.solve_anonymous_type(template.tag.module, origin, strings)?;
        let mut arguments = Vec::with_capacity(template.spans.len() + 1);

        arguments.push(strings);
        arguments.extend(template.spans.iter().copied());

        Ok(CallTerm {
            source: template.source,
            callee: template.tag,
            member: None,
            candidates: Vec::new(),
            generic_arguments: template.generic_arguments.clone(),
            arguments,
        })
    }

    /// Return whether one tagged template still waits on local operands.
    fn tagged_template_has_unresolved_input(
        &self,
        template: &TaggedTemplateTerm,
    ) -> CompilerResult<bool> {
        for variable in template.referenced_variables(self) {
            if self.variable_solution(variable)?.is_none() {
                return Ok(true);
            }
        }

        Ok(false)
    }
}
