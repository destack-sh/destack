# Resolve

Resolve connects modules into a graph and figures out what cross-module references point to for a specific profile.
Resolve is the first phase that treats the program as a system rather than isolated trees.

Resolve treats `destack:`, `platform:`, `node:`, `bun:`, and `deno:` as builtin protocol namespaces.
Unknown protocol schemes fail with dedicated resolve diagnostics instead of falling back to generic unresolved module errors.
Protocol availability is target gated by the active profile: runtime, output, and platform.
Runtime specific protocols are gated explicitly: `node:` is supported on Node compatible runtimes, `bun:` on Bun, and `deno:` on Deno.
Bare Node builtin compatibility is canonicalized to `node:` form only after standard resolution misses and ambient Node builtin bindings are available for the active profile.
Internal `platform:` imports can be policy gated via `compilerOptions.noInternalImport` with allow, warn, or deny behavior.
