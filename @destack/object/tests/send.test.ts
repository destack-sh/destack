import { reconciliation, testCallKey } from "@destack/runtime/test";
import { callTables, testEvents, serversOf } from "../src/test/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { AccessContext, accessTables, Caller, principal, relation } from "@destack/access";

import { asc, defineDatabase, Transaction } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { Clock, Identifier, present, schema } from "@destack/schema";
import type { Mutation } from "@destack/sync";

import { outbox } from "@destack/runtime/outbox";
import { CallError } from "@destack/runtime/error";
import { RequestId } from "@destack/runtime/request";
import type { RunDelivery, RunRequest } from "@destack/runtime/trigger";
import { defineObject, defineShape, field, Follow } from "../src/index.ts";
import { defineServer, ObjectStore } from "../src/server/index.ts";
import { PackageHandle } from "@destack/runtime";
import { until } from "@destack/runtime/timer";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { userContext, users } from "./fixture/user.ts";
import { createRouterClient } from "@destack/runtime/server";

/** The space with the accounts. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000021");

/** Accounts their owners sign up, each welcomed by a mail sent after the sign-up commits. */
const member = defineObject({
    name: "member",
    plural: "members",
    scope: space,
    fields: { owner: field.reference(principal.user).caller(), name: field.string() },
    permissions: { write: relation("owner") },
})
    .methods((method) => ({
        create: method.create("write", { fields: ["name"] }),
        welcome: method.update("write", { fields: ["name"] }),
    }))
    .implement({
        methods: {
            // sign up, sending the welcome, and refuse a sign-up of the refused name after sending
            create: async (call, next) => {
                const name = call.input.name;
                await call.send({ call: member.calls().welcome({ id: "member-1", name }) });
                if (name === "Refused") {
                    throw new TypeError("the sign-up is refused");
                }

                return next();
            },
        },
    });

/** Notes their owners write and rename, which a follower replicates and changes. */
const note = defineObject({
    name: "note",
    plural: "notes",
    scope: space,
    fields: { owner: field.reference(principal.user).caller(), title: field.string() },
    permissions: { write: relation("owner") },
}).methods((method) => ({
    create: method.create("write", { fields: ["title"] }),
    rename: method.update("write", { fields: ["title"] }),
}));

/** The database keeping the notes. */
const noteDatabase = defineDatabase({
    name: "main",
    tables: [...callTables, outbox, ...note.tables],
});

/** A follower's database replicating the notes. */
const followerDatabase = defineDatabase({
    name: "main",
    tables: [outbox, ...accessTables],
    replicated: [note.table],
});

/** The database with the members, the journal and the outbox. */
const memberDatabase = defineDatabase({
    name: "main",
    tables: [...callTables, outbox, ...member.tables],
});

