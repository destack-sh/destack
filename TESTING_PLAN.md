# Testing Plan

This document describes the intended long term architecture for Destack testing.
It is a design and migration plan, not a statement that all listed suites already exist or pass.

## Goals

The primary goal is to make Destack testing coherent, scalable, and explicit about what compatibility claims each suite supports.
The plan should make it easy to add large imported conformance suites without contorting the internal fixture tree.
The plan should preserve a clean authoring model for first party tests while still supporting imported and adapted external suites.
The plan should support structural reporting in [TESTING.md](/Users/florian/symbol/destack-1/TESTING.md) and in the root [language/test/README.md](/Users/florian/symbol/destack-1/language/test/README.md).
The plan should describe the intended final shape rather than a reduced initial scope.
The plan should allow the scaffolding, metadata, and reporting to land before every target suite is implemented or passing.

## Master Checklist

- [x] Write the architecture plan.
- [x] Finalize the top level taxonomy and migration boundaries for the final shape.
- [x] Add shared `conformance` scaffolding under `language/test/src/`.
- [x] Add shared `fixtures/conformance/` scaffolding under `language/test/fixtures/`.
- [x] Add shared JSON suite metadata loading and validation.
- [x] Add shared `suite.json` and `status.json` files for the final conformance roots.
- [x] Add shared JSON expectations loading, matching, and update commands.
- [x] Add shared conformance reporting and generated markdown section updates.
- [x] Update [TESTING.md](/Users/florian/symbol/destack-1/TESTING.md) to include the generated conformance catalog table.
- [x] Update [language/test/README.md](/Users/florian/symbol/destack-1/language/test/README.md) to describe the new taxonomy.
- [x] Migrate parser conformance onto the shared catalog and reporter.
- [x] Migrate formatter conformance onto the shared catalog and reporter.
- [x] Create a dedicated `regression` suite family.
- [ ] Convert flat `known-failures.txt` and `ignored.txt` files to structured JSON expectations across all suite families.
- [ ] Define the adaptation workflow for imported `.js` suites that become runnable Destack fixtures.
- [ ] Land `ecma/test262` on the new architecture.
- [ ] Land runnable Web feature suites on the new architecture.
- [ ] Land runnable `web/webgpu` coverage if WebGPU is part of the compatibility claim.
- [ ] Land runnable Node feature suites on the new architecture.
- [ ] Land targeted `ecma/v8` imports.
- [ ] Land targeted `ecma/jsc` imports.
- [ ] Add builtin API surface and contract testing.
- [ ] Add differential testing harnesses where they materially help.
- [ ] Add property based and fuzz testing hooks to the shared reporting model.
- [ ] Add target and environment aware CI routing for hardware backed suites.
- [ ] Audit every imported conformance harness for missing or incorrect oracles, expectations, and parity checks before expanding suite coverage further.
- [ ] Audit the wider `language/test/src` harness layer for hidden suite semantics, misleading aggregation, and other ad hoc behavior during the runner reorg.
- [ ] Audit `specification` and `mdtest` semantics during the runner reorg and decide what structural cleanup is actually needed.
- [ ] Slice the migration into reviewable check-ins: conformance infra, inherited suite cleanup, then translated pilot suites.

## Primary Shape

The top level language test organization should be:

- `specification`
- `conformance`
- `ecosystem`
- `regression`
- `smoke`
- `stress`

The `conformance` family should then be organized by compatibility claim:

- `ecma`
- `web`
- `node`
- `formatter`

The intended next level depends on the domain:

- `ecma`: corpus first
- `web`: claimed API surface first
- `node`: claimed module or platform surface first
- `formatter`: corpus first

Representative examples are:

- `ecma/test262`
- `ecma/v8`
- `ecma/jsc`
- `web/fetch`
- `web/webcrypto`
- `web/webaudio`
- `web/webgpu`
- `node/fs`
- `node/crypto`
- `node/process`
- `formatter/prettier`
- `formatter/oxfmt`

The tree should stay shallow where possible.
Avoid extra grouping layers like `engines/` unless they add real clarity.
For engine derived ECMA corpora, the intended shape is the flat form `ecma/v8` and `ecma/jsc`.

