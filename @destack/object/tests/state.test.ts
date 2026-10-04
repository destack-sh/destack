import { expect, test } from "@destack/test";
import { TABLE } from "@destack/db";
import { defineObject, field } from "../src/index.ts";
import { space } from "./fixture/space.ts";

/** A review machine shared by the declarations below. */
const review = {
    initial: "pending",
    transitions: { approve: { from: ["pending"], to: "approved", permission: "review" } },
} as const;

test("derive one column and one method per transition for each state field of an object", () => {
    const article = defineObject({
        name: "article",
        plural: "articles",
        scope: space,
        fields: {
            title: field.string(),
            status: field.state({
                initial: "draft",
                transitions: { publish: { from: ["draft"], to: "published", permission: "write" } },
            }),
            review: field.state(review),
        },
        permissions: ["read", "write", "review"],
        methods: (method) => ({ get: method.get("read") }),
    });

    // keep each machine in its own column, starting in its initial state
    const columns = article.table[TABLE].columns;
    expect([
        columns.status.definition.default,
        columns.review.definition.default,
        columns.review.definition.enumValues,
    ]).toEqual(["draft", "pending", ["pending", "approved"]]);

    // derive every transition of every state field as a method
    expect(Object.keys(article.methods).toSorted()).toEqual(["approve", "get", "publish"]);
    expect(article.methods.approve.transition).toEqual({
        field: "review",
        from: ["pending"],
        to: "approved",
    });
});

test("refuse a transition name two state fields declare and a transition naming an undeclared permission", () => {
    // refuse the same transition name in two state fields
    expect(() =>
        defineObject({
            name: "clash",
            plural: "clashes",
            scope: space,
            fields: { first: field.state(review), second: field.state(review) },
            permissions: ["review"],
        }),
    ).toThrow(new TypeError("transition approve is declared by more than one state field"));

    // refuse a transition with an undeclared permission
    expect(() =>
        defineObject({
            name: "orphan",
            plural: "orphans",
            scope: space,
            fields: { review: field.state(review) },
            permissions: ["read"],
        }),
    ).toThrow(new TypeError("object orphan has no permission review"));
});

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
