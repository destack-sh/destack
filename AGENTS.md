## Code Style

### Naming

- Names should be obvious, clear, and idiomatic to the language and topic.
- Shorter, stronger nouns and verbs are almost always better.
- Clear naming, pristine nouns and verbs, are part of a clear design. As a corollary, muddy naming strongly indicates an unclear design with muddy boundaries.
- Where relevant prior art exists, we should follow existing modern terminology.
- Prefer writing out most names and words (even in variable names, `extension` > `ext`, `directory` > `dir`).
- As with logic, symmetry in naming across related logic is simpler, and simpler is better.
- Avoid single-letter variables unless obvious (e.g., `i`, `x`, `Vector.x` are fine).
- Booleans should start with `is_` unless already clear (or otherwise required by context), though enums are usually better anyway.
- Abstraction sludge names like "seam", "lane", "parts", "info", "factory", "syntax", "semantics", "data", "inner", "wrapper", "facts", "summary", "channel", "boundary", "contract", "surface", "currency", "load bearing", .. and friends are to evil, they're be treated with high suspicion and are almost certainly wrong (and temptation to use them implies conceptual muddiness that should be revisited).
- The same logic applies for module and file names too: single part file names are clearer while "support", "helper" and "utils" are sludgy.
- It can be tempting to name things along the lines of "x_for_y" in certain overload-ish situations, however, this is almost always a modeling smell and means we haven't properly generalised or reified our invariants yet. (Note that this does *not* mean we should introduce arbitrary interfaces or abstractions just to please this rule, that would be just another factoring issue. Sometimes "x_for_y" is fine, commonly in data transcribing, but usually it's just sludge.)
- The name of a thing should describe its actual behavior or purpose. This sounds trivial, but e.g., when a function creates or updates a variable, it should be called `upsert*`, when a function only conditionally allocates something it should be called `allocate*maybe` (or `allocate*if*`), and so on.

### Logic

- Less is more, every line of code is a liability, every bit of state is suspicious. Fewer overloads are better, fewer fields are better, fewer dependencies are better, etc.
- When writing some logic or function and it turns into 500 lines, wonder if it could be done in 100 lines. If it's 100 lines, maybe it could be 10. If it's 10, maybe we can remove it altogether, or phrase the problem differently to eliminate the need for this whole piece in the first place.
- Long methods are allowed if the logic isn't meaningfully extractable / reusable.
- Having many overloads (or quasi-overloads) that just call one another with different arguments and little or no additional logic is almost always a smell and annoying to read (and a bad source of pointless code bloat).
- Prefer pure(ish) functions, pass in context explicitly when needed (usually as the last argument).
- Break larger code blocks into logical chunks with whitespace and/or preamble comments.
- All logic in functions and outside should be broken into small-ish coherent blocks (2-6 lines or so) with a preceding comment.
- Logic blocks are always separated by blank lines (except the very first in a function).
- Usually you want the comment before the if clause / loop / whatever, not inside.
- Every logic block should have a comment (returns may omit the comment), and every logic block (except the first) should have a blank line before it. See commenting for how to comment properly.
- The return value implicit or explicit should also have a blank line before it, even if it's uncommented (which is, again, fine).
- Use temporary variables for non-trivial operations (yes, it's deliberately verbose, but really only for non-trivial operations, unary operators like reference/dereference, plus/minus, .. don't need temporaries):

```rust
let first_digit = (dt_bytes[0] - b'0') as i64;
let second_digit = (dt_bytes[1] - b'0') as i64;
let number = 10 * first_digit + second_digit;
```

- It is usually preferable to "spell out" branches at the same "level" whenever possible, instead of doing repeated continue/return/whatever jumps (which are harder to trace mentally):

```
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

- As a corollary, it is good practice to try and keep the flow, breadth _and_ depth of branching predictable and consistent to make it easier to scan.
- There are really two main kinds of branching: unexpected / early exit guards, and "main" if-else-if-else chains (however they may manifest). Early exist can use the if-jump/return style, but anything that is a serious of if-jump-if-jump-if-jump should usually be turned into a coherent logic blocked legible chain as above with proper if-else-if-else chains, and/or use match statements:

```
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