## Principles

First party suites should be organized around Destack semantics and subsystems.
Imported suites should be organized around the compatibility claim and upstream corpus they support.
Execution capability such as parse, check, run, emit, format, and LSP should be expressed in runner logic and metadata, not as the main fixture tree.
Environment and hardware requirements should be expressed in metadata and CI routing, not as directory roots.
Provenance should be documented and recorded in metadata where needed, but should not dominate the filesystem layout.
Known failures and ignores should move from flat text files to structured data.
Adapted imported cases should keep source `.js` files and translated `.ts` or `.ds` files adjacent inside the checked in suite `tests/` tree.

## Target Map

The conformance target map should look roughly like this.

| Domain | Suite | Main purpose | Notes | Priority |
| --- | --- | --- | --- | --- |
| `ecma` | `test262` | Core ECMA language and library overlap | Includes most builtins and language semantics, including `Number`, `Math`, `Date`, `Promise`, `BigInt`, `Intl`, and `Temporal` where applicable | `P0` |
| `ecma` | `v8` | Supplemental differential and regression source | Use targeted imports, not as the main scorecard | `P2` |
| `ecma` | `jsc` | Supplemental differential and regression source | Use targeted imports from WebKit `JSTests`, not as the main scorecard | `P2` |
| `web` | `fetch`, `url`, `streams`, `encoding`, `fileapi`, `indexeddb`, `webcrypto`, `filesystem-access`, `workers`, `webaudio`, `webserial`, `bluetooth`, `webgpu` | Feature first Web++ conformance | Use WPT as the broad base for most Web APIs and dedicated CTS suites where they exist | `P0` |
| `web` | browser engine suites | Supplemental gap filler | Only for features where WPT is weak or adaptation is unusually difficult | `P2` |
| `node` | `crypto`, `fs`, `net`, `os`, `path`, `process`, `random`, `streams`, `thread`, `time`, `tls`, `tty`, `url` | Feature first Node++ conformance | Focus on modules and capabilities already exposed under `language/builtin/library/platform/` and adjacent Node overlap surfaces | `P0` |
| `formatter` | `prettier` | Formatting parity | Existing suite should migrate under the new structure | `P0` |
| `formatter` | `oxfmt` | Formatting parity | Existing suite should migrate under the new structure | `P0` |

The full testing program needs a broader accounting than just imported conformance corpora.
The complete suite inventory should be tracked explicitly.

### Origin Map

The final conformance architecture should track the actual source corpora behind each claim, not just the suite label.
The current intended source map is:

| Domain | Suite | Primary origin | Additional origin layers | Feature focus |
| --- | --- | --- | --- | --- |
| `ecma` | `test262` | `tc39/test262` | `tc39/test262-parser-tests` for parser-only coverage | core language, modules, `Promise`, `Number`, `Math`, `Date`, `BigInt`, `Intl`, `Temporal` |
| `ecma` | `babel` | `babel/babel` | none | parser coverage for TS, JSX, TSX, and modern ECMA |
| `ecma` | `swc` | `swc-project/swc` | none | parser coverage for TS, JSX, TSX, and modern ECMA |
| `ecma` | `biome` | `biomejs/biome` | none | parser coverage for JS, TS, JSX, and TSX |
| `ecma` | `v8` | `v8/v8` | none | targeted language, builtin, and regression imports from engine suites |
| `ecma` | `jsc` | `WebKit/WebKit` | `JSTests` inside the WebKit tree | targeted language, builtin, and regression imports from engine suites |
| `web` | `fetch`, `url`, `streams`, `encoding`, `fileapi`, `indexeddb`, `webcrypto`, `filesystem-access`, `workers`, `webaudio`, `webserial`, `bluetooth` | `web-platform-tests/wpt` | browser-engine suites only where WPT is weak | the corresponding modern Web API surfaces |
| `web` | `webgpu` | `gpuweb/cts` | WPT `webgpu` coverage can supplement integration gaps | `WebGPU`, WGSL, adapter and device behavior |
| `node` | `crypto`, `fs`, `net`, `os`, `path`, `process`, `random`, `streams`, `thread`, `time`, `tls`, `tty`, `url` | `nodejs/node` | Node-maintained WPT imports where applicable | Node core modules and behaviors already represented in `language/builtin/library/platform/` |
| `formatter` | `prettier` | `prettier/prettier` | none | broad formatter parity |
| `formatter` | `oxfmt` | `oxc-project/oxc` | none | focused formatter parity |

