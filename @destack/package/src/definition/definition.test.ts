import { readFile, writeFile } from "node:fs/promises";
import { expect, test } from "@destack/test";
import { toJsonSchema } from "@destack/schema";
import { Definition, PackageDefinition } from "./definition.ts";

/** The published JSON Schema of destack.json, served per release. */
const SCHEMA_FILE = new URL("../../schemas/destack.json", import.meta.url);

test("publish the JSON Schema of destack.json as the package and workspace definitions declare it", async () => {
    // rewrite the file on request and compare it with the declaration
    const described = `${JSON.stringify(toJsonSchema(Definition), null, 4)}\n`;
    if (process.env["UPDATE_SCHEMAS"] === "1") {
        await writeFile(SCHEMA_FILE, described);
    }

    expect(await readFile(SCHEMA_FILE, "utf8")).toEqual(described);
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
