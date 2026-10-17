import { testEvents, serversOf } from "@destack/object/test";
import { principal, relation } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { space } from "@destack/account/object";
import { defineDatabase, eq, TABLE } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { ObjectStore } from "@destack/object/server";
import { Identifier, present, schema } from "@destack/schema";
import { RequestId } from "@destack/runtime/request";
import { subjectContext, testCallKey } from "@destack/runtime/test";
import type { Call } from "@destack/sync";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { ids, spaceDatabase, StackFixture } from "../test/index.ts";
import { approval, change, changeRequest } from "@destack/workflow";
import { ControlLoop } from "@destack/runtime/control";
import { setting } from "@destack/setting/object";
import { branchApprovals } from "../setting/index.ts";
import workflowPackage from "@destack/workflow/package";
import workflowServer from "@destack/workflow/server";
import { branch, branchEntry, branchChange, branchType } from "./branch.ts";
import { Clock } from "@destack/environment";

/** Notes a branch drafts before the main line has them. */
const note = defineObject(
    {
        name: "note",
        plural: "notes",
        scope: space,
        fields: {
            author: field.reference(principal.user).caller(),
            title: field.string(schema.string().min(1)),
        },
        permissions: { read: relation("author"), write: relation("author") },
    },
    (method) => ({
        create: method.create("write", { fields: ["title"] }),
    }),
);

/** The person the owner asks to review a branch. */
const REVIEWER = principal.user.reference(
    "universe",
    schema.identifier("user").parse("user-01996ab0-0000-7000-8000-0000000000b9"),
);

/** A created object's identifier. */
const Created = schema.looseObject({ id: schema.string() });

/** The space's database with its notes beside the stack's tables and branches. */
const database = defineDatabase({
    name: "space",
    tables: [
        ...spaceDatabase.tables.filter((table) => !spaceDatabase.replicates(table)),
        ...note.tables,
        ...Object.values(workflowPackage.objects).flatMap((type) => type.tables),
    ],
    replicated: spaceDatabase.tables.filter((table) => spaceDatabase.replicates(table)),
});

/** Serve the space's notes and branches, and let its owner call them. */
async function serveBranches() {
    const fixture = await StackFixture.open({ database });
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: principal.space.reference("universe", ids.space),
        servers: [...serversOf([note, branch, branchEntry, branchChange]), workflowServer],
        database: fixture.database,
        callKey: testCallKey,
        events: await testEvents(fixture.database),
        origin: { package: note.package },
        branch: branchType,
    });
    // let the owner's changes run as the space's code, as the owner consents on first use
    const ownerSubject = principal.user.reference("universe", ids.owner);
    const consented = [note, branch, changeRequest, ...Object.values(change.objects)].flatMap(
        (type) =>
            Object.keys(type.policy.definition.permissions).map((name) => ({
                ...type.policy.permission(name),
                scope: ids.space,
            })),
    );
    await new AccessFixture(fixture.database).replicateGrant(
        ownerSubject,
        principal.space.reference("universe", ids.space),
        consented,
    );
    const owner = subjectContext(ownerSubject, ids.space);
    const reviewer = subjectContext(REVIEWER, ids.space);
    const call = async <const Calls extends readonly Call[]>(calls: Calls) =>
        store.mutate(owner.access(), { id: RequestId.create(Date.now()), calls });
    const review = async <const Calls extends readonly Call[]>(calls: Calls) =>
        store.mutate(reviewer.access(), { id: RequestId.create(Date.now()), calls });
    const read = async (id: Identifier<"branch">) => {
        const [opened] = await fixture.database
            .select({ state: branch.table.state, tables: branch.table.tables })
            .from(branch.table)
            .where(eq(branch.table.id, id));
        const changes = await fixture.database
            .select({ table: branchChange.table.table, after: branchChange.table.after })
            .from(branchChange.table)
            .where(eq(branchChange.table.parentId, id));
        const rows = await fixture.database.select().from(note.table);
        const notes = rows.map((row) => note.table[TABLE].encode(row));

        return { branch: opened, changes, notes };
    };
    const open = async (title: string) => {
        const [opened] = await call([branch.call("create", { spaceId: ids.space, title })]);

        return schema.identifier("branch").parse(Created.parse(opened).id);
    };

    return { store, call, review, read, open };
}