### Full Suite Inventory

The full intended suite inventory is:

| Family | Suite | Purpose | Current status |
| --- | --- | --- | --- |
| first party | `unit` | crate local focused invariants | exists |
| first party | `specification` | Destack language semantics and diagnostics | exists |
| first party | `query` | lower level IDE and query behavior | exists |
| first party | `lsp` | applied editor and language server scenarios | exists |
| first party | `resolver` | module and package resolution behavior | exists |
| first party | `formatter` | first party roundtrip, transform, and smoke formatting behavior | exists |
| first party | `emit` | output snapshots and emit behavior | exists |
| first party | `smoke` | cheap broad no crash coverage | exists |
| first party | `stress` | huge and pathological inputs | exists |
| first party | `optimize` | optimization and benchmark adjacent validation | exists |
| first party | `regression` | surgical bug reproductions across all layers | planned |
| conformance | `ecma/test262` | primary ECMA compatibility scorecard | planned migration target |
| conformance | `ecma/v8` | supplemental ECMA differential and regression cases | planned |
| conformance | `ecma/jsc` | supplemental ECMA differential and regression cases | planned |
| conformance | `web/fetch` and related Web feature suites | primary Web++ compatibility scorecards by feature | planned |
| conformance | `web/webgpu` | dedicated WebGPU compatibility scorecard | planned |
| conformance | `node/fs` and related Node feature suites | primary Node++ compatibility scorecards by feature | planned |
| conformance | `formatter/prettier` | external formatter parity | exists, needs migration |
| conformance | `formatter/oxfmt` | external formatter parity | exists, needs migration |
| compatibility | `ecosystem` | curated package compatibility evidence | exists |
| api surface | `builtin-language-surface` | builtin symbol and module surface coverage against declared registries | planned |
| api surface | `builtin-platform-surface` | host module and API surface coverage for Web++ and Node++ | planned |
| differential | `checker-differential` | targeted comparison against reference tools where overlap is intentional | planned |
| differential | `runtime-differential` | targeted comparison against reference hosts for specific claims | planned |
| generative | `property` | invariant driven generated tests | planned |
| generative | `fuzz` | parser, formatter, resolver, and runtime fuzzing | partial outside this plan, needs integration |
| equivalence | `cross-target` | equivalent observable behavior across backends and targets | planned |
| adaptation | `translation-validation` | validation of adapted external cases | planned |

### Detailed Coverage Map

The architecture should explicitly account for the following coverage targets.

#### ECMA Coverage

The `ecma` domain should cover:

- syntax and grammar
- early errors
- module semantics
- control flow and execution semantics
- async and generators
- `Promise`
- `Number`
- `Math`
- `Date`
- `BigInt`
- `RegExp`
- `JSON`
- collections and iterators
- typed arrays and binary data
- `Proxy` and `Reflect`
- `Atomics` and shared memory where claimed
- `Intl`
- `Temporal`

#### Web Coverage

The `web` domain should cover the final Web++ claim surface, not a reduced bootstrap subset.
The goal is to cover the modern Web API surface comprehensively through WPT, dedicated CTS suites where they exist, and first party supplements where needed.
Candidate targets include:

- `fetch`
- URL and URL infrastructure
- streams
- encoding
- `FileAPI`
- `IndexedDB`
- `WebCryptoAPI`
- file system access
- workers and event loop related APIs
- `WebAudio`
- `WebGPU`
- `WebSerial`
- bluetooth

The intent is to keep growing this list until it matches the full modern Web API surface we want Destack to claim.
The plan should assume broad modern coverage rather than a permanently selective posture.

#### Node Coverage

