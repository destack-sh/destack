# Agents

The key words MUST, MUST NOT, SHOULD, SHOULD NOT and MAY follow RFC 2119.
Every rule has a code: verb letter, noun letter, number.
Codes number the rules in reading order and change when rules move.
Rules come in short blocks of related rules, from the general to the specific.
A lint name in parentheses, like (`destack/comment-style`), marks a rule the linter enforces.
Destack packages MUST also follow the package rules in [`@destack/template-blank/AGENTS.md`](@destack/template-blank/AGENTS.md).

## Plan (P)

Shape the change before writing it.

### Shaping (PS)

- **PS01** Agents MUST follow these rules proactively.
- **PS02** Agents SHOULD suggest the tools, media, representations and questions that settle the final design before work starts.
- **PS03** Proposals MUST illustrate each point or decision with concrete artifacts: data shapes, interfaces, code snippets, sample data, or noun, state and verb diagrams.

- **PS04** Designs MUST model and solve a real world problem in a way the machine can execute efficiently.
- **PS05** Designs SHOULD settle the main nouns and verbs first: data structures, fields, methods, and the call flows between them.
- **PS06** Designs SHOULD state who owns which state, and who reads and writes it where and in what order.
- **PS07** Designs for code that runs often SHOULD reason through the minimal mechanical steps the target hardware performs, for compute, memory and bandwidth, using data oriented design.

- **PS08** Designs SHOULD combine two tools: big boxes with lines (with concrete data structures, interfaces and methods), and tracer bullets that run and connect a vertical slice.
- **PS09** Designs SHOULD start with boxes and lines to center the discussion, then sketch and implement a tracer bullet, then fill in the rest in bursts.

- **PS10** Data structures SHOULD come first in any design, as real code in the target language with the expected values and value ranges.
- **PS11** Ideas SHOULD come with code, as pseudocode at least, ideally in a form expected to run.
- **PS12** Interfaces SHOULD be designed like APIs, since everything is an API in some sense.
- **PS13** Noun trees and main boxes and lines SHOULD be drawn in ASCII, annotated with field names, types, methods and signatures.
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

Model the problem with the fewest, most pristine nouns and verbs.

### Modeling (DM)

- **DM01** Code MUST solve real world problems and model them with the fewest, most pristine nouns and verbs the target machine understands well, using the fewest resources the expected hardware and usage allow.
- **DM02** Designs SHOULD follow good relevant prior art, especially in terminology, configuration, interfaces, and behavior where sensible.
- **DM03** External code and documentation MUST be treated with suspicion; most of it is not very good.
- **DM04** Changes SHOULD be read as the question "what shape should the codebase have to support changes like this?", even when the answer means more work now.
- **DM05** Changes MAY be refused: sometimes the right answer is "no, not here, not now".

- **DM06** Types MUST NOT be bag nouns that only split fields out of a larger type without adding behavior or structure.
- **DM07** Types SHOULD be fewer and fatter unless the domain or the machine needs finer granularity.
- **DM08** Abstractions MUST NOT be forced; forced abstraction is worse than duplication.
- **DM09** Components SHOULD NOT pull in a deep object graph when used ("banana and the jungle"), except where an obvious god object exists, such as a game's current `World`.
- **DM10** State and responsibility SHOULD be bundled with the nouns they belong to, without inheritance hierarchies.

- **DM11** Every piece of information SHOULD have one clear representation with one obvious owner; values derivable from it, such as the acting principal being the last delegate, are computed instead of stored.
- **DM12** Logic and state SHOULD be incrementally granular, reusable at several levels of granularity.
- **DM13** Internal code SHOULD expose details and treat callers inside the codebase as consenting adults.

- **DM14** Associated functions SHOULD live on their noun when one presents itself, even when they use no state.
- **DM15** Verbs SHOULD live on the noun that performs them, such as `authorization.grant` or `Scope.own`, instead of exported free functions; modules MAY keep their helpers unexported behind that noun.
- **DM16** Functions that could be methods SHOULD be methods: `foo(definition: &Definition) -> bool` becomes `Definition.foo`.
- **DM17** Method matrices such as `x_for_y` SHOULD be replaced by realigned state and construction flows.

