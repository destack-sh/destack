import { fileURLToPath } from "node:url";
import { expect, test } from "@destack/test";
import { toJsonSchema } from "@destack/schema";
import { Definition, PackageDefinition } from "./definition.ts";

/** The published JSON Schema of destack.json, served per release. */
const SCHEMA_FILE = fileURLToPath(new URL("../../schemas/destack.json", import.meta.url));

test("publish the JSON Schema of destack.json as the package and workspace definitions declare it", async () => {
    // compare the checkout's file with the declaration, which vitest's update rewrites
    const described = `${JSON.stringify(toJsonSchema(Definition), null, 4)}\n`;
    await expect(described).toMatchFileSnapshot(SCHEMA_FILE);
});

test("read the packages whose build extensions compile a package, refusing an empty list", () => {
    // read a definition listing the style extension, then one listing none
    expect([
        PackageDefinition.read(definition(["@destack/style"])).extensions,
        (() => {
            try {
                return PackageDefinition.read(definition([]));
            } catch (error) {
                return error instanceof Error ? error.name : error;
            }
        })(),
    ]).toEqual([["@destack/style"], "ZodError"]);
});

test("read the modules a host starts, refusing an empty list and a path outside the package", () => {
    // read a definition listing a Worker script, then one listing none and one escaping the package
    const read = (entrypoints: readonly string[]) => {
        try {
            return PackageDefinition.read(
                JSON.stringify({ ...JSON.parse(definition(["@destack/style"])), entrypoints }),
            ).entrypoints;
        } catch (error) {
            return error instanceof Error ? error.name : error;
        }
    };

    expect([read(["src/worker.ts"]), read([]), read(["../outside.ts"])]).toEqual([
        ["src/worker.ts"],
        "ZodError",
        "ZodError",
    ]);
});

/** Write a destack.json listing extensions. */
function definition(extensions: readonly string[]): string {
    return JSON.stringify({
        $schema: "https://destack.app/schemas/2026.9.0/destack.json",
        id: "package-01996ab0-0000-7000-8000-0000000000a1",
        language: "typescript",
        runtimes: ["browser"],
        extensions,
    });
}
