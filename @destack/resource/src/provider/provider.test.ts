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

test("accept providers fencing their resources and declaring the controllers their host runs", () => {
    const bucket = defineResourceKind("bucket", { spec: schema.object({}) });
    const provided = {
        kind: bucket,
        code: "memory",
        object: "bucket",
        provision: { provision, destroy },
    };
    type Swept = Provider<typeof bucket, unknown, never, never, never, Sweep>;

    // accept a whole fence and the controllers the host runs, typed by the host's controller alone
    const swept = { ...provided, fence: { fence, lift: destroy }, controllers: [sweep] };
    expectTypeOf(swept).toExtend<Swept>();
    expectTypeOf(swept).not.toExtend<Provider<typeof bucket>>();

    // refuse half a fence and a controller of another type
    expectTypeOf({ ...provided, fence: { fence } }).not.toExtend<Swept>();
    expectTypeOf({ ...provided, controllers: ["sweep"] }).not.toExtend<Swept>();
});

/** A controller a host runs beside its providers. */
interface Sweep {
    /** Reconcile the provider's resources, answering when to run again. */
    reconcile(): Promise<number>;
}

/** The controller sweeping the provider's resources. */
const sweep: Sweep = { reconcile: async () => 0 };

/** Provision a resource in memory. */
async function provision() {
    return { reference: "memory:x" };
}

/** Destroy nothing. */
async function destroy() {}

/** Fence nothing, reporting the fence set. */
async function fence() {
    return true;
}
