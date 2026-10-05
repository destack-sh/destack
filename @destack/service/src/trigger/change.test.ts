import { expect, test } from "@destack/test";
import { describeTrigger } from "../inspect/index.ts";
import { defineService } from "../declare/service.ts";
import { MAX_LAG_MILLISECONDS } from "./change.ts";
import { defineTrigger, RunEvent } from "./trigger.ts";

/** Summarize the changed note. */
function summarize() {
    return { method: "note.summarize", input: {}, release: "2026.9.0" };
}

/** Derive the request of a change at a position, at a snapshot row when keyed. */
function changeRequest(epoch: string, sequence: number, key?: string): string {
    return RunEvent.requestId({
        change: { position: { epoch, sequence }, ...(key === undefined ? {} : { key }) },
    });
}

test("describe a declared change trigger with its object reference, condition, operations and start", () => {
    const note = { package: defineService("notes", {}).package, name: "note" };
    const trigger = defineTrigger({
        name: "published",
        on: {
            change: {
                object: note,
                where: { status: "published" },
                operations: ["create", "update"],
            },
        },
        call: summarize,
    });

    expect(describeTrigger(trigger)).toEqual({
        name: "published",
        on: {
            change: {
                object: { packageId: note.package.id, name: "note" },
                where: { status: "published" },
                operations: ["create", "update"],
                from: "now",
                maxLag: MAX_LAG_MILLISECONDS,
            },
        },
    });

    // refuse a snapshot for a trigger without creations
    expect(() =>
        defineTrigger({
            name: "removed",
            on: { change: { object: note, operations: ["delete"], from: "snapshot" } },
            call: summarize,
        }),
    ).toThrow(
        "trigger removed starts with a snapshot, whose rows are created, but fires on no creations",
    );
});

test("derive each event's request, ordering changes as the log orders them", () => {
    // key occurrences by time, deliveries by path and sender, and changes by position and row
    expect([
        RunEvent.requestId({ at: 1_700_000_000_000 }),
        RunEvent.requestId({ delivery: { id: "d-1", path: "/notes" } }),
        changeRequest("e1", 42, "note-1"),
    ]).toEqual(["0001700000000000", "/notes d-1", "e1/0000000000000042/note-1"]);

    // order changes by epoch, by sequence across digit counts and by snapshot row
    const ordered = [
        changeRequest("e1", 9),
        changeRequest("e1", 10),
        changeRequest("e1", 10, "a"),
        changeRequest("e2", 1),
    ];
    expect(ordered.toReversed().toSorted()).toEqual(ordered);
});
