import { expect, onTestFinished, test } from "@destack/test";
import { principal, relation } from "@destack/access";
import { defineDatabase } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { schema, Clock } from "@destack/schema";

import { RequestId } from "@destack/runtime/request";
import { defineObject, field } from "../src/index.ts";
import { ObjectStore } from "../src/server/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { spaceId } from "./fixture/device.ts";
import { testCallKey } from "@destack/runtime/test";
import { callTables, testEvents, serversOf } from "../src/test/index.ts";
import { userContext, users } from "./fixture/user.ts";

/** Cards whose `title` became `name`. */
const card = defineObject({
    name: "card",
    plural: "cards",
    scope: space,
    moved: { fields: { name: "title" } },
    fields: {
        owner: field.reference(principal.user).caller(),
        name: field.string(schema.string().min(1)),
    },
    permissions: { read: relation("owner"), write: relation("owner") },
}).methods((method) => ({
    create: method.create("write"),
    update: method.update("write"),
}));

test.for(TEST_DIALECTS)(
    "execute calls made against an earlier release with the fields they renamed, and refuse calls of a later release, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(
            dialect,
            defineDatabase({
                name: "main",
                tables: [...callTables, ...card.tables],
            }),
            { isMigrated: true },
        );
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        const store = new ObjectStore({
            chain: async () => undefined,
            clock: Clock.system,
            principal: spacePrincipal(spaceId),
            servers: serversOf([card]),
            database: storage.database,
            callKey: testCallKey,
            events: await testEvents(storage.database),
            origin: {
                package: card.package,
            },
        });
        const context = userContext(users.alice, spaceId);

        // create and rename a card through calls of the release before the rename
        const [result] = await store.mutate(context.access(), {
            id: RequestId.create(Date.now()),
            calls: [
                {
                    type: card.typeReference,
                    method: "create",
                    release: "2026.8.0",
                    input: { spaceId, title: "Plan" },
                },
            ],
        });
        const created = schema.looseObject({ id: schema.string() }).parse(result);
        await store.mutate(context.access(), {
            id: RequestId.create(Date.now()),
            calls: [
                {
                    type: card.typeReference,
                    method: "update",
                    release: "2026.8.0",
                    input: { spaceId, id: created.id, title: "Roadmap" },
                },
            ],
        });

        // refuse a call made against a release the server does not serve yet
        const later = store.mutate(context.access(), {
            id: RequestId.create(Date.now()),
            calls: [
                {
                    type: card.typeReference,
                    method: "update",
                    release: "2026.10.0",
                    input: { spaceId, id: created.id, name: "Next" },
                },
            ],
        });
        await expect(later).rejects.toMatchObject({
            code: "BAD_REQUEST",
            message:
                "card.update was made against release 2026.10.0 of card, but card is at 2026.9.0",
        });
        const stored = await store.query(
            context.access(),
            card.call("get", { spaceId, id: created.id }),
        );
        expect(stored.name).toBe("Roadmap");
    },
);
