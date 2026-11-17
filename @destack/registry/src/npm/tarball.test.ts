import { Pipe } from "@destack/environment";
import { Tarball, TarballEntry } from "@destack/package/archive";
import { MemoryBuild } from "@destack/package/test";
import { PackageVersion } from "@destack/package";
import { present } from "@destack/schema";
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

test("archive a build's files below package/ in path order, its workspace dependencies at the linked versions and no development needs", async () => {
    // build a package that depends on a workspace package and an npm one
    await using simulation = testSimulation();
    const { crypto } = simulation.environment("test");
    const authored = {
        name: "@acme/notes",
        version: "2026.11.3",
        exports: { "./package": "./src/package.ts" },
        dependencies: { "@destack/object": "workspace:*", zod: "4.1.0" },
        devDependencies: { "@destack/test": "workspace:*" },
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
    const zipped = await NpmTarball.archive(entries);
    const archive = Pipe.through(new Blob([zipped]).stream(), compression.decompress("gzip"));
    for await (const entry of Tarball.read(archive)) {
        read.push([entry.path, new TextDecoder().decode(await TarballEntry.bytes(entry))]);
    }
    const { devDependencies: _development, ...installed } = authored;
    const released = {
        ...installed,
        dependencies: { "@destack/object": "2026.11.2", zod: "4.1.0" },
    };
    expect(read).toEqual([
        ["package/package.json", `${JSON.stringify(released, null, 4)}\n`],
        ["package/src/package.ts", "export {};\n"],
    ]);

    // archive equal entries to equal bytes, hashed as npm's integrity
    const again = await NpmTarball.archive(
        await NpmTarball.entries(build, { "@destack/object": "2026.11.2" }),
    );
    expect([again.toHex() === zipped.toHex(), await NpmTarball.integrity(zipped, crypto)]).toEqual([
        true,
        `sha512-${new Uint8Array(await crypto.digest("SHA-512", zipped)).toBase64()}`,
    ]);

    // follow the default tag for a workspace dependency the lock does not link
    const unlinked = (await NpmTarball.entries(build, {})).find(
        (entry) => entry.path === NpmTarball.packageFile,
    );
    const followed: unknown = JSON.parse(
        new TextDecoder().decode(await TarballEntry.bytes(present(unlinked, "the package file"))),
    );
    expect(followed).toEqual({
        ...installed,
        dependencies: { "@destack/object": "latest", zod: "4.1.0" },
    });
});