The `node` domain should cover all Node++ modules and capabilities that already exist under [language/builtin/library/platform](/Users/florian/symbol/destack-1/language/builtin/library/platform).
This should be driven by the platform library itself, not by an arbitrarily reduced hand picked shortlist.
The current platform library shape already spans at least the following domains:

- accessibility
- audio
- core
- crypto
- debug
- device
- display
- error
- ffi
- fs
- gpu
- input
- io
- ipc
- memory
- net
- os
- process
- random
- resource
- runtime
- security
- thread
- time
- tls
- tty

Candidate Node and platform targets therefore include:

- module resolution and loading behavior
- `fs`
- `path`
- `url`
- `crypto`
- streams
- timers
- process and environment APIs
- worker related APIs

This list is illustrative, not limiting.
The final target should be all modules and capabilities already represented in the platform library.

#### Builtin Surface Coverage

The builtins and platform libraries need explicit surface tests in addition to imported behavior tests.
These should validate:

- exported symbol presence
- exported module presence
- basic constructor and function availability
- key descriptor and shape expectations
- consistency with internal registries such as [language/builtin/language/registry.json](/Users/florian/symbol/destack-1/language/builtin/language/registry.json)

#### Tooling Coverage

Tooling coverage should explicitly include:

- formatter
- query
- LSP
- resolver
- emit
- optimizer behavior where correctness claims exist

Lint conformance remains optional.
It should only be added once there is a clear claimed compatibility target.

The `test262` based ECMA lane is the main umbrella for ECMA surface area.
That means there should not be separate top level suites for `Number`, `Math`, `Date`, `Promise`, `BigInt`, `Intl`, and `Temporal` unless there is a compelling reason to create first party supplements.
Those surfaces belong in the `ecma/test262` claim first, with first party tests added only where Destack semantics diverge or where adaptation gaps remain.

The `web` lane should be organized around claimed Web++ surfaces directly, not around one monolithic imported corpus directory.
Candidate feature roots and their main imported sources include:

- `fetch`: mainly WPT
- `url`: mainly WPT
- `streams`: mainly WPT
- `encoding`: mainly WPT
- `fileapi`: mainly WPT
- `indexeddb`: mainly WPT
- `webcrypto`: mainly WPT
- `filesystem-access`: mainly WPT
- `workers`: mainly WPT
- `webaudio`: mainly WPT
- `webserial`: mainly WPT
- `bluetooth`: mainly WPT
- `webgpu`: GPUWeb CTS first, with WPT as supplemental coverage

The `node` lane should also be organized around claimed Node++ surfaces and modules directly, with the intended end state being coverage for all platform modules we already expose.
Candidate Node feature roots include:

- `fs`
- `path`
- `url`
- `crypto`
- `net`
- `os`
- `process`
- `random`
- `streams`
- `thread`
- `time`
- `tls`
- `tty`

Lint conformance is worth planning for, but should not be top priority.
If it lands, it should probably live under `conformance/formatter` or a sibling `conformance/linter` only once Destack has a clear linter compatibility story.
Do not add it until there is a concrete claim such as parity with ESLint style diagnostics or a defined subset.

## Environment And Hardware Model

Environment and hardware requirements should be represented as metadata on suites and cases.
A small shared vocabulary should be enough:

- `hostless`
- `fs`
- `network`
- `worker`
- `gpu`
- `audio-offline`
- `device-serial`
- `device-usb`
- `bluetooth`
- `manual`

These values should drive CI routing and local expectations.
They should not drive the directory tree.

The intended gate mapping is:

- `quick`: `hostless`, `fs`, and selected `network`
- `full`: everything from `quick` plus heavier host backed suites
- `nightly`: `gpu`, `audio-offline`, and expensive adapted conformance sweeps
- `lab`: real device APIs such as serial, USB, and bluetooth when emulation is insufficient

## Runner And Fixture Architecture

The `src` and `fixtures` trees should align roughly at the suite family and domain level.
They do not need to mirror every subdirectory exactly.

The intended code shape is:

