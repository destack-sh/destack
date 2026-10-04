import type { CallableName, CallOutput } from "../src/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { eq } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import type { Page } from "@destack/sync";
import { schema } from "@destack/schema";

import { RequestId } from "@destack/service/request";
import { v7 } from "uuid";
import { ObjectServer } from "../src/server/index.ts";
import { comment, folder, objectDatabase, task, taskVersion, team } from "./schema.ts";
import { principal } from "@destack/access";
import { openSpace } from "./fixture/space.ts";
import { testCallKey } from "@destack/service/test";
import { userContext } from "./fixture/user.ts";

/** How long a short grant lasts, in milliseconds. */
const GRANT_MILLISECONDS = 300;

/** The space containing the tasks. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

/** Serve two tasks of the first user and follow what the second user sees. */
async function serveTasks(dialect: (typeof TEST_DIALECTS)[number]) {
    const storage = await TestDatabase.create(dialect, objectDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, spaceId);

    // insert two tasks of the first user
    const insert = async (title: string) => {
        const id = schema.identifier("task").parse(`task-${v7()}`);
        await database.insert(task.table).values({
            id,
            scope: spaceId,
            ownerId: "user-1",
            title,
            estimate: 5,
            createdAt: Date.now(),
            updatedAt: Date.now(),
        });

        return id;
    };
    const first = await insert("First");
    const second = await insert("Second");

    // serve methods as each user and follow the tasks the second user sees
    const server = new ObjectServer({
        objects: { task, team, comment, taskVersion, folder },
        database,
        callKey: testCallKey,
        origin: {
            package: task.package,
            service: "test",
        },
    });
    const controller = new AbortController();
    onTestFinished(() => controller.abort());
    const context = userContext("user-2", spaceId, { signal: controller.signal });
    const execute = async <
        Object extends typeof task | typeof team,
        Name extends CallableName<Object>,
    >(
        caller: string,
        object: Object,
        name: Name,
        input: Record<string, unknown>,
    ): Promise<CallOutput<Object, Name>> => {
        return server.call(object, name, { spaceId, ...input }, userContext(caller, spaceId));
    };
    const pages = server.source.relayed(
        server.source.queriesShape.subscription({
            name: "objects",
            scope: spaceId,
            below: spaceId,
            parameters: {},
        }),
        context,
    );
    const next = async (): Promise<[boolean, string, unknown, unknown][]> => {
        // skip the bare pages that only move the position and read the next page of changes
        let page = await nextPage(pages);
        while (page.changes.length === 0) {
            page = await nextPage(pages);
        }

        return page.changes.map((change) => [
            page.reset,
            change.operation,
            change.row["title"],
            change.row["estimate"],
        ]);
    };

    // read a snapshot of every object type while none exist yet
    const snapshot: Page[] = [];
    while (snapshot.at(-1)?.complete !== true) {
        snapshot.push(await nextPage(pages));
    }
    expect(snapshot.flatMap((page) => page.changes)).toEqual([]);

    return { database, execute, next, first, second };
}

test.each(TEST_DIALECTS)(
    "follow the tasks a caller sees as grants, changes and ownership move them on %s",
    async (dialect) => {
        const { database, execute, next, first, second } = await serveTasks(dialect);

        // select the task a grant relates the caller to
        const viewer = principal.user.reference("universe", "user-2");
        const shared = await execute("user-1", task, "grant", {
            id: first,
            requestId: RequestId.create(),
            relation: "viewer",
            subject: viewer,
        });
        expect(await next()).toEqual([[false, "insert", "First", undefined]]);

        // send changes of visible tasks, never of invisible ones
        await execute("user-1", task, "update", {
            id: second,
            requestId: RequestId.create(),
            revision: 1,
            title: "Hidden",
        });
        await execute("user-1", task, "update", {
            id: first,
            requestId: RequestId.create(),
            revision: 1,
            title: "Renamed",
        });
        expect(await next()).toEqual([[false, "update", "Renamed", undefined]]);

        // select the task once ownership moves to the caller, and drop it when ownership leaves
        await database
            .update(task.table)
            .set({ ownerId: "user-2" })
            .where(eq(task.table.id, second));
        expect(await next()).toEqual([[false, "insert", "Hidden", 5]]);
        await database
            .update(task.table)
            .set({ ownerId: "user-1" })
            .where(eq(task.table.id, second));
        expect(await next()).toEqual([[false, "delete", undefined, undefined]]);

        // leave the task once the grant is revoked
        await execute("user-1", task, "revoke", {
            id: first,
            requestId: RequestId.create(),
            relationshipId: shared.id,
        });
        expect(await next()).toEqual([[false, "delete", undefined, undefined]]);
    },
);

test.each(TEST_DIALECTS)(
    "follow the tasks a team grants once the caller joins the team on %s",
    async (dialect) => {
        const { database, execute, next, first } = await serveTasks(dialect);

        // let the first user's team view the first task, before the caller belongs to it
        const teamId = schema.identifier("team").parse(`team-${v7()}`);
        await database.insert(team.table).values({
            id: teamId,
            scope: spaceId,
            ownerId: "user-1",
            createdAt: Date.now(),
            updatedAt: Date.now(),
        });
        await execute("user-1", task, "grant", {
            id: first,
            requestId: RequestId.create(),
            relation: "viewer",
            subject: { ...team.reference(spaceId, teamId), relation: "member" },
        });

        // select the task once the caller joins the team, a relationship on the team alone
        await execute("user-1", team, "grant", {
            id: teamId,
            requestId: RequestId.create(),
            relation: "member",
            subject: principal.user.reference("universe", "user-2"),
        });
        expect(await next()).toEqual([[false, "insert", "First", undefined]]);
    },
);

test.each(TEST_DIALECTS)(
    "let go of a task once the grant relating it expires, with no change in the log, on %s",
    async (dialect) => {
        const { execute, next, first } = await serveTasks(dialect);

        // select a task through a grant that expires shortly
        await execute("user-1", task, "grant", {
            id: first,
            requestId: RequestId.create(),
            relation: "viewer",
            subject: principal.user.reference("universe", "user-2"),
            expiresAt: Date.now() + GRANT_MILLISECONDS,
        });
        expect(await next()).toEqual([[false, "insert", "First", undefined]]);

        // drop it once the grant expires
        expect(await next()).toEqual([[false, "delete", undefined, undefined]]);
    },
);

/** Read the next page of a sync stream, refusing an ended stream. */
async function nextPage(pages: AsyncGenerator<Page>): Promise<Page> {
    const read = await pages.next();
    if (read.done === true) {
        throw new TypeError("the sync stream ended");
    }

    return read.value;
}
