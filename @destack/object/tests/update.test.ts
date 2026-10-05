import type { CallableName } from "../src/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { principal, relation } from "@destack/access";
import { journal } from "@destack/audit";
import { defineDatabase } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";

import { RequestId } from "@destack/service/request";
import { defineObject, field } from "../src/index.ts";
import { ObjectServer } from "../src/server/index.ts";
import { openSpace, space } from "./fixture/space.ts";
import { testCallKey } from "@destack/service/test";
import { userContext } from "./fixture/user.ts";

/** The space containing the chores. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000003");

/** Chores on lists, finished a list at a time. */
const chore = defineObject({
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
    methods: (method) => ({
        list: method.list("read"),
        create: method.create("write", { fields: ["list", "done"] }),
        finish: method.updateMany("write", { fields: ["done"], match: ["list"] }),
        claim: method.updateMany("write", { fields: ["assignee"], match: ["list", "assignee"] }),
    }),
});

/** The database with the chores, their access and the journal. */
const choreDatabase = defineDatabase({
    name: "main",
    tables: [journal, ...chore.tables],
});

test.each(TEST_DIALECTS)(
    "change the matched objects the caller has the permission on at once, advancing their revisions, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, choreDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        const server = new ObjectServer({
            objects: { chore },
            database: storage.database,
            callKey: testCallKey,
            origin: {
                package: chore.package,
                service: "test",
            },
        });
        let context = userContext("user-1", spaceId);
        const call = <Name extends CallableName<typeof chore>>(
            name: Name,
            input: Record<string, unknown>,
        ) =>
            server.call(chore, name, { spaceId, requestId: RequestId.create(), ...input }, context);

        // insert two home chores and one work chore of user-1, and one home chore of user-2
        await call("create", { list: "home", done: false });
        await call("create", { list: "home", done: false });
        await call("create", { list: "work", done: false });
        context = userContext("user-2", spaceId);
        await call("create", { list: "home", done: false });

        // finish user-1's home list and leave the work chore and user-2's chore
        context = userContext("user-1", spaceId);
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
            .map((row) => `${row.owner} ${row.list} ${String(row.done)} ${row.revision}`)
            .toSorted();

        // claim user-1's unassigned home chores, leaving none
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
            [
                "user-1 home true 2",
                "user-1 home true 2",
                "user-1 work false 1",
                "user-2 home false 1",
            ],
        ]);
    },
);

test("refuse many updates at once of guarded fields", () => {
    expect(() =>
        defineObject({
            name: "secret-chore",
            plural: "secret-chores",
            scope: space,
            fields: {
                owner: field.reference(principal.user).caller(),
                done: field.boolean().guardWrite("write"),
            },
            permissions: { read: relation("owner"), write: relation("owner") },
            methods: (method) => ({
                finish: method.updateMany("write", { fields: ["done"], match: ["owner"] }),
            }),
        }),
    ).toThrow(
        new TypeError("object secret-chore guards field done, which many updates at once skip"),
    );
});
