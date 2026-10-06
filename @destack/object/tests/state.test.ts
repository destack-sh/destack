import { expect, test } from "@destack/test";
import { defineObject, field } from "../src/index.ts";
import { space } from "./fixture/space.ts";

/** A review machine shared by the declarations below. */
const review = {
    initial: "pending",
    transitions: { approve: { from: ["pending"], to: "approved", permission: "review" } },
} as const;

test("change state fields only through their transitions, modifiers keeping the machine", () => {
    const article = defineObject({
        name: "article",
        plural: "articles",
        scope: space,
        fields: { title: field.string(), review: field.state(review).optional() },
        permissions: ["write", "review"],
        methods: (method) => ({ create: method.create("write"), update: method.update("write") }),
    });

    // leave the state out of what callers write, and keep its transition through the modifier
    expect([article.written, Object.keys(article.methods).toSorted()]).toEqual([
        ["title"],
        ["approve", "create", "update"],
    ]);
});
