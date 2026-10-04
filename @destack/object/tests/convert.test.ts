import { expect, onTestFinished, test } from "@destack/test";
import { principal, relation } from "@destack/access";
import { journal } from "@destack/audit";
import { defineDatabase, Expression } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";

import { RequestId } from "@destack/service/request";
import { defineObject, field } from "../src/index.ts";
import { ObjectServer } from "../src/server/index.ts";
import { openSpace, space } from "./fixture/space.ts";
import { spaceId } from "./fixture/device.ts";
import { testCallKey } from "@destack/service/test";
import { userContext } from "./fixture/user.ts";

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
    methods: (method) => ({
        get: method.get("read"),
        create: method.create("write"),
        update: method.update("write"),
    }),
});

test.for(TEST_DIALECTS)(
    "execute calls made against an earlier release with the fields they renamed, and refuse calls of a later release, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(
            dialect,
            defineDatabase({
                name: "main",
                tables: [journal, ...card.tables],
            }),
            { isMigrated: true },
        );
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        const server = new ObjectServer({
            objects: { card },
            database: storage.database,
            callKey: testCallKey,
            origin: {
                package: card.package,
                service: "test",
            },
        });
        const context = userContext("alice", spaceId);

        // create and rename a card through calls of the release before the rename
        const [result] = await server.mutate(
            {
                id: RequestId.create(),
                calls: [
                    {
                        method: "card.create",
                        release: "2026.8.0",
                        input: { spaceId, title: "Plan" },
                    },
                ],
            },
            context,
        );
        const created = schema.looseObject({ id: schema.string() }).parse(result);
        await server.mutate(
            {
                id: RequestId.create(),
                calls: [
                    {
                        method: "card.update",
                        release: "2026.8.0",
                        input: { spaceId, id: created.id, title: "Roadmap" },
                    },
                ],
            },
            context,
        );

        // refuse a call made against a release the server does not serve yet
        const later = server.mutate(
            {
                id: RequestId.create(),
                calls: [
                    {
                        method: "card.update",
                        release: "2026.10.0",
                        input: { spaceId, id: created.id, name: "Next" },
                    },
                ],
            },
            context,
        );
        await expect(later).rejects.toMatchObject({
            code: "BAD_REQUEST",
            message: "card.update was made against release 2026.10.0 of card, which is at 2026.9.0",
        });
        const stored = await server.call(card, "get", { spaceId, id: created.id }, context);
        expect(stored.name).toBe("Roadmap");
    },
);

test("refuse method conversions keyed by a release after the object's package release", () => {
    expect(() =>
        defineObject({
            name: "sheet",
            plural: "sheets",
            scope: space,
            fields: { name: field.string() },
            permissions: ["write"],
            methods: (method) => ({
                create: method.create("write", {
                    convert: { "2026.10.0": { name: Expression.column("title") } },
                }),
            }),
        }),
    ).toThrow(
        new TypeError(
            "conversion of sheet.create is keyed by 2026.10.0, after its release 2026.9.0",
        ),
    );
});

test("refuse a field moved from the name of a current field", () => {
    expect(() =>
        defineObject({
            name: "sheet",
            plural: "sheets",
            scope: space,
            moved: { fields: { name: "title" } },
            fields: { name: field.string(), title: field.string() },
            permissions: ["write"],
        }),
    ).toThrow(new TypeError("field sheet.name moved from title, which names a current field"));
});
