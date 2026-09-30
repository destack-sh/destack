import { expect, onTestFinished, test } from "@destack/test";
import {
    accessProposal,
    accessRelationship,
    anyone,
    decisionTables,
    none,
    principal,
    Relationship,
    relation,
} from "@destack/access";
import { copyScope } from "@destack/access/test";
import { DirectoryDatabase } from "@destack/directory";
import { directoryTables } from "@destack/directory";
import type { AuditRecorder } from "@destack/audit";
import type { DatabaseConnection } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { defineObject, field, method } from "@destack/object";
import { ObjectServer } from "@destack/object/server";
import { identifier } from "@destack/schema";
import { defineJournal, Journal } from "@destack/service/database";
import { subjectContext } from "@destack/service/test";
import { ServiceError } from "@destack/service/error";
import { RequestId } from "@destack/service/request";
import { Scope, Feed, replica, replicaTables } from "@destack/sync";
import { v7 } from "uuid";
import { ZoneTransfer } from "../src/server/index.ts";

/** Replayable method requests. */
const request = defineJournal("journal");

/** The spaces of the keys, readable by anyone. */
const space = defineObject({
    name: "space",
    plural: "spaces",
    scope: "universe",
    isScope: true,
    fields: {},
    relations: { reader: { subjects: [anyone.all()] } },
    permissions: { read: relation("reader"), share: none() },
    shareable: { by: "share" },
});

/** The space the transfer moves. */
const spaceId = identifier("space").parse("space-01996ab0-0000-7000-8000-000000000011");

/** Keys that a space's owners have, logged whole. */
const key = defineObject({
    name: "key",
    plural: "keys",
    scope: space,
    fields: {
        owner: field.reference(principal.user).caller(),
        name: field.string(),
    },
    permissions: { read: relation("owner"), write: relation("owner") },
    methods: {
        get: method.get("read"),
        create: method.create("write", { fields: ["name"] }),
        update: method.update("write", { fields: ["name"] }),
    },
});

test.each(TEST_DIALECTS)(
    "hand a space's database to another cell, and send clients on to it: follow, fence, reach, verify, promote and place on %s",
    async (dialect) => {
        // open the source and target databases of the space, and the directory keeping its zone
        const tables = [...key.tables, request, ...replicaTables];
        const [source, target, global] = await Promise.all([
            TestDatabase.create(dialect, tables, { isMigrated: true }),
            TestDatabase.create(dialect, tables, { isMigrated: true }),
            TestDatabase.create(dialect, directoryTables, {
                isMigrated: true,
            }),
        ]);
        onTestFinished(async () => {
            await Promise.all([source.close(), target.close(), global.close()]);
        });
        await openSpace(source.database);
        const directory = new DirectoryDatabase(global.database);
        const zone = { id: spaceId, scope: "universe", cell: "host-a", epoch: 1 };
        await directory.place(zone);

        // write keys to the space as its owner
        const serve = (database: DatabaseConnection) =>
            new ObjectServer({
                objects: { key },
                database,
                context: () => ({
                    subjects: [principal.user.reference("universe", "owner")],
                    now: Date.now(),
                    attributes: {},
                }),
                journal: new Journal(request),
                audit: () =>
                    ({ record: async () => {} }) as unknown as AuditRecorder<DatabaseConnection>,
            });
        const context = subjectContext(principal.user.reference("universe", "owner"), spaceId);
        const create = (database: DatabaseConnection, name: string) =>
            serve(database).call(
                key,
                "create",
                { spaceId, requestId: RequestId.create(), name },
                context,
            );
        const deployed = (await create(source.database, "deploy")) as { id: string };

        // follow the space as a reader does, from its first page
        const reader = (signal: AbortSignal) =>
            subjectContext(principal.user.reference("universe", "owner"), spaceId, signal);
        const read = () =>
            serve(source.database).sync(spaceId, reader(AbortSignal.timeout(4000)), {});
        const following = read();
        await following.next();

        // follow the source's rows on the target while the source serves them
        const transfer = new ZoneTransfer({
            zone,
            target: "host-b",
            tables: [...decisionTables, accessProposal, key.table],
        });
        const feed = new Feed(source.database, [
            ...decisionTables,
            accessProposal,
            key.table,
            replica,
        ]);
        const copying = new AbortController();
        const copied = transfer.follow(
            target.database,
            (after, stop) => feed.subscribe(transfer.replica.queries, after, stop),
            copying.signal,
        );

        // fence the source to redirect further writes to the target cell
        const fenced = await transfer.fence(source.database, Date.now());
        const moved = new ServiceError("MOVED", {
            status: 421,
            message: `${spaceId} moves to host-b`,
            data: { scope: spaceId, cell: "host-b" },
        });
        await expect(create(source.database, "late")).rejects.toEqual(moved);

        // end the reads following the moved space, and answer new ones, with its new cell too
        await expect(following.next()).rejects.toEqual(moved);
        await expect(read().next()).rejects.toEqual(moved);
        await expect(
            serve(source.database).call(key, "get", { spaceId, id: deployed.id }, context),
        ).rejects.toEqual(moved);

        // reach the fenced position on the target, and verify the copy
        expect(await transfer.reach(target.database, fenced, AbortSignal.timeout(4000))).toBe(true);
        copying.abort();
        await copied;
        await transfer.verify(target.database, fenced);

        // own the rows on the target and locate the zone there at the next epoch
        await transfer.promote(target.database);
        await create(target.database, "after");
        expect([
            await transfer.activate(directory),
            await directory.locate(spaceId),
            (await target.database.select().from(key.table)).map((row) => row.name),
        ]).toEqual([
            { ...zone, cell: "host-b", epoch: 2 },
            { ...zone, cell: "host-b", epoch: 2 },
            ["deploy", "after"],
        ]);
    },
);

/** Record the space as a copy of its home's access has it, with anyone as its reader. */
async function openSpace(database: DatabaseConnection): Promise<void> {
    // record the space's scope
    const object = space.reference(Scope.universe.id, spaceId);
    await copyScope(database, object);

    // relate anyone to the space as its reader
    await database.insert(accessRelationship).values(
        Relationship.encode(
            {
                id: identifier("relationship").parse(`relationship-${v7()}`),
                object,
                relation: "reader",
                subject: anyone.reference("*", "*"),
                createdAt: 0,
                expiresAt: null,
            },
            spaceId,
        ),
    );
}
