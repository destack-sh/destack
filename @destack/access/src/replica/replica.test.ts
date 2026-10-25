import { TEST_START } from "@destack/test/world";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { Snapshot, asc, eq, type DatabaseConnection } from "@destack/db";
import { Feed, Replica, Scope, type Subscription } from "@destack/sync";
import { nth } from "@destack/schema";
import {
    AccessContext,
    accessRelationship,
    accessTables,
    Authorization,
    Authorizer,
    Caller,
    CHAIN_PUBLICATION,
    principal,
    Relationship,
    Role,
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
import { openDatabase, openFixture } from "../test/database.ts";

/** The owner of the account and the space. */
const alice = AccessContext.of(Caller.of(principal.user.reference("universe", "alice")), 1000);

/** A member of the account who has roles through the membership. */
const carol: AccessContext = {
    grants: [],
    subject: principal.user.reference("universe", "carol"),
    groups: [{ ...account.reference("universe", "account-1"), relation: "member" }],
    now: 1000,
    attributes: {},
    actors: [],
    contacts: [],
    linkSecrets: [],
};

/** Build a scope's chain replica for an authorizer's database, with its subscription. */
function chainOf(
    authorizer: Authorizer,
    scope: string,
    target = scope,
    scopes: readonly string[] = [target, scope],
) {
    const name = `${CHAIN_PUBLICATION}:${target}`;
    const local = scope === target ? authorizer.local : [];
    const parameters = {
        scopes: [...scopes],
        local: [...local],
        types: [],
        via: [],
        between: [],
    };
    const subscription: Subscription = {
        name,
        publication: CHAIN_PUBLICATION,
        scope,
        target,
        parameters,
    };

    return {
        replica: authorizer.replicateChain({ name, scope, target }, parameters),
        subscription,
    };
}

/** Build the universe's chain replica of a target, with the relationships on its chain. */
function universeOf(authorizer: Authorizer, chain: readonly string[]) {
    return chainOf(authorizer, Scope.universe.id, nth(chain, 0), chain);
}

test("relay an account's access through the space's database into an app's, and follow its revocation", async () => {
    // open the universe's database with accounts and spaces, the space's, and an app's with notes
    const global = await openDatabase();
    const regional = await openDatabase("personal");
    const app = await openFixture("app");
    onTestFinished(async () => {
        await Promise.all([global.close(), regional.close(), app.close()]);
    });
    const accounts = new Authorizer(policies, homeMappings, { partition: Scope.universe.id });
    const spaces = new Authorizer(policies, mappings);
    const notes = app.authorizer;

    // create the account of alice, with a reader role bound to its members
    await global.database.insert(accountTable).values({ id: "account-1", scope: "universe" });
    const object = account.reference("universe", "account-1");
    const owner = <Value>(call: (authorization: Authorization) => Promise<Value>) =>
        Authorization.run(accounts, global.database, () => alice, app.environment, call);
    await owner((authorization) => authorization.create(object, { owner: alice.subject }));
    const reader = await owner((authorization) =>
        Role.create(
            object,
            {
                name: "reader",
                description: "Read every note",
                permissions: [node.permission("read")],
            },
            authorization,
        ),
    );
    const binding = await owner((authorization) =>
        Relationship.grant(
            {
                object,
                role: reader.id,
                subject: nth(carol.groups, 0),
            },
            authorization,
        ),
    );

    // create the space in the universe beside its account
    await global.database.insert(spaceTable).values({ id: "personal", account: "account-1" });
    await owner((authorization) =>
        authorization.create(space.reference("account-1", "personal"), {
            owner: alice.subject,
        }),
    );
    const accountFeed = new Feed(global.database, [...accessTables, policyTable], {});

    // relay the universe, the account and the space through the space's database into the app's, where carol reads every note
    const spaceFeed = new Feed(regional.database, [...accessTables, policyTable], {});
    const chain = ["personal", "account-1", Scope.universe.id];
    const relay = async () => {
        await replicate(regional.database, universeOf(spaces, chain), accountFeed);
        await replicate(regional.database, chainOf(spaces, "account-1"), accountFeed);
        await replicate(regional.database, chainOf(spaces, "personal"), accountFeed);
        await replicate(app.database, universeOf(notes, chain), spaceFeed);
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
    await owner((authorization) => Relationship.revoke(object, binding.id, authorization));
    const revoked = await global.database.log.position();
    const impatient = new Authorizer(policies, mappings, { lag: 50 });
    const late = { ...carol, now: TEST_START + 60 };
    expect(await refusal(impatient.resolve(Snapshot.live(app.database), "personal", late))).toEqual(
        [
            "SERVICE_UNAVAILABLE",
            "STALE",
            "the home of personal has not confirmed access for over 50 ms",
        ],
    );

    // relay the revocation: carol reads nothing, and the app's replica reflects the universe's position
    await relay();
    expect(await readable(carol)).toEqual([]);
    const origins = await Replica.origins(app.database, CHAIN_PUBLICATION, [Scope.universe.id]);
    expect(origins.get(Scope.universe.id)?.position).toEqual(revoked);

    // refuse writing the account's access in the app's replica
    expect(
        await refusal(
            Authorization.run(
                notes,
                app.database,
                () => alice,
                app.environment,
                (authorization) =>
                    Relationship.grant(
                        {
                            object,
                            relation: "member",
                            subject: principal.user.reference("universe", "dave"),
                        },
                        authorization,
                    ),
            ),
        ),
    ).toEqual([
        "FORBIDDEN",
        "ACCESS_NOT_LOCAL",
        "cannot write the access of account account-1 here: this database has a replica of universe",
    ]);

    // read the app's relationships, about its notes, through a fresh snapshot of the space's replica
    await replicate(app.database, chainOf(notes, "personal"), spaceFeed, "snapshot");
    const local = await app.database
        .select({ type: accessRelationship.objectType, objectId: accessRelationship.objectId })
        .from(accessRelationship)
        .orderBy(asc(accessRelationship.objectType));
    expect(local.filter((row) => row.type === node.name)).toEqual([
        { type: "node", objectId: "b" },
    ]);
});

test("write a space's own access beside its row where its creator stores it, and replicate it to the home born below it", async () => {
    // create an account and its space of alice in the universe, which stores the space's row and owners
    const global = await openDatabase();
    const home = await openDatabase();
    onTestFinished(async () => {
        await Promise.all([global.close(), home.close()]);
    });
    const accounts = new Authorizer(policies, homeMappings, { partition: Scope.universe.id });
    const spaces = new Authorizer(policies, mappings, { partition: "personal" });
    const owner = <Value>(call: (authorization: Authorization) => Promise<Value>) =>
        Authorization.run(accounts, global.database, () => alice, home.environment, call);
    await global.database.insert(accountTable).values({ id: "account-1", scope: "universe" });
    await owner((authorization) =>
        authorization.create(account.reference("universe", "account-1"), {
            owner: alice.subject,
        }),
    );
    await global.database.insert(spaceTable).values({ id: "personal", account: "account-1" });
    const personal = space.reference("account-1", "personal");
    await owner((authorization) => authorization.create(personal, { owner: alice.subject }));

    // replicate the home's chain, store its row, and receive it
    const feed = new Feed(global.database, [...accessTables, policyTable], {});
    await replicate(home.database, chainOf(spaces, "account-1", "personal"), feed);
    await home.database.insert(spaceTable).values({ id: "personal", account: "account-1" });
    await Authorization.run(
        spaces,
        home.database,
        () => alice,
        home.environment,
        (authorization) => authorization.receive(personal, ["account-1"]),
    );

    // bind alice to the owner role in the universe, which the home replicates
    expect({
        home: await bound(home.database),
        universe: await bound(global.database),
        isHome: await spaces.isHome(Snapshot.live(home.database), "personal"),
    }).toEqual({
        home: [[Role.OWNER, "alice"]],
        universe: [[Role.OWNER, "alice"]],
        isHome: true,
    });

    // grant on the space in the universe, and refuse it at its home
    const grant = (authorizer: Authorizer, database: DatabaseConnection) =>
        Authorization.run(
            authorizer,
            database,
            () => alice,
            home.environment,
            (authorization) =>
                Relationship.grant(
                    {
                        object: personal,
                        role: Role.OWNER,
                        subject: principal.user.reference("universe", "dave"),
                    },
                    authorization,
                ),
        );
    await grant(accounts, global.database);
    expect(await refusal(grant(spaces, home.database))).toEqual([
        "FORBIDDEN",
        "ACCESS_NOT_LOCAL",
        "cannot write the access of space personal here: this database has a replica of account-1",
    ]);
});

/** Read the role and subject of each relationship on the personal space. */
async function bound(database: DatabaseConnection) {
    const rows = await database
        .select({ roleId: accessRelationship.roleId, subjectId: accessRelationship.subjectId })
        .from(accessRelationship)
        .where(eq(accessRelationship.objectId, "personal"));

    return rows.map((row) => [row.roleId, row.subjectId]);
}

/** Apply a source's feed to a replica until it arrives at the source's head. */
async function replicate(
    database: DatabaseConnection,
    chain: { readonly replica: Replica; readonly subscription: Subscription },
    feed: Feed,
    from: "position" | "snapshot" = "position",
): Promise<void> {
    // read the pages up to the source's head
    const { replica, subscription } = chain;
    await replica.register(database, { subscription, audience: "contained" });
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
    await Array.fromAsync(replica.apply(database, pages, {}));
}
