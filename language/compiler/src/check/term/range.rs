use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, GenericArgument, Origin, TypeOperand, TypeOperationTerm, TypeTerm,
};

/// Runtime range expression term.
///
/// ```ds
/// start..end
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct RangeTerm {
    /// The source range expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The lower bound expression type.
    pub(in crate::check) start: Option<TypeOperand>,
    /// The upper bound expression type.
    pub(in crate::check) end: Option<TypeOperand>,
    /// The range end kind.
    pub(in crate::check) end_kind: dir::RangeEnd,
}

impl CheckState<'_> {
    /// Reduce one runtime range expression to its nominal range value type.
    pub(in crate::check) fn reduce_range_value_term(
        &mut self,
        range: RangeTerm,
    ) -> CompilerResult<Answer<TypeOperand>> {
        let Some(item) = Self::range_language_item(&range) else {
            return self.range_type(range.source, dir::LanguageItem::RangeFull, None);
        };
        let element = self.range_element_type(&range);

        self.range_type(range.source, item, Some(element))
    }

    /// Return the language item selected by one range expression shape.
    fn range_language_item(range: &RangeTerm) -> Option<dir::LanguageItem> {
        let item = match (range.start.is_some(), range.end.is_some(), range.end_kind) {
            (true, true, dir::RangeEnd::Open) => dir::LanguageItem::Range,
            (true, true, dir::RangeEnd::Inclusive) => dir::LanguageItem::RangeInclusive,
            (true, false, _) => dir::LanguageItem::RangeFrom,
            (false, true, dir::RangeEnd::Open) => dir::LanguageItem::RangeTo,
            (false, true, dir::RangeEnd::Inclusive) => dir::LanguageItem::RangeToInclusive,
            (false, false, _) => return None,
        };

        Some(item)
    }

    /// Return the shared bound type for one range expression.
    fn range_element_type(&mut self, range: &RangeTerm) -> TypeOperand {
        let elements = range.start.into_iter().chain(range.end).collect();
        let operation = self
            .inference
            .push_term(TypeOperationTerm::BestCommon { elements });
        let term = self.inference.push_term(TypeTerm::Operation(operation));

        term.into()
    }

    /// Return one canonical range type.
    fn range_type(
        &mut self,
        source: dir::GlobalNodeIdAny,
        item: dir::LanguageItem,
        element: Option<TypeOperand>,
    ) -> CompilerResult<Answer<TypeOperand>> {
        let symbol = self.language_symbol(item);
        let arguments = element
            .into_iter()
            .map(|element| GenericArgument::Type(element.into()))
            .collect();

        let term = TypeTerm::Reference {
            origin: Origin::Node(source),
            symbol,
            arguments,
        };
        let operand = self.type_term_operand(term);

        Ok(Answer::Ready(operand))
    }
}
