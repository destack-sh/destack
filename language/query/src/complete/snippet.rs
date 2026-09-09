use crate::{CompletionInsertion, Snippet};

impl CompletionInsertion {
    /// Build a call insertion from already selected parameter names.
    pub(super) fn call<'a>(
        name: &str,
        parameters: impl ExactSizeIterator<Item = Option<&'a str>>,
    ) -> Self {
        if parameters.len() == 0 {
            return Self::Text(format!("{name}()"));
        }

        // write the callee and each parameter into one snippet
        let mut snippet = Snippet::default();
        snippet.write_text(name);
        snippet.write_text("(");
        for (index, parameter) in parameters.enumerate() {
            if index > 0 {
                snippet.write_text(", ");
            }
            snippet.write_placeholder(index + 1, parameter);
        }
        snippet.write_text(")");

        Self::Snippet(snippet.finish(true))
    }

    /// Build a struct expression from required field names.
    pub(super) fn struct_expression<'a>(
        name: &str,
        fields: impl ExactSizeIterator<Item = &'a str>,
    ) -> Self {
        if fields.len() == 0 {
            return Self::Text(format!("{name} {{}}"));
        }

        // write each required field and its value placeholder
        let mut snippet = Snippet::default();
        snippet.write_text(name);
        snippet.write_text(" { ");
        for (index, field) in fields.enumerate() {
            if index > 0 {
                snippet.write_text(", ");
            }
            snippet.write_text(field);
            snippet.write_text(": ");
            snippet.write_placeholder(index + 1, None);
        }
        snippet.write_text(" }");

        Self::Snippet(snippet.finish(true))
    }

    /// Build one object field and its value placeholder.
    pub(super) fn field(name: &str) -> Self {
        let mut snippet = Snippet::default();
        snippet.write_text(name);
        snippet.write_text(": ");
        snippet.write_placeholder(1, None);

        Self::Snippet(snippet.finish(false))
    }
}
