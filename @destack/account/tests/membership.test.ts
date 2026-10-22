import { STACKS } from "../src/test/index.ts";
import { AccessContext, accessTables, Caller, principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { defineDatabase, Transaction, type DatabaseConnection, Snapshot } from "@destack/db";
import { TestDatabase, reconciliation } from "@destack/db/test";
import { FeedPublisher } from "@destack/object/test";
import { tablesOf } from "@destack/object";
import { implementPackage, ObjectStore } from "@destack/object/server";
import { PackageHandle } from "@destack/object";
import { Package } from "@destack/package";
import { Identifier, present } from "@destack/schema";
import { Feed, Scope, type Subject } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import {
    account,
    organisation,
    placement,
    residency,
    space,
    spaceShape,
    user,
} from "../src/object/index.ts";
import accountPackage from "../src/package.ts";
import { Account } from "../src/object/account.ts";
import { Follow } from "@destack/object";
import { Clock } from "@destack/environment";

/** How long a replica may take to catch up: 5 s, far above the milliseconds it takes. */
const REPLICA_MILLISECONDS = 5000;

/** The types whose rows and access a space's chain replica decides with. */
const CHAIN_TYPES = [user, organisation, residency, account, space];

/** The space's chain, handing down the types it decides with. */
const CHAIN = Follow.chain(CHAIN_TYPES);

/** The tables the universe publishes to the space: access and the rows the space follows. */
const PUBLISHED = [
    ...accessTables,
    ...[...CHAIN_TYPES, ...spaceShape.objects].map((type) => type.table),
];

test("decide a space's access in the space from its row and the relationships on it, homed at the universe and replicated down live", async () => {
    // store alice's account with a space at the universe, and serve the space's replica of its chain
    const { universe, spaceId, accountId, people, replica, follow } = await openSpace();
    const reference = space.reference(accountId, spaceId);
    const access = new AccessFixture(universe);
    await access.replicateRelationship(accountId, reference, "viewer", people.bob);
    await access.replicateRelationship(accountId, reference, "guest", people.carol);
    await follow();

    // decide reading and listing the space in the space as each person
    const decide = (subject: Subject, name: "read" | "list") =>
        decideIn(replica, spaceId, subject, space.permission(name), reference);
    const decisions = async () => ({
        read: [
            await decide(people.alice, "read"),
            await decide(people.bob, "read"),
            await decide(people.carol, "read"),
            await decide(people.dave, "read"),
        ],
        list: [await decide(people.carol, "list"), await decide(people.dave, "list")],
    });

    // let the account's owner and the viewer read it, the guest find it alone, and nobody else either
    expect(await decisions()).toEqual({
        read: [true, true, false, false],
        list: [true, false],
    });

    // let a viewer related at the universe later read it in the space once the replica has the tuple
    await access.replicateRelationship(accountId, reference, "viewer", people.dave);
    await follow();
    expect(await decide(people.dave, "read")).toBe(true);
});

/** Store an account of alice with one space at the universe, and its space's store following it. */
async function openSpace() {
    // store the universe's people, account and space
    const universeStorage = await TestDatabase.create(
        defineDatabase({
            name: "account",
            ...tablesOf({ packages: [accountPackage], follows: [], tables: [] }),
        }),
        { isMigrated: true },
    );
    const universe = universeStorage.database;
    const ids = await insertAccount(universe);

    // store the space's rows and its chain as the space's follows bring them
    const follows = [
        CHAIN,
        Follow.of(spaceShape, {
            keys: [{ key: { scope: ids.accountId, space: ids.spaceId, machine: ids.machineId } }],
        }),
    ];
    const spaceStorage = await TestDatabase.create(
        defineDatabase({ name: "main", ...tablesOf({ packages: [], follows, tables: [] }) }),
        { isMigrated: true },
    );
    onTestFinished(async () => {
        await Promise.all([universeStorage.close(), spaceStorage.close()]);
    });

    // serve the space's store, replicating the chain above the space from the universe
    const feed = new Feed(universe, PUBLISHED, { clock: Clock.system });
    const replica: ObjectStore = new ObjectStore({
        chain: async () => ({ kind: "home", parent: ids.accountId }),
        clock: Clock.system,
        principal: principal.space.reference(Scope.universe.id, ids.spaceId),
        servers: [implementPackage(PackageHandle.of(Package.of(accountPackage), {}), { follows })],
        policies: CHAIN_TYPES,
        database: spaceStorage.database,
        uplink: {
            ...FeedPublisher.of(
                feed,
                (subscription) => replica.source.buildReplica(subscription).queries,
            ),
            receive: () => Promise.reject(new Error("the replica receives no changes")),
            call: () => Promise.reject(new Error("the stand-in answers no queries")),
        },
        origin: { package: Package.of(accountPackage) },
    });

    // follow the chain, then give the space a separate scope below the replicated account
    const follow = await following(replica, universe);
    await follow();
    await new AccessFixture(spaceStorage.database).replicateScope(
        space.reference(ids.accountId, ids.spaceId),
        "self",
    );

    return { ...ids, universe, replica, follow };
}

/** Insert alice, bob, carol and dave, and an account of alice with one space, at the universe. */
async function insertAccount(universe: DatabaseConnection) {
    // insert the residency and the people
    const now = Date.now();
    const times = { createdAt: now, updatedAt: now };
    await universe.insert(residency.table).values({ id: "eu", name: "European Union", ...times });
    const createUser = async (name: string) => {
        const id = Identifier.create("user");
        await universe.insert(user.table).values({
            id,
            name,
            email: `${name}@example.test`,
            timeZone: "Europe/Vienna",
            locale: "en",
            homeId: Account.home(Account.personal(id)),
            ...times,
        });

        return id;
    };
    const userIds = {
        alice: await createUser("alice"),
        bob: await createUser("bob"),
        carol: await createUser("carol"),
        dave: await createUser("dave"),
    };
    const people = {
        alice: person(userIds.alice),
        bob: person(userIds.bob),
        carol: person(userIds.carol),
        dave: person(userIds.dave),
    };

    // store alice's account and its space, alice as the account's owner
    const accountId = Identifier.create("account");
    const spaceId = Identifier.create("space");
    const machineId = Identifier.create("machine");
    const owner = userIds.alice;
    await universe.insert(account.table).values({
        id: accountId,
        scope: owner,
        handle: "acme",
        name: "Acme",
        residencyId: "eu",
        kind: "personal",
        ...times,
    });
    await universe
        .insert(space.table)
        .values({ id: spaceId, scope: accountId, name: "notes", stack: STACKS.space, ...times });
    await universe.insert(placement.table).values({
        id: spaceId,
        scope: accountId,
        machineId,
        epoch: 1,
        ...times,
    });
    const access = new AccessFixture(universe);
    await access.replicateScope(user.reference(Scope.universe.id, owner));
    await access.replicateScope(account.reference(owner, accountId));
    await access.bindOwner(account.reference(owner, accountId), people.alice);

    return { accountId, spaceId, machineId, people };
}

/** Follow a store's chain replicas, answering a wait until each caught up with the universe. */
async function following(
    replica: ObjectStore,
    universe: DatabaseConnection,
): Promise<() => Promise<void>> {
    // follow each scope's replica until the test ends
    const follower = present(
        replica.controllers().find((each) => each.name === "replica"),
        "the replica controller",
    );
    const controller = new AbortController();
    const followed = new Map<string, Promise<unknown>>();
    onTestFinished(async () => {
        controller.abort();
        await Promise.allSettled(followed.values());
    });

    return async () => {
        // read the universe's head and start the replica deadline
        const head = await universe.log.position();
        const signal = AbortSignal.any([
            controller.signal,
            AbortSignal.timeout(REPLICA_MILLISECONDS),
        ]);

        // follow every replica the store lists now
        const keys = await follower.list(Snapshot.live(replica.database));
        for (const key of keys.filter((listed) => !followed.has(listed))) {
            followed.set(key, follower.reconcile(key, reconciliation(controller.signal)));
        }

        // wait until each listed scope arrives at the universe's head
        const scopes = new Set(
            keys.map((key) => present(key.split(" ")[1], `the scope of ${key}`)),
        );
        for (const scope of scopes) {
            expect(await replica.wait({ scope, ...head }, signal)).toBe(true);
        }
    };
}

/** Decide a subject's permission on an object in a store's scope. */
async function decideIn(
    store: ObjectStore,
    scope: string,
    subject: Subject,
    permission: ReturnType<typeof space.permission>,
    object: ReturnType<typeof space.reference>,
): Promise<boolean> {
    return Transaction.run(store.database, async (transaction) => {
        const authorization = await store.authorize(
            transaction,
            scope,
            AccessContext.of(Caller.of(subject), Date.now()),
            { now: Date.now() },
        );

        return (await authorization.check(permission, object)).isAllowed;
    });
}

/** Reference a person of the universe as a subject. */
function person(id: Identifier<"user">) {
    return principal.user.reference(Scope.universe.id, id);
}
