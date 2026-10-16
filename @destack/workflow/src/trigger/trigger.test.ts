import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { describeWorkflow } from "../inspect/index.ts";
import { defineWorkflow } from "../workflow/workflow.ts";
import { MAX_LAG_MILLISECONDS } from "./change.ts";
import { Trigger } from "./trigger.ts";

/** The note type a change trigger follows, by its package and name. */
const note = { package: import.meta.destack.package, name: "note" };

/** Summarise a note each morning and as notes are published. */
const summarize = defineWorkflow({
    name: "summarize",
    input: schema.object({ id: schema.string() }),
    triggers: {
        daily: {
            kind: "schedule",
            schedule: {
                timing: { timing: "cron", cron: "0 9 * * *", timezone: "UTC" },
                concurrency: "forbid",
                deadline: 60_000,
            },
            input: (occurrence) => ({ id: String(occurrence.at) }),
        },
        published: {
            kind: "change",
            change: {
                object: note,
                where: { status: "published" },
                operations: ["create", "update"],
            },
            input: (change) => ({ id: schema.string().parse(change.after?.["id"]) }),
        },
    },
});

test("describe a workflow's schedule and change triggers with what fires them", () => {
    expect(describeWorkflow(summarize).triggers).toEqual([
        {
            name: "daily",
            kind: "schedule",
            schedule: {
                timing: { timing: "cron", cron: "0 9 * * *", timezone: "UTC" },
                concurrency: "forbid",
                deadline: 60_000,
            },
        },
        {
            name: "published",
            kind: "change",
            change: {
                object: { packageId: note.package.id, name: "note" },
                where: { status: "published" },
                operations: ["create", "update"],
                from: "now",
                maxLag: MAX_LAG_MILLISECONDS,
            },
        },
    ]);
});

test("refuse a change trigger starting with a snapshot of created rows that does not fire on creations", () => {
    expect(() =>
        Trigger.of("removed", {
            kind: "change",
            change: { object: note, operations: ["delete"], from: "snapshot" },
            input: () => ({ id: "" }),
        }),
    ).toThrow(
        "trigger removed starts with a snapshot of created rows but does not fire on creations",
    );
});

test("key each event, ordering changes as the log orders them", () => {
    // key an occurrence by its time, a delivery by its parameters and sender, and a change by its position and row
    const delivery = {
        id: "d-1",
        event: "push",
        payload: null,
        parameters: { repository: "a" },
        receivedAt: 1,
    };
    expect([
        Trigger.key({ at: 1_700_000_000_000 }),
        Trigger.key(delivery),
        changeKey("e1", 42, "note-1"),
    ]).toEqual(["0001700000000000", '{"repository":"a"} d-1', "e1/0000000000000042/note-1"]);

    // order changes by epoch, by sequence across digit counts and by snapshot row
    const ordered = [
        changeKey("e1", 9),
        changeKey("e1", 10),
        changeKey("e1", 10, "a"),
        changeKey("e2", 1),
    ];
    expect(ordered.toReversed().toSorted()).toEqual(ordered);
});

/** Key a created change at a log position, at a snapshot row when given. */
function changeKey(epoch: string, sequence: number, id?: string): string {
    return Trigger.key({
        position: { epoch, sequence },
        operation: "create",
        ...(id === undefined ? {} : { key: { id } }),
    });
}
