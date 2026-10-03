import { expect, test } from "@destack/test";
import { ModuleMetadata } from "../definition/metadata.ts";
import { Package } from "../definition/package.ts";
import { PackageError } from "../error/error.ts";
import { definePackage } from "./package.ts";
import type { Declaration } from "./declaration.ts";

/** The package the module transform stamps. */
const notes = Package.parse({
    id: "package-01996ab0-0000-7000-8000-000000000001",
    name: "@example/notes",
    version: "2026.10.0",
});

/** The metadata of a module of the package. */
const module = ModuleMetadata.parse({ package: notes });

/** Declare a named declaration of the package. */
function declared(name: string): Declaration & { state(): Record<string, never> } {
    return { package: notes, name, state: () => ({}) };
}

test("key a package's declarations by their own names, once across resources and secrets", () => {
    // stamp the handle with the package and its declarations
    const handle = definePackage(
        { resources: { main: declared("main") }, secrets: { token: declared("token") } },
        module,
    );
    expect([handle.name, Object.keys(handle.resources), Object.keys(handle.secrets)]).toEqual([
        "@example/notes",
        ["main"],
        ["token"],
    ]);

    // refuse a declaration listed under another name, and a name both kinds use
    expect(() => definePackage({ resources: { main: declared("other") } }, module)).toThrow(
        new PackageError("INVALID_DEFINITION", "declaration other is listed as main"),
    );
    expect(() =>
        definePackage(
            { resources: { main: declared("main") }, secrets: { main: declared("main") } },
            module,
        ),
    ).toThrow(new PackageError("INVALID_DEFINITION", "duplicate declaration: main"));
});
