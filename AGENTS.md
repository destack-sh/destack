# Agents

The key words MUST, MUST NOT, SHOULD, SHOULD NOT and MAY follow RFC 2119.
- Every rule has a code: verb letter, noun letter, number.
- A lint name in parentheses, like (`destack/comment-style`), marks a rule our linters enforce.

## Plan (P)

Shape the change and desired behavior systematically before and after writing it.

### Shaping (PS)

Work proactively and concretely:
- **PS01** Agents MUST follow these rules proactively.
- **PS02** Agents SHOULD suggest the tools, media, representations and questions that settle the final design before work starts.
- **PS03** Proposals MUST illustrate each point or decision with concrete artifacts: data shapes, interfaces, code snippets, sample data, or noun, state and verb diagrams.

Model the real problem for the real targets:
- **PS10** Designs MUST model and solve a real world problem in a way the machine can execute efficiently.
- **PS11** Designs SHOULD settle the main nouns and verbs first: packages, modules, data structures, fields, methods, and the call / service / data flows between them.
- **PS12** Designs SHOULD state who owns which state, and who reads and writes it where and in what order.
- **PS13** Designs for code that runs often SHOULD reason through the minimal mechanical steps the target hardware performs, for compute, memory and bandwidth, using data oriented design.

Combine boxes and lines with tracer bullets:
- **PS20** Designs SHOULD combine two tools: big boxes with lines (with concrete data structures, interfaces and methods), and tracer bullets that run and connect a vertical slice.
- **PS21** Designs SHOULD start with boxes and lines to center the discussion, then sketch and implement a tracer bullet, then fill in the rest in bursts.

Show designs as code and diagrams:
- **PS30** Data structures SHOULD come first in any design, as real code in the target language with the expected values and value ranges.
- **PS31** Ideas SHOULD come with code, as pseudocode at least, ideally in a form expected to run.
- **PS32** Interfaces SHOULD be designed like APIs, since everything is an API in some sense.
- **PS33** Noun trees and main boxes and lines SHOULD be drawn in ASCII, annotated with field names, types, methods and signatures.
  ```text
  Heap // per owner
  ├── HeapOptions { gc: GcOptions, size_classes: SizeClassTable, ... }
  ├── HeapLimits
  ├── GcPacer
  ├── is_gc_requested: bool
  └── HeapStorage
      ├── SmallStorage
      │   ├── SizeClassTable
      │   ├── SmallSpan[]
      │   ├── partial_spans
      │   └── cursors: cache index -> span
      ├── LargeStorage
      │   ├── LargeBlock[]
      │   └── free_large_block_ids
      ├── page_table: logical page -> PageOwner
      ├── AllocationUsage
      ├── GcState
      └── CollectorState

  /// One heap small space.
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub(crate) struct SmallStorage {
      /// The configured size-class table.
      pub(crate) size_classes: SizeClassTable,
      /// The configured span width.
      pub(crate) span_size_bytes: usize,
      /// The live heap spans.
      pub(crate) spans: Vec<SmallSpan>,
      /// The reusable non-full spans per exact small-span class, excluding the class cursor.
      pub(crate) partial_spans: BTreeMap<SmallSpanClass, Vec<usize>>,
      /// The span each small allocation class reserves from, by class cache index.
      pub(crate) cursors: Vec<Option<usize>>,
  }

  // ... and so on ...
  ```

### Tracing (PT)

Trace callers and scenarios first:
- **PT01** Designs SHOULD first write down what callers of the new code would write, where their inputs come from, and what limits, expectations and environment apply.
  ```text
  const user = service.signup(...); // user from API or wherever
  // ... some more illustrative logic ...
  ```
- **PT02** Designs SHOULD trace each main scenario across real state and call flows as a tree.
  ```text
  UserService.signup(name: string, email: string, password: string, session, ...)
   -> User.create(..., session) // create in DB
     -> User.validate(...)      // validate inputs
     -> User.save(...)          // save to DB
     -> NotificationService.send()
      -> Workflow::trigger()
     -> Session.commit(...)
  ```
  ```text
  provide_program_analysis(profile, target)
   ├─ load each module's MirAnalyzed (per-module LinkGraph, cached, incremental)
   ├─ roots = exported symbols of the target's root modules
   ├─ LinkSupergraph::build(&link_graphs)        // transient: dense index + CSR
   │   └─ live = supergraph.reachable(&roots)     // CSR BFS -> BitSet
   │      (later: references, address_taken, internal — same walk)
   └─ persist ProgramAnalysis { symbols, live }   // columns only, O(symbols) bits
  ```

