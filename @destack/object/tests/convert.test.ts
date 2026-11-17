import { testSimulation } from "../src/test/index.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { principal, relation } from "@destack/access";
import { defineDatabase, Expression } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";

import { RequestId, Scope } from "@destack/sync";
import { defineObject, field } from "../src/index.ts";
import { openSpace, space, spacePrincipal } from "../src/test/space.ts";
import { spaceId } from "./fixture/device.ts";
import { testCallKey } from "@destack/object/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { userContext, users } from "./fixture/user.ts";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { TestObjectStore } from "../src/test/index.ts";
import { argument, plural, t } from "@destack/locale";

/** Cards whose `title` became `name`. */
const card = defineObject(
    {
        name: "card",
        plural: "cards",
        title: t`${plural(argument("count"), { one: "Card", other: "Cards" })}`,
        scope: space,
        convert: { 2: { name: Expression.column("title") } },
        fields: {
            owner: field.reference(principal.user).caller(),
            name: field.string(schema.string().min(1)),
        },
        access: {
            permissions: { read: relation("owner"), write: relation("owner") },
        },
    },
    (method) => ({
        create: method.create("write"),
        update: method.update("write"),
    }),
);

test("execute calls made against an earlier edition with the fields they renamed, and refuse calls of a later edition", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({
                packages: [objectPackage, { objects: { card }, tables: [] }],
                follows: [],
                tables: [],
            }),
        }),
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([card]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: {
            package: card.package,
        },
    });
    const context = userContext(users.alice, spaceId, { environment: world });

    // create and rename a card through calls of the edition before the rename
    const [result] = await store.mutate(context.access(), {
        id: RequestId.create(world),
        calls: [
            {
                type: card.typeReference,
                method: "create",
                release: "2026.8.0",
                edition: 1,
                input: { spaceId, title: "Plan" },
            },
        ],
    });
    const created = schema.looseObject({ id: schema.string() }).parse(result);
    await store.mutate(context.access(), {
        id: RequestId.create(world),
        calls: [
            {
                type: card.typeReference,
                method: "update",
                release: "2026.8.0",
                edition: 1,
                input: { spaceId, id: created.id, title: "Roadmap" },
            },
        ],
    });

    // refuse a call made against an edition the server does not serve yet
    const later = store.mutate(context.access(), {
        id: RequestId.create(world),
        calls: [
            {
                type: card.typeReference,
                method: "update",
                release: objectPackage.version,
                edition: 3,
                input: { spaceId, id: created.id, name: "Next" },
            },
        ],
    });
    expect(await refusal(later)).toEqual([
        "BAD_REQUEST",
        "RELEASE_INVALID",
        "edition '3' is invalid: 'card.update' targets it, but 'card' is at edition '2'",
    ]);
    const stored = await store.query(
        context.access(),
        card.call("get", { spaceId, id: created.id }),
    );
    expect(stored.name).toBe("Roadmap");
});
