import { expectTypeOf, test } from "@destack/test";
import { schema } from "@destack/schema";
import { defineResourceKind, type Provider } from "../index.ts";

test("require reconciling from providers of kinds with a state, and whole capabilities from any", () => {
    const database = defineResourceKind("database", {
        spec: schema.object({}),
        state: schema.object({ tables: schema.array(schema.string()) }),
    });
    const bucket = defineResourceKind("bucket", { spec: schema.object({}) });

    // refuse a stateful kind's provider without reconciling, half a capability, and reconciling without a state
    const unreconciled = { kind: database, code: "memory", object: "database" };
    const half = { kind: bucket, code: "memory", object: "bucket", provision: { provision } };
    const stateless = {
        kind: bucket,
        code: "memory",
        object: "bucket",
        reconcile: { plan: async () => ({ steps: [] }), apply: async () => {} },
    };
    expectTypeOf(unreconciled).not.toExtend<Provider<typeof database>>();
    expectTypeOf(half).not.toExtend<Provider<typeof bucket>>();
    expectTypeOf(stateless).not.toExtend<Provider<typeof bucket>>();

    // accept a stateless kind's provider with a whole capability
    const hosted = {
        kind: bucket,
        code: "memory",
        object: "bucket",
        provision: { provision, destroy },
    };
    expectTypeOf(hosted).toExtend<Provider<typeof bucket>>();
});

/** Provision a resource in memory. */
async function provision() {
    return { reference: "memory:x" };
}

/** Destroy nothing. */
async function destroy() {}
