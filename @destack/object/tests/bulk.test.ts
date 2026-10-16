import { Package } from "@destack/package";
import type { CallableName } from "../src/index.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { principal, relation } from "@destack/access";

import { asc, defineDatabase, type DatabaseConnection } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { definePackage } from "@destack/runtime";
import { schema, Clock, aligned, type Identifier } from "@destack/schema";
import { RequestId } from "@destack/runtime/request";
import { ObjectReference } from "@destack/sync";
import { testCallKey } from "@destack/runtime/test";
import { callTables, testEvents, serversOf } from "../src/test/index.ts";
import { ObjectClient } from "../src/client/index.ts";
import { defineObject, field } from "../src/index.ts";
import { ObjectStore, defineServer } from "../src/server/index.ts";
import { serveObjects } from "./fixture/device.ts";
import { openSpace, space, spacePrincipal, unmoved } from "./fixture/space.ts";
import { userContext, users, userName } from "./fixture/user.ts";
import { request, type RequestInput } from "./fixture/request.ts";

/** The space containing the labels. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000005");

/** Labels with names unique in their space, created and trashed many at a time by their owner. */
const label = defineObject(
    {
        name: "label",
        plural: "labels",
        scope: space,
        fields: {
            owner: field.reference(principal.user).caller(),
            name: field.string(schema.string().min(1)),
            color: field.string(),
        },
        permissions: { read: relation("owner"), write: relation("owner") },
        recoverable: { within: { days: 30 }, by: "write" },
        constraints: { name: { kind: "unique", on: ["scope", "name"] } },
    },
    (method) => ({
        create: method.create("write", { fields: ["name", "color"] }),
        createMany: method.createMany("write", { fields: ["name", "color"] }),
        deleteMany: method.deleteMany("write", { match: ["color"] }),
        recolor: method.updateItems("write", { fields: ["color"] }),
    }),
);

/** The database with the labels, their access and the journal. */
const labelDatabase = defineDatabase({ name: "main", tables: [...callTables, ...label.tables] });

/** Serve the labels of the space to the user of each call. */
async function serveLabels(dialect: (typeof TEST_DIALECTS)[number]) {
    // open the space and its store
    const storage = await TestDatabase.create(dialect, labelDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([label]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: { package: label.package },
    });

    // call as a user, and read every label as name, color, owner and trash state
    const call = <Name extends CallableName<typeof label>>(
        user: Identifier<"user">,
        name: Name,
        input: NoInfer<RequestInput<typeof label, Name>>,
    ) =>
        request(
            store,
            label,
            name,
            spaceId,
            { requestId: RequestId.create(Date.now()), ...input },
            userContext(user, spaceId),
        );
    const read = async () =>
        (await rows(storage.database)).map(
            (row) =>
                `${userName(row.ownerId)} ${row.name} ${row.color} ${row.deletedAt === null ? "live" : "trashed"}`,
        );

    return { storage, store, call, read };
}

/** Read every label by name. */
function rows(database: DatabaseConnection) {
    return database.select().from(label.table).orderBy(asc(label.table.name));
}

