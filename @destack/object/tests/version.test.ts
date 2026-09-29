import { AuditOutbox, auditOutboxTables } from "@destack/audit/outbox";
import { expect, onTestFinished, test } from "@destack/test";
import { AuditRecorder } from "@destack/audit";
import type { DatabaseConnection } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { identifier } from "@destack/schema";
import { Journal } from "@destack/service/database";
import { RequestId } from "@destack/service/request";
import { Bookmark } from "@destack/service/bookmark";
import type { ServiceContext } from "@destack/service/server";
import { v7 } from "uuid";
import { defineObject, field, method } from "../src/index.ts";
import { ObjectServer } from "../src/server/index.ts";
import { objectDatabase, request, task, taskVersion } from "./schema.ts";
import { principal } from "@destack/access";
import { openSpace, space } from "./fixture/space.ts";

/** The space containing the objects. */
const spaceId = identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

test.each(TEST_DIALECTS)(
    "number versions within their parent and point the parent at its current one on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, objectDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        const database = storage.database;
        await openSpace(database, spaceId);
        const taskId = identifier("task").parse(`task-${v7()}`);
        await database.insert(task.table).values({
            id: taskId,
            scope: spaceId,
            owner: "user-1",
            title: "Plan",
            createdAt: Date.now(),
            updatedAt: Date.now(),
        });

        // create versions numbered from one, taking the number the runtime assigns
        const server = new ObjectServer({
            objects: { task, taskVersion },
            database,
            context: () => ({
                subjects: [principal.user.reference("global", "user-1")],
                now: Date.now(),
                attributes: {},
            }),
            journal: new Journal(request),
            audit: AuditRecorder.service(new AuditOutbox(database), {
                package: task.package,
                service: "test",
            }),
        });
        const context = {
            scope: spaceId,
            requireCaller: () => ({ id: "user-1" }),
            bookmark: new Bookmark(),
            observed: new Bookmark(),
        } as unknown as ServiceContext;
        const execute = (name: string, input: Record<string, unknown>) =>
            server.call(
                taskVersion,
                name,
                { spaceId, requestId: RequestId.create(), ...input },
                context,
            ) as Promise<typeof taskVersion.table.$inferSelect>;
        const first = await execute("create", { parentId: taskId, title: "Plan" });
        const second = await execute("create", { parentId: taskId, title: "Ship" });
        expect([first.number, second.number]).toEqual([1, 2]);

        // point the task at each version in turn through its own update
        const point = async (id: string, revision: number) =>
            (await server.call(
                task,
                "update",
                { spaceId, requestId: RequestId.create(), id: taskId, revision, current: id },
                context,
            )) as { current: string; revision: number };
        const pointed = await point(first.id, 1);
        expect((await point(second.id, pointed.revision)).current).toBe(second.id);

        // refuse updates to immutable versions
        expect(() =>
            defineObject({
                name: "draft",
                plural: "drafts",
                scope: space,
                nested: { in: task, receive: "write" },
                versioned: true,
                fields: { title: field.string() },
                permissions: ["write"],
                methods: {
                    update: method.update("write"),
                },
            }),
        ).toThrow(new TypeError("versions of draft are immutable and cannot be updated"));
    },
);