### Factoring

- The point of all code is to solve real-world problems and model them with the fewest, most pristine nouns and verbs (types and functions) possible _that the machine understands well_, using the fewest possible resources (bytes, instructions, cycles, whatever) on the expected hardware and under expected usage scenarios.
- Where good relevant prior art exists, we should try to follow it, especially in terminology, configuration, interfaces, and even behavior where sensible.
- Most code on the internet, on StackOverflow, or on open source libraries, and even in their documentation, is not very good. Anything external we take in should be treated with great suspicion.
- Every proposed change is really a question: "what shape should the codebase have in the long term to support changes and features _like_ this?"; the answer to that question leads to a more maintainable codebase, even if it means more work in the short term.
- Sometimes the right answer is "no", and the right response to a change is "no, not here, not now".
- One of the few things worse than superfluous duplication is forced abstraction.
- Often, when properly factored, the real world (and thus the way to model it) is surprisingly symmetrical at varying scales (types, functions, files, modules, sub-systems). Identifying symmetry and generalising it - even if only informally, no "real" language-level interface required - is extremely valuable (naming, parameter conventions, file names and placement, module layout, .. anything).
- Logic should be "incrementally granular" (as per Casey Muratori), i.e., ideally we should be able to reuse logic _and_ state at various pieces of granularity.
- Conceptually, incremental granularity means not hiding details too much, and assuming (especially internally, within the castle) that the caller is a consenting adult.
- Relatedly, try hard to _avoid_ "banana and the jungle" shaped model solutions where pulling in one component requires pulling in a whole deep object graph (except in situations where there really are obvious god objects, like a current `World` in a game or whatever).
- That said, it is often beneficial to have strong clear nouns and verbs, and it's usually easier to think about state when it is bundled in nouns (dare I say "objects", but no OOP abstraction nonsense).
- Even associated functions (that don't depend on state at all) often benefit from being tied to relevant nouns in cases where one presents itself, just because it reads nicer.
- More specifically, as a trivial example, when a function takes an array of something, try to make it work on a single "element" instead and just loop in the caller. Prefer parametric mutability. etc. etc., that sort of thing.
- Usually, in each file, the "top" / most important nouns should go up top (constants at the very top above it), followed by successively more internal / inner nouns, and any relevant free functions at the very bottom (+ tests as needed ofc).
- Generally, methods that _could_ be methods _should_ be methods - if we have a top level function like `foo(definition: &Definition) -> bool` we should probably just make that `Definition.foo`
- Often, when we're tempted to add a matrix of methods like "x_for_y", the more pristine factoring is to back up and (re)align state and logic construction flows in a more natural way.
- When a method mutates state it should be obvious by name and signature, and ideally we want to return mutated state / take the mutator instead of mutating internally when possible (e.g. `resolve_x` should return the resolved thing, not mutate an internal resolver cache and return void). This isn't always possible, and performance matters a lot, but when we can have both it's much preferred.

### Refactoring

- Just like writing is editing, programming is refactoring, and we refactor as we go and as our understanding of the problem deepens and the right solution shape reveals itself.
- If we do our job right, and have the right level of testing, refactors should be reasonably painless and only touch the parts of the model we actually needed.
- If we find that refactors are touching more than it "should"; that is worthy of investigation and maybe we should broaden the refactor or do plan a follow up refactor to crispen the boundaries of the model (if we can, this doesn't always work unfortunately).
- As with factoring, we should always try to make our work easier as we go: "make the change easy, then make the change". This often means we _should_ abandon "intermediate" or "transitional" states and just go straight for the final model / solution we want.
- Sometimes it is however easier to just rip out a component altogether and rewrite it completely, especially if it's say <5k LoC or so.
- We should always strive to refactor and "clean" as we go, continuously re-audit and semantically compress where the opportunity presents itself. Nothing is final.
- Relatedly, as we go, we must never assume that what is already there is good just because it exists, even if it's in use, even if it's already tested.
- As a corollary, failing tests do not _always_ mean that the new code is wrong, the tests might also be wrong. That said, tests and expectations should never be silently changed without explicit prior discussion and agreement.
- Every noun, verb, type, variant, field, line, .. must be earned. The final model should capture the essential complexity of the problem in its most pristine form, nothing more, nothing less.
- Bloat is deadly, and often we only realise something was bloated as we get further along and the true shape of the problem reveals itself (hence, refactor as we go)
- Almost never introduce "transitional" or "for now" logic, we always want the final ideal shape, nothing in between.
- In general, it is quite often better to break / change the source directly and then let the compiler guide us to all usage sites.

### Comments

- Inline comments should be short and begin with a lowercase letter.
- (This extends to comments in _any_ code file, even scripts. I just like lowercase better.)
- Place comments above a related code block (usually 2-10 lines).
- Most comments are <1 sentence and should not include a period at the end (again, lowercase).
- Avoid using hyphens inside comments, instead prefer colons or commas (except for proper compound words)
- Inline comments may also just be single words or sequences of words if the "scoping" is clear; i.e., not every inline comment needs to be a sentence.
- Comments serve to organize the reader's mental model of the code, so they can be just anything from a one-word summary, a three word phrase, or a short explanatory note.
- Most logic block comments of more than one/two words should be action / verb shaped, e.g.:
  "// build drop plan for each function" is much better than "// each function gets an independent drop plan" (begin with a verb!)
- Trivial functions (<3-4 lines) do not _need_ comments / blank lines, especially when the comments just repeat the documentation above.
- Also, tests don't need quite the same level of comments, especially within obvious test cases.
- Documentation comments for functions/types/etc. _should_ be proper sentences _with_ punctuation.
- Files should NOT have a top-level documentation comments. They always get stale.
- Go multiline if there is more than one sentence. Only one sentence should begin per line.
- For methods, documentation should be imperative, usually starting with a verb (e.g., "Send a message").
- _All_ functions, types, variants/fields, etc. should have documentation (one line is fine).
- Documentation comments do not need to start with a verb, they should just plainly state what the thing is (e.g., for a field, "The blocks built so far." is better than "Represents the blocks built up to this point."; more succinct is better).
- When documenting if/else-if/else-_like_ logic, the comments should go _before_ each case like so:

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

- The logic block treatment also applies just as well to TSX and tree-like structures, so for example:

```tsx
<div>
    {/* Top button */}
    <button /> ... </button>

    {/* Side panel */}
    <div> ... </div>
</div>
```

- For ===-like separators for large comment blocks, you may use upper case sentences:

```text
// ================================================================================
// Binary operator precedence
// ================================================================================
```

- Though try to minimize the number of these, they're quite noisy.
- Comments MAY start with keywords:
    - `NOTE`: call out something important
    - `TODO`: something to address eventually
    - `FUGU`: temporary, f-ed up, should be addressed before going upstream
- Keywords should include tags (like "NOTE #Suspicious: allocating in runtime seems wrong?"):
    - `#Performance`: could be faster or more efficient
    - `#Robustness`: might be flaky in some cases
    - `#Broken`: doesn't work in likely cases
    - `#Cleanup`: could be simpler or better structured
    - `#Incomplete`: obvious feature is missing
    - `#Suspicious`: something that looks wrong or weird
    - `#Security`: may allow more access than intended
    - `#Architecture`: larger design issue to reconsider

### Performance

- Performance is a feature and always a strong implicit requirement, even when no hard boundaries have been set (and usually, they aren't).
- The folk-lore idea that "premature optimisation is the root of all evil" is wrong, since what makes modern computers happy (clear, compact, aligned data structures and simple parallel processing) also lines up very well with what makes modern software pristine.
- We should always try to stratify and define the performance characteristics of any systems we work with before we touch them and keep them in mind while we work. (What are the bounds for X, Y, Z? latency, RPS, IOPS, throughput, what about p50 p95 p99, ...)
- Performance has many meanings, but in general it means using the absolute minimum level of resources to solve the real problem we actually have (bandwidth, disk, memory, CPU, whatever it is).
- Often, though not always, performance "tradeoffs" - like between memory usage and cycles, or between niceness and speed - are not really tradeoffs at all, just poorly factored code that could be much better if we zoom out a little and solve the problem well (or find a way not to do it at all!).
- Clean code is usually fast code, if by "clean" we mean properly semantically compressed, stupid simple approaches, and not some arbitrary and silly notion of convoluted, theoretical abstraction ideals.
- The fastest code is code that doesn't run at all, the best data structures are the ones we don't need. Text book data structures, algorithms and fanciness are rarely required.
- Most of the time, for most problems, arrays and linear approaches are perfectly fine and even beat out anything "smarter". Maps are okay too, usually.
- Memory access patterns are the dominating factor in most modern software problems, thus, something "dumber" but tighter (like a dense array) is often faster than something "smarter" but looser (like a map) even at high scales.
- Have sympathy for the real hardware and underlying machinery that must actually execute whatever we write down, and usually that happens in roughly the same way we wrote it, since compilers can't be that smart.
- Hardware awareness and full stack understanding are especially important in areas we do not fully control, like when we codegen to JS or write something to the web, or some foreign graphics API - how does it _actually_ execute? Which low level operations does what we're doing map to, and what do we really need? 
- Working bottoms up - which bits and cycles do we _really_ need to spend - is the only true way to bound the lower end of performance, and often a great way of demystifying a system and getting order of magnitude improvements.

### Failures

- Always prefer explicit, loud errors through conventional, idiomatic channels.
- As a corollary, silent failures of any kind are evil and only ever cause downstream trouble. We must never fail silently in any live code, and it's _especially_ evil to suppress failures in a way that doesn't even look like a failure (silent fallbacks, defaults, null-ish / sentinel values, etc.).
- Outside of tests, errors should almost never be suppressed or somehow fall back to "default values" (especially evil are things like defaulting `unwrap_or(0)`, or other special values like `-1`, `MAX`).
- On the flipside, in general, and especially internally, we should assume that both sides of an API are consenting adults and we should _not_ check every conceivable failure state in every location - this is usually more noise than it's worth.
- Specifically, being overly defensive and "scared" in some code path is usually a big smell that we haven't really understood and defined the model and its invariants well enough yet. (e.g., handling usize overflows in a modern allocator is just noise)

### Boundaries

- Prefer loud failures even and especially for invariants coming from other subsystems, and _especially_ for subsystems we control.
- For example, if some upstream shape or contract implies a certain field in some state should be there at some point, but it's not, we MUST treat that as an error instead of working around it in any capacity.
- Attempting to work around issues in upstream / other dependencies is always dangerous, but doing it for dependencies _we control_ is just a recipe for maintenance disaster.
- Invariants should be clear and crisp, and if they're not, that is a design issue to be surfaced and discussed.
- Stronger, harder invariants are usually _more_ forgiving than looser ones since they force the consumer into the right model, which is more predictable and crisper for all.
- The "higher up" / "sooner" we can encode requirements, expectations and invariants, the better, that is, if the compiler fails on bad usages that's ideal, if the linter fails it's still good, if the unit tests fail also good, then we go down the list of less desirable places to find out something is wrong.

### Dependencies

- Fewer dependencies is better, but sometimes it's worth it, especially when they wrap or define som ebig ugly contract (a la `windows_sys`) that we would just have to redefine and maintain ourselves anyway.
- When simple logic is needed, we just implement it ourselves.
- Moderately complex logic is sometimes vendored.
- Complex or dev-only dependencies are sometimes okay.
- When adding a dependency, we should use the latest _stable_ version.

### Testing

- If something is awkward and hard to test, it is almost always poorly factored.
- That, however, does not mean introducing factory / DI sludge, instead, there is basically always a better way with crisper modeling and running more realistic tests.
- Tests should start with `test_` (or equivalent) and state their content as a verb. (e.g., `test_roundtrip_duration`, `test_send_receive_message`)
- The first line or docstring should describe desired behavior (don't mention "test").
- Prefer property-based testing and roundtrip testing where possible.
- If there is an opportunity to test "the entire thing" vs "part of it", prefer complete exercises and assertions (e.g., if we're generating string output, compare the entire output, not just "contains").
- More generally, we should always test _specific outcomes_ like "these two errors with that message" rather than "expect failed" or "any two errors".
- Even better, where possible, we should assert the entire expected output (snapshot style) rather than just "contains" or "doesn't contain".
- For any non-trivial assertions you should comment the logic block like we do with any other logic block, though you don't need to comment _every_ logic block as with regular/main logic.

### Formatting

You should always format code before you're "done" with a change.
Ideally, you should format code _before_ running it (via tests or otherwise), so we don't compile twice.
(Most directories have a `just fmt` or equivalent command, see the context.)

## Rust

- Comments/documentation goes before _all_ attributes (like `#[inline]`, `#[derive]`, etc.)
- No `crate::X` within functions, prefer relative references (again, imports at the top)
- Place imports at the top, prefer `use std::time::Instant` patterns
- Just use `pub use submodule::*` for public exports, we use `pub` properly
- Relatedly, we like to just use `use crate::x` directly at the top level (when possible)
- `mod.rs` and `main.rs` are intended strictly for re-exports (and submodule declarations like `mod submodule;`)
- Modules should either be `module.rs` or have `module/mod.rs` + real `module/whatever.rs`, never both
- `Into/From` is great (and sometimes `TryFrom`) and self contained to/from style conversions should usually use them for clarity
- Avoid "hiding" payloads inside `enum`s that sound like scalars (e.g., no values in variants of a `*Kind` named enum)
- Avoid `include!` or convoluted `#[path]` to bypass
- Avoid nesting `mod x { }` inside a file (except for `tests`)
- Avoid `Cell` and `RefCell`, they almost always imply a bad ownership model
- Import aliases are evil. 
- Prefer direct `expr.clone()` over `Arc.clone(expr)`
- Heavy `.clone()` are to be avoided (memory is expensive, fragmentatio is even more expensive)
- Some `unsafe` is not that terrible if we can prove and test the invariants
- Put constants at the top of the file (no magic numbers/values)
- Avoid `unwrap`/`expect`/`panic` etc. outside tests; fail explicitly, use proper Result handling
- Tests go in a trailing `mod tests` or in standalone test modules/crates (contextual)
- Inline variables in format macros if possible: `format!("name is {name}")`
- Prefer multiline raw strings for longer strings
- Prefer re-defining variables if we're just transforming them
  (e.g., `let module = modules.get(); let module = module.read();` is fine)
- Avoid nesting items inside of functions (like other functions, lambdas, types, etc.)

## Working Style

- You should always try hard to behave in accordance with this and proactively work this way, and suggest the right tools, media forms, representation, and questions to nail down the final design _before_ we get started and keep at it as we keep going.
- Always try to illustrate any point or decision with concrete data shapes, interfaces, code snippets, sample data, noun / state / verb diagrams, or whatever other concrete artifact helps nail down the exact workstream and tradeoffs.
- In general, there are two good ways of shaping out what some software should look like: big boxes with lines (and concrete data structures, interfaces and methods), and tracer bullets (that actually run and connect it all for some vertical slice).
- We like to use both, and we like to use both in tandem, they are very complementary. Usually we begin with the first to center the discussion, then sketch out and implement the second, then fill in and feel out the rest in bursts to lock in the shape.
- The whole point of writing software is to model and solve some real world problem (in a way that is machine-emphatic and actually executable efficiently.)
- Usually, we should try to figure out the main nouns and verbs (data structures, fields, and methods) first, and the main call flows between them. Who owns what state, who reads / writes what where and in what order.
- Ideally, for anything we expect to execute halfway frequently, we should think hard about how to use data oriented design and reason through the actual minimal mechanical steps that the target architectures will have to do, both compute and memory (and bandwidth etc.) wise, to do what we're asking. This matters tremendously.
- Data structures are incredibly important and I usually want to see them first since they clarify so much about the design. Whenever possible, this should be actual code in whichever languages we're using showing the real changes to / additions of data structures (and which values and value ranges we expect them to have).
- Code and actual logic is always useful to show and illustrate ideas, even in pseudocode form, but ideally in a real form that we actually expect to execute on some level. Think like an API designer here, since really, everything is an API in some sense.
- Bugs are not a natural phenomenon in code, it's just misalignment. "Bugs happen" is a fallacy, and we should try hard to engineer correct systems with clarity, simplicity, and reliability. Usually, this means simplicity, strong invariants, and clear expectations.
- Relatedly, "fixes" are not a natural activity in a healthy codebase _on their _own_ - instead, we want to treat every issue and every "bug" as a challenge to the model we're implementing, and figure out what the long term shape of the model should be. 
- Corollary: fixes are rarely _additive_, and certainly not "en masse". It is tempting but wrong to add many "fixes" to robustify some part of a codebase, but true correctness generally requires changing and refining the model. Checking invariants is fine (and inevitable), making bad states impossible is better.
- When possible, we should first think through what the example use cases would write in code to do the thing that we're trying to implement, where they're coming from, what the limits and expectations and environment is, and so on:

```
const user = service.signup(...); // user from API or wherever
// ... some more illustrative logic ...
```

- When possible, we should model the noun trees and the main boxes and lines in ASCII form, either as literal ASCII art with boxes and lines and/or with nice noun trees and schemas. Ideally we should annotate exact field names and types, though both can be complementary

```
Heap // per owner
├── HeapOptions
├── HeapLimits
├── GcPacer
├── GcRequest
└── HeapStorage
    ├── Arc<Allocator>
    ├── PageSpanCache
    ├── AddressSpace
    ├── YoungSpace
    │   ├── YoungCursor
    │   ├── YoungRange[]
    │   ├── YoungSpan[]
    │   └── page_spans[]
    ├── SmallSpace
    │   ├── SizeClassTable
    │   ├── SmallSpan[]
    │   └── partial_spans
    ├── LargeSpace
    │   ├── LargeBlock[]
    │   └── free_large_block_ids
    ├── page_map: logical page -> HeapPageMapEntry
    ├── AllocationUsage
    ├── young AllocationUsage
    ├── GcState
    └── LocalGcState

/// One pending local GC request.
pub(super) enum GcRequest {
    /// Run one young mark-and-sweep cycle.
    Minor,
    /// Run one full mark-and-sweep cycle.
    Full,
}

/// One heap small space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SmallSpace {
    /// The configured size-class table.
    pub(crate) size_classes: SizeClassTable,
    /// The configured span width.
    pub(crate) span_size_bytes: usize,
    /// The live heap spans.
    pub(crate) spans: CowTable<SmallSpan>,
    /// The reusable non-full spans per exact small-span class.
    pub(crate) partial_spans: BTreeMap<SmallSpanClass, Vec<usize>>,
}

// ... and so on ...
```

- When possible, we should literally think through example use cases for every main scenario, and then trace it out across real state and call flows in a tree form:

```
UserService.signup(name: string, email: string, password: string, session, ...)
 -> User.create(..., session)
   -> User.validate(...)
   -> User.save(...)
   -> NotificationService.send()
    -> Workflow::trigger()
   -> Session.commit(...)
```

### Checks

- Fix all the lints from `cargo check -p <crate>` and `cargo clippy -p <crate>`
- Most clippy allow stuff should go on top of the `impl`, not individual functions (like too many arguments is almost always fine at a broad scope)
- In general, ignore too many arguments and type complexity warnings
- Put lint suppression at the top of the impl block, not individual functions

## Markdown

- One sentence per line. Always (in prose, tables and such are different).
- Use proper rich formatting: sections, sub-sections, highlighting, code examples, tables, etc.
- Non-prose items (lists, code blocks, tables) in a subsection should be preceded by a prose line

## Commands

We use `justfile`s for commands. See `just --list` for all commands.
Be careful not to pull in unrelated fmts / checks for local edits, that might make the diff noisy.

```sh
just check
just fmt
just build
just test
```

### Commits

- Typically, agents aren't supposed to commit or merge directly without being explicitly instructed to.
- For commit message format, follow `CONTRIBUTING.md#commit-style`.
- We typically work with branches and worktrees off a main branch.
- We try to frequently rebase off main and merge back into main.
- When merging into main, try to fast-forward or cherry-pick to retain the commit history (except when there are a _lot_ of small commits, feel free to squash then).
