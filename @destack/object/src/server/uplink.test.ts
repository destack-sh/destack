import type { Environment } from "@destack/environment";
import { testSimulation } from "../test/index.ts";
import { testCallKey } from "../test/context.ts";
import { TestDatabase } from "@destack/db/test";
import { expect, onTestFinished, test, refusal } from "@destack/test";
import { AccessContext, Caller, none, principal, relation } from "@destack/access";
import { TestAccess } from "@destack/access/test";
import { type DatabaseConnection, defineDatabase, eq } from "@destack/db";
import { RequestId, Replica, Scope } from "@destack/sync";
import { present, schema } from "@destack/schema";
import { defineObject, defineShape, field, Shape } from "../index.ts";
import { followReplicas, serversOf, testEvents } from "../test/index.ts";
import { ObjectStore } from "./index.ts";
import { tablesOf } from "../stack/db.ts";
import objectPackage from "../package.ts";
import { TestObjectStore } from "../test/index.ts";
import { argument, plural, t } from "@destack/locale";

/** The space whose store replicates the people it joined. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000041");

/** A person of the universe, who joined the space. */
const adaId = schema.identifier("person").parse("person-01996ab0-0000-7000-8000-000000000042");

/** People of the universe, read and renamed by the spaces they joined, greeted by code at home. */
const person = defineObject(
    {
        name: "person",
        plural: "people",
        title: t`${plural(argument("count"), { one: "Person", other: "People" })}`,
        scope: "universe",
        fields: { name: field.string() },
        container: "scope",
        access: {
            relations: { joined: { subjects: [principal.space], grantedBy: null } },
            permissions: { read: relation("joined"), write: relation("joined") },
        },
    },
    (method) => ({
        update: method.update("write", { fields: ["name"] }),
        greeting: method.query({
            title: t`Greeting`,
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
    title: t`${plural(argument("count"), { one: "Label", other: "Labels" })}`,
    scope: "universe",
    fields: { text: field.string() },
    access: {
        permissions: { read: none() },
    },
});

/** The space's grant to its installation. */
const GRANT_ID = schema.identifier("grant").parse("grant-01996ab0-0000-7000-8000-000000000043");

/** The space acting as itself. */
const space = principal.space.reference(Scope.universe.id, spaceId);

/** The space's installation acting for the space under a stored grant giving some permissions. */
async function delegated(
    database: DatabaseConnection,
    environment: Pick<Environment, "clock" | "crypto" | "random">,
    permissions: readonly ("read" | "write")[],
): Promise<AccessContext> {
    // store the space's grant to its installation
    const installation = principal.installation.reference(spaceId, "installation-a");
    const granted = await new TestAccess(database, environment).replicateGrant(
        space,
        installation,
        permissions.map((name) => person.permission(name)),
        environment.clock.now(),
        GRANT_ID,
    );

    return AccessContext.of(
        { ...Caller.of(space), actors: [installation], grants: [granted] },
        environment.clock.now(),
    );
}

/** The people the space reads, followed at the universe. */
const PEOPLE = defineShape({
    name: "people",
    parameters: schema.object({}),
    objects: [person],
    queries: () => [person.query({})],
}).at({ key: { scope: Scope.universe.id } });

/** Serve ada at the universe, and a space store replicating her without serving people. */
async function serveHome(world: Environment) {
    // store ada at home, joined by the space
    const home = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { person }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { environment: world, isMigrated: true },
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
        { environment: world, isMigrated: true },
    );
    onTestFinished(async () => {
        await Promise.all([home.close(), replicating.close()]);
    });
    for (const database of [home.database, replicating.database]) {
        // store ada's scope and her joining the space, at home and in the space's access replica
        const access = new TestAccess(database, world);
        await access.replicateScope(person.reference(Scope.universe.id, adaId), person.policy);
        await access.replicateRelationship(
            person.reference(Scope.universe.id, adaId),
            "joined",
            space,
        );
    }
    await home.database.transaction(
        async (transaction) =>
            await transaction.insert(person.table).values({
                id: adaId,
                name: "Ada",
                scope: Scope.universe.id,
                createdAt: 1,
                updatedAt: 1,
            }),
    );

    // serve people at home, and replicate them into the space over the home's uplink
    const universe = TestObjectStore.of({
        chain: async () => undefined,
        environment: world,
        principal: Scope.universe,
        servers: serversOf([greeted]),
        database: home.database,
        callKey: testCallKey,
        events: await testEvents(home.database, world),
        origin: { package: person.package },
    });
    const store: ObjectStore = TestObjectStore.of({
        chain: async () => ({ kind: "follower", parent: Scope.universe.id }),
        environment: world,
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
        events: await testEvents(replicating.database, world),
        origin: { package: person.package },
    });

    // follow the universe's rows until the test ends
    const universeKey = `${Shape.replicaName(Scope.universe.id)} ${Scope.universe.id}`;
    const { keys, controller, running } = await followReplicas(store, {
        isFollowed: (key) => key === universeKey,
        report: () => {},
    });
    onTestFinished(async () => {
        controller.abort();
        await running;
    });
    expect(keys).toEqual([universeKey]);
    await Replica.wait(
        replicating.database,
        Scope.universe.id,
        await home.database.log.position(),
        world.clock.timeout(5000),
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
    // run in a world of its own
    const world = testSimulation().environment("test");

    const { store, name, database } = await serveHome(world);
    const rename = (value: string) => ({
        id: RequestId.create(store.environment),
        calls: [person.call("update", { id: adaId, name: value })],
    });

    // greet ada at home, as the space and as its installation reading for it
    const greeting = person.call("greeting", { id: adaId });
    const asSpace = await store.query(
        AccessContext.of(Caller.of(space), world.clock.now()),
        greeting,
    );
    const asInstallation = await store.query(await delegated(database, world, ["read"]), greeting);

    // rename her under a grant giving reading alone, then under one giving writing
    const refused = await outcome(
        store.mutate(await delegated(database, world, ["read"]), rename("Ada Lovelace")),
    );
    const unchanged = await name();
    const [renamed] = await store.mutate(
        await delegated(database, world, ["read", "write"]),
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
            "GRANT_REFUSES_PERMISSION_DENIED",
            `'person.write' in '${Scope.universe.id}' is forbidden: grant 'grant-01996ab0-0000-7000-8000-000000000043' does not give it`,
        ],
        unchanged: "Ada",
        renamed: "Ada L.",
        home: "Ada L.",
    });
});
