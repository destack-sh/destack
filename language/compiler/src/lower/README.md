# Lowering: DIR to MIR

This document describes how Destack's high-level semantic representation (elaborated, canonical DIR) is lowered to machine-level IR (MIR) for native targets.

See also:
- [INTRINSICS.md](INTRINSICS.md) for intrinsic operations
- [INTEROPERABILITY.md](INTEROPERABILITY.md) for JS/TS compatibility and FFI

---

# Overview

## Objectives

The overarching dream is **Rust performance with TypeScript semantics and ergonomics**.
Naturally, performance and ergonomics are in some tension, and we want to enable *up to* Rust performance with some additional constructs while improving modern TS performance to around Go/C#-level reliable performance without _requiring_ additional changes:

- **Best case (target):** Rust-tier performance (zero-cost abstractions, no GC pauses)
- **Average case (target):** Go-tier performance (efficient GC, good concurrency)
- **Worst case (target):** Competitive with optimized JS runtimes (V8, JSC, SpiderMonkey)

AOT compilation provides predictable performance without warmup, but astounding levels of engineering have already gone into making V8's speculative optimization beat static compilation on some dynamic patterns.
Our advantage is consistency and control, and, of course, you don't need to ship a JS runtime.

Specifically, Destack lowering is focused on:
1. **TypeScript semantics**: TS and Destack code behaves identically^x
2. **Comptime**: Full compile-time evaluation
3. **Reflection**: Types-as-values for comptime and runtime reflection
4. **Ownership**: Manual memory or GC as needed
5. **Erasure**: Clean codegen to JS/TS

x=Where behavior differs between JS/TS runtimes and native, this difference should be obvious and misuse should have loud diagnostics. Perfect semantic equivalence in all scenarios is not required or even possible, since that would require emulating _all_ the non-standard dynamic quirks of common JS runtimes (like optimizer behavior, scheduling, etc.).

### Performance

Destack targets Go-level performance by default and Rust-level performance on-demand.
The compiler relies on a known-good set of optimizations proven out by Go, Rust, Zig, and modern C++ compilers:
- Escape analysis and stack promotion of managed allocations
- Copy elision, return value optimization, and move elimination
- Bounds check elimination with range analysis
- Devirtualization and inlining for class calls
- Monomorphization and specialization control per profile
- Strict borrow mode for `&mut` to enable `noalias` and vectorization
- Explicit SIMD types and intrinsics with scalar fallback
- LTO and PGO for package and program scope inlining and layout decisions

Compilation units are defined by ltoMode.
Module is the default compilation unit.
Thin LTO uses package scope and Full LTO uses program scope.
Auto enables Thin LTO at O4 and disables LTO at lower levels.
Lower emits deterministic mangled symbol names into MIR.
These names are used as the stable identity for cross module call graph stitching.