test.each(TEST_DIALECTS)(
    "create one object per item in one call, returning their identifiers in item order, on %s",
    async (dialect) => {
        const { storage, call } = await serveLabels(dialect);

        // create three labels, one under a chosen identifier
        const chosen = label.generateId();
        const created = await call(users.frank, "createMany", {
            items: [
                { name: "urgent", color: "red" },
                { id: chosen, name: "later", color: "blue" },
                { name: "done", color: "green" },
            ],
        });

        // write each as its own creation would: owned by the caller at the first revision
        const written = (await rows(storage.database)).map((row) => [
            row.id,
            row.name,
            row.color,
            row.ownerId,
            row.revision,
            ObjectReference.at("createdBy", row),
        ]);
        const urgent = aligned(created.ids, 0);
        const done = aligned(created.ids, 2);
        const author = principal.user.reference("universe", users.frank);
        expect(created.ids[1]).toBe(chosen);
        expect(written).toEqual([
            [done, "done", "green", users.frank, 1, author],
            [chosen, "later", "blue", users.frank, 1, author],
            [urgent, "urgent", "red", users.frank, 1, author],
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "refuse the whole batch for a taken unique name or an invalid item, naming the item, on %s",
    async (dialect) => {
        const { call, read } = await serveLabels(dialect);
        await call(users.frank, "create", { name: "urgent", color: "red" });

        // refuse a name another label has, one the batch takes twice, and an empty name
        const refusals = [
            await refusal(
                call(users.frank, "createMany", {
                    items: [
                        { name: "later", color: "blue" },
                        { name: "urgent", color: "red" },
                    ],
                }),
            ),
            await refusal(
                call(users.frank, "createMany", {
                    items: [
                        { name: "later", color: "blue" },
                        { name: "done", color: "green" },
                        { name: "later", color: "red" },
                    ],
                }),
            ),
            await refusal(
                call(users.frank, "createMany", {
                    items: [
                        { name: "later", color: "blue" },
                        { name: "", color: "green" },
                    ],
                }),
            ),
        ];

        // leave only the first label
        expect([refusals, await read()]).toEqual([
            [
                ["CONFLICT", "items.1: another label has the same scope and name"],
                ["CONFLICT", "items.2: another label has the same scope and name"],
                [
                    "BAD_REQUEST",
                    "invalid input: items.1.name: too small: expected string to have >=1 characters",
                ],
            ],
            ["frank urgent red live"],
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "delete the matching objects the caller may delete into the trash, passing trashed ones, on %s",
    async (dialect) => {
        const { call, read } = await serveLabels(dialect);

        // create two red and one blue label of frank, and a red one of grace
        await call(users.frank, "createMany", {
            items: [
                { name: "urgent", color: "red" },
                { name: "late", color: "red" },
                { name: "later", color: "blue" },
            ],
        });
        await call(users.grace, "create", { name: "alarm", color: "red" });

        // trash frank's red labels once, finding none the second time
        const deleted = [
            await call(users.frank, "deleteMany", { where: { color: "red" } }),
            await call(users.frank, "deleteMany", { where: { color: "red" } }),
        ];
        expect([deleted, await read()]).toEqual([
            [{ count: 2 }, { count: 0 }],
            [
                "grace alarm red live",
                "frank late red trashed",
                "frank later blue live",
                "frank urgent red trashed",
            ],
        ]);
    },
);

test("undo a bulk creation by deleting each created object on the device and the server", async () => {
    // serve the labels of a space
    const { storage } = await serveLabels("sqlite");
    const labels = definePackage({ objects: { label } });
    const served = serveObjects(
        aligned(
            new ObjectStore({
                servers: [defineServer(labels, {})],
                chain: async () => undefined,
                clock: Clock.system,
                origin: { package: Package.of(labels) },
                principal: spacePrincipal(spaceId),
                callKey: testCallKey,
                events: await testEvents(storage.database),
                database: storage.database,
            }).packages,
            0,
        ),
        spaceId,
    );

    // store the space's labels on a device that pushes and follows
    const local = await TestDatabase.create("sqlite", ObjectClient.tables({ label }), {
        storage: "file",
    });
    onTestFinished(() => local.close());
    const client = await ObjectClient.open({
        clock: Clock.system,
        database: local.database,
        objects: { label },
        scope: spaceId,
        caller: principal.user.reference("universe", users.alice),
        endpoint: served.endpoint(users.alice),
        reconnect: unmoved,
    });
    client.queryType(label).findMany().subscribe();
    const stopping = new AbortController();
    const errors: unknown[] = [];
    const loops = [
        client.follow(stopping.signal, (error) => errors.push(error)),
        client.push(stopping.signal, (error) => errors.push(error)),
    ];

    // create two labels under the identifiers the device predicted
    const created = client.mutate(label).createMany({
        items: [
            { name: "urgent", color: "red" },
            { name: "later", color: "blue" },
        ],
    });
    const { ids } = await created.predicted;
    await created.confirmed;
    const labelsIn = async (database: DatabaseConnection) =>
        (await rows(database)).map((row) => [row.id, row.deletedAt === null]);
    const before = [await labelsIn(client.database), await labelsIn(storage.database)];

    // undo the creation into the trash on the device and through the server
    await client.undo().confirmed;
    stopping.abort();
    await Promise.all(loops);
    expect([before, await labelsIn(client.database), await labelsIn(storage.database), errors]).toEqual(
        [
            [
                [
                    [ids[1], true],
                    [ids[0], true],
                ],
                [
                    [ids[1], true],
                    [ids[0], true],
                ],
            ],
            [],
            [
                [ids[1], false],
                [ids[0], false],
            ],
            [],
        ],
    );
    await client.close();
});

test.each(TEST_DIALECTS)(
    "run a sent createMany once though it arrives twice, replaying its request, on %s",
    async (dialect) => {
        // deliver the same sent mutation twice as frank
        const { store, read } = await serveLabels(dialect);
        const sent = {
            id: RequestId.create(Date.now()),
            calls: [
                label.call("createMany", {
                    spaceId,
                    items: [
                        { name: "red", color: "red" },
                        { name: "blue", color: "blue" },
                    ],
                }),
            ],
        };
        const user = principal.user.reference("universe", users.frank);
        await store.receive(sent, user);
        await store.receive(sent, user);

        expect(await read()).toEqual(["frank blue blue live", "frank red red live"]);
    },
);

/** Build labels of one color for a bulk creation of a size. */
function items(size: number) {
    return Array.from({ length: size }, (_, index) => ({
        name: `label-${String(size)}-${String(index)}`,
        color: `color-${String(size)}`,
    }));
}

test.each(TEST_DIALECTS)(
    "create and trash a thousand objects on %s in as many statements as one object",
    async (dialect) => {
        const { storage, call } = await serveLabels(dialect);
        const state = storage.database.driver.state;

        // count the statements of creating one and a thousand labels, then of trashing the thousand
        const counted = async (run: () => Promise<unknown>) => {
            const before = state.statements;
            await run();

            return state.statements - before;
        };
        const statements = {
            createOne: await counted(() => call(users.frank, "createMany", { items: items(1) })),
            createThousand: await counted(() =>
                call(users.frank, "createMany", { items: items(1000) }),
            ),
            trashThousand: await counted(() =>
                call(users.frank, "deleteMany", { where: { color: "color-1000" } }),
            ),
        };

        // spend a few statements per batch of rows, PostgreSQL assigning its transaction identity
        const extra = dialect === "postgresql" ? 1 : 0;
        expect(statements).toEqual({
            createOne: 16 + extra,
            createThousand: 16 + extra,
            trashThousand: 18 + extra,
        });
    },
);

test.each(TEST_DIALECTS)(
    "update listed objects at their revisions in one call, and refuse the whole batch for one stale revision or one the caller may not change, on %s",
    async (dialect) => {
        const { storage, call, read } = await serveLabels(dialect);
        const { ids } = await call(users.frank, "createMany", {
            items: [
                { name: "urgent", color: "red" },
                { name: "later", color: "blue" },
            ],
        });
        const urgent = aligned(ids, 0);
        const later = aligned(ids, 1);
        const { ids: alarms } = await call(users.grace, "createMany", {
            items: [{ name: "alarm", color: "red" }],
        });

        // recolor both labels at their first revision, then refuse one stale revision and another's label
        const updated = await call(users.frank, "recolor", {
            items: [
                { id: urgent, revision: 1, color: "green" },
                { id: later, revision: 1, color: "green" },
            ],
        });
        const refusals = [
            await refusal(
                call(users.frank, "recolor", {
                    items: [
                        { id: urgent, revision: 2, color: "white" },
                        { id: later, revision: 1, color: "white" },
                    ],
                }),
            ),
            await refusal(
                call(users.frank, "recolor", {
                    items: [
                        { id: urgent, revision: 2, color: "white" },
                        { id: aligned(alarms, 0), revision: 1, color: "white" },
                    ],
                }),
            ),
        ];

        // take the first update alone, each label at its next revision
        const revisions = (await rows(storage.database)).map((row) => [row.name, row.revision]);
        expect({ updated, refusals, labels: await read(), revisions }).toEqual({
            updated: { count: 2 },
            refusals: [
                ["CONFLICT", "label revision has changed"],
                ["NOT_FOUND", "label not found"],
            ],
            labels: ["grace alarm red live", "frank later green live", "frank urgent green live"],
            revisions: [
                ["alarm", 1],
                ["later", 2],
                ["urgent", 2],
            ],
        });
    },
);
