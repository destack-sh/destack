import { expect, test } from "@destack/test";
import { GitHubSignature } from "../github/index.ts";
import { defineTrigger } from "../trigger/index.ts";
import { triggerSymbols } from "./trigger.ts";

test("relate a scheduled trigger to the object method it invokes, and other triggers to nothing", () => {
    // call the note type's archive method every hour
    const scheduled = defineTrigger({
        name: "digest",
        on: {
            schedule: {
                timing: { timing: "interval", interval: 3_600_000, startsAt: 0 },
                concurrency: "forbid",
                deadline: 60_000,
                call: { method: "note.archive", input: {}, release: "2026.9.0" },
            },
        },
    });

    // receive signed deliveries instead
    const webhook = defineTrigger({
        name: "digest",
        on: {
            webhook: {
                signature: new GitHubSignature(),
                path: "/{repository}",
                secret: async () => "secret",
            },
        },
        call: () => ({ method: "note.archive", input: {}, release: "2026.9.0" }),
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

test("refuse a scheduled call naming no object type", () => {
    const scheduled = defineTrigger({
        name: "digest",
        on: {
            schedule: {
                timing: { timing: "once", startsAt: 0 },
                concurrency: "allow",
                deadline: 0,
                call: { method: "archive", input: {}, release: "2026.9.0" },
            },
        },
    });

    expect(() => triggerSymbols(scheduled)).toThrow(
        new TypeError("trigger call names no object type: archive"),
    );
});