```text
language/test/src/
  conformance/
    catalog.rs
    expectation.rs
    report.rs
    ecma/
    web/
    node/
    formatter/
  ecosystem/
  specification/
  smoke/
  stress/
  regression/
```

The intended fixture shape is:

```text
language/test/fixtures/
  conformance/
    ecma/
      test262/
      v8/
      jsc/
    web/
      fetch/
      url/
      streams/
      webcrypto/
      webaudio/
      webserial/
      webgpu/
    node/
      fs/
      crypto/
      process/
      net/
    formatter/
      prettier/
      oxfmt/
  specification/
  ecosystem/
  smoke/
  stress/
  regression/
```

The current `parser/conformance` and `formatter/conformance` code should converge into one shared conformance framework.
Parser, checker, runtime, formatter, and related capabilities should become configurable runner modes over a shared suite catalog.
The wider `language/test/src` tree should also be audited during this reorg so suite semantics are explicit instead of hiding in custom runners, CLI wrappers, or one off fixture conventions.
Every imported harness should also be audited for what it is actually asserting.
That includes checking whether a suite is using parity, idempotence, parser failure expectations, snapshot oracles, or only partial assertions.
Do not assume an imported harness is semantically correct just because it runs.
The `formatter/prettier` parity gap is the concrete example that should drive this audit.
The same audit should also cover first party suites such as `specification`, so we understand exactly which semantics live in `mdtest`, which live in the suite runner, and which should be made more explicit.

## Status Model

Every suite and case should have a structural status model.
The baseline status vocabulary should be:

- `pass`
- `known-fail`
- `ignore`
- `env-blocked`
- `flaky`
- `manual`

This status model should be shared by reporting, update commands, and generated documentation.
It should replace the implicit semantics currently spread across multiple ad hoc text files and runner specific conventions.

## Adapted External Tests

Destack cannot execute `.js` directly.
That means most imported runtime facing suites are source corpora that require adapted runnable fixtures.

This plan does not force provenance heavy directory naming.
Instead, the adaptation strategy should be documented in suite metadata and in translated test annotations.

The important distinction is conceptual:

- preserve the external suite as the compatibility source
- store or generate runnable Destack fixtures for the cases we actually execute
- record enough metadata to keep the mapping understandable

The exact adaptation workflow can vary by corpus:

- parser level suites may use original upstream sources more directly
- runtime and API suites will often need checked in adapted `.ts` or `.ds` fixtures
- hand adapted cases are acceptable when they are reviewed and explicitly tied to a named source suite

## Suite Metadata

Conformance suites should be described structurally in JSON.
The metadata should be simple enough to hand edit and stable enough to use for reporting and CI.

Each imported suite should have a `suite.json`.
Each imported suite should also have a `status.json`.

An indicative `suite.json` shape is:

```json
{
  "id": "ecma.test262",
  "domain": "ecma",
  "suite": "test262",
  "title": "ECMA Conformance",
  "origin": {
    "kind": "git",
    "repo": "https://github.com/tc39/test262",
    "ref": "main-or-pinned-commit"
  },
  "fetch": {
    "entries": [
      {
        "kind": "source",
        "label": "test262 parser",
        "version": "2026-01-29",
        "layout": "preserve",
        "paths": ["pass", "pass-explicit", "fail", "early"]
      }
    ]
  }
}
```

An indicative `status.json` shape is:

```json
{
  "entries": [
    {
      "source_file": "api/headers/headers-basic.any.js",
      "source_subcase": "Check keys method",
      "target_file": "api/headers/headers-basic.any.ts",
      "target_subcase": "Check keys method",
      "status": "translated"
    },
    {
      "source_file": "api/headers/headers-basic.any.js",
      "source_subcase": "Check append method",
      "status": "excluded",
      "reason": "unsound"
    },
    {
      "pattern": "tests/**",
      "status": "known-fail",
      "reason": "unsupported host hook",
      "capabilities": ["run"]
    },
    {
      "source_file": "serial/device-case.ts",
      "status": "env-blocked",
      "reason": "requires serial device lane",
      "environments": ["device-serial"]
    }
  ]
}
```