SIMD follows a Zig-like model.
Vector lane counts are static parameters and operators are elementwise.
Lower maps vector operations to target SIMD instructions when available.
(If the target lacks support, Lower scalarizes to loops.)
See [INTRINSICS.md](INTRINSICS.md#simd) for details.

## Pipeline Position

Lower is **phase M** in the compiler pipeline (see [compiler/README.md](../README.md)).
It receives patched DIR from Execute and produces target-specific MIR.

Pipeline summary:

| Stage | Output | Notes |
| --- | --- | --- |
| Execute | DIR (patched) | runs comptime blocks via MIR |
| Generate/JS | JS/TS output | preserves type-erased polymorphism |
| Lower | MIR | monomorphized, typed, target-specific |
| Optimize | MIR | verify + transform passes |
| Generate | native binary | Cranelift backend |

Pipeline diagram:

<pre>
DIR (elaborated, canonical, profile-dependent)
 │
 ├─→ Execute: run comptime blocks via MIR, patch DIR
 │
 ├─→ Generate/JS: direct JS/TS output (preserves type-erased polymorphism)
 │
 └─→ Lower (THIS DOCUMENT)
      │
      MIR (monomorphized, typed, target-specific)
       │
       ├─→ Optimize (see optimize/README.md)
       │    ├─→ Verify: borrow-check, move-check, drop-insert
       │    └─→ Transform: inline, dead code elim, escape analysis, etc.
       │
       └─→ Generate
            └─→ Cranelift → native binary (.exe, .dylib)
</pre>

### Input: Canonical DIR (Comptime Patched)

Lower receives "canonical" typed DIR after Analyze, Elaborate, and Execute (see [analyze/](../analyze/) and [elaborate/](../elaborate/)):
- All desugaring complete (e.g., `+=` → `+` and assign)
- All patterns expanded to decision trees
- All types fully inferred ("Types")
- All overloads resolved ("Resolutions")
- All static parameters resolved to values ("StaticExpression")
- All polymorphic instances created ("Instances")
- All comptime blocks executed and results patched in

### Elaborate Transforms

The following constructs are transformed by Elaborate before Lower sees them:
- **Destructuring patterns** → explicit field/element access
- **Spread/rest operators** → explicit array operations
- **Range literals** → `RangeExclusive`/`RangeInclusive` structs
- **Match expressions** → decision trees with `is` type checks
- **Default arguments** → `arg === undefined ? default : arg` checks
- **Null coalescing** (`??`, `?.`) → explicit null checks

Lower receives canonical DIR with these constructs already desugared.
See [elaborate/README.md](../elaborate/README.md) for transform details.

### Output: Target-Specific MIR

MIR is generated per-target with target-specific decisions:
- Memory layout (LP64, ILP32, etc.)
- Calling conventions (C, System, etc.)
- Alignment requirements
- Policy-controlled checks (bounds, overflow, etc.)

## Phases

Lower executes in four explicit phases with clear dependencies:
Phases flow left to right: **Types → Declarations → Tables → Emit**.

<pre>
┌─────────────────────────────────────────────────────────────────────────┐
│                          Lower Pipeline                                  │
│                                                                          │
│   Phase 1      Phase 2          Phase 3        Phase 4                  │
│   ───────      ───────          ───────        ───────                  │
│    Types   →  Declarations  →    Tables    →    Emit                    │
│                                                                          │
│   layouts      signatures       vtables         blocks                   │
│   lineages     globals          itabs           terminators             │
│   slots        init order       RTTI            ownership                │
│                                 string tags     barriers                 │
└─────────────────────────────────────────────────────────────────────────┘
</pre>

| Phase | Input | Output | Why Separate |
|-------|-------|--------|--------------|
| Types | DIR types | MIR types, layouts, lineages | Must know sizes before allocating |
| Declarations | DIR items | Function shells, globals | Enables forward references in Emit |
| Tables | Types + lineages | VTables, ITabs, RTTI | Needs complete type info |
| Emit | Everything above | Complete MIR functions | Has all context for codegen |

See [Phases](#phases-1) section for detailed descriptions.

## Implementation Structure

Lower mirrors the phase boundaries in its module layout:

- `module/`: orchestration and phase sequencing
- `type/`: type lowering and layout (including nominal field layouts)
- `item/`: declaration lowering (globals, functions, methods)
- `table/`: dispatch tables (vtables, itabs) and RTTI
- `emit/`: function body lowering (statements, values, control)

Nominal layouts are computed from declared fields only and are predeclared in the Types phase.

## Layout Map

Lower treats layout as a **queryable, cached graph** instead of a monolithic pass.
The goal is to keep representation decisions explicit and local.
Lower can answer "what is the layout of this type?" at any point during lowering.

### Layout Categories

We model layout in two layers:
- **Shape**: logical fields, tags, tables, and constraints.
- **Placement**: concrete offsets, alignment, size class, inline vs boxed decisions.

This keeps unions, intersections, and interfaces manageable.
Their shape often exists without a single concrete placement.

Layout categories are Lower metadata, not MIR:

| Category | Shape | Placement | Notes |
| --- | --- | --- | --- |
| Scalar/Immediate | scalar value | fixed width | includes tagged pointer immediates |
| Pointer | address | pointer-sized | heap-managed or external |
| Tuple | ordered fields | packed/offset fields | homogeneous is still tuple |
| Struct | named fields | packed/offset fields | nominal, value semantics |
| Class | instance fields | managed reference to payload layout | reference semantics |
| Array/Slice | element + length | header + data | policy: inline vs heap |
| Function | signature | pointer or fat pointer | closure captures add env ptr |
| Tagged Union | tag + payload | inline or boxed | tag value + payload layout |
| Untagged Union | set of layouts | external discrimination | RTTI or caller-provided tag |
| Interface | dispatch surface | itab/vtable + data | separate dispatch layout |
| Intersection | composed view | no new storage | layout = primary + itabs |

### Structs, Classes, and Boxing

Struct layouts describe value payloads with field offsets.
Class instance types are represented as `ref<managed @Payload>` where `@Payload` is the class field layout.
Dispatch metadata is attached out of line in type metadata, and polymorphic classes include a vtable pointer in the payload layout when virtual dispatch remains.
Lower inserts boxing when a struct value is used in a reference typed context, which is modeled as `managed.alloc` plus a `store` of the value.
The `new` expression constructs a value for structs and allocates a managed reference for classes.

### Union Strategy

Union layout is chosen per union:
- **Inline tagged**: tag + payload in one block (size ≤ 2×ptr size)
- **Boxed tagged**: tag + pointer to payload
- **Untagged**: no tag, relies on RTTI or external discriminant

The chosen strategy is stored on the union layout so Emit can generate the
correct tag checks and field accesses.

### Dispatch Layout (VTables/ITabs)

Dispatch layout is modeled separately from data layout:
- **VTable**: class/nominal method table for virtual dispatch
- **ITab**: interface table for structural/nominal interface dispatch

A type layout can reference zero or more dispatch layouts, but dispatch layouts
never alter the data placement (they are attached metadata).

### Layout Query Flow

Layout queries should be deterministic and cacheable:
1. **Resolve type identity** (nominal reference, instance arguments, etc.)
2. **Build shape** (fields, tag, tables, constraints)
3. **Select placement policy** (inline vs boxed, tag scheme)
4. **Finalize placement** (offsets, alignment, size)
5. **Cache** and return

This is the backbone for later work on unions, interfaces, and RTTI without
refactoring the phase boundaries again.

## Target Policies Summary

Lower behavior is configured by target policies (see [Target Configuration](#target-configuration)):

| Policy | Effect on Lower |
|--------|-----------------|
| `boundsChecks` | Insert/omit array bounds checks |
| `overflowChecks` | Insert/omit integer overflow checks |
| `panic` | Abort immediately or unwind |
| `debugInfo` | Controls debug metadata granularity |
| `debugMode` | Execution mode for debug workflows |
| `osrMode` | OSR entry placement for native execution |
| `safepointMode` | Safepoint insertion strategy |
| `safepointInterval` | Instruction interval for safepoint polling |
| `speculationMode` | Guarded speculation mode |
| `profilingMode` | Runtime profiling |
| `determinism` | Deterministic scheduling and randomness |
| `replay` | External I/O record or replay |
| `stripLevel` | Symbol table stripping |
| `unwindFormat` | Unwind info format (DWARF/SEH/None) |
| `allocator` | Global allocator selection |
| `borrowMode` | Hint vs strict borrow enforcement |
| `relocationModel` | PIC/PIE/static code generation |
| `linkMode` | Static vs dynamic linking preference |

## VM and Runtime Interop

Lower must emit metadata for VM <-> native transitions.
This metadata enables deopt, OSR, and GC correctness when execution shifts between tiers.

The required metadata includes:

Lower emits runtime/VM metadata for transitions:

- **safepoint table** keyed by native PC
- **deopt maps** for reconstructing MIR frames
- **OSR entries** for entering native code at MIR block boundaries
- **GC stack maps** for managed reference tracing

### Safepoint Table

Safepoints are inserted at call sites, loop back-edges, and allocation points.
Each safepoint entry includes:

```ds
type DeoptMapId = uint32;
type StackMapId = uint32;
type OsrEntryId = uint32;

type Safepoint = {
    pc: uint64,
    deoptMap: DeoptMapId,
    gcStackMap: StackMapId,
    osrEntry?: OsrEntryId,
};
```

### Deopt Maps

Deopt maps reconstruct MIR frames from native registers and stack slots.
Frames are ordered from outermost to innermost, with the last frame active.

```ds
type FunctionId = uint32;
type BlockId = uint32;
type InstructionId = uint32;

type DeoptMap = {
    frames: FrameMap[],
};

type FrameMap = {
    function: FunctionId,
    block: BlockId,
    instruction: InstructionId,
    values: ValueLoc[],
    locals: ValueLoc[],
    returnDestination?: ValueLoc,
};

type Register = { kind: 'reg', index: uint16 };
type StackSlot = { kind: 'stack', index: uint32, offset: int32 };
type Constant = { kind: 'const', value: ConstantValue };
type ValueLoc = Register | StackSlot | Constant;
```

Missing values are materialized as `Void` during reconstruction.
Constants use the MIR constant encoding and do not require native storage.

### OSR Entries

OSR entries allow native execution to start at MIR block boundaries.
Each entry specifies the MIR block and the live value set required to enter.

### GC Stack Maps

GC stack maps identify managed references in native frames.
Lower must emit precise maps for all safepoints and native call frames.

### Metadata Encoding

NOTE #Incomplete #ABI: this metadata layout defines the ABI surface and will be tightened as the runtime and codegen converge.

Lower emits a single metadata blob per native artifact.
The runtime reads this blob to drive deopt, OSR, GC, profiling, and debugging.
All integer fields are little-endian.
All offsets are byte offsets from the start of the metadata blob.
`pointerWidth` is 4 for 32-bit targets and 8 for 64-bit targets.
`entrySize` is zero for variable-length entries.
Section headers are contiguous and ordered by ascending `offset`.

```ds
type MetadataHeader = {
    magic: uint32,
    version: uint32,
    pointerWidth: uint8,
    endianness: uint8,
    sectionCount: uint32,
    sectionTableOffset: uint32,
    compilerHash: uint64,
    targetHash: uint64,
};

const METADATA_MAGIC: uint32 = 0x44534d44;
const ENDIAN_LITTLE: uint8 = 1;

type SectionHeader = {
    kind: uint16,
    entryCount: uint32,
    entrySize: uint32,
    offset: uint32,
    length: uint32,
};

const SECTION_SAFEPOINTS: uint16 = 1;
const SECTION_DEOPT_MAPS: uint16 = 2;
const SECTION_STACK_MAPS: uint16 = 3;
const SECTION_OSR_ENTRIES: uint16 = 4;
const SECTION_PROFILE_SITES: uint16 = 5;
```

The runtime must validate `magic`, `version`, and `endianness` before use.
The runtime must validate `compilerHash` and `targetHash` against the executing artifact.
The runtime must reject overlapping sections and out-of-bounds offsets.
The runtime must reject section tables with unknown `kind` values.

### Safepoint Section

```ds
type SafepointEntry = {
    pc: uint64,
    deoptMap: uint32,
    stackMap: uint32,
    osrEntry: uint32,
};

const NO_OSR_ENTRY: uint32 = 0xffffffff;
```

`osrEntry` uses `NO_OSR_ENTRY` when no OSR is available at the safepoint.

### Deopt Map Section

```ds
type DeoptMapEntry = {
    frameCount: uint16,
    frameOffset: uint32,
};

type FrameMapEntry = {
    functionId: uint32,
    blockId: uint32,
    instructionId: uint32,
    valueCount: uint16,
    valueOffset: uint32,
    localCount: uint16,
    localOffset: uint32,
    returnDestination: ValueLocEntry,
};

type ValueLocEntry = {
    kind: uint8,
    regIndex: uint16,
    stackIndex: uint32,
    stackOffset: int32,
    constId: uint32,
};

const VALUELOC_REG: uint8 = 1;
const VALUELOC_STACK: uint8 = 2;
const VALUELOC_CONST: uint8 = 3;
const VALUELOC_VOID: uint8 = 4;
```

`VALUELOC_VOID` is used only for dead SSA values at the safepoint.
`constId` indexes the MIR constant pool for the owning function.

### Stack Map Section

```ds
type StackMapEntry = {
    pc: uint64,
    rootCount: uint16,
    rootOffset: uint32,
};

type RootLocEntry = {
    kind: uint8,
    regIndex: uint16,
    stackIndex: uint32,
    stackOffset: int32,
};

const ROOTLOC_REG: uint8 = 1;
const ROOTLOC_STACK: uint8 = 2;
```

Stack maps identify managed references at the given `pc`.
Roots are interpreted using MIR types recorded for the corresponding frame.

### OSR Entry Section

```ds
type OsrEntry = {
    functionId: uint32,
    blockId: uint32,
    valueCount: uint16,
    valueOffset: uint32,
    localCount: uint16,
    localOffset: uint32,
};
```

OSR entries materialize live values and locals required for the target block.

### Profiling Site Section

```ds
type ProfileSiteEntry = {
    kind: uint8,
    functionId: uint32,
    blockId: uint32,
    instructionId: uint32,
    flags: uint32,
};

const PROFILE_CALL: uint8 = 1;
const PROFILE_BACKEDGE: uint8 = 2;
const PROFILE_ALLOC: uint8 = 3;
const PROFILE_BRANCH: uint8 = 4;
const PROFILE_GUARD: uint8 = 5;
const PROFILE_INDIRECT_CALL: uint8 = 6;
```

Profiling site interpretation is defined by the compiler and runtime together.

### Write Barriers

Lower must preserve managed write sites (`field.set`, `element.set`, stores through
managed references) so the runtime can attach GC barriers in VM and native code.

### Transition Policy

VM/native transitions are policy-controlled and must be consistent across the runtime,
lowered metadata, and optimizer.

**Debug execution mode** is configured via `debugMode`:

| Variant | Behavior |
|---------|----------|
| `Auto` | Use `Deopt` for debug builds, `Native` for release |
| `Vm` | Force interpreter execution |
| `Deopt` | Run native with deopt-first debugging |
| `Native` | Run native only (no deopt) |

**Deopt fidelity** must match the debug mode:
- `Vm`: no native deopt metadata required
- `Deopt`: full reconstruction of live values and locals at every safepoint
- `Native`: minimal metadata is allowed (for crash reporting only)

**Inline frame encoding** is required when `debugMode = Deopt`.
Inline frames are optional when `debugMode = Native` and `debugInfo != Full`.

**OSR policy** follows `osrMode` (default: loop headers).
Lower may disable OSR or restrict OSR to explicit sites via target profile.
Each OSR entry must define the live value set and phi materialization at the entry block.

**Safepoint policy** follows `safepointMode` (default: calls, allocations, loop back-edges).
Targets may add instruction-budget safepoints via `safepointInterval`.

**Speculation and guards** follow `speculationMode` and are permitted when every guard has:
- a side-effect-free fast path
- a deopt map to reconstruct the pre-guard state
- a runtime-visible reason for deopt (for debugging and profiling)

**Profiling** follows `profilingMode` and is scoped per isolate.
Lower emits profiling sites and inline cache anchors when enabled.

**Non-replayable regions** must not allow deopt across them.
Lower marks these regions and forbids guards from spanning:
- FFI calls and syscalls
- I/O operations
- atomic operations with observable ordering
- runtime callbacks into user code

**Stack map precision** is exact at all safepoints.
Conservative scanning is not permitted for normal execution.

**Value materialization** rules:
- missing values are only allowed for dead SSA values at the safepoint
- locals and live SSA values must be materialized in `Deopt` mode

**Metadata versioning** is compiler-version private.
Lower emits versioned sections; the runtime must reject mismatched versions.
Incremental caches key off the metadata version and target policy hash.

## Semantic Guarantees

Lower preserves JavaScript/TypeScript semantics unless explicitly noted.
These guarantees are fundamental to TS compatibility and cannot be changed without breaking user code.

### String Equality

`===` on strings is **value equality** (content comparison), not reference equality.
String interning is an optimization detail with no semantic guarantees.

```ds
const a = "hello";
const b = "hel" + "lo";
a === b  // true (same content)
```

### Symbol Identity

- `Symbol("desc")` creates a **unique** symbol each call
- `Symbol.for("key")` returns the **same** symbol for a given key (global registry)
- `===` compares symbol identity (interned integer comparison)

```ds
Symbol("a") === Symbol("a")        // false (unique each call)
Symbol.for("a") === Symbol.for("a") // true (same from registry)
```

**Registry scope:** The `Symbol.for()` registry is global across all modules in a compilation unit.
If module A calls `Symbol.for("key")` and module B calls `Symbol.for("key")`, they get the same symbol.
This matches JavaScript's global symbol registry behavior.

### Property Enumeration

Object properties enumerate in **insertion order**, matching ES2015+.
This applies to `Object.keys()`, `for...in`, and RTTI field iteration.

### Default Arguments

Default argument expressions evaluate **per-call** when the argument is `undefined`.
Evaluation occurs in the function's scope, not at definition time.

```ds
function log(timestamp = Date.now()) { ... }
log()  // evaluates Date.now() on this call
log()  // evaluates Date.now() again (different value)
```

## Native Runtime Library

Lower doesn't special-case types like `String` or `Array<T>`.
These are defined in `language/builtin/lib/native/` as regular Destack structs (with some intrinsics).
Lower treats them basically like any other user-defined type.
The native and JS builtins live here:

<pre>
language/builtin/
├── core/       # Operator interfaces (Add, Index, etc.) - compiler desugars to these
├── std/        # Universal extensions
└── lib/
    ├── native/ # Native target types: String, Array, Map, etc.
    ├── es/     # JS target types (uses JS-defined built-ins)
    └── ...
</pre>

---

# Phases

Lower executes in four explicit phases.
Each phase produces complete, self-contained output before the next phase begins.
This ensures clear invariants and enables forward references.

## Phase 1: Types

Lower all type declarations to MIR types with computed layouts.

### Input
- DIR type declarations (structs, classes, enums, newtypes, interfaces)
- Target layout parameters (pointer size, alignment rules)

### Output
- MIR `Type` definitions with field offsets and sizes
- Layout cache mapping DIR types to MIR types
- Lineage information for class inheritance chains
- VTable slot assignments for virtual methods

### Operations

**Monomorphization:** Each generic instantiation becomes a concrete MIR type.
`Array<int>` and `Array<string>` produce separate MIR struct types with different layouts.

**Layout computation:** Fields are ordered by alignment (largest first) to minimize padding.
The default layout minimizes size; `@layout("C")` matches C ABI; `@layout("source")` preserves declaration order.

**Lineage computation:** For classes with `extends`, compute the inheritance chain.
Parent fields come first, ensuring pointer compatibility for upcasts.

**VTable slot assignment:** For polymorphic classes, assign vtable slots in declaration order.
Child classes inherit parent slots; overrides reuse the same slot.

### Invariants After Phase 1
- Every DIR type maps to exactly one MIR type
- All MIR types have known size and alignment
- All class lineages are computed
- All vtable slots are assigned (but vtables not yet generated)

### MIR Metadata Emission
Lower populates MIR metadata tables incrementally as phases complete.
Phase 1 records type layouts and lineages in `NodeTree.type_table.type_metadata_by_id`.
Phase 2 records function memory effects and pointer attributes on MIR `Function`.
Phase 3 registers dispatch tables and type descriptors in `NodeTree.type_table.dispatch_tables` and type metadata.
Phase 4 records callsite metadata in `NodeTree.call_table`, memory access metadata (including alias
scopes and TBAA tags) in `NodeTree.memory_table`, and debug scopes in `NodeTree.debug_info`.

MIR metadata structures live in `language/mir/src/metadata/` and define the canonical expectations
for layout, dispatch, and memory semantics. Lower must emit metadata that matches those invariants:
- `TypeLayout.field_offsets` length matches the field or element count
- `DispatchSlot` order follows the vtable/itab slot rules defined below
- Callsite `CallMetadata` uses `CallDispatchKind` and provides `receiver` when required
- Memory access metadata records sizes, alignment, volatility, ordering, and alias scopes when known

## Phase 2: Declarations

Create function signatures and global variable bindings.

### Input
- DIR function and method declarations
- DIR global variable declarations
- MIR types from Phase 1

### Output
- Function shells (signature only, no body)
- Global variable bindings
- Module initialization order

### Operations

**Function signatures:** Create MIR function declarations with parameter types and return type.
Bodies are not lowered yet; this creates "shells" that can be referenced.

**Global variables:** Lower global `const` and `let` declarations to MIR globals.
Compute initialization order based on dependencies.

**Module initialization:** Generate `__init` function per module containing:
- Static global initializers (in dependency order)
- Top-level `using` statements
- Module-level side effects

Initialization order follows import dependencies.
Circular dependencies are a compile error (detected earlier in Analyze).

### Invariants After Phase 2
- All functions have MIR declarations (callable by reference)
- All globals have MIR bindings
- Module initialization order is determined

## Phase 3: Tables

Generate dispatch tables and runtime type information.

### Input
- MIR types with layouts and lineages
- VTable slot assignments
- Interface implementations

### Output
- VTable constants for polymorphic classes
- ITab constants for (Type, Interface) pairs
- TypeDescriptor constants for RTTI
- Interned string tag tables

### Operations

**VTable generation:** For each polymorphic class, emit a constant vtable:
```ds
struct VTable {
    typeDescriptor: &TypeDescriptor  // slot 0, for instanceof/T.is
    destructor: fn()                 // slot 1, drop glue
    methods: [fn; N]                 // virtual methods in slot order
}
```

**ITab generation:** For each (Type, Interface) pair where the type implements the interface:
```ds
struct ITab {
    typeDescriptor: &TypeDescriptor  // for T.is on interface refs
    methods: [fn; M]                 // interface methods in declaration order
}
```

**TypeDescriptor generation:** For types that need RTTI (used with `instanceof`, `T.is`, `typeOf`, stored in `unknown`):
```ds
struct TypeDescriptor {
    id: uint32                  // index into RTTI table
    typeIdOffset: uint32        // offset to TypeId string
    nameOffset: uint32          // offset to name string
    size: uint32                // sizeof in bytes
    alignment: uint16           // alignof in bytes
    kind: uint8                 // struct/class/enum/etc.
    flags: uint8                // nominal, sealed, etc.
    ...
}
```

**String tag interning:** TypeScript-style discriminated unions use string tags.
Lower interns these to integer discriminants:
```ds
"loading" → 0
"success" → 1
"error"   → 2
```

### Invariants After Phase 3
- All vtables are generated as global constants
- All itabs are generated as global constants
- All needed TypeDescriptors are generated
- String tags are interned to integers

## Phase 4: Emit

Lower function bodies to MIR blocks.

### Input
- DIR function bodies (expressions, statements)
- All context from Phases 1-3

### Output
- Complete MIR functions with blocks and terminators
- Ownership markers on `^T` values (drops inserted by Optimize)
- GC write barrier insertions
- Debug info metadata

### Operations

**Block generation:** Each control flow construct becomes MIR blocks:
- `if/else` → conditional branch to then/else blocks
- `match` → switch or branch cascade
- `while/for/loop` → header, body, and exit blocks

**Value lowering:** Expressions become MIR instructions:
- Literals → `const`
- Binary ops → `binary` or builtin intrinsics
- Function calls → `call` (direct) or `call.indirect` (virtual)
- Field access → `field.get`/`field.addr`

**Ownership marking:** For `^T` owned values, mark ownership in MIR.
Optimize's `drop-insert` pass later inserts actual drops at last use points.

**GC barrier insertion:** For writes to managed reference fields:
```mir
intrinsic.gc.write_barrier(field_addr, new_value)
store field_addr, new_value
```

**Terminator generation:** Each block ends with a terminator:
- `return` for function exit
- `jump` for unconditional branch
- `branch` for conditional
- `switch` for multi-way
- `yield` for coroutines
- `unreachable` for dead code

### Invariants After Phase 4
- All functions have complete bodies
- All ownership is marked (drops inserted by Optimize)
- All GC barriers are inserted
- MIR is ready for Optimize's verification and transformation passes

---

# Data Model

This section defines the semantic model for lowering: how high-level concepts map to low-level representations.

## Lowering Model

### Monomorphization

DIR has Instance-level information from Analysis, but is still polymorphic.
For native targets, we fully monomorphize these Instances into concrete types with known sizes and offsets.
(Thus, each generic instantiation gets its own specialized MIR logic.)

```ds
function identity<T>(x: T): T { x }

identity<int32>(5)      // generates: identity_int32
identity<string>("hi")  // generates: identity_string
```

Monomorphization trades code size for runtime performance:

| Aspect | Benefit | Cost |
|--------|---------|------|
| No boxing | Zero overhead generics | More code copies |
| Known offsets | Direct field access | Larger binary |
| Inlining | Cross-generic optimization | Longer compile times |
| Register allocation | Optimal per-instantiation | More codegen work |

The DIR `Instance` type tracks generic instantiations with their static arguments.
Lower receives these `Instance`s and generates specialized MIR for each instantiation.
(For JS/TS codegen, we preserve polymorphic code with no monomorphization needed, except for
non-erasable value parameters like `const N: int`.)

### Name Mangling

Monomorphized functions need unique, deterministic names for linking.
We use a human-readable scheme (inspired by Rust and Zig):

**Format:** `@<module_path>.<type>.<method>__<type_args>__h<hash>`

**Examples:**
```mir
@std.collections.Map.get__string__i32__h7f9a3e1     // Map<string, i32>.get()
@myapp.models.User.getName__h1b2c3d4                // User.getName()
@myapp.utils.identity__Point__h2c3d4e5              // identity<Point>()
@core.ops.Add.add__Vec2__Vec2__h3d4e5f6             // Vec2's Add<Vec2>.add()
```

**Rules:**
- `@` prefix (like Zig/MIR convention)
- `.` separates module/type/method segments (valid MIR identifier char)
- `__` separates type arguments
- Module path included for uniqueness
- Type arguments appended for monomorphized generics
- Hash suffix `__h...` for exported/linkable symbols (matches Rust/Zig practice)

Examples omit the hash suffix for internal-only symbols to keep snippets readable.
This produces predictable, readable symbol names in debuggers and error messages.

### Resolution

Resolution tells Lower *which symbol* is being called (or might be called in dynamic resolution) at a call site.
This is determined by Analyze and attached to DIR nodes.
- **Resolution** answers: "Which declaration are we targeting?"
- **Dispatch** answers: "How do we invoke that target at runtime?"

Even with `Resolution::Static`, the target may require vtable dispatch if it's a virtual method on a polymorphic type.
Dynamic resolutions are reified into if-else chains with `is` type checks in the Elaborate phase; i.e., the Lower phase only sees `Resolution::Static` and `Resolution::Builtin`.
(`Resolution::Dynamic` is specifically for *union symbols* where different union variants call different target symbols; but, remember, symbols are polymorphic in DIR so this may be different MIR symbols even for the same DIR symbols).

#### Builtin Resolution

Primitive operations on builtin types.
No function call needed; Lower emits MIR instructions directly.

```ds
a + b  // builtin resolution -> int32 + int32
```

Lowers to:
```mir
v2 = iadd v0, v1
```

#### Static Resolution

Single known target symbol.
The *symbol* is known, but dispatch may still be virtual or direct depending on the method.

**Direct dispatch** (function, non-virtual method, final class, struct method):
```ds
user.getName()  // static resolution, non-virtual
```

Lowers to:
```mir
v1 = call @User.getName(v0)
```

**Virtual dispatch** (virtual method on class/interface):
```ds
node.update(delta)  // static resolution { target: Node::update }, but virtual
```

Lowers to vtable lookup:
```mir
v1 = field.get v0, 0           ; load vtable pointer from object layout
v2 = field.get v1, 2           ; load update method at vtable slot 2
v3 = call.indirect v2(v0, delta) ; indirect call through vtable
```

The key: static resolution means we know *which method signature* (Node::update), but if it's virtual, the actual implementation depends on the concrete type.

#### Dynamic Resolution

Union-based dispatch: different union symbols call different target symbols.
Elaborate has already generated the symbol-dispatch logic; Lower just emits it.

**Union method dispatch:**
```ds
function render(obj: Mesh | Light) {
    // different implementations for Mesh.draw and Light.draw
    obj.draw(ctx);
}
```

Elaborate transforms to a type guard chain (conceptually `instanceof`/`T.is`), Lower then emits:
```mir
type @RenderContext = struct { ... }
type @ObjectWithVTable = struct { ref<raw void> }

function @render(v0: ref<@ObjectWithVTable>, ctx: ref<@RenderContext>) -> void {
block0(v0: ref<@ObjectWithVTable>, ctx: ref<@RenderContext>):
    v1 = field.get v0, 0       ; vtable pointer
    v2 = field.get v1, 0       ; type descriptor from vtable slot 0
    v3 = global.const @Mesh_TypeDescriptor
    v4 = icmp_eq v2, v3
    branch v4, block1, block2
block1:
    call @Mesh.draw(v0, ctx)
    jump block4
block2:
    v5 = global.const @Light_TypeDescriptor
    v6 = icmp_eq v2, v5
    branch v6, block3, block5
block3:
    call @Light.draw(v0, ctx)
    jump block4
block5:
    unreachable              ; exhaustive match
block4:
    return
}
```

In the `Mesh | Light` case we could also have used a `Drawable` interface or `Object3D` base type, and then this would be solved with vtable dispatch instead of dynamic resolution.

## Type Representation

All layouts here are post-monomorphization and target-specific.

### Primitives

| DIR Type | MIR Type | Native Representation |
|----------|----------|----------------------|
| `boolean` | `Boolean` | `i8` (0 or 1) |
| `int8..int64` | `Int { width, signed: true }` | Native integer |
| `uint8..uint64` | `Int { width, signed: false }` | Native integer |
| `float32`, `float64` | `Float { width }` | IEEE 754 float |
| `void` | `Void` | Zero-sized |

#### Integer Semantics

| Type | Width | Notes |
|------|-------|-------|
| `int` | 64-bit signed | Alias for `int64`, fixed across all platforms |
| `uint` | 64-bit unsigned | Alias for `uint64`, fixed across all platforms |
| `int32`, `uint32`, etc. | As named | Explicit width types |

**Rationale:** Fixed 64-bit default ensures predictable overflow behavior across platforms.
32-bit platforms are rare (<1% of modern targets).

### Overflow Behavior

Integer arithmetic has defined overflow semantics (unlike C, inspired by Zig and Rust).
Default overflow behavior is configurable per target in build configuration (`overflowChecks`) or via `safetyPreset`.
Explicit `overflowChecks` overrides any `safetyPreset` value.

When overflow checks are enabled, the lowerer emits explicit overflow checks for `+`, `-`, and `*` and traps on overflow.
When overflow checks are disabled, arithmetic wraps in two's complement.
Signed division and remainder trap on division by zero and on `min_value / -1`.
When safety checks are disabled, the lowerer emits `div.unchecked` and `rem.unchecked` intrinsics.

**Explicit operators:** Always available regardless of mode.
- `+%`, `-%`, `*%`: wrapping (two's complement wrap)
- `+|`, `-|`, `*|`: saturating (clamp to min/max)

```ds
const a: uint8 = 250;
const b: uint8 = 10;

a + b       // checked when overflowChecks enables safety
a +% b      // always wrapping: 250 + 10 = 4
a +| b      // always saturating: 250 + 10 = 255
```

MIR representation:
- `Binary` uses wrapping semantics for integer add, sub, and mul
- overflow checks use `add.overflow` and related intrinsics plus `check`
- unchecked operations use `*.unchecked` intrinsics

### Null and Undefined

TypeScript has both `null` and `undefined`. For native targets, both use the same runtime representation:
- `null`: Zero/null pointer (0x0)
- `undefined`: Distinguished sentinel value (0x1)

The distinction between `null` and `undefined` exists only at the type system level.
At runtime, code that needs to distinguish them (rare) uses the sentinel values.
Most code treats them equivalently: "no value present."

For pointer types, we use **niche optimization** (like Rust's `Option<&T>`).
The null pointer (0x0) is an invalid address for valid objects, so we can use it
as the "none" discriminant without adding a tag byte:

```ds
type MaybeRef<T> = T | null  // T is a reference type
// layout: same size as T, null = 0x0
```

This is the same optimization Rust uses for `Option<Box<T>>`, `Option<&T>`, etc.
The "niche" is the invalid bit pattern (null pointer) that we repurpose as a discriminant.

For value types, there's no invalid bit pattern to exploit, so we need a tag:
```ds
type MaybeInt = int | null
// layout: { tag: u8, value: int }  // 2 bytes overhead minimum
```

**Niche optimizations:**

Like Rust, we can exploit invalid bit patterns to save space in type layouts:
- `boolean | null` → use value 2 for null (bool only uses 0 and 1)
- `character | null` → use invalid Unicode scalar values
- Enums with < 256 variants → use unused discriminant values

### BigInt

BigInt (`bigint` type, `42n` literals) is a **library type with operator overloading**.
It is not a compiler intrinsic; it's defined in `language/builtin/lib/native/` and uses the standard operator interfaces (`Add`, `Subtract`, `Multiply`, etc.).

For native, we use a tagged pointer representation with small-integer optimization.
(This is similar to what many JS runtimes do internally as well.)

| Case | Layout | Notes |
| --- | --- | --- |
| Small bigint (fits in 63 bits) | `[63-bit value][1-bit tag=1]` | inline, no heap allocation |
| Large bigint (> 63 bits) | `[pointer to limb array][1-bit tag=0]` | heap-allocated, arbitrary precision |

```ds
const small: bigint = 42n          // inline: 0x0000000000000055 (42 << 1 | 1)
const large: bigint = 2n ** 100n   // heap: pointer to limb array
```

The native `bigint` has TypeScript semantics:
- `===` compares values, not identity (bigints are value types semantically, same as JS/TS)
- Small bigints compare inline; large bigints compare limb-by-limb
- Overflow from small to large is automatic and transparent (same layout size)

**Operators:**
BigInt implements `Add<bigint>`, `Subtract<bigint>`, `Multiply<bigint>`, etc.
Operators lower to method calls: `a + b` → `a.add(b)`.

**Mixed operations:**
`bigint + int` requires explicit conversion. No implicit coercion between bigint and other numeric types.

**Division:**
`/` returns `bigint` (truncated). Use library methods for remainder, divmod.

### Symbol

TypeScript's `symbol` is a unique identifier.
For native, symbols are interned integers:

```ds
const symbol = Symbol("description")
```

MIR representation:
- `symbolId: uint64` (unique per-symbol, assigned at creation)
- Description string stored separately in symbol table

Symbol comparison is just integer comparison.
`Symbol.for()` looks up in a global string-to-symbol map.

### Strings

Strings are immutable byte sequences with target-specific payload encoding.
Native targets use UTF-8 payloads by default.
WASM and JS-interop targets use UTF-16 payloads to avoid boundary transcoding.

**Ownership semantics** (like Rust):

| Destack | Rust | Description |
|---------|-----------------|-------------|
| `string` | ~`Rc<str>` | GC-managed immutable string (default) |
| `^string` | `String` | Owned mutable string (manual/RAII) |
| `&string` | `&str` | Borrowed immutable view |

Most code uses `string` (GC-managed). Use `^string` for performance-critical code with explicit ownership.

#### String Layout

The header layout is identical across targets, and only the payload encoding changes.
UTF-8 payloads use `uint8` data and UTF-16 payloads use `uint16` data.

```ds
struct string {
    lengthUtf16: uint32,      // UTF-16 code unit count (for TS compatibility)
    lengthBytes: uint32,      // byte length of UTF-8 data
    hash: uint64,             // cached hash (valid when HasHash is set)
    capacity: uint32,         // allocated capacity in bytes
    flags: uint32,            // runtime metadata flags
    data: *uint8,             // UTF-8 bytes
}
```

GC metadata and type descriptors are stored out of line (see [Managed Object Metadata](#managed-object-metadata)).
For TypeScript semantic compatibility:
- `.length` returns UTF-16 code unit count (not bytes, not codepoints)
- `.charAt(i)` indexes by UTF-16 code units
- ASCII-only strings (common case) use O(1) indexing
On UTF-16 targets, `lengthBytes` caches the UTF-8 byte length and is computed lazily.
The payload encoding is fixed per target configuration.
The `string` payload pointer type is `*uint8` on UTF-8 targets and `*uint16` on UTF-16 targets.
The `string` header uses `capacity` for owned `^string` growth and usually keeps `capacity == lengthBytes` for GC-managed `string`.
Flags are runtime metadata bits with stable meanings.
- `HasHash`: `hash` is populated and valid
- `IsAscii`: payload is ASCII-only
- `IsStatic`: payload is static read only data
- `IsInterned`: string content is interned
- `IsExternal`: payload is owned outside the managed heap

#### String Literals

String literals are interned at compile time in a read only data section:

```ds
const s = "hello"   // pointer to static data (never collected)
```

**Interning scope:** String literals are globally deduplicated within a compilation unit.
The same literal `"hello"` appearing in multiple modules resolves to the same address.
However, runtime-created strings are NOT guaranteed to be interned.
`"hel" + "lo"` at runtime creates a new string, even if `"hello"` exists as a literal.
String equality (`===`) is always value comparison, never reference comparison.

#### Template Literals

```ds
`Hello, ${name}!`
```

Lowers to string concatenation:
```mir
type @string = ref<struct { u32, u32, u64, u32, u32, *u8 }>

function @template_example(v0: @string) -> @string {
block0(v0: @string):
    v1 = global.const @str_Hello
    v2 = call @string.concat(v1, v0)
    v3 = global.const @str_Bang
    v4 = call @string.concat(v2, v3)
    return v4
}
```

### Arrays

Arrays are heap-allocated, dynamically-sized collections (like Rust's `Vec<T>`).

**Fixed-size arrays** `T[N]` are inline values with no header.
**Dynamic arrays** `T[]` are heap-allocated, growable buffers.

| Array kind | Layout | Notes |
| --- | --- | --- |
| `T[N]` | inline `elements: [T; N]` | size = `N * sizeof(T)` |
| `T[]` | header `{ length: uint32, capacity: uint32, data: T[capacity] }` | heap allocated, growable |

| Pattern | MIR Type | Notes |
|---------|----------|-------|
| `T[N]` | `Type::Array { element, length: N }` | Inline, value semantics |
| `T[]` | `ref<managed @Array<T>>` | Heap, reference semantics |
| `TypedArray` | `Type::Reference(kind: Raw)` to buffer | Direct memory access |

**Important:** `T[]` is NOT `Array<unknown>`. After monomorphization, we know T.
`Array<unknown>` boxes elements and uses RTTI for type checks.

**Array operations:**
```mir
; a[i] where a: int[], a is v0, i is v1
v2 = field.get v0, 2        ; get data pointer
v3 = element.get v2, v1     ; load element at index

; a.push(x) where x is v4
call @Array.push(v0, v4)

; a.length
v5 = field.get v0, 1        ; load length field
```

**Bounds checks:**
Array and slice indexing emits bounds checks by default.
The policy is configured per target (`boundsChecks` in `dsconfig.json` or via `safetyPreset`):
- `always`: checks in all builds
- `debug`: checks only when `target.debug` is true (default)
- `never`: no checks (unsafe, fastest)
Explicit `boundsChecks` overrides any `safetyPreset` value.

**Null checks:**
Reference operations emit null checks when required.
The policy is configured per target (`nullChecks` or `safetyPreset`):
- `always`: checks in all builds
- `debug`: checks only when `target.debug` is true (default)
- `never`: no checks (unsafe, fastest)
Explicit `nullChecks` overrides any `safetyPreset` value.

**Division checks:**
Division and remainder emit checks for division by zero (and signed `min_value / -1`) when enabled.
The policy is configured per target (`divisionChecks` or `safetyPreset`):
- `always`: checks in all builds
- `debug`: checks only when `target.debug` is true (default)
- `never`: no checks (unsafe, fastest)
Explicit `divisionChecks` overrides any `safetyPreset` value.

**Shift range checks:**
Shift operations can emit range checks when enabled.
The policy is configured per target (`shiftChecks` or `safetyPreset`):
- `always`: checks in all builds
- `debug`: checks only when `target.debug` is true (default)
- `never`: no checks (unsafe, fastest)
Explicit `shiftChecks` overrides any `safetyPreset` value.

**Check failure behavior:**
When a check fails, the compiler can trap, panic, or abort.
The policy is configured per target (`checkFailure`):
- `trap`: emit a trap/unreachable
- `panic`: call the panic runtime (uses `panic`/`unwind` policy)
- `abort`: abort immediately
The safety preset does not modify `checkFailure`.

**Slices:**
Slices are explicit view types with a pointer and length (`Slice<T>`).
They are distinct from borrowing the array object itself.
Functions that accept a slice can take `Slice<T>` or `&Slice<T>`.
Borrowing an array object does not imply a slice view.

### Structs and Classes

Structs are value types with no identity, while classes are reference types with identity.
Structural object types are reference types even when written as type aliases.
Ownership modifiers (`^T`, `&T`) describe ownership and borrowing without changing identity semantics.

| Aspect | struct | class |
|--------|--------|-------|
| Reference identity | No (`===` is compile error) | Yes (`===` compares pointers) |
| Type identity | Via metadata or fat pointer when needed | Via vtable or metadata when needed |
| Equality | By value (`==` compares properties) | By reference (unless `Equal` implemented) |
| Extends | No | Yes |
| Implements | Yes | Yes |
| Virtual | None (all calls static) | Methods virtual by default |
| Default passing | Value | Reference |
| Default storage | Inline | Managed reference |

**Virtual method dispatch for classes:**
- All class methods are virtual by default (like TypeScript/JavaScript prototype methods)
- Private methods (`#method`) use direct dispatch (not inheritable)
- **Devirtualization**: The optimizer analyzes the class hierarchy and converts virtual calls to direct calls when safe:
  - No subclasses exist in the compilation unit → direct call
  - Method not overridden by any subclass → direct call
  - Receiver type is exactly known (not a supertype) → direct call
- Vtables are only emitted when dynamic dispatch remains, fully devirtualized classes can omit vtables

The `final` keyword on methods or classes is an API contract ("you may not override/extend"), not an optimization hint.
For whole-program compilation, the optimizer already knows what's overridden.
`final` matters for libraries where downstream users could extend classes.

Struct layouts are value payloads with computed property offsets.
Class instance types are managed references to payload layouts, and the payload includes a vtable pointer when dynamic dispatch remains.
Both can carry type identity metadata for `instanceof`, `T.is`, or `typeOf` when needed.

#### RTTI and Type Tags

RTTI (runtime type identity) is unified via `TypeDescriptor` pointers.
Polymorphic classes include a vtable pointer in the payload layout when virtual dispatch remains.
Vtable slot 0 points at the `TypeDescriptor` for fast `instanceof`, `T.is`, and `typeOf`.
Structs remain headerless and never store a vtable pointer.
Thin-pointer checks on structs recover `TypeDescriptor` from GC metadata when needed.
Interface and `unknown` values carry `TypeDescriptor` in fat pointers.
Class references are thin pointers, so the vtable pointer must live in the object layout when present.

GC metadata lookup only applies to managed references.
Non-managed values require explicit tags (union tags or fat pointers) or compile-time type knowledge.

RTTI is only emitted when runtime type checks are possible:
- Used with `instanceof`, `T.is`, or `typeOf` on unknown values
- Stored in `unknown`
- Used in runtime reflection
- Used in untagged unions that require runtime discrimination

**JS targets:** RTTI-enabled structs/classes emit a non-enumerable symbol property
with their `TypeId` during construction. This keeps objects "plain" for JS semantics
while enabling `instanceof`, `T.is`, and `typeOf` without a global WeakMap.

#### Struct Layout

Structs have no **reference identity** (no `===`).
Structs are always headerless and use metadata or fat pointers for RTTI.

```ds
struct Point { x: float32, y: float32 }

struct PointLayout {
    x: float32,              // offset 0
    y: float32,              // offset 4
}
// size: 8 bytes
```

Structs are data-oriented: two structs with the same properties are equal by value (`==`) by default.
(Reference comparison (`===`) on structs is a compile error.)

**Prefer discriminated unions** for performance-critical code to avoid runtime RTTI lookups:

```ds
struct Circle { kind: "circle" = "circle", radius: float };
struct Rect { kind: "rect" = "rect", width: float, height: float };
type Shape = Circle | Rect;

function area(shape: Shape): float {
    match (shape.kind) {
        "circle" => 3.14159 * shape.radius * shape.radius
        "rect" => shape.width * shape.height
    }
}
```

String tags are interned to integers (see [String Tag Interning](#string-tag-interning)).

#### Class Layout

Polymorphic classes have vtable pointers for virtual dispatch and RTTI:

```ds
class Node {
    name: string;
    update(delta: float): void { }
}
```

Native layout:
```ds
struct NodeLayout {
    vtablePtr: &VTable,      // offset 0, vtable[0] = &Node_TypeDescriptor
    name: ref<string>,       // offset 8
}
```

Classes have both reference identity (`===` compares pointers) and type identity (via vtable or metadata).
Polymorphic classes store their vtable pointer because class references are thin pointers and dynamic dispatch is required.
(This is consistent with Java and C++ class objects while keeping struct layouts headerless like Go.)
Non-polymorphic classes omit the vtable pointer and use metadata or fat pointers for RTTI when needed.

Class layouts are static on native targets.
There are no hidden classes or runtime shape transitions.
Dynamic property addition must use explicit map/dictionary types.

#### Managed Object Metadata

Managed objects have no per object GC header.
GC metadata is stored out of line in allocator side tables, similar to Go.
Type tags are pointers to TypeDescriptor values, not integer ids.
Null and undefined use niche optimization in the pointer (e.g., 0x0 for null, 0x1 for undefined).

Per span metadata includes:
- Mark bits for GC tracing
- Size class and allocation layout info
- A TypeDescriptor pointer per object for scanning and type queries

Polymorphic classes store a vtable pointer in the object for virtual dispatch and fast `instanceof`/`T.is`.
Structs remain headerless and rely on metadata or fat pointers for RTTI.

Tradeoffs:
- Predictable object layouts and smaller per object overhead
- Thin pointer RTTI queries require a metadata lookup

**Explicit ownership:**
Use `^T` to require a single owner and enable drop semantics:
```ds
function process(point: ^Point) {    // ownership transferred into the callee
    // modifications don't affect the caller
}
```

#### Field Layout Policy

Field layout is deterministic per target and part of the ABI.
The default layout minimizes padding by sorting fields by alignment and size.
Source order is the stable tie-break when alignment and size are equal.
Class layouts place parent fields first, then apply the same policy to new fields.
Use `@layout("C")` to match the C ABI for FFI.
Use `@layout("source")` to preserve declared order.

### Class Inheritance

Classes with `extends` get special handling for field layout and method dispatch.
Parent fields come first, then child fields.

```ds
class Node {
    name: string
}

class Sprite extends Node {
    texture: Texture
}
```

Sprite's MIR layout (polymorphic):
- offset 0: vtablePtr
- offset 8: name (from Node)
- offset 16: texture (from Sprite)

This ensures a `Sprite` pointer can be used where a `Node` pointer is expected.
Polymorphic classes use vtables for virtual methods (and non-virtual methods are direct calls).

```ds
class Node {
    update(delta: float): void { }
}

class Sprite extends Node {
    update(delta: float): void { this.animate(delta) }
}
```

### Tuples

Tuples lower to anonymous `Type::Struct` with indexed fields.

```ds
(int, string, bool)  →  Struct { fields: [i64, String, i8] }
```

### Associated Types

Associated types (like `Container<T>.Item`) are resolved at compile time during the Elaborate phase.
By the time Lower runs, all associated types have been replaced with their concrete types.
Lower never sees associated type references; it only sees the resolved concrete types.
(Sort of like we deal with Resolution::Dynamic.)

```ds
interface Container<T> {
    type Item = T
}

// In DIR after Elaborate:
// Container<int>.Item is already resolved to int
```

### Newtypes

Newtypes are fully erased at the MIR level.
They exist only for type checking; the runtime representation is identical to the wrapped type.

```ds
newtype UserId = int;
const id: UserId = UserId(42);  // lowers to: const id: int = 42
```

Pattern matching on newtypes extracts the inner value with no runtime cost:
```ds
match (id) {
    UserId(n) => print(n)  // lowers to: print(id)
}
```

## Union Types

Destack supports TypeScript's structural unions.

### Discriminated Unions (Tagged)

When all variants share a discriminant field (e.g., `kind`), use inline tagging:

```ds
type Result<T, E> = { kind: "ok", value: T } | { kind: "err", error: E }
```

Lowers to a MIR struct with:
- `tag: uint8` (0 = ok, 1 = err)
- `payload: uint8[N]` (max of sizeof(T), sizeof(E))

The `tag` field is the discriminant.
Pattern matching becomes a switch on tag.

#### String Tag Interning

TypeScript-style discriminated unions typically use string literals as tags:
```ds
type LoadState<T> =
    | { kind: "loading" }
    | { kind: "success", data: T }
    | { kind: "error", msg: string }
```

This is a very common pattern in TypeScript, and we can optimize it nicely for native targets.
We intern these string tags to integer discriminants at compile time; conceptually this looks like this:

```ds
// tag mapping (compile time)
const TAG_LOADING: uint8 = 0   // "loading"
const TAG_SUCCESS: uint8 = 1   // "success"
const TAG_ERROR: uint8 = 2     // "error"

// reverse mapping: runtime string access
const TAG_STRINGS: string[] = ["loading", "success", "error"]

// runtime representation
struct LoadingState { tag: uint8 }
struct SuccessState<T> { tag: uint8, data: T }
struct ErrorState { tag: uint8, msg: string }
```

**Operations:**
- `x.kind === "success"` → `x.tag == TAG_SUCCESS` (fast integer compare)
- `console.log(x.kind)` → `TAG_STRINGS[x.tag]` (string lookup only when needed)

This preserves TS semantics while enabling efficient native dispatch.

**Tag assignment determinism:** Tag integers are assigned deterministically within a compilation unit.
The same union type always gets the same tag assignments in the same compilation.
However, tag values are NOT stable across different compilations or compiler versions.
Code should never serialize or persist tag integers; use the string values for serialization.

#### TypeId Interning

We use the same interning mechanism for type identifiers (`TypeId`).
At the source level, `TypeId` is a string like `"@destack-sh/ui/components/button:Button"`.
At runtime, it's an interned integer for fast comparison:

```ds
// source level API
newtype TypeId = string   // "myapp/models:User"

// interning (compile time)
const TYPEID_USER: uint32 = 42         // interned id for "myapp/models:User"
const TYPEID_ORDER: uint32 = 43        // interned id for "myapp/models:Order"

// reverse mapping: reflection
const TYPEID_STRINGS: string[] = [..., "myapp/models:User", ...]
```

This unifies discriminated union tags and type identifiers under a single
string interning mechanism, reducing complexity and code duplication.

TypeId is a stable string identity for reflection and JS interop.
Native dynamic dispatch does not use TypeId for equality checks.
Native type tags are pointers to TypeDescriptor values.

### Union Representation

Lower chooses union representation based on these rules (in order):

1. **Niche optimization** - All members are nullable references or undefined **and** runtime
   discrimination does not require metadata lookup (or a type tag is already available).
   Examples:

   ```ds
   type MaybeString = string | null
   // layout: same size as string reference, null = 0x0, no tag

   type MaybeStringOrUndefined = string | undefined
   // layout: same size as string reference, undefined = 0x1, no tag

   type MaybeUser = User | null | undefined
   // layout: same size as User reference, null = 0x0, undefined = 0x1, no tag
   ```
   This requires managed references with at least 2-byte alignment.

2. **Inline tagged** - Total size ≤ 2×pointer_size (16 bytes on 64-bit).
   Examples:

   ```ds
   type IntOrBool = int32 | bool
   // layout: { tag: u8, value: int64 }

   type IntOrNull = int32 | null
   // layout: { tag: u8, value: int32 }

   type PointOrLine = Point | Line
   // layout: { tag: u8, data: [u8; max(sizeof)] }
   ```

3. **Boxed** - Large or heterogeneous unions, or when runtime discrimination
   needs RTTI but the variants are not tagged.
   Examples:

   ```ds
   type Dynamic = unknown
   // layout: { typeDescriptor: &TypeDescriptor, payload: word }

   type LargeUnion = LargeA | LargeB
   // layout: { tag: u8, data: pointer to variant }
   ```

The inline size threshold is fixed per target for ABI stability.
**Owned unions** (`^(A | B)`) prefer inline representation when the variant is known at runtime
without additional RTTI. If RTTI is required for drop, the union is boxed with an explicit tag.
The `typeDescriptor` in boxed unions points at the RTTI descriptor.

### Dynamic Types (unknown)

**Native targets do not support `any`.** Only `unknown` is available, requiring explicit type checks via RTTI before use. This matches Rust's approach: dynamic typing requires explicit casts and runtime checks.

```ds
const value: unknown = getUnknownValue();
value.foo()              // ERROR: cannot access property on unknown
if (value is User) {
    value.name           // OK: narrowed to User
}
```

`unknown` uses a fat-pointer layout:

```ds
struct unknown {
    typeDescriptor: &TypeDescriptor
    payload: word
}
```

The `payload` is a pointer-sized word interpreted by `typeDescriptor`.
`word` is a pointer-sized integer type (u64 on 64-bit, u32 on 32-bit).
Managed references store the object pointer in `payload`.
Small primitives store their bitwise representation directly in `payload`.
Large values are boxed into managed memory and referenced by `payload`.

**JS targets:** Both `any` and `unknown` are supported with standard TypeScript semantics.
`any` bypasses type checking; `unknown` requires narrowing. For portable code, prefer `unknown`.

### Reflection

Destack's types-as-values feature makes `Type<T>` a first-class value, enabling
both compile-time and runtime reflection (as needed).

**Source-level API** (from `@destack-sh/core/reflection`):
```ds
Type<T> = StructType<T> | ClassType<T> | EnumType<T> | ...

struct StructType<T> {
    kind: "struct"
    name: string
    id: TypeId              // stable identifier: "@destack-sh/ui/components/button:Button"
    properties: Property[]
    decorators: DecoratorInfo[]
}
```

**Comptime:** Full type information is available as compile-time data.
Type operations execute in the VM interpreter (which runs MIR), so comptime and runtime share the same representation.
This simplifies the design: there's no separate "comptime type format" vs "runtime type format".

```ds
const PROP_COUNT = comptime User.properties.length;    // → literal 3
const HAS_NAME = comptime User.properties.some(p => p.name == "name");  // → true

if (comptime User.properties.some(p => p.type == string)) {
    // branch selected at compile time, other eliminated
}
```

**Runtime:** When type information is needed at runtime, we generate RTTI.
The RTTI table is a static array embedded in the binary.
The native RTTI representation is a compact binary format that maps to the high-level
`Type<T>` API from `language/builtin/core/reflect/type.ds`:

```ds
// high level API
newtype Type<T> = StructType<T> | ClassType<T> | EnumType<T> | ...

struct StructType<T> {
    kind: "struct" = "struct"
    name: string
    id: TypeId                  // "myapp/models:User"
    properties: readonly Property[]
    decorators: readonly DecoratorInfo[]
    description?: string
}

// native RTTI: binary format, used by runtime
struct TypeDescriptor {
    id: uint32                  // index into RTTI table
    typeIdOffset: uint32        // offset to TypeId string ("myapp/models:User")
    nameOffset: uint32          // offset to name string ("User")
    size: uint32                // sizeof(T) in bytes
    alignment: uint16           // alignof(T) in bytes
    kind: uint8                 // maps to Type<T> discriminant
    flags: uint8                // nominal, sealed, etc.
    parentDesc: &TypeDescriptor // for class inheritance (null if none)
    vtablePtr: &VTable          // for virtual dispatch (null if none)
    gcLayoutOffset: uint32      // offset to GC layout bitmap
    gcLayoutWordCount: uint16   // number of words in GC bitmap
    propertiesOffset: uint32    // offset to PropertyDescriptor array
    propertyCount: uint16       // number of properties
    decoratorsOffset: uint32    // offset to DecoratorDescriptor array
    decoratorCount: uint16      // number of decorators
}

struct PropertyDescriptor {
    nameOffset: uint32          // offset into string table
    typeDescriptor: &TypeDescriptor   // TypeDescriptor for property type
    offset: uint32              // byte offset within parent struct
    flags: uint8                // optional, readonly, etc.
}
```

At runtime, when user code accesses `User.properties` or `typeOf(value)`, the `TypeDescriptor`
data is accessed directly.
(Comptime and runtime share the same MIR representation, so no synthesis or conversion step is needed.)
Runtime type tags are pointers to TypeDescriptor values.
When a vtable exists, slot 0 stores the TypeDescriptor pointer. Interface and `unknown` values carry it in fat pointers,
and thin pointers recover it via GC metadata when needed.

**Lowering Type<T> operations:**

| Source | Comptime | Runtime (if needed) |
|--------|----------|---------------------|
| `User` (in type position) | Type check | N/A |
| `User` (in value position) | Constant TypeDescriptor* | Load from RTTI table |
| `User.name` | Constant "User" | `rtti[user_id].name` |
| `User.properties` | Constant array | Load property descriptors |
| `value instanceof User` | Eliminated if type known | Compare `value.typeDescriptor == &User_TypeDescriptor` |
| `User.is(value)` | Eliminated if type known | Compare `value.typeDescriptor == &User_TypeDescriptor` |
| `typeOf(value)` | Constant if type known | Load `value.typeDescriptor`, return descriptor |

When a value is a thin pointer without an embedded type tag, we get the TypeDescriptor pointer from GC metadata for comparison.

**RTTI generation rules:**
RTTI (TypeDescriptor) is only emitted for types that need runtime type checks.
Lower conservatively emits RTTI for any type that might need it:
- Types used with `instanceof` or `T.is` on values of unknown concrete type
- Types used with `typeOf()` on values of unknown concrete type
- Types stored in `unknown` (need RTTI for later extraction)
- Types with runtime reflection (non-comptime `.properties`, `.name`, etc.)
- Types used in untagged unions that require runtime discrimination

Lower does NOT emit RTTI for:
- Types only used with statically-known concrete types
- Types where all `instanceof`/`T.is` checks are eliminated by type narrowing
- Primitives (handled by tag bits, not full TypeDescriptor)
- Newtypes (erased at runtime)

Dead code elimination in the Optimize phase removes unused RTTI entries.
If all type operations resolve at comptime, no RTTI overhead appears in the binary.

## Dispatch

Method calls are resolved and dispatched differently based on structural or nominal types.
TypeScript's duck typing means any object with matching methods can satisfy an interface, which creates some interesting challenges for native codegen.

Polymorphic class dispatch uses vtables stored in the object layout, like C++ and Java.
Interface dispatch uses itabs carried by fat pointers, like Go.
Union dispatch generates type checking code when a value could be multiple types.

### VTable Layout

Types with virtual methods have a vtable.
The vtable is an array of function pointers, one per virtual method.

**VTable structure (conceptual):**
```ds
struct VTable {
    typeDescriptor: &TypeDescriptor    // for instanceof, T.is, and typeOf
    destructor: () => void       // cleanup function
    methods: ((...args: unknown[]) => unknown)[] // virtual method pointers
}
```

**Example vtable layout:**
<pre>
Node vtable:
  slot 0: typeDescriptor = &Node_TypeDescriptor
  slot 1: destructor = Node_drop
  slot 2: update = Node.update

Sprite vtable (inherits Node):
  slot 0: typeDescriptor = &Sprite_TypeDescriptor
  slot 1: destructor = Sprite_drop
  slot 2: update = Sprite.update      // overrides Node::update
</pre>

**Slot assignment (inheritance-preserving):**
- Slot 0: always `typeDescriptor` (for `instanceof`, `T.is`, `typeOf`)
- Slot 1: always `destructor` (drop glue)
- Slots 2+: virtual methods in declaration order
- Child classes inherit all parent slots at the same indices
- Overriding methods reuse the parent's slot index
- New methods append after the last inherited slot

This inheritance-preserving order ensures:
- Upcasting requires no vtable adjustment (same slots, same indices)
- Parent code works on child objects without recompilation
- Binary compatibility when adding methods to leaf classes

**Virtual call lowering:**
```mir
; node.update(delta) where node could be Node or Sprite
v1 = field.get v0, 0           ; load vtable pointer from object
v2 = field.get v1, 2           ; load update method (slot 2)
v3 = call.indirect v2(v0, delta) ; call with self as first arg
```

**Super calls:**
Super calls compile to direct calls to parent implementation.

```ds
class Sprite extends Node {
    update(delta: float): void {
        super.update(delta)  // call Node.update
        this.animate(delta)
    }
}
```

Lowers to:
```mir
type @Sprite = struct { ref<raw void>, ref<string>, ref<Texture> }

function @Sprite.update(v0: ref<@Sprite>, delta: f32) -> void {
block0(v0: ref<@Sprite>, delta: f32):
    call @Node.update(v0, delta)   ; direct call, no vtable lookup
    call @Sprite.animate(v0, delta)
    return
}
```

### Interface Dispatch (ITabs)

Interfaces use itabs, and interface values carry a fat pointer:
```ds
type InterfaceRef<I> = { objectPtr: &Object, itabPtr: &InterfaceItab<I> }
```

Each (Type, Interface) pair has its own itab mapping interface methods to concrete implementations.

#### Structural Interfaces

Structural interfaces require **fat pointers** because the itab layout varies per (Type, Interface) pair:

```ds
interface Drawable { draw(): void }
interface Resizable { resize(w: int, h: int): void }

struct Circle { radius: float }
struct Rectangle { width: float, height: float }
```

When a `Circle` is used as `Drawable`, we create a fat pointer:

```ds
// fat pointer representation
struct InterfaceRef<I> {
    objectPtr: &unknown       // actual object (type erased)
    itabPtr: &InterfaceItab<I>  // interface itab
}
```

**Fat pointer size:** Interface references are exactly `2 * sizeof(usize)` (16 bytes on 64-bit).
The layout is `(objectPtr, itabPtr)` with no padding. This matches Go's interface representation.

Each (Type, Interface) pair generates its own itab:

```ds
// circle as Drawable
const Circle_Drawable_itab: InterfaceItab<Drawable> = {
    typeDescriptor: &Circle_TypeDescriptor,
    draw: @Circle.draw
}

// rectangle as Drawable
const Rectangle_Drawable_itab: InterfaceItab<Drawable> = {
    typeDescriptor: &Rectangle_TypeDescriptor,
    draw: @Rectangle.draw
}
```

**Interface call lowering:**

Each (Type, Interface) pair gets its own itab with slots assigned in interface method declaration order.
Slot 0 is always `typeDescriptor`, then methods follow.
The compiler generates the itab at compile time, and interface references carry a pointer to the appropriate itab.

```ds
function render(d: Drawable) { d.draw(); }
```

Lowers to:
```mir
type @Drawable = struct { ref<raw void>, ref<raw void> }

function @render(v0: ref<raw @Drawable>) -> void {
block0(v0: ref<raw @Drawable>):
    ; v0 is a fat pointer: { objectPtr, itabPtr }
    v1 = field.get v0, 0       ; load objectPtr
    v2 = field.get v0, 1       ; load itabPtr
    v3 = field.get v2, 1       ; load draw method from itab slot 1
    call.indirect v3(v1)       ; call with object as self
    return
}
```

#### Nominal Interfaces

Nominal interfaces (`newtype interface`) use the same fat pointer representation as structural interfaces.
The difference is typing: nominal interfaces require explicit `implements` declarations.
This makes itabs fully known at compile time and avoids runtime method-set checks.

**Multiple interface implementation:** When a type implements multiple interfaces, each (Type, Interface)
pair gets its own itab. Methods are listed in that interface's declaration order.

**Method ambiguity resolution:** If two interfaces require methods with the same name and signature,
one implementation satisfies both. If signatures differ (same name, different types), the compiler
emits a compile error with guidance on disambiguation:

```ds
interface Reader { read(): bytes }
interface JsonReader { read(): JsonValue }  // different return type

// ERROR: Ambiguous implementation of 'read' for FileParser
// Hint: Use explicit interface qualification or rename one method
class FileParser implements Reader, JsonReader { ... }
```

Resolution options:
1. Rename one method in the interface (if you control it)
2. Use wrapper types with explicit delegation
3. Implement only one interface directly, delegate the other

```ds
newtype interface Hashable {
    hash(): uint64
}

extension for Circle implements Hashable {
    hash(): uint64 { ... }
}
```

#### ITab Generation

For each (Type, Interface) pair where the type implements the interface:

1. Create a static itab with method pointers in interface declaration order
2. Store the itab as a global constant
3. When creating an interface reference, pair the object with the appropriate itab
4. For `unknown` or dynamic casts, build and cache the itab at runtime on first use
5. The cache is global per runtime and keyed by `(concrete TypeDescriptor, interface TypeDescriptor)`

**Itab layout:**
```ds
struct InterfaceItab<I> {
    typeDescriptor: &TypeDescriptor  // for T.is on interface refs
    methods: [FunctionPointer] // one per interface method, in declaration order
}
```

#### Cost Model

| Operation | Structural Interface | Nominal Interface | Class Virtual |
|-----------|---------------------|-------------------|---------------|
| Reference size | 2 pointers (fat) | 2 pointers (fat) | 1 pointer |
| Method call | 2 loads + indirect | 2 loads + indirect | 2 loads + indirect |
| Creation | Method-set check + itab | Direct itab | Just cast |
| Memory per type | itab per interface | itab per interface | vtable in object |

Structural interfaces enable TypeScript's duck typing but have overhead:
- 2× pointer size for interface references (fat pointer)
- One itab per (Type, Interface) pair
- Cache locality may suffer from double indirection

### Extension Methods

Extension methods are direct calls with the receiver as the first argument.
No vtable or itab lookup is needed.

```ds
extension for Vector2 {
    magnitude(): float { sqrt(this.x * this.x + this.y * this.y) }
}

const v = Vector2 { x: 3, y: 4 };
v.magnitude();  // direct call
```

Lowers to:
```mir
function @Vector2_ext.magnitude(v0: ref<@Vector2>) -> f64 {
block0(v0: ref<@Vector2>):
    v1 = field.get v0, 0       ; load x
    v2 = field.get v0, 1       ; load y
    v3 = fmul v1, v1
    v4 = fmul v2, v2
    v5 = fadd v3, v4
    v6 = intrinsic.sqrt(v5)
    return v6
}

; call site: v.magnitude()
v1 = call @Vector2_ext.magnitude(v0)
```

Extension methods are resolved statically at compile time based on the receiver type.
They do not participate in virtual dispatch.

### Getters and Setters

Property accessors lower to method calls.
There is nothing special about them at the MIR level.

```ds
class Circle {
    #radius: float64

    get area(): float64 { 3.14159 * this.#radius * this.#radius }
    set radius(r: float64) { this.#radius = r }
}

c.area          // call @Circle.get_area(c)
c.radius = 5    // call @Circle.set_radius(c, 5)
```

#### Pattern Matching with Getters

When pattern matching on an object with getters:
- Getter is called **once** per pattern
- Result is bound to the pattern variable
- Evaluation order: left-to-right in pattern

```ds
match (obj) {
    { foo: 0 } => ...  // calls obj.foo getter, compares to 0
    { foo: x } => ...  // calls obj.foo getter, binds to x
}
```

Equivalent to:
```ds
const __foo = obj.foo;  // one getter call
if (__foo === 0) { ... }
else { const x = __foo; ... }
```

## Memory Model

Memory allocation and ownership at the MIR level.
TypeScript/JavaScript uses garbage collection with no explicit memory management.
Destack preserves this simplicity by default for reference types, and value types are inline unless boxed.
It enables opt in control for performance critical code.
The **GC** implementation is assumed abstractly as "managed allocate" and TS compatible, we assume potential pauses and add some barriers.

### Allocation Modes

Destack supports three allocation modes, controllable via `AllocationMode` per function.
Users can annotate functions with `@noManaged` or `@stackOnly` decorators to enforce these modes:

| Mode | Allocations Allowed | Use Case |
|------|---------------------|----------|
| `Any` | All | Default, full TS compatibility |
| `NoManaged` | `RawAlloc`, `StackAlloc` | Realtime-safe, no GC pauses |
| `StackOnly` | `StackAlloc` | Embedded, deterministic |

#### Managed Allocation (GC)

`ManagedAlloc` creates GC-tracked objects.
The runtime provides garbage collection; Lower just emits the allocation instructions.
Allocator selection for native targets is configured per-target (`allocator`).

```mir
type @Point = struct { f32, f32 }

function @alloc_example() -> ref<@Point> {
block0:
    v0 = managed.alloc @Point
    return v0
}
```

**Write barriers:** Lower automatically inserts `Intrinsic::GcWriteBarrier` for all `ManagedReference` field writes.
The runtime uses this for concurrent marking.
See [INTRINSICS.md](INTRINSICS.md#garbage-collection) for details.

**Roots:** Each function has a stack map describing which slots contain managed references.
The GC uses these to find roots during collection.
Managed allocations do not include implicit headers, and any vtable pointer is part of the payload layout.
The allocator side tables store mark bits, size class, and the TypeDescriptor pointer used for scanning.

GC implementation details are target-specific and live in the runtime/codegen layers.
The general approach (when GC is enabled) is Go-like: insertion write barriers with a concurrent mark phase.

**Target-dependent behavior:**
GC features are conditional on target configuration. 
For targets without GC (freestanding, `@noManaged` code):
- No write barriers are emitted
- No stack maps are generated
- Managed allocations are a compile error
- Only `^T` ownership and `&T` borrows are available

Lower queries the target profile to determine which GC features to emit.
The runtime provides the actual GC implementation; Lower just emits the hooks.

#### Safepoints and Stack Maps

*Safepoints and stack maps only apply when GC is enabled for the target.*
With a Go-style concurrent GC, true "stop-the-world" pauses are minimal.
However, the GC still needs to find roots on each thread's stack during the mark phase.
This requires knowing which stack slots contain managed references at any given instruction.

**Why stack maps (not traditional safepoints):**
Go and similar runtimes use conservative stack scanning or async preemption.
For precise GC with AOT compilation, we generate stack maps that describe root locations.
The GC can scan roots at any point by consulting the stack map for the current PC.

**Stack map generation:**
Lower emits stack map metadata at:
1. **Function calls** - roots must be live across the call
2. **Allocation sites** - GC may trigger, need current roots
3. **Loop back-edges** - for long-running loops (optional, for latency)

The stack map covers the full function; codegen generates per-PC maps for call sites.
Between calls, the GC can async-preempt and scan conservatively if needed (like Go 1.14+).

**Stack maps:**
Each safepoint has an associated stack map describing which stack slots and registers
contain managed references at that point. 
Codegen emits these as metadata attached to the safepoint location. 
The GC uses stack maps to find roots during collection.

#### Raw Allocation

`raw.alloc` creates manually-managed heap memory for owned values (`^T`).
Cleanup uses `raw.drop` (with dispose) or `raw.free` (without dispose).

```mir
type @SomeType = struct { i64 }

v0 = raw.alloc @SomeType
; ... use v0 ...
raw.drop v0    ; drop glue: dispose + deallocate
```

For manual deallocation without dispose (FFI, low-level code):

```mir
v0 = raw.alloc @SomeType
; ... use v0 ...
raw.free v0    ; just deallocate, no dispose
```

No GC overhead.
Used with ownership annotations (`^T`) for Rust-like semantics.
In debug builds, a tracing allocator (like Zig's) can detect leaks, double-frees, and use-after-free.

#### Stack Allocation

`stack.alloc` creates frame-local storage.
Cleanup uses `stack.drop` for dispose; deallocation happens automatically when the frame exits.

```mir
type @SomeType = struct { i64 }

v0 = stack.alloc @SomeType
; ... use v0 ...
stack.drop v0    ; drop glue: dispose only, frame handles memory
```

No heap allocation.
The optimizer promotes `raw.alloc` to `stack.alloc` via escape analysis when the value doesn't escape the function.

### Drop Glue

The drop instructions (`raw.drop`, `stack.drop`) perform **drop glue**:

1. **Drop owned fields** in reverse declaration order (LIFO, like Rust/C++)
2. **Call dispose** (`Symbol.dispose`) if the type implements `Drop`
3. **Deallocate** (only for `raw.drop`; `stack.drop` skips this)

**Field drop order:** Fields are dropped in reverse declaration order.
This matches C++ and Rust destruction semantics: last declared, first destroyed.
For inherited classes, child fields drop before parent fields.

```ds
class Resource {
    a: ^FileHandle;  // declared first, dropped last
    b: ^Connection;  // declared second, dropped second
    c: ^Buffer;      // declared last, dropped first
}
// drop order: c, b, a
```

Drop glue metadata is attached to MIR type definitions during lowering.
Lower has full DIR type information (including `Drop` trait bounds) and generates the appropriate drop glue for each type.

**Important:** Lower only *marks* ownership on types and values (via `^T` modifiers and allocation instructions).
The actual drop instruction insertion happens in Optimize's `drop-insert` pass, which runs as part of the Verify phase.
This separation ensures drops are placed at precise last use points after all control flow is lowered.

| Instruction | Drop Fields | Call Dispose | Deallocate |
|-------------|-------------|--------------|------------|
| `raw.drop` | Yes (LIFO) | If `Drop` | Yes |
| `stack.drop` | Yes (LIFO) | If `Drop` | No (frame) |
| `raw.free` | No | No | Yes |

For managed allocations (`managed.alloc`), there is no drop instruction.
The GC handles cleanup, with finalizers for any `^T` fields (nondeterministic).

### Ownership

Destack aims to cover the "managedness" spectrum from TS to Go to Rust: managed references by default for reference types, explicit ownership when needed.
Most code just uses the default, and that should still be plenty fast thanks to real AOT compilation and fixed layouts (more like Go, Java, C#).
Performance critical code adds these ownership modifiers for manual control.

**Explicit Ownership:**

By default, a plain type `T` follows its type semantics.
Structs and primitives are values.
Classes and structural object types are managed references.
Type aliases inherit the semantics of their underlying type.

| Modifier | Semantics | After `foo(x)` | Who cleans up? |
|----------|-----------|----------------|----------------|
| `T` | Type default (value or managed reference) | `x` still valid | Type default |
| `&T` | Borrow (read only) | `x` still valid | Original owner |
| `&mut T` | Borrow (mutable) | `x` still valid, maybe changed | Original owner |
| `^T` | Ownership transfer | `x` **invalid** | New owner (or GC fallback) |
| `^mut T` | Ownership transfer (mutable) | `x` **invalid** | New owner (or GC fallback) |

Managed reference types are collected by the GC.
Value types only drop when owned or used with `using`.

For the default (`T`), the compiler optimizes automatically:
- Small value types are passed by copy
- Large value types may be passed indirectly or boxed when required
- Reference types are passed as references
- Escape analysis promotes managed allocations to stack when safe

Reference types are GC managed by default and can be shared freely.
Value types are copied by default and are not implicitly shared.
`^T` is for when there should only be one owner.
Accordingly, when calling a function with `^T`, the caller gives up ownership of the value to the callee.
After the transfer, the original binding is invalid:

```ds
function consume(data: ^LargeData) { ... }
const d = LargeData { ... }
consume(^d)    // ownership transferred
print(d.value) // ERROR: use after ownership transfer
```

Use after move is an error.
When a `^T` value reaches its **last proven use** without being transferred, it is **dropped**:

```ds
function process() {
    const data = ^LargeData { ... }  // we own this
    doWork(&data)                     // borrow it
    // data can be dropped before the next statement
}
```

`Drop` is a marker interface that opts a type into last use cleanup when possible.
(Types that implement `Drop` must also implement `Symbol.dispose`, which is invoked by the drop glue).
Optimize's `drop-insert` pass inserts drops at last use points (non lexical), including before
control flow merges and before coroutine suspension when the value is not used after resume.

**Borrowing:**

`&T` and `&mut T` are explicit references (pointers) to data.
They lower directly to pointer types in MIR:

```ds
function process(data: &Point) { ... }   // read only reference
function mutate(data: &mut Point) { ... } // mutable reference
```

Lowers to (conceptual):
```mir
type @Point = struct { f32, f32 }

function @process(v0: ref<borrowed @Point>) -> void { ... }
function @mutate(v0: ref<borrowed mut @Point>) -> void { ... }
```

Borrowing subfields lowers to explicit address projections (`field.addr`, `element.addr`).
Borrowed references are verified by the borrow check pass in Optimize.
Borrows are created by `field.addr`, `element.addr`, and by calls that return borrowed references with lifetimes.
A borrow ends when the reference value is no longer live.
Borrow checking uses liveness and alias analysis to detect conflicts and invalidations.
Dropping or freeing a value while it is borrowed is always an error.
In strict mode, conflicting borrows and invalidating stores are errors.
In lenient mode, the same situations produce warnings.

**Raw pointers:**

`*T` and `*mut T` are unsafe pointers with no borrow tracking.
They lower directly to `ref<raw T>` and `ref<raw mut T>`.
Deref and mutation use explicit `load`/`store` and pointer operations.
Conversions between borrowed references and raw pointers are explicit.

**Address spaces:**

Lower preserves address space annotations on references for native and accelerator targets.
The default address space is `generic`.
Non generic address spaces are only valid for borrowed and raw references.
`constant` references are always immutable.
Address space changes are explicit and use the `addrspace.cast` intrinsic.

**Borrow Modes:**

By default, `&T` and `&mut T` are hints and violations produce warnings.
They help document APIs, guide drops, and enable limited optimizations.
Set `borrowMode: "strict"` in `dsconfig.json` to enforce exclusive `&mut` borrows.
Strict mode enables stronger `noalias` optimizations and errors on violations.
Strict mode forbids:
- Aliasing `&mut` with any other borrow
- Storing `&mut` inside managed objects
- Holding `&mut` across `await` or generator suspension

### Closures

Functions that capture variables become closure values:

```ds
const x = 10
const f = (y: int) => x + y  // captures x
```

Lowers to a closure struct plus a lifted function.
The closure struct (MIR-level) captures the environment:
- `x: int64`

The closure value pairs the function pointer with the environment:
- `fnPtr: FunctionPointer`
- `env: ManagedReference<ClosureEnv>`

**Capture semantics:**
- `const` bindings are captured by value (copied into closure struct)
- `let` bindings are captured by reference (pointer to original location)
- This matches JavaScript's closure semantics

**Environment layout:**
Captured variables are stored in the closure struct in declaration order (order of first capture).
The struct is alignment-packed to minimize size. Interior pointers are used for reference captures.

The closure body receives `env` as an implicit first parameter.
Closure calls: load `fnPtr` and `env`, call with env prepended to arguments.

### Copy Elision and Move Semantics

Avoiding unnecessary copies is critical for "systems-level" performance.
Lower implements several strategies to minimize data movement.

**Return Value Optimization (RVO):**
When a function returns a locally-constructed value, we allocate directly into the caller's destination.

**Named RVO (NRVO):**
Extends RVO to named variables when there's a single return path.

**Inline hints:**
The `@inline` decorator is a hint, not a directive. Lower emits the function normally.
The Optimize phase decides whether to actually inline based on:
- Function size and complexity
- Call site frequency (from profiling if available)
- Whether inlining enables further optimizations
`@inline("always")` is a stronger hint but still not guaranteed.
`@inline("never")` prevents inlining (useful for debugging, code size).

**Move Semantics:**
By default, Destack follows TypeScript semantics for reference types.
Classes and structural object types are GC-managed references.
Assignment shares references; variables remain valid after being passed to functions.
Value types are copied unless moved with `^T`.
Move semantics only apply with explicit `^T` ownership.

### Stack Safety

#### Stack Overflow

Native targets use **guard pages** for stack overflow detection.
Overflow triggers immediate abort with diagnostic (like Go/Rust).

Stack overflow is treated as a bug (infinite recursion), not a recoverable condition.
No stack check prologue overhead in normal functions.

#### Stack Size

Default stack size is target-dependent (typically 1-8 MB).
Configurable via target profile or runtime initialization.

## Control Flow

Control flow constructs (errors, async, generators) lower to MIR blocks and terminators.
JavaScript's `throw`/`catch` and `async`/`await` are powerful but have runtime costs: exception tables, stack unwinding, state machine overhead.
**Result first error handling**: recoverable errors use `Result<T, E>` (zero cost early returns), while `throw` becomes an abort in native code.
Explicit errors in the type system, panics for bugs only (like Rust).
Async functions lower to state machines, preserving JS `Promise` semantics without the JS runtime overhead.

### Errors and Exceptions

Destack uses **Result-first error handling**: recoverable errors use `Result<T, E>`, while `throw` is reserved for unrecoverable panics (bugs).

| Mechanism | Use For | Example |
|-----------|---------|---------|
| `Result<T, E>` | Recoverable errors | Parse failures, file not found, network timeout |
| `throw` | Unrecoverable panics | Assertion failures, invariant violations, bugs |

**Panics indicate bugs**, not expected error conditions.
Use `Result` for anything the caller might want to handle.

The `?` operator propagates errors ergonomically:

```ds
function readConfig(): Result<Config, Error> {
    const text = readFile("config.json")?;    // propagates Err
    const json = parseJson(text)?;            // propagates Err
    Result.ok(Config.from(json))
}
```

Lowers to early return on error.

**Panic (throw):**
`throw` indicates an unrecoverable error (bug, invariant violation).
Unlike traditional exceptions, panics are not meant to be caught.
On native targets, `throw` aborts without unwinding.
Panic policy is configured per target (`panic`), and unwind requires runtime support.

### Coroutines: Async & Generators

TypeScript already has "function coloring": `await` is only valid inside `async function`, `yield` only inside `function*`.
This is baked into the language semantics we preserve.
State machine transformation is therefore the natural implementation strategy.

**Design principle:** Promise is a library type, state machines are a Lower transformation, the runtime glues them together.
External effects cross the runtime boundary through yield and resume so they can be recorded and replayed.

<pre>
┌─────────────────────────────────────────────────────────────────────────┐
│                              Architecture                                │
│                                                                          │
│   ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐     │
│   │  async function │───▶│  State Machine  │───▶│     Promise     │     │
│   │    (source)     │    │  (Lower output) │    │ (library type)  │     │
│   └─────────────────┘    └─────────────────┘    └─────────────────┘     │
│                                   │                      ▲              │
│                                   │                      │              │
│                                   ▼                      │              │
│                          ┌─────────────────┐             │              │
│                          │     Runtime     │─────────────┘              │
│                          │  (event loop)   │                            │
│                          └─────────────────┘                            │
└─────────────────────────────────────────────────────────────────────────┘
</pre>

#### Separation of Concerns

| Component | Responsibility | Location |
|-----------|----------------|----------|
| **Lower** | Transform async bodies to state machines, emit `yield` terminators | `compiler/src/lower/` |
| **Promise** | Store state (pending/fulfilled/rejected), manage continuations | `builtin/lib/native/` |
| **Runtime** | Event loop, microtask/macrotask queues, I/O primitives | `builtin/lib/native/` |

Lower does NOT know Promise internals.
Lower just emits `yield` terminators; the runtime hooks them to Promise continuations.

#### Promise as Library Type

Promise is a regular class defined in `language/builtin/lib/native/`:

```ds
class Promise<T> {
    #state: "pending" | "fulfilled" | "rejected" = "pending";
    #value: T | Error | null = null;
    #continuations: Array<Continuation<T>> = [];

    // register continuation, called when resolved
    then<U>(
        onFulfilled?: (value: T) => U | Promise<U>,
        onRejected?: (error: Error) => U | Promise<U>
    ): Promise<U> { ... }

    catch<U>(onRejected: (error: Error) => U | Promise<U>): Promise<U> { ... }
    finally(onFinally: () => void): Promise<T> { ... }

    // internal: resolve/reject (called by state machine or runtime)
    static resolve<T>(value: T): Promise<T> { ... }
    static reject<T>(error: Error): Promise<T> { ... }

    // combinators
    static all<T>(promises: Promise<T>[]): Promise<T[]> { ... }
    static race<T>(promises: Promise<T>[]): Promise<T> { ... }
    static any<T>(promises: Promise<T>[]): Promise<T> { ... }
    static allSettled<T>(promises: Promise<T>[]): Promise<SettledResult<T>[]> { ... }
}
```

This is a normal Destack type, and Lower treats it like any other type.
The "magic" is in how the runtime connects yield terminators to Promise continuations.

#### State Machine Transformation

Lower transforms each async function into a **tagged union state machine**.
The state is represented as an explicit enum, and all locals that survive across await points
are stored in the state machine struct.

**Components:**
1. **State machine struct** - captures locals that live across await points
2. **Entry function** - creates Promise, starts execution, returns Promise
3. **Poll function** - advances state machine, called on each resume

**State representation:**
The state field is a `uint8` (or larger if needed) representing which await point we're at:
- State 0: initial entry, before first await
- State N: suspended at await point N, waiting for promise
- State MAX: completed (terminal state)

**Local storage strategy:**
All locals that are potentially live across ANY await point are stored in the state struct.
This is conservative but correct. The optimizer may later prove some locals don't need storage.
Locals that are only used between await points are stack-allocated in the poll function.

```ds
// source
async function fetchUser(id: string): Promise<User> {
    const response = await fetch(`/users/${id}`);
    const data = await response.json();
    return User.from(data);
}
```

Conceptually, Lower generates:

```ds
// state machine struct: locals that survive across awaits
struct @fetchUser_StateMachine {
    state: uint8;
    id: string;
    response: Response | null;
    data: JsonValue | null;
    promise: Promise<User>;  // the outer promise to resolve
}

// entry: create state machine and promise, start execution
function fetchUser(id: string): Promise<User> {
    const promise = Promise<User>.new();
    const sm = @fetchUser_StateMachine {
        state: 0,
        id: id,
        response: null,
        data: null,
        promise: promise
    };
    @fetchUser_poll(sm);  // start execution
    return promise;        // return immediately
}

// poll: advance state machine one step
function @fetchUser_poll(sm: &@fetchUser_StateMachine): void {
    match (sm.state) {
        0 => {
            const p = fetch(`/users/${sm.id}`);
            p.then(response => {
                sm.response = response;
                sm.state = 1;
                @fetchUser_poll(sm);
            });
        }
        1 => {
            const p = sm.response!.json();
            p.then(data => {
                sm.data = data;
                sm.state = 2;
                @fetchUser_poll(sm);
            });
        }
        2 => {
            sm.promise.resolve(User.from(sm.data!));
        }
    }
}
```

#### MIR: The Yield Terminator

At the MIR level, `await` becomes a `yield` terminator:

```mir
yield <awaited_promise>, <resume_block>(<captured_values>)
```

Semantics:
1. **Suspend**: Save current state, return control to caller
2. **Register**: Runtime calls `awaited_promise.then(resume_callback)`
3. **Resume**: When promise resolves, runtime calls resume block with result

Captured values are passed as resume arguments, with the resolved value appended after them in the
resume block parameter list.

Full MIR for the async function:

```mir
type @fetchUser_SM = struct {
    u8,                    ; state
    ref<string>,           ; id
    ref<Response | null>,  ; response
    ref<JsonValue | null>, ; data
    ref<Promise<User>>     ; promise
}

; entry function: creates state machine and promise
function @fetchUser(id: ref<string>) -> ref<Promise<User>> {
block0(id: ref<string>):
    v0 = managed.alloc @Promise<User>
    call @Promise.init(v0)
    v1 = managed.alloc @fetchUser_SM
    field.set v1, 0, 0              ; state = 0
    field.set v1, 1, id             ; id
    field.set v1, 2, null           ; response
    field.set v1, 3, null           ; data
    field.set v1, 4, v0             ; promise
    call @fetchUser_poll(v1)        ; start execution
    return v0                        ; return promise immediately
}

; poll function: state machine
function @fetchUser_poll(sm: ref<@fetchUser_SM>) -> void {
block_dispatch(sm: ref<@fetchUser_SM>):
    v0 = field.get sm, 0            ; load state
    switch v0, [block_state0, block_state1, block_state2]

block_state0:
    v1 = field.get sm, 1            ; load id
    v2 = call @fetch(v1)            ; returns Promise<Response>
    yield v2, block_resume0(sm)

block_resume0(sm: ref<@fetchUser_SM>, response: ref<Response>):
    field.set sm, 2, response       ; store response
    field.set sm, 0, 1              ; state = 1
    jump block_dispatch             ; continue to next state

block_state1:
    v3 = field.get sm, 2            ; load response
    v4 = call @Response.json(v3)    ; returns Promise<JsonValue>
    yield v4, block_resume1(sm)

block_resume1(sm: ref<@fetchUser_SM>, data: ref<JsonValue>):
    field.set sm, 3, data           ; store data
    field.set sm, 0, 2              ; state = 2
    jump block_dispatch             ; continue to next state

block_state2:
    v5 = field.get sm, 3            ; load data
    v6 = call @User.from(v5)
    v7 = field.get sm, 4            ; load promise
    call @Promise.resolve(v7, v6)   ; resolve outer promise
    return
}
```

#### Runtime: Connecting Yield to Promise

The runtime interprets `yield` terminators by:

1. Creating a closure that captures the resume block and args
2. Calling `awaited_promise.then(closure)`
3. When promise resolves, closure calls resume block with result

```ds
// runtime pseudocode: handling yield
function handleYield<T>(
    awaitedPromise: Promise<T>,
    resumeBlock: (T, ...args: unknown[]) => void,
    resumeArgs: unknown[]
): void {
    awaitedPromise.then(value => {
        // schedule as microtask per JS semantics
        queueMicrotask(() => {
            resumeBlock(value, ...resumeArgs);
        });
    });
}
```

#### Event Loop Semantics

The runtime maintains JS-compatible task queues:

| Queue | Contents | When Drained |
|-------|----------|--------------|
| **Microtask** | Promise continuations (`.then`), `queueMicrotask` | After each task, completely |
| **Macrotask** | `setTimeout`, `setInterval`, I/O callbacks | One per event loop iteration |

```ds
setTimeout(() => console.log("A"), 0);           // macrotask
Promise.resolve().then(() => console.log("B"));  // microtask
console.log("C");
// output: C, B, A (same as JS)
```

Order of execution:
1. Current synchronous code runs to completion → prints "C"
2. Microtask queue drains → prints "B"
3. Next macrotask runs → prints "A"

#### Usage Scenarios

**Scenario 1: Simple await**
```ds
const response = await fetch(url);
```
- `fetch` returns `Promise<Response>` immediately
- Lower emits `yield fetchPromise, nextBlock(sm)`
- Runtime: `fetchPromise.then(response => resume(response))`
- When HTTP completes, resume block executes with response

**Scenario 2: Promise.all**
```ds
const [a, b, c] = await Promise.all([fetchA(), fetchB(), fetchC()]);
```
- `Promise.all` is a library function, creates a new Promise
- Internally tracks which inputs resolved
- Resolves output Promise when all inputs resolve
- State machine yields on the combined Promise

**Scenario 3: setTimeout**
```ds
await new Promise(resolve => setTimeout(resolve, 1000));
```
- `setTimeout` schedules callback on macrotask queue
- Callback calls `resolve()`, which resolves the Promise
- State machine resumes after 1000ms (plus microtask processing)

**Scenario 4: Error handling**
```ds
try {
    const data = await fetchData();
} catch (e) {
    console.error(e);
}
```
- If `fetchData()` rejects, Promise stores error
- Runtime calls reject handler instead of fulfill handler
- State machine jumps to catch block

#### Async Cancellation

**There is no implicit cancellation.**
This matches JS semantics: once an async function starts, its state machine runs to completion (or rejection).

The state machine remains alive as long as any continuation holds a reference to it.
When you `await` a Promise, the runtime registers a continuation closure that captures the state machine.
The state machine is eligible for GC only when:
- It completes (resolves or rejects)
- All continuations are unreachable (Promise is abandoned)

For explicit cancellation, use `AbortController` (same as modern JS/TS):

```ds
const controller = new AbortController();
const signal = controller.signal;

// pass signal to async operation
const data = await fetchData({ signal });

// elsewhere: cancel the operation
controller.abort();
```

Inside async functions, check `signal.aborted` at suspension points:

```ds
async function fetchWithCancel(url: string, signal: AbortSignal): Promise<Data> {
    const response = await fetch(url, { signal });
    if (signal.aborted) {
        return Result.err(AbortError.new());
    }
    const data = await response.json();
    return Result.ok(data);
}
```

The `AbortController`/`AbortSignal` pattern is a library concern, not a Lower concern.
Lower simply emits state machines; the library implements cancellation semantics via signal checking.

#### Generators

Generators use the same state machine approach but yield values to caller instead of awaiting:

```ds
function* range(start: int, end: int): Generator<int> {
    for (let i = start; i < end; i++) {
        yield i;
    }
}
```

The state machine implements `Iterator<T>`:
- `next()` advances to next yield, returns `{ value: T, done: boolean }`
- State persists across `next()` calls

```mir
function @range_next(sm: ref<@range_SM>) -> @IteratorResult<int> {
block0(sm: ref<@range_SM>):
    v0 = field.get sm, 0            ; state
    v1 = field.get sm, 1            ; i
    v2 = field.get sm, 2            ; end
    v3 = icmp_lt v1, v2
    branch v3, block_yield, block_done

block_yield:
    v4 = iadd v1, 1
    field.set sm, 1, v4             ; i++
    v5 = aggregate @IteratorResult<int> { value: v1, done: false }
    return v5

block_done:
    v6 = aggregate @IteratorResult<int> { value: undefined, done: true }
    return v6
}
```

#### Async Generators

Async generators combine both: `await` suspends, `yield` produces values.

```ds
async function* fetchPages(urls: string[]): AsyncGenerator<Page> {
    for (const url of urls) {
        const response = await fetch(url);
        yield await response.json();
    }
}
```

`next()` returns `Promise<IteratorResult<T>>`:
- Caller awaits the returned Promise
- State machine may suspend multiple times per `next()` call (on awaits)
- Eventually yields a value or completes

#### JS Target Behavior

On JS targets, async functions compile directly to JS async/await.
No state machine transformation is needed; the JS runtime handles it natively.
This preserves perfect Promise interop with existing JS code and avoids double-transformation overhead.

### Iterator Protocol

`for...of` uses the JavaScript iterator protocol.

**Interfaces:**
```ds
interface Iterator<T> {
    next(): { value: T, done: boolean }
}

interface Iterable<T> {
    [Symbol.iterator](): Iterator<T>
}
```

**Lowering:**
```mir
; for (const x of iterable) { body }
v0 = call iterable[Symbol.iterator]()
block_loop:
    v1 = call v0.next()
    v2 = field.get v1, "done"
    branch v2, block_exit, block_body
block_body:
    v3 = field.get v1, "value"
    ; ... body with x = v3 ...
    jump block_loop
block_exit:
```

**Range Iteration:**
Elaborate transforms `0..10` to `RangeExclusive { start: 0, end: 10 }`.
Range types implement `Iterable<int>`.

## Concurrency

Destack preserves JS and TS concurrency semantics by default while enabling native level parallelism on supported targets.
The core ideas are a single threaded event loop by default, `Promise` and `async` for concurrency, and `Worker` for parallelism.
Target specific implementations keep surface semantics consistent across JS, WASM, and native targets.

**Threading Model:**
Default behavior is a single threaded event loop with microtask and macrotask queues.
Native targets support worker threads with the same `Worker` API.
JS targets map to real JS `Worker` instances.
WASM targets map to host specific workers when available.
Native targets map to OS threads with message passing.
Shared memory is explicit and opt-in.

**Memory Model:**
Shared memory follows JS Atomics semantics.
Atomic operations are sequentially consistent by default and accept explicit orderings when needed.
Non atomic loads and stores have no cross thread ordering guarantees.
Data races on shared non atomic memory are undefined behavior on native targets.
This preserves JS and TS semantics while enabling native performance when code uses atomics.

**GC and Threads:**
GC heaps are per worker by default to match JS semantics and avoid sharing mutable GC objects.
GC managed objects are not shared across workers unless explicitly frozen or copied.
Shared memory uses raw pointers or explicit shared buffers.
Native targets may add a shared heap mode when the runtime provides the required GC and synchronization support.

---

# Target Configuration

Lower behavior is configured by target policies defined in `dsconfig.json` and the target profile.
See `language/workspace/src/config/target.rs` for the canonical definitions.

## Runtime Checks

### Bounds Checks

Control array and slice bounds checking.

| Variant | Behavior |
|---------|----------|
| `Always` | Bounds checks in all builds |
| `Debug` | Bounds checks only in debug builds (default) |
| `Never` | No bounds checks (unsafe, fastest) |

Bounds check failure triggers a panic (abort on native targets).

### Overflow Checks

Control integer overflow checking.

| Variant | Behavior |
|---------|----------|
| `Always` | Overflow checks in all builds |
| `Debug` | Overflow checks only in debug builds (default) |
| `Never` | No overflow checks; signed overflow is UB |

Overflow check failure triggers a panic.
Explicit wrapping (`+%`) and saturating (`+|`) operators bypass this policy.

## Error Handling

### Panic Policy

What happens when a panic occurs (via `throw` or failed assertions).

| Variant | Behavior |
|---------|----------|
| `Abort` | Terminate immediately via `Intrinsic::Abort` (default) |
| `Unwind` | Stack unwinding for destructors |

Abort is simpler and has no overhead.
Unwind enables deterministic destructor calls but requires exception tables.

### Unwind Format

Format for unwind information (for debuggers and profilers).

| Variant | Platform |
|---------|----------|
| `None` | No unwind info |
| `Dwarf` | DWARF CFI (Unix, macOS) |
| `Seh` | Structured Exception Handling (Windows) |

Even with `panic: Abort`, minimal unwind info can be emitted for debuggers.

## Debug and Symbols

### Debug Info Level

Granularity of debug information.

| Variant | Content |
|---------|---------|
| `None` | No debug info |
| `Line` | Line numbers and function names |
| `Full` | Line numbers, variables, types (default for debug) |

Debug info is emitted as DWARF (or platform equivalent) by codegen.

### Debug Execution Mode

Selects how debug workflows execute native or VM code.

| Variant | Behavior |
|---------|----------|
| `Auto` | Use `Deopt` for debug builds, `Native` for release |
| `Vm` | Force interpreter execution |
| `Deopt` | Run native with deopt-first debugging |
| `Native` | Run native only (no deopt) |

`debugMode` controls whether Lower emits full deopt metadata and inline frame maps.
`debugInfo` only affects native symbol/line metadata, not deopt fidelity.

### Strip Level

Symbol table stripping for release builds.

| Variant | Behavior |
|---------|----------|
| `None` | Keep all symbols |
| `Partial` | Strip internal symbols, keep exports |
| `Full` | Strip all symbols (smallest binary) |

## Execution Policy

Execution policy governs tiering, profiling, and determinism for native targets.
JS/TS targets ignore these settings and rely on their external runtimes.

### Tiering Model

Destack uses two execution tiers:

- **VM**: interpreter for comptime, deterministic debugging, and fallback execution
- **Native**: optimized native code for production performance

There is no baseline native tier by default.
The `debugMode` policy selects VM vs native execution for debugging workflows.

### Profiling Mode

Controls how runtime profiling data is collected for tiering and optimization.

| Variant | Behavior |
|---------|----------|
| `None` | No profiling collection |
| `Counters` | Call/branch/allocation counters only |
| `Sampling` | Sampling only (periodic opcode/site sampling) |
| `Hybrid` | Counters + sampling, optional inline caches (default) |

Profiling is scoped per isolate and consumed by the optimizer and runtime.
Inline caches are optional; if not implemented, `Hybrid` behaves like counters + sampling.

Profiling signals include:

- Call counts per function and callsite.
- Loop backedge counts per loop header.
- Allocation counts and bytes per site.
- Guard failures and deopt reasons per site.
- Branch direction bias per conditional branch.
- Indirect call target counts for interface dispatch.
- PC sampling for hot instruction ranges when sampling is enabled.

The runtime may expose these signals to telemetry systems, but the compiler is the source of truth for their semantics and collection points.
Telemetry libraries should consume these signals rather than re-instrumenting hot paths.

### Tiering Triggers

The runtime decides when to tier or OSR, but all triggers are expected to be available and tunable per target profile.

Tiering triggers include:

- Backedge counts and loop hotness.
- Wall-clock time spent in a function or loop.
- Allocation rate and allocation pressure.
- Explicit `@hot` or `@cold` hints on functions or blocks.
- Deopt frequency and guard failure rates.

Lower emits metadata for these sites so the runtime can make decisions without recompiling MIR.

### Speculation Policy

Controls guarded speculative optimizations in native code.

| Variant | Behavior |
|---------|----------|
| `None` | No speculative optimizations |
| `Guarded` | Guarded speculations with explicit deopt metadata (default) |
| `Aggressive` | Wider speculation surface with more guards |

All speculations must have a guard, a deopt map, and a recorded deopt reason.
Guard failures are local: in `debugMode = Deopt` they trigger deopt, otherwise they branch to the slow path.
There is no global invalidation for static layouts.

### OSR Policy

Controls on-stack replacement (VM → native) entry placement.

| Variant | Behavior |
|---------|----------|
| `Disabled` | No OSR |
| `LoopHeaders` | OSR at loop headers (default) |
| `Explicit` | OSR only at explicit sites |

Each OSR entry must define live values and phi materialization.

### Safepoint Policy

Controls safepoint insertion for preemption and deopt latency.

| Variant | Behavior |
|---------|----------|
| `CallsAllocBackEdges` | Calls, allocations, loop back-edges only |
| `Budgeted` | Add instruction-budget safepoints |

`safepointInterval` sets the instruction interval when `Budgeted` is enabled.
The interval is an abstract step budget.
The VM decrements the budget per threaded instruction.
Native code decrements at inserted safepoint polls and backedges.
Lower inserts budget polls at loop backedges by default and may add them to hot block entries when the target opts in.
Smaller intervals reduce preemption latency but increase overhead.

### Determinism Policy

Controls scheduling and randomness determinism.

| Variant | Behavior |
|---------|----------|
| `BestEffort` | No determinism guarantees |
| `Deterministic` | Deterministic scheduling + controlled randomness (default) |

`Deterministic` fixes scheduler decisions and PRNG seeds.

### Replay Policy

Controls whether external I/O is recorded or replayed.

| Variant | Behavior |
|---------|----------|
| `Off` | External I/O is not recorded |
| `Record` | Record external I/O |
| `Replay` | Replay external I/O |

`Record` records external I/O at runtime boundaries; unshimmed FFI/syscalls are rejected in this mode.
`Replay` consumes recorded external I/O and rejects unlogged effects.
External I/O is any operation outside the VM interpreter.
Filesystem and network access are external I/O.
Process, environment, and clock sources are external I/O.
Randomness and entropy sources are external I/O.
Host callbacks and FFI calls are external I/O.
All external I/O must go through runtime shims in `Record` and `Replay`.
Record and Replay enable time-travel debugging and simulation testing in userland libraries.

## Memory

### Allocator

Global allocator selection for native targets.

| Variant | Description |
|---------|-------------|
| `System` | Platform default (malloc/free) |
| `MiMalloc` | Microsoft's mimalloc (fast, low fragmentation) |
| `JeMalloc` | FreeBSD's jemalloc (good for large heaps) |
| `Custom` | User-provided allocator |

The allocator provides both managed (GC) and raw allocations.

### GC Strategy

Native targets use a Go-style, headerless managed heap with side tables.
The default GC strategy is generational with incremental marking.
Concurrent marking may be enabled by the runtime, but is not required by the ABI.

Write barriers are inserted by Lower at all managed write sites.
Stack maps are exact at all safepoints for precise tracing.
The barrier model is Go-style Dijkstra with shade-on-write.

When `determinism` is `Deterministic` or `replay` is `Record` or `Replay`, GC scheduling must be deterministic.
The runtime should use allocation-count thresholds and deterministic mark/sweep scheduling.
Concurrent or parallel marking is permitted only when the runtime can guarantee deterministic scheduling.

### Snapshots

Snapshots capture an isolate for deterministic replay, testing, and debugging.
Snapshots are required to implement `Record` and `Replay`.
Snapshots are a core debugging tool for deterministic execution.

A snapshot captures:

- Managed heap, raw heap, and globals are captured.
- VM continuations and stacks are captured.
- Runtime scheduler state for the isolate is captured.

Snapshots exclude external handles.
Any external handle must be reattached explicitly by the runtime after restore.

Snapshots are versioned and tied to the target ABI.
The runtime must reject snapshot restore when the compiler version, target triple, or GC layout does not match.

### Borrow Mode

How borrow annotations are enforced.

| Variant | Behavior |
|---------|----------|
| `Hint` | Warnings only, no hard errors (default) |
| `Strict` | Hard errors on borrow violations; enables `noalias` |

Strict mode enables stronger optimizations but requires more careful code.

## Codegen

### Relocation Model

Position-independent code generation.

| Variant | Use Case |
|---------|----------|
| `Static` | Fixed addresses (executables on some platforms) |
| `Pic` | Position-independent code (shared libraries) |
| `Pie` | Position-independent executable (default for security) |

### Link Mode

Preference for linking dependencies.

| Variant | Behavior |
|---------|----------|
| `Static` | Prefer static linking (larger binary, no runtime deps) |
| `Dynamic` | Prefer dynamic linking (smaller binary, runtime deps) |

### CPU and Features

Target CPU and feature detection.

- `cpu`: Target CPU model (e.g., "haswell", "apple-m1", "generic")
- `cpuFeatures`: Enabled features (e.g., "avx2", "neon", "simd128")

Lower uses `cpuFeatures` to gate SIMD codegen.
If a feature is unavailable, vector operations scalarize to loops.

---

# Code Generation

MIR is target-independent, so code generation is mostly mechanical translation.
See `language/codegen/` for target-specific backends (Cranelift for native/WASM).

## Debug Info

Lower preserves source information so native code can emit high-quality debug symbols.
This is required for source-level debugging, accurate stack traces, and profiling.
Debug info emission is configured per target (`debugInfo`).

**What Lower records:**
- Source spans on every block and instruction (file, line, column)
- Function debug names (human-readable) plus mangled symbol names for linkage
- Local variable names and lexical scope ranges
- Type debug descriptors (struct/class names, field names, offsets)
- Inline callsite chains for inlined functions

**Codegen output:**
Cranelift consumes this metadata and emits DWARF debug info for native targets.
The MIR itself remains layout-only; debug names are metadata attached to MIR nodes.

## Symbol Visibility

Symbols have visibility levels that control linking:

| Visibility | Description |
|------------|-------------|
| `Local` | Internal to module, not exported |
| `Export` | Visible outside module, public API |
| `Import` | Declared here, defined elsewhere |

**@export decorator:**
```ds
@export("C")
function add(a: int32, b: int32): int32 { a + b }
```

Exports the function with C ABI for FFI.
Without a calling convention, uses Destack ABI and is compiler-version private.

**public modifier:**
```ds
public function process(data: Data): Result { ... }
```

Exports with Destack ABI.

## Module Initialization

Each module may have initialization code that runs before `main()`:
- Static global initializers
- Top-level `using` statements (for resource acquisition)
- Module-level side effects

Lower generates a `__init` function per module containing this code.
Initialization order follows import dependencies: if module A imports module B, B's `__init` runs first.

### Initialization Order

1. Imports are initialized in depth-first order
2. Each module's `__init` runs exactly once
3. Top-level statements execute in source order within a module
4. `main()` runs after all `__init` functions complete

### Cycle Detection

**Value-evaluation cycles are a compile error.**
The Analyze phase performs static analysis to detect cycles involving evaluated expressions.

**Allowed:**
- Type-only imports in cycles (types have no initialization code)
- Function references (not called at init time)
- Lazy values (computed on first access, not at init)

**Forbidden (compile error):**
- `const x = otherModule.y` where `otherModule.y` depends on `x`
- Circular `using` declarations
- Any cycle where module A's init reads a value from module B, and B's init reads from A

This is the same approach Go uses: deterministic initialization order, no runtime cycle detection.

```ds
// module.ds
const config = loadConfig();  // runs during init

using logger = Logger.new();  // acquired during init, released at shutdown

export function process() { ... }
```

Generates:
```mir
function @module.__init() -> void {
block0:
    v0 = call @loadConfig()
    global.store @config, v0
    v1 = call @Logger.new()
    global.store @logger, v1
    return
}
```
