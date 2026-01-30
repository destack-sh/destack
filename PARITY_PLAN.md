# Optimizer Parity Plan

## Goal
Close obvious conceptual gaps that prevent top‑tier optimization, using Destack’s full‑stack integration across language, libraries, runtime, and tooling.

## Principles
Treat correctness as a hard constraint and performance as a measurable outcome.
Prioritize wins that are only possible in an integrated TS/TS++ stack.
Prefer end‑to‑end pipeline wins over micro‑optimizations that increase compile time without measurable gains.
Make every optimization explainable, testable, and togglable.
Extract shared logic into `language/compiler/src/optimize/common/` whenever it generalizes.

## Success Metrics
Maintain a living “gap list” with owners, blockers, and exit criteria.
Track compile time, peak memory, runtime, and code size for internal workloads once a harness exists.
Require each gap closure to introduce tests and measurable pipeline invariants.
Report regressions immediately and block merges that exceed thresholds.

## Findings (In Progress)
- [fixed] Call target resolution was optimistic for virtual/interface dispatch and could drive stack/lifetime verification with an under-approximate target set.
- [fixed] Restrict call target resolution to direct call edges so virtual/interface/indirect dispatch fall back to signature-based rules.

## Baseline Infrastructure
Create a reproducible internal benchmarking harness with fixed toolchain versions and flags.
Seed the harness with small microbenchmarks and compiler throughput tests until real suites exist.
Reuse `language/test/src/optimize/` suites and `language/test/fixtures/mirbench/` as the initial seed corpus.
Integrate PGO capture from TS++ libraries and apps, and export canonical profiles.
Add A/B pipeline testing to compare candidate passes and options.
Log per‑pass timings and per‑pass code size deltas to identify ROI.

## Phase 0: Alignment and Ground Truth
Document language semantics that affect optimization safety, including aliasing, lifetime, and undefined behavior rules.
Define the canonical optimization levels and what they are allowed to do, following `language/compiler/src/optimize/README.md`.
List all profiling metadata and semantic metadata available from the TS++ ecosystem.
Create a mapping from metadata → optimization opportunities.
Confirm IR invariants and validation passes for each stage.
Build and maintain a “conceptual gap” inventory, including missing analyses, missing passes, and missing metadata.

## Phase 1: Analysis Precision
Strengthen alias analysis with stack escape, scoped noalias, and metadata aware reasoning.
Add flow‑sensitive escape analysis for stack, owned, and managed references.
Improve range, SCEV, and value numbering to unlock loop and scalar opts.
Add function summary analyses for interprocedural propagation.
Ensure memory SSA captures all side effects conservatively when metadata is absent.

## Phase 2: Scalar Optimization Core
Improve GVN, PRE, and DCE to handle common TS++ patterns and library idioms.
Expand instruction combining with type‑aware and semantic‑aware folds.
Enhance constant propagation with aggressive but safe folding for intrinsics.
Add canonicalization passes that normalize patterns for later passes.
Add targeted peephole optimizations for known library hot paths.

## Phase 3: Loop and Control Flow Optimization
Improve loop analysis, canonicalization, and induction variable recognition.
Add loop invariant code motion with stronger alias and effect checks.
Implement strength reduction, unrolling, and fusion with profile guidance.
Add loop distribution and vectorization preconditions and guards when vector work is active.
Introduce speculative execution with precise exception and side‑effect modeling, gated by O‑level policy.

## Phase 4: Memory and Allocation Optimization
Improve SROA and scalar replacement on managed and owned aggregates.
Optimize allocation patterns using escape analysis and lifetime metadata.
Add partial and full stack promotion for non‑escaping allocations.
Introduce allocator specialization and region inference using TS++ library data.
Reduce GC barriers with precise write‑barrier analysis and effect tracking.

## Phase 5: Interprocedural and Whole‑Program
Improve inlining with PGO data and semantic size modeling.
Add argument promotion and specialization where profile data supports it.
Propagate noalias and lifetime metadata across call boundaries.
Enable whole‑program DCE, devirtualization, and monomorphization where safe.
Cache and reuse summaries across builds for incremental compiles.

## Phase 6: Codegen and Backend Integration
Align MIR patterns with Cranelift strengths and add backend‑aware canonicalizations.
Introduce target‑specific lowering for vector/tensor operations.
Use profile data to guide branch layout, block ordering, and scheduling.
Add PGO‑driven inlining and code layout for hot paths.
Measure backend instruction selection quality and add mid‑end rewrites to improve it.

## Phase 7: Library and Runtime Co‑Design
Formalize an optimization contract for TS++ standard libraries.
Add metadata emission from libraries for purity, aliasing, and allocation behavior.
Introduce library‑level intrinsics for common data structures and iterators.
Enable cross‑module specialization for library generics with real workloads.
Validate that library upgrades preserve optimization semantics.

