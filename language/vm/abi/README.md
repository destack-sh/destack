# VM ABI

Versioned metadata wire types shared by compiler, VM, and runtime.
This crate contains wire types only, not executable VM logic.

## Scope

- safepoint tables
- deopt maps and frame maps
- OSR entries
- GC stack maps
- metadata headers and versioning
- native state and deopt requests (compiled engine)

## Non-goals

- Value representation
- heap layout or GC implementation details
- interpreter entrypoints or runtime scheduling

## Layout

<pre>
┌─────────────────────────────────────────────────────────────────────────────┐
│                                  VM ABI                                     │
│                                                                             │
│  metadata: safepoints, deopt maps, stack maps, OSR entries                   │
│  versioning: headers, compatibility checks                                  │
│  native: machine state + deopt request                                      │
└─────────────────────────────────────────────────────────────────────────────┘
</pre>
