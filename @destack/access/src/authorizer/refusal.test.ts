import { expect, onTestFinished, test } from "@destack/test";
import { Snapshot } from "@destack/db";
import { nth } from "@destack/schema";
import { account, item, node, rows, space } from "../test/fixture.ts";
import { TestAccess } from "../test/access.ts";
import { openFixture, userSubject } from "../test/database.ts";

test("refuse a reader of a suspended space for its suspension, and a stranger as denied", async () => {
    // let bob read the nodes of a space and own one there, then suspend it
    const fixture = await openFixture();
    onTestFinished(() => fixture.close());
    const { database, authorizer, alice, bob, environment } = fixture;
    const access = new TestAccess(database, environment);
    await access.replicateScope(account.reference("universe", "near"), account);
    const suspended = space.reference("near", "suspended");
    await new TestAccess(database, environment).replicateScope(suspended, space);
    const reader = { name: "reader", description: "", permissions: [node.permission("read")] };
    await access.replicateRole(suspended, reader, userSubject("bob"), 1);
    await access.suspendReplica("suspended", 1);
    const stored = {
        ...nth(rows, 0),
        id: "in-suspended",
        scope: "suspended",
        parent: null,
        owner: "bob",
    };
    await database.transaction(
        async (transaction) => await transaction.insert(item).values([stored]),
    );

    // refuse each caller for the gate that stops them, none stopping the stranger
    const snapshot = Snapshot.live(database);
    const read = node.permission("read");
    const refusal = async (caller: typeof bob) => {
        const resolved = await authorizer.resolve(snapshot, "suspended", caller);
        const refused = await authorizer.refusal(snapshot, read, resolved, async (each) =>
            (await authorizer.checkRows(snapshot, read, each, [stored])).permitted.has(0),
        );

        return refused?.message;
    };
    expect([await refusal(bob), await refusal(alice)]).toEqual([
        "scope 'suspended' is forbidden: it is suspended",
        undefined,
    ]);
});
