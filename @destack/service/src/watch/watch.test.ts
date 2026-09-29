import { expect, test } from "@destack/test";
import { Condition } from "@destack/db/query";
import { describeWatch } from "../inspect/index.ts";
import { defineService } from "../declare/service.ts";
import { defineWatch, MAX_LAG_MILLISECONDS } from "./watch.ts";

test("describe a declared watch with its object reference, condition, operations and start", () => {
    const note = { package: defineService("notes", {}).package, name: "note" };
    const watch = defineWatch({
        name: "published",
        object: note,
        where: Condition.eq("status", "published"),
        on: ["create", "update"],
    });

    expect(watch.kind).toBe("watch");
    expect(watch.object).toBe(note);
    expect(describeWatch(watch)).toEqual({
        name: "published",
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

    // refuse a snapshot for a watch without creations
    expect(() =>
        defineWatch({ name: "removed", object: note, on: ["delete"], from: "snapshot" }),
    ).toThrow(
        "watch removed starts with a snapshot, whose rows are created, but consumes no creations",
    );
});
