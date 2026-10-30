import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Task, TaskInput, TaskOutput, defineWorkflow } from "../src/index.ts";
import { alice, note, openServed } from "./workflow.ts";
import { TEST_START } from "@destack/object/test";
import { NOTES } from "./package.ts";

/** The notes package's tasks. */
const task = defineWorkflow({ name: "task", input: TaskInput, output: TaskOutput }, NOTES);

test("make a task's call once at its time as its actor, answering the call's result", async () => {
    // start a note's creation as alice to run in a minute
    const fixture = await openServed(
        task,
        task.implement((step, input) => new Task(step, input).run()),
    );
    await fixture.start((spaceId) =>
        task.start(
            { call: note.call("create", { spaceId, title: "later" }) },
            { at: TEST_START + 60_000, actor: alice },
        ),
    );
    const waiting = await fixture.until(() => true);

    // run it once its time comes, answering the created note
    const [run] = await fixture.until((rows) => rows[0]?.["state"] === "completed");
    const notes = await fixture.served.database
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
