import { principal } from "@destack/access";
import objectPackage from "@destack/object/package";
import { eq, TABLE } from "@destack/db";
import { serversOf, subjectContext } from "@destack/object/test";
import { Identifier, present, schema } from "@destack/schema";
import { type Call, RequestId, Scope } from "@destack/sync";
import { expect, refusal, test } from "@destack/test";
import { approval, change, changeRequest } from "@destack/workflow";
import { ids, TestSpace } from "../test/index.ts";
import { note } from "../test/schema.ts";
import { branchApprovals } from "../setting/index.ts";
import { branch, branchChange } from "./branch.ts";

/** A created object's identifier. */
const Created = schema.looseObject({ id: schema.string() });

/** Serve the space's notes and branches, and let its owner and a reviewer call them. */
async function serveBranches() {
    const fixture = await TestSpace.open({ packages: [{ objects: { note }, tables: [] }] });
    const served = await fixture.serve({ servers: serversOf([note]) });
    const { store } = served;
    const person = await fixture.universe.signIn("reviewer@example.com");
    const owner = subjectContext(fixture.owner.subject, ids.space, {
        environment: store.environment,
    });
    const reviewer = subjectContext(person.subject, ids.space, { environment: store.environment });
    const call = async <const Calls extends readonly Call[]>(calls: Calls) =>
        store.mutate(owner.access(), { id: RequestId.create(store.environment), calls });
    const review = async <const Calls extends readonly Call[]>(calls: Calls) =>
        store.mutate(reviewer.access(), { id: RequestId.create(store.environment), calls });
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

    // grant the space acting for the owner at the owner's home, as the owner consents on first use
    const consented = [note, branch, changeRequest, ...Object.values(change.objects)].flatMap(
        (type) =>
            Object.keys(type.policy.definition.permissions).map((name) => ({
                ...type.policy.permission(name),
                scope: ids.space,
            })),
    );
    await fixture.owner.client(objectPackage).grant.create({
        scope: fixture.owner.id,
        requestId: RequestId.create(store.environment),
        delegate: principal.space.reference(Scope.universe.id, ids.space),
        permissions: consented,
    });
    await fixture.settle(served);

    return { fixture, served, reviewer: person.subject, call, review, read, open };
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

    // record the note as the branch's change, then write the same row to the main line and drop the change
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

    // record no change and no note
    const table = note.table[TABLE].sqlName;
    expect(await read(id)).toEqual({
        branch: { state: "discarded", tables: [table] },
        changes: [],
        notes: [],
    });
});

test("merge a branch into a space requiring an approval only through its change request once approved", async () => {
    // require one approval of the space's branches through its stack, and push a note onto a branch shared with a reviewer
    const { fixture, served, reviewer, call, review, read, open } = await serveBranches();
    await fixture.apply(served, [], {
        settings: {
            [`${branchApprovals.reference.packageId}/${branchApprovals.reference.name}`]: {
                ...branchApprovals.reference,
                mode: "set",
                value: 1,
            },
        },
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
            subject: reviewer,
        }),
    ]);

    // refuse merging it directly
    const merge = branch.call("merge", { spaceId: ids.space, id });
    const direct = await refusal(call([merge]));

    // open a change request with the merge, which asks the reviewer once the space settles
    const [requested] = await call([
        changeRequest.call("create", {
            spaceId: ids.space,
            parent: { packageId: branch.package.id, type: branch.name, id },
            call: merge,
            approvers: [reviewer],
        }),
    ]);
    await fixture.settle(served);
    const [asked] = await fixture.database.select().from(approval.table);

    // approve as the reviewer, and expect the change run to merge the branch as the requester
    await review([approval.call("approve", approval.target(present(asked, "the approval")))]);
    await fixture.settle(served);
    const [row] = await fixture.database
        .select({ state: changeRequest.table.state })
        .from(changeRequest.table)
        .where(eq(changeRequest.table.id, requested.id));
    expect([direct, row?.state, (await read(id)).branch?.state]).toEqual([
        [
            "CONFLICT",
            "CHANGE_REQUEST_REQUIRED",
            `branch ${id} can only merge through an approved change request`,
        ],
        "applied",
        "merged",
    ]);
});
