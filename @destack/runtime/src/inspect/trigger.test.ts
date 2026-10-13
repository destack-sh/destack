import { StandardSignature } from "../trigger/index.ts";
import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { defineTrigger } from "../trigger/index.ts";
import { triggerSymbols } from "./trigger.ts";

/** The package-qualified note type the calls call. */
const NOTE = {
    packageId: schema.identifier("package").parse("package-01996ab0-0000-7000-8000-000000000001"),
    type: "note",
};

test("relate a scheduled trigger to the object method it invokes, and other triggers to nothing", () => {
    // call the note type's archive method every hour
    const scheduled = defineTrigger({
        name: "digest",
        on: {
            schedule: {
                timing: { timing: "interval", interval: 3_600_000, startsAt: 0 },
                concurrency: "forbid",
                deadline: 60_000,
                call: { type: NOTE, method: "archive", input: {}, release: "2026.9.0" },
            },
        },
    });

    // receive signed deliveries instead
    const webhook = defineTrigger({
        name: "digest",
        on: {
            webhook: {
                signature: new StandardSignature(),
                path: "/{repository}",
                secret: async () => "secret",
            },
        },
        call: () => ({ type: NOTE, method: "archive", input: {}, release: "2026.9.0" }),
    });

    expect([triggerSymbols(scheduled), triggerSymbols(webhook)]).toEqual([
        [
            {
                relationships: [
                    {
                        kind: "invokes",
                        symbol: {
                            kind: "method",
                            name: "archive",
                            parent: { kind: "object", name: "note" },
                        },
                    },
                ],
            },
        ],
        [],
    ]);
});
