#!/usr/bin/env bun
// load package sources with their module metadata, which loads rolldown's native binding
await import("@destack/package/bun/preload");

// remove the binding's path, which every other native binding would load as its own
delete process.env["NAPI_RS_NATIVE_LIBRARY_PATH"];

/** The environments of the machine Bun runs on. */
const { BunEnvironment } = await import("@destack/environment/bun");

// answer the host's requests on this machine
await (await import("./build/worker.ts")).CompilerWorker.run(BunEnvironment.system);

export {};
