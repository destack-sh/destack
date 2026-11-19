import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, onTestFinished, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { Project } from "./project.ts";

/** The workspace's installed packages, which a scratch package links to resolve its types. */
const MODULES = fileURLToPath(new URL("../../../../node_modules/", import.meta.url));

/** Write a scratch package of files in the workspace, removed after the test. */
async function scratchPackage(sources: Readonly<Record<string, string>>): Promise<string> {
    // make the package's directory, linking the workspace's installed packages
    const { files } = testHost();
    const directory = await files.makeTemporaryDirectory("project-");
    onTestFinished(() => files.remove(directory, { recursive: true }));
    await files.symlink(MODULES, join(directory, "node_modules"));

    // write its manifest, definition, README and sources
    const manifest = {
        name: "@example/scratch",
        description: "Check a scratch package.",
        type: "module",
    };
    const definition = [
        "{",
        '    "$schema": "https://destack.app/schemas/2026.9.0/destack.json",',
        '    "id": "package-01996ab0-0000-7000-8000-000000000001",',
        '    "role": "library",',
        '    "language": "typescript",',
        '    "runtimes": ["bun"]',
        "}",
        "",
    ].join("\n");
    await files.writeTextFile(
        join(directory, "package.json"),
        `${JSON.stringify(manifest, null, 4)}\n`,
    );
    await files.writeTextFile(join(directory, "destack.json"), definition);
    await files.writeTextFile(
        join(directory, "README.md"),
        "# @example/scratch\n\nCheck a scratch package.\n",
    );
    for (const [path, source] of Object.entries(sources)) {
        await files.mkdir(join(directory, path, ".."), { recursive: true });
        await files.writeTextFile(join(directory, path), source);
    }

    return directory;
}

test("report a package's format, type and lint findings as one list of diagnostics", async () => {
    // check a package with an unformatted, mistyped and undocumented export
    const directory = await scratchPackage({
        "src/count.ts": "export const count: number =  'one'\n",
    });
    const project = await Project.open(directory, testHost());
    await project.configure();
    const { diagnostics } = await project.check();

    // expect the formatter's file, the compiler's error and the missing documentation
    expect(diagnostics.map((each) => [each.filename, each.code])).toEqual([
        ["src/count.ts", "format"],
        ["src/count.ts", "TS2322"],
        ["src/count.ts", "destack(require-jsdoc)"],
    ]);
});

test("format a package and leave the findings a person fixes", async () => {
    // fix a package with an unformatted, undocumented export
    const directory = await scratchPackage({ "src/count.ts": "export const count =  1\n" });
    const project = await Project.open(directory, testHost());
    const { diagnostics } = await project.fix();
    const { files } = testHost();

    // expect the source formatted, and the missing documentation left
    expect({
        source: await files.readTextFile(join(directory, "src/count.ts")),
        codes: diagnostics.map((each) => each.code),
    }).toEqual({ source: "export const count = 1;\n", codes: ["destack(require-jsdoc)"] });
});