This metadata should replace flat `known-failures.txt` and `ignored.txt` files over time.
The old text files can be supported during migration, but the end state should be structured JSON.

## Reporting

The current auto updated summary sections in parser and formatter conformance docs are the right general model.
That model should become shared infrastructure rather than bespoke runner specific logic.

The intended reporting model is:

- suites are declared structurally in `suite.json`
- each run emits structured results
- a shared reporter updates marked sections in repository docs

The main generated summary should live in [TESTING.md](/Users/florian/symbol/destack-1/TESTING.md).
That summary should list the exact conformance suites, their capabilities, environments, gates, and pass rates.

An indicative `TESTING.md` table shape is:

| Domain | Suite | Capability | Env | Gate | Passed | Failed | Skipped | Total | Rate | Origin Ref |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | --- |

Suite specific detail should live in suite metadata and generated tables rather than per suite READMEs.
The root `language/test/README.md` should summarize the suite taxonomy and point to the richer generated tables.

The generated reporting plan should include:

- [ ] one global conformance catalog table in [TESTING.md](/Users/florian/symbol/destack-1/TESTING.md)
- [ ] one language specific suite taxonomy table in [language/test/README.md](/Users/florian/symbol/destack-1/language/test/README.md)
- [ ] one per corpus generated summary section in the shared conformance docs
- [ ] category and feature breakdowns for corpora that have natural upstream categories
- [ ] exact origin refs in generated output
- [ ] exact environment and gate information in generated output
- [ ] explicit indication of skipped, known fail, and env blocked cases

## First Party Suites

First party suites still matter and should not be swallowed by imported conformance work.
The intended responsibilities are:

- `specification`: Destack language semantics, diagnostics, and explicit divergences
- `ecosystem`: curated real world packages and workflows
- `regression`: surgical bug reproductions that are too narrow or awkward elsewhere
- `smoke`: broad cheap no crash coverage
- `stress`: huge and pathological inputs

The current `specification` tree should remain semantic and subsystem oriented.
The current `ecosystem` suite should remain distinct from conformance.
It is compatibility evidence, not a normative conformance source.

The `specification` suite should also be audited so its test contract is clearer.
That includes making expected outcomes, manifests, divergence tests, and harness behavior easier to see and harder to accidentally skew, without assuming in advance which parts of `mdtest` should stay or change.

The `resolver` suite should remain first party even though it uses external fixture material in places.
It is fundamentally a Destack behavior suite, not an imported compatibility scorecard.

## Additional Test Modes

The architecture should make room for more than fixture comparison.
Important test modes that should fit under the same scaffolding are:

- differential testing against imported references where that is useful
- property based testing for parser, formatter, resolver, and emit invariants
- metamorphic testing such as formatter idempotence and semantic preserving rewrites
- API surface testing for builtins and host modules
- translation validation for adapted external cases
- cross target equivalence testing when the same program can run under multiple backends

These modes should generally reuse the same suite catalog and reporting infrastructure.
They do not need separate top level directory families unless they grow into large standalone programs.

The project should explicitly account for all of the following test modes:

- unit tests
- fixture tests
- imported conformance tests
- curated ecosystem tests
- regression repro tests
- differential tests
- API surface tests
- property based tests
- metamorphic tests
- fuzz tests
- translation validation tests
- cross target equivalence tests
- hardware and lab tests

## Landing Slices

The implementation should proceed in a small number of large but manageable slices.
The intended slices are:

### Slice 1: Reorg And Shared Scaffolding

- [x] add shared `conformance` modules under `language/test/src/`
- [x] add `fixtures/conformance/` under `language/test/fixtures/`
- [x] add `TESTING_PLAN.md`
- [x] create the final top level directory shape for `conformance/{ecma,web,node,formatter}`
- [x] add the initial shared catalog, expectation, and report modules
- [x] add a dedicated `regression` suite family for future targeted cases

### Slice 2: Metadata, Status, And Reporting