- **DM18** Mutating methods MUST make the mutation obvious in name and signature.
- **DM19** Methods SHOULD return mutated state or take the mutator instead of mutating internally, where performance allows: `resolve_x` returns the resolved value instead of filling a cache and returning nothing.
- **DM20** Functions SHOULD operate on one element, with callers looping, instead of taking arrays.
- **DM21** Mutability SHOULD be parametric.

- **DM22** Designs SHOULD look for symmetry at every scale (types, functions, files, modules, subsystems) and generalise it, informally when no language interface is needed.
- **DM23** Sum type variants SHOULD be conceptually and lexically symmetric; asymmetric variants often signal a model problem.

- **DM24** Files SHOULD order constants first, then the most important nouns, then inner nouns, then free functions, then tests.
- **DM25** Long sequences of fields, variants or methods SHOULD be grouped logically, with blank lines and optional line comments, symmetrically across all groups.

### Invariant (DI)

- **DI01** Invariants MUST be clear and crisp; unclear invariants are design issues to surface and discuss.
- **DI02** Invariants SHOULD be strong, since strong invariants force consumers into the right model.
- **DI03** Requirements and invariants SHOULD be encoded as early as possible: compiler, then linter, then unit tests, then later stages.

- **DI04** Violated invariants MUST fail loudly, especially invariants from other subsystems, and especially from subsystems we control.
- **DI05** Code MUST treat a missing value that an upstream shape or contract promises as an error instead of working around it.
- **DI06** Code MUST NOT work around issues in dependencies we control.
- **DI07** Code MUST NOT recover state, compensate for owned upstream logic, or bypass the owner of state through side channels.
- **DI08** Agents MUST surface suspected boundary violations and suspicious interactions found while reading or touching code in passing.

### Failure (DF)

- **DF01** Failures MUST be explicit, loud errors through conventional, idiomatic channels.
- **DF02** Live code MUST NOT fail silently, and MUST NOT disguise failures as success through fallbacks, defaults, or null-ish or sentinel values. (`destack/no-silent-fallback`)
- **DF03** Errors outside tests MUST NOT be suppressed or replaced by default values such as `unwrap_or(0)`, `-1` or `MAX`.
- **DF04** Error messages MUST start lowercase and omit the final period. (`destack/error-message-style`)

- **DF05** Internal code SHOULD NOT check every conceivable failure; both sides of an internal API are consenting adults.
- **DF06** Defensive code SHOULD be read as a sign that the model and its invariants are not yet understood, such as handling `usize` overflow in a modern allocator.
- **DF07** Code MUST NOT panic, trap or mark code unreachable, except in well guarded tight internal data structures with clear, visible invariants.

### Performance (DP)

- **DP01** Performance MUST be treated as a feature and an implicit requirement, even without stated bounds.
- **DP02** Designs SHOULD reject "premature optimisation is the root of all evil": compact, aligned data and simple parallel processing make both machines and code better.
- **DP03** Apparent performance tradeoffs SHOULD be read as poor factoring until a wider view shows otherwise.
- **DP04** Code SHOULD be semantically compressed and simple, since clean code is usually fast code.

