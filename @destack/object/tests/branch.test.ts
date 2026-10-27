import { principal } from "@destack/access";
import { eq, TABLE } from "@destack/db";
import { present, schema } from "@destack/schema";
import { RequestId, Scope } from "@destack/sync";
import { expect, refusal, test } from "@destack/test";
import objectPackage from "../src/package.ts";
import {
    approval,
    branch,
    changeRequest,
    branchChange,
    defineObject,
    definePackage,
    field,
} from "../src/index.ts";
import { TestObjectStore } from "../src/test/index.ts";
import { users } from "./fixture/user.ts";

/** A note of a space, drafted on branches. */
const note = defineObject(
    {
        name: "note",
        plural: "notes",
        scope: [principal.space],
        fields: { title: field.string(schema.string().min(1)) },
        permissions: ["manage"],
    },
    (method) => ({ create: method.create("manage") }),
);

/** Object's package with the space's notes, as this test module belongs to it. */
const notesPackage = definePackage({ objects: { ...objectPackage.objects, note } });

/** Serve alice's space with notes and branches, and read a branch with its changes. */
async function serveBranches() {
    const objects = await TestObjectStore.open({
        packages: [notesPackage],
        owners: [principal.user.reference(Scope.universe.id, users.alice)],
    });
    const client = objects.as(users.alice).client(notesPackage);
    const spaceId = objects.scope;
    const request = () => ({ scope: spaceId, requestId: RequestId.create(objects.environment) });
    const draft = note.call("create", { ...note.in(spaceId), title: "draft" });
    const read = async (id: string) => {
        const [opened] = await objects.database
            .select({ state: branch.table.state, tables: branch.table.tables })
            .from(branch.table)
            .where(eq(branch.table.id, schema.identifier("branch").parse(id)));
        const changes = await objects.database
            .select({ table: branchChange.table.table, after: branchChange.table.after })
            .from(branchChange.table)
            .where(eq(branchChange.table.parentId, schema.identifier("branch").parse(id)));
        const rows = await objects.database.select().from(note.table);
        const notes = rows.map((row) => note.table[TABLE].encode(row));

        return { branch: opened, changes, notes };
    };

    return { objects, client, request, draft, read };
}

/** Drop a row's identifier and times, which a replay writes anew. */
function replayed(row: Readonly<Record<string, unknown>>): Readonly<Record<string, unknown>> {
    const { id: _id, createdAt: _createdAt, updatedAt: _updatedAt, ...fields } = row;

    return fields;
}

test("draft a note on a branch over the main line, then merge it into the main line", async () => {
    // open a branch and push a note's creation onto it
    const { objects, client, request, draft, read } = await serveBranches();
    await using _objects = objects;
    const { id } = await client.branch.create({ ...request(), title: "drafts" });
    await client.branch.push({ ...request(), id, calls: [draft] });
    const drafted = await read(id);

    // merge the branch, writing the note to the main line
    await client.branch.merge({ ...request(), id });
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
    const { objects, client, request, draft, read } = await serveBranches();
    await using _objects = objects;
    const { id } = await client.branch.create({ ...request(), title: "abandoned" });
    await client.branch.push({ ...request(), id, calls: [draft] });
    await client.branch.discard({ ...request(), id });

    // record no change and no note
    const table = note.table[TABLE].sqlName;
    expect(await read(id)).toEqual({
        branch: { state: "discarded", tables: [table] },
        changes: [],
        notes: [],
    });
});

test("merge a branch whose role lets bob merge once another approves, through a change request alice approves", async () => {
    // let bob read, review and merge alice's branch once one other approves, with a note drafted on it
    const { objects, client, request, draft, read } = await serveBranches();
    await using _objects = objects;
    const { id } = await client.branch.create({ ...request(), title: "reviewed" });
    await client.branch.push({ ...request(), id, calls: [draft] });
    const contributor = await client.role.create({
        ...request(),
        name: "contributor",
        description: "Reviews branches and merges them once another approves",
        permissions: [
            branch.permission("read"),
            branch.permission("approve"),
            { ...branch.permission("merge"), approvals: 1 },
        ],
    });
    const alice = principal.user.reference(Scope.universe.id, users.alice);
    const bob = principal.user.reference(Scope.universe.id, users.bob);
    await client.relationship.create({
        ...request(),
        object: branch.reference(objects.scope, id),
        role: contributor.id,
        subject: bob,
    });

    // merge as bob, refused for its approval, and ask alice in a change request
    const merging = objects.as(users.bob).client(notesPackage);
    const refused = await refusal(merging.branch.merge({ ...request(), id }));
    await merging.changeRequest.create({
        ...request(),
        parent: { packageId: branch.package.id, type: branch.name, id },
        call: branch.call("merge", { scope: objects.scope, id }),
        approvers: [alice],
    });

    // approve as alice, which merges the branch as bob
    await objects.settle();
    const [asked] = await objects.database.select().from(approval.table);
    await client.approval.approve({
        ...approval.target(present(asked, "alice's approval")),
        requestId: RequestId.create(objects.environment),
    });
    await objects.settle();
    const [requested] = await objects.database.select().from(changeRequest.table);
    expect([refused, (await read(id)).branch?.state, requested?.state]).toEqual([
        ["FORBIDDEN", "APPROVAL_REQUIRED", `merge on branch ${id} needs 1 approval`],
        "merged",
        "applied",
    ]);
});
