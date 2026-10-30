import { Failure } from "@destack/error";
import { CallError } from "@destack/object";
import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { defineWorkflow } from "../src/index.ts";
import { actor, alice, openServed } from "./workflow.ts";
import { NOTES } from "./package.ts";
import { NO_OUTPUT } from "../src/index.ts";

/** A number a run takes or answers. */
const Value = schema.object({ value: schema.number().int() });

/** Double a number, refusing a negative one. */
const double = defineWorkflow({ name: "double", input: Value, output: Value }, NOTES);

/** Double two numbers in child runs and add the results. */
const sum = defineWorkflow(
    {
        name: "sum",
        input: schema.object({ left: schema.number().int(), right: schema.number().int() }),
        output: Value,
        waits: [double.run],
    },
    NOTES,
);

/** Serve the sum workflow over the doubling one, doubling as given. */
function openSum(doubled: (value: number) => number = (value) => 2 * value) {
    return openServed(
        sum,
        sum.implement(async (step, input) => {
            const left = await step.invoke("left", double, { value: input.left });
            const right = await step.invoke("right", double, { value: input.right });

            return { value: left.value + right.value };
        }),
        [
            {
                workflow: double,
                served: double.implement(async (_step, input) => {
                    if (input.value < 0) {
                        throw CallError.http("BAD_REQUEST", { message: "a negative number" });
                    }

                    return { value: doubled(input.value) };
                }),
            },
        ],
    );
}

test("add the outputs of two child runs, each started once under a key its step derives", async () => {
    // sum two doubled numbers
    const fixture = await openSum();
    const started = await fixture.start(() => sum.start({ left: 2, right: 3 }, actor()));
    const { id } = schema.looseObject({ id: schema.string() }).parse(started);
    const [run] = await fixture.until((rows) => rows[0]?.["state"] === "completed");

    // expect the sum, and the two child runs completed under the parent's keys
    const children = (await fixture.served.database.select().from(double.run.table))
        .map((row) => [row["key"], row["state"], row["output"]])
        .toSorted((left, right) =>
            schema.string().parse(left[0]).localeCompare(schema.string().parse(right[0])),
        );
    expect([run?.["output"], children]).toEqual([
        { value: 10 },
        [
            [`${id}/left`, "completed", { value: 4 }],
            [`${id}/right`, "completed", { value: 6 }],
        ],
    ]);
});

test("invoke a child run again once its parent is reset to the child's step, reading the new run", async () => {
    // sum two numbers doubled with a hundred per earlier child run, then reset the sum to its right step as alice
    let invoked = 0;
    const fixture = await openSum((value) => 2 * value + 100 * invoked++);
    const started = await fixture.start(() => sum.start({ left: 2, right: 3 }, actor()));
    const { id } = schema.looseObject({ id: schema.string() }).parse(started);
    await fixture.until((rows) => rows[0]?.["state"] === "completed");
    const reset = await fixture.objects(alice).call(sum.run.name, "reset", { id, step: "right" });
    const [run] = await fixture.until((rows) => rows[0]?.["state"] === "completed");

    // expect the right child run started again under its key, and the sum read from the new run
    const children = (await fixture.served.database.select().from(double.run.table))
        .map((row) => [
            row["key"],
            schema.looseObject({ value: schema.number() }).parse(row["output"]).value,
        ])
        .toSorted((left, right) => Number(left[1]) - Number(right[1]));
    expect([reset, run?.["output"], children]).toEqual([
        { forgotten: 1 },
        { value: 210 },
        [
            [`${id}/left`, 4],
            [`${id}/right`, 106],
            [`${id}/right`, 206],
        ],
    ]);
});

test("fail a run with the refusal its child run failed with", async () => {
    const fixture = await openSum();
    await fixture.start(() => sum.start({ left: 2, right: -1 }, actor()));
    const [run] = await fixture.until((rows) => rows[0]?.["state"] === "failed");
    expect(run?.["failure"]).toEqual(Failure.http("BAD_REQUEST", "a negative number"));
});

/** Sleep an hour, answering nothing. */
const nap = defineWorkflow({ name: "nap", input: schema.object({}), output: NO_OUTPUT }, NOTES);

/** Invoke a nap, cancelling it with this run or letting it run on. */
const watch = defineWorkflow(
    {
        name: "watch",
        input: schema.object({ isAbandoned: schema.boolean() }),
        waits: [nap.run],
        output: NO_OUTPUT,
    },
    NOTES,
);

test("cancel a child run once its parent is cancelled, and leave an abandoned one running", async () => {
    // start a watch that cancels its nap with it, and one that abandons it
    const fixture = await openServed(
        watch,
        watch.implement(async (step, input) => {
            const onParentClose = input.isAbandoned ? "abandon" : "cancel";
            await step.invoke("nap", nap, {}, { onParentClose });
        }),
        [
            {
                workflow: nap,
                served: nap.implement(async (step) => {
                    await step.sleep("nap", { hours: 1 });
                }),
            },
        ],
    );
    const ids: string[] = [];
    for (const isAbandoned of [false, true]) {
        const started = await fixture.start(() => watch.start({ isAbandoned }, actor()));
        ids.push(schema.looseObject({ id: schema.string() }).parse(started).id);
    }
    await fixture.until(() => true);

    // cancel both watches as their actor
    for (const id of ids) {
        await fixture
            .objects(actor().actor)
            .call(watch.run.name, "cancel", watch.run.target({ scope: fixture.served.scope, id }));
    }
    await fixture.served.simulation.idle();

    // expect the first nap cancelled with its watch, and the abandoned one still sleeping
    const naps = await fixture.served.database.select().from(nap.run.table);
    const state = (id: string) => naps.find((row) => row["key"] === `${id}/nap`)?.["state"];
    expect(ids.map((id) => state(id))).toEqual(["cancelled", "waiting"]);
});
