import { Pipe } from "@destack/environment";
import { Tarball, TarballEntry } from "@destack/package/archive";
import { MemoryBuild } from "@destack/package/test";
import { PackageVersion } from "@destack/package";
import { expect, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";
import { NpmTarball } from "./tarball.ts";

/** The app the fixture build releases. */
const NOTES = PackageVersion.parse({
    id: "package-019f5530-8000-7000-8000-0000000000a1",
    name: "@acme/notes",
    version: "2026.11.3",
});

/** Encode text as a file's bytes. */
function bytes(text: string): Uint8Array<ArrayBuffer> {
    return new TextEncoder().encode(text);
}

test("archive a build's files below package/ in path order, its workspace dependencies at the linked versions", async () => {
    // build a package that depends on a workspace package and an npm one
    await using simulation = testSimulation();
    const { crypto } = simulation.environment("test");
    const authored = {
        name: "@acme/notes",
        version: "2026.11.3",
        exports: { "./package": "./src/package.ts" },
        dependencies: { "@destack/object": "workspace:*", zod: "4.1.0" },
    };
    const build = await MemoryBuild.write(
        new Map([
            ["src/package.ts", bytes("export {};\n")],
            ["package.json", bytes(`${JSON.stringify(authored)}\n`)],
        ]),
        { package: NOTES, role: "app" },
        crypto,
    );

    // archive it at the version its lock links
    const entries = await NpmTarball.entries(build, { "@destack/object": "2026.11.2" });
    const read: [string, string][] = [];
    const { compression } = simulation.environment("test");
    const zipped = NpmTarball.stream(entries, compression);
    const archive = Pipe.through(zipped, compression.decompress("gzip"));
    for await (const entry of Tarball.read(archive)) {
        read.push([entry.path, new TextDecoder().decode(await TarballEntry.bytes(entry))]);
    }
    const released = {
        ...authored,
        dependencies: { "@destack/object": "2026.11.2", zod: "4.1.0" },
    };
    expect(read).toEqual([
        ["package/package.json", `${JSON.stringify(released, null, 4)}\n`],
        ["package/src/package.ts", "export {};\n"],
    ]);

    // refuse a workspace dependency whose release the lock does not link
    await expect(NpmTarball.entries(build, {})).rejects.toThrow(
        "dependency '@destack/object' is invalid: its release is not linked",
    );
});
