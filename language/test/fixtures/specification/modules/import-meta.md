# import.meta

## tests

Tests for module metadata available via `import.meta`.

### output is a known union

> `import.meta.output` returns the output format for the profile.

```ds
const output = import.meta.output;
output satisfies "js" | "ts" | "wasm" | "native";
```

### runtime is a known union

> `import.meta.runtime` returns the runtime for the profile.

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
    | "native-hosted"
    | "native-freestanding"
    | "native-embedded";
```

### platform is a known union

> `import.meta.platform` returns the target platform for the profile.

```ds
const platform = import.meta.platform;
platform satisfies
    | "web"
    | "windows"
    | "macos"
    | "linux"
    | "ios"
    | "android"
    | "wasi"
    | "bare-metal"
    | "universal";
```

### debug and test are booleans

> `import.meta.debug` and `import.meta.test` are booleans.

```ds
const debug: boolean = import.meta.debug;
const test: boolean = import.meta.test;
```

### module paths are typed

> URL is always present and paths are optional.

```ds
const url: string = import.meta.url;
const path: string | undefined = import.meta.path;
const file: string | undefined = import.meta.file;
const filename: string | undefined = import.meta.filename;
const dir: string | undefined = import.meta.dir;
const dirname: string | undefined = import.meta.dirname;
```

### env exposes strings and helper fields

> `import.meta.env` exposes string values and helpers.

```ds
const mode: "development" | "production" | "test" | undefined =
    import.meta.env.NODE_ENV;
const dev: boolean = import.meta.env.DEV;
const prod: boolean = import.meta.env.PROD;
const test: boolean = import.meta.env.TEST;
const value: string | undefined = import.meta.env.CUSTOM_KEY;
```