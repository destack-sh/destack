# import.meta

## metadata

Module metadata available via `import.meta`.

### output is a known union

`import.meta.output` returns the output format for the profile.

```ds
const output = import.meta.output;
output satisfies "js" | "ts" | "wasm" | "native";
```

### runtime is a known union

`import.meta.runtime` returns the runtime for the profile.

```ds
const runtime = import.meta.runtime;
runtime satisfies
    | "browser"
    | "node"
    | "deno"
    | "bun"
    | "worker"
    | "wasm-js"
    | "wasm-wasi"
    | "native-managed"
    | "native-freestanding"
    | "native-embedded";
```

### platform is a known union

`import.meta.platform` returns the target platform for the profile.

```ds
const platform = import.meta.platform;
platform satisfies
    | "web"
    | "windows"
    | "macos"
    | "linux"
    | "freebsd"
    | "openbsd"
    | "netbsd"
    | "dragonfly"
    | "solaris"
    | "illumos"
    | "haiku"
    | "fuchsia"
    | "redox"
    | "hermit"
    | "ios"
    | "android"
    | "wasi"
    | "emscripten"
    | "bare-metal"
    | "universal";
```

### target metadata is typed

`import.meta.target` exposes structured target metadata.

```ds
const family = import.meta.target.family;
family satisfies
    | "web"
    | "windows"
    | "unix"
    | "wasm"
    | "bare-metal"
    | "universal"
    | "other";

const vendor: string = import.meta.target.vendor;
const env: string | undefined = import.meta.target.env;
const abi: string | undefined = import.meta.target.abi;
const arch: string | undefined = import.meta.target.arch;
```

### builtin modes have boolean shorthands

Built-in modes are exposed as boolean shorthands.

```ds
const debug: boolean = import.meta.debug;
const dev: boolean = import.meta.dev;
const prod: boolean = import.meta.prod;
const test: boolean = import.meta.test;
const bench: boolean = import.meta.bench;
const lint: boolean = import.meta.lint;
```

### module paths are typed

URL is always present and local paths are optional.

```ds
const url: string = import.meta.url;
const path: string | undefined = import.meta.path;
const dir: string | undefined = import.meta.dir;
```

### env exposes strings and helper fields

`import.meta.env` exposes string values and helpers.

```ds
const mode: "development" | "production" | "test" | undefined =
    import.meta.env.NODE_ENV;
const dev: boolean = import.meta.env.DEV;
const prod: boolean = import.meta.env.PROD;
const test: boolean = import.meta.env.TEST;
const value: string | undefined = import.meta.env.CUSTOM_KEY;
```
