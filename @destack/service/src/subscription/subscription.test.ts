import { expect, test } from "@destack/test";
import { Condition } from "@destack/db/query";
import { describeSubscription } from "../inspect/index.ts";
import { defineService } from "../declare/service.ts";
import { defineSubscription, MAX_LAG_MILLISECONDS } from "./subscription.ts";

test("describe a declared subscription with its object reference, condition, operations and start", () => {
    const note = { package: defineService("notes", {}).package, name: "note" };
    const subscription = defineSubscription({
        name: "published",
        object: note,
        where: Condition.eq("status", "published"),
        on: ["create", "update"],
    });

    expect(subscription.kind).toBe("subscription");
    expect(subscription.object).toBe(note);
    expect(describeSubscription(subscription)).toEqual({
        name: "published",
        version: 1,
        object: { packageId: note.package.id, name: "note" },
        where: {
            kind: "compare",
            operator: "eq",
            left: { kind: "column", name: "status" },
            right: { kind: "literal", value: "published" },
        },
        on: ["create", "update"],
        from: "now",
        maxLag: MAX_LAG_MILLISECONDS,
    });

    // refuse a snapshot for a subscription consuming no creations, which its rows are
    expect(() =>
        defineSubscription({ name: "removed", object: note, on: ["delete"], from: "snapshot" }),
    ).toThrow(
        "subscription removed starts with a snapshot, whose rows are created, but consumes no creations",
    );
});
