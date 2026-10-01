import { expect, test } from "@destack/test";
import { account, user, zone } from "@destack/account/object";
import type { ObjectType } from "@destack/object";
import { Scope } from "@destack/sync";
import { Condition } from "@destack/db/query";
import { serveObjects } from "../src/server/index.ts";
import { ids, openSpace, spaceOptions } from "./fixture/space.ts";

test("copy every global row a space may read from the universe, beside the chain above it", async () => {
    // keep the owner's space
    const database = await openSpace();
    const server = serveObjects(await spaceOptions(database));

    // request the space's chain copies and one copy of the users, their accounts and the zones it may read
    const reference = (object: ObjectType) => ({
        packageId: object.policy.definition.packageId,
        type: object.policy.definition.name,
    });
    const requests = await server.source.replicaRequests(ids.space, { isHome: true });
    expect(requests.map((request) => [request.name, request.scope, request.rows])).toEqual([
        ["chain", ids.account, []],
        ["chain", Scope.universe.id, []],
        [
            ids.space,
            Scope.universe.id,
            [
                { type: reference(user), where: Condition.all() },
                { type: reference(zone), where: Condition.all() },
                { type: reference(account), where: Condition.all(), within: [reference(user)] },
            ],
        ],
    ]);
});
