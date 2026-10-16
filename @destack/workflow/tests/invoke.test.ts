import { CallError } from "@destack/runtime/error";
import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { defineWorkflow } from "../src/index.ts";
import { actor, openServed } from "./fixture.ts";

/** A number a run takes or answers. */
const Value = schema.object({ value: schema.number().int() });

/** Double a number, refusing a negative one. */
const double = defineWorkflow({ name: "double", input: Value, output: Value });

/** Double two numbers in child runs and add the results. */
const sum = defineWorkflow({
    name: "sum",
    input: schema.object({ left: schema.number().int(), right: schema.number().int() }),
    output: Value,
    waits: [double.run],
});

/** Serve the sum workflow over the doubling one. */
function openSum() {
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
                        throw new CallError("BAD_REQUEST", { message: "a negative number" });
                    }

                    return { value: 2 * input.value };
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
    const children = (await fixture.store.database.select().from(double.run.table))
        .map((row) => [row["key"], row["state"], row["output"]])
        .toSorted((left, right) => String(left[0]).localeCompare(String(right[0])));
    expect([run?.["output"], children]).toEqual([
        { value: 10 },
        [
            [`${id}/left`, "completed", { value: 4 }],
            [`${id}/right`, "completed", { value: 6 }],
        ],
    ]);
});

test("fail a run with the refusal its child run failed with", async () => {
    const fixture = await openSum();
    await fixture.start(() => sum.start({ left: 2, right: -1 }, actor()));
    const [run] = await fixture.until((rows) => rows[0]?.["state"] === "failed");
    expect(run?.["failure"]).toEqual({
        code: "BAD_REQUEST",
        status: 400,
        message: "a negative number",
    });
});
