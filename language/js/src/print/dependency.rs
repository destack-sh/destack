use destack_core::StringId;
use destack_source::{NodeSpanRegion, NodeSpanType, Span};

use super::printer::Printer;
use crate::{
    Annotation, DependencyBinding, DependencyForm, DependencyItem, JsPrintResult, Keyword,
    LocalNodeId, Path,
};

impl<'a> Printer<'a> {
    /// Print one annotation.
    pub(crate) fn print_annotation(&mut self, annotation: &Annotation) -> JsPrintResult<()> {
        match annotation {
            Annotation::Comment { string, .. } => {
                self.write_punct("//");
                self.write_string_id(*string);
            }
        }

        Ok(())
    }

    /// Print one import binding.
    pub(crate) fn print_import_binding(
        &mut self,
        target: StringId,
        items: &[LocalNodeId<DependencyItem>],
        target_span: Option<Span>,
    ) -> JsPrintResult<()> {
        let tree = self.tree;
        let first_item = items.first().map(|item| tree.get(*item));

        if items.len() == 1
            && let Some(first_item) = first_item
            && first_item.binding == DependencyBinding::Namespace
        {
            self.write_punct("*");
            self.write_keyword(Keyword::As);

            if let Some(item_id) = items.first()
                && let Some(alias) = first_item.alias
            {
                let alias_span = self.source_part_span(item_id.id, NodeSpanType::Main);
                self.write_identifier_with_source_span(alias, alias_span);
            }
        } else if let Some(first_item) = first_item
            && first_item.binding == DependencyBinding::Default
        {
            if let Some(item_id) = items.first()
                && let Some(alias) = first_item.alias
            {
                let alias_span = self.source_part_span(item_id.id, NodeSpanType::Main);
                self.write_identifier_with_source_span(alias, alias_span);
            }

            let rest_items: Vec<LocalNodeId<DependencyItem>> =
                items.iter().skip(1).copied().collect();

            if !rest_items.is_empty() {
                self.write_punct(",");
                self.write_punct("{");
                self.print_dependency_item_list(&rest_items)?;
                self.write_punct("}");
            }
        } else if !items.is_empty() {
            self.write_punct("{");
            self.print_dependency_item_list(items)?;
            self.write_punct("}");
        }

        if !items.is_empty() {
            self.write_keyword(Keyword::From);
        }

        self.write_string_literal_with_source_span(target, target_span);
        Ok(())
    }

    /// Print one export binding.
    pub(crate) fn print_export_binding(
        &mut self,
        target: Option<StringId>,
        items: &[LocalNodeId<DependencyItem>],
        target_span: Option<Span>,
    ) -> JsPrintResult<()> {
        let tree = self.tree;
        let first_item = items.first().map(|item| tree.get(*item));

        if let Some(first_item) = first_item
            && let Some(value) = first_item.value
        {
            self.write_punct("=");
            self.print_expression_id(value)?;
            return Ok(());
        }

        if items.len() == 1
            && let Some(first_item) = first_item
            && first_item.binding == DependencyBinding::Namespace
        {
            self.write_punct("*");

            if let Some(item_id) = items.first()
                && let Some(alias) = first_item.alias
            {
                let alias_span = self.source_part_span(item_id.id, NodeSpanType::Main);
                self.write_keyword(Keyword::As);
                self.write_identifier_with_source_span(alias, alias_span);
            }
        } else if !items.is_empty() {
            self.write_punct("{");
            self.print_dependency_item_list(items)?;
            self.write_punct("}");
        }

        if let Some(target) = target {
            self.write_keyword(Keyword::From);
            self.write_string_literal_with_source_span(target, target_span);
        }

        Ok(())
    }

    /// Print one dependency item.
    pub(crate) fn print_dependency_item(
        &mut self,
        item_id: LocalNodeId<DependencyItem>,
        item: &DependencyItem,
    ) -> JsPrintResult<()> {
        let name_span =
            self.source_part_span(item_id.id, NodeSpanType::Region(NodeSpanRegion::Type));
        let alias_span = self.source_part_span(item_id.id, NodeSpanType::Main);

        if item.form == Some(DependencyForm::Type) {
            self.write_keyword(Keyword::Type);
        }

        if item.binding == DependencyBinding::Default {
            self.write_keyword(Keyword::Default);

            if let Some(alias) = item.alias {
                self.write_keyword(Keyword::As);
                self.write_identifier_with_source_span(alias, alias_span);
            }
        } else {
            if let Some(name) = item.name {
                self.write_name_with_source_span(name, name_span);
            }

            if let Some(alias) = item.alias {
                self.write_keyword(Keyword::As);
                self.write_identifier_with_source_span(alias, alias_span);
            }
        }

        Ok(())
    }

    /// Print one path.
    pub(crate) fn print_path(&mut self, path: &Path) {
        for (index, segment) in path.segments.iter().enumerate() {
            if index > 0 {
                self.write_punct(".");
            }

            self.write_string_id(*segment);
        }
    }
}