test.each(TEST_DIALECTS)(
    "send a call from a method once its transaction commits, nothing from one that rolls back, and let go of one the machine refuses for good, on %s",
    async (dialect) => {
        // serve the members with a machine recording their runs
        const storage = await TestDatabase.create(dialect, memberDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        const recorded: [RunRequest, RunDelivery | undefined][] = [];
        const reports: string[] = [];
        const store = new ObjectStore({
            chain: async () => undefined,
            clock: Clock.system,
            principal: spacePrincipal(spaceId),
            servers: serversOf([member]),
            database: storage.database,
            callKey: testCallKey,
            events: await testEvents(storage.database),
            origin: {
                package: member.package,
            },
            report: (error) => {
                if (!(error instanceof Error)) {
                    throw error;
                }
                reports.push(error.message);
            },
            runs: {
                // refuse the welcome of the bounced name for good
                send: async (request, delivery) => {
                    if (request.call.input["name"] === "Bounced") {
                        throw new CallError("BAD_REQUEST", { message: "invalid call" });
                    }
                    recorded.push([request, delivery]);
                },
            },
        });
        const context = userContext(users.frank, spaceId);
        const signUp = (name: string) =>
            store
                .mutate(context.access(), {
                    id: RequestId.create(Date.now()),
                    calls: [member.call("create", { spaceId, name })],
                })
                .then(([written]) => written);

        // sign up a name the machine refuses before another, and fail a sign-up after it sent
        await signUp("Bounced");
        await signUp("Ada");
        await expect(signUp("Refused")).rejects.toThrow("the sign-up is refused");

        // deliver the outbox through the server's controller, twice to show one delivery
        const sends = present(
            store.controllers().find((controller) => controller.name === "sends"),
            "the sends controller",
        );
        const signal = new AbortController().signal;
        await sends.reconcile("sends", reconciliation(signal));
        await sends.reconcile("sends", reconciliation(signal));

        // record Ada's welcome past the refused one for its sender, reporting the refusal once
        expect([
            recorded.map(([request, delivery]) => [request, typeof delivery?.requestId]),
            reports,
        ]).toEqual([
            [
                [
                    {
                        call: member.calls().welcome({ id: "member-1", name: "Ada" }),
                        exchange: { subject: principal.user.reference("universe", users.frank) },
                    },
                    "string",
                ],
            ],
            ["a sent call was refused permanently"],
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "send a follower's calls on replicated rows over its uplink to the server keeping them, run there as the follower, and let go of one it may not make, on %s",
    async (dialect) => {
        // keep the notes in their own database, with ada's note written there
        const storage = await TestDatabase.create(dialect, noteDatabase, { isMigrated: true });
        const replicas = await TestDatabase.create(dialect, followerDatabase, { isMigrated: true });
        onTestFinished(async () => {
            await Promise.all([storage.close(), replicas.close()]);
        });
        await openSpace(storage.database, spaceId);
        const home = new ObjectStore({
            chain: async () => undefined,
            clock: Clock.system,
            principal: spacePrincipal(spaceId),
            servers: serversOf([note]),
            database: storage.database,
            callKey: testCallKey,
            events: await testEvents(storage.database),
            origin: { package: note.package },
        });
        const written = await home
            .mutate(userContext(users.ada, spaceId).access(), {
                id: RequestId.create(Date.now()),
                calls: [note.call("create", { spaceId, title: "Ada's" })],
            })
            .then(([created]) => created);

        // follow them as bob, whose uplink runs his changes at home
        const reports: string[] = [];
        const follower = new ObjectStore({
            chain: async () => ({ kind: "follower" }),
            clock: Clock.system,
            principal: spacePrincipal(spaceId),
            servers: serversOf([]),
            policies: [note],
            database: replicas.database,
            origin: { package: note.package },
            report: (error) => {
                if (!(error instanceof Error)) {
                    throw error;
                }
                reports.push(error.message);
            },
            uplink: {
                stream: async function* (_subscription, signal) {
                    await until(signal);
                    yield* [];
                },
                receive: (mutation) =>
                    home.receiveFrom(mutation, userContext(users.bob, spaceId).access()),
            },
        });

        // send a note and a rename of ada's toward their home, delivering the sends twice to show one delivery
        await Transaction.run(replicas.database, async (transaction) => {
            await follower.send(note.call("create", { spaceId, title: "Bob's" }), transaction);
            await follower.send(
                note.call("rename", { spaceId, id: written.id, title: "Bob's now" }),
                transaction,
            );
        });
        const sends = present(
            follower.controllers().find((controller) => controller.name === "sends"),
            "the sends controller",
        );
        const signal = new AbortController().signal;
        await sends.reconcile("sends", reconciliation(signal));
        await sends.reconcile("sends", reconciliation(signal));

        // keep bob's note as his, leave ada's unrenamed, and report the refused rename once
        const notes = await storage.database
            .select({ owner: note.table.ownerId, title: note.table.title })
            .from(note.table)
            .orderBy(asc(note.table.title));
        expect({ notes, reports }).toEqual({
            notes: [
                { owner: users.ada, title: "Ada's" },
                { owner: users.bob, title: "Bob's" },
            ],
            reports: ["a sent call was refused permanently"],
        });
    },
);

test("refuse a pushed mutation of replicated rows, alone or beside kept rows, which only their home writes", async () => {
    // keep the notes at home and replica them in a follower serving members
    const storage = await TestDatabase.create("sqlite", noteDatabase, { isMigrated: true });
    const replicas = await TestDatabase.create(
        "sqlite",
        defineDatabase({
            name: "main",
            tables: [outbox, ...accessTables, ...callTables, ...member.tables],
            replicated: [note.table],
        }),
        { isMigrated: true },
    );
    onTestFinished(async () => {
        await Promise.all([storage.close(), replicas.close()]);
    });
    await openSpace(storage.database, spaceId);
    await openSpace(replicas.database, spaceId);
    const follower = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([member]),
        policies: [note],
        database: replicas.database,
        callKey: testCallKey,
        events: await testEvents(replicas.database),
        origin: { package: note.package },
    });

    // push a note's creation as bob, then one beside a member's
    const create = note.calls().create({ title: "Bob's" });
    const join = member.calls().create({ name: "Bob" });
    const pushed = await createRouterClient(follower.router(), {
        context: userContext(users.bob, spaceId),
    }).replica.push({
        scope: spaceId,
        mutations: [
            { id: RequestId.create(Date.now()), calls: [create] },
            { id: RequestId.create(Date.now()), calls: [join, create] },
        ],
    });
    const notes = await storage.database
        .select({ owner: note.table.ownerId, title: note.table.title })
        .from(note.table);

    expect({
        outcomes: pushed.outcomes.map(({ outcome }) =>
            "error" in outcome ? outcome.error.code : outcome.value,
        ),
        notes,
    }).toEqual({
        outcomes: ["BAD_REQUEST", "BAD_REQUEST"],
        notes: [],
    });
});

test("send a change of replicated rows to the publisher of the replica keeping them, not the subscriber's uplink", async () => {
    // keep the notes at home, replicated into a follower whose uplink refuses changes
    const storage = await TestDatabase.create("sqlite", noteDatabase, { isMigrated: true });
    const replicas = await TestDatabase.create("sqlite", followerDatabase, { isMigrated: true });
    onTestFinished(async () => {
        await Promise.all([storage.close(), replicas.close()]);
    });
    await openSpace(storage.database, spaceId);
    const home = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([note]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: { package: note.package },
    });
    const noteShape = defineShape({
        name: "notes",
        parameters: schema.object({}),
        objects: [note],
        queries: () => [note.query()],
    });
    const refused: string[] = [];
    const follower = new ObjectStore({
        chain: async () => ({ kind: "follower" }),
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: [
            defineServer(PackageHandle.of(note.package, {}), {
                follows: [Follow.of(noteShape, { keys: [{ key: { scope: spaceId } }] })],
            }),
        ],
        database: replicas.database,
        origin: { package: note.package },
        publisher: () => ({
            stream: async function* (_subscription, signal) {
                await until(signal);
                yield* [];
            },
            receive: (mutation: Mutation) =>
                home.receiveFrom(mutation, userContext(users.bob, spaceId).access()),
        }),
        uplink: {
            stream: async function* (_subscription, signal) {
                await until(signal);
                yield* [];
            },
            receive: async (mutation) => {
                refused.push(mutation.id);
            },
        },
    });

    // receive bob's note through the follower
    const created = note.calls().create({ title: "Bob's" });
    await follower.receive(
        {
            id: RequestId.create(Date.now()),
            calls: [{ ...created, input: { ...created.input, spaceId } }],
        },
        spacePrincipal(spaceId),
    );
    const notes = await storage.database
        .select({ owner: note.table.ownerId, title: note.table.title })
        .from(note.table);

    expect({ notes, refused }).toEqual({
        notes: [{ owner: users.bob, title: "Bob's" }],
        refused: [],
    });
});

test("refuse an in-process change of replicated rows, and send it toward their home for the caller's subject under its grant", async () => {
    // serve the notes over a follower's replicas of them, whose uplink receives nothing
    const replicas = await TestDatabase.create(
        "sqlite",
        defineDatabase({
            name: "main",
            tables: [outbox, ...accessTables, ...callTables],
            replicated: [note.table],
        }),
        { isMigrated: true },
    );
    onTestFinished(() => replicas.close());
    const follower = new ObjectStore({
        chain: async () => ({ kind: "follower" }),
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([note]),
        database: replicas.database,
        callKey: testCallKey,
        events: await testEvents(replicas.database),
        origin: { package: note.package },
        uplink: {
            stream: async function* (_subscription, signal) {
                await until(signal);
                yield* [];
            },
            receive: async () => {},
        },
    });

    // refuse writing a note through the replica as an installation acting for a person under a grant
    const person = principal.user.reference("universe", users.frank);
    const grant = { id: Identifier.create("grant"), scope: person.id, permissions: [] };
    const caller = AccessContext.of(
        {
            ...Caller.of(person),
            actors: [principal.installation.reference(spaceId, "installation-notes")],
            grant,
        },
        Date.now(),
    );
    const call = note.call("create", { spaceId, title: "First" });
    await expect(
        follower.mutate(caller, { id: RequestId.create(Date.now()), calls: [call] }),
    ).rejects.toThrow(
        new CallError("CONFLICT", {
            message: `note rows of ${spaceId} are replicas here, so send their changes to their home`,
        }),
    );

    // send it instead, which the outbox keeps with the exchange for the person
    await Transaction.run(replicas.database, (transaction) =>
        follower.send(call, transaction, {
            exchange: { subject: person, grant: grant.id },
        }),
    );
    const sent = await replicas.database
        .select({ id: outbox.id, message: outbox.message })
        .from(outbox);
    const [only] = sent;
    expect(sent.map((row) => schema.json().parse(JSON.parse(row.message)))).toEqual([
        {
            requestId: present(only, "the sent call").id,
            request: { call: call, exchange: { subject: person, grant: grant.id } },
        },
    ]);
});
