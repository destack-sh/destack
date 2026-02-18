use ast::{LocalNodeId, NodeParentIndex, NodeTree, TokenSpan};
use destack_ast as ast;
use destack_source::{File, MultiSpan};
use smallvec::SmallVec;

use crate::format::context::{Annotation, FormatterAnnotationEntry};

use super::blank::resolve_formatter_blank_trivia_attachment;
use super::index::{build_formatter_trivia_owner_index, build_formatter_trivia_seam_index};
use super::resolve::resolve_formatter_comment_trivia_attachment;
pub(crate) fn build_formatter_annotation_projection(
    file: &File,
    tree: &NodeTree,
    tokens: &[TokenSpan],
    _side_tokens: &[TokenSpan],
    _side_span: &MultiSpan,
    parents: &NodeParentIndex,
) -> (
    Vec<FormatterAnnotationEntry>,
    Vec<SmallVec<[LocalNodeId<Annotation>; 4]>>,
) {
    let node_count = tree.next_id() as usize;
    let mut entries = Vec::new();
    let mut by_node_id = vec![SmallVec::new(); node_count];

    // add parser semantic annotations first
    for (&target_id, annotation_ids) in tree.get_all_annotations() {
        if target_id as usize >= by_node_id.len() {
            continue;
        }

        for &annotation_id in annotation_ids {
            let ast_annotation = tree.get(annotation_id);
            let annotation = match ast_annotation {
                ast::Annotation::Doc { node, position } => Annotation::Doc {
                    node: *node,
                    position: *position,
                },
                ast::Annotation::Decorator { node, position } => Annotation::Decorator {
                    node: *node,
                    position: *position,
                },
            };

            let local_id = LocalNodeId::new(entries.len() as u32);
            entries.push(FormatterAnnotationEntry {
                annotation,
                span: tree.get_span(annotation_id),
            });
            by_node_id[target_id as usize].push(local_id);
        }
    }

    // build formatter-side owner indexes for trivia placement
    let owner_index = build_formatter_trivia_owner_index(tree, tokens);
    let seam_index = build_formatter_trivia_seam_index(tree);

    // add comment trivia with formatter-side placement resolution
    for trivia in tree.comment_trivia().iter().copied() {
        let (target_id, position) = resolve_formatter_comment_trivia_attachment(
            file,
            tree,
            tokens,
            trivia,
            &owner_index,
            parents,
        );

        let Some(target_id) = target_id else {
            continue;
        };
        if target_id as usize >= by_node_id.len() {
            continue;
        }

        let local_id = LocalNodeId::new(entries.len() as u32);
        entries.push(FormatterAnnotationEntry {
            annotation: Annotation::Comment {
                node: trivia.comment,
                position,
            },
            span: trivia.span,
        });
        by_node_id[target_id as usize].push(local_id);
    }

    // add blank trivia with formatter-side placement resolution
    for trivia in tree.blank_trivia().iter().copied() {
        let (target_id, position) = resolve_formatter_blank_trivia_attachment(
            tree,
            tokens,
            trivia,
            &owner_index,
            &seam_index,
            parents,
        );

        let Some(target_id) = target_id else {
            continue;
        };
        if target_id as usize >= by_node_id.len() {
            continue;
        }

        let local_id = LocalNodeId::new(entries.len() as u32);
        entries.push(FormatterAnnotationEntry {
            annotation: Annotation::Blank {
                node: trivia.blank,
                position,
            },
            span: trivia.span,
        });
        by_node_id[target_id as usize].push(local_id);
    }

    // keep node-local annotation order source-stable
    for annotation_ids in &mut by_node_id {
        if annotation_ids.len() <= 1 {
            continue;
        }

        annotation_ids.sort_by(
            |left: &LocalNodeId<Annotation>, right: &LocalNodeId<Annotation>| {
                let left_span = entries[left.id as usize].span;
                let right_span = entries[right.id as usize].span;
                left_span
                    .start
                    .cmp(&right_span.start)
                    .then(left_span.end.cmp(&right_span.end))
                    .then(left.id.cmp(&right.id))
            },
        );
    }

    (entries, by_node_id)
}
