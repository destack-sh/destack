import { STACKS } from "../src/test/index.ts";
import { AccessContext, accessTables, Caller, principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { defineDatabase, Transaction, type DatabaseConnection } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { callTables } from "@destack/object/test";
import { defineServer, ObjectStore } from "@destack/object/server";
import { PackageHandle } from "@destack/runtime";
import { Package } from "@destack/package";
import { Identifier, present } from "@destack/schema";
import { reconciliation } from "@destack/runtime/test";
import { Feed, Replica, Scope, type Subject } from "@destack/sync";
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

/** The tables the universe publishes to the space: access, the chain's rows and the placement. */
const PUBLISHED = [...accessTables, ...CHAIN_TYPES.map((type) => type.table), placement.table];

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
        "sqlite",
        defineDatabase({
            name: "account",
            tables: [...callTables, ...CHAIN_TYPES.flatMap((type) => type.tables), placement.table],
        }),
        { isMigrated: true },
    );
    const spaceStorage = await TestDatabase.create(
        "sqlite",
        defineDatabase({
            name: "main",
            tables: accessTables,
            replicated: [...CHAIN_TYPES.map((type) => type.table), placement.table],
        }),
        { isMigrated: true },
    );
    onTestFinished(async () => {
        await Promise.all([universeStorage.close(), spaceStorage.close()]);
    });
    const universe = universeStorage.database;
    const ids = await insertAccount(universe);

    // serve the space's store, replicating the chain above the space from the universe
    const feed = new Feed(universe, PUBLISHED, { clock: Clock.system });
    const replica: ObjectStore = new ObjectStore({
        chain: async () => ({ kind: "home", parent: ids.accountId }),
        clock: Clock.system,
        principal: principal.space.reference(Scope.universe.id, ids.spaceId),
        servers: [
            defineServer(PackageHandle.of(Package.of(accountPackage), {}), {
                follows: [
                    Follow.of(spaceShape, {
                        keys: [{ key: { scope: ids.accountId, space: ids.spaceId } }],
                    }),
                ],
            }),
        ],
        policies: CHAIN_TYPES,
        database: spaceStorage.database,
        uplink: {
            receive: () => Promise.reject(new Error("the replica receives no changes")),
            stream: (subscription, signal) =>
                feed.subscribe(
                    replica.source.buildReplica(subscription).queries,
                    subscription.after,
                    signal,
                ),
        },
        origin: { package: Package.of(accountPackage) },
    });

    // follow the chain, then give the space its own scope below the replicated account
    const follow = await following(replica, spaceStorage.database, universe);
    await follow();
    await new AccessFixture(spaceStorage.database).replicateScope(
        space.reference(ids.accountId, ids.spaceId),
        "own",
    );

    return { ...ids, universe, replica, follow };
}

/** Insert alice, bob, carol and dave, and an account alice owns with one space, at the universe. */
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

    // store alice's account and its space, alice owning the account
    const accountId = Identifier.create("account");
    const spaceId = Identifier.create("space");
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
        machineId: Identifier.create("machine"),
        epoch: 1,
        ...times,
    });
    const access = new AccessFixture(universe);
    await access.replicateScope(user.reference(Scope.universe.id, owner));
    await access.replicateScope(account.reference(owner, accountId));
    await access.bindOwner(account.reference(owner, accountId), people.alice);

    return { accountId, spaceId, people };
}

/** Follow a store's chain replicas, answering a wait until each caught up with the universe. */
async function following(
    replica: ObjectStore,
    database: DatabaseConnection,
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
        // follow every replica the store lists now, and wait until each reached the universe's head
        const head = await universe.log.position();
        const signal = AbortSignal.any([
            controller.signal,
            AbortSignal.timeout(REPLICA_MILLISECONDS),
        ]);
        for (const key of await follower.list()) {
            if (!followed.has(key)) {
                followed.set(key, follower.reconcile(key, reconciliation(controller.signal)));
            }
            const scope = present(key.split(" ")[1], `the scope of ${key}`);
            expect(await Replica.wait(database, scope, head, signal)).toBe(true);
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
