import { anyone, principal } from "@destack/access";
import { Scope } from "@destack/sync";
import { copyRole, copyScope } from "@destack/access/test";
import { account } from "@destack/account/object";
import { AuditRecorder } from "@destack/audit";
import { AuditOutbox } from "@destack/audit/outbox";
import type { DatabaseConnection } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { announcement, delivery, notification, subscription } from "@destack/notification";
import type { ObjectType } from "@destack/object";
import { ObjectServer } from "@destack/object/server";
import { identifier } from "@destack/schema";
import { Journal } from "@destack/service/database";
import { ServiceError } from "@destack/service/error";
import { RequestId } from "@destack/service/request";
import { subjectContext, testJournalKey } from "@destack/service/test";
import { comment, reaction } from "@destack/social";
import { space } from "@destack/space/object";
import { expect, onTestFinished, test } from "@destack/test";
import { v7 } from "uuid";
import { project, task } from "../src/object/index.ts";
import { tasksDatabase, tasksJournal } from "../src/stack/index.ts";

/** The people the scenario acts as. */
const people = {
    alice: principal.user.reference(Scope.universe.id, "alice"),
    bob: principal.user.reference(Scope.universe.id, "bob"),
    carol: principal.user.reference(Scope.universe.id, "carol"),
    dave: principal.user.reference(Scope.universe.id, "dave"),
    eve: principal.user.reference(Scope.universe.id, "eve"),
};

/** A person the scenario acts as. */
type Person = keyof typeof people;

test.for(TEST_DIALECTS)(
    "let the readers of a task comment on it, counting the comments on the task, on %s",
    async (dialect) => {
        // serve the tasks database in a space of its own
        const storage = await TestDatabase.create(dialect, tasksDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        const spaceId = await openSpace(storage.database);
        const server = new ObjectServer({
            objects: {
                project,
                task,
                comment,
                reaction,
                subscription,
                notification,
                announcement,
                delivery,
            },
            database: storage.database,
            context: (context) => ({
                subjects: [context.requireCaller().authentication.subject],
                now: Date.now(),
                attributes: {},
            }),
            journal: new Journal(tasksJournal, testJournalKey),
            audit: AuditRecorder.service(new AuditOutbox(storage.database), {
                package: task.package,
                service: "tasks",
            }),
        });
        const call = async (person: Person, object: ObjectType, name: string, input: object) =>
            (await server.call(
                object,
                name,
                { spaceId, requestId: RequestId.create(), ...input },
                context(spaceId, person),
            )) as Record<string, unknown> & { readonly id: string };

        // plan a project with a member and a viewer, and assign a task to someone outside it
        const launch = await call("alice", project, "create", { name: "Launch" });
        await call("alice", project, "grant", {
            id: launch.id,
            relation: "member",
            subject: people.bob,
        });
        await call("alice", project, "grant", {
            id: launch.id,
            relation: "viewer",
            subject: people.carol,
        });
        const draft = await call("bob", task, "create", {
            parentId: launch.id,
            title: "Draft the announcement",
            assignee: people.dave.id,
        });
        const host = { parent: { packageId: task.package.id, type: "task", id: draft.id } };

        // let the viewer comment and the assignee reply, counted on the task
        const first = await call("carol", comment, "create", {
            ...host,
            body: { text: "Which date?", mentions: [] },
        });
        await call("dave", comment, "create", {
            ...host,
            body: { text: "Friday", mentions: [] },
            thread: first.id,
        });
        expect((await call("bob", task, "get", { id: draft.id })).commentCount).toBe(2);

        // refuse a comment by someone who cannot read the task
        await expect(
            call("eve", comment, "create", { ...host, body: { text: "Hi", mentions: [] } }),
        ).rejects.toEqual(new ServiceError("NOT_FOUND", { message: `no task ${draft.id}` }));
    },
);

/** Open a new space below a new account, readable by anyone through a member role, returning its identifier. */
async function openSpace(database: DatabaseConnection) {
    // record the account and the space as copies of their home's access hold them
    const accountId = identifier("account").parse(`account-${v7()}`);
    const spaceId = identifier("space").parse(`space-${v7()}`);
    const reference = space.reference(accountId, spaceId);
    await copyScope(database, account.reference(Scope.universe.id, accountId));
    await copyScope(database, reference);

    // let anyone read the space through a member role
    await copyRole(
        database,
        reference,
        { name: "member", description: "Reads the space", permissions: [space.permission("read")] },
        anyone.reference("*", "*"),
    );

    return spaceId;
}

/** Build a person's request context in a space. */
function context(spaceId: string, person: Person) {
    const controller = new AbortController();
    onTestFinished(() => controller.abort());

    return subjectContext(people[person], spaceId, controller.signal);
}
