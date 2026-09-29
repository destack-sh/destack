import { expect, onTestFinished, test } from "@destack/test";
import { asc, type DatabaseConnection } from "@destack/db";
import { Feed, Replica } from "@destack/sync";
import {
    AccessFollower,
    accessRelationship,
    accessRole,
    accessRolePermission,
    accessScope,
    ACCESS_TABLES,
    Authorization,
    Authorizer,
    type AccessRelay,
} from "../index.ts";
import {
    account,
    accountTable,
    homeMappings,
    node,
    policies,
    space,
    spaceTable,
} from "../test/fixture.ts";
import { openFixture, userSubject } from "../test/database.ts";

test("copy a space's access and its account's from a relay, and follow a grant", async () => {
    // hold an account and its space, both owned by alice, in the home database
    const home = await openFixture();
    const app = await openFixture();
    onTestFinished(async () => {
        await Promise.all([home.close(), app.close()]);
    });
    const owner = new Authorization(new Authorizer(policies, homeMappings), home.database, () => ({
        ...home.alice,
        now: Date.now(),
    }));
    await home.database.insert(accountTable).values({ id: "account-1", scope: "global" });
    await owner.create(account.reference("global", "account-1"), { owner: userSubject("alice") });
    await home.database.insert(spaceTable).values({ id: "personal", account: "account-1" });
    await owner.create(space.reference("account-1", "personal"), { owner: userSubject("alice") });

    // relay the space's chain from the home database
    const feed = new Feed(home.database, ACCESS_TABLES);
    const relay: AccessRelay = {
        scope: "personal",
        watch: (request, signal) =>
            feed.subscribe(
                Authorizer.replicaOf(request.scope, request.held).queries,
                request.after,
                signal,
            ),
    };
    const follower = new AccessFollower(app.database, app.authorizer.held, relay);
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

    // list the space alone until its copy names the account containing it
    expect(await follower.list()).toEqual(["personal"]);
    following.push(follower.follow("personal", controller.signal));
    await reach("personal");
    expect(await follower.list()).toEqual(["personal", "account-1"]);

    // copy the account's access as well, row for row as the home database holds it
    following.push(follower.follow("account-1", controller.signal));
    await reach("account-1");
    expect(await copied(app.database)).toEqual(await copied(home.database));

    // follow a grant on the account into the copy
    await owner.grant({
        object: account.reference("global", "account-1"),
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
        scopes: await database.select().from(accessScope).orderBy(asc(accessScope.scope)),
        roles: await database.select().from(accessRole).orderBy(asc(accessRole.id)),
        permissions: await database
            .select()
            .from(accessRolePermission)
            .orderBy(asc(accessRolePermission.id)),
        relationships: relationships.filter((row) => row.type !== node.name),
    };
}