- [x] add shared JSON suite metadata support
- [x] add shared JSON expectation loading support
- [x] add shared reporting support for generated summary sections
- [x] introduce generated conformance summary sections in `TESTING.md`
- [x] update `language/test/README.md` to describe the new structure
- [x] migrate parser and formatter conformance status reporting onto shared infrastructure
- [x] keep old runner entry points temporarily, but back them with the shared catalog and reporter

### Slice 3: Existing Suite Migration

- [x] move parser conformance fixtures under `fixtures/conformance/ecma/test262` and sibling corpora
- [x] move formatter conformance fixtures under `fixtures/conformance/formatter`
- [ ] move or alias runner code from `src/parser/conformance` and `src/formatter/conformance` into shared `src/conformance/*`
- [ ] audit and simplify the surrounding `language/test/src` runner structure while doing that move
- [x] convert `*-known-failures.txt` and `*-ignored.txt` to structured JSON expectations
- [ ] retain a compatibility loader during transition if helpful
- [x] rewrite update commands so they update `status.json` rather than text files

### Slice 4: Core Conformance Expansion

- [ ] land `ecma/test262` on the new architecture first
- [x] land `formatter/prettier` and `formatter/oxfmt` on the new architecture
- [ ] define the adaptation workflow for imported `.js` cases that become runnable Destack fixtures
- [ ] document the workflow in suite metadata and conformance docs
- [x] ensure suite metadata can express capability and environment specific expectations for adapted cases

### Slice 5: Web And Node Conformance

- [x] add feature first Web suite roots such as `web/fetch`, `web/webcrypto`, `web/webaudio`, `web/webserial`, and `web/webgpu`
- [x] add feature first Node suite roots such as `node/fs`, `node/crypto`, `node/process`, and `node/net`
- [ ] wire environment routed execution for network, worker, gpu, audio, and device backed suites
- [ ] begin inventory driven suite creation for all modules already exposed under `language/builtin/library/platform/`

### Slice 6: Deep Coverage And Long Tail

- [ ] land targeted `ecma/v8` and `ecma/jsc` imports only after the shared architecture is stable
- [ ] add builtin language surface tests tied to internal registries
- [ ] add builtin platform surface tests for Node++ and Web++ claims
- [ ] add differential runner support where reference hosts materially help
- [ ] add property based and metamorphic runner support where invariants are strong
- [ ] integrate fuzz lanes into the suite catalog and reporting model
- [ ] add cross target equivalence lanes where multiple backends exist
- [ ] add environment routed nightly and lab lanes for hardware backed APIs
- [ ] finish the `specification` and `mdtest` audit, then apply the structural cleanup that audit actually justifies

## Discussion Points

These questions still need explicit answers during implementation:

- which exact WPT paths and adaptation strategy should back each runnable Web feature lane
- which exact Node test subtrees and adaptation strategy should back each runnable Node feature lane
- how adapted `.js` to runnable `.ts` or `.ds` cases should be reviewed and tracked
- which Web APIs need dedicated hardware or lab lanes versus emulated or offline lanes
- whether linter conformance is a real near term claim or should remain out of scope
- how much of `v8` and `jsc` should be imported versus used only as a source of targeted regressions
- whether `regression` should be a standalone top level family immediately or start as a planning category only
- what exact JSON schema versioning and validation story we want for suite metadata

## Remaining Questions

The main remaining questions are:

- Whether `Intl` and `Temporal` should be treated as required `P0` scorecard items immediately or simply folded into the broader `test262` migration work without special staging.
- Whether `WebAudio` should start with offline and software backed coverage only before any real device or browser hosted work.
- Whether `WebSerial` and similar device APIs should start life as `env-blocked` inventory entries before a lab lane exists.
- Whether `v8` and `jsc` imports should start as targeted regression sources only instead of broad scorecards.
- Whether linter conformance should remain explicitly out of scope until there is a named compatibility target.

## Immediate Next Steps

The immediate execution plan should be:

1. Land this planning document.
2. Add shared `conformance` scaffolding in code.
3. Define and validate the first `suite.json` and `status.json` format.
4. Teach the shared reporter to update generated summary sections in `TESTING.md`.
5. Migrate existing parser and formatter conformance reporting onto the shared layer before moving every fixture.
