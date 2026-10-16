import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { ScheduledCall, defineWorkflow } from "../src/index.ts";
import { alice, note, openServed, START } from "./fixture.ts";

/** The notes package's scheduled calls. */
const calls = defineWorkflow({
    name: "call",
    input: ScheduledCall.input,
    output: ScheduledCall.output,
});

test("make a scheduled call once at its time as its actor, answering the call's result", async () => {
    // send a note's creation as alice to run in a minute
    const fixture = await openServed(calls, calls.implement(ScheduledCall.run));
    await fixture.start((spaceId) =>
        ScheduledCall.send(calls, note.call("create", { spaceId, title: "later" }), {
            at: START + 60_000,
            actor: alice,
        }),
    );
    const waiting = await fixture.until(() => true);

    // run it once its time comes, answering the created note
    const [run] = await fixture.until((rows) => rows[0]?.["state"] === "completed");
    const notes = await fixture.store.database
        .select({ title: note.table.title, ownerId: note.table.ownerId })
        .from(note.table);
    expect({
        waiting: waiting.map((row) => row["state"]),
        notes,
        title: schema.looseObject({ title: schema.string() }).parse(run?.["output"]).title,
    }).toEqual({
        waiting: ["pending"],
        notes: [{ title: "later", ownerId: alice.id }],
        title: "later",
    });
});
