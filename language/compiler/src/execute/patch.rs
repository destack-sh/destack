use crate::{Compiler, ExecuteError};

use destack_dir as dir;
use destack_source::{FileContent, ModuleId};
use destack_workspace::ComptimeOutput;

/// Patch information for a single comptime slot.
#[derive(Debug, Clone)]
pub(crate) struct ComptimePatch {
    /// The expression to replace.
    pub expression_id: dir::LocalNodeIdAny,
    /// The computed result.
    pub result: Option<ComptimeOutput>,
}

impl Compiler {
    /// Apply comptime results by patching the DIR in place.
    pub(crate) fn apply_comptime_patch(
        &self,
        module_id: ModuleId,
        tree: &mut dir::NodeTree,
        patch: ComptimePatch,
    ) {
        // resolve the expression to patch
        let expression_id = patch
            .expression_id
            .try_into_typed::<dir::Expression>()
            .map_err(|_| ExecuteError::UnsupportedConstruct {
                node: patch.expression_id.into_global(module_id),
            });
        let expression_id = match expression_id {
            Ok(expression_id) => expression_id,
            Err(error) => {
                self.error(error);
                return;
            }
        };

        // skip missing results
        let Some(output) = patch.result.as_ref() else {
            return;
        };

        // skip results that lack a dir value
        let Some(static_value) = output.dir.as_ref() else {
            self.error(ExecuteError::UnsupportedConstruct {
                node: patch.expression_id.into_global(module_id),
            });
            return;
        };

        // build the replacement expression from the static value
        let scope = tree.get_scope(expression_id);
        let replacement = match self.static_expression_to_expression(
            tree,
            module_id,
            expression_id.into_any(),
            expression_id.into_any(),
            scope,
            static_value,
        ) {
            Ok(replacement) => replacement,
            Err(error) => {
                self.error(error);
                return;
            }
        };
        tree.replace(expression_id, replacement);

        // attach a comment with the original comptime source
        if self.options.retain_comptime_as_comment {
            self.attach_comptime_comment(module_id, tree, expression_id);
        }
    }

    /// Attach a postfix comment with the original comptime source.
    fn attach_comptime_comment(
        &self,
        module_id: ModuleId,
        tree: &mut dir::NodeTree,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) {
        // derive the original comptime source snippet
        let Some(comment) = self.make_comptime_comment_text(module_id, tree, expression_id) else {
            return;
        };

        // skip if the same comment already exists
        let comment_id = self.program.strings.intern(&comment);
        if self.has_annotation_comment(tree, expression_id, comment_id) {
            return;
        }

        // attach the comment as a postfix annotation
        let scope = tree.get_scope(expression_id);
        let annotation = dir::Annotation::Comment {
            position: dir::AnnotationPosition::Postfix,
            string: comment_id,
        };
        let reserved = tree.reserve_from(
            dir::NodeType::Annotation,
            expression_id.into_any(),
            scope,
            None,
        );
        let annotation_id = tree.insert(reserved, annotation);
        tree.append_annotation(expression_id.into_any(), annotation_id);
    }

    /// Build the comment text from the original comptime source span.
    fn make_comptime_comment_text(
        &self,
        module_id: ModuleId,
        tree: &dir::NodeTree,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<String> {
        // resolve the source span for the comptime expression
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let ast = module.ast.as_ref()?;
        let source_id = tree.get_source(expression_id.id);
        let span = ast.tree.get_span_by_id(source_id);
        if span.is_empty() {
            return None;
        }

        // extract the source slice for the comptime expression
        let file = self.program.files.get(span.file);
        let content = match &file.content {
            FileContent::Text { content } => content.as_str(),
            FileContent::Json { content, .. } => content.as_str(),
            FileContent::Binary { .. } | FileContent::Unloaded => return None,
        };
        let slice = content.get(span.start as usize..span.end as usize)?;
        let trimmed = slice.trim();
        if trimmed.is_empty() {
            return None;
        }

        // normalize whitespace for a compact inline comment
        let max_length = self.options.retain_comptime_comment_max_length;
        if max_length == 0 {
            return None;
        }
        let compact = trimmed.split_whitespace().collect::<Vec<_>>().join(" ");
        if compact.chars().count() <= max_length {
            return Some(compact);
        }

        // truncate while preserving the requested cap
        if max_length <= 3 {
            return Some(compact.chars().take(max_length).collect());
        }
        let truncated: String = compact.chars().take(max_length - 3).collect();
        Some(format!("{truncated}..."))
    }

    /// Check whether an equivalent comment is already attached.
    fn has_annotation_comment(
        &self,
        tree: &dir::NodeTree,
        expression_id: dir::LocalNodeId<dir::Expression>,
        comment_id: destack_base::StringId,
    ) -> bool {
        tree.get_annotations(expression_id.id)
            .iter()
            .any(|annotation_id| {
                let annotation = tree.get(*annotation_id);
                matches!(
                    annotation,
                    dir::Annotation::Comment {
                        position: dir::AnnotationPosition::Postfix,
                        string,
                    } if *string == comment_id
                )
            })
    }
}
