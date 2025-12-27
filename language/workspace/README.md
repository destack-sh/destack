# workspace

Core data structures for managing Destack projects.
The workspace crate provides three main things: 
 1. **Configuration**: unified configuration
 2. **Containers**: container types (Session, Workspace, Program)
 3. **Queries**: query infrastructure for IDE features

## Configuration

The `config/` module defines unified configuration for the toolchain.

### dsconfig.json

Destack's project configuration, similar to `tsconfig.json` but with Destack-specific options.

```json
{
    "compilerOptions": {
        "target": "es2024",
        "strict": true,
        "noImplicitAny": true
    },
    "formatter": {
        "lineWidth": 100,
        "indentStyle": "space"
    },
    "linter": {
        "preset": "recommended"
    },
    "targets": {
        "web": { "output": "js", "platform": "browser" },
        "native": { "output": "wasm", "optimize": true }
    }
}
```

Child packages inherit from parent `dsconfig.json` with "most restrictive wins" semantics.

### Target

Targets define what we build and where the code ultimately runs.
Runtime and platform define semantics and APIs.
Target triples define native ABI and architecture.
(Codegen backends actually generate the code for some specific target.)

**Build target configuration:**

```ds
struct Target {
    name: string,
    output: OutputFormat,      // Js, Ts, Wasm, Native
    mode: OutputMode,          // Directory or File
    platform: Platform,        // Web, Windows, MacOS, Linux, IOS, Android, Wasi, BareMetal, Universal
    runtime: Runtime,          // Browser, Node, Bun, Deno, WasmJs, WasmWasi, NativeHosted, NativeFreestanding, NativeEmbedded
    runtimeVersion: string | null,
    optimize: boolean,
    optimizeLevel: OptimizeLevel,
    debug: boolean,
    boundsChecks: BoundsCheckPolicy,
    overflowChecks: OverflowCheckPolicy,
    panic: PanicPolicy,
    unwind: UnwindFormat,
    debugInfo: DebugInfoLevel,
    strip: StripLevel,
    allocator: Allocator,
    relocationModel: RelocationModel,
    linkMode: LinkMode,
    targetTriple: string | null,
    cpu: string | null,
    cpuFeatures: string[],
    // ...
}
```

### Formatter / Linter

Unified options that flow through the entire toolchain.
These are parsed from `dsconfig.json` and passed to formatter and linter.


## Containers

Outside of tests, we need to actually put the Program and Modules *somewhere*.
The workspace crate defines three levels of "containment":

```
Session (daemon/LSP lifetime)
    │
    ├── Workspace (monorepo or single package)
    │       │
    │       └── Program (compilation unit)
    │               │
    │               ├── Package (npm package with package.json)
    │               │       └── Module (single source file)
    │               │
    │               └── Artifacts (generated outputs)
```

### Session

Long-lived state for daemon/LSP use cases.
A session manages file watching, program caching, and shared registries.

```ds
struct Session {
    workspace: Workspace,
    cwd: Path,
    fs: FileSystem,
    files: FileRegistry,
    programs: Map<Path, Program>,
    builtins: LanguageBuiltins,
    // ...
}
```

Multiple programs can exist in a session (e.g., different build targets).
The session shares file and module registries across programs.

### Workspace

Organizational structure discovered from disk.
Represents either a real monorepo (npm/pnpm workspaces) or just a single-package project.

```ds
struct Workspace {
    root: Path,
    kind: WorkspaceKind,
    config: DsConfig | null,
    // ...
}
```

### Program

The main compilation unit.
Contains all packages, modules, and artifacts for a single compilation context.

```ds
struct Program {
    cwd: Path,
    files: FileRegistry,
    packages: PackageRegistry,
    modules: ModuleRegistry,
    artifacts: ArtifactRegistry,
    diagnostics: DiagnosticCollector,
    // ...
}
```

A program holds the AST, DIR, and MIR for each module, plus generated artifacts from codegen.

## Queries

The `query/` module provides common "queries" for IDE features.
These handlers power editor integrations and map directly to LSP (without depending on it, like rust-analyzer).
