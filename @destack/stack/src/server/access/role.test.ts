import { STACKS } from "@destack/account/test";
import { testEvents, serversOf } from "@destack/object/test";
import { ids, spaceDatabase } from "../../test/index.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { accessRelationship, principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { account, space } from "@destack/account/object";
import * as sqlite from "@destack/db/bun";
import { ObjectStore, Stack } from "@destack/object/server";
import { testCallKey } from "@destack/runtime/test";
import { nth, schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { implementRole, implementRelationship } from "../index.ts";
import { Clock } from "@destack/environment";

/** The stack declaring the roles. */
const stackId = schema
    .identifier("installation")
    .parse("installation-01996ab0-0000-7000-8000-000000000003");

/** A person the stack relates first. */
const carol = "user-01996ab0-0000-7000-8000-000000000008";

/** A person the stack relates once it changes. */
const dave = "user-01996ab0-0000-7000-8000-000000000009";

test("keep a stack's declared relationship under its identifier until it changes, then replace it", async () => {
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

    // apply as the space, owning itself as its creation binds it
    const spacePrincipal = principal.space.reference(Scope.universe.id, ids.space);
    await access.bindOwner(spaceObject, spacePrincipal);
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal,
        servers: serversOf([implementRole(), implementRelationship()]),
        policies: [space],
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: space.package },
    });

    // declare a reader role bound to one user and rebind it to another
    const apply = (user: string, via: object = { role: "reader" }) =>
        Stack.apply({
            database,
            objects: [implementRole(), implementRelationship()],
            policies: [space.policy],
            manager: { installationId: stackId, packageId: ids.package },
            as: spacePrincipal,
            store,
            scope: ids.space,
            now: Date.now(),
            document: {
                roles: { reader: { description: "Reads installations", permissions: [] } },
                relationships: { readers: { ...via, subject: { id: user } } },
            },
        });
    const bound = async () =>
        (await database.select().from(accessRelationship))
            .filter((row) => row.managerInstallationId !== null)
            .map((row) => [row.id, row.subjectId]);
    await apply(carol);
    const first = await bound();
    await apply(carol);
    const kept = await bound();
    await apply(dave);
    const replaced = await bound();

    // refuse a relationship through both a relation and a role, or neither
    const refused = [
        await refusal(apply(dave, { role: "reader", relation: "read" })),
        await refusal(apply(dave, {})),
    ];

    // expect the same row while unchanged and a new one for the other user
    const [id, subject] = nth(replaced, 0);
    const message = "a grant has exactly one relation or role";
    expect([kept, replaced.length, subject, id === nth(nth(first, 0), 0), refused]).toEqual([
        first,
        1,
        dave,
        false,
        [
            ["INVALID_CALL", message],
            ["INVALID_CALL", message],
        ],
    ]);
});
