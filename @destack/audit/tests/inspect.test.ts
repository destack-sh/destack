import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { renameDocument } from "./storage.ts";
import { auditActionVocabulary, describeAuditAction } from "../src/inspect/index.ts";

/** The JSON Schema dialect every described shape names. */
const DIALECT = "https://json-schema.org/draft/2020-12/schema";

/** The JSON Schema of the renamed document target. */
const targets = {
    $schema: DIALECT,
    type: "object",
    properties: {
        document: {
            type: "object",
            properties: { type: { type: "string", const: "document" }, id: { type: "string" } },
            required: ["type", "id"],
            additionalProperties: false,
        },
    },
    required: ["document"],
    additionalProperties: false,
};

/** The JSON Schema of the rename details. */
const details = {
    $schema: DIALECT,
    type: "object",
    properties: { name: { type: "string" } },
    required: ["name"],
    additionalProperties: false,
};

test("describe an action with its package and call shapes, and list its term from the description", () => {
    // describe the action, then read its term from the serialized description
    const described = describeAuditAction(renameDocument);
    const serialized = schema
        .record(schema.string(), schema.json())
        .parse(JSON.parse(JSON.stringify(described)));

    expect({ described, vocabulary: auditActionVocabulary(serialized) }).toEqual({
        described: {
            name: "document.rename",
            package: {
                id: "package-01996ab0-0000-7000-8000-000000000004",
                name: "@example/document",
                version: "2026.9.0",
            },
            targets,
            details,
        },
        vocabulary: { "document.rename": { targets, details } },
    });
});
