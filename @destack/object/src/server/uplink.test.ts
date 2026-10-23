import { testCallKey } from "../test/context.ts";
import { reconciliation, TestDatabase } from "@destack/db/test";
import { expect, onTestFinished, test, refusal } from "@destack/test";
import { AccessContext, Caller, none, principal, relation } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { type DatabaseConnection, defineDatabase, eq, Snapshot } from "@destack/db";
import { RequestId, Replica, Scope } from "@destack/sync";
import { present, schema } from "@destack/schema";
import { defineObject, defineShape, field, Follow } from "../index.ts";
import { serversOf, testEvents } from "../test/index.ts";
import { ObjectStore } from "./index.ts";
import { Clock } from "@destack/environment";
import { tablesOf } from "../stack/db.ts";
import objectPackage from "../package.ts";

/** The space whose store replicates the people it joined. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000041");

/** A person of the universe, who joined the space. */
const adaId = schema.identifier("person").parse("person-01996ab0-0000-7000-8000-000000000042");

/** People of the universe, read and renamed by the spaces they joined, greeted by code at home. */
const person = defineObject(
    {
        name: "person",
        plural: "people",
        scope: "universe",
        isScope: true,
        fields: { name: field.string() },
        relations: { joined: { subjects: [principal.space], grantedBy: null } },
        permissions: { read: relation("joined"), write: relation("joined") },
    },
    (method) => ({
        update: method.update("write", { fields: ["name"] }),
        greeting: method.query({
            permission: "read",
            output: schema.object({ greeting: schema.string() }),
        }),
    }),
);

/** People as the universe serves them, greeting each by name. */
const greeted = person.implement({
    methods: {
        greeting: async (context) => ({ greeting: `hello ${context.target.name}` }),
    },
});

/** Labels the space stores itself, so its store serves their methods. */
const label = defineObject({
    name: "label",
    plural: "labels",
    scope: "universe",
    fields: { text: field.string() },
    permissions: { read: none() },
});

/** The space's grant to its installation. */
const GRANT_ID = schema.identifier("grant").parse("grant-01996ab0-0000-7000-8000-000000000043");

/** The space acting as itself. */
const space = principal.space.reference(Scope.universe.id, spaceId);

/** The space's installation acting for the space under a stored grant giving some permissions. */
async function delegated(
    database: DatabaseConnection,
    permissions: readonly ("read" | "write")[],
): Promise<AccessContext> {
    // store the space's grant to its installation
    const installation = principal.installation.reference(spaceId, "installation-a");
    const id = await new AccessFixture(database).replicateGrant(
        space,
        installation,
        permissions.map((name) => person.permission(name)),
        Date.now(),
        GRANT_ID,
    );

    return AccessContext.of(
        { ...Caller.of(space), actors: [installation], grants: [{ id, scope: spaceId }] },
        Date.now(),
    );
}

/** The people the space reads, followed at the universe. */
const PEOPLE = Follow.of(
    defineShape({
        name: "people",
        parameters: schema.object({}),
        objects: [person],
        queries: () => [person.query({})],
    }),
    { keys: [{ key: { scope: Scope.universe.id } }] },
);

