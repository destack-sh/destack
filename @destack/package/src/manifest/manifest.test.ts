import { expect, refusal, test } from "@destack/test";
import { PackageManifest } from "./manifest.ts";

/** A manifest's fields beside its package. */
const FIELDS = {
    formatVersion: 1,
    language: "typescript",
    lists: {
        dependencies: {
            path: "manifest/dependencies.json",
            digest: "0".repeat(64),
            size: 2,
            mediaType: "application/json",
        },
        files: {
            path: "manifest/files.json",
            digest: "0".repeat(64),
            size: 2,
            mediaType: "application/json",
        },
        sourceMaps: {
            path: "manifest/sourceMaps.json",
            digest: "0".repeat(64),
            size: 2,
            mediaType: "application/json",
        },
        graph: {
            path: "manifest/graph.json",
            digest: "0".repeat(64),
            size: 2,
            mediaType: "application/json",
        },
    },
    outputs: {},
} as const;

test("read a Destack package's build as its package, refusing an npm unit's build", async () => {
    // read a package's manifest, then an npm unit's
    const destack = PackageManifest.parse({
        ...FIELDS,
        package: {
            id: "package-01996ab0-0000-7000-8000-0000000000a1",
            name: "@example/notes",
            version: "2026.9.0",
        },
    });
    const npm = PackageManifest.parse({
        ...FIELDS,
        package: { name: "zod", version: "4.6.5", integrity: "sha512-fixture" },
    });

    expect([
        PackageManifest.package(destack),
        await refusal(Promise.resolve().then(() => PackageManifest.package(npm))),
    ]).toEqual([
        {
            id: "package-01996ab0-0000-7000-8000-0000000000a1",
            name: "@example/notes",
            version: "2026.9.0",
        },
        [
            "BAD_REQUEST",
            "INVALID_BUILD",
            "zod@4.6.5 is an npm package build, not a Destack package build",
        ],
    ]);
});
