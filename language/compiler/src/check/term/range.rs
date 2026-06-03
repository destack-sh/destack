use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CheckState, GenericArgument, Origin, Progress, Reduction, TypeOperand, TypeOperationTerm,
    TypeTerm, VariableId,
};

/// Runtime range expression term.
///
/// ```ds
/// start..end
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct RangeValueTerm {
    /// The source range expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The lower bound expression type.
    pub(in crate::check) start: Option<TypeOperand>,
    /// The upper bound expression type.
    pub(in crate::check) end: Option<TypeOperand>,
    /// The range end kind.
    pub(in crate::check) end_kind: dir::RangeEnd,
}

impl RangeValueTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 2]> {
        let mut variables = SmallVec::new();

        if let Some(start) = self.start {
            variables.extend(start.referenced_variables(state));
        }
        if let Some(end) = self.end {
            variables.extend(end.referenced_variables(state));
        }

        variables
    }
}

impl CheckState<'_> {
    /// Reduce one runtime range expression to its nominal range value type.
    pub(in crate::check) fn reduce_range_value_term(
        &mut self,
        module: ModuleId,
        range: &RangeValueTerm,
    ) -> CompilerResult<Reduction<TypeTerm>> {
        let Some(item) = Self::range_language_item(range) else {
            return self.range_type(module, range.source, dir::LanguageItem::RangeFull, None);
        };
        let element = self.range_element_type(range);

        self.range_type(module, range.source, item, Some(element))
    }

    /// Expect one runtime range value to use a contextual element type.
    pub(in crate::check) fn expect_range_value_term(
        &mut self,
        origin: Origin,
        range: &RangeValueTerm,
        expected: &TypeTerm,
    ) -> CompilerResult<Progress> {
        let Some(item) = Self::range_language_item(range) else {
            return Ok(Progress::Unchanged);
        };
        let TypeTerm::Reference {
            origin: _,
            symbol,
            arguments,
        } = expected
        else {
            return Ok(Progress::Unchanged);
        };
        if self.environment.language.item(*symbol) != Some(item) {
            return Ok(Progress::Unchanged);
        }
        let Some(element) = self.type_argument_variable_at(arguments, 0) else {
            return Ok(Progress::Unchanged);
        };
        let mut progress = Progress::Unchanged;

        // push element context into present bounds
        if let Some(start) = range.start {
            progress =
                progress.merge(self.relate_contextual_type_assignability(origin, start, element)?);
        }
        if let Some(end) = range.end {
            progress =
                progress.merge(self.relate_contextual_type_assignability(origin, end, element)?);
        }

        Ok(progress)
    }

    /// Return the language item selected by one range expression shape.
    fn range_language_item(range: &RangeValueTerm) -> Option<dir::LanguageItem> {
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
    fn range_element_type(&mut self, range: &RangeValueTerm) -> TypeOperand {
        let elements = range.start.into_iter().chain(range.end).collect();
        let operation = self
            .inference
            .push_term(TypeOperationTerm::BestCommon { elements });
        let term = self.inference.push_term(TypeTerm::Operation(operation));

        term.into()
    }

    /// Return one nominal range language item type.
    fn range_type(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        item: dir::LanguageItem,
        element: Option<TypeOperand>,
    ) -> CompilerResult<Reduction<TypeTerm>> {
        let symbol = self.language_symbol(module, item);
        let arguments = element
            .into_iter()
            .map(|element| GenericArgument::Type(element.into()))
            .collect();

        Ok(Reduction::value(TypeTerm::Reference {
            origin: Origin::Node(source),
            symbol,
            arguments,
        }))
    }
}