/** Drop a row's identifier and times, which a replay writes anew. */
function replayed(row: Readonly<Record<string, unknown>>): Readonly<Record<string, unknown>> {
    const { id: _id, createdAt: _createdAt, updatedAt: _updatedAt, ...fields } = row;

    return fields;
}

test("draft a note on a branch over the main line, then merge it into the main line", async () => {
    // open a branch and push a note's creation onto it
    const { call, read, open } = await serveBranches();
    const id = await open("drafts");
    const created = note.call("create", { spaceId: ids.space, title: "draft" });
    await call([branch.call("push", { spaceId: ids.space, id, calls: [created] })]);
    const drafted = await read(id);

    // merge the branch, writing the note to the main line
    await call([branch.call("merge", { spaceId: ids.space, id })]);
    const merged = await read(id);

    // keep the note as the branch's change, then write the same row to the main line and drop the change
    const table = note.table[TABLE].sqlName;
    const after = present(drafted.changes[0], "the drafted change").after;
    expect({
        drafted: { ...drafted, changes: drafted.changes.map((entry) => entry.table) },
        merged: { ...merged, notes: merged.notes.map(replayed) },
    }).toEqual({
        drafted: { branch: { state: "open", tables: [table] }, changes: [table], notes: [] },
        merged: {
            branch: { state: "merged", tables: [table] },
            changes: [],
            notes: [replayed(present(after, "the drafted note"))],
        },
    });
});

test("discard a branch, dropping its changes and leaving the main line as it was", async () => {
    // push a note's creation onto a branch and discard it
    const { call, read, open } = await serveBranches();
    const id = await open("abandoned");
    const created = note.call("create", { spaceId: ids.space, title: "draft" });
    await call([branch.call("push", { spaceId: ids.space, id, calls: [created] })]);
    await call([branch.call("discard", { spaceId: ids.space, id })]);

    // keep no change and no note
    const table = note.table[TABLE].sqlName;
    expect(await read(id)).toEqual({
        branch: { state: "discarded", tables: [table] },
        changes: [],
        notes: [],
    });
});

test("merge a branch into a space requiring an approval only through its change request once approved", async () => {
    // require one approval of the space's branches, and push a note onto a branch shared with a reviewer
    const { store, call, review, read, open } = await serveBranches();
    const now = Date.now();
    await store.database.insert(setting.table).values({
        id: Identifier.create("setting"),
        scope: ids.space,
        packageId: branchApprovals.package.id,
        name: branchApprovals.name,
        mode: "set",
        value: 1,
        release: branchApprovals.package.version,
        createdAt: now,
        updatedAt: now,
    });
    const id = await open("reviewed");
    await call([
        branch.call("push", {
            spaceId: ids.space,
            id,
            calls: [note.call("create", { spaceId: ids.space, title: "draft" })],
        }),
        branch.call("grant", {
            spaceId: ids.space,
            id,
            relation: "collaborator",
            subject: REVIEWER,
        }),
    ]);

    // refuse merging it directly
    const merge = branch.call("merge", { spaceId: ids.space, id });
    const direct = await refusal(call([merge]));

    // run the change requests, and open one holding the merge, which asks the reviewer
    const stopping = new AbortController();
    const loop = new ControlLoop(
        store.database,
        store.controllers().filter((controller) => controller.name.startsWith("change")),
        { clock: Clock.system, report: (_controller, _key, error) => stopping.abort(error) },
    );
    const running = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });
    const [requested] = await call([
        changeRequest.call("create", {
            spaceId: ids.space,
            parent: { packageId: branch.package.id, type: branch.name, id },
            call: merge,
            approvers: [REVIEWER],
        }),
    ]);
    await expect
        .poll(async () => (await store.database.select().from(approval.table)).length)
        .toBe(1);

    // approve as the reviewer, and expect the change run to merge the branch as the requester
    const [asked] = await store.database.select().from(approval.table);
    await review([approval.call("approve", approval.target(present(asked, "the approval")))]);
    await expect.poll(async () => (await read(id)).branch?.state).toBe("merged");
    const [applied] = await store.database
        .select({ state: changeRequest.table.state })
        .from(changeRequest.table)
        .where(eq(changeRequest.table.id, requested.id));
    expect([direct, applied?.state]).toEqual([
        ["CONFLICT", `branch ${id} merges only through an approved change request`],
        "applied",
    ]);
});
