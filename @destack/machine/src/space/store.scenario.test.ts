import { Shape } from "@destack/object";
import { principal } from "@destack/access";
import { Snapshot } from "@destack/db";
import { spaceShape } from "@destack/account/object";
import { present } from "@destack/schema";
import { Scope } from "@destack/sync";
import { expect, refusal, test } from "@destack/test";
import stackPackage from "@destack/stack/package";
import { ids, readStack, TestSpace } from "../test/index.ts";

test("follow a space's own rows as its machine serves it", async () => {
    // serve a space its universe placed on the machine
    const fixture = await TestSpace.open();
    const store = await fixture.store();
    const machineId = (await fixture.space).machine.id;

    // list the replicas of the space's own shape in its database
    const expected = Shape.subscription(
        Scope.universe.id,
        [{ shape: spaceShape, keys: [{ space: ids.space, machine: machineId }] }],
        { target: ids.space, subject: principal.machine.reference(Scope.universe.id, machineId) },
    );
    const listed = await present(store.subscriber, "the space's subscriber").subscriptions(
        Snapshot.live(store.database),
    );
    const own = listed
        .map((entry) => entry.subscription)
        .filter((each) => each.name === expected.name);

    // follow the space's row, placement, assignments and serving machine at the universe
    expect(own).toEqual([expected]);
});

test("serve a person calling the space with the token their client exchanges for it, and refuse a stranger's", async () => {
    // list the space's installations as its owner, and as a person with no role in it
    const fixture = await TestSpace.open();
    await fixture.serve();
    const stranger = await fixture.universe.signIn("stranger@example.test");
    const listing = (caller: ReturnType<TestSpace["person"]>) =>
        caller.client(stackPackage).installation.list({ scope: fixture.id });
    const owned = await listing(fixture.person(fixture.owner));
    const refused = await refusal(listing(fixture.person(stranger)));

    // list the space's stack to its owner, and conceal the space from the stranger
    const stack = await readStack(fixture.database);
    expect([owned.items.map((item) => item.id), refused]).toEqual([
        [stack.id],
        ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope '${fixture.id}' not found`],
    ]);
});
