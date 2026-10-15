import { STACKS } from "@destack/account/test";
import { Caller, principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { account, space } from "@destack/account/object";
import { Transaction } from "@destack/db";
import * as sqlite from "@destack/db/bun";
import { ObjectStore } from "@destack/object/server";
import { Clock } from "@destack/schema";
import { Scope } from "@destack/sync";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { ids, spaceDatabase } from "../src/test/index.ts";
import { serversOf } from "@destack/object/test";

test("decide an owner's permissions on a space in its own store through the predefined owner role, for its person and the space itself", async () => {
    // keep the space's row and chain in its database, owned by its person and by itself
    const database = await sqlite.connectBunSqlite(":memory:", spaceDatabase);
    onTestFinished(() => database.close());
    await database.migrate(spaceDatabase.tables);
    const access = new AccessFixture(database);
    await access.replicateScope(account.reference(Scope.universe.id, ids.account));
    const spaceObject = space.reference(ids.account, ids.space);
    await access.replicateScope(spaceObject, "own");
    await database.insert(space.table).values({
        id: ids.space,
        scope: ids.account,
        name: "personal",
        stack: STACKS.space,
        createdAt: 0,
        updatedAt: 0,
    });
    const owner = principal.user.reference(Scope.universe.id, ids.owner);
    const spacePrincipal = principal.space.reference(Scope.universe.id, ids.space);
    await access.bindOwner(spaceObject, owner);
    await access.bindOwner(spaceObject, spacePrincipal);

    // serve the space's store as the space
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal,
        servers: serversOf([]),
        policies: [space],
        database,
        origin: { package: space.package },
    });
    const decide = async (subject: typeof owner, name: "update" | "replicate") => {
        const caller = Caller.of(subject, Date.now());
        const authorization = await store.authorize(
            Transaction.reading(database),
            ids.space,
            caller,
        );

        return refusal(authorization.require(space.permission(name), spaceObject));
    };

    // grant both owners an owner's permission, and refuse them the reserved one and a stranger both
    const stranger = principal.user.reference(Scope.universe.id, "user-stranger");
    expect({
        person: await decide(owner, "update"),
        space: await decide(spacePrincipal, "update"),
        reserved: await decide(owner, "replicate"),
        stranger: await decide(stranger, "update"),
    }).toEqual({
        person: "done",
        space: "done",
        reserved: ["FORBIDDEN", "permission denied: replicate"],
        stranger: ["FORBIDDEN", "permission denied: update"],
    });
});
