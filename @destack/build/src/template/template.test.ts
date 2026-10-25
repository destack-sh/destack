import { testSimulation } from "../test/world.ts";
import type { TemplateInput } from "@destack/package/template";
import { test } from "@destack/test";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { Template } from "./template.ts";
import { PackageId } from "@destack/package";
import { expectDirectory } from "../../tests/fixture.ts";

test.each(["stack", "blank"])("instantiate the %s package template", async (name) => {
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await files.makeTemporaryDirectory("template-");
    const template = await Template.read(
        fileURLToPath(new URL(`../../../../@template/${name}/`, import.meta.url)),
        files,
    );
    const parameters: TemplateInput = {
        id: PackageId.parse("package-01996ab0-0000-7000-8000-000000000005"),
        name: `@example/${name}`,
        dependencies:
            name === "blank"
                ? {
                      "@template/stack": {
                          id: PackageId.parse("package-01996ab0-0000-7000-8000-000000000006"),
                          name: "@example/stack",
                          version: "2026.9.0",
                      },
                  }
                : {},
    };

    try {
        // instantiate the package into a new directory
        const destination = join(directory, name);
        await template.write(destination, parameters, files);

        // compare every written file against the fixture
        const entries = await files.readDirectory(destination, { recursive: true });
        const written = new Map<string, Uint8Array>();
        for (const entry of entries.filter((each) => each.kind === "file")) {
            written.set(entry.name, await files.readFile(join(destination, entry.name)));
        }
        await expectDirectory(
            written,
            new URL(`../../tests/fixture/template/${name}/expected/`, import.meta.url),
            environment,
        );
    } finally {
        await files.remove(directory, { recursive: true });
    }
});
