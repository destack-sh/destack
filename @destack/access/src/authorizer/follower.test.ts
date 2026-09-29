import { expect, onTestFinished, test } from "@destack/test";
import { asc, type DatabaseConnection } from "@destack/db";
import { Condition } from "@destack/db/query";
import { Feed, Replica, Scope, type ChainRelay } from "@destack/sync";
import {
    accessRelationship,
    accessRole,
    accessRolePermission,
    accessTables,
    Authorization,
    Authorizer,
} from "../index.ts";
import {
    account,
    accountTable,
    homeMappings,
    node,
    policies,
    policyTable,
    space,
    spaceTable,
} from "../test/fixture.ts";
import { openFixture, userSubject } from "../test/database.ts";

test("copy a space's chain up to the universe from a relay, with the policies each scope hands down, and follow a grant", async () => {
    // hold an account and its space, both owned by alice, in the home database
    const home = await openFixture();
    const app = await openFixture();
    onTestFinished(async () => {
        await Promise.all([home.close(), app.close()]);
    });
    const authorizer = new Authorizer(policies, homeMappings);
    const owner = new Authorization(authorizer, home.database, () => ({
        ...home.alice,
        now: Date.now(),
    }));
    await home.database.insert(accountTable).values({ id: "account-1", scope: "universe" });
    await owner.create(account.reference("universe", "account-1"), { owner: userSubject("alice") });
    await home.database.insert(spaceTable).values({ id: "personal", account: "account-1" });
    await owner.create(space.reference("account-1", "personal"), { owner: userSubject("alice") });

    // require a value everywhere and of the account's spaces, and set one for the account alone
    await home.database.insert(policyTable).values([
        { id: "everywhere", scope: "universe", mode: "require", value: "on" },
        { id: "required", scope: "account-1", mode: "require", value: "on" },
        { id: "own", scope: "account-1", mode: "set", value: "off" },
    ]);

    // relay the space's chain from the home database
    const feed = new Feed(home.database, [...accessTables, policyTable]);
    const relay: ChainRelay = {
        scope: "personal",
        watch: (request, signal) =>
            feed.subscribe(
                authorizer.replicaOf(request.scope, request.held, request.copied).queries,
                request.after,
                signal,
            ),
    };
    const follower = app.authorizer.follower(app.database, app.authorizer.held, relay);
    const controller = new AbortController();
    const following: Promise<void>[] = [];
    onTestFinished(async () => {
        controller.abort();
        await Promise.allSettled(following);
    });
    const reach = async (scope: string) => {
        const head = await home.database.log.position();
        await Replica.reach(app.database, scope, head, controller.signal);
    };

    // list the space alone until its copy lists the account containing it, up to the universe
    expect(await follower.list()).toEqual(["personal"]);
    following.push(follower.follow("personal", controller.signal));
    await reach("personal");
    expect(await follower.list()).toEqual(["personal", "account-1", "universe"]);

    // copy the account's access row for row, and only the policies the account and the universe hand down
    following.push(follower.follow("account-1", controller.signal));
    following.push(follower.follow("universe", controller.signal));
    await reach("account-1");
    await reach("universe");
    expect([
        await copied(app.database),
        await app.database.select().from(policyTable).orderBy(asc(policyTable.id)),
    ]).toEqual([
        await copied(home.database),
        [
            { id: "everywhere", scope: "universe", mode: "require", value: "on" },
            { id: "required", scope: "account-1", mode: "require", value: "on" },
        ],
    ]);

    // leave the access rows of held types and of universe-living types out of every copy
    const types = [...app.authorizer.held, ...authorizer.universal];
    expect(
        authorizer.replicaOf("universe", app.authorizer.held, []).where.get(accessRelationship),
    ).toEqual(
        Condition.not(
            Condition.any(
                ...types.map((type) =>
                    Condition.all(
                        Condition.eq("packageId", type.packageId),
                        Condition.eq("type", type.type),
                    ),
                ),
            ),
        ),
    );
    expect(authorizer.universal.map((type) => type.type)).toEqual(["region", "user"]);

    // follow a grant on the account into the copy
    await owner.grant({
        object: account.reference("universe", "account-1"),
        relation: "member",
        subject: userSubject("carol"),
    });
    await reach("account-1");
    expect(await copied(app.database)).toEqual(await copied(home.database));
});

/** Read the decision rows about the account and the space, skipping the nodes' own relationships. */
async function copied(database: DatabaseConnection) {
    // read each decision table in key order
    const relationships = await database
        .select()
        .from(accessRelationship)
        .orderBy(asc(accessRelationship.id));

    return {
        scopes: await database.select().from(Scope.table).orderBy(asc(Scope.table.scope)),
        roles: await database.select().from(accessRole).orderBy(asc(accessRole.id)),
        permissions: await database
            .select()
            .from(accessRolePermission)
            .orderBy(asc(accessRolePermission.id)),
        relationships: relationships.filter((row) => row.type !== node.name),
    };
}
