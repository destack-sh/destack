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

    // import
    pub const IMPORT_MODULE_PARSE: TimingTag = TimingTag::new("import.module.parse");
    pub const IMPORT_MODULE_PARSE_CACHE_READ: TimingTag =
        TimingTag::new("import.module.parse.cache.read");
    pub const IMPORT_MODULE_PARSE_CACHE_WRITE: TimingTag =
        TimingTag::new("import.module.parse.cache.write");
    pub const IMPORT_MODULE_PARSE_LEX: TimingTag = TimingTag::new("import.module.parse.lex");
    pub const IMPORT_MODULE_PARSE_TREE: TimingTag = TimingTag::new("import.module.parse.tree");
    pub const IMPORT_MODULE_BIND: TimingTag = TimingTag::new("import.module.bind");
    pub const IMPORT_MODULE_DESUGAR: TimingTag = TimingTag::new("import.module.desugar");
    pub const IMPORT_MODULE_VALIDATE: TimingTag = TimingTag::new("import.module.validate");

    // resolve
    pub const RESOLVE_BUILTINS: TimingTag = TimingTag::new("resolve.builtins");
    pub const RESOLVE_LIBS: TimingTag = TimingTag::new("resolve.libs");
    pub const RESOLVE_LIBS_DEPENDENCIES: TimingTag = TimingTag::new("resolve.libs.dependencies");
    pub const RESOLVE_LIBS_CONFLICTS: TimingTag = TimingTag::new("resolve.libs.conflicts");
    pub const RESOLVE_LIBS_LOAD_MODULES: TimingTag = TimingTag::new("resolve.libs.load");
    pub const RESOLVE_LIBS_DEPENDENCY_ITEMS: TimingTag =
        TimingTag::new("resolve.libs.dependencies.items");
    pub const RESOLVE_LIBS_GLOBAL_CACHE: TimingTag = TimingTag::new("resolve.libs.cache.global");
    pub const RESOLVE_LIBS_DECLARED_NAMES: TimingTag =
        TimingTag::new("resolve.libs.declared.names");
    pub const RESOLVE_LIBS_DECLARED_SYMBOLS: TimingTag =
        TimingTag::new("resolve.libs.declared.symbols");
    pub const RESOLVE_LIBS_AMBIENT_SYMBOLS: TimingTag =
        TimingTag::new("resolve.libs.ambient.symbols");
    pub const RESOLVE_LIBS_AMBIENT_SOURCES: TimingTag =
        TimingTag::new("resolve.libs.ambient.sources");
    pub const RESOLVE_LIBS_WELL_KNOWN: TimingTag = TimingTag::new("resolve.libs.well_known");
    pub const RESOLVE_MODULE_PREPARE: TimingTag = TimingTag::new("resolve.module.prepare");
    pub const RESOLVE_MODULE_PREPARE_STATIC_IF: TimingTag =
        TimingTag::new("resolve.module.prepare.static_if");
    pub const RESOLVE_MODULE_PREPARE_EXPORTS: TimingTag =
        TimingTag::new("resolve.module.prepare.exports");
    pub const RESOLVE_MODULE_PREPARE_BINDING_EXPORTS: TimingTag =
        TimingTag::new("resolve.module.prepare.binding_exports");
    pub const RESOLVE_MODULE_PREPARE_CACHE_WRITE: TimingTag =
        TimingTag::new("resolve.module.prepare.cache.write");
    pub const RESOLVE_MODULE_DIRECT: TimingTag = TimingTag::new("resolve.module.direct");
    pub const RESOLVE_MODULE_DEPENDENCIES: TimingTag =
        TimingTag::new("resolve.dependencies.resolve");
    pub const RESOLVE_MODULE_EXPRESSIONS: TimingTag = TimingTag::new("resolve.expressions.resolve");
    pub const RESOLVE_MODULE_DECLARATIONS: TimingTag =
        TimingTag::new("resolve.declarations.resolve");
    pub const RESOLVE_MODULE_EXPORTS: TimingTag = TimingTag::new("resolve.exports.finalize");
    pub const RESOLVE_MODULE_CANONICAL: TimingTag = TimingTag::new("resolve.module.canonicalize");
    pub const RESOLVE_DEPENDENCY_ITEM_IMPORT: TimingTag =
        TimingTag::new("resolve.dependency.item.import");
    pub const RESOLVE_DEPENDENCY_ITEM_SYMBOL: TimingTag =
        TimingTag::new("resolve.dependency.item.symbol");
    pub const RESOLVE_DEPENDENCY_ITEM_NAMESPACE: TimingTag =
        TimingTag::new("resolve.dependency.item.namespace");

    // analyze declare
    pub const ANALYZE_MODULE_DECLARE: TimingTag = TimingTag::new("analyze.module.declare");
    pub const ANALYZE_DECLARE_TYPES: TimingTag = TimingTag::new("analyze.types.evaluate");
    pub const ANALYZE_TYPES_EVALUATE_EXPRESSION: TimingTag =
        TimingTag::new("analyze.types.evaluate.expression");
    pub const ANALYZE_TYPES_EVALUATE_EXPRESSION_LITERAL: TimingTag =
        TimingTag::new("analyze.types.evaluate.expression.literal");
    pub const ANALYZE_TYPES_EVALUATE_EXPRESSION_TYPE_OP: TimingTag =
        TimingTag::new("analyze.types.evaluate.expression.type_op");
    pub const ANALYZE_TYPES_EVALUATE_SIGNATURE: TimingTag =
        TimingTag::new("analyze.types.evaluate.signature");
    pub const ANALYZE_TYPES_EVALUATE_TEMPLATE: TimingTag =
        TimingTag::new("analyze.types.evaluate.template");
    pub const ANALYZE_TYPES_EVALUATE_TYPEOF: TimingTag =
        TimingTag::new("analyze.types.evaluate.typeof");
    pub const ANALYZE_TYPES_EVALUATE_CONDITIONAL: TimingTag =
        TimingTag::new("analyze.types.evaluate.conditional");
    pub const ANALYZE_TYPES_EVALUATE_MAPPED: TimingTag =
        TimingTag::new("analyze.types.evaluate.mapped");
    pub const ANALYZE_TYPES_EVALUATE_INDEX: TimingTag =
        TimingTag::new("analyze.types.evaluate.index");
    pub const ANALYZE_TYPES_EVALUATE_REFERENCE: TimingTag =
        TimingTag::new("analyze.types.evaluate.reference");
    pub const ANALYZE_TYPES_EVALUATE_REFERENCE_CANONICAL: TimingTag =
        TimingTag::new("analyze.types.evaluate.reference.canonical");
    pub const ANALYZE_TYPES_EVALUATE_REFERENCE_ARGUMENTS: TimingTag =
        TimingTag::new("analyze.types.evaluate.reference.arguments");
    pub const ANALYZE_TYPES_EVALUATE_REFERENCE_WELL_KNOWN: TimingTag =
        TimingTag::new("analyze.types.evaluate.reference.well_known");
    pub const ANALYZE_DECLARE_DECLARATIONS: TimingTag =
        TimingTag::new("analyze.declarations.declare");
    pub const ANALYZE_DECLARE_DECORATORS: TimingTag = TimingTag::new("analyze.decorators.register");

    // analyze export
    pub const ANALYZE_MODULE_EXPORT: TimingTag = TimingTag::new("analyze.module.export");
    pub const ANALYZE_EXPORT_VALUES: TimingTag = TimingTag::new("analyze.exports.declare");
    pub const ANALYZE_EXPORT_ALIASES: TimingTag = TimingTag::new("analyze.exports.materialize");
    pub const ANALYZE_EXPORT_NAMESPACE: TimingTag = TimingTag::new("analyze.namespace.declare");

    // analyze infer
    pub const ANALYZE_MODULE_INFER: TimingTag = TimingTag::new("analyze.module.infer");
    pub const ANALYZE_FLOW_REQUIREMENTS: TimingTag = TimingTag::new("analyze.flow.requirements");
    pub const ANALYZE_FLOW_GRAPH_BUILD: TimingTag = TimingTag::new("analyze.flow.graph.build");
    pub const ANALYZE_FLOW_TABLE_COMPUTE: TimingTag = TimingTag::new("analyze.flow.table.compute");
    pub const ANALYZE_EXPRESSION_INFER: TimingTag = TimingTag::new("analyze.expression.infer");
    pub const ANALYZE_INFER_EXPRESSION_CALL: TimingTag =
        TimingTag::new("analyze.infer.expression.call");
    pub const ANALYZE_INFER_EXPRESSION_MEMBER: TimingTag =
        TimingTag::new("analyze.infer.expression.member");
    pub const ANALYZE_INFER_EXPRESSION_OPERATOR: TimingTag =
        TimingTag::new("analyze.infer.expression.operator");
    pub const ANALYZE_INFER_EXPRESSION_LITERAL: TimingTag =
        TimingTag::new("analyze.infer.expression.literal");
    pub const ANALYZE_INFER_EXPRESSION_TEMPLATE: TimingTag =
        TimingTag::new("analyze.infer.expression.template");
    pub const ANALYZE_INFER_EXPRESSION_REFERENCE: TimingTag =
        TimingTag::new("analyze.infer.expression.reference");
    pub const ANALYZE_INFER_REGISTER_INSTANCES: TimingTag =
        TimingTag::new("analyze.infer.instances.register");
    pub const ANALYZE_INFER_SOLVE_CONSTRAINTS: TimingTag =
        TimingTag::new("analyze.infer.constraints.solve");
    pub const ANALYZE_INFER_ASSIGN_CHECK: TimingTag = TimingTag::new("analyze.infer.assign.check");
    pub const ANALYZE_INFER_TYPE_NORMALIZE: TimingTag =
        TimingTag::new("analyze.infer.type.normalize");
    pub const ANALYZE_INFER_TYPE_MATERIALIZE: TimingTag =
        TimingTag::new("analyze.infer.type.materialize");
    pub const ANALYZE_INFER_TYPE_SUBSTITUTE: TimingTag =
        TimingTag::new("analyze.infer.type.substitute");
    pub const ANALYZE_INFER_TYPE_APPARENT: TimingTag =
        TimingTag::new("analyze.infer.type.apparent");
    pub const ANALYZE_INFER_STATIC_RESOLVE: TimingTag =
        TimingTag::new("analyze.infer.static.resolve");
    pub const ANALYZE_INFER_STATIC_EVALUATE: TimingTag =
        TimingTag::new("analyze.infer.static.evaluate");
    pub const ANALYZE_INFER_STATIC_MATERIALIZE: TimingTag =
        TimingTag::new("analyze.infer.static.materialize");
    pub const ANALYZE_INFER_OVERLOAD_RESOLVE: TimingTag =
        TimingTag::new("analyze.infer.overload.resolve");
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
