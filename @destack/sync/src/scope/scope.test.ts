import { expect, test } from "@destack/test";
import { TEST_DIALECTS } from "@destack/db/test";
import { Snapshot } from "@destack/db";
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

        // fence a scope for its target cell, then lift the fence
        await Scope.fence(database, "space", "cell-b", 1000);
        const moved = async () =>
            (await Scope.chain(Snapshot.live(database), "space")).map((link) => link.movedTo);
        expect(await moved()).toEqual(["cell-b", undefined, undefined]);
        await Scope.unfence(database, "space");
        expect(await moved()).toEqual([undefined, undefined, undefined]);
    },
);
