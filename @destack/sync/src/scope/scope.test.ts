import { expect, onTestFinished, refusal, test } from "@destack/test";
import { TestDatabase } from "@destack/db/test";
import { eq, Snapshot, Transaction, Table } from "@destack/db";
import { PackageId } from "@destack/package";
import { open } from "../test/fixture.ts";
import { Scope } from "./scope.ts";

/** The package declaring the scope types of the scenarios. */
const PACKAGE_ID = PackageId.parse("package-01996ab0-0000-7000-8000-000000000009");

test("read chains up to the universe, refuse unknown scopes, and fence a scope for a transfer", async () => {
    const database = await open([Scope.table]);
    await database
        .insert(Scope.table)
        .values([
            folder("account", Scope.universe.id, []),
            folder("space", "account", ["account"]),
            folder("orphan", "missing", ["missing"]),
        ]);
    const chain = async (scope: string) =>
        (await Scope.chain(Snapshot.live(database), scope)).map((link) => link.object.id);

    // end rooted chains at the universe, leave a chain with a missing parent unrooted, and read nothing of an unknown scope
    expect(await Promise.all(["space", Scope.universe.id, "orphan", "unknown"].map(chain))).toEqual(
        [["space", "account", "universe"], ["universe"], ["orphan"], []],
    );

    // read a scope's object and refuse an unknown one
    expect(await Scope.object(Snapshot.live(database), "space")).toEqual({
        packageId: PACKAGE_ID,
        type: "folder",
        scope: "account",
        id: "space",
    });
    expect(await refusal(Scope.object(Snapshot.live(database), "unknown"))).toEqual([
        "NOT_FOUND",
        "NOT_FOUND",
        "unknown scope: unknown",
    ]);

    // fence a scope for its target machine and lift the fence
    await Transaction.run(database, (transaction) =>
        Scope.fence(transaction, "space", "machine-b", 1000),
    );
    const moved = async () =>
        (await Scope.chain(Snapshot.live(database), "space")).map((link) => link.movedTo);
    expect(await moved()).toEqual(["machine-b", undefined, undefined]);
    await Transaction.run(database, (transaction) => Scope.unfence(transaction, "space"));
    expect(await moved()).toEqual([undefined, undefined, undefined]);
});

test("read a scope and every scope below it, nearest first, leaving its siblings and parent out", async () => {
    const database = await open([Scope.table]);
    await database
        .insert(Scope.table)
        .values([
            folder("account", Scope.universe.id, []),
            folder("space", "account", ["account"]),
            folder("sibling", "account", ["account"]),
            folder("room", "space", ["space", "account"]),
            folder("hall", "space", ["space", "account"]),
            folder("desk", "room", ["room", "space", "account"]),
        ]);

    // read the space's subtree and a leaf's
    expect([
        await Scope.subtree(Snapshot.live(database), "space"),
        await Scope.subtree(Snapshot.live(database), "desk"),
    ]).toEqual([["space", "hall", "room", "desk"], ["desk"]]);
});

test("lie within itself, a scope above it and the universe, apart from a sibling and a scope below", () => {
    // read enclosure from the space's row alone
    const space = folder("space", "account", ["account"]);
    expect([
        Scope.within(space, "space"),
        Scope.within(space, "account"),
        Scope.within(space, Scope.universe.id),
        Scope.within(space, "sibling"),
        Scope.within(folder("account", Scope.universe.id, []), "space"),
    ]).toEqual([true, true, true, false, false]);
});

test("read several scopes' chains at once, each as it reads alone", async () => {
    const database = await open([Scope.table]);
    await database
        .insert(Scope.table)
        .values([
            folder("account", Scope.universe.id, []),
            folder("space", "account", ["account"]),
            folder("other", "account", ["account"]),
            folder("orphan", "missing", ["missing"]),
        ]);

    // read the chains of scopes sharing an ancestor, an unrooted one and an unknown one
    const chains = await Scope.chains(Snapshot.live(database), [
        "space",
        "other",
        "orphan",
        "unknown",
    ]);
    expect(
        [...chains].map(([scope, links]) => [scope, links.map((link) => link.object.id)]),
    ).toEqual([
        ["space", ["space", "account", "universe"]],
        ["other", ["other", "account", "universe"]],
        ["orphan", ["orphan"]],
        ["unknown", []],
    ]);
});

test("fail the commit of a transaction that read a chain before a fence, and show it the fence when it runs again", async () => {
    const storage = await TestDatabase.create([Scope.table], { isMigrated: true });
    onTestFinished(() => storage.close());
    await storage.database
        .insert(Scope.table)
        .values([
            folder("account", Scope.universe.id, []),
            folder("space", "account", ["account"]),
        ]);

    // write into the space, fencing the account after the first attempt read the chain
    let attempts = 0;
    const written = await Transaction.run(storage.database, async (transaction) => {
        attempts += 1;
        const chain = await Scope.chain(transaction.view(), "space");
        if (attempts === 1) {
            await Transaction.run(storage.database, (fencing) =>
                Scope.fence(fencing, "account", "machine-b", 1000),
            );
        }
        const note = Table.complete(Scope.table, folder("note", "space", ["space", "account"]));
        await transaction.write({ table: Scope.table, before: null, after: note });

        return chain.map((link) => link.movedTo);
    });

    // commit the second attempt, which reads the fence
    const notes = await storage.database
        .select({ scope: Scope.table.scope })
        .from(Scope.table)
        .where(eq(Scope.table.scope, "note"));
    expect([attempts, written, notes]).toEqual([
        2,
        [undefined, "machine-b", undefined],
        [{ scope: "note" }],
    ]);
});

/** Describe a folder scope below a parent, with its ancestors nearest first. */
function folder(scope: string, parent: string, ancestors: string[]) {
    return {
        scope,
        parent,
        partition: Scope.universe.id,
        ancestors,
        packageId: PACKAGE_ID,
        type: "folder",
    };
}
