#[cfg(feature = "timings")]
use std::cell::RefCell;
#[cfg(feature = "timings")]
use std::collections::HashMap;
use std::ptr::NonNull;
use std::time::Duration;
#[cfg(feature = "timings")]
use std::time::Instant;

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
    /// The inclusive duration recorded for this tag.
    pub duration: Duration,
    /// The self duration recorded for this tag.
    pub self_duration: Duration,
    /// The number of samples recorded.
    pub count: usize,
}

/// Timing collector for parser instrumentation.
#[cfg(feature = "timings")]
#[derive(Debug, Default)]
pub struct ParserTimings {
    entries: RefCell<HashMap<&'static str, ParserTimingEntry>>,
    scope_stack: RefCell<Vec<Duration>>,
}

/// Timing collector for parser instrumentation.
#[cfg(not(feature = "timings"))]
#[derive(Debug, Default)]
pub struct ParserTimings;

#[cfg(feature = "timings")]
impl ParserTimings {
    /// Record a timing sample.
    pub fn record(&self, tag: ParserTimingTag, duration: Duration) {
        self.record_scoped(tag, duration, duration);
    }

    /// Record a timing sample with explicit inclusive and self durations.
    fn record_scoped(&self, tag: ParserTimingTag, duration: Duration, self_duration: Duration) {
        let mut entries = self.entries.borrow_mut();
        let entry = entries.entry(tag.name()).or_insert(ParserTimingEntry {
            name: tag.name(),
            duration: Duration::new(0, 0),
            self_duration: Duration::new(0, 0),
            count: 0,
        });
        entry.duration += duration;
        entry.self_duration += self_duration;
        entry.count += 1;
    }

    /// Enter a timed scope.
    fn enter_scope(&self) {
        let mut scope_stack = self.scope_stack.borrow_mut();
        scope_stack.push(Duration::new(0, 0));
    }

    /// Exit a timed scope and attribute elapsed time.
    fn leave_scope(&self, tag: ParserTimingTag, elapsed: Duration) {
        let mut scope_stack = self.scope_stack.borrow_mut();
        let child_duration = scope_stack.pop().unwrap_or_default();
        if let Some(parent_child_duration) = scope_stack.last_mut() {
            *parent_child_duration += elapsed;
        }
        drop(scope_stack);

        let self_duration = elapsed.saturating_sub(child_duration);
        self.record_scoped(tag, elapsed, self_duration);
    }

    /// Snapshot current timing entries.
    pub fn snapshot(&self) -> Vec<ParserTimingEntry> {
        let entries = self.entries.borrow();
        let mut snapshot: Vec<ParserTimingEntry> = entries.values().copied().collect();
        snapshot.sort_by_key(|entry| entry.name);
        snapshot
    }
}

#[cfg(not(feature = "timings"))]
impl ParserTimings {
    /// Record a timing sample.
    #[inline]
    pub fn record(&self, _tag: ParserTimingTag, _duration: Duration) {}

    /// Snapshot current timing entries.
    #[inline]
    pub fn snapshot(&self) -> Vec<ParserTimingEntry> {
        Vec::new()
    }
}

/// Scoped timing guard that records elapsed time on drop.
#[cfg(feature = "timings")]
#[derive(Debug)]
pub struct ParserTimingScope {
    timings: Option<NonNull<ParserTimings>>,
    name: &'static str,
    started_at: Option<Instant>,
}

/// Scoped timing guard that records elapsed time on drop.
#[cfg(not(feature = "timings"))]
#[derive(Debug, Clone, Copy, Default)]
pub struct ParserTimingScope;

#[cfg(feature = "timings")]
impl ParserTimingScope {
    /// Create a disabled timing scope.
    #[inline]
    pub const fn disabled() -> Self {
        Self {
            timings: None,
            name: "",
            started_at: None,
        }
    }

