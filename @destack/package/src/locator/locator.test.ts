import { testSimulation } from "@destack/test/world";
import type { FileSystem } from "@destack/environment";
import { join } from "node:path";
import { expect, test } from "@destack/test";
import { PackageLocator } from "./locator.ts";

/** The schema every package definition references. */
const SCHEMA = "https://destack.app/schemas/2026.9.0/destack.json";

/** Write a Destack package declaring a version, with one module, into a directory. */
async function writePackage(
    files: FileSystem,
    directory: string,
    name: string,
    id: string,
): Promise<void> {
    await files.mkdir(join(directory, "src"), { recursive: true });
    await files.writeTextFile(
        join(directory, "package.json"),
        JSON.stringify({ name, version: "2026.9.0" }),
    );
    await files.writeTextFile(
        join(directory, "destack.json"),
        JSON.stringify({ $schema: SCHEMA, id, role: "library", language: "typescript" }),
    );
    await files.writeTextFile(join(directory, "src", "index.ts"), "export {};\n");
}

test("read listed source packages at their given versions and other packages at their declared ones", async () => {
    // write a listed and an unlisted source package, and an installation of the listed name
    await using simulation = testSimulation();
    const { files } = simulation.environment("test");
    const root = await files.makeTemporaryDirectory("locator-");
    const packages = [
        { directory: join(root, "notes"), name: "@example/notes" },
        { directory: join(root, "tasks"), name: "@example/tasks" },
        {
            directory: join(root, "notes", "node_modules", "@example", "notes"),
            name: "@example/notes",
        },
    ];
    for (const [index, { directory, name }] of packages.entries()) {
        await writePackage(
            files,
            directory,
            name,
            `package-01996ab0-0000-7000-8000-00000000000${index + 1}`,
        );
    }
    // find each with a dev version for the listed package, and without versions
    const versions = { "@example/notes": "2026.9.0-dev.20260912100004.0123456789ab" };
    const read = async (locator: PackageLocator) =>
        await Promise.all(
            packages.map(async ({ directory }) => {
                const found = await locator.find(join(directory, "src", "index.ts"));

                return found?.metadata.package.version;
            }),
        );

    expect({
        listed: await read(new PackageLocator(files, versions)),
        declared: await read(new PackageLocator(files)),
    }).toEqual({
        listed: ["2026.9.0-dev.20260912100004.0123456789ab", "2026.9.0", "2026.9.0"],
        declared: ["2026.9.0", "2026.9.0", "2026.9.0"],
    });
});

test("find no package for a host's scheme-named modules, and find an npm package Bun also supplies", async () => {
    // install npm packages named bun and ws, and another package, above a module's directory
    await using simulation = testSimulation();
    const { files } = simulation.environment("test");
    const root = await files.makeTemporaryDirectory("locator-");
    for (const name of ["bun", "ws", "left-pad"]) {
        const installed = join(root, "node_modules", name);
        await files.mkdir(installed, { recursive: true });
        await files.writeTextFile(
            join(installed, "package.json"),
            JSON.stringify({ name, version: "1.0.0" }),
        );
    }
    const from = join(root, "src");
    await files.mkdir(from, { recursive: true });

    // look each up: scheme-named modules and bun resolve to none, ws to its installed package
    const locator = new PackageLocator(files);
    const names = ["node:path", "bun:sqlite", "bun", "ws", "left-pad"];
    const found = await Promise.all(names.map((name) => locator.directory(name, from)));

    expect(found).toEqual([
        undefined,
        undefined,
        undefined,
        await files.realPath(join(root, "node_modules", "ws")),
        await files.realPath(join(root, "node_modules", "left-pad")),
    ]);
});
