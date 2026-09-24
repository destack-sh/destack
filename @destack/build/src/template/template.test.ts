import type { TemplateParameters } from "@destack/package/template";
import { test } from "@destack/test";
import { mkdtemp, readdir, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { Template } from "../template/index.ts";
import { PackageId } from "@destack/package";
import { expectDirectory } from "../../tests/fixture.ts";

test.each(["stack", "blank"])("instantiate the %s package template", async (name) => {
    const directory = await mkdtemp(join(tmpdir(), "destack-template-"));
    const template = await Template.read(
        fileURLToPath(new URL(`../../../template-${name}/`, import.meta.url)),
    );
    const parameters: TemplateParameters = {
        id: PackageId.parse("package-01996ab0-0000-7000-8000-000000000005"),
        name: `@example/${name}`,
        dependencies:
            name === "blank"
                ? {
                      "@destack/template-stack": {
                          id: PackageId.parse("package-01996ab0-0000-7000-8000-000000000006"),
                          name: "@example/stack",
                          version: "1.0.0",
                      },
                  }
                : {},
    };

    try {
        // instantiate the package into a new directory
        const destination = join(directory, name);
        await template.write(destination, parameters);

        // compare every written file against the fixture
        const paths = await readdir(destination, { recursive: true, withFileTypes: true });
        const written = new Map<string, Uint8Array>();
        for (const path of paths) {
            if (!path.isFile()) {
                continue;
            }
            const absolute = join(path.parentPath, path.name);
            written.set(absolute.slice(destination.length + 1), await readFile(absolute));
        }
        await expectDirectory(
            written,
            new URL(`../../tests/fixture/template/${name}/expected/`, import.meta.url),
        );
    } finally {
        await rm(directory, { recursive: true });
    }
});
