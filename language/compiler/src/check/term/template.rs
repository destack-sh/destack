use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CallArgument, CallCallee, CallTerm, CheckState, Dependency, GenericArgument, Origin,
    TermId, TypeLiteralTerm, TypeOperand, TypeTerm, VariableId,
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

impl CheckState<'_> {
    /// Reduce one runtime template string to string.
    pub(in crate::check) fn reduce_template_term(
        &mut self,
        template: TermId<TemplateTerm>,
    ) -> CompilerResult<Answer<TypeOperand>> {
        let template = self.inference.term(template);

        // wait for interpolations so failed operands own their diagnostics
        for span in &template.spans {
            if self.resolved_type_operand(*span).is_none() {
                return Ok(Answer::pending(span.dependencies(self)));
            }
        }

        let term = TypeTerm::Literal(TypeLiteralTerm::Primitive(dir::PrimitiveType::String));
        let operand = self.type_term_operand(term);

        Ok(Answer::Ready(operand))
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

impl CheckState<'_> {
    /// Reduce one tagged template as a tag function call.
    pub(in crate::check) fn reduce_tagged_template_term(
        &mut self,
        origin: Origin,
        template: TermId<TaggedTemplateTerm>,
    ) -> CompilerResult<Answer<TypeOperand>> {
        let blockers = self.tagged_template_blockers(template);
        if !blockers.is_empty() {
            return Ok(Answer::pending(blockers));
        }

        let call = self.tagged_template_call(template)?;
        let call = self.inference.push_term(call);
        let source = self.inference.term(template).source;

        self.reduce_call_term(origin, source.module_id, call)
    }

    /// Check a tagged template call to produce the expected result.
    pub(in crate::check) fn expect_tagged_template_term(
        &mut self,
        origin: Origin,
        template: TermId<TaggedTemplateTerm>,
        result: VariableId,
    ) -> CompilerResult<Answer<()>> {
        let blockers = self.tagged_template_blockers(template);
        if !blockers.is_empty() {
            return Ok(Answer::pending(blockers));
        }

        let call = self.tagged_template_call(template)?;
        let call = self.inference.push_term(call);

        self.expect_call_term(origin, call, result)
    }

    /// Return the lowered call shape for one tagged template.
    fn tagged_template_call(
        &mut self,
        template: TermId<TaggedTemplateTerm>,
    ) -> CompilerResult<CallTerm> {
        let template = self.inference.term(template);
        let source = template.source;
        let tag = template.tag;
        let generic_arguments = template.generic_arguments.clone();
        let spans = template.spans.clone();
        let string = TypeTerm::Literal(TypeLiteralTerm::Primitive(dir::PrimitiveType::String));
        let string = self.inference.push_term(string);
        let strings = TypeTerm::Array {
            element: string.into(),
        };
        let strings = self.inference.push_term(strings);
        let mut arguments = SmallVec::<[CallArgument; 4]>::new();

        arguments.push(CallArgument {
            source,
            ty: strings.into(),
            is_spread: false,
        });
        arguments.extend(spans.iter().copied().map(|ty| CallArgument {
            source,
            ty,
            is_spread: false,
        }));

        Ok(CallTerm {
            source,
            callee: CallCallee::Expression(tag),
            generic_arguments,
            arguments,
        })
    }

    /// Return unresolved dependencies referenced by one tagged template.
    fn tagged_template_blockers(
        &self,
        template: TermId<TaggedTemplateTerm>,
    ) -> SmallVec<[Dependency; 4]> {
        let template = self.inference.term(template);
        let dependencies = template
            .tag
            .dependencies(self)
            .into_iter()
            .chain(
                template
                    .generic_arguments
                    .iter()
                    .flat_map(|argument| argument.dependencies(self)),
            )
            .chain(
                template
                    .spans
                    .iter()
                    .flat_map(|span| span.dependencies(self)),
            );
        let mut blockers = SmallVec::new();

        // keep only unsolved variables that can still wake this template
        for variable in dependencies.filter_map(Dependency::solution_variable) {
            if self.variable_solution(variable).is_none() {
                blockers.push(Dependency::Variable(variable));
            }
        }

        blockers
    }
}