- **DP05** Agents SHOULD bound a system's performance characteristics before touching it: latency, RPS, IOPS, throughput, p50, p95, p99.
- **DP06** Agents MUST do the napkin math for every operation, unprompted and especially in performance discussions: how much data, where, how, and how long it should take, using [napkin math](https://github.com/sirupsen/napkin-math).
- **DP07** Performance work SHOULD start bottom up from the bits and cycles actually needed.
- **DP08** Slow code SHOULD first be reformulated to remove the requirement or do less work, before it is optimised.

- **DP09** Designs SHOULD use data oriented design by default.
- **DP10** Collections SHOULD default to arrays and linear passes, with maps where they help.
- **DP11** Designs SHOULD favor tight memory access patterns, such as dense arrays over maps, even at large scale.
- **DP12** Designs SHOULD avoid work and data structures that are not needed, and SHOULD rarely need textbook structures or algorithms.

- **DP13** Code MUST solve the real problem correctly and robustly with the least bandwidth, disk, memory and CPU.
- **DP14** Code SHOULD respect the hardware and machinery that execute it, since execution usually follows the written code.
- **DP15** Code for targets we do not control, such as JavaScript, the web or foreign graphics APIs, SHOULD be written with knowledge of how it actually executes.

### Dependency (DD)

- **DD01** Dependencies SHOULD be few.
- **DD02** Simple logic MUST be implemented in the repository.
- **DD03** Moderately complex logic MAY be vendored.

- **DD04** Dependencies MAY wrap a large contract we would otherwise define and maintain, such as `windows_sys`.
- **DD05** Complex or development-only dependencies MAY be added.
- **DD06** New dependencies MUST use their latest stable version.

## Write (W)

Write code and prose that read plainly.

### Naming (WN)

- **WN01** Names MUST be obvious, clear and idiomatic to their language and topic.
- **WN02** Names SHOULD use shorter, stronger nouns and verbs.
- **WN03** Names SHOULD name the thing itself, not where it came from, what it is for or what stage it has reached: `Permission`, not `DeclaredPermission`; `Log`, not `ChangeLog`.
- **WN04** Nouns SHOULD be preferred over verbed nouns: `Outcome` over `SettledMutation`, `Submission` over `Pending`, `Advance` over `Applied`, `Call` over `Invocation`.
- **WN05** Names MUST describe actual behavior or purpose: a function that creates or updates is `upsert*`, one that allocates conditionally is `allocate*_maybe` or `allocate*_if*`.

- **WN06** Names SHOULD follow Simplified Technical English (STE).
- **WN07** Names SHOULD follow modern prior art terminology where it exists.
- **WN08** Names MUST NOT invent vocabulary where the codebase already has a word: `procedures`, not `contract`.
- **WN09** Terminology MUST be established and kept consistent across nouns, verbs and their families for types, methods, enums, variants and fields.

- **WN10** Names of related logic SHOULD be symmetric.
- **WN11** Names SHOULD NOT take the shape `x_for_y`; it usually means the invariants are not generalised yet.
- **WN12** Generalisations MUST NOT introduce arbitrary interfaces only to avoid `x_for_y`.
- **WN13** Names MAY keep `x_for_y` in data transcription.

- **WN14** Names MUST write words out, including variables: `extension`, not `ext`; `directory`, not `dir`. (`destack/prevent-abbreviations`)
- **WN15** Variables MUST NOT use single letters unless obvious, such as `i`, `x` or `Vector.x`. (`eslint/id-length`)
- **WN16** Booleans SHOULD start with `is` unless already clear or required by context. (`destack/boolean-prefix`)
- **WN17** Booleans SHOULD give way to enums where an enum fits.

- **WN18** File and module names SHOULD be single words. (`unicorn/filename-case`)
- **WN19** File and module names MUST describe their domain or purpose.
- **WN20** Accessors SHOULD NOT nest projections such as `revision_files`; a general `files` with a filter, or `files_at_revision`, reads better.
- **WN21** Audit actions MUST be named `Noun.verb`, with PascalCase nouns and a camelCase present-tense verb, such as `Login.signIn` or `ServiceAccount.create`.

### Logic (WL)

- **WL01** Code SHOULD be minimal: every line is a liability and every bit of state is suspicious.
- **WL02** Long logic SHOULD be questioned: 500 lines might be 100, 100 might be 10, and 10 might not be needed at all.
- **WL03** Methods MAY be long when their logic is not meaningfully extractable or reusable.
- **WL04** Functions SHOULD be pure, taking context explicitly, usually as the last argument.

- **WL05** Overloads, fields and dependencies SHOULD be few.
- **WL06** Overloads MUST NOT merely call one another with different arguments and little logic.

- **WL07** Logic MUST be split into small coherent blocks of 2 to 6 lines, inside and outside functions.
- **WL08** Blocks MUST be separated by blank lines, except the first block in a function.
- **WL09** Blocks MUST start with a comment, except returns. (`destack/require-block-comment`)
- **WL10** Comments for if statements and loops SHOULD sit before the statement, not inside it.
- **WL11** Returns MUST have a blank line before them, even without a comment. (`destack/padding-before-return`)

- **WL12** Trivial operations such as unary operators MAY skip temporary variables.
- **WL13** Non-trivial operations MUST use temporary variables.
  ```rust
  let first_digit = (dt_bytes[0] - b'0') as i64;
  let second_digit = (dt_bytes[1] - b'0') as i64;
  let number = 10 * first_digit + second_digit;
  ```

- **WL14** Branches SHOULD be spelled out at the same level as if-else chains instead of repeated continue and return jumps.
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
- **WL15** Branching SHOULD keep a predictable, consistent flow, breadth and depth.
- **WL16** Early exits MAY use guard returns.
- **WL17** Main branching SHOULD use coherent if-else chains or match statements instead of sequences of jumps.
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
- **WL18** Unstructured branches that are not general preconditions SHOULD be regrouped into structured if-else or match statements.

### Commenting (WC)

- **WC01** Comments MUST start with a lowercase letter, in every code file including scripts. (`destack/comment-style`)
- **WC02** Comments MUST NOT end with a period. (`destack/comment-style`)
- **WC03** Continued comment lines MUST start with one extra space. (`destack/comment-style`)
- **WC04** Continued comments SHOULD read naturally line by line, even when that splits a sentence.
- **WC05** Comments SHOULD separate clauses with colons or commas instead of hyphens, except in compound words. (`destack/comment-style`)

- **WC06** Comments MUST sit directly above the code block they describe, usually 2 to 6 lines.
- **WC07** Case comments MUST precede each if, else-if and else case. (`destack/branch-comment-position`)
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
- **WC08** Tree-shaped sources such as TSX MUST follow the same block comment rules.
  ```tsx
  {/* Container */}
  <div>
      {/* Top button */}
      <button /> ... </button>

      {/* Side panel */}
      <div> ... </div>
  </div>
  ```
- **WC09** Comments MAY consist of a single word or phrase when their scope is clear.
- **WC10** Trivial functions under 4 lines MAY omit comments and blank lines, especially when comments would repeat their documentation.

- **WC11** Comments SHOULD span at most one sentence.
- **WC12** Comments MUST state plainly what is.
- **WC13** Comments SHOULD organise the reader's mental model, as a one-word summary, a short phrase or a short explanatory note.
- **WC14** Block comments SHOULD start with a verb when longer than two words.
  - `// build drop plan for each function`, not `// each function gets an independent drop plan`

- **WC15** Comments MUST NOT contain LLM slop, statements about what things are not, or negative parallelisms.
- **WC16** Comments MUST avoid indirect speech and sentences that are hard to parse.
  - `// borrow the object from a root local when reached by a target`, not `// borrow the object a reference in a root local names when the target reaches through it`
- **WC17** Comments MUST NOT pack several complex clauses into one sentence with commas; multi part, noun-heavy sentences are the worst case.

- **WC18** Separator comments MAY frame an uppercase title with `===` lines.
  ```text
  // ================================================================================
  // Binary operator precedence
  // ================================================================================
  ```
- **WC19** Separator comments SHOULD stay rare, since they are noisy.
- **WC20** Keyword comments MUST start with `NOTE`, `TODO` or `FUGU` and carry a tag, such as `NOTE #Suspicious: allocating in runtime seems wrong?`. (`destack/comment-style`)
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

- **WC21** Tests MAY use fewer comments within obvious cases.
- **WC22** Test assertions SHOULD state what they check and why when non-trivial.

### Documenting (WD)

- **WD01** Prose MUST be plain, simple technical English in the active voice, in code, comments and docs.
- **WD02** Documentation MUST cover all functions, types, variants and fields, in one line where possible. (`destack/require-jsdoc`)
- **WD03** Documentation MUST consist of proper sentences with punctuation. (`destack/jsdoc-sentence`)
- **WD04** Files MUST NOT have top-level documentation comments, since they always get stale. (`destack/jsdoc-sentence`)

- **WD05** Documentation SHOULD fit one line at the 100 character width.
- **WD06** Documentation with more than one sentence MUST go multiline, with one sentence per line. (`destack/jsdoc-sentence`)
- **WD07** Multiline documentation SHOULD use a header line, a blank line, then paragraph lines. (`destack/jsdoc-sentence`)

- **WD08** Method documentation SHOULD be imperative and start with a verb, such as "Send a message".
- **WD09** Type and field documentation SHOULD plainly state what the thing is: "The blocks built so far.", not "Represents the blocks built up to this point."

- **WD10** Markdown prose MUST put one sentence per line; tables and similar structures are exempt. (`markdown/one-sentence-per-line`)
- **WD11** Markdown SHOULD use rich formatting: sections, subsections, highlighting, code examples and tables.

### TypeScript (WT)

- **WT01** Class fields MUST be declared explicitly, with documentation above each field. (`destack/require-jsdoc`)
- **WT02** Class fields MUST be assigned in constructor bodies, not through constructor parameter properties. (`typescript/parameter-properties`)

- **WT03** Domain operations MUST live in noun modules; `*Store` names MUST be reserved for persistence.
- **WT04** Shared code MUST use web-standard APIs, such as `Uint8Array.toHex` or `crypto.subtle`, so it runs on every target; Node or Bun APIs belong only in target-specific modules.

### Rust (WR)

- **WR01** These rules MUST also apply to other Rust-like languages, including the Rust side of Destack and TS++.
- **WR02** `mod.rs` and `main.rs` MUST contain only re-exports and submodule declarations.
- **WR03** Modules MUST be either `module.rs` or `module/mod.rs` with real `module/whatever.rs` files, never both.
- **WR04** Code MUST NOT use `include!` or convoluted `#[path]` to bypass module structure.
- **WR05** Files MUST NOT nest `mod x { }`, except for `tests`.

- **WR06** Imports MUST sit at the top, in the form `use std::time::Instant`.
- **WR07** Top-level imports SHOULD use `use crate::x` directly where possible.
- **WR08** Paths MUST NOT use `crate::X` inside functions.
- **WR09** Public exports SHOULD use `pub use submodule::*`, relying on correct `pub` visibility.
- **WR10** Imports MUST NOT use aliases. (`destack/no-import-alias`)

- **WR11** Comments and documentation MUST precede all attributes, such as `#[inline]` and `#[derive]`.
- **WR12** Constants MUST sit at the top of the file, with no magic numbers or values.
- **WR13** Functions MUST NOT nest items such as functions, lambdas or types.
- **WR14** cfg-gated statements MUST be grouped into their own blank-line-delimited sections.
  ```rust
  mod foo;
  mod baz;

  pub use foo::*;

  #[cfg(not(target_arch = "wasm32"))]
  mod websocket;
  #[cfg(not(target_arch = "wasm32"))]
  pub use websocket::{WebSocketServer, WebSocketServerError};
  ```

- **WR15** Self-contained conversions SHOULD use `Into`, `From` and sometimes `TryFrom`.
- **WR16** Enums named like scalars, such as `*Kind`, MUST NOT carry payloads in their variants.
- **WR17** Code SHOULD avoid `Cell`, `RefCell` and `UnsafeCell`, which almost always imply a bad ownership model.
- **WR18** `unsafe` MAY be used when its invariants are proven and tested.

- **WR19** Code outside tests MUST fail explicitly through `Result` handling instead of `unwrap`, `expect` or `panic`.
- **WR20** Tests MUST live in a trailing `mod tests` or in standalone test modules or crates, depending on context.

- **WR21** Clones SHOULD use `expr.clone()` over `Arc.clone(expr)`.
- **WR22** Heavy clones SHOULD be avoided, since memory and fragmentation are expensive.
- **WR23** Format macros SHOULD inline variables: `format!("name is {name}")`.
- **WR24** Longer strings SHOULD use multiline raw strings.
- **WR25** Transformed variables SHOULD be redefined under the same name: `let module = modules.get(); let module = module.read();`.

## Rewrite (R)

Rewrite as understanding grows, toward the final shape.

### Surveying (RS)

- **RS01** Code SHOULD be refactored, cleaned, re-audited and semantically compressed continuously; nothing is final.
- **RS02** Agents MUST NOT assume existing code is good because it exists, is in use or is tested.
- **RS03** Failing tests MAY be wrong instead of the new code.
- **RS04** Tests and expectations MUST NOT change without explicit prior discussion and agreement.
- **RS05** Stubs that clearly mark an unfinished shape SHOULD NOT be removed when they are meant to be filled in later; ask when unsure.

- **RS06** Refactors MUST start by surveying the Chesterton's fence of the status quo: how it got there, and why it might be that way.
- **RS07** Nouns, verbs, types, variants, fields and lines MUST each be earned.
- **RS08** Final models MUST capture the essential complexity of the problem in its most pristine form, nothing more and nothing less.
- **RS09** Bloat SHOULD be removed as soon as it becomes visible, since it often only shows as the problem's true shape emerges.

### Replacing (RR)

- **RR01** Code SHOULD be refactored as understanding deepens and the right shape reveals itself; programming is refactoring as writing is editing.
- **RR02** Refactors SHOULD be painless and touch only the parts of the model that need to change.
- **RR03** Refactors that touch more than they should MUST be investigated.
- **RR04** Investigated refactors MAY widen, or trigger a follow-up, to crispen the model's boundaries.

- **RR05** Changes SHOULD make the next change easy first: "make the change easy, then make the change".
- **RR06** Changes SHOULD skip intermediate and transitional states and go straight to the final model.
- **RR07** Code MUST NOT introduce transitional or "for now" logic unless explicitly requested.
- **RR08** Components under roughly 5k lines MAY be ripped out and rewritten completely when that is easier.
- **RR09** Changes SHOULD break or change the source directly and follow the compiler to every usage site.

- **RR10** Issues in other systems MUST be surfaced and discussed instead of papered over or hidden.
- **RR11** Surfaced issues MAY be ignored only once explicitly acknowledged and deferred.

### Fixing (RF)

- **RF01** Bugs MUST be treated as misalignment, not as a natural phenomenon; correct systems come from clarity, simplicity, strong invariants and clear expectations.
- **RF02** Issues MUST be treated as challenges to the model, answered by the model's long term shape rather than by standalone fixes.
- **RF03** Fixes SHOULD NOT be additive or applied en masse; checking invariants is fine, making bad states impossible is better, and simplifying / compressing the model or avoiding the problem alltogether is best.
- **RF04** Failures SHOULD act as a searchlight: study the model they expose, find its gaps, and correct the model in the most general, compressed, subtractive way.

## Check (C)

Check every change with tests, formatting and lints.

### Testing (CT)

- **CT01** Tests MUST live in the package that implements the behavior they check; packages MUST NOT test upstream mechanics they only use, such as access decisions in an app.
- **CT02** Tests MUST NOT rely on factory or dependency injection layers.
- **CT03** Code that is awkward to test SHOULD be refactored, since it is almost always poorly factored.
- **CT04** Hard-to-test code SHOULD get crisper models and more realistic tests instead.

- **CT05** Tests MUST be named with a verb that states their content, such as `test_roundtrip_duration` or `test_send_receive_message`.
- **CT06** Test first lines or docstrings MUST describe the desired behavior without mentioning "test".

- **CT07** Tests SHOULD be property-based or roundtrip tests where possible.
- **CT08** Tests SHOULD exercise the entire thing over a part, comparing complete output.
- **CT09** Tests SHOULD assert the entire expected output as a snapshot.

- **CT10** Tests MUST assert specific outcomes, such as "these two errors with that message", not "expect failed" or "any two errors".
- **CT11** Tests MUST NOT assert partial values such as `x.contains('part of foo')`. (`destack/no-partial-assertions`)
- **CT12** Non-trivial assertions SHOULD be commented like other logic blocks.

- **CT13** Tests slower than a few milliseconds, tens at most, MUST be investigated for fundamental model issues.
- **CT14** Test runs MUST use a tight timeout, such as building first without a timeout and then testing with one; slow tests are a hard failure.

### Formatting (CF)

- **CF01** Code MUST be formatted before a change is done.
- **CF02** Code SHOULD be formatted before it runs or is checked, so it compiles only once.
- **CF03** Formatting MUST stay within the change's scope, using the directory's `just fmt` or equivalent.

### Linting (CL)

- **CL01** Rust changes MUST fix all lints from `cargo check -p <crate>` and `cargo clippy -p <crate>`.
- **CL02** Too-many-arguments and type-complexity warnings MAY be ignored.

- **CL03** Clippy allowances SHOULD sit on top of the `impl`, not on individual functions.
- **CL04** Lint suppressions MUST sit at the top of the impl block, not on individual functions.

## Ship (S)

Land changes through branches and plain commits.

### Branching (SB)

- **SB01** Work SHOULD happen on branches and worktrees off a main branch.
- **SB02** Branches SHOULD rebase off main frequently and merge back into main.

- **SB03** Merges into main SHOULD fast-forward or cherry-pick to retain history.
- **SB04** Merges into main MAY squash many small commits.

### Committing (SC)

- **SC01** Agents MUST NOT commit or merge without explicit instruction.
- **SC02** Agents MAY present suggested commit slices after re-reviewing their work.
- **SC03** Commit messages MUST NOT mention non-human authors or contributors: no co-authors and no bylines.

- **SC04** Commits MUST use conventional commits in present tense, Simplified Technical English and verb-first shape.
- **SC05** Commit subjects MUST follow `type(scope): verb noun` with an imperative summary under 100 characters.
- **SC06** Commit scopes MUST use the full, sometimes stylised, scope, such as `language/ast`, `language/compiler/analyze` or `library/ui`.
- **SC07** Commits touching several scopes MUST use the highest scope or `all`, or split into smaller commits when large.
- **SC08** Commits in large packages or crates MAY use subscopes such as `language/compiler/sema`.

- **SC09** Commit types MUST be one of, from most to least coalescing:
  - `feat`: extend, generalise, or add non-trivial model changes
  - `refactor`: replace or reshape the model in some non-trivial way
  - `fix`: correct the model in some non-trivial sense that did not require a refactor
  - `test`: extend, update, bless, or otherwise modify the tests
  - `chore`: modify or adapt in a semantically trivial way (format, trivial API adaptation, ..)
  - `docs`: edit the docs (in code or otherwise)
  - `dev`: make some meta change about the dev setup

- **SC10** Commit messages MUST be boring like boiled potatoes and dry bread: what changed, as plainly as possible, without a story.
- **SC11** Commit messages MUST NOT mention meta context such as "part 1" or "landed feature X".
- **SC12** Commit messages SHOULD use simple verbs (add, remove, reshape, move, update X to do Y) and specific code concepts.
- **SC13** Commit messages SHOULD name code concepts as proper nouns, not files: `add SiteTable, rename Foo -> Bar`.
- **SC14** Commit messages SHOULD follow the style of the last 50 or so commits when in doubt.
