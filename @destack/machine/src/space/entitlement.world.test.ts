import { space, type SpaceResource } from "@destack/account/object";
import { eq, Snapshot } from "@destack/db";
import { Scope } from "@destack/sync";
import { expect, test } from "@destack/test";
import { ids, TestSpace } from "../test/index.ts";

/** Serve the fixture's space on a machine that enforces its account's entitlements. */
async function open() {
    const fixture = await TestSpace.open({ isEnforcingEntitlements: true });
    await fixture.serve();

    // read the space's conditions once its replicas of the account's entitlements settle
    const control = async () => {
        await fixture.simulation.idle();
        const [row] = await fixture.database
            .select({ conditions: space.table.conditions })
            .from(space.table)
            .where(eq(space.table.id, ids.space));

        return Object.fromEntries(
            Object.entries(row?.conditions ?? {})
                .filter(([name]) => name !== "placement")
                .map(([name, condition]) => [name, [condition.status, condition.reason]]),
        );
    };

    // set the account within its plan until a test changes an entitlement, as its residency's home does
    const entitlement = (resource: SpaceResource, state: "within" | "blocked") =>
        fixture.universe.entitle(ids.account, resource, { kind: "metered", limit: 1000, state });
    for (const resource of ["database", "bucket", "compute", "network"] as const) {
        await entitlement(resource, "within");
    }

    return { fixture, entitlement, control };
}

test("record a space's conditions from its account's entitlements, each exceeded resource blocked until its plan allows it again", async () => {
    // block the database, then compute too, then bring both within the plan
    const { entitlement, control } = await open();
    await entitlement("database", "blocked");
    const database = await control();
    await entitlement("compute", "blocked");
    const compute = await control();
    await entitlement("database", "within");
    await entitlement("compute", "within");
    const within = await control();

    const allowed = ["true", "Within"];
    const blocked = ["false", "Blocked"];
    expect({ database, compute, within }).toEqual({
        database: { database: blocked, bucket: allowed, compute: allowed, network: allowed },
        compute: { database: blocked, bucket: allowed, compute: blocked, network: allowed },
        within: { database: allowed, bucket: allowed, compute: allowed, network: allowed },
    });
});

test("cap writes within a space while its database entitlement is exceeded, and lift the cap once it is within", async () => {
    // read whether the space's scope row caps its writes after the controller runs
    const { fixture, entitlement, control } = await open();
    const isCapped = async () => {
        await control();
        const [link] = await Scope.chain(Snapshot.live(fixture.database), ids.space);

        return link?.isCapped;
    };

    // block the database, look again, and lift it as a plan change does
    await entitlement("database", "blocked");
    const blocked = await isCapped();
    const again = await isCapped();
    await entitlement("database", "within");
    const lifted = await isCapped();

    expect({ blocked, again, lifted }).toEqual({ blocked: true, again: true, lifted: false });
});
