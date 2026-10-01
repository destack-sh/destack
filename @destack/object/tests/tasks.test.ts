import { AuditOutbox } from "@destack/audit/outbox";
import { ObjectServer } from "../src/server/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { AuditRecorder } from "@destack/audit";
import { Condition } from "@destack/db/query";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { identifier } from "@destack/schema";
import { Caller } from "@destack/service/authentication";
import { createClient } from "@destack/service/client";
import { Health } from "@destack/service/health";
import { RequestId } from "@destack/service/request";
import { Server } from "@destack/service/server";
import { v7 } from "uuid";
import { tasksDatabase, tasksService, tasksJournal } from "./fixture/tasks.ts";
import { principal } from "@destack/access";
import { openSpace } from "./fixture/space.ts";
import { testJournalKey } from "@destack/service/test";

/** The space holding the projects. */
const spaceId = identifier("space").parse(`space-${v7()}`);

/** The package serving the projects. */
const audience = PackageId.parse("package-01a0d5eb-fb66-76d2-93b6-5568ae045a69");

test.each(TEST_DIALECTS)(
    "plan a project with its members, assignees, guarded budgets and comments on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, tasksDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        const database = storage.database;
        await openSpace(database, spaceId);

        // serve the space's tasks to users named by their bearer credential
        const server = Server.start({
            ...ObjectServer.serve(tasksService, {
                journal: tasksJournal,
                journalKey: testJournalKey,
                database,
                audit: AuditRecorder.service(new AuditOutbox(database), {
                    package: tasksService.package,
                    service: "test",
                }),
            }),
            audience,
            scope: spaceId,
            resources: new ResourceContext(),
            health: new Health("tasks"),
            drainTimeout: 1000,
            authorizeHost: async () => {},
            authenticate: async (request) => {
                const id = request.headers.get("authorization")!.slice("Bearer ".length);
                const subject = principal.user.reference("universe", id);
                const now = Date.now();

                return new Caller({
                    subject,
                    subjects: [subject],
                    credential: { kind: "user", id },
                    audience,
                    scope: spaceId,
                    verifiedAt: now,
                    expiresAt: now + 60_000,
                });
            },
        });
        onTestFinished(() => server.close());
        const as = (user: string) =>
            createClient(tasksService, {
                url: "https://tasks.test",
                headers: { authorization: `Bearer ${user}` },
                fetch: (request: Request) => server.fetch(request),
            });
        const [alice, bob, carol, dave] = ["alice", "bob", "carol", "dave"].map(as);
        const person = (id: string) => principal.user.reference("universe", id);

        // let the owner create a project and a budgeted task assigned to another user
        const launch = await alice!.project.create({
            spaceId,
            requestId: RequestId.create(),
            name: "Launch",
        });
        expect(launch.owner).toBe("alice");
        const announce = await alice!.task.create({
            spaceId,
            requestId: RequestId.create(),
            parentId: launch.id,
            title: "Write the announcement",
            assignee: "bob",
            priority: "high",
            budget: 500,
            due: 1_790_000_000_000,
        });
        expect([announce.author, announce.status, announce.budget, announce.due]).toEqual([
            "alice",
            "open",
            500,
            1_790_000_000_000,
        ]);

        // let the assignee read and work the task without planning it or seeing its budget
        const assigned = await bob!.task.get({ spaceId, id: announce.id });
        expect([assigned.title, "budget" in assigned]).toEqual(["Write the announcement", false]);
        await bob!.task.start({ spaceId, id: announce.id, requestId: RequestId.create() });
        await expect(
            bob!.task.update({
                spaceId,
                id: announce.id,
                requestId: RequestId.create(),
                revision: 2,
                title: "Rename",
            }),
        ).rejects.toMatchObject({ code: "FORBIDDEN", message: "permission denied: plan" });
        await expect(bob!.project.get({ spaceId, id: launch.id })).rejects.toMatchObject({
            code: "NOT_FOUND",
            message: `no project ${launch.id}`,
        });

        // let a user ask to join, and plan once the owner accepts
        const asked = await carol!.project.propose({
            spaceId,
            id: launch.id,
            requestId: RequestId.create(),
            relationship: { relation: "member", subject: person("carol") },
            purpose: "help with the launch",
        });
        expect(
            (await alice!.project.proposals({ spaceId, id: launch.id })).items.map(
                (proposal) => proposal.purpose,
            ),
        ).toEqual(["help with the launch"]);
        await alice!.project.accept({
            spaceId,
            id: launch.id,
            requestId: RequestId.create(),
            proposalId: asked.id,
        });
        const listed = await carol!.task.list({
            spaceId,
            where: Condition.eq("parentId", launch.id),
        });
        expect(listed.items.map((item) => [item.title, item.status, "budget" in item])).toEqual([
            ["Write the announcement", "active", false],
        ]);
        await carol!.task.create({
            spaceId,
            requestId: RequestId.create(),
            parentId: launch.id,
            title: "Book the venue",
        });
        await expect(
            carol!.task.update({
                spaceId,
                id: announce.id,
                requestId: RequestId.create(),
                revision: 2,
                budget: 900,
            }),
        ).rejects.toMatchObject({ code: "FORBIDDEN", message: "field budget is not writable" });

        // let everyone reading a task comment on it
        await bob!.comment.create({
            spaceId,
            requestId: RequestId.create(),
            parentId: announce.id,
            text: "Draft is ready",
        });
        expect(
            (
                await carol!.comment.list({ spaceId, where: Condition.eq("parentId", announce.id) })
            ).items.map((item) => [item.author, item.text]),
        ).toEqual([["bob", "Draft is ready"]]);
        await bob!.task.complete({ spaceId, id: announce.id, requestId: RequestId.create() });

        // follow up a task of another space by its qualified reference
        const origin = {
            scope: identifier("space").parse(`space-${v7()}`),
            id: identifier("task").parse(`task-${v7()}`),
        };
        const followUp = await alice!.task.create({
            spaceId,
            requestId: RequestId.create(),
            parentId: launch.id,
            title: "Reuse last year's checklist",
            origin,
        });
        expect(followUp.origin).toEqual(origin);

        // refuse requests naming another space than the caller authenticated for
        const elsewhere = identifier("space").parse(`space-${v7()}`);
        await expect(
            alice!.project.get({ spaceId: elsewhere, id: launch.id }),
        ).rejects.toMatchObject({
            code: "NOT_FOUND",
            message: `scope ${elsewhere} is outside the pinned scope`,
        });

        // hide the project from users with no relation to it
        expect((await dave!.project.list({ spaceId })).items).toEqual([]);
        await expect(dave!.task.get({ spaceId, id: announce.id })).rejects.toMatchObject({
            code: "NOT_FOUND",
            message: `no task ${announce.id}`,
        });
    },
);
