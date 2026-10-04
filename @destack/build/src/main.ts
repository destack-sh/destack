#!/usr/bin/env bun
/** The tools on disk, read before any module loads rolldown. */
const { Toolchain } = await import("@destack/check/toolchain");

// load rolldown's native binding from the toolchain beside a standalone compiler
if (Toolchain.directory !== undefined) {
    const binding = Toolchain.locate(
        [`@rolldown/binding-${Toolchain.platform}`],
        Toolchain.directory,
    );
    process.env["NAPI_RS_NATIVE_LIBRARY_PATH"] =
        `${binding}/rolldown-binding.${Toolchain.platform}.node`;
}

// load package sources with their module metadata before the compiler loads any, which loads rolldown
await import("@destack/package/bun/preload");

// keep the binding's path from the tools the compiler starts, which load their bindings themselves
delete process.env["NAPI_RS_NATIVE_LIBRARY_PATH"];

// answer the host's requests
await (await import("./build/worker.ts")).runCompiler();

export {};
