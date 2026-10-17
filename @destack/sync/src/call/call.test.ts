import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Mutation } from "./call.ts";

/** A call a mutation makes. */
const CALL = {
    type: {
        packageId: schema
            .identifier("package")
            .parse("package-01996ab0-0000-7000-8000-000000000001"),
        type: "note",
    },
    method: "create",
    release: "2026.10.0",
    input: {},
};

test("accept a created UUIDv7 or a derived UUIDv8 as a mutation's request identifier, refusing others", () => {
    // parse a created, a derived, a random and a name-based identifier
    const ids = [
        "01996ab0-0000-7000-8000-000000000001",
        "01996ab0-0000-8000-8000-000000000001",
        "01996ab0-0000-4000-8000-000000000001",
        "01996ab0-0000-5000-8000-000000000001",
    ];
    const parsed = ids.map((id) => {
        const result = Mutation.safeParse({ id, calls: [CALL] });

        return result.success
            ? "accepted"
            : result.error.issues.map((issue) => issue.path.join("."));
    });

    // accept the time-ordered ones, refusing the others at their identifier
    expect(parsed).toEqual(["accepted", "accepted", ["id"], ["id"]]);
});