## Phase 8: Benchmarking, Validation, and Regression Prevention
Add golden output tests for optimizer transformations with full‑output comparisons.
Require every new optimization to add at least one targeted regression test.
Run nightly benchmark suites and publish trend dashboards once suites exist.
Automate bisects on performance regressions.
Add fuzzing for MIR optimizer pipelines to catch miscompilations.

## Phase 9: Vector and Tensor Optimization (Deferred)
Define a canonical vector and tensor IR lowering strategy with legal and fast paths.
Add algebraic simplification and shape‑aware rewrites for tensor ops.
Integrate MLIR‑style pattern rewriting infrastructure for tensor algebra.
Use layout metadata to choose optimal memory orders and tiling.
Enable hardware vectorization and SIMD lowering through Cranelift extensions where possible.

## Deliverables
Gap inventory with priorities, owners, and exit criteria.
Optimization roadmap with per‑pass goals and measurable wins.
Integrated metadata contract between TS++ libraries and the optimizer.
Performance dashboards and automated regression alerts.
Quarterly internal reports on closed gaps and remaining blockers.

## TODO Checklist
Severity tags: [high], [medium], [low].
- [ ] [high] Define a minimal internal benchmark harness and seed workloads.
- [ ] [medium] Add a perf‑mode runner to `language/test/src/optimize/` that captures compile time, runtime, and code size deltas for fixtures.
- [ ] [medium] Curate `language/test/fixtures/mirbench/` as an initial seed corpus, with notes on what each case stresses.
- [ ] [medium] Add `url_decode` in `dispatch` with expected output and validate or execute parity checks.
- [ ] [medium] Add `log_entry_parse` in `dispatch` with expected output and validate or execute parity checks.
- [ ] [medium] Add `json_tiny_parse` in `dispatch` with expected output and validate or execute parity checks.
- [ ] [medium] Add `ini_parser` in `dispatch` with expected output and validate or execute parity checks.
- [ ] [medium] Add `lru_cache` in `memory` with expected output and validate or execute parity checks.
- [ ] [medium] Add `template_render` in `dispatch` with expected output and validate or execute parity checks.
- [ ] [medium] Add `http_header_parse` in `dispatch` with expected output and validate or execute parity checks.
- [ ] [medium] Add `yield_stream_join` in `calls` with expected output and validate or execute parity checks.
- [ ] [medium] Add `object_pool` in `memory` with expected output and validate or execute parity checks.
- [ ] [medium] Add `chunked_decode` in `dispatch` with expected output and validate or execute parity checks.
- [ ] [medium] Add `cookie_parse` in `dispatch` with expected output and validate or execute parity checks.
- [ ] [medium] Add `template_conditional` in `dispatch` with expected output and validate or execute parity checks.
- [ ] [medium] Add `header_map_merge` in `memory` with expected output and validate or execute parity checks.
- [ ] [medium] Add `yield_batch_filter` in `calls` with expected output and validate or execute parity checks.
- [ ] [medium] Add `managed_vec_compact` in `memory` with expected output and validate or execute parity checks.
- [ ] Document language optimization semantics, aliasing, and UB rules.
- [ ] Map TS++ library metadata to optimizer hooks.
- [ ] Build A/B pipeline and per‑pass profiling infrastructure.
- [ ] Implement missing analysis precision features for alias, escape, and SCEV.
- [ ] Expand scalar optimizations with TS++ idiom coverage.
- [ ] Improve loop opts with PGO‑guided heuristics.
- [ ] Implement allocation and lifetime‑guided memory optimization.
- [ ] Add interprocedural summaries and specialization.
- [ ] Integrate backend‑aware rewrites for Cranelift.
- [ ] Formalize library optimization contracts and intrinsics.
- [ ] Add fuzzing and regression automation for optimizer pipelines.
- [ ] [low] Defer vector and tensor canonicalization and lowering until codegen and lowering mature.
- [ ] [low] Extract shared logic into `language/compiler/src/optimize/common/` as it emerges.

## Decisions Captured
Unix‑style targets are first class, with Linux and macOS prioritized and Windows as a nice‑to‑have.
Optimization levels and compile‑time budgets follow `language/compiler/src/optimize/README.md`.
Speculative optimization and devirtualization aggressiveness follow O‑level policy from `language/compiler/src/optimize/README.md`.
Vector and tensor workloads are deferred until lowering and codegen maturity improves.
Types do not alias unless accessed via raw pointers, per `language/DESIGN.md` and `language/SPECIFICATION.md`.
MIR metadata is the canonical carrier for profile data, sourced from owned libraries and toolchain.
No internal benchmark suites exist yet, so the initial harness will rely on `language/test/src/optimize/` and `language/test/fixtures/mirbench/` as a temporary seed.

## Questions and Decisions Needed
Which internal workloads should seed the benchmark harness in the absence of suites.
When vector and tensor work starts, which workloads define success.
How to prioritize runtime speed, code size, and compile time when O‑level policies conflict.
