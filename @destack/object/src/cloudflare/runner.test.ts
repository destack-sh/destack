import { testSimulation } from "../test/index.ts";
import { expect, test } from "@destack/test";
import { DurableObjectRunner } from "./runner.ts";
import { Failure } from "@destack/error";
import { defineWorkload } from "../workload/workload.ts";
import type {} from "@destack/package/import-meta";

test("forget a stored start that fails to resume, and still answer the object's events", async () => {
    // keep a start in the object's storage that no longer parses, as one a host replaced does not run
    const simulation = testSimulation();
    const deleted: string[] = [];
    const object = new DurableObjectRunner(
        {
            storage: {
                sql: {
                    exec: () => {
                        throw new TypeError("the test object runs no statement");
                    },
                    databaseSize: 0,
                },
                transaction: (closure) => closure(),
                deleteAll: async () => undefined,
                get: async () => ({ start: "stale" }),
                put: async () => undefined,
                delete: async (key) => {
                    deleted.push(key);

                    return true;
                },
            },
            blockConcurrencyWhile: (closure) => closure(),
        },
        {
            PARTITION: {
                fetch: async () => new Response(null, { status: 401 }),
            },
        },
        {
            workload: defineWorkload(
                { name: "main", start: async () => ({ packages: [] }) },
                import.meta.destack,
            ),
            resources: {},
            remote: () => {
                throw new TypeError("the test workload reaches no remote");
            },
            environment: simulation.environment("test"),
        },
    );

    // refuse a request as no workload running, where the failed resume once failed every event
    const served = await object.fetch(new Request("https://workload.test/served"));
    const refusal = Failure.read(served);
    expect([served.status, (await refusal).reason, deleted]).toEqual([
        503,
        "WORKLOAD_UNAVAILABLE",
        ["start"],
    ]);
});
