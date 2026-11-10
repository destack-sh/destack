#!/usr/bin/env bun
// load package sources with their module metadata, which loads rolldown's native binding
await import("@destack/package/bun/preload");

// remove the binding's path, which every other native binding would load as its own
delete process.env["NAPI_RS_NATIVE_LIBRARY_PATH"];

/** The environments of the machine Bun runs on. */
const { BunEnvironment } = await import("@destack/environment/bun");

/** The compiler's own program, package locator and definition. */
const [{ Program }, { PackageLocator }, { DependencyName, PackageDefinition }, own] =
    await Promise.all([
        import("@destack/package/bun"),
        import("@destack/package/locator"),
        import("@destack/package"),
        import("../destack.json", { with: { type: "json" } }),
    ]);

/** The modules the compiler shares with the code it loads, from its compiled table or its files. */
const program = await Program.current(
    async () => new PackageLocator(BunEnvironment.system.files).shared(import.meta.dirname),
    BunEnvironment.system.process.loader,
);

/** The packages the compiler's definition shares, its toolchain, apart from the builtins. */
const { shares = [] } = PackageDefinition.parse(own.default);

// link their modules known before loading, so the extensions the compiler imports share them
await program.link(program.specifiers.filter((each) => shares.includes(DependencyName.of(each))));

/** The toolchain directory the compiler's starter names, required in a release. */
const directory = BunEnvironment.system.process.variables["DESTACK_TOOLCHAIN"];

// refuse a release started without its toolchain
if (BunEnvironment.system.process.runtime.isStandalone && directory === undefined) {
    throw new Error("DESTACK_TOOLCHAIN is not set");
}

/** The tools on disk: a checked-out toolchain, or a workspace's packages. */
const { Toolchain } = await import("@destack/check/toolchain");

/** The tools the compiler runs: the named toolchain, else the workspace's. */
const toolchain = directory === undefined ? Toolchain.workspace() : Toolchain.at(directory);

// answer the host's requests on this machine
await (
    await import("./build/worker.ts")
).CompilerWorker.run(program, toolchain, BunEnvironment.system);

export {};
