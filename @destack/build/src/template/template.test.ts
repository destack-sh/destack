import { testSimulation } from "../test/world.ts";
import type { TemplateInput } from "@destack/package/template";
import { expect, test } from "@destack/test";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { Toolchain } from "@destack/check/toolchain";
import { replaceTitle, Template } from "./template.ts";
import { PackageId } from "@destack/package";
import { expectDirectory } from "../../tests/fixture.ts";

/** The packages the instances depend on at a released version, the others at the default tag. */
const RELEASED = ["@destack/object", "@destack/setting"];

test("instantiate the template package", async () => {
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await files.makeTemporaryDirectory("template-");
    const template = await Template.read(
        fileURLToPath(new URL("../../../template/", import.meta.url)),
        files,
    );
    const parameters: TemplateInput = {
        id: PackageId.parse("package-01996ab0-0000-7000-8000-000000000005"),
        name: "@example/notes",
        dependencies: {},
        versions: Object.fromEntries(RELEASED.map((each) => [each, "2026.9.0"])),
    };

    try {
        // instantiate the package into a new directory
        const destination = join(directory, "notes");
        await template.write(destination, parameters, Toolchain.workspace(), files);

        // compare every written file against the fixture
        const entries = await files.readDirectory(destination, { recursive: true });
        const written = new Map<string, Uint8Array>();
        for (const entry of entries.filter((each) => each.kind === "file")) {
            written.set(entry.name, await files.readFile(join(destination, entry.name)));
        }
        await expectDirectory(
            written,
            new URL("../../tests/fixture/template/notes/expected/", import.meta.url),
            environment,
        );
    } finally {
        await files.remove(directory, { recursive: true });
    }
});

/** Replace a module's title with Notes, reading a refusal's message. */
function refused(source: string): string {
    try {
        return replaceTitle("src/package.ts", source, "Notes");
    } catch (error) {
        return error instanceof Error ? error.message : String(error);
    }
}

test("replace a definition's literal title, refusing a module without exactly one", () => {
    // replace a title, escaping a backtick, and refuse a module without a literal title
    const defined = "export default definePackage({\n    title: t`Template`,\n});\n";

    // expect the title replaced in place, and each module without one literal title refused
    const message =
        "template source 'src/package.ts' is invalid: it must define its package with one literal title";
    expect([
        replaceTitle("src/package.ts", defined, "Notes `beta`"),
        refused("export default definePackage({ title: t`A ${name}` });\n"),
        refused("export const value = 1;\n"),
    ]).toEqual([
        "export default definePackage({\n    title: t`Notes \\`beta\\``,\n});\n",
        message,
        message,
    ]);
});
