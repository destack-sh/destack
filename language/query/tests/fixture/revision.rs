use std::path::PathBuf;

use destack_query::QueryMethod;
use indexmap::IndexMap;

use crate::MarkdownCase;

use super::{QueryAssertion, QueryChange, QueryFile, require_query_file};

/// One exact workspace revision in a query fixture.
#[derive(Debug, Clone)]
pub(super) struct QueryRevision {
    /// The atomic changes that produce this revision.
    pub(super) changes: Vec<QueryChange>,
    /// The query assertions executed at this revision.
    pub(super) assertions: Vec<QueryAssertion>,
}

impl QueryRevision {
    /// Parse every workspace revision and its exact query responses.
    pub(super) fn parse(
        markdown: &MarkdownCase,
        files: &IndexMap<PathBuf, QueryFile>,
        method: QueryMethod,
    ) -> Result<Vec<Self>, String> {
        let mut files = files.clone();
        let mut revisions = Vec::new();
        let mut changes = vec![Vec::new()];
        let mut assertions = Vec::new();
        let mut is_sequence_open = false;
        let mut index = 0;

        // consume assertions and workspace changes in declaration order
        while let Some(block) = markdown.blocks.get(index) {
            // append one query assertion
            if block.language.starts_with("query ") {
                let (assertion, applied_files, next_index) =
                    QueryAssertion::parse(&markdown.blocks, index, &files, method)?;
                assertions.push(assertion);
                index = next_index;

                // publish applied query edits in the next revision
                if let Some(applied_files) = applied_files {
                    Self::append(&mut revisions, changes, assertions)?;
                    changes = vec![Vec::new()];
                    assertions = Vec::new();
                    is_sequence_open = false;
                    for target in applied_files {
                        let current = require_query_file(&target.path, &files)?;
                        let change = QueryChange::edit(current, target)?;
                        change.apply(&mut files)?;
                        changes[0].push(change);
                    }
                }

                continue;
            }

            // require one recognized workspace change
            if !QueryChange::matches(block) {
                return Err(format!(
                    "query fixture expected a query or workspace change block, found '{}'",
                    block.language
                ));
            }

            // finish the preceding observed revision
            let is_sequential = QueryChange::is_sequential(block);
            if !assertions.is_empty() {
                Self::append(&mut revisions, changes, assertions)?;
                changes = vec![Vec::new()];
                assertions = Vec::new();
                is_sequence_open = false;
            }
            if is_sequence_open {
                return Err(
                    "sequential source changes must be the only changes in a revision".to_string(),
                );
            }

            // parse the declared atomic or sequential revisions
            let parsed = QueryChange::parse(block, &files)?;
            if is_sequential {
                if changes.len() != 1 || !changes[0].is_empty() {
                    return Err(
                        "sequential source changes must be the only changes in a revision"
                            .to_string(),
                    );
                }
                for change in &parsed {
                    change.apply(&mut files)?;
                }
                changes = parsed.into_iter().map(|change| vec![change]).collect();
                is_sequence_open = true;
            } else {
                for change in &parsed {
                    change.apply(&mut files)?;
                }
                changes[0].extend(parsed);
            }
            index += 1;
        }

        // finish the final observed revision
        Self::append(&mut revisions, changes, assertions)?;

        Ok(revisions)
    }

    /// Append atomic revisions with success checks before the exact final assertions.
    fn append(
        revisions: &mut Vec<Self>,
        mut changes: Vec<Vec<QueryChange>>,
        assertions: Vec<QueryAssertion>,
    ) -> Result<(), String> {
        if assertions.is_empty() {
            return Err("query fixture revision has no assertions".to_string());
        }
        let final_changes = changes
            .pop()
            .ok_or_else(|| "query fixture has no revision changes".to_string())?;

        // append intermediate revisions with success assertions
        for changes in changes {
            let assertions = assertions
                .iter()
                .map(QueryAssertion::require_success)
                .collect();
            revisions.push(Self {
                changes,
                assertions,
            });
        }

        // append the final revision with exact assertions
        revisions.push(Self {
            changes: final_changes,
            assertions,
        });

        Ok(())
    }
}
