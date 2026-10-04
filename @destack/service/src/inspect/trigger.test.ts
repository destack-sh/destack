import { expect, test } from "@destack/test";
import type { JsonValue } from "@destack/schema";
import { triggerSymbols } from "./trigger.ts";

/** Describe a trigger firing on an event, as a manifest lists it. */
const description = (on: JsonValue) => ({ name: "digest", on });

test("relate a scheduled trigger to the object method it invokes, and other triggers to nothing", () => {
    // call the note type's archive method every hour
    const scheduled = description({
        schedule: {
            timing: { timing: "interval", interval: 3_600_000, startsAt: 0 },
            concurrency: "forbid",
            deadline: 60_000,
            call: { method: "note.archive", input: {}, release: "2026.9.0" },
        },
    });

    // receive signed deliveries instead
    const webhook = description({
        webhook: { verification: "github", route: "/{repository}" },
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
    const scheduled = description({
        schedule: {
            timing: { timing: "once", startsAt: 0 },
            concurrency: "allow",
            deadline: 0,
            call: { method: "archive", input: {}, release: "2026.9.0" },
        },
    });

    expect(() => triggerSymbols(scheduled)).toThrow(
        new TypeError("trigger call names no object type: archive"),
    );
});
