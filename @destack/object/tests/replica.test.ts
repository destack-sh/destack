import { users } from "./fixture/user.ts";
import { AccessContext, AccessError, Caller, none, principal, relation } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { defineDatabase, TABLE } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { RequestId } from "@destack/runtime/request";
import { subjectContext, testCallKey } from "@destack/runtime/test";
import { RequestContext } from "@destack/runtime/server";
import { ResourceContext } from "@destack/resource/context";
import { Clock, Identifier, schema } from "@destack/schema";
import {
    ObjectReference,
    replica,
    Replica,
    replicaRow,
    replicaTables,
    Scope,
    Subject,
} from "@destack/sync";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { defineObject, defineShape, field, QUERIES_PUBLICATION, Shape } from "../src/index.ts";
import { defineServer, ObjectStore } from "../src/server/index.ts";
import { definePackage } from "@destack/runtime";
import { callTables, testEvents, serversOf } from "../src/test/index.ts";
import { spaceId } from "./fixture/device.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";

/** A mention of a person in a space, readable by that person wherever a replica keeps it. */
const mention = defineObject({
    name: "mention",
    plural: "mentions",
    scope: space,
    fields: { recipient: field.subject(), text: field.string() },
    relations: { recipient: { subjects: [principal.user] } },
    permissions: { read: relation("recipient"), post: none() },
}).methods((method) => ({
    create: method.create("post", {
        fields: ["recipientPackageId", "recipientType", "recipientScope", "recipientId", "text"],
    }),
}));

/** People, whose home space follows their replicas on their behalf. */
const person = defineObject({
    name: "user",
    plural: "users",
    scope: "universe",
    represents: principal.user,
    fields: {},
    relations: { home: { subjects: [principal.space] } },
    permissions: { follow: relation("home") },
}).methods();

/** The mentions of a space a home follows for its person. */
const mentionShape = defineShape({
    name: "mentions",
    parameters: schema.object({}),
    objects: [mention],
    queries: () => [mention.query({})],
});

test("list and follow the mentions a home keeps as replicas of another space, from the moment the replica appears", async () => {
    // keep a mention of alice in another space beside her home, both readable by anyone
    const storage = await TestDatabase.create(
        "sqlite",
        [...mention.tables, ...callTables, ...replicaTables],
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const { database } = storage;
    const other = Identifier.create("space");
    await openSpace(database, spaceId);
    await openSpace(database, other);
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([mention]),
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: mention.package },
    });
    const alice = principal.user.reference("universe", users.alice);
    const [created] = await store.mutate(
        AccessContext.of(Caller.of(spacePrincipal(other)), Date.now()),
        {
            id: RequestId.create(Date.now()),
            calls: [
                mention.call("create", {
                    spaceId: other,
                    ...ObjectReference.columns("recipient", alice),
                    text: "@alice",
                }),
            ],
        },
    );
    const { id } = schema.looseObject({ id: schema.string() }).parse(created);

    // follow the home's mentions as alice, then keep the mention in a replica decided for her below the home, as a follow on her behalf writes it
    const listed = async () => {
        const { items } = schema
            .looseObject({ items: schema.array(schema.looseObject({ id: schema.string() })) })
            .parse(
                await store.query(
                    AccessContext.of(Caller.of(alice), Date.now()),
                    mention.call("list", { spaceId }),
                ),
            );

        return items.map((item) => item.id);
    };
    const following = new AbortController();
    onTestFinished(() => following.abort());
    const pages = store.source.relayed(
        store.source.queriesPublication.subscription({
            name: "objects",
            scope: spaceId,
            below: spaceId,
            parameters: { queries: { mentions: { object: "mention" } } },
        }),
        subjectContext(alice, spaceId, { signal: following.signal }),
    );
    const first = await pages.next();
    const before = await listed();
    const table = mention.table[TABLE].sqlName;
    await database.transaction(async (replicating) => {
        await replicating.insert(replica).values({
            name: "mentions",
            scope: other,
            ...Replica.subscriptionColumns({
                name: "mentions",
                publication: QUERIES_PUBLICATION,
                scope: other,
                below: spaceId,
                subject: alice,
                parameters: {},
            }),
            audience: "caller",
            tables: [table],
        });
        await replicating.insert(replicaRow).values({
            name: "mentions",
            scope: other,
            table,
            key: JSON.stringify([table, id]),
            concealed: [],
        });
    });

    // list the replicated mention in the home, and send it to the follower once the replica appears
    const inserted = async () => {
        for await (const page of pages) {
            const changes = page.changes.filter((change) => change.table === table);
            if (changes.length > 0) {
                return changes.map((change) => [
                    change.operation,
                    change.row["id"],
                    change.row["scope"],
                ]);
            }
        }

        return [];
    };
    expect([first.done, before, await listed(), await inserted()]).toEqual([
        false,
        [],
        [id],
        [["insert", id, other]],
    ]);
});

