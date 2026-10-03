import { readFile, writeFile } from "node:fs/promises";
import { expect, test } from "@destack/test";
import { toJsonSchema } from "@destack/schema";
import { Definition, PackageDefinition } from "./definition.ts";
import { PackageError } from "../error/error.ts";

/** The published JSON Schema of destack.json, served per release. */
const SCHEMA_FILE = new URL("../../schemas/destack.json", import.meta.url);

test("publish the JSON Schema of destack.json as the package and workspace definitions declare it", async () => {
    // rewrite the file on request, then compare it with the declaration
    const described = `${JSON.stringify(toJsonSchema(Definition), null, 4)}\n`;
    if (process.env["UPDATE_SCHEMAS"] === "1") {
        await writeFile(SCHEMA_FILE, described);
    }

    expect(await readFile(SCHEMA_FILE, "utf8")).toEqual(described);
});

/** The fields every destack.json carries. */
const REQUIRED = {
    $schema: "https://destack.app/schemas/2026.10.0/destack.json",
    id: "package-01996ab0-0000-7000-8000-000000000001",
    language: "typescript",
};

test("list an export's runtimes, its own over the package's, and refuse an export with none", () => {
    // read the package's runtimes, and an export's override of them
    const definition = PackageDefinition.parse({
        ...REQUIRED,
        runtimes: ["workerd"],
        exports: { "./browser": { runtimes: ["browser"] } },
    });
    expect([
        PackageDefinition.runtimes(definition, "."),
        PackageDefinition.runtimes(definition, "./browser"),
    ]).toEqual([["workerd"], ["browser"]]);

    // refuse an export of a package declaring no runtimes
    expect(() => PackageDefinition.runtimes(PackageDefinition.parse(REQUIRED), "./server")).toThrow(
        new PackageError("INVALID_DEFINITION", "no runtimes declared for export: ./server"),
    );
});
