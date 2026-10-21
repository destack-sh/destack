import { defineObject, field, ObjectDescription } from "../src/index.ts";
import type { CallableName } from "../src/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { principal, relation } from "@destack/access";
import { defineDatabase } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";

import { RequestId } from "@destack/sync";
import { ObjectStore } from "../src/server/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { testCallKey } from "@destack/object/test";
import { callTables, testEvents, serversOf } from "../src/test/index.ts";
import { userContext, users, userName } from "./fixture/user.ts";
import { request, type RequestInput } from "./fixture/request.ts";
import { Clock } from "@destack/environment";

/** The space containing the chores. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000003");

/** Chores on lists, finished a list at a time. */
const chore = defineObject(
    {
        name: "chore",
        plural: "chores",
        scope: space,
        fields: {
            owner: field.reference(principal.user).caller(),
            list: field.string(schema.string().min(1)),
            done: field.boolean(),
            assignee: field.string().optional(),
        },
        permissions: { read: relation("owner"), write: relation("owner") },
    },
    (method) => ({
        create: method.create("write", { fields: ["list", "done"] }),
        finish: method.updateMany("write", { fields: ["done"], match: ["list"] }),
        claim: method.updateMany("write", { fields: ["assignee"], match: ["list", "assignee"] }),
    }),
);

/** The database with the chores, their access and the journal. */
const choreDatabase = defineDatabase({
    name: "main",
    tables: [...callTables, ...chore.tables],
});

test.each(TEST_DIALECTS)(
    "change the matched objects the caller has the permission on at once, advancing their revisions, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, choreDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        const store = new ObjectStore({
            chain: async () => undefined,
            clock: Clock.system,
            principal: spacePrincipal(spaceId),
            servers: serversOf([chore]),
            database: storage.database,
            callKey: testCallKey,
            events: await testEvents(storage.database),
            origin: {
                package: chore.package,
            },
        });
        let context = userContext(users.frank, spaceId);
        const call = <Name extends CallableName<typeof chore>>(
            name: Name,
            input: NoInfer<RequestInput<typeof chore, Name>>,
        ) =>
            request(
                store,
                chore,
                name,
                spaceId,
                { requestId: RequestId.create(Date.now()), ...input },
                context,
            );

        // insert two home chores and one work chore of frank, and one home chore of grace
        await call("create", { list: "home", done: false });
        await call("create", { list: "home", done: false });
        await call("create", { list: "work", done: false });
        context = userContext(users.grace, spaceId);
        await call("create", { list: "home", done: false });

        // finish frank's home list and leave the work chore and grace's chore
        context = userContext(users.frank, spaceId);
        const finished = await call("finish", { where: { list: "home" }, done: true });
        const rows = await storage.database
            .select({
                owner: chore.table.ownerId,
                list: chore.table.list,
                done: chore.table.done,
                revision: chore.table.revision,
            })
            .from(chore.table);
        const summary = rows
            .map((row) => `${userName(row.owner)} ${row.list} ${String(row.done)} ${row.revision}`)
            .toSorted();

        // claim frank's unassigned home chores, leaving none
        const claimed = await call("claim", {
            where: { list: "home", assignee: null },
            assignee: "carol",
        });
        const again = await call("claim", {
            where: { list: "home", assignee: null },
            assignee: "dave",
        });
        expect([claimed, again]).toEqual([{ count: 2 }, { count: 0 }]);
        expect([finished, summary]).toEqual([
            { count: 2 },
            ["frank home true 2", "frank home true 2", "frank work false 1", "grace home false 1"],
        ]);
    },
);

test("refuse many updates at once of guarded fields", () => {
    expect(() =>
        ObjectDescription.of(
            defineObject(
                {
                    name: "secret-chore",
                    plural: "secret-chores",
                    scope: space,
                    fields: {
                        owner: field.reference(principal.user).caller(),
                        done: field.boolean().guardWrite("write"),
                    },
                    permissions: { read: relation("owner"), write: relation("owner") },
                },
                (method) => ({
                    finish: method.updateMany("write", { fields: ["done"], match: ["owner"] }),
                }),
            ),
        ),
    ).toThrow(
        new TypeError(
            "object secret-chore guards field done, so it cannot apply many updates at once",
        ),
    );
});
