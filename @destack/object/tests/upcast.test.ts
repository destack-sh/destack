import { expect, onTestFinished, test } from "@destack/test";
import { principal, relation } from "@destack/access";
import type { AuditRecorder } from "@destack/audit";
import type { DatabaseConnection } from "@destack/db";
import { defineDatabase } from "@destack/db/declare";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";
import { Bookmark } from "@destack/service/bookmark";
import { Journal } from "@destack/service/database";
import { RequestId } from "@destack/service/request";
import type { ServiceContext } from "@destack/service/server";
import { defineObject, field, method } from "../src/index.ts";
import { ObjectServer } from "../src/server/index.ts";
import { request } from "./schema.ts";
import { openSpace, space } from "./fixture/space.ts";
import { spaceId } from "./fixture/device.ts";

/** Cards at their second version, with `title` renamed to `name`. */
const card = defineObject({
    name: "card",
    plural: "cards",
    scope: space,
    version: 2,
    upcast: (name, input, from) => {
        // rename the first version's title
        if (from < 2 && (name === "create" || name === "update") && "title" in input) {
            const { title, ...rest } = input;

            return { ...rest, name: title };
        }

        return { ...input };
    },
    fields: {
        owner: field.reference(principal.user).caller(),
        name: field.string(schema.string().min(1)),
    },
    permissions: { read: relation("owner"), write: relation("owner") },
    methods: {
        get: method.get("read"),
        create: method.create("write"),
        update: method.update("write"),
    },
});

test.for(TEST_DIALECTS)(
    "execute calls made against an earlier version through the upcast, and refuse calls of a later one, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(
            dialect,
            defineDatabase({ name: "main", tables: [request, ...card.tables] }),
            { isMigrated: true },
        );
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        const server = new ObjectServer({
            objects: { card },
            database: storage.database,
            context: () => ({
                subjects: [principal.user.reference("global", "alice")],
                now: Date.now(),
                attributes: {},
            }),
            journal: new Journal(request),
            audit: () =>
                ({ record: async () => {} }) as unknown as AuditRecorder<DatabaseConnection>,
        });
        const context = {
            scope: spaceId,
            requireCaller: () => ({ id: "alice" }),
            bookmark: new Bookmark(),
            observed: new Bookmark(),
        } as unknown as ServiceContext;

        // create and rename a card through unversioned first-version calls
        const [created] = (await server.mutate(
            {
                id: RequestId.create(),
                calls: [{ method: "card.create", input: { spaceId, title: "Plan" } }],
            },
            context,
        )) as [{ id: string }];
        await server.mutate(
            {
                id: RequestId.create(),
                calls: [
                    { method: "card.update", input: { spaceId, id: created.id, title: "Roadmap" } },
                ],
            },
            context,
        );

        // refuse a call made against a version the server does not know yet
        const later = server.mutate(
            {
                id: RequestId.create(),
                calls: [
                    {
                        method: "card.update",
                        input: { spaceId, id: created.id, name: "Next" },
                        version: 3,
                    },
                ],
            },
            context,
        );
        await expect(later).rejects.toMatchObject({
            code: "BAD_REQUEST",
            message: "card.update was made against version 3 of card, which is at version 2",
        });
        const stored = (await server.call(card, "get", { spaceId, id: created.id }, context)) as {
            name: string;
        };
        expect(stored.name).toBe("Roadmap");
    },
);

test("refuse an object at a later version without an upcast", () => {
    expect(() =>
        defineObject({
            name: "sheet",
            plural: "sheets",
            scope: space,
            version: 2,
            fields: {},
            permissions: ["read"],
        }),
    ).toThrow(new TypeError("object sheet is at version 2 without an upcast"));
});
