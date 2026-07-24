use std::ops::Range;

use super::{QueryCall, QueryFile, ResponseRows};

/// The exact expected result of one query call.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum QueryExpectation {
    /// The exact response rows.
    Rows {
        /// The parsed response rows.
        rows: ResponseRows,
        /// The response content range in the Markdown file.
        content_range: Range<usize>,
        /// The exact fenced response body.
        body: String,
    },
    /// The complete files produced by one edit response.
    Files(Vec<QueryFile>),
}

/// One query call and its exact expected result.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct QueryAssertion {
    /// The query call.
    pub(super) call: QueryCall,
    /// The exact expected response.
    pub(super) expected: QueryExpectation,
}
