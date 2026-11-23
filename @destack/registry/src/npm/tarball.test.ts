import { TestBuildStore } from "@destack/build/test";
import { Pipe } from "@destack/environment";
import { Tarball, TarballEntry } from "@destack/package/archive";
import { BuildImage } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import { PackageVersion } from "@destack/package";
import { expect, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";
import { NpmTarball } from "./tarball.ts";

/** The app the fixture build releases. */
const NOTES = PackageVersion.parse({
    id: "package-019f5530-8000-7000-8000-0000000000a1",
    name: "@example/notes",
    version: "2026.11.3",
});

/** The library the fixture build links. */
const OBJECT = PackageVersion.parse({
    id: "package-019f5530-8000-7000-8000-0000000000a2",
    name: "@destack/object",
    version: "2026.11.2",
});

/** Encode text as a file's bytes. */
function bytes(text: string): Uint8Array<ArrayBuffer> {
    return new TextEncoder().encode(text);
}

test("pack an image's root build below package/ in path order, with what a listing says of it", async () => {
    // store a package that depends on a workspace package and an npm one, in an image with the first
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { crypto, compression } = environment;
    await using store = await TestBuildStore.create(environment);
    const authored = {
        name: "@example/notes",
        version: "2026.11.3",
        exports: { "./package": "./src/package.ts" },
        dependencies: { "@destack/object": "workspace:*", zod: "4.1.0" },
        devDependencies: { "@destack/test": "workspace:*" },
    };
    const root = await store.put(
        await MemoryBuild.write(
            new Map([
                ["src/package.ts", bytes("export {};\n")],
                ["package.json", bytes(`${JSON.stringify(authored)}\n`)],
            ]),
            { package: NOTES, role: "app" },
            crypto,
        ),
    );
    const linked = await store.put(
        await MemoryBuild.write(
            new Map([["package.json", bytes("{}\n")]]),
            { package: OBJECT, role: "library" },
            crypto,
        ),
    );
    const image = await store.putImage(
        BuildImage.of(root, { [root]: { "@destack/object": linked }, [linked]: {} }),
    );

    // pack it twice, and read the archive back
    const packed = await NpmTarball.pack(store, image, environment);
    const again = await NpmTarball.pack(store, image, environment);
    const read: [string, string][] = [];
    const archive = Pipe.through(new Blob([packed.bytes]).stream(), compression.decompress("gzip"));
    for await (const entry of Tarball.read(archive)) {
        read.push([entry.path, new TextDecoder().decode(await TarballEntry.bytes(entry))]);
    }

    // release the package file at the linked version, the default tag where the image links none
    const released = {
        ...authored,
        dependencies: { "@destack/object": "2026.11.2", zod: "4.1.0" },
        devDependencies: { "@destack/test": "latest" },
    };
    expect({
        read,
        listed: packed.listed,
        isEqual: again.listed.integrity === packed.listed.integrity,
    }).toEqual({
        read: [
            ["package/package.json", `${JSON.stringify(released, null, 4)}\n`],
            ["package/src/package.ts", "export {};\n"],
        ],
        listed: {
            manifest: { dependencies: released.dependencies },
            integrity: `sha512-${new Uint8Array(await crypto.digest("SHA-512", packed.bytes)).toBase64()}`,
        },
        isEqual: true,
    });
});
