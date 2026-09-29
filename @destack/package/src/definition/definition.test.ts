import { readFile, writeFile } from "node:fs/promises";
import { expect, test } from "vitest";
import { toJsonSchema } from "@destack/schema";
import { PackageDefinition } from "./definition.ts";

/** The published JSON Schema of destack.json, served per release. */
const SCHEMA_FILE = new URL("../../schemas/destack.json", import.meta.url);

test("publish the JSON Schema of destack.json as the definition declares it", async () => {
    // rewrite the file on request, then compare it with the declaration
    const described = `${JSON.stringify(toJsonSchema(PackageDefinition), null, 4)}\n`;
    if (process.env.UPDATE_SCHEMAS === "1") {
        await writeFile(SCHEMA_FILE, described);
    }

    expect(await readFile(SCHEMA_FILE, "utf8")).toEqual(described);
});
