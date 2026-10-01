import { AuditOutbox } from "@destack/audit/outbox";
import { expect, onTestFinished, test } from "@destack/test";
import { principal, relation } from "@destack/access";
import { AuditRecorder } from "@destack/audit";
import { defineDatabase } from "@destack/db/declare";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { identifier, schema } from "@destack/schema";
import { Bookmark } from "@destack/service/bookmark";
import { defineJournal, Journal } from "@destack/service/database";
import { RequestId } from "@destack/service/request";
import type { ServiceContext } from "@destack/service/server";
import { defineObject, field, method } from "../src/index.ts";
import { ObjectServer } from "../src/server/index.ts";
import { openSpace, space } from "./fixture/space.ts";
import { testJournalKey } from "@destack/service/test";

/** The space containing the chores. */
const spaceId = identifier("space").parse("space-01996ab0-0000-7000-8000-000000000003");

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
    methods: {
        list: method.list("read"),
        create: method.create("write", { fields: ["list", "done"] }),
        finish: method.updateMany("write", { fields: ["done"], match: ["list"] }),
        claim: method.updateMany("write", { fields: ["assignee"], match: ["list", "assignee"] }),
    },
});

/** Replayable chore requests. */
const journal = defineJournal("journal");

/** The database holding the chores, their access and the journal. */
const choreDatabase = defineDatabase({
    name: "main",
    tables: [journal, ...chore.tables],
});

test.each(TEST_DIALECTS)(
    "change the matched objects the caller holds the permission on at once, advancing their revisions, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, choreDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        let current = "user-1";
        const server = new ObjectServer({
            objects: { chore },
            database: storage.database,
            context: () => ({
                subjects: [principal.user.reference("universe", current)],
                now: Date.now(),
                attributes: {},
            }),
            journal: new Journal(journal, testJournalKey),
            audit: AuditRecorder.service(new AuditOutbox(storage.database), {
                package: chore.package,
                service: "test",
            }),
        });
        const context = {
            scope: spaceId,
            requireCaller: () => ({ id: current }),
            bookmark: new Bookmark(),
            observed: new Bookmark(),
        } as unknown as ServiceContext;
        const call = (name: string, input: Record<string, unknown>) =>
            server.call(chore, name, { spaceId, requestId: RequestId.create(), ...input }, context);

        // hold two home chores and one work chore of user-1, and one home chore of user-2
        await call("create", { list: "home", done: false });
        await call("create", { list: "home", done: false });
        await call("create", { list: "work", done: false });
        current = "user-2";
        await call("create", { list: "home", done: false });

        // finish user-1's home list and leave the work chore and user-2's chore
        current = "user-1";
        const finished = await call("finish", { where: { list: "home" }, done: true });
        const rows = await storage.database
            .select({
                owner: chore.table.owner,
                list: chore.table.list,
                done: chore.table.done,
                revision: chore.table.revision,
            })
            .from(chore.table);
        const summary = rows
            .map((row) => `${String(row.owner)} ${row.list} ${String(row.done)} ${row.revision}`)
            .sort();

        // claim user-1's unassigned home chores, then none are left
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
                done: field.boolean().guard({ write: "write" }),
            },
            permissions: { read: relation("owner"), write: relation("owner") },
            methods: { finish: method.updateMany("write", { fields: ["done"], match: ["owner"] }) },
        }),
    ).toThrow(
        new TypeError("object secret-chore guards field done, which many updates at once skip"),
    );
});
