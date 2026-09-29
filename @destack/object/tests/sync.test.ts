import { AuditOutbox, auditOutboxTables } from "@destack/audit/outbox";
import { expect, onTestFinished, test } from "@destack/test";
import { AuditRecorder } from "@destack/audit";
import { eq, type DatabaseConnection } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import type { QueryPage } from "@destack/sync";
import { identifier } from "@destack/schema";
import { Journal } from "@destack/service/database";
import { RequestId } from "@destack/service/request";
import { Bookmark } from "@destack/service/bookmark";
import type { ServiceContext } from "@destack/service/server";
import { v7 } from "uuid";
import { ObjectServer } from "../src/server/index.ts";
import { comment, folder, objectDatabase, request, task, taskVersion, team } from "./schema.ts";
import { principal } from "@destack/access";
import { openSpace } from "./fixture/space.ts";

/** How long a short grant lasts, in milliseconds. */
const GRANT_MILLISECONDS = 300;

/** The space containing the tasks. */
const spaceId = identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

/** Serve two tasks of the first user and follow what the second user sees. */
async function serveTasks(dialect: (typeof TEST_DIALECTS)[number]) {
    const storage = await TestDatabase.create(dialect, objectDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, spaceId);

    // insert two tasks of the first user
    const insert = async (title: string) => {
        const id = identifier("task").parse(`task-${v7()}`);
        await database.insert(task.table).values({
            id,
            scope: spaceId,
            owner: "user-1",
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
    let current = "user-1";
    const server = new ObjectServer({
        objects: { task, team, comment, taskVersion, folder },
        database,
        context: (_context: ServiceContext) => ({
            subjects: [principal.user.reference("global", current)],
            now: Date.now(),
            attributes: {},
        }),
        journal: new Journal(request),
        audit: AuditRecorder.service(new AuditOutbox(database), {
            package: task.package,
            service: "test",
        }),
    });
    const controller = new AbortController();
    onTestFinished(() => controller.abort());
    const context = {
        scope: spaceId,
        requireCaller: () => ({ id: current }),
        bookmark: new Bookmark(),
        observed: new Bookmark(),
        signal: controller.signal,
        request: new Request("https://test.local", { signal: controller.signal }),
    } as unknown as ServiceContext;
    const execute = async (
        caller: string,
        name: string,
        input: Record<string, unknown>,
        object: typeof task | typeof team = task,
    ) => {
        current = caller;
        const result = await server.call(object, name, { spaceId, ...input }, context);
        current = "user-2";

        return result as { id: string };
    };
    current = "user-2";
    const pages = server.sync(spaceId, context);
    const next = async (): Promise<[boolean, string, unknown, unknown][]> => {
        // skip the bare pages that only move the position, reading the next page of changes
        let page = (await pages.next()).value as QueryPage;
        while (page.changes.length === 0) {
            page = (await pages.next()).value as QueryPage;
        }

        return page.changes.map((change) => [
            page.reset,
            change.operation,
            change.row.title,
            change.row.estimate,
        ]);
    };

    // read a snapshot of every object type, holding none of them yet
    const snapshot: QueryPage[] = [];
    while (!snapshot.at(-1)?.complete) {
        snapshot.push((await pages.next()).value as QueryPage);
    }
    expect(snapshot.flatMap((page) => page.changes)).toEqual([]);

    return { database, execute, next, first, second };
}

test.each(TEST_DIALECTS)(
    "follow the tasks a caller sees as grants, changes and ownership move them on %s",
    async (dialect) => {
        const { database, execute, next, first, second } = await serveTasks(dialect);

        // hold the task a grant relates the caller to
        const viewer = principal.user.reference("global", "user-2");
        const shared = await execute("user-1", "grant", {
            id: first,
            requestId: RequestId.create(),
            relation: "viewer",
            subject: viewer,
        });
        expect(await next()).toEqual([[false, "insert", "First", undefined]]);

        // send changes of visible tasks, never of invisible ones
        await execute("user-1", "update", {
            id: second,
            requestId: RequestId.create(),
            revision: 1,
            title: "Hidden",
        });
        await execute("user-1", "update", {
            id: first,
            requestId: RequestId.create(),
            revision: 1,
            title: "Renamed",
        });
        expect(await next()).toEqual([[false, "update", "Renamed", undefined]]);

        // hold the task once ownership moves to the caller, and drop it when ownership leaves
        await database.update(task.table).set({ owner: "user-2" }).where(eq(task.table.id, second));
        expect(await next()).toEqual([[false, "insert", "Hidden", 5]]);
        await database.update(task.table).set({ owner: "user-1" }).where(eq(task.table.id, second));
        expect(await next()).toEqual([[false, "delete", undefined, undefined]]);

        // leave the task once the grant is revoked
        await execute("user-1", "revoke", {
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
        const teamId = identifier("team").parse(`team-${v7()}`);
        await database.insert(team.table).values({
            id: teamId,
            scope: spaceId,
            owner: "user-1",
            createdAt: Date.now(),
            updatedAt: Date.now(),
        });
        await execute("user-1", "grant", {
            id: first,
            requestId: RequestId.create(),
            relation: "viewer",
            subject: { ...team.reference(spaceId, teamId), relation: "member" },
        });

        // hold the task once the caller joins the team, a relationship on the team alone
        await execute(
            "user-1",
            "grant",
            {
                id: teamId,
                requestId: RequestId.create(),
                relation: "member",
                subject: principal.user.reference("global", "user-2"),
            },
            team,
        );
        expect(await next()).toEqual([[false, "insert", "First", undefined]]);
    },
);

test.each(TEST_DIALECTS)(
    "let go of a task once the grant holding it expires, with no change in the log, on %s",
    async (dialect) => {
        const { execute, next, first } = await serveTasks(dialect);

        // hold a task through a grant that expires shortly
        await execute("user-1", "grant", {
            id: first,
            requestId: RequestId.create(),
            relation: "viewer",
            subject: principal.user.reference("global", "user-2"),
            expiresAt: Date.now() + GRANT_MILLISECONDS,
        });
        expect(await next()).toEqual([[false, "insert", "First", undefined]]);

        // drop it once the grant expires
        expect(await next()).toEqual([[false, "delete", undefined, undefined]]);
    },
);
