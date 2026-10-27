import type { Environment } from "@destack/environment";
import { testSimulation } from "@destack/object/test";
import { accountDatabase } from "@destack/account/stack";
import { testEvents, serversOf, TestObjectStore } from "@destack/object/test";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import {
    accessInvitation,
    accessRelationship,
    anyone,
    decisionTables,
    none,
    principal,
    Relationship,
    relation,
} from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { IdentityKeystore } from "@destack/account/identity";
import { LocalKeyring } from "@destack/identity";
import { account, Placement } from "@destack/account/object";
import { TestDirectory, TestIdentityRegistry } from "@destack/account/test";
import { spaceDatabase } from "../../test/index.ts";
import type { EventStore } from "@destack/event";
import { journal, defineObject, field } from "@destack/object";
import { type DatabaseConnection, Snapshot, Transaction } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { ObjectStore } from "@destack/object/server";
import { canonicalize, Identifier, present, schema } from "@destack/schema";
import { subjectContext, testCallKey } from "@destack/object/test";
import { RequestId, Scope, Replica, replicaTables } from "@destack/sync";
import { ProviderIndex, SpaceReplica, SpaceTransfer } from "../index.ts";

/** The identifiers the scenarios place, assign and publish. */
const ids = {
    machineA: schema.identifier("machine").parse("machine-01996ab0-0000-7000-8000-000000000201"),
    machineB: schema.identifier("machine").parse("machine-01996ab0-0000-7000-8000-000000000202"),
    owner: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-000000000203"),
};

/** The spaces of the keys, readable by anyone. */
const space = defineObject({
    name: "space",
    plural: "spaces",
    scope: "universe",
    isScope: true,
    fields: {},
    relations: { reader: { subjects: [anyone.all()] } },
    permissions: { read: relation("reader"), share: none() },
    shareable: { by: "share" },
});

/** The account the moved space belongs to in the directory. */
const accountId = schema
    .identifier("account")
    .parse("account-01996ab0-0000-7000-8000-000000000010");

/** The space the transfer moves. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000011");

/** Keys that a space's owners have, logged whole. */
const key = defineObject(
    {
        name: "key",
        plural: "keys",
        scope: space,
        fields: {
            owner: field.reference(principal.user).caller(),
            name: field.string(),
        },
        permissions: { read: relation("owner"), write: relation("owner") },
    },
    (method) => ({
        create: method.create("write", { fields: ["name"] }),
        update: method.update("write", { fields: ["name"] }),
    }),
);

/** The owner of the moving space. */
const OWNER = schema.identifier("user").parse("user-01996ab0-0000-7000-8000-000000000006");

