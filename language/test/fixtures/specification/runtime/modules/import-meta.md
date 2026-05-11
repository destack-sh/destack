# import.meta

## metadata

Module metadata available via `import.meta`.

### runtime is a known union

`import.meta.runtime` returns the runtime for the profile.

```ds
const runtime = import.meta.runtime;
runtime satisfies "destack" | "js";
```

### platform is a known union

`import.meta.platform` returns the target platform for the profile.

```ds
type Platform =
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

const platform = import.meta.platform;
platform satisfies Platform;
```

### target metadata is typed

`import.meta.target` exposes structured target metadata.

```ds
const family = import.meta.target.family;
family satisfies "web" | "windows" | "unix" | "wasm" | "bare-metal" | "universal" | "other";

const vendor: string | undefined = import.meta.target.vendor;
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

### env exposes configured strings

`import.meta.env` exposes configured string values.

```ds

const value: string | undefined = import.meta.env.CUSTOM_KEY;
```
