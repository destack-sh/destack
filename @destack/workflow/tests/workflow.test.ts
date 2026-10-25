import { schema } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { expect, test } from "@destack/test";
import { defineWorkflow } from "../src/index.ts";
import { actor, alice, note, notesPackage, openWorkflow, type WorkflowFixture } from "./fixture.ts";
import { NOTES } from "./package.ts";

/** Write a draft, read it back, pause, then wait an hour for its approval note. */
const draft = defineWorkflow(
    {
        name: "draft",
        input: schema.object({ title: schema.string().min(1) }),
        output: schema.object({ draft: schema.string(), approval: schema.string().nullable() }),
        waits: [note],
    },
    NOTES,
);

test("advance a run step by step as its actor: call and read once, sleep, wait for an object, and complete", async () => {
    // write a note, read it, count its title, sleep five minutes and wait an hour for its approval
    const fixture: WorkflowFixture = await openWorkflow(draft, async (step, input) => {
        const spaceId = fixture.served.scope;
        const written = await step.call(
            "write",
            note.call("create", { spaceId, title: input.title }),
        );
        const read = await step.query("read", note.call("get", { spaceId, id: written.id }));
        const counted = await step.run("count", schema.number(), async () => read.title.length);
        await step.sleep("pause", { minutes: 5 });
        const approval = await step.waitFor(
            "approval",
            note.call("list", { spaceId, where: { title: `Approved: ${input.title}` }, limit: 1 }),
            { timeout: { hours: 1 } },
        );

        return { draft: `${read.title} (${counted})`, approval: approval?.title ?? null };
    });

    // start a run as alice, wake it after the sleep, and write the approval note, which wakes it at once
    await fixture.start(() => draft.start({ title: "Plan" }, actor()));
    await fixture.until((rows) => rows[0]?.["state"] === "waiting" && rows[0]["waitType"] === null);
    await fixture.simulation.jump(fixture.simulation.clock.now() + 5 * 60_000);
    await fixture.until(
        (rows) => rows[0]?.["waitType"] !== null && rows[0]?.["waitType"] !== undefined,
    );
    await fixture.served
        .as(alice)
        .client(notesPackage)
        .note.create({
            spaceId: fixture.served.scope,
            requestId: RequestId.create(fixture.environment),
            title: "Approved: Plan",
        });
    const [run] = await fixture.until((rows) => rows[0]?.["state"] === "completed");

    // expect one draft note, and the run completed with both notes' titles
    const titles = (
        await fixture.served.database.select({ title: note.table.title }).from(note.table)
    )
        .map((row) => row.title)
        .toSorted();
    expect({ titles, output: run?.["output"], failure: run?.["failure"] }).toEqual({
        titles: ["Approved: Plan", "Plan"],
        output: { approval: "Approved: Plan", draft: "Plan (4)" },
        failure: null,
    });
});