## Design (D)

Model the problem with the fewest, most pristine nouns and verbs in a way the target machine understands well.

### Modeling (DM)

Solve the real problem:
- **DM01** Code MUST solve real world problems and model them with the fewest, most pristine nouns and verbs the target machine understands well, using the fewest resources the expected hardware and usage allow.
- **DM02** Designs SHOULD follow good relevant prior art, especially in terminology, configuration, interfaces, and behavior where sensible.
- **DM03** External code and documentation MUST be treated with suspicion; most of it is not very good.
- **DM04** Changes SHOULD be read as the question "what shape should the codebase have to support changes like this?", even when the answer means more work now.
- **DM05** Changes MAY be refused: sometimes the right answer is "no, not here, not now".

Keep types few and complete:
- **DM10** Types MUST NOT be bag nouns that only split fields out of a larger type without adding behavior or structure.
- **DM11** Types SHOULD be fewer and fatter unless the domain or the machine needs finer granularity.
- **DM12** Abstractions MUST NOT be forced; forced abstraction is worse than duplication.
- **DM13** Components SHOULD NOT pull in a deep object graph when used ("banana and the jungle"), except where an obvious god object exists, such as a game's current `World`.
- **DM14** State and responsibility SHOULD be bundled with the nouns they belong to, without inheritance hierarchies (unless mechanically required).

Give each fact one owner:
- **DM20** Every piece of the model SHOULD have one clear representation with one obvious owner; values derivable from it, such as the acting principal being the last delegate, are computed instead of stored.

Expose granular, reusable parts:
- **DM30** Logic and state SHOULD be incrementally granular, reusable at several levels of granularity.
- **DM31** Internal code SHOULD expose details and treat callers inside the codebase as consenting adults.

Put verbs on appropriate nouns:
- **DM40** Associated functions SHOULD live on their noun when one presents itself, even when they use no state.
- **DM41** Verbs SHOULD live on the noun that performs them, such as `authorization.grant` or `Scope.own`, instead of exported free functions; modules MAY keep their helpers unexported behind that noun.
- **DM42** Instance nouns (or even singleton nouns) SHOULD be preferred over static nouns that act as just "namespaces"; that is, reshaping to allow the logic to live on "instances" SHOULD BE preferred over both static methods and generic free functions where feasible.
- **DM43** Functions that *could* be methods SHOULD be methods: `foo(definition: &Definition) -> bool` becomes `Definition.foo` (this applies for both free functions and static methods); having loads of static methods / free functions that carry a related set of state is a strong smell that we SHOULD introduce proper nouns.
- **DM44** Method matrices such as `x_for_y` SHOULD be replaced by realigned state and construction flows.
- **DM45** Functions SHOULD operate on individual "units", with callers looping / mapping, instead of taking entire collections / arrays (wherever feasible).

Make mutation obvious:
- **DM50** Mutating methods MUST make the mutation obvious in name and signature.
- **DM51** Methods SHOULD return mutated state or take the mutator instead of mutating internally, where performance allows: `resolve_x` returns the resolved value instead of filling a cache and returning nothing.

Seek symmetry across all scales:
- **DM60** Designs SHOULD aim for symmetry at every scale (types, functions, files, modules, subsystems) and generalise it and enforce it, even if just informally when no language interface is needed.
- **DM61** Sum type variants SHOULD be conceptually and lexically symmetric; asymmetric variants often signal a model problem.

Lay out code in a consistent order:
- **DM70** Files SHOULD order constants first, then the most important nouns, then inner nouns, then free functions, then tests.
- **DM71** Long sequences of fields, variants or methods SHOULD be grouped logically, with blank lines and optional line comments, symmetrically across all groups.

### Invariant (DI)

Make invariants crisp, strong and early:
- **DI01** Invariants MUST be clear and crisp; unclear invariants are design issues to surface and discuss.
- **DI02** Invariants SHOULD be strong, since strong invariants force consumers into the right model.
- **DI03** Requirements and invariants SHOULD be encoded as early as possible: compiler, then linter, then unit tests, then later stages.

Fail loudly and fix at the owner:
- **DI10** Violated invariants MUST fail loudly, especially invariants from other subsystems, and especially from subsystems we control.
- **DI11** Code MUST treat a missing value that an upstream shape or contract promises as an error instead of working around it.
- **DI12** Code MUST NOT work around issues in dependencies we control in any direct or indirect way.
- **DI13** Code MUST NOT recover state, compensate for owned upstream logic, or bypass the owner of state through side channels.
- **DI14** Agents MUST surface suspected boundary violations and suspicious interactions found while reading or touching code in passing.

