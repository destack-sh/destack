import { mkdir, mkdtemp, realpath, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { expect, test } from "@destack/test";
import { PackageLocator } from "./locator.ts";

/** The schema every package definition references. */
const SCHEMA = "https://destack.app/schemas/2026.9.0/destack.json";

/** Write a Destack package declaring a version, with one module, into a directory. */
async function writePackage(directory: string, name: string, id: string): Promise<void> {
    await mkdir(join(directory, "src"), { recursive: true });
    await writeFile(join(directory, "package.json"), JSON.stringify({ name, version: "2026.9.0" }));
    await writeFile(
        join(directory, "destack.json"),
        JSON.stringify({ $schema: SCHEMA, id, language: "typescript" }),
    );
    await writeFile(join(directory, "src", "index.ts"), "export {};\n");
}

test("read listed source packages at their given versions and other packages at their declared ones", async () => {
    // write a listed and an unlisted source package, and an installation of the listed name
    const root = await realpath(await mkdtemp(join(tmpdir(), "destack-locator-")));
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
            directory,
            name,
            `package-01996ab0-0000-7000-8000-00000000000${index + 1}`,
        );
    }
    try {
        // find each with a dev version for the listed package, and without versions
        const versions = { "@example/notes": "2026.9.0-dev.4" };
        const read = async (locator: PackageLocator) =>
            await Promise.all(
                packages.map(async ({ directory }) => {
                    const found = await locator.find(join(directory, "src", "index.ts"));

                    return found?.metadata.package.version;
                }),
            );

        expect({
            listed: await read(new PackageLocator(versions)),
            declared: await read(new PackageLocator()),
        }).toEqual({
            listed: ["2026.9.0-dev.4", "2026.9.0", "2026.9.0"],
            declared: ["2026.9.0", "2026.9.0", "2026.9.0"],
        });
    } finally {
        await rm(root, { recursive: true });
    }
});