test("follow a person's mentions on their behalf as the home they relate, and refuse another space", async () => {
    // keep mentions of alice and bob in a space, and relate alice's home as her home
    const storage = await TestDatabase.create(
        "sqlite",
        [...mention.tables, ...person.tables, ...callTables, ...replicaTables],
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const { database } = storage;
    const home = Identifier.create("space");
    const stranger = Identifier.create("space");
    await openSpace(database, spaceId);
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([mention, person]),
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: mention.package },
    });
    const aliceId = Identifier.create("user");
    const alice = principal.user.reference("universe", aliceId);
    const bob = principal.user.reference("universe", Identifier.create("user"));
    await store.mutate(AccessContext.of(Caller.of(spacePrincipal(spaceId)), Date.now()), {
        id: RequestId.create(Date.now()),
        calls: [alice, bob].map((recipient) =>
            mention.call("create", {
                spaceId,
                ...ObjectReference.columns("recipient", recipient),
                text: "@",
            }),
        ),
    });
    await database
        .insert(person.table)
        .values({ id: aliceId, scope: "universe", createdAt: 0, updatedAt: 0 });
    await new AccessFixture(database).replicateRelationship(
        "universe",
        person.reference("universe", aliceId),
        "home",
        spacePrincipal(home),
    );

    // read the first page of alice's mentions as a space, for the readers below it
    const subscription = Shape.subscription(mentionShape, { scope: spaceId }, home, alice);
    const first = async (context: RequestContext) => {
        const pages = store.source.relayed(subscription, context);
        const page = await pages.next();
        await pages.return(undefined);

        return page.done === true
            ? []
            : page.value.changes.map((change) => [change.operation, change.row["recipientId"]]);
    };
    const following = new AbortController();
    onTestFinished(() => following.abort());
    const as = (principalOf: string) =>
        subjectContext(spacePrincipal(principalOf), spaceId, { signal: following.signal });

    // decide the home's page for alice alone, and refuse a space alice does not relate
    expect(await first(as(home))).toEqual([["insert", alice.id]]);
    await expect(first(as(stranger))).rejects.toThrow(
        new AccessError("FORBIDDEN", "permission denied: follow"),
    );
});

test("admit a home's follow on its person's behalf with a token bound to the followed space, and refuse a token without the person's home set", async () => {
    // keep a mention of alice in a space, and her user row without any relationship
    const storage = await TestDatabase.create(
        "sqlite",
        [...mention.tables, ...person.tables, ...callTables, ...replicaTables],
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const { database } = storage;
    await openSpace(database, spaceId);
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([mention, person]),
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: mention.package },
    });
    const aliceId = Identifier.create("user");
    const alice = principal.user.reference("universe", aliceId);
    await store.mutate(AccessContext.of(Caller.of(spacePrincipal(spaceId)), Date.now()), {
        id: RequestId.create(Date.now()),
        calls: [
            mention.call("create", {
                spaceId,
                ...ObjectReference.columns("recipient", alice),
                text: "@",
            }),
        ],
    });
    await database
        .insert(person.table)
        .values({ id: aliceId, scope: "universe", createdAt: 0, updatedAt: 0 });

    // follow alice's mentions as her home with a token bound to the followed space, stating its sets
    const home = spacePrincipal(Identifier.create("space"));
    const subscription = Shape.subscription(mentionShape, { scope: spaceId }, home.id, alice);
    const following = new AbortController();
    onTestFinished(() => following.abort());
    const bound = (sets: readonly Subject[]) => {
        const now = Date.now();
        const request = new Request("https://test.local", { signal: following.signal });

        return new RequestContext(request, {
            audience: mention.package.id,
            scope: spaceId,
            authentication: {
                caller: { ...Caller.of(home), groups: sets },
                credential: {
                    kind: "space-key",
                    id: home.id,
                    audience: mention.package.id,
                    scope: spaceId,
                    verifiedAt: now,
                    expiresAt: now + 60_000,
                },
            },
            resources: new ResourceContext(),
            clock: Clock.system,
        });
    };
    const first = async (context: RequestContext) => {
        const pages = store.source.relayed(subscription, context);
        const page = await pages.next();
        await pages.return(undefined);

        return page.done === true
            ? []
            : page.value.changes.map((change) => [change.operation, change.row["recipientId"]]);
    };

    // admit the token stating her home set, and refuse one without it
    expect([
        await first(bound([{ ...alice, relation: "home" }])),
        await refusal(first(bound([]))),
    ]).toEqual([[["insert", alice.id]], ["FORBIDDEN", "permission denied: follow"]]);
});