### Failure (DF)

Fail explicitly and loudly:
- **DF01** Failures MUST be explicit, loud errors through conventional, idiomatic channels.
- **DF02** Logic MUST NOT fail silently, and MUST NOT disguise failures as success through fallbacks, defaults, or null-ish or sentinel values.
- **DF03** Errors outside tests MUST NOT be suppressed or replaced by default values such as `unwrap_or(0)`, `-1` or `MAX`.
- **DF04** Error messages MUST start lowercase and omit the final period. (`destack/error-message-style`)
- **DF05** Code MUST NOT panic, trap or mark code unreachable, except in well guarded tight internal data structures with clear, visible invariants.

Trust internal callers:
- **DF10** Internal code SHOULD NOT check every conceivable failure; both sides of an internal API are consenting adults.
- **DF11** Defensive code SHOULD be read as a sign that the model and its invariants are not yet understood, such as handling `usize` overflow in a modern allocator.

### Performance (DP)

Take performance seriously at all times:
- **DP01** Performance MUST be treated as a feature and an implicit requirement, even without stated bounds.
- **DP02** Designs SHOULD reject "premature optimisation is the root of all evil": compact, aligned data and simple parallel processing make both machines and code better.
- **DP03** Apparent performance tradeoffs SHOULD be read as poor factoring until a wider view shows otherwise.
- **DP04** Code SHOULD be semantically compressed and simple, since clean code is usually fast code.

