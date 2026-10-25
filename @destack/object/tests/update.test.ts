import { testSimulation } from "../src/test/index.ts";
import { defineObject, field, ObjectDescription } from "../src/index.ts";
import type { CallableName } from "../src/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { principal, relation } from "@destack/access";
import { defineDatabase } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";

import { RequestId, Scope } from "@destack/sync";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { testCallKey } from "@destack/object/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { userContext, users, userName } from "./fixture/user.ts";
import { request, type RequestInput } from "./fixture/request.ts";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { TestObjectStore } from "../src/test/index.ts";

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
    ...tablesOf({
        packages: [objectPackage, { objects: { chore }, tables: [] }],
        follows: [],
        tables: [],
    }),
});

test("change the matched objects the caller has the permission on at once, advancing their revisions", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create(choreDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([chore]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: {
            package: chore.package,
        },
    });
    let context = userContext(users.frank, spaceId, { environment: world });
    const call = <Name extends CallableName<typeof chore>>(
        name: Name,
        input: NoInfer<RequestInput<typeof chore, Name>>,
    ) =>
        request(
            store,
            chore,
            name,
            spaceId,
            { requestId: RequestId.create(world), ...input },
            context,
        );

    // insert two home chores and one work chore of frank, and one home chore of grace
    await call("create", { list: "home", done: false });
    await call("create", { list: "home", done: false });
    await call("create", { list: "work", done: false });
    context = userContext(users.grace, spaceId, { environment: world });
    await call("create", { list: "home", done: false });

    // finish frank's home list and leave the work chore and grace's chore
    context = userContext(users.frank, spaceId, { environment: world });
    const finished = await call("finish", { where: { list: "home" }, done: true });
    const rows = await storage.database
        .select({
            owner: chore.table.ownerId,
            list: chore.table.list,
            done: chore.table.done,
            revision: chore.table.revision,
        })
        .from(chore.table);
    const described = rows
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
    expect([finished, described]).toEqual([
        { count: 2 },
        ["frank home true 2", "frank home true 2", "frank work false 1", "grace home false 1"],
    ]);
});

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