    /// Start a timing scope if timings are enabled.
    #[inline]
    pub fn new(timings: Option<NonNull<ParserTimings>>, tag: ParserTimingTag) -> Self {
        if let Some(timings_ptr) = timings {
            // pointer originates from parser-owned Rc and stays valid for parser lifetime
            unsafe {
                timings_ptr.as_ref().enter_scope();
            }
        }
        let started_at = timings.is_some().then(Instant::now);
        Self {
            timings,
            name: tag.name(),
            started_at,
        }
    }
}

#[cfg(not(feature = "timings"))]
impl ParserTimingScope {
    /// Create a disabled timing scope.
    #[inline]
    pub const fn disabled() -> Self {
        Self
    }

    /// Start a timing scope if timings are enabled.
    #[inline]
    pub const fn new(_timings: Option<NonNull<ParserTimings>>, _tag: ParserTimingTag) -> Self {
        Self
    }
}

#[cfg(feature = "timings")]
impl Drop for ParserTimingScope {
    #[inline]
    fn drop(&mut self) {
        let Some(started_at) = self.started_at else {
            return;
        };
        let Some(timings) = self.timings else {
            return;
        };
        let elapsed = started_at.elapsed();

        // pointer originates from parser-owned Rc and stays valid for parser lifetime
        unsafe {
            timings
                .as_ref()
                .leave_scope(ParserTimingTag::new(self.name), elapsed);
        }
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
    pub const PARSE_KEYWORD_DECLARATION: ParserTimingTag =
        ParserTimingTag::new("parse.keyword.declaration");
    pub const PARSE_KEYWORD_BINDING: ParserTimingTag =
        ParserTimingTag::new("parse.keyword.binding");
    pub const PARSE_KEYWORD_CONTROL: ParserTimingTag =
        ParserTimingTag::new("parse.keyword.control");
    pub const PARSE_KEYWORD_DEPENDENCY: ParserTimingTag =
        ParserTimingTag::new("parse.keyword.dependency");
    pub const PARSE_KEYWORD_EXPRESSION: ParserTimingTag =
        ParserTimingTag::new("parse.keyword.expression");

    pub const PARSE_COMMENTS: ParserTimingTag = ParserTimingTag::new("parse.comments");

    pub const PARSE_POSITIONS_BUILD: ParserTimingTag =
        ParserTimingTag::new("parse.positions.build");
    pub const PARSE_ALLOC_NODE: ParserTimingTag = ParserTimingTag::new("parse.alloc.node");
    pub const PARSE_ALLOC_STRING_INTERN: ParserTimingTag =
        ParserTimingTag::new("parse.alloc.string_intern");
    pub const PARSE_ALLOC_IDENTIFIER_INTERN: ParserTimingTag =
        ParserTimingTag::new("parse.alloc.identifier_intern");
    pub const PARSE_ALLOC_MARK: ParserTimingTag = ParserTimingTag::new("parse.alloc.mark");
    pub const PARSE_ALLOC_RESTORE: ParserTimingTag = ParserTimingTag::new("parse.alloc.restore");
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
    pub const PARSE_INTERFACE: ParserTimingTag = ParserTimingTag::new("parse.interface");
    pub const PARSE_TYPE_DECLARATION: ParserTimingTag =
        ParserTimingTag::new("parse.type.declaration");
    pub const PARSE_NAMESPACE: ParserTimingTag = ParserTimingTag::new("parse.namespace");
    pub const PARSE_LET: ParserTimingTag = ParserTimingTag::new("parse.let");
    pub const PARSE_USING: ParserTimingTag = ParserTimingTag::new("parse.using");
    pub const PARSE_DECLARATOR: ParserTimingTag = ParserTimingTag::new("parse.declarator");
    pub const PARSE_MATCH: ParserTimingTag = ParserTimingTag::new("parse.match");
    pub const PARSE_WHERE: ParserTimingTag = ParserTimingTag::new("parse.where");
    pub const PARSE_IMPORT: ParserTimingTag = ParserTimingTag::new("parse.import");
}