/** Serve ada at the universe, and a space store replicating her without serving people. */
async function serveHome() {
    // store ada at home, joined by the space
    const home = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { person }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        {
            isMigrated: true,
        },
    );
    const replicating = await TestDatabase.create(
        defineDatabase({
            name: "space",
            ...tablesOf({
                packages: [objectPackage, { objects: { label }, tables: [] }],
                follows: [PEOPLE],
                tables: [],
            }),
        }),
        { isMigrated: true },
    );
    onTestFinished(async () => {
        await Promise.all([home.close(), replicating.close()]);
    });
    for (const database of [home.database, replicating.database]) {
        // store ada's scope and her joining the space, at home and in the space's access replica
        const access = new AccessFixture(database);
        await access.replicateScope(person.reference(Scope.universe.id, adaId));
        await access.replicateRelationship(
            person.reference(Scope.universe.id, adaId),
            "joined",
            space,
        );
    }
    await home.database
        .insert(person.table)
        .values({ id: adaId, name: "Ada", scope: Scope.universe.id, createdAt: 1, updatedAt: 1 });

    // serve people at home, and replicate them into the space over the home's uplink
    const universe = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: Scope.universe,
        servers: serversOf([greeted]),
        database: home.database,
        callKey: testCallKey,
        events: await testEvents(home.database),
        origin: { package: person.package },
    });
    const store: ObjectStore = new ObjectStore({
        chain: async () => ({ kind: "follower", parent: Scope.universe.id }),
        clock: Clock.system,
        principal: space,
        servers: serversOf([label], { follows: [PEOPLE] }),
        policies: [person],
        database: replicating.database,
        uplink: {
            ...universe.uplink,
            stream: (asked, signal) =>
                universe.source.replicate(asked, asked.after, { subject: space }, signal),
        },
        callKey: testCallKey,
        events: await testEvents(replicating.database),
        origin: { package: person.package },
    });

    // follow the universe's rows until the test ends
    const follower = present(
        store.controllers().find((each) => each.name === "replica"),
        "the replica controller",
    );
    const controller = new AbortController();
    const key = present(
        (await follower.list(Snapshot.live(store.database))).find(
            (entry) =>
                entry ===
                `${PEOPLE.shape.replicaName({ scope: Scope.universe.id })} ${Scope.universe.id}`,
        ),
        "the universe's replica key",
    );
    const following = follower.reconcile(key, reconciliation(controller.signal));
    onTestFinished(async () => {
        controller.abort();
        await following.catch(() => undefined);
    });
    await Replica.wait(
        replicating.database,
        Scope.universe.id,
        await home.database.log.position(),
        AbortSignal.timeout(5000),
    );

    // read ada's name at home
    const name = async () => {
        const [row] = await home.database
            .select({ name: person.table.name })
            .from(person.table)
            .where(eq(person.table.id, adaId));

        return present(row, "ada at home").name;
    };

    return { store, name, database: replicating.database };
}

/** Read how a promise settles: its value, or its refusal's code, reason and message. */
async function outcome(promise: Promise<unknown>): Promise<unknown> {
    let value: unknown;
    const settled = await refusal(
        promise.then((answer) => {
            value = answer;
        }),
    );

    return settled === "done" ? value : settled;
}

test("answer a query nothing here handles at its rows' home, and send a write there within the caller's grant", async () => {
    const { store, name, database } = await serveHome();
    const rename = (value: string) => ({
        id: RequestId.create(Date.now()),
        calls: [person.call("update", { id: adaId, name: value })],
    });

    // greet ada at home, as the space and as its installation reading for it
    const greeting = person.call("greeting", { id: adaId });
    const asSpace = await store.query(AccessContext.of(Caller.of(space), Date.now()), greeting);
    const asInstallation = await store.query(await delegated(database, ["read"]), greeting);

    // rename her under a grant giving reading alone, then under one giving writing
    const refused = await outcome(
        store.mutate(await delegated(database, ["read"]), rename("Ada Lovelace")),
    );
    const unchanged = await name();
    const [renamed] = await store.mutate(
        await delegated(database, ["read", "write"]),
        rename("Ada L."),
    );

    // greet at home, refuse the write the grant does not give, and answer the given one's row
    expect({
        asSpace,
        asInstallation,
        refused,
        unchanged,
        renamed: schema.looseObject({ name: schema.string() }).parse(renamed).name,
        home: await name(),
    }).toEqual({
        asSpace: { greeting: "hello Ada" },
        asInstallation: { greeting: "hello Ada" },
        refused: [
            "FORBIDDEN",
            "GRANT_REFUSES_PERMISSION",
            `grant grant-01996ab0-0000-7000-8000-000000000043 does not give person.write in ${Scope.universe.id}`,
        ],
        unchanged: "Ada",
        renamed: "Ada L.",
        home: "Ada L.",
    });
});
