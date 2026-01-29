use std::time::{Duration, Instant};

use crate::{Compiler, CompilerStats};

/// Static timing tag for section level instrumentation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct TimingTag {
    name: &'static str,
}

impl TimingTag {
    /// Create a timing tag from a static name.
    pub const fn new(name: &'static str) -> Self {
        Self { name }
    }

    /// Return the tag name.
    pub const fn name(self) -> &'static str {
        self.name
    }
}

/// Scoped timing guard that records elapsed time on drop.
#[derive(Debug)]
pub struct TimingScope<'a> {
    stats: &'a CompilerStats,
    name: &'static str,
    started_at: Option<Instant>,
}

impl<'a> TimingScope<'a> {
    /// Start a timing scope if timings are enabled.
    pub fn new(stats: &'a CompilerStats, tag: TimingTag) -> Self {
        let started_at = if stats.timings_enabled() {
            Some(Instant::now())
        } else {
            None
        };
        Self {
            stats,
            name: tag.name(),
            started_at,
        }
    }

    /// Record timing explicitly for an external duration.
    pub fn record(stats: &'a CompilerStats, tag: TimingTag, duration: Duration) {
        if stats.timings_enabled() {
            stats.record_timing(tag.name(), duration);
        }
    }
}

impl Drop for TimingScope<'_> {
    fn drop(&mut self) {
        let Some(started_at) = self.started_at else {
            return;
        };
        let elapsed = started_at.elapsed();
        self.stats.record_timing(self.name, elapsed);
    }
}

impl Compiler {
    /// Start a timing scope for the compiler.
    pub fn timing_scope(&self, tag: TimingTag) -> TimingScope<'_> {
        TimingScope::new(&self.stats, tag)
    }
}

pub mod tags {
    use super::TimingTag;

    pub const IMPORT_MODULE_PARSE: TimingTag = TimingTag::new("import.module.parse");
    pub const IMPORT_MODULE_BIND: TimingTag = TimingTag::new("import.module.bind");
    pub const IMPORT_MODULE_DESUGAR: TimingTag = TimingTag::new("import.module.desugar");
    pub const IMPORT_MODULE_VALIDATE: TimingTag = TimingTag::new("import.module.validate");

    pub const RESOLVE_BUILTINS: TimingTag = TimingTag::new("resolve.builtins");
    pub const RESOLVE_LIBS: TimingTag = TimingTag::new("resolve.libs");
    pub const RESOLVE_MODULE_PREPARE: TimingTag = TimingTag::new("resolve.module.prepare");
    pub const RESOLVE_MODULE_DIRECT: TimingTag = TimingTag::new("resolve.module.direct");
    pub const RESOLVE_MODULE_DEPENDENCIES: TimingTag =
        TimingTag::new("resolve.dependencies.resolve");
    pub const RESOLVE_MODULE_EXPRESSIONS: TimingTag = TimingTag::new("resolve.expressions.resolve");
    pub const RESOLVE_MODULE_DECLARATIONS: TimingTag =
        TimingTag::new("resolve.declarations.resolve");
    pub const RESOLVE_MODULE_EXPORTS: TimingTag = TimingTag::new("resolve.exports.finalize");
    pub const RESOLVE_MODULE_CANONICAL: TimingTag = TimingTag::new("resolve.module.canonicalize");

    pub const ANALYZE_MODULE_DECLARE: TimingTag = TimingTag::new("analyze.module.declare");
    pub const ANALYZE_DECLARE_TYPES: TimingTag = TimingTag::new("analyze.types.evaluate");
    pub const ANALYZE_DECLARE_DECLARATIONS: TimingTag =
        TimingTag::new("analyze.declarations.declare");
    pub const ANALYZE_DECLARE_DECORATORS: TimingTag = TimingTag::new("analyze.decorators.register");
    pub const ANALYZE_MODULE_EXPORT: TimingTag = TimingTag::new("analyze.module.export");
    pub const ANALYZE_EXPORT_VALUES: TimingTag = TimingTag::new("analyze.exports.declare");
    pub const ANALYZE_EXPORT_ALIASES: TimingTag = TimingTag::new("analyze.exports.materialize");
    pub const ANALYZE_EXPORT_NAMESPACE: TimingTag = TimingTag::new("analyze.namespace.declare");
    pub const ANALYZE_MODULE_INFER: TimingTag = TimingTag::new("analyze.module.infer");
    pub const ANALYZE_FLOW_GRAPH_BUILD: TimingTag = TimingTag::new("analyze.flow.graph.build");
    pub const ANALYZE_FLOW_TABLE_COMPUTE: TimingTag = TimingTag::new("analyze.flow.table.compute");
    pub const ANALYZE_EXPRESSION_INFER: TimingTag = TimingTag::new("analyze.expression.infer");
    pub const ANALYZE_INFER_REGISTER_INSTANCES: TimingTag =
        TimingTag::new("analyze.infer.instances.register");
    pub const ANALYZE_INFER_SOLVE_CONSTRAINTS: TimingTag =
        TimingTag::new("analyze.infer.constraints.solve");
    pub const ANALYZE_MODULE_CAPTURE: TimingTag = TimingTag::new("analyze.module.capture");
    pub const ANALYZE_MODULE_VALIDATE: TimingTag = TimingTag::new("analyze.module.validate");

    pub const ELABORATE_MODULE_TRANSFORM: TimingTag = TimingTag::new("elaborate.transform.apply");
    pub const ELABORATE_MODULE_REIFY: TimingTag = TimingTag::new("elaborate.reify.apply");

    pub const EXECUTE_MODULE_PREPARE: TimingTag = TimingTag::new("execute.module.prepare");
    pub const EXECUTE_MODULE_PATCH: TimingTag = TimingTag::new("execute.module.patch");
    pub const EXECUTE_EXPRESSION: TimingTag = TimingTag::new("execute.expression.run");

    pub const LOWER_MODULE: TimingTag = TimingTag::new("lower.module.emit");

    pub const OPTIMIZE_MODULE: TimingTag = TimingTag::new("optimize.module.run");
    pub const OPTIMIZE_PACKAGE: TimingTag = TimingTag::new("optimize.package.run");
    pub const OPTIMIZE_PROGRAM: TimingTag = TimingTag::new("optimize.program.run");

    pub const GENERATE_MODULE: TimingTag = TimingTag::new("generate.module.emit");

    pub const EMIT_MODULE: TimingTag = TimingTag::new("emit.module.write");
    pub const EMIT_PACKAGE: TimingTag = TimingTag::new("emit.package.write");
    pub const EMIT_PROGRAM: TimingTag = TimingTag::new("emit.program.write");

    pub const LINK_TARGET: TimingTag = TimingTag::new("link.target.run");

    pub const LINT_MODULE: TimingTag = TimingTag::new("lint.module.run");
    pub const LINT_PACKAGE: TimingTag = TimingTag::new("lint.package.run");
}
