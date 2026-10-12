import { reconciliation, testCallKey } from "@destack/service/test";
import { expect, onTestFinished, test } from "@destack/test";
import { accessTables, principal, relation, Caller } from "@destack/access";
import { journal } from "@destack/audit/stack";
import { asc, defineDatabase, Transaction } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { schema, present, Clock } from "@destack/schema";
import { defineShape, type Mutation, Replica } from "@destack/sync";

import { outbox } from "@destack/service/outbox";
import { ServiceError } from "@destack/service/error";
import { RequestId } from "@destack/service/request";
import type { RunDelivery, RunRequest } from "@destack/service/trigger";
import { defineObject, field } from "../src/index.ts";
import { ObjectStore } from "../src/server/index.ts";
import { until } from "@destack/service/timer";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { userContext } from "./fixture/user.ts";
import { createRouterClient } from "@destack/service/server";

/** The space with the accounts. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000021");

/** Accounts their owners sign up, each welcomed by a mail sent after the sign-up commits. */
const member = defineObject({
    name: "member",
    plural: "members",
    scope: space,
    fields: { owner: field.reference(principal.user).caller(), name: field.string() },
    permissions: { write: relation("owner") },
    methods: (method) => ({
        create: method.create("write", { fields: ["name"] }),
        welcome: method.update("write", { fields: ["name"] }),
    }),
}).implement({
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

/** Notes their owners write and rename, which a follower copies and changes through the server keeping them. */
const note = defineObject({
    name: "note",
    plural: "notes",
    scope: space,
    fields: { owner: field.reference(principal.user).caller(), title: field.string() },
    permissions: { write: relation("owner") },
    methods: (method) => ({
        create: method.create("write", { fields: ["title"] }),
        rename: method.update("write", { fields: ["title"] }),
    }),
});

/** The database keeping the notes. */
const noteDatabase = defineDatabase({
    name: "main",
    tables: [journal, outbox, ...note.tables],
});

/** A follower's database copying the notes. */
const followerDatabase = defineDatabase({
    name: "main",
    tables: [outbox, ...accessTables],
    copies: [note.table],
});

/** The database with the members, the journal and the outbox. */
const memberDatabase = defineDatabase({
    name: "main",
    tables: [journal, outbox, ...member.tables],
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
            clock: Clock.system,
            principal: spacePrincipal(spaceId),
            objects: { member },
            database: storage.database,
            callKey: testCallKey,
            origin: {
                package: member.package,
                service: "test",
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
                        throw new ServiceError("BAD_REQUEST", { message: "invalid call" });
                    }
                    recorded.push([request, delivery]);
                },
            },
        });
        const context = userContext("user-1", spaceId);
        const signUp = (name: string) =>
            store
                .mutate(context.caller(), {
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

        // record Ada's welcome past the refused one, reporting the refusal once
        expect([
            recorded.map(([request, delivery]) => [request, typeof delivery?.requestId]),
            reports,
        ]).toEqual([
            [
                [
                    {
                        call: member.calls().welcome({ id: "member-1", name: "Ada" }),
                    },
                    "string",
                ],
            ],
            ["a sent call was refused for good"],
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "send a follower's change of copied rows over its uplink to the server keeping them, run there as the follower, and let go of one it may not make, on %s",
    async (dialect) => {
        // keep the notes in their own database, with ada's note written there
        const storage = await TestDatabase.create(dialect, noteDatabase, { isMigrated: true });
        const copies = await TestDatabase.create(dialect, followerDatabase, { isMigrated: true });
        onTestFinished(async () => {
            await Promise.all([storage.close(), copies.close()]);
        });
        await openSpace(storage.database, spaceId);
        const home = new ObjectStore({
            clock: Clock.system,
            principal: spacePrincipal(spaceId),
            objects: { note },
            database: storage.database,
            callKey: testCallKey,
            origin: { package: note.package, service: "test" },
        });
        const written = await home
            .mutate(userContext("ada", spaceId).caller(), {
                id: RequestId.create(Date.now()),
                calls: [note.call("create", { spaceId, title: "Ada's" })],
            })
            .then(([created]) => created);

        // follow them as bob, whose uplink runs his changes at home
        const reports: string[] = [];
        const follower = new ObjectStore({
            clock: Clock.system,
            principal: spacePrincipal(spaceId),
            objects: {},
            policies: [note],
            database: copies.database,
            origin: { package: note.package, service: "test" },
            report: (error) => {
                if (!(error instanceof Error)) {
                    throw error;
                }
                reports.push(error.message);
            },
            subscriber: {
                uplink: {
                    stream: async function* (_subscription, signal) {
                        await until(signal);
                        yield* [];
                    },
                    receive: (mutation) => home.receiveFrom(mutation, userContext("bob", spaceId)),
                },
                subscriptions: async () => [],
            },
        });

        // write a note and rename ada's through the copy, delivering the sends twice to show one delivery
        const transaction = await Transaction.open(copies.database);
        await follower.mutate(
            Caller.of(spacePrincipal(spaceId), Date.now()),
            {
                id: RequestId.create(Date.now()),
                calls: [
                    note.call("create", { spaceId, title: "Bob's" }),
                    note.call("rename", { spaceId, id: written.id, title: "Bob's now" }),
                ],
            },
            { transaction },
        );
        await transaction.commit();
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
                { owner: "ada", title: "Ada's" },
                { owner: "bob", title: "Bob's" },
            ],
            reports: ["a sent call was refused for good"],
        });
    },
);

