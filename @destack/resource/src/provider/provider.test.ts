import { expect, expectTypeOf, test } from "@destack/test";
import { schema } from "@destack/schema";
import { defineResourceKind, Provider } from "../index.ts";

test("require reconciling from providers of kinds with a state, and whole capabilities from any", () => {
    const database = defineResourceKind("database", {
        spec: schema.object({}),
        state: schema.object({ tables: schema.array(schema.string()) }),
    });
    const bucket = defineResourceKind("bucket", { spec: schema.object({}) });

    // refuse a stateful kind's provider without reconciling, half a capability, and reconciling without a state
    const unreconciled = { kind: database, code: "memory", object: "database" };
    const half = { kind: bucket, code: "memory", object: "bucket", provision };
    const stateless = {
        kind: bucket,
        code: "memory",
        object: "bucket",
        plan: async () => ({ steps: [] }),
    };
    expectTypeOf(unreconciled).not.toExtend<Provider<typeof database>>();
    expectTypeOf(half).not.toExtend<Provider<typeof bucket>>();
    expectTypeOf<Provider<typeof bucket>>().not.toHaveProperty("plan");

    // report each provider's capabilities
    const hosted: Provider<typeof bucket> = {
        kind: bucket,
        code: "memory",
        object: "bucket",
        provision,
        destroy,
    };
    expect(
        [unreconciled, half, stateless, hosted].map((provider) => [
            Provider.reconciles(provider),
            Provider.provisions(provider),
        ]),
    ).toEqual([
        [false, false],
        [false, false],
        [false, false],
        [false, true],
    ]);
});

/** Provision a resource in memory. */
async function provision() {
    return { reference: "memory:x" };
}

/** Destroy nothing. */
async function destroy() {}
