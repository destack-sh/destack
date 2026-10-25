import { testSimulation } from "../src/test/index.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import type { Call } from "@destack/sync";
import { defineDatabase } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";
import { RequestId, Scope } from "@destack/sync";
import { subjectContext, testCallKey } from "@destack/object/test";
import { defineObject, field } from "../src/index.ts";
import { testEvents, serversOf } from "../src/test/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { TestObjectStore } from "../src/test/index.ts";

/** The space the articles live in. */
const SPACE_ID = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000051");

/** A review machine shared by the declarations below. */
const review = {
    initial: "pending",
    transitions: { approve: { from: ["pending"], to: "approved", permission: "review" } },
} as const;

test("change state fields only through their transitions, with modifiers enforcing the machine", () => {
    const article = defineObject({
        name: "article",
        plural: "articles",
        scope: space,
        fields: { title: field.string(), review: field.state(review).optional() },
        permissions: ["write", "review"],
    });

    // leave the state out of what callers write, and its transition through the modifier
    expect([article.written, Object.keys(article.methods).toSorted()]).toEqual([
        ["title"],
        ["approve", "create", "delete", "get", "list", "update"],
    ]);
});

test("record a transition's declared fields with its state, refusing a call without them", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // serve articles whose rejection records its reason, with the space's owner as owner
    const article = defineObject(
        {
            name: "article",
            plural: "articles",
            scope: space,
            fields: {
                title: field.string(),
                reason: field.string().optional(),
                review: field.state({
                    initial: "pending",
                    transitions: {
                        reject: {
                            from: ["pending"],
                            to: "rejected",
                            permission: "review",
                            fields: ["reason"],
                        },
                    },
                }),
            },
            permissions: ["write", "review"],
        },
        (method) => ({ create: method.create("write") }),
    );
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({
                packages: [objectPackage, { objects: { article }, tables: [] }],
                follows: [],
                tables: [space.table],
            }),
        }),
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const { database } = storage;
    const store = await TestObjectStore.at(space.reference(Scope.universe.id, SPACE_ID), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(SPACE_ID),
        servers: serversOf([article]),
        policies: [space],
        database,
        callKey: testCallKey,
        events: await testEvents(database, world),
        origin: { package: article.package },
    });
    await openSpace(database, world, SPACE_ID);
    const caller = subjectContext(spacePrincipal(SPACE_ID), SPACE_ID, {
        environment: world,
    }).access();
    const mutate = (call: Call) =>
        store.mutate(caller, { id: RequestId.create(world), calls: [call] });

    // reject an article with its reason, and refuse a rejection without one
    const [created] = await mutate(article.call("create", { spaceId: SPACE_ID, title: "Draft" }));
    const { id } = schema.looseObject({ id: schema.string() }).parse(created);
    const rejection = article.call("reject", { spaceId: SPACE_ID, id, reason: "off topic" });
    const { reason: _reason, ...unreasoned } = rejection.input;
    const refused = await refusal(mutate({ ...rejection, input: unreasoned }));
    await mutate(rejection);
    const [row] = await database.select().from(article.table);
    expect({ refused, stored: [row?.review, row?.reason] }).toEqual({
        refused: [
            "BAD_REQUEST",
            "INPUT_INVALID",
            "invalid input: reason: invalid input: expected string, received undefined",
        ],
        stored: ["rejected", "off topic"],
    });
});
