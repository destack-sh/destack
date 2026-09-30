import { expect, test } from "@destack/test";
import { renameDocument } from "./storage.ts";
import { auditActionVocabulary, describeAuditAction } from "../src/inspect/index.ts";

test("list the term an action fixes in recorded events, with the shapes its events record", () => {
    const described = JSON.parse(JSON.stringify(describeAuditAction(renameDocument)));

    expect(auditActionVocabulary(described)).toEqual({
        "Document.rename": { targets: described.targets, details: described.details },
    });
});
