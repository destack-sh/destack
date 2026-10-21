import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Task, defineWorkflow } from "../src/index.ts";
import { alice, note, openServed, START } from "./fixture.ts";

/** The notes package's tasks. */
const task = defineWorkflow({ name: "task", input: Task.input, output: Task.output });

test("make a task's call once at its time as its actor, answering the call's result", async () => {
    // start a note's creation as alice to run in a minute
    const fixture = await openServed(
        task,
        task.implement((step, input) => Task.run(step, input)),
    );
    await fixture.start((spaceId) =>
        task.start(
            { call: note.call("create", { spaceId, title: "later" }) },
            { at: START + 60_000, actor: alice },
        ),
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
