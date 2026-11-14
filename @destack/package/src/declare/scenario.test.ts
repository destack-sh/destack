import { schema } from "@destack/schema";
import { expect, test } from "vitest";
import {
    defineScenario,
    type Interaction,
    type Operation,
    Reference,
    type Parametric,
} from "./scenario.ts";

/** A counter's step: counting once. */
const CountStep = schema.object({ action: schema.literal("count") });

/** The interaction of counters. */
const countInteraction: Interaction<
    schema.Infer<typeof CountStep>,
    { readonly kind: "count" },
    Record<string, never>
> = {
    name: "count",
    step: CountStep,
    observation: schema.object({ kind: schema.literal("count") }),
    globals: schema.object({}),
};

/** Start a counter. */
function createCounter(): number {
    return 0;
}

/** Declare a counter scenario counting once, then through operations, with some parameters. */
function declare(
    operations: readonly Parametric<
        Operation<{ readonly action: "count" }, { readonly kind: "count" }>
    >[],
    parameters: Readonly<Record<string, number>> = {},
) {
    return defineScenario({
        of: createCounter,
        interaction: countInteraction,
        name: "count",
        description: "count once, then through operations",
        given: { examples: [] },
        parameters,
        when: [{ action: "count" }, ...operations],
        then: [{ observe: { kind: "count" }, is: 1 }],
    });
}

/** Read a declaration's refusal message. */
function refusal(declared: () => unknown): string {
    try {
        declared();
    } catch (error) {
        return error instanceof Error ? error.message : String(error);
    }

    return "declared";
}

test("declare checked operations", () => {
    const counted = { action: "count" } as const;
    const checks = [
        { spans: { name: "counter.count", host: "member-*.device-*" }, count: 3 },
        { logs: { severity: "ERROR" }, count: 0 },
        { metric: { name: "db.client.operation.duration", host: "home" }, atMost: 2 },
    ];
    const declared = declare([
        { operation: "counter.count", times: 3, step: counted, then: checks },
        { operation: "counter.countTwice", times: 1, step: counted },
    ]);

    expect(declared.when.slice(1)).toEqual([
        { operation: "counter.count", times: 3, step: counted, then: checks },
        { operation: "counter.countTwice", times: 1, step: counted },
    ]);
});

test("refuse invalid operations", () => {
    const counted = { action: "count" } as const;

    expect([
        refusal(() => declare([{ operation: "count", times: 1, step: counted }])),
        refusal(() => declare([{ operation: "counter.count", times: 0, step: counted }])),
        refusal(() =>
            declare([
                { operation: "counter.count", times: 1, step: counted },
                { operation: "counter.count", times: 2, step: counted },
            ]),
        ),
    ]).toEqual([
        "scenario 'count' is invalid: 'count' names no object and verb",
        "scenario 'count' is invalid: it repeats 'counter.count' no times",
        "scenario 'count' is invalid: it names 'counter.count' twice",
    ]);
});

test("resolve parameter references", () => {
    // repeat by one parameter, expect spans by the product of two, and records by a sum with an integer
    const declared = declare(
        [
            {
                operation: "counter.count",
                times: "{writes}",
                step: { action: "count" },
                then: [
                    { spans: { name: "{n}" }, count: "{writes * devices + writes}" },
                    { logs: { severity: "ERROR" }, count: "{2 * writes + 1}" },
                ],
            },
        ],
        { writes: 10, devices: 3 },
    );

    expect([declared.parameters, Reference.resolve(declared.when[1], declared.parameters)]).toEqual(
        [
            { writes: 10, devices: 3 },
            {
                operation: "counter.count",
                times: 10,
                step: { action: "count" },
                then: [
                    { spans: { name: "{n}" }, count: 40 },
                    { logs: { severity: "ERROR" }, count: 21 },
                ],
            },
        ],
    );
});

test("refuse unknown parameters", () => {
    const counted = { action: "count" } as const;

    expect([
        refusal(() => declare([{ operation: "counter.count", times: "{writes}", step: counted }])),
        refusal(() => declare([], Object.fromEntries([["n", 1]]))),
        refusal(() =>
            declare(
                [{ operation: "counter.count", times: "{n * calls}", step: { action: "count" } }],
                {
                    calls: 2,
                },
            ),
        ),
    ]).toEqual([
        "scenario 'count' is invalid: it refers to 'writes', which is none of its parameters",
        "scenario 'count' is invalid: it names a parameter 'n', the repetition each step takes",
        "scenario 'count' is invalid: it refers to 'n', which is none of its parameters",
    ]);
});

test("number repetitions", () => {
    expect(
        Reference.repeat(
            { call: "issue.create", input: { title: "Issue {n}", rank: "{n}", tags: ["{n}"] } },
            3,
        ),
    ).toEqual({ call: "issue.create", input: { title: "Issue 3", rank: 3, tags: [3] } });
});
