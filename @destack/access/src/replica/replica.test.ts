import { expect, onTestFinished, test } from "@destack/test";
import { Snapshot, asc, eq, type DatabaseConnection } from "@destack/db";
import { Feed, Replica, Scope, type Subscription } from "@destack/sync";
import { aligned, Clock } from "@destack/schema";
import {
    AccessError,
    accessTables,
    accessRelationship,
    CHAIN_PUBLICATION,
    Authorization,
    Authorizer,
    principal,
    Role,
    type AccessContext,
} from "../index.ts";
import {
    account,
    accountTable,
    item,
    homeMappings,
    mappings,
    node,
    policies,
    policyTable,
    space,
    spaceTable,
} from "../test/fixture.ts";
import { openFixture } from "../test/database.ts";

/** The object types whose universe relationships the tests' decisions read: every test type's. */
const TYPES = policies.map((each) => each.typeReference);

/** The owner of the account and the space. */
const alice: AccessContext = {
    subject: principal.user.reference("universe", "alice"),
    groups: [],
    now: 1000,
    attributes: {},
    actors: [],
    contacts: [],
    linkSecrets: [],
};

/** A member of the account who has roles through the membership. */
const carol: AccessContext = {
    subject: principal.user.reference("universe", "carol"),
    groups: [{ ...account.reference("universe", "account-1"), relation: "member" }],
    now: 1000,
    attributes: {},
    actors: [],
    contacts: [],
    linkSecrets: [],
};

/** Build a scope's chain replica for an authorizer's database, with its subscription. */
function chainOf(authorizer: Authorizer, scope: string) {
    const subscription = Authorizer.chainSubscription({
        name: Authorizer.chainReplica(scope),
        scope,
        below: scope,
        parameters: {
            local: [...authorizer.local],
            replicated: [...authorizer.replicated],
            via: [],
            between: [],
            types: TYPES,
        },
    });

    return { replica: authorizer.replicateChain(subscription), subscription };
}

test("relay an account's access through the space's database into an app's, and follow its revocation", async () => {
    // open the universe's database with accounts and spaces, the space's own, and an app's with notes
    const global = await openFixture();
    const regional = await openFixture();
    const app = await openFixture();
    onTestFinished(async () => {
        await Promise.all([global.close(), regional.close(), app.close()]);
    });
    const accounts = new Authorizer(policies, homeMappings);
    const spaces = new Authorizer(policies, mappings);
    const notes = app.authorizer;

    // create the account owned by alice, with a reader role bound to its members
    await global.database.insert(accountTable).values({ id: "account-1", scope: "universe" });
    const object = account.reference("universe", "account-1");
    const owner = <Value>(call: (authorization: Authorization) => Promise<Value>) =>
        Authorization.run(accounts, global.database, () => alice, call);
    await owner((authorization) => authorization.create(object, { owner: alice.subject }));
    const reader = await owner((authorization) =>
        authorization.createRole(object, {
            name: "reader",
            description: "Read every note",
            permissions: [node.permission("read")],
        }),
    );
    const binding = await owner((authorization) =>
        authorization.grant({
            object,
            role: reader.id,
            subject: aligned(carol.groups, 0),
        }),
    );

    // create the space in the universe beside its account, and replica both into the space's database
    await global.database.insert(spaceTable).values({ id: "personal", account: "account-1" });
    await owner((authorization) =>
        authorization.create(space.reference("account-1", "personal"), {
            owner: alice.subject,
        }),
    );
    const accountFeed = new Feed(global.database, [...accessTables, policyTable], {
        clock: Clock.system,
    });
    await replicate(regional.database, chainOf(spaces, "account-1"), accountFeed);
    await replicate(regional.database, chainOf(spaces, "personal"), accountFeed);

    // relay the space and the account from the cell's database into the app's, where carol reads every note
    const spaceFeed = new Feed(regional.database, [...accessTables, policyTable], {
        clock: Clock.system,
    });
    const relay = async () => {
        await replicate(regional.database, chainOf(spaces, "account-1"), accountFeed);
        await replicate(regional.database, chainOf(spaces, "personal"), accountFeed);
        await replicate(app.database, chainOf(notes, "account-1"), spaceFeed);
        await replicate(app.database, chainOf(notes, "personal"), spaceFeed);
    };
    await relay();
    const readable = async (context: AccessContext) =>
        (
            await app.database
                .select({ id: item.id })
                .from(item)
                .where(
                    notes.where(
                        node.permission("read"),
                        await notes.resolve(Snapshot.live(app.database), "personal", context),
                    ),
                )
                .orderBy(asc(item.id))
        ).map((row) => row.id);
    expect(await readable(carol)).toEqual(["a", "b", "c"]);

    // revoke the binding in the account's database and refuse deciding on unconfirmed replicas
    await owner((authorization) => authorization.revoke(object, binding.id));
    const revoked = await global.database.log.position();
    const impatient = new Authorizer(policies, mappings, { lag: 50 });
    const late = { ...carol, now: Date.now() + 60 };
    await expect(
        impatient.resolve(Snapshot.live(app.database), "personal", late),
    ).rejects.toMatchObject({
        code: "STALE",
    });

    // relay the revocation: carol reads nothing, and the app's replica reflects the account's own position
    await relay();
    expect(await readable(carol)).toEqual([]);
    const origins = await Replica.origins(app.database, CHAIN_PUBLICATION, ["account-1"]);
    expect(origins.get("account-1")?.position).toEqual(revoked);

    // refuse writing the account's access in the app's replica
    await expect(
        Authorization.run(
            notes,
            app.database,
            () => alice,
            (authorization) =>
                authorization.grant({
                    object,
                    relation: "member",
                    subject: principal.user.reference("universe", "dave"),
                }),
        ),
    ).rejects.toEqual(
        new AccessError(
            "FORBIDDEN",
            "cannot write the access of account account-1 here: this database has a replica",
        ),
    );

    // keep the app's own relationships, about its notes, through a fresh snapshot of the space's replica
    await replicate(app.database, chainOf(notes, "personal"), spaceFeed, "snapshot");
    const own = await app.database
        .select({ type: accessRelationship.objectType, objectId: accessRelationship.objectId })
        .from(accessRelationship)
        .orderBy(asc(accessRelationship.objectType));
    expect(own.filter((row) => row.type === node.name)).toEqual([{ type: "node", objectId: "b" }]);
});

