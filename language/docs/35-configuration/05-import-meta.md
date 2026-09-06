---
title: import.meta
description: Module-level constants.
---

# import.meta

For module-level constants we keep and extend the `import.meta` convention:

| Field | Description | Type | Examples |
|-------|-------------|------|----------|
| `import.meta.url` | current module URL | `string` | `"file:///app/src/main.ds"`, `"https://example.com/mod.ds"` |
| `import.meta.path` | current local file path, when available | `string \| undefined` | `"/app/src/main.ds"`, `undefined` |
| `import.meta.dir` | current local directory, when available | `string \| undefined` | `"/app/src"`, `undefined` |
| `import.meta.output` | output artifact | `Output` | `"bundle"`, `"program"` |
| `import.meta.platform` | target operating system | `Platform` | `"linux"`, `"windows"`, `"none"` |
| `import.meta.host` | target host environment | `Host` | `"browser"`, `"native"`, `"wasi"` |
| `import.meta.target` | target family and ABI | `Target` | `{ family: "unix", arch: "x64", abi: "gnu" }` |
| `import.meta.targetName` | active build target name | `string \| undefined` | `"web"`, `"native"` |
| `import.meta.product` | active deliverable product name | `Product \| undefined` | `"app"`, `"server"` |
| `import.meta.version` | active package version | `string \| undefined` | `"2026.5.2"` |
| `import.meta.stability` | active package or product stability | `Stability \| undefined` | `"alpha"`, `"stable"` |
| `import.meta.runtime` | semantic runtime | `Runtime` | `"destack"`, `"js"` |
| `import.meta.<mode>` | mode shorthands for `debug`, `dev`, `prod`, `test`, `bench`, `lint` | `boolean` | `import.meta.test`, `import.meta.prod` |
| `import.meta.env` | configured build environment | `{ readonly [key: string]: string \| undefined }` | `{ NODE_ENV: "production" }` |
