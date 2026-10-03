import { expect, test } from "@destack/test";
import { PackageError } from "../error/error.ts";
import { PackageDefinition } from "./definition.ts";
import { Capabilities } from "./capability.ts";

/** The fields every destack.json carries. */
const REQUIRED = {
    $schema: "https://destack.app/schemas/2026.10.0/destack.json",
    id: "package-01996ab0-0000-7000-8000-000000000001",
    language: "typescript",
};

/** An indexer's capabilities, each with its reason. */
const indexer = {
    process: { reason: "indexes checkouts with native tools" },
    network: {
        connect: ["api.github.com", "*.githubusercontent.com:443"],
        reason: "fetches linked repositories",
    },
    run: { commands: ["git"], reason: "reads history" },
    fs: { access: "read", reason: "indexes your notes" },
    listen: { reason: "serves its index to the editor" },
    "clipboard-write": { reason: "copies search results" },
};

test("run workloads in a Bun process only from a package compiling for Bun", () => {
    // refuse the process capability of a workerd package, and accept a Bun library without it
    const read = (fields: object) =>
        PackageDefinition.read(JSON.stringify({ ...REQUIRED, ...fields }));
    expect(() =>
        read({ runtimes: ["workerd"], capabilities: { process: indexer.process } }),
    ).toThrow(
        new PackageError("INVALID_DEFINITION", "the process capability requires the bun runtime"),
    );
    expect(read({ runtimes: ["bun"] }).runtimes).toEqual(["bun"]);
});

test("select the capabilities a workload names, refusing undeclared ones", () => {
    const declared = Capabilities.parse(indexer);
    expect(Capabilities.select(declared, ["network", "run", "clipboard-write"])).toEqual({
        network: indexer.network,
        run: indexer.run,
        "clipboard-write": indexer["clipboard-write"],
    });
    expect(() => Capabilities.select({ network: indexer.network }, ["network", "env"])).toThrow(
        new PackageError("INVALID_DEFINITION", "the workload uses undeclared capabilities: env"),
    );
});

test("grant the required capabilities and only the optional ones an installation allows", () => {
    // declare an optional network, directory access and camera beside a required process, commands and clipboard
    const declared = Capabilities.parse({
        ...indexer,
        network: { ...indexer.network, optional: true },
        fs: { ...indexer.fs, optional: true },
        camera: { reason: "scans receipts", optional: true },
    });
    const { network: _network, fs: _fs, camera: _camera, ...required } = declared;

    // keep the required ones without allowances, and add the optional ones under allowed names
    expect([
        Capabilities.grant(declared, []),
        Capabilities.grant(declared, ["network", "camera"]),
        Capabilities.grant(declared, ["fs", "microphone"]),
    ]).toEqual([
        required,
        { ...required, network: declared.network, camera: declared.camera },
        { ...required, fs: declared.fs },
    ]);
});
