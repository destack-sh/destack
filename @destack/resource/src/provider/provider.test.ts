import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { defineResourceKind, Provider } from "../index.ts";

test("require reconciling from providers of kinds with a state, and whole capabilities from any", () => {
    const database = defineResourceKind("database", {
        spec: schema.object({}),
        state: schema.object({ tables: schema.array(schema.string()) }),
    });
    const bucket = defineResourceKind("bucket", { spec: schema.object({}) });
    const provision = async () => ({ reference: "memory:x" });
    const destroy = async () => {};

    // refuse a stateful kind's provider without reconciling, half a capability, and reconciling without a state
    // @ts-expect-error a provider of a kind with a state plans and applies
    const unreconciled: Provider<typeof database> = {
        kind: database,
        code: "memory",
        object: "database",
    };
    // @ts-expect-error a provider provisions and destroys, or does neither
    const half: Provider<typeof bucket> = {
        kind: bucket,
        code: "memory",
        object: "bucket",
        provision,
    };
    const stateless: Provider<typeof bucket> = {
        kind: bucket,
        code: "memory",
        object: "bucket",
        // @ts-expect-error a provider of a kind without a state reconciles nothing
        plan: async () => ({ steps: [] }),
    };

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
