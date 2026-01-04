# Runtime

Runtime library for WASM and Native Destack binaries.
Provides memory management, async execution, I/O, and debug support.

---

# Scope

The Runtime is **only** for WASM and Native targets.
JS/TS targets use external runtimes (Node, Bun, Deno, browsers) and don't link this library at all.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                 JS/TS TARGETS                               │
│                                                                             │
│  Stages:  Source ───► Compiler ───► JS/TS ───► Node/Bun/Deno                │
│  Output:    .ds        codegen      .js/.ts    external runtime             │
│                                                                             │
│                       (no Destack runtime needed)                           │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                              WASM/NATIVE TARGETS                            │
│                                                                             │
│  Stages:  Source ───► Compiler ───► Codegen ───► Binary                     │
│  Output:    .ds          MIR        .wasm/.o    linked with Runtime         │
│                                                                             │
│                       (Runtime provides all services)                       │
└─────────────────────────────────────────────────────────────────────────────┘
```

The Runtime gets linked into compiled binaries; it's not a separate process.