test("refuse an anonymous follower of a subject's shape as unauthenticated, and another subject as forbidden", async () => {
    // serve a space's mentions and the people a home follows them for
    const storage = await TestDatabase.create(
        "sqlite",
        [...mention.tables, ...person.tables, ...callTables, ...replicaTables],
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const { database } = storage;
    await openSpace(database, spaceId);
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([mention, person]),
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: mention.package },
    });

    // follow alice's mentions without a credential, then as a space she does not relate
    const alice = principal.user.reference("universe", Identifier.create("user"));
    const subscription = Shape.subscription(mentionShape, { scope: spaceId }, spaceId, alice);
    const first = (context: RequestContext) => store.source.relayed(subscription, context).next();
    const anonymous = new RequestContext(new Request("https://test.local"), {
        audience: mention.package.id,
        scope: spaceId,
        resources: new ResourceContext(),
        clock: Clock.system,
    });
    const stranger = subjectContext(spacePrincipal(Identifier.create("space")), spaceId);

    // ask the anonymous follower to authenticate, and deny the authenticated stranger
    expect([await refusal(first(anonymous)), await refusal(first(stranger))]).toEqual([
        ["UNAUTHORIZED", "missing caller credential"],
        ["FORBIDDEN", "permission denied: follow"],
    ]);
});

test("refuse a follow with malformed chain parameters as a bad request", async () => {
    // serve a space's mentions
    const storage = await TestDatabase.create(
        "sqlite",
        [...mention.tables, ...callTables, ...replicaTables],
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const { database } = storage;
    await openSpace(database, spaceId);
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([mention]),
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: mention.package },
    });

    // follow the space's chain as the space itself with parameters naming no scopes
    const following = new AbortController();
    onTestFinished(() => following.abort());
    const pages = store.source.relayed(
        {
            name: `chain:${spaceId}`,
            publication: store.source.chainPublication.name,
            scope: spaceId,
            below: spaceId,
            parameters: { local: [], replicated: [], types: [] },
        },
        subjectContext(spacePrincipal(spaceId), spaceId, { signal: following.signal }),
    );

    // refuse the parameters as the follower's own bad input, naming the missing fields
    expect(await refusal(pages.next())).toEqual([
        "BAD_REQUEST",
        "invalid input: via: invalid input: expected array, received undefined; between: invalid input: expected array, received undefined",
    ]);
});

test("answer a replicated type's query over its replica rows, and refuse its mutations", async () => {
    // keep a realm's row as a replica of its home, enclosing the notes the store serves
    const realm = defineObject({
        name: "realm",
        plural: "realms",
        scope: "universe",
        isScope: true,
        partition: true,
        fields: { title: field.string() },
        permissions: { read: none(), write: none() },
    })
        .methods((method) => ({
            rename: method.update("write", { fields: ["title"] }),
            /** Read the realm's title. */
            describe: method.query({
                permission: "read",
                output: schema.object({ title: schema.string() }),
            }),
        }))
        .implement({
            methods: { describe: async (call) => ({ title: call.target.title }) },
        });
    const note = defineObject({
        name: "note",
        plural: "notes",
        scope: realm,
        fields: { text: field.string() },
        permissions: { read: none(), write: none() },
    }).methods((method) => ({ create: method.create("write") }));
    const storage = await TestDatabase.create(
        "sqlite",
        defineDatabase({
            name: "main",
            tables: [...callTables, ...note.tables],
            replicated: [realm.table],
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const { database } = storage;
    const realmId = Identifier.create("realm");
    const owner = principal.user.reference(Scope.universe.id, users.owner);
    const access = new AccessFixture(database);
    await access.replicateScope(realm.reference(Scope.universe.id, realmId), "own");
    await access.bindOwner(realm.reference(Scope.universe.id, realmId), owner);
    await database.insert(realm.table).values({
        id: realmId,
        scope: Scope.universe.id,
        title: "Atlas",
        createdAt: 1,
        updatedAt: 1,
    });

    // serve the notes, which make the enclosing realm known
    const store = new ObjectStore({
        servers: [defineServer(definePackage({ objects: { note } }), {})],
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(realmId),
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: note.package },
    });

    // answer the realm's query as its owner, and refuse renaming it, which its home does
    const caller = subjectContext(owner, realmId).access();
    expect({
        described: await store.query(caller, realm.call("describe", { id: realmId })),
        renamed: await refusal(
            store.mutate(caller, {
                id: RequestId.create(Date.now()),
                calls: [realm.call("rename", { id: realmId, title: "Globe" })],
            }),
        ),
    }).toEqual({
        described: { title: "Atlas" },
        renamed: ["CONFLICT", "realm rows are replicas here, so send their changes to their home"],
    });
});