Bound and calculate before optimising:
- **DP10** Agents SHOULD bound a system's performance characteristics before touching it: latency, RPS, IOPS, throughput, p50, p95, p99.
- **DP11** Agents MUST do the napkin math for every operation, unprompted and especially in performance discussions: how much data, where, how, and how long it should take, using [napkin math](https://github.com/sirupsen/napkin-math).
- **DP12** Performance work SHOULD start bottom up from the bits and cycles actually needed.
- **DP13** Slow code SHOULD first be reformulated to remove the requirement or do less work, before it is optimised.

Lay out data for the machine:
- **DP20** Designs SHOULD use data oriented design by default.
- **DP21** Collections SHOULD default to arrays and linear passes, with maps where they help.
- **DP22** Designs SHOULD favor tight memory access patterns, such as dense arrays over maps, even at large scale.
- **DP23** Designs SHOULD avoid work and data structures that are not needed, and SHOULD rarely need textbook structures or algorithms.

Respect the machine that runs the code:
- **DP30** Code MUST solve the real problem correctly and robustly with the least bandwidth, disk, memory and CPU.
- **DP31** Code SHOULD respect the hardware and machinery that execute it, since execution usually follows the written code.
- **DP32** Code for targets we do not control, such as JavaScript, the web or foreign graphics APIs, SHOULD be written with knowledge of how it actually executes.

### Dependency (DD)

Own as much as is reasonable:
- **DD01** Dependencies SHOULD be few.
- **DD02** Simple logic MUST be implemented in the repository.
- **DD03** Moderately complex logic MAY be vendored.

Use dependencies only when they earn it:
- **DD10** Dependencies MAY wrap a large contract we would otherwise define and maintain, such as `windows_sys`.
- **DD11** Complex or development-only dependencies MAY be added.
- **DD12** New dependencies MUST use their latest stable version.

## Write (W)

Write code and prose that read plain and boring.

### Naming (WN)

Name the thing itself, plainly:
- **WN01** Names MUST be obvious, clear and idiomatic to their language and topic.
- **WN02** Names SHOULD use shorter, stronger nouns and verbs.
- **WN03** Names SHOULD name the thing itself, not where it came from, what it is for or what stage it has reached (e.g., `Permission`, not `DeclaredPermission`; `Log`, not `ChangeLog`).
- **WN04** Idiomatic, independent / namespaced nouns SHOULD be preferred over "verbed" nouns (e.g., `Outcome` over `SettledMutation`, `Submission` over `Pending`, `Advance` over `Applied`, `Call` over `Invocation`); participle nouns generally imply that the model is over-reified in the state dimension.
- **WN05** Names MUST describe actual behavior or purpose, and where applicable shouuld immediately reveal their purpose and mechanism (e.g., a function that creates or updates is `upsert*`, one that allocates conditionally is `allocate*_maybe` or `allocate*_if*`).

Use the idiomatic vocabulary with zero fluff:
- **WN10** Names SHOULD follow Simplified Technical English (STE).
- **WN11** Names SHOULD follow modern prior art terminology where it exists.
- **WN12** Names MUST NOT invent vocabulary where the codebase already has a concept / word.
- **WN13** Terminology MUST be established and kept consistent, continously, across nouns, verbs and their families for types, methods, enums, variants and fields.
- **WN14** Names and prose SHOULD NOT use abstraction sludge words _unless_ the topic idiomatically requires them: seam, lane, part, parts, info, factory, syntax, semantics, data, inner, wrapper, facts, seat, summary, channel, boundary, contract, surface, currency, accounting, load bearing, any "-bearing", spine, spelling, computation, recipe, glue, judge, proof, evidence, drive, carry, own, demand, grammar, reach, truth, product, atom, axes, coordinates, transcribe, law, knot, tie, seal, pin, tighten, slot, mint, helper, util, support, misc, name (as a verb), carry, admit, keep.

Keep names symmetric and general:
- **WN20** Names of related logic and state SHOULD be symmetric (across nouns, verbs, adjectives, etc.).
- **WN21** Names should share common and consistent prefixes (whether implicit namespaces or explicit preambles) at common layers (e.g., the sibling noun for `LocalKeyring` should be `LocalKeyringReader`, not just `KeyringReader` if it's specific to `LocalKeyring`).
- **WN22** Names and particularly nouns SHOULD prefer shorter cleaner composable names (that are potentially namespaced, e.g. `gpu.Display` is preferable over `GpuDisplay`); dual or triple nouns SHOULD be avoided where feasible.
- **WN23** Names SHOULD NOT take the shape `x_for_y`; it usually means the invariants are not generalised yet, and generalisations SHOULD NOT introduce arbitrary interfaces only to avoid `x_for_y`.
- **WN24** Names MAY keep `x_for_y` in data transcription.

Write words out properly:
- **WN30** Names MUST write words out, including variables: `extension`, not `ext`; `directory`, not `dir`. (`destack/prevent-abbreviations`)
- **WN31** Variables MUST NOT use single letters unless obvious, such as `i`, `x` or `Vector.x`. (`eslint/id-length`)

Mark booleans, prefer enums:
- **WN40** Booleans SHOULD start with `is` unless already clear or required by context. (`destack/boolean-prefix`)
- **WN41** Booleans SHOULD give way to enums where an enum fits.

Organise files and accessors by domain:
- **WN50** File and module names SHOULD be single words. (`unicorn/filename-case`)
- **WN51** File and module names MUST describe their domain or purpose.
- **WN52** Accessors SHOULD NOT nest projections such as `revision_files`; a general `files` with a filter, or `files_at_revision`, reads better.

### Logic (WL)

Keep code minimal:
- **WL01** Code SHOULD be minimal: every line is a liability.
- **WL02** Long logic SHOULD be questioned: 500 lines might be 100, 100 might be 10, and 10 might not be needed at all.
- **WL03** Methods MAY be long when their logic is not meaningfully extractable or reusable.

Keep overloads few:
- **WL10** Overloads SHOULD be few.
- **WL11** Overloads MUST NOT merely call one another with different arguments and little logic.

Write logic in commented blocks:
- **WL20** Logic MUST be split into small coherent blocks of 2 to 6 lines, inside and outside functions.
- **WL21** Blocks MUST be separated by blank lines, except the first block in a function.
- **WL22** Blocks MUST start with a comment, except returns. (`destack/require-block-comment`)
- **WL23** Comments for if statements and loops SHOULD sit before the statement, not inside it.
- **WL24** Returns MUST have a blank line before them, even without a comment. (`destack/padding-before-return`)

Branch in structured, symmetric chains:
- **WL30** Branches SHOULD be spelled out at the same level as if-else chains (or similar multi branch-y group) instead of repeated _discontinous_ continue and return jumps.
  ```text
  // option A
  if A {
      Ok(..)
  }
  // option B
  else if B {
      Ok(..)
  }
  // fallback
  else {
      Error(..)
  }
  ```
- **WL31** Branching SHOULD keep a predictable, consistent flow, with symmetrical breadth and depth.
- **WL32** Early exits MAY use guard returns, but continuous changes are preferred.
- **WL33** Main branching SHOULD use coherent if-else chains or match statements instead of sequences of jumps.
  ```text
  let Some(extracted) = extract(foo) else {
      return;
  };
  if invalid(extracted) {
      return;
  }

  match extracted {
      // ... each variant ...
      // ... could also be if-else-if-else if that reads better
  }
  ```
- **WL34** Unstructured branches that are not general preconditions SHOULD be regrouped into structured if-else or match statements.

### State (WS)

Keep state minimal and explicit:
- **WS01** Every bit of state SHOULD be treated as suspicious.
- **WS02** Functions SHOULD be pure, taking context explicitly, usually as the last argument.
- **WS03** Fields and dependencies SHOULD be few.
- **WS04** Fewer state combinations is better, less entropy is better. 

Minimise and model indirect state:
- **WS10** Indirect state (global, static, etc.) SHOULD be minimised and is generally considered evil by default and ALL global state new and pre-existing MUST be reported (this includes those inside classes or similar such constructs, *any* hidden side channel state).
- **WS11** General effect state (time, file system, network, etc.) SHOULD use the platform idiomatic mechanism and MAY be encapsulated in an indirection. 

Name intermediate values:
- **WS20** Trivial operations such as unary operators MAY skip temporary variables.
- **WS21** Non-trivial operations MUST use temporary variables.
  ```rust
  let first_digit = (dt_bytes[0] - b'0') as i64;
  let second_digit = (dt_bytes[1] - b'0') as i64;
  let number = 10 * first_digit + second_digit;
  ```
- **WS22** Transformed variables SHOULD be redefined under the same name: `let module = modules.get(); let module = module.read();`.

### Commenting (WC)

Format comments consistently:
- **WC01** Comments MUST start with a lowercase letter, in every code file including scripts. (`destack/comment-style`)
- **WC02** Comments MUST NOT end with a period. (`destack/comment-style`)
- **WC03** Continued comment lines MUST start with one extra space. (`destack/comment-style`)
- **WC04** Continued comments SHOULD read naturally line by line, even when that splits a sentence.
- **WC05** Comments SHOULD separate clauses with colons or commas instead of hyphens, except in compound words. (`destack/comment-style`)

Place comments above what they describe:
- **WC10** Comments MUST sit directly above the code block they describe, usually 2 to 6 lines.
- **WC11** Case comments MUST precede each if, else-if and else case. (`destack/branch-comment-position`)
  ```text
  // do this
  if (...) {
    ...
  }
  // otherwise do this
  else if (...) {
    ...
  }
  // fall back to this
  else {
    ...
  }
  ```
- **WC12** Tree-shaped sources such as TSX MUST follow the same block comment rules.
  ```tsx
  {/* Container */}
  <div>
      {/* Top button */}
      <button /> ... </button>

      {/* Side panel */}
      <div> ... </div>
  </div>
  ```
- **WC13** Comments MAY consist of a single word or phrase when their scope is clear.
- **WC14** Trivial functions under 4 lines MAY omit comments and blank lines, especially when comments would repeat their documentation.

Write short, plain comments:
- **WC20** Comments SHOULD span at most one sentence.
- **WC21** Comments MUST state plainly what is.
- **WC22** Comments SHOULD organise the reader's mental model, as a one-word summary, a short phrase or a short explanatory note.
- **WC23** Block comments SHOULD start with a verb when longer than two words.
  - `// build drop plan for each function`, not `// each function gets an independent drop plan`

Avoid sludge and tangled sentences:
- **WC30** Comments MUST NOT contain sludge words (WN14), LLM slop, statements about what things are _not_, negative parallelisms, or other such confusions.
- **WC31** Comments MUST avoid indirect speech and sentences that are hard to parse:
  - `// borrow the object from a root local when reached by a target`, NOT `// borrow the object a reference in a root local names when the target reaches through it`
  - "Traits add columns and methods to every object.", NOT "A trait adds columns and methods to every object whose definition names it."
- **WC32** Comments MUST NOT pack several complex clauses into one sentence with commas; multi part, noun-heavy sentences are the worst case.

Mark sections and keywords explicitly:
- **WC40** Separator comments MAY frame an uppercase title with `===` lines.
  ```text
  // ================================================================================
  // Binary operator precedence
  // ================================================================================
  ```
- **WC41** Separator comments SHOULD stay rare, since they are noisy.
- **WC42** Keyword comments MUST start with `NOTE`, `TODO` or `FUGU` and carry a tag, such as `NOTE #Suspicious: allocating in runtime seems wrong?`. (`destack/comment-style`)
  - `NOTE`: call out something important
  - `TODO`: something to address eventually
  - `FUGU`: temporary, broken, fix before going upstream
  - `#Performance`: could be faster or more efficient
  - `#Robustness`: might be flaky in some cases
  - `#Broken`: does not work in likely cases
  - `#Cleanup`: could be simpler or better structured
  - `#Incomplete`: obvious feature is missing
  - `#Suspicious`: something that looks wrong or weird
  - `#Security`: may allow more access than intended
  - `#Architecture`: larger design issue to reconsider
- **WC43**: All relevant "keyword" comments, pre-existing / found and newly added, MUST be mentioned and brought to attention before proposing commits / at the end of a task.

Comment tests where it helps:
- **WC50** Tests MAY use fewer comments within obvious cases.
- **WC51** Test assertions SHOULD state what they check and why when non-trivial.

### Documenting (WD)

Document everything in plain sentences:
- **WD01** Prose MUST be plain, simple technical English in the active voice, in code, comments and docs.
- **WD02** Documentation MUST cover all functions, types, variants and fields, in one line where possible. (`destack/require-jsdoc`)
- **WD03** Documentation MUST consist of proper sentences with punctuation. (`destack/jsdoc-sentence`)
- **WD04** Files MUST NOT have top-level documentation comments, since they always get stale. (`destack/jsdoc-sentence`)
- **WD05** Constants SHOULD be external facts, identities, or tuning values, each documented with its source or the napkin math that sets it; a tuning value callers vary becomes the default of an option on the noun that owns it.
  ```ts
  /** The longest identifier PostgreSQL stores without truncation, NAMEDATALEN minus one. */
  const MAX_IDENTIFIER_LENGTH = 63;
  ```

Keep documentation to one line:
- **WD10** Documentation SHOULD fit one line at the 100 character width (but this is not a hard limit).
  In rare, complex cases multiline documentation MAY come after a blank line in a multiline doc comment.
- **WD11** Documentation with more than one sentence MUST go multiline, with one sentence per line. (`destack/jsdoc-sentence`)
- **WD12** Multiline documentation SHOULD use a header line, a blank line, then paragraph lines. (`destack/jsdoc-sentence`)

Say what methods do and what types are:
- **WD20** Method documentation SHOULD be imperative and start with a verb, such as "Send a message".
- **WD21** Type and field documentation SHOULD plainly state what the thing is: "The blocks built so far.", not "Represents the blocks built up to this point."

Write Markdown one sentence per line:
- **WD30** Markdown prose MUST put one sentence per line; tables and similar structures are exempt. (`markdown/one-sentence-per-line`)
- **WD31** Markdown SHOULD use rich formatting: sections, subsections, highlighting, code examples and tables.

### TypeScript-y (WT)

Declare and assign class fields explicitly:
- **WT01** Class fields MUST be declared explicitly, with documentation above each field. (`destack/require-jsdoc`)
- **WT02** Class fields MUST be assigned in constructor bodies, not through constructor parameter properties. (`typescript/parameter-properties`)

Prefer web-standard APIs:
- **WT10** Shared code SHOULD use web-standard APIs, such as `Uint8Array.toHex` or `crypto.subtle`, so it runs on every target; Node or Bun APIs MAY be used in target-specific modules.

### Rust-y (WR)

Keep modules flat and real:
- **WR10** `mod.rs` and `main.rs` MUST contain only re-exports and submodule declarations.
- **WR11** Modules MUST be either `module.rs` or `module/mod.rs` with real `module/whatever.rs` files, never both.
- **WR12** Code MUST NOT use `include!` or convoluted `#[path]` to bypass module structure.
- **WR13** Files MUST NOT nest `mod x { }`, except for `tests`.

Import plainly at the top:
- **WR20** Imports MUST sit at the top, in the form `use std::time::Instant`.
- **WR21** Top-level imports SHOULD use `use crate::x` directly where possible.
- **WR22** Paths MUST NOT use `crate::X` inside functions.
- **WR23** Public exports SHOULD use `pub use submodule::*`, relying on correct `pub` visibility.
- **WR24** Imports MUST NOT use aliases. (`destack/no-import-alias`)

Order items predictably:
- **WR30** Comments and documentation MUST precede all attributes, such as `#[inline]` and `#[derive]`.
- **WR31** Constants MUST sit at the top of the file, with no magic numbers or values.
- **WR32** Functions MUST NOT nest items such as functions, lambdas or types.
- **WR33** cfg-gated statements MUST be grouped into their own blank-line-delimited sections.
  ```rust
  mod foo;
  mod baz;

  pub use foo::*;

  #[cfg(not(target_arch = "wasm32"))]
  mod websocket;
  #[cfg(not(target_arch = "wasm32"))]
  pub use websocket::{WebSocketServer, WebSocketServerError};
  ```

Lean on the type system:
- **WR40** Self-contained conversions SHOULD use `Into`, `From` and sometimes `TryFrom`.
- **WR41** Enums named like scalars, such as `*Kind`, MUST NOT carry payloads in their variants.
- **WR42** Code SHOULD avoid `Cell`, `RefCell` and `UnsafeCell`, which almost always imply a bad ownership model.
- **WR43** `unsafe` MAY be used when its invariants are proven and tested.

Return errors and keep tests separate:
- **WR50** Code outside tests MUST fail explicitly through `Result` handling instead of `unwrap`, `expect` or `panic`.
- **WR51** Tests MUST live in a trailing `mod tests` or in standalone test modules or crates, depending on context.

Write idiomatic expressions:
- **WR60** Clones SHOULD use `expr.clone()` instead of `Arc.clone(expr)`.
- **WR61** Heavy clones SHOULD be avoided, since memory and fragmentation are expensive.
- **WR62** Format macros SHOULD inline variables: `format!("name is {name}")`.
- **WR63** Longer strings SHOULD use multiline raw strings.

## Rewrite (R)

Rewrite as understanding grows, toward the final shape, always rewrite: generalise, compress, subtract.

### Surveying (RS)

Question what exists, keep what is intended:
- **RS01** Code SHOULD be refactored, cleaned, re-audited and semantically compressed continuously; nothing is final.
- **RS02** Agents MUST NOT assume existing code is good because it exists, is in use or is tested.
- **RS03** Failing tests MAY be wrong instead of the new code.
- **RS04** Tests and expectations MUST NOT change without explicit prior discussion and agreement.
- **RS05** Stubs that clearly mark an unfinished shape SHOULD NOT be removed when they are meant to be filled in later; ask when unsure.

Earn every part of the final model:
- **RS10** Refactors MUST start by surveying the Chesterton's fence of the status quo: how it got there, and why it might be that way.
- **RS11** Nouns, verbs, types, variants, fields and lines MUST each be earned.
- **RS12** Final models MUST capture the essential complexity of the problem in its most pristine form, nothing more and nothing less.
- **RS13** Bloat SHOULD be removed as soon as it becomes visible, since it often only shows as the problem's true shape emerges.
- **RS14** Entropy of the overall model SHOULD be continously minimised and compressed as we figure out which parts of the state space are actually occupied by reality.

### Replacing (RR)

Refactor as the shape emerges:
- **RR01** Code SHOULD be refactored as understanding deepens and the right shape reveals itself; programming is refactoring just as writing is editing.
- **RR02** Refactors SHOULD be painless and touch only the parts of the model that need to change.
- **RR03** Refactors that touch more than they should MUST be investigated.
- **RR04** Investigated refactors MAY widen, or trigger a follow-up, to crispen the model's boundaries.

Go straight to the final model:
- **RR10** Changes SHOULD make the next change easy first: "make the change easy, then make the change".
- **RR11** Changes SHOULD skip intermediate and transitional states and go straight to the final model.
- **RR12** Code MUST NOT introduce transitional or "for now" logic unless explicitly requested.
- **RR13** Components under roughly 5k lines MAY be ripped out and rewritten completely when that is easier.
- **RR14** Changes SHOULD break or change the source directly and follow the compiler to every usage site.

Surface issues in other systems:
- **RR20** Issues in other systems MUST be surfaced and discussed instead of papered over or hidden.
- **RR21** Surfaced issues MAY be ignored only once explicitly acknowledged and deferred.

### Fixing (RF)

Fix the model, not the symptom:
- **RF01** Bugs MUST be treated as model misalignment, not as a natural phenomenon; correct systems come from clarity, simplicity, strong invariants and clear expectations.
- **RF02** Issues of any kind MUST be treated as challenges to the model, answered by the model's long term shape rather than by standalone fixes.
- **RF03** Fixes SHOULD NOT be additive or applied en masse; checking invariants is fine, making bad states impossible is better, and simplifying / compressing the model or avoiding the problem alltogether is best.
- **RF04** Failures SHOULD act as a searchlight: study the model they expose, find its gaps, and correct the model in the most general, compressed, subtractive way.

## Check (C)

Check every change with tests, formatting, lints, and any relevant static and dynamic analysis.

### Testing (CT)

Test the real thing in its scope:
- **CT01** Tests MUST live in the package / crate / scope that implements the behavior they check; they MUST NOT test "upstream" mechanics that they only use. (e.g., no point testing the underlying database when the scope is about schema generation; test the *actual* thing)
- **CT02** Tests SHOULD NOT rely on factory or dependency injection layers.
- **CT03** Code that is awkward to test SHOULD be refactored, since it is almost always poorly factored (also see above).
- **CT04** Hard-to-test code SHOULD get crisper models and more realistic tests instead.

Name tests by behavior:
- **CT10** Tests MUST be named with a verb that states their content, such as `roundtrip duration` or `test_send_receive_message`.
- **CT11** Test first lines or docstrings MUST describe the desired behavior without mentioning "test".

Test whole outputs, as hard as possible:
- **CT20** Tests SHOULD be the strongest, most brutal form we can think of: PBT, DST, roundtrip tests, whatever is applicable.
- **CT21** Tests SHOULD exercise the entire thing over a part, comparing complete output.
- **CT22** Tests SHOULD assert the entire expected output as some sort of snapshot or invasive assertion where possible.

Assert exact outcomes:
- **CT30** Tests MUST assert specific outcomes, such as "these two errors with that message", not "expect failed" or "any two errors".
- **CT31** Tests MUST NOT assert partial values such as `x.contains('part of foo')`. (`destack/no-partial-assertions`)
- **CT32** Non-trivial assertions SHOULD be commented like other logic blocks.

Keep tests fast:
- **CT40** Tests slower than a few milliseconds, tens at most, MUST be investigated for fundamental model issues.
- **CT41** Test runs MUST use a tight timeout, such as building first without a timeout and then testing with one; slow tests are a hard failure.

### Formatting (CF)

Format every change, within its scope:
- **CF01** Code MUST be formatted before a change is done.
- **CF02** Code SHOULD be formatted before it runs or is checked, so it compiles only once.
- **CF03** Formatting MUST stay within the change's scope, using the directory's `just fmt` or equivalent.

### Linting (CL)

Fix every lint:
- **CL01** Rust changes MUST fix all lints from `cargo check -p <crate>` and `cargo clippy -p <crate>`.
- **CL02** Too-many-arguments and type-complexity warnings MAY be ignored.

Suppress lints on the impl:
- **CL10** Clippy allowances SHOULD sit on top of the `impl`, not on individual functions.
- **CL11** Lint suppressions MUST sit at the top of the impl block, not on individual functions.

## Ship (S)

Land changes through branches and plain commits under the strict cooperation protocol.

### Branching (SB)

Work on branches off main:
- **SB01** Work SHOULD happen on branches and worktrees off a main branch.
- **SB02** Branches SHOULD rebase off main frequently and merge back into main.

Merge back with history intact:
- **SB10** Merges into main SHOULD fast-forward (or cherry-pick) to retain history.
- **SB11** Merges into main MAY squash many small commits.

### Committing (SC)

Commit only on explicit instruction:
- **SC01** Agents MUST NOT commit or merge without explicit instruction.
- **SC02** Agents MAY present suggested commit slices after re-reviewing their work.

Use conventional commit subjects:
- **SC10** Commits MUST use conventional commits in present tense, Simplified Technical English and verb-first shape.
- **SC11** Commit subjects MUST follow `type(scope): verb noun` with an imperative summary under 100 characters.
- **SC12** Commit scopes MUST use the full, sometimes stylised, scope, such as `language/ast`, `language/compiler/analyze` or `library/ui`.
- **SC13** Commits touching several scopes MUST use the highest scope or `all`, or split into smaller commits when large.
- **SC14** Commits in large packages or crates MAY use subscopes such as `language/compiler/sema`.

Pick the commit type:
- **SC20** Commit types MUST be one of, from most to least coalescing:
  - `feat`: extend, generalise, or add non-trivial model changes
  - `refactor`: replace or reshape the model in some non-trivial way
  - `fix`: correct the model in some non-trivial sense that did not require a refactor
  - `test`: extend, update, bless, or otherwise modify the tests
  - `chore`: modify or adapt in a semantically trivial way (format, trivial API adaptation, ..)
  - `docs`: edit the docs (in code or otherwise)
  - `dev`: make some meta change about the dev setup

Write boring, plain commit messages:
- **SC30** Commit messages MUST be boring like boiled potatoes and dry bread: what changed, as plainly as possible, without a story.
- **SC31** Commit messages MUST NOT mention meta context such as "part 1" or "landed feature X".
- **SC32** Commit messages SHOULD use simple verbs (e.g., add, remove, rename X -> Y, reshape, move, update X -> Y) and specific code concepts (e.g., Player.move, CharacterController).
- **SC33** Commit messages SHOULD name code concepts as proper nouns, not files: `add SiteTable, rename Foo -> Bar`.
- **SC34** Commit messages SHOULD follow the style of the last 50 or so commits when in doubt.
- **SC35** Commit messages MUST NOT mention non-human authors or contributors: no co-authors and no bylines.