test("hand a space's database to another machine and send its clients there", async () => {
    // run every party in the test's own world, on its clock
    const simulation = testSimulation();
    const { clock } = simulation;
    const world = simulation.environment("test");

    // open the source and target databases of the space and the universe placing it
    const tables = [
        ...new Set([...spaceDatabase.tables, ...key.tables, journal, ...replicaTables]),
    ];
    const [source, target, universe] = await Promise.all([
        TestDatabase.create(tables, { environment: world, isMigrated: true }),
        TestDatabase.create(tables, { environment: world, isMigrated: true }),
        TestDatabase.create(accountDatabase, { environment: world, isMigrated: true }),
    ]);
    onTestFinished(async () => {
        await Promise.all([source.close(), target.close(), universe.close()]);
    });
    await openSpace(source.database, world);
    await new AccessFixture(universe.database, world).replicateScope(
        account.reference(Scope.universe.id, accountId),
        account.policy,
    );
    const { registry } = await TestIdentityRegistry.open(universe.database, world, ids.machineA);
    const placement = { id: spaceId, scope: accountId, machine: ids.machineA, epoch: 1 };
    for (const machine of [ids.machineA, ids.machineB]) {
        await TestDirectory.publish(universe.database, machine, accountId, null, clock);
    }
    await TestDirectory.place(universe.database, placement, clock);

    // write keys to the space as its owner, and record each database's calls
    const events = new Map<DatabaseConnection, EventStore>([
        [source.database, await testEvents(source.database, world)],
        [target.database, await testEvents(target.database, world)],
    ]);
    const open = (database: DatabaseConnection) =>
        TestObjectStore.at(principal.space.reference("universe", spaceId), {
            chain: async () => undefined,
            environment: world,
            principal: principal.space.reference("universe", spaceId),
            servers: serversOf([key]),
            database,
            callKey: testCallKey,
            events: present(events.get(database), "the database's events"),
            origin: { package: key.package },
        });
    const stores = new Map<DatabaseConnection, ObjectStore>([
        [source.database, await open(source.database)],
        [target.database, await open(target.database)],
    ]);
    const serve = (database: DatabaseConnection) =>
        present(stores.get(database), "the database's store");
    const context = subjectContext(principal.user.reference("universe", OWNER), spaceId, {
        environment: world,
    });
    const create = (database: DatabaseConnection, name: string) =>
        serve(database)
            .mutate(context.access(), {
                id: RequestId.create(world),
                calls: [key.call("create", { spaceId, name })],
            })
            .then(([written]) => written);
    const deployed = schema
        .looseObject({ id: schema.string() })
        .parse(await create(source.database, "deploy"));

    // follow the space from its first page
    const reader = (signal: AbortSignal) =>
        subjectContext(principal.user.reference("universe", OWNER), spaceId, {
            environment: world,
            signal,
        });
    const read = () => {
        const { source: served } = serve(source.database);

        return served.relayed(
            served.queriesPublication.subscription({
                name: "objects",
                scope: spaceId,
                target: spaceId,
                parameters: {},
            }),
            reader(AbortSignal.timeout(4000)),
        );
    };
    const following = read();
    await following.next();

    // follow the source's rows on the target while the source serves them
    const keyTables = [...decisionTables, accessInvitation, key.table];
    const replicaOf = (database: DatabaseConnection) =>
        new SpaceReplica({ database, spaceId, scopes: [spaceId], tables: keyTables });
    const [leaving, arriving] = [replicaOf(source.database), replicaOf(target.database)];
    const transfer = (moving: SpaceReplica) =>
        new SpaceTransfer({ placement, target: ids.machineB, space: moving });
    const replicating = new AbortController();
    const replicated = arriving.replica
        .follow(
            target.database,
            ({ after }, stop) => leaving.feed().subscribe(arriving.replica.queries, after, stop),
            replicating.signal,
            { synced: () => {} },
        )
        .catch((error: unknown) => {
            if (!replicating.signal.aborted) {
                throw error;
            }
        });

    // fence the source to redirect further writes to the target machine
    const fenced = await transfer(leaving).fence(
        {
            providers: new ProviderIndex(
                schema.identifier("machine").parse("machine-01996ab0-0000-7000-8000-0000000000aa"),
                [],
            ),
            runtimes: [],
        },
        clock.now(),
    );
    const moved = ["MISDIRECTED_REQUEST", "SCOPE_MOVED", `${spaceId} moved to ${ids.machineB}`];
    const late = await refusal(create(source.database, "late"));

    // end the reads following the moved space and answer new ones with its new machine too
    const redirected = [
        late,
        await refusal(following.next()),
        await refusal(read().next()),
        await refusal(
            serve(source.database).query(
                context.access(),
                key.call("get", { spaceId, id: deployed.id }),
            ),
        ),
    ];
    expect(redirected).toEqual([moved, moved, moved, moved]);

    // wait for the fenced position on the target and verify the replica
    const isReached = await Replica.wait(
        target.database,
        spaceId,
        fenced.position,
        AbortSignal.timeout(4000),
    );
    replicating.abort();
    await replicated;
    const digest = await SpaceReplica.digest(
        target.database,
        arriving.digestTables(),
        arriving.scopes,
    );
    expect([isReached, digest]).toEqual([true, fenced.digest]);

    // promote the rows on the target and refuse writes until the space is located there at the next epoch
    const transferId = Identifier.create("transfer", world);
    await TestDirectory.place(universe.database, placement, clock, {
        target: ids.machineB,
        transferId,
    });
    await Transaction.run(target.database, (transaction) => arriving.replica.promote(transaction));
    const early = await refusal(create(target.database, "early"));

    // place the space on the target with the identity keys the move sent, and rekey them
    const keystore = new IdentityKeystore(
        await LocalKeyring.read(LocalKeyring.generate(world), world),
        registry,
        world,
    );
    await keystore.generate(target.database, spaceId);
    const carried = present(await registry.find(spaceId), "the carried identity");
    const publisher = await TestDirectory.uplink(universe.database, ids.machineB, world);
    const completed = await transfer(arriving).place(
        { publisher, keystore },
        target.database,
        world,
    );
    const rekeyed = present(await registry.find(spaceId), "the rekeyed identity");
    await create(target.database, "after");
    expect([
        early,
        completed,
        (await Placement.find(Snapshot.live(universe.database), [spaceId]))[0],
        [rekeyed.signingKeys.length, canonicalize(rekeyed) === canonicalize(carried)],
        (await target.database.select().from(key.table)).map((row) => row.name),
    ]).toEqual([
        moved,
        { ...placement, machine: ids.machineB, epoch: 2 },
        { ...placement, machine: ids.machineB, epoch: 2 },
        [1, false],
        ["deploy", "after"],
    ]);
});

/** Record the space as a replica of its home's access has it, with anyone as its reader. */
async function openSpace(database: DatabaseConnection, environment: Environment): Promise<void> {
    // record the space's scope
    const object = space.reference(Scope.universe.id, spaceId);
    await new AccessFixture(database, environment).replicateScope(object, space.policy);

    // relate anyone to the space as its reader
    await database.transaction(
        async (transaction) =>
            await transaction.insert(accessRelationship).values(
                Relationship.encode({
                    id: Identifier.create("relationship", environment),
                    object,
                    relation: "reader",
                    subject: anyone.reference("*", "*"),
                    createdAt: 0,
                    expiresAt: null,
                }),
            ),
    );
}
