import { anyone, principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { account } from "@destack/account/object";
import { type DatabaseConnection } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { Call, type CallableName, type ObjectType } from "@destack/object";
import { ObjectClient } from "@destack/object/client";
import { ObjectServer } from "@destack/object/server";
import { PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { Authentication } from "@destack/service/authentication";
import { Health } from "@destack/service/health";
import { Server } from "@destack/service/server";
import { schema } from "@destack/schema";
import { RequestId } from "@destack/service/request";
import { subjectContext, testCallKey } from "@destack/service/test";
import { branch, branchCall, branchRow, branchType, space } from "@destack/space/object";
import { Scope, Subject } from "@destack/sync";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { v7 } from "uuid";
import { note, notebook } from "../src/object/index.ts";
import { notesService } from "../src/service/index.ts";
import { notesDatabase, notesTables } from "../src/stack/index.ts";
import { postcard, sent } from "./fixture/postcard.ts";

/** The people the scenario acts as. */
const people = {
    alice: principal.user.reference(Scope.universe.id, "alice"),
    bob: principal.user.reference(Scope.universe.id, "bob"),
    carol: principal.user.reference(Scope.universe.id, "carol"),
};

/** A person the scenario acts as. */
type Person = keyof typeof people;

/** A person a request's bearer token gives. */
const PERSON = schema.enum(["alice", "bob", "carol"]);

test.for(TEST_DIALECTS)(
    "merge a branch of notes in one transaction with the merger's permission, each note as its author, keeping a refused merge open, on %s",
    async (dialect) => {
        // serve a space's notes and their branches
        const storage = await TestDatabase.create(dialect, notesDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        const spaceId = await openSpace(storage.database);
        const server = new ObjectServer({
            objects: { notebook, note, branch, branchCall, branchRow },
            branch: branchType,
            database: storage.database,
            callKey: testCallKey,
            origin: {
                package: note.package,
                service: "notes",
            },
        });
        const call = async <Object extends ObjectType, Name extends CallableName<Object>>(
            person: Person,
            object: Object,
            name: Name,
            input: object,
        ) =>
            server.call(
                object,
                name,
                { spaceId, requestId: RequestId.create(), ...input },
                contextOf(spaceId, person),
            );

        // let bob edit alice's notebook, and keep carol out of it
        const book = await call("alice", notebook, "create", { name: "Travel" });
        await call("alice", notebook, "grant", {
            id: book.id,
            relation: "editor",
            subject: people.bob,
        });

        // draft two notes on a branch, and share it with bob and carol
        const packing = await call("alice", branch, "create", { title: "Packing" });
        await call("alice", branch, "push", {
            id: packing.id,
            calls: [
                Call.record(note, "create", { parentId: book.id, title: "Socks" }),
                Call.record(note, "create", { parentId: book.id, title: "Charger" }),
            ],
        });
        for (const subject of [people.bob, people.carol]) {
            await call("alice", branch, "grant", {
                id: packing.id,
                relation: "collaborator",
                subject,
            });
        }

        // refuse a call no served type has, and carol's merge as a whole
        const invalid = await refusal(
            call("alice", branch, "push", {
                id: packing.id,
                calls: [Call.record(notebook, "create", { name: "Elsewhere" })].map((entry) => ({
                    ...entry,
                    method: "note.explode",
                })),
            }),
        );
        const denied = await refusal(call("carol", branch, "merge", { id: packing.id }));
        const open = await call("alice", branch, "get", { id: packing.id });

        // merge as bob, answering the merged branch and keeping alice as the notes' author, and refuse pushing to it
        const answered = await call("bob", branch, "merge", { id: packing.id });
        const notes = await server.call(
            note,
            "list",
            { spaceId, where: {} },
            contextOf(spaceId, "bob"),
        );
        const merged = await call("alice", branch, "get", { id: packing.id });
        const closed = await refusal(
            call("alice", branch, "push", {
                id: packing.id,
                calls: [Call.record(note, "create", { parentId: book.id, title: "Hat" })],
            }),
        );
        expect({
            invalid,
            denied,
            open: open.state,
            notes: notes.items
                .toSorted((left, right) => left.title.localeCompare(right.title))
                .map((item) => [item.title, Subject.read(item.owner).id]),
            merged: merged.state,
            answered: [answered.id, answered.state],
            closed,
        }).toEqual({
            invalid: ["BAD_REQUEST", "no mutating method note.explode"],
            denied: ["NOT_FOUND", `no notebook ${book.id}`],
            open: "open",
            notes: [
                ["Charger", people.alice.id],
                ["Socks", people.alice.id],
            ],
            merged: "merged",
            answered: [packing.id, "merged"],
            closed: ["CONFLICT", "branch is merged"],
        });
    },
);

/** Open a new space below a new account, readable by anyone through a member role, returning its identifier. */
async function openSpace(database: DatabaseConnection) {
    // record the account and the space as copies of their home's access keep them
    const accountId = schema.identifier("account").parse(`account-${v7()}`);
    const spaceId = schema.identifier("space").parse(`space-${v7()}`);
    const reference = space.reference(accountId, spaceId);
    const accessCopies = new AccessFixture(database);
    await accessCopies.copyScope(account.reference(Scope.universe.id, accountId));
    await accessCopies.copyScope(reference);

    // let anyone read the space through a member role
    await accessCopies.copyRole(
        reference,
        { name: "member", description: "Reads the space", permissions: [space.permission("read")] },
        anyone.reference("*", "*"),
    );

    return spaceId;
}

/** Build a person's request context in a space. */
function contextOf(spaceId: string, person: Person) {
    const controller = new AbortController();
    onTestFinished(() => controller.abort());

    return subjectContext(people[person], spaceId, { signal: controller.signal });
}

test("edit a branch on two devices live, read it on the server, and merge it with each note as its author", async () => {
    // serve a space's notes and their branches to two people's devices
    const { server, spaceId } = await serveSpace();
    const alice = await openDevice("alice", server, spaceId);
    const bob = await openDevice("bob", server, spaceId);

    // let bob edit alice's notebook
    const book = alice.client.mutate(notebook).create({ name: "Travel" });
    const { id: parentId } = await book.predicted;
    await book.confirmed;
    await alice.client
        .mutate(notebook)
        .grant({ id: parentId, relation: "editor", subject: people.bob }).confirmed;

    // draft a note on a new branch, and let bob work on it too
    const packing = alice.client.mutate(branch).create({ title: "Packing" });
    const { id } = await packing.predicted;
    await alice.client.checkout(id);
    await alice.client.mutate(note).create({ parentId, title: "Socks" }).confirmed;
    await alice.client.mutate(branch).grant({ id, relation: "collaborator", subject: people.bob })
        .confirmed;
    const drafted = await titles(alice);
    await alice.client.checkout(undefined);
    const main = await titles(alice);

    // read the branch on the server as bob, with alice's note as hers
    const read = (await bob.client.read(note).list({ branch: id })).items.map((item) => [
        item.title,
        Subject.read(item.owner).id,
    ]);
    const [opened] = (await bob.client.read(branch).list({})).items;

    // show alice's edit on bob's checkout once it arrives, and add bob's own
    await bob.client.checkout(id);
    await arrival(bob, branchCall);
    const diff = bob.client.diff(id);
    onTestFinished(() => diff.close());
    const drafts = (await diff.read()).map((entry) => [
        entry.object.name,
        entry.change,
        entry.after?.["title"],
        entry.fields,
    ]);
    await bob.client.mutate(note).create({ parentId, title: "Charger" }).confirmed;
    const preview = await titles(bob);

    // merge from the checked-out branch as bob, whose device shows the main line after
    await bob.client.mutate(branch).merge({ id }).confirmed;
    await expect.poll(() => bob.client.checkedOut()).toBeUndefined();
    const [merged] = (await bob.client.read(branch).list({})).items;
    expect({
        drafted,
        main,
        read,
        drafts,
        base: typeof opened?.base.sequence,
        preview,
        server: (await bob.client.read(note).list({})).items
            .toSorted((left, right) => left.title.localeCompare(right.title))
            .map((item) => [item.title, Subject.read(item.owner).id]),
        state: merged?.state,
        errors: [...alice.errors, ...bob.errors],
    }).toEqual({
        drafted: ["Socks"],
        main: [],
        read: [["Socks", people.alice.id]],
        drafts: [["note", "created", "Socks", ["title", "pinned"]]],
        base: "number",
        preview: ["Charger", "Socks"],
        server: [
            ["Charger", people.bob.id],
            ["Socks", people.alice.id],
        ],
        state: "merged",
        errors: [],
    });
});

/** Serve a new space's notes and their branches over HTTP, until the test ends. */
async function serveSpace() {
    // serve the notes over HTTP to bearer-named people
    const storage = await TestDatabase.create("sqlite", notesDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const spaceId = await openSpace(storage.database);
    const served = ObjectServer.serve(notesService, {
        callKey: testCallKey,
        branch: branchType,
        database: storage.database,
    });
    const server = Server.start({
        ...served,
        audience: AUDIENCE,
        scope: spaceId,
        resources: new ResourceContext(),
        health: new Health(notesService.name),
        drainTimeout: 1000,
        authorizeHost: async () => {},
        authenticate: async (request) => {
            const id = PERSON.parse(request.headers.get("authorization")?.slice("Bearer ".length));
            const now = Date.now();

            return new Authentication({
                subject: people[id],
                subjects: [people[id]],
                credential: { kind: "user", id },
                audience: AUDIENCE,
                scope: spaceId,
                verifiedAt: now,
                expiresAt: now + 60_000,
            });
        },
    });
    onTestFinished(() => server.close());

    return { server, spaceId };
}

/** The package serving the notes in the scenario. */
const AUDIENCE = PackageId.parse("package-01a0d5eb-fb8a-74f4-ba37-8a4d6970e238");

/** Open a person's device on its own database, following and pushing until the test ends. */
async function openDevice(person: Person, server: Server, spaceId: string) {
    // keep the space's notes and branches in a local database
    const objects = { notebook, note, branch, branchCall, branchRow };
    const storage = await TestDatabase.create("sqlite", ObjectClient.tables(objects));
    onTestFinished(() => storage.close());
    const endpoint = {
        url: "https://notes.test",
        headers: { authorization: `Bearer ${person}` },
        fetch: (request: Request) => server.fetch(request),
    };
    const client = await ObjectClient.open({
        database: storage.database,
        objects,
        package: notesService.package,
        branch: branchType,
        scope: spaceId,
        caller: people[person],
        endpoint,
        reconnect: () => endpoint,
    });
    for (const object of [note, branch]) {
        client.queryOf(object).findMany().subscribe();
    }

    // follow and push until the test ends, collecting failures
    const errors: unknown[] = [];
    const controller = new AbortController();
    const running = client.run(controller.signal, (error) => errors.push(error));
    onTestFinished(async () => {
        controller.abort();
        await running;
        await client.close();
    });

    return { client, errors };
}

/** Read the note titles a device shows, outside the trash. */
async function titles(opened: { readonly client: ObjectClient }): Promise<string[]> {
    const rows = await opened.client.database.select({ title: note.table.title }).from(note.table);

    return rows.map((row) => row.title).toSorted((left, right) => left.localeCompare(right));
}

/** Wait until a device's copy has a row of an object type. */
async function arrival(
    opened: { readonly client: ObjectClient },
    object: ObjectType,
): Promise<void> {
    // follow the type's rows until one arrives
    const live = opened.client.queryOf(object).findMany().subscribe();
    const controller = new AbortController();
    try {
        for await (const rows of live.watch(controller.signal)) {
            if (rows.length > 0) {
                return;
            }
        }
    } finally {
        controller.abort();
        await live.close();
    }
}

test("rebuild a branch's rows once the main line removes what its calls need", async () => {
    // draft a note on a branch under a notebook alice's device follows
    const { server, spaceId } = await serveSpace();
    const alice = await openDevice("alice", server, spaceId);
    alice.client.query.notebook.findMany().subscribe();
    const book = alice.client.mutate(notebook).create({ name: "Travel" });
    const { id: parentId } = await book.predicted;
    await book.confirmed;
    await arrival(alice, notebook);
    const { id } = await alice.client.mutate(branch).create({ title: "Packing" }).predicted;
    await alice.client.checkout(id);
    await alice.client.mutate(note).create({ parentId, title: "Socks" }).confirmed;
    await alice.client.checkout(undefined);
    const read = async () =>
        (await alice.client.read(note).list({ branch: id })).items.map((item) => item.title);
    const drafted = await read();

    // delete the notebook on the main line, and find the branch's note gone once it rebuilds
    await alice.client.mutate(notebook).delete({ id: parentId }).confirmed;
    await expect.poll(read).toEqual([]);
    expect({ drafted, errors: alice.errors }).toEqual({ drafted: ["Socks"], errors: [] });
});

test.for(TEST_DIALECTS)(
    "send a branch's external work once at its merge, and never in its rows, on %s",
    async (dialect) => {
        // serve notes and postcards, whose sending is external work
        const storage = await TestDatabase.create(dialect, [...notesTables, ...postcard.tables], {
            isMigrated: true,
        });
        onTestFinished(() => storage.close());
        const spaceId = await openSpace(storage.database);
        const server = new ObjectServer({
            objects: { notebook, note, postcard, branch, branchCall, branchRow },
            branch: branchType,
            database: storage.database,
            callKey: testCallKey,
            origin: {
                package: note.package,
                service: "notes",
            },
        });
        const call = async <Object extends ObjectType, Name extends CallableName<Object>>(
            object: Object,
            name: Name,
            input: object,
        ) =>
            server.call(
                object,
                name,
                { spaceId, requestId: RequestId.create(), ...input },
                contextOf(spaceId, "alice"),
            );
        sent.length = 0;

        // push a postcard to a branch, sending nothing and changing no rows
        const trip = await call(branch, "create", { title: "Trip" });
        await call(branch, "push", {
            id: trip.id,
            calls: [Call.record(postcard, "create", { to: "grandma" })],
        });
        const { items: rows } = await call(branchRow, "list", {
            where: { parentId: trip.id },
        });
        const pushed = [...sent];

        // merge, sending the postcard once as alice
        await call(branch, "merge", { id: trip.id });
        const { items: postcards } = await call(postcard, "list", {});
        expect({
            rows,
            pushed,
            sent,
            postcards: postcards.map((item) => [item.to, item.senderId]),
        }).toEqual({
            rows: [],
            pushed: [],
            sent: ["grandma"],
            postcards: [["grandma", people.alice.id]],
        });
    },
);