test("refuse a pushed mutation of copied rows, alone or beside kept rows, which only their home writes", async () => {
    // keep the notes at home and copy them in a follower serving members
    const storage = await TestDatabase.create("sqlite", noteDatabase, { isMigrated: true });
    const copies = await TestDatabase.create(
        "sqlite",
        defineDatabase({
            name: "main",
            tables: [outbox, ...accessTables, journal, ...member.tables],
            copies: [note.table],
        }),
        { isMigrated: true },
    );
    onTestFinished(async () => {
        await Promise.all([storage.close(), copies.close()]);
    });
    await openSpace(storage.database, spaceId);
    await openSpace(copies.database, spaceId);
    const follower = new ObjectStore({
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        objects: { member },
        policies: [note],
        database: copies.database,
        callKey: testCallKey,
        origin: { package: note.package, service: "test" },
    });

    // push a note's creation as bob, then one beside a member's
    const create = note.calls().create({ title: "Bob's" });
    const join = member.calls().create({ name: "Bob" });
    const pushed = await createRouterClient(follower.router(), {
        context: userContext("bob", spaceId),
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

test("send a change of copied rows to the publisher of the copy keeping them, not the subscriber's uplink", async () => {
    // keep the notes at home, copied into a follower from a publisher receiving changes beside an uplink refusing them
    const storage = await TestDatabase.create("sqlite", noteDatabase, { isMigrated: true });
    const copies = await TestDatabase.create("sqlite", followerDatabase, { isMigrated: true });
    onTestFinished(async () => {
        await Promise.all([storage.close(), copies.close()]);
    });
    await openSpace(storage.database, spaceId);
    const home = new ObjectStore({
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        objects: { note },
        database: storage.database,
        callKey: testCallKey,
        origin: { package: note.package, service: "test" },
    });
    const noteShape = defineShape({
        name: "notes",
        parameters: schema.object({}),
        audience: "reader",
        isReadinessGate: true,
        replica: ({ name, scope }) => new Replica({ name, scope, tables: [note.table] }),
    });
    const refused: string[] = [];
    const follower = new ObjectStore({
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        objects: {},
        policies: [note],
        database: copies.database,
        origin: { package: note.package, service: "test" },
        shapes: [noteShape],
        subscriber: {
            uplink: {
                stream: async function* (_subscription, signal) {
                    await until(signal);
                    yield* [];
                },
                receive: async (mutation) => {
                    refused.push(mutation.id);
                },
            },
            subscriptions: async () => [
                {
                    subscription: noteShape.subscription({
                        name: "notes",
                        scope: spaceId,
                        below: "follower",
                        parameters: {},
                    }),
                    publisher: {
                        stream: async function* (_subscription, signal) {
                            await until(signal);
                            yield* [];
                        },
                        receive: (mutation: Mutation) =>
                            home.receiveFrom(mutation, userContext("bob", spaceId)),
                    },
                },
            ],
        },
    });

    // receive bob's note through the follower
    const created = note.calls().create({ title: "Bob's" });
    await follower.receive(
        { ...created, input: { ...created.input, spaceId } },
        spacePrincipal(spaceId),
    );
    const notes = await storage.database
        .select({ owner: note.table.ownerId, title: note.table.title })
        .from(note.table);

    expect({ notes, refused }).toEqual({ notes: [{ owner: "bob", title: "Bob's" }], refused: [] });
});
