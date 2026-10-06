import { expect, onTestFinished, test } from "@destack/test";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { eq, Snapshot, sql } from "@destack/db";
import { schema } from "@destack/schema";
import { PackageId } from "@destack/package";
import { open } from "../test/fixture.ts";
import { Scope } from "./scope.ts";

/** The package declaring the scope types of the scenarios. */
const PACKAGE_ID = PackageId.parse("package-01996ab0-0000-7000-8000-000000000009");

test.for(TEST_DIALECTS)(
    "read chains up to the universe, refuse unknown scopes, and fence a scope for a transfer on %s",
    async (dialect) => {
        const database = await open(dialect, [Scope.table]);
        const row = (scope: string, parent: string, ancestors: string[]) => ({
            scope,
            parent,
            ancestors,
            packageId: PACKAGE_ID,
            type: "folder",
        });
        await database
            .insert(Scope.table)
            .values([
                row("account", Scope.universe.id, []),
                row("space", "account", ["account"]),
                row("orphan", "missing", ["missing"]),
            ]);
        const chain = async (scope: string) =>
            (await Scope.chain(Snapshot.live(database), scope)).map((link) => link.object.id);

        // end rooted chains at the universe, leave a chain with a missing parent unrooted, and read nothing of an unknown scope
        expect(
            await Promise.all(["space", Scope.universe.id, "orphan", "unknown"].map(chain)),
        ).toEqual([["space", "account", "universe"], ["universe"], ["orphan"], []]);

        // read a scope's own object and refuse an unknown one
        expect(await Scope.object(Snapshot.live(database), "space")).toEqual({
            packageId: PACKAGE_ID,
            type: "folder",
            scope: "account",
            id: "space",
        });
        await expect(Scope.object(Snapshot.live(database), "unknown")).rejects.toMatchObject({
            code: "NOT_FOUND",
        });

        // fence a scope for its target cell and lift the fence
        await Scope.fence(database, "space", "cell-b", 1000);
        const moved = async () =>
            (await Scope.chain(Snapshot.live(database), "space")).map((link) => link.movedTo);
        expect(await moved()).toEqual(["cell-b", undefined, undefined]);
        await Scope.unfence(database, "space");
        expect(await moved()).toEqual([undefined, undefined, undefined]);
    },
);

test.for(TEST_DIALECTS)(
    "read several scopes' chains at once, each as it reads alone, on %s",
    async (dialect) => {
        const database = await open(dialect, [Scope.table]);
        const row = (scope: string, parent: string, ancestors: string[]) => ({
            scope,
            parent,
            ancestors,
            packageId: PACKAGE_ID,
            type: "folder",
        });
        await database
            .insert(Scope.table)
            .values([
                row("account", Scope.universe.id, []),
                row("space", "account", ["account"]),
                row("other", "account", ["account"]),
                row("orphan", "missing", ["missing"]),
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
    },
);

test.skipIf(!TEST_DIALECTS.includes("postgresql"))(
    "run a write waiting behind a fence again once the fence commits, showing it the fence on postgresql",
    async () => {
        const storage = await TestDatabase.create("postgresql", [Scope.table], {
            isMigrated: true,
        });
        const other = await storage.connect([Scope.table]);
        onTestFinished(async () => {
            await other.close();
            await storage.close();
        });
        const row = (scope: string, parent: string, ancestors: string[]) => ({
            scope,
            parent,
            ancestors,
            packageId: PACKAGE_ID,
            type: "folder",
        });
        await storage.database
            .insert(Scope.table)
            .values([row("account", Scope.universe.id, []), row("space", "account", ["account"])]);

        // hold a fence of the account open on another connection
        const fenced = Promise.withResolvers<void>();
        const release = Promise.withResolvers<void>();
        const fencing = other.transaction(async (transaction) => {
            await Scope.fence(transaction, "account", "cell-b", 1000);
            fenced.resolve();
            await release.promise;
        });
        await fenced.promise;

        // write into the space once its guard returns, waiting until the guard queues behind the fence
        const write = async () =>
            await storage.database.transaction(async (transaction) => {
                const chain = await Scope.guard(transaction, "space");
                await transaction.insert(Scope.table).values(row("note", "space", ["space"]));

                return chain.map((link) => link.movedTo);
            });
        const writing = write().catch((error: unknown) => error);
        await expect
            .poll(async () => {
                const [waiting] = await other.execute(
                    sql`SELECT count(*)::int AS count FROM pg_locks WHERE NOT granted`,
                    schema.object({ count: schema.number() }),
                );

                return waiting?.count ?? 0;
            })
            .toBeGreaterThan(0);
        release.resolve();
        await fencing;

        // run the waiting write again, its guard showing the fence
        const written = await writing;
        const notes = await storage.database
            .select({ scope: Scope.table.scope })
            .from(Scope.table)
            .where(eq(Scope.table.scope, "note"));
        expect([written, notes]).toEqual([[undefined, "cell-b", undefined], [{ scope: "note" }]]);
    },
);
