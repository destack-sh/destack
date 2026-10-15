import { testEvents, serversOf } from "@destack/object/test";
import { principal, relation } from "@destack/access";
import { space } from "@destack/account/object";
import { defineDatabase, eq, TABLE } from "@destack/db";
import { Call, defineObject, field } from "@destack/object";
import { ObjectStore } from "@destack/object/server";
import { Identifier, present, schema, Clock } from "@destack/schema";
import { RequestId } from "@destack/runtime/request";
import { subjectContext, testCallKey } from "@destack/runtime/test";
import { expect, test } from "@destack/test";
import { ids, spaceDatabase, StackFixture } from "../test/index.ts";
import { branch, branchCall, branchChange, branchType } from "./branch.ts";

/** Notes a branch drafts before the main line has them. */
const note = defineObject({
    name: "note",
    plural: "notes",
    scope: space,
    fields: {
        author: field.reference(principal.user).caller(),
        title: field.string(schema.string().min(1)),
    },
    permissions: { read: relation("author"), write: relation("author") },
}).methods((method) => ({
    create: method.create("write", { fields: ["title"] }),
}));

/** A created object's identifier. */
const Created = schema.looseObject({ id: schema.string() });

/** The space's database with its notes beside the stack's tables and branches. */
const database = defineDatabase({
    name: "space",
    tables: [
        ...spaceDatabase.tables.filter((table) => !spaceDatabase.replicates(table)),
        ...note.tables,
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
        servers: serversOf([note, branch, branchCall, branchChange]),
        database: fixture.database,
        callKey: testCallKey,
        events: await testEvents(fixture.database),
        origin: { package: note.package },
        branch: branchType,
    });
    const context = subjectContext(principal.user.reference("universe", ids.owner), ids.space);
    const call = async (calls: readonly ReturnType<typeof branch.call>[]) =>
        store.mutate(context.access(), { id: RequestId.create(Date.now()), calls: [...calls] });
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

    return { call, read, open };
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
    const created = Call.record(note, "create", { spaceId: ids.space, title: "draft" });
    await call([branch.call("push", { spaceId: ids.space, id, calls: [created] })]);
    const drafted = await read(id);

    // merge the branch, writing the note to the main line
    await call([branch.call("merge", { spaceId: ids.space, id })]);
    const merged = await read(id);

    // keep the note as the branch's change, then write the same row to the main line and drop the change
    const table = note.table[TABLE].sqlName;
    const after = present(drafted.changes[0], "the drafted change").after;
    expect({
        drafted: { ...drafted, changes: drafted.changes.map((change) => change.table) },
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
    const created = Call.record(note, "create", { spaceId: ids.space, title: "draft" });
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
