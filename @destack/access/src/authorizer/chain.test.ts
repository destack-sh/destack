import { expect, onTestFinished, test } from "@destack/test";
import { asc, type DatabaseConnection } from "@destack/db";
import { Feed, Replica, Scope } from "@destack/sync";
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
    // insert an account and its space, both owned by alice, in the home database
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
    const requests = () => app.authorizer.chain(app.database, "personal", { isHome: false });
    const scopes = async () => (await requests()).map((request) => request.scope);
    const follow = async (scope: string) => {
        const request = (await requests()).find((entry) => entry.scope === scope);
        if (request === undefined) {
            throw new Error(`no chain request for ${scope}`);
        }
        following.push(
            app.authorizer.chainShape
                .replica(request)
                .follow(
                    app.database,
                    ({ after }, signal) =>
                        feed.subscribe(
                            authorizer.chainShape.replica(request).queries,
                            after,
                            signal,
                        ),
                    controller.signal,
                    { subscription: request },
                ),
        );
    };
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
    expect(await scopes()).toEqual(["personal"]);
    await follow("personal");
    await reach("personal");
    expect(await scopes()).toEqual(["personal", "account-1", "universe"]);

    // copy the account's access row for row, and only the policies the account and the universe hand down
    await follow("account-1");
    await follow("universe");
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

    // leave the access rows of local types and of universe-living types out of every copy
    const types = [...app.authorizer.local, ...authorizer.universal];
    expect(
        authorizer.chainShape
            .replica(
                authorizer.chainShape.subscription({
                    name: "chain",
                    scope: "universe",
                    below: "personal",
                    parameters: { local: [...app.authorizer.local], copied: [] },
                }),
            )
            .where.get(accessRelationship),
    ).toEqual({
        NOT: { OR: types.map((type) => ({ packageId: type.packageId, type: type.type })) },
    });
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
