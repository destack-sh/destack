use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// Static timing tag for parser instrumentation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ParserTimingTag {
    name: &'static str,
}

impl ParserTimingTag {
    /// Create a timing tag from a static name.
    pub const fn new(name: &'static str) -> Self {
        Self { name }
    }

    /// Return the tag name.
    pub const fn name(self) -> &'static str {
        self.name
    }
}

/// Aggregated timing entry for parser tags.
#[derive(Debug, Clone, Copy)]
pub struct ParserTimingEntry {
    /// The timing tag name.
    pub name: &'static str,
    /// The total duration recorded for this tag.
    pub duration: Duration,
    /// The number of samples recorded.
    pub count: usize,
}

/// Timing collector for parser instrumentation.
#[derive(Debug, Default)]
pub struct ParserTimings {
    entries: RefCell<HashMap<&'static str, ParserTimingEntry>>,
}

impl ParserTimings {
    /// Record a timing sample.
    pub fn record(&self, tag: ParserTimingTag, duration: Duration) {
        let mut entries = self.entries.borrow_mut();
        let entry = entries.entry(tag.name()).or_insert(ParserTimingEntry {
            name: tag.name(),
            duration: Duration::new(0, 0),
            count: 0,
        });
        entry.duration += duration;
        entry.count += 1;
    }

    /// Snapshot current timing entries.
    pub fn snapshot(&self) -> Vec<ParserTimingEntry> {
        let entries = self.entries.borrow();
        let mut snapshot: Vec<ParserTimingEntry> = entries.values().copied().collect();
        snapshot.sort_by_key(|entry| entry.name);
        snapshot
    }
}

/// Scoped timing guard that records elapsed time on drop.
#[derive(Debug)]
pub struct ParserTimingScope {
    timings: Option<Rc<ParserTimings>>,
    name: &'static str,
    started_at: Option<Instant>,
}

impl ParserTimingScope {
    /// Start a timing scope if timings are enabled.
    pub fn new(timings: Option<Rc<ParserTimings>>, tag: ParserTimingTag) -> Self {
        let started_at = timings.is_some().then(Instant::now);
        Self {
            timings,
            name: tag.name(),
            started_at,
        }
    }
}

impl Drop for ParserTimingScope {
    fn drop(&mut self) {
        let Some(started_at) = self.started_at else {
            return;
        };
        let Some(timings) = self.timings.as_ref() else {
            return;
        };
        let elapsed = started_at.elapsed();
        timings.record(ParserTimingTag::new(self.name), elapsed);
    }
}

pub mod tags {
    use super::ParserTimingTag;

    pub const PARSE_BLOCK_BODY: ParserTimingTag = ParserTimingTag::new("parse.block.body");
    pub const PARSE_EXPRESSION: ParserTimingTag = ParserTimingTag::new("parse.expression");
    pub const PARSE_EXPRESSION_PRIMARY: ParserTimingTag =
        ParserTimingTag::new("parse.expression.primary");
    pub const PARSE_EXPRESSION_POSTFIX: ParserTimingTag =
        ParserTimingTag::new("parse.expression.postfix");
    pub const PARSE_EXPRESSION_INFIX: ParserTimingTag =
        ParserTimingTag::new("parse.expression.infix");
    pub const PARSE_EXPRESSION_PRIMARY_IDENTIFIER: ParserTimingTag =
        ParserTimingTag::new("parse.expression.primary.identifier");
    pub const PARSE_EXPRESSION_PRIMARY_LITERAL: ParserTimingTag =
        ParserTimingTag::new("parse.expression.primary.literal");
    pub const PARSE_EXPRESSION_PRIMARY_KEYWORD: ParserTimingTag =
        ParserTimingTag::new("parse.expression.primary.keyword");
    pub const PARSE_EXPRESSION_PRIMARY_GROUP: ParserTimingTag =
        ParserTimingTag::new("parse.expression.primary.group");
    pub const PARSE_EXPRESSION_POSTFIX_CALL: ParserTimingTag =
        ParserTimingTag::new("parse.expression.postfix.call");

    pub const PARSE_ANNOTATIONS_INDEX: ParserTimingTag =
        ParserTimingTag::new("parse.annotations.index");
    pub const PARSE_ANNOTATIONS_COLLECT: ParserTimingTag =
        ParserTimingTag::new("parse.annotations.collect");
    pub const PARSE_ANNOTATIONS_WRAPPERS: ParserTimingTag =
        ParserTimingTag::new("parse.annotations.wrappers");
    pub const PARSE_ANNOTATIONS_SIDE: ParserTimingTag =
        ParserTimingTag::new("parse.annotations.side");
    pub const PARSE_ANNOTATIONS_MAIN: ParserTimingTag =
        ParserTimingTag::new("parse.annotations.main");
    pub const PARSE_ANNOTATIONS_SORT: ParserTimingTag =
        ParserTimingTag::new("parse.annotations.sort");

    pub const PARSE_POSITIONS_BUILD: ParserTimingTag =
        ParserTimingTag::new("parse.positions.build");
    pub const PARSE_TYPE: ParserTimingTag = ParserTimingTag::new("parse.type");
    pub const PARSE_PATTERN: ParserTimingTag = ParserTimingTag::new("parse.pattern");
    pub const PARSE_PROPERTY: ParserTimingTag = ParserTimingTag::new("parse.property");
    pub const PARSE_LITERAL: ParserTimingTag = ParserTimingTag::new("parse.literal");
    pub const PARSE_ARGUMENT: ParserTimingTag = ParserTimingTag::new("parse.argument");
    pub const PARSE_PATH: ParserTimingTag = ParserTimingTag::new("parse.path");
    pub const PARSE_FUNCTION: ParserTimingTag = ParserTimingTag::new("parse.function");
    pub const PARSE_DECLARATION: ParserTimingTag = ParserTimingTag::new("parse.declaration");
    pub const PARSE_ENUM: ParserTimingTag = ParserTimingTag::new("parse.enum");
    pub const PARSE_STRUCT: ParserTimingTag = ParserTimingTag::new("parse.struct");
    pub const PARSE_NAMESPACE: ParserTimingTag = ParserTimingTag::new("parse.namespace");
    pub const PARSE_MATCH: ParserTimingTag = ParserTimingTag::new("parse.match");
    pub const PARSE_WHERE: ParserTimingTag = ParserTimingTag::new("parse.where");
    pub const PARSE_IMPORT: ParserTimingTag = ParserTimingTag::new("parse.import");
}
