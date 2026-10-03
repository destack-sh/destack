import { expect, test } from "@destack/test";
import { ResourceId } from "./declaration.ts";
import { schema } from "@destack/schema";
import { defineResourceKind } from "./kind.ts";

test("read stored resources and desired states through their kind, refusing another kind's specification, invalid values and states of a stateless kind", () => {
    const database = defineResourceKind("database", {
        spec: schema.object({ tier: schema.enum(["zone", "global"]) }),
        state: schema.object({ tables: schema.array(schema.string()) }),
    });
    const bucket = defineResourceKind("bucket", { spec: schema.object({}) });
    const stored = {
        id: ResourceId.parse("database-01996ab0-0000-7000-8000-000000000001"),
        scope: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002"),
        spec: { tier: "zone" },
        reference: null,
    };

    // read a database and its states, and refuse a bucket reading its specification, an invalid state and states of a stateless kind
    expect([
        outcome(() => database.record(stored).spec),
        outcome(() => database.states([{ tables: ["note"] }])),
        outcome(() => bucket.record(stored)),
        outcome(() => database.states([{ tables: "note" }])),
        outcome(() => bucket.states([{}])),
        outcome(() => bucket.states([{ tables: [] }])),
    ]).toEqual([{ tier: "zone" }, [{ tables: ["note"] }], "ZodError", "ZodError", [], "TypeError"]);
});

/** Read a value, or the class name of the error reading it throws. */
function outcome(read: () => unknown) {
    try {
        return read();
    } catch (error) {
        if (!(error instanceof Error)) {
            throw error;
        }

        return error.constructor.name;
    }
}
