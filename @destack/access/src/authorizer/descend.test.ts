import { expect, onTestFinished, test } from "@destack/test";
import { TEST_DIALECTS } from "@destack/db/test";
import { Snapshot } from "@destack/db/log";
import type { Access } from "./access.ts";
import { account, item, node, rows, space } from "../test/fixture.ts";
import { copyRole, copyScope, suspendCopy } from "../test/copy.ts";
import { openFixture, userSubject } from "../test/database.ts";

/** Describe a caller's resolved access in a scope. */
function describe(access: Access | undefined) {
    return (
        access && {
            scope: access.scope,
            scopes: access.scopes,
            authorities: access.authorities,
            grants: [...access.grants],
            readers: access.granting(node.permission("read")),
            isSuspended: access.isSuspended,
            moved: access.moved,
            until: access.until,
        }
    );
}

test.for(TEST_DIALECTS)(
    "resolve a caller in the scopes one encloses exactly as each scope resolves alone, on %s",
    async (dialect) => {
        // place three spaces below an account and one below another
        const fixture = await openFixture(dialect);
        onTestFinished(() => fixture.close());
        const { database, authorizer, bob } = fixture;
        await copyScope(database, account.reference("universe", "near"));
        await copyScope(database, account.reference("universe", "far"));
        const spaces = ["plain", "shared", "suspended"].map((id) => space.reference("near", id));
        for (const scope of [...spaces, space.reference("far", "outside")]) {
            await copyScope(database, scope);
        }

        // let bob read the nodes of two spaces through a role, and suspend one of them
        const reader = { name: "reader", description: "", permissions: [node.permission("read")] };
        await copyRole(database, spaces[1]!, reader, userSubject("bob"), 1);
        await copyRole(database, spaces[2]!, reader, userSubject("bob"), 1);
        await suspendCopy(database, "suspended", 1);
        const nodes = ["plain", "shared", "suspended"].map((scope) => ({
            ...rows[0]!,
            id: `in-${scope}`,
            scope,
            parent: null,
        }));
        await database.insert(item).values(nodes);

        // resolve the account's spaces at once, leaving out the space of another account and an unknown scope
        const snapshot = Snapshot.live(database);
        const above = await authorizer.resolve(snapshot, "near", bob);
        const below = await above.descend(snapshot, [
            "plain",
            "shared",
            "suspended",
            "outside",
            "unknown",
        ]);
        expect([...below.keys()]).toEqual(["plain", "shared", "suspended"]);
        expect([...below.values()].map(describe)).toEqual(
            await Promise.all(
                [...below.keys()].map(async (scope) =>
                    describe(await authorizer.resolve(snapshot, scope, bob)),
                ),
            ),
        );

        // decide the spaces' nodes in one read, each by its own space's roles and suspension
        const read = node.permission("read");
        const alone = await authorizer.checkRows(snapshot, read, above, nodes);
        const together = await authorizer.checkRows(snapshot, read, above, nodes, undefined, below);
        expect([[...alone.held], [...together.held]]).toEqual([[], [1]]);
    },
);
