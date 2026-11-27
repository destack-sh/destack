import { principal } from "@destack/access";
import { Failure } from "@destack/error";
import { schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { expect, refusal, test } from "@destack/test";
import { t } from "@destack/locale";
import { defineWorkflow } from "../src/index.ts";
import { actor, alice, bob, note, openWorkflow } from "./workflow.ts";
import { NOTES } from "./package.ts";

/** The machine the placed steps run on. */
const RUNNER = principal.machine.reference(
    Scope.universe.id,
    "machine-01996ab0-0000-7000-8000-0000000000a1",
);

/** A workflow placing one compile step on the runner and answering its output. */
const compiling = defineWorkflow(
    {
        name: "compiling",
        title: t`Compiling`,
        input: schema.object({ source: schema.string() }),
        output: schema.string(),
    },
    NOTES,
);

/** Serve the workflow, placing its step on the runner, and start a run waiting for it. */
async function placed() {
    // place the compile step with the run's source
    const fixture = await openWorkflow(compiling, async (step, input) =>
        step.place(
            "compile",
            schema.string(),
            { function: "compile", input: { source: input.source } },
            RUNNER,
        ),
    );

    // start a run and wait until it waits for its placed step
    await fixture.start(() => compiling.start({ source: "main.ts" }, actor()));
    const [run] = await fixture.waitUntil((rows) => rows[0]?.["state"] === "waiting");
    const [step] = await fixture.served.database.select().from(compiling.step.table);

    return { fixture, run, step: { id: String(step?.["id"]) }, row: step };
}

test("place a step on its runner, and complete the run with the output the runner records", async () => {
    // place the step, and complete it as the runner
    const { fixture, step, row } = await placed();
    await fixture.objects(RUNNER).run(compiling.step, "complete", { ...step, output: "built" });
    const [run] = await fixture.waitUntil((rows) => rows[0]?.["state"] === "completed");

    // record the function, its input and the runner, and answer the runner's output
    expect({
        placed: [row?.["function"], row?.["input"], row?.["runnerId"]],
        ended: [run?.["state"], run?.["output"]],
    }).toEqual({
        placed: ["compile", { source: "main.ts" }, RUNNER.id],
        ended: ["completed", "built"],
    });
});

test("fail the run with the failure its runner records, refusing anyone else and a second end", async () => {
    // refuse ending the step as a member who cannot see the run, as only its runner may
    const { fixture, step } = await placed();
    const refused = await refusal(
        fixture.objects(bob).run(compiling.step, "complete", { ...step, output: "forged" }),
    );

    // fail it as the runner, and refuse ending it again
    const failure = Failure.http("BAD_REQUEST", "could not compile main.ts");
    await fixture.objects(RUNNER).run(compiling.step, "fail", { ...step, failure });
    const [run] = await fixture.waitUntil((rows) => rows[0]?.["state"] === "failed");
    const again = await refusal(
        fixture.objects(RUNNER).run(compiling.step, "complete", { ...step, output: "late" }),
    );

    expect({ refused, failure: run?.["failure"], again }).toEqual({
        refused: ["NOT_FOUND", "NOT_FOUND", `compiling-step '${step.id}' not found`],
        failure,
        again: [
            "BAD_REQUEST",
            "STEP_INVALID",
            "step 'compile' is invalid: only an open placed step ends by its runner",
        ],
    });
});

test("fail the run once its runner records an internal failure, without retrying the step", async () => {
    // fail the step as the runner with a failure a retry would answer the same
    const { fixture, step } = await placed();
    const failure = Failure.http("INTERNAL_SERVER_ERROR", "git commit exited with status 1");
    await fixture.objects(RUNNER).run(compiling.step, "fail", { ...step, failure });

    // fail the run with the recorded failure on its first advance, as replaying the step answers it again
    const [run] = await fixture.waitUntil((rows) => rows[0]?.["state"] === "failed");
    expect({ failure: run?.["failure"], attempts: run?.["attempts"] }).toEqual({
        failure,
        attempts: 0,
    });
});

test("forward a placed step's call as the run's actor for its runner alone", async () => {
    // create a note through the step as its runner, and try as a member who is not
    const { fixture, step } = await placed();
    const spaceId = fixture.served.scope;
    const created = await fixture.objects(RUNNER).run(compiling.step, "forward", {
        ...step,
        call: note.call("create", { spaceId, title: "built" }),
    });
    const refused = await refusal(
        fixture.objects(bob).run(compiling.step, "forward", {
            ...step,
            call: note.call("create", { spaceId, title: "forged" }),
        }),
    );

    // create the note as the run's actor, answer it to the runner, and refuse the member
    const notes = await fixture.served.database.select().from(note.table);
    expect({
        notes: notes.map((row) => [row.title, row.ownerId]),
        answered: created,
        refused,
    }).toEqual({
        notes: [["built", alice.id]],
        answered: schema.jsonObject().parse(JSON.parse(JSON.stringify(notes[0]))),
        refused: ["NOT_FOUND", "NOT_FOUND", `compiling-step '${step.id}' not found`],
    });
});

test("wait at a placed step its run's code wraps in a catch, recording nothing until the runner ends it", async () => {
    // place the step inside a catch that records the failure as a step of its own
    const fixture = await openWorkflow(compiling, async (step, input) => {
        try {
            return await step.place(
                "compile",
                schema.string(),
                { function: "compile", input: { source: input.source } },
                RUNNER,
            );
        } catch (error) {
            await step.run("failed", schema.string(), async () => String(error));
            throw error;
        }
    });
    await fixture.start(() => compiling.start({ source: "main.ts" }, actor()));
    const [waiting] = await fixture.waitUntil((rows) => rows[0]?.["state"] === "waiting");

    // complete it as the runner, which completes the run with its output
    const steps = await fixture.served.database.select().from(compiling.step.table);
    const [compile] = steps;
    await fixture
        .objects(RUNNER)
        .run(compiling.step, "complete", { id: String(compile?.["id"]), output: "built" });
    const [completed] = await fixture.waitUntil((rows) => rows[0]?.["state"] === "completed");

    expect({
        waiting: waiting?.["state"],
        recorded: steps.map((row) => row["name"]),
        completed: [completed?.["state"], completed?.["output"]],
    }).toEqual({
        waiting: "waiting",
        recorded: ["compile"],
        completed: ["completed", "built"],
    });
});
