import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { renameDocument } from "./storage.ts";
import { auditActionVocabulary, describeAuditAction } from "../src/inspect/index.ts";

test("list the term an action fixes in recorded calls, with the shapes its calls record", () => {
    const described = schema
        .record(schema.string(), schema.json())
        .parse(JSON.parse(JSON.stringify(describeAuditAction(renameDocument))));

    expect(auditActionVocabulary(described)).toEqual({
        "document.rename": { targets: described["targets"], details: described["details"] },
    });
});