test("write a home's own access at the home as it is born, beside the chain it replicates, and none where its creator keeps its row", async () => {
    // create an account and its space owned by alice in the universe, which keeps the space's row alone
    const global = await openFixture();
    const home = await openFixture();
    onTestFinished(async () => {
        await Promise.all([global.close(), home.close()]);
    });
    const accounts = new Authorizer(policies, homeMappings, { partition: Scope.universe.id });
    const spaces = new Authorizer(policies, mappings, { partition: "personal" });
    const owner = <Value>(call: (authorization: Authorization) => Promise<Value>) =>
        Authorization.run(accounts, global.database, () => alice, call);
    await global.database.insert(accountTable).values({ id: "account-1", scope: "universe" });
    await owner((authorization) =>
        authorization.create(account.reference("universe", "account-1"), {
            owner: alice.subject,
        }),
    );
    await global.database.insert(spaceTable).values({ id: "personal", account: "account-1" });
    const personal = space.reference("account-1", "personal");
    await owner((authorization) => authorization.create(personal, { owner: alice.subject }));

    // replicate the home's chain before it is born, keep its row, and receive it with its owner
    const feed = new Feed(global.database, [...accessTables, policyTable], { clock: Clock.system });
    const born = await spaces.chain(
        home.database,
        "personal",
        { kind: "home", parent: "account-1" },
        TYPES,
    );
    for (const subscription of born) {
        await replicate(
            home.database,
            { replica: spaces.replicateChain(subscription), subscription },
            feed,
        );
    }
    await home.database.insert(spaceTable).values({ id: "personal", account: "account-1" });
    await Authorization.run(
        spaces,
        home.database,
        () => alice,
        (authorization) => authorization.receive(personal, ["account-1"], { owner: alice.subject }),
    );

    // bind alice to the one owner role at the home, and write nothing of the space's access in the universe
    expect({
        home: await bound(home.database),
        universe: await bound(global.database),
        keeps: await spaces.keeps(Snapshot.live(home.database), "personal"),
    }).toEqual({
        home: [[Role.OWNER, "alice"]],
        universe: [],
        keeps: true,
    });

    // grant on the space at its home, and refuse it at the universe
    const grant = (authorizer: Authorizer, database: DatabaseConnection) =>
        Authorization.run(
            authorizer,
            database,
            () => alice,
            (authorization) =>
                authorization.grant({
                    object: personal,
                    role: Role.OWNER,
                    subject: principal.user.reference("universe", "dave"),
                }),
        );
    await grant(spaces, home.database);
    await expect(grant(accounts, global.database)).rejects.toEqual(
        new AccessError(
            "FORBIDDEN",
            "cannot write the access of space personal here: it lives at its home",
        ),
    );
});

/** Read the role and subject of each relationship on the personal space. */
async function bound(database: DatabaseConnection) {
    const rows = await database
        .select({ roleId: accessRelationship.roleId, subjectId: accessRelationship.subjectId })
        .from(accessRelationship)
        .where(eq(accessRelationship.objectId, "personal"));

    return rows.map((row) => [row.roleId, row.subjectId]);
}

/** Apply a source's feed to a replica until it reaches the source's head. */
async function replicate(
    database: DatabaseConnection,
    chain: { readonly replica: Replica; readonly subscription: Subscription },
    feed: Feed,
    from: "position" | "snapshot" = "position",
): Promise<void> {
    // read the pages up to the source's head
    const { replica, subscription } = chain;
    await replica.register(database, { subscription, clock: Clock.system });
    const head = await feed.database.log.position();
    const after = from === "snapshot" ? undefined : await replica.position(database);
    const pages = [];
    for await (const page of feed.subscribe(replica.queries, after, AbortSignal.timeout(5000))) {
        pages.push(page);
        if (page.complete && page.position.sequence >= head.sequence) {
            break;
        }
    }

    // apply them in order
    await Array.fromAsync(replica.apply(database, pages, { clock: Clock.system }));
}
