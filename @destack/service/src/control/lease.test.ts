import { expect, onTestFinished, test } from "@destack/test";
import { TestDatabase } from "@destack/db/test";
import { present } from "@destack/schema";
import { controllerLease, Leases } from "./lease.ts";

/** The time the scenarios start at, in UTC epoch milliseconds. */
const START = 1_800_000_000_000;

test("take over a lapsed lease at the next epoch, refusing the old holder's renewal", async () => {
    const storage = await TestDatabase.create("sqlite", [controllerLease], { isMigrated: true });
    onTestFinished(() => storage.close());
    const acquire = (holder: string, duration: number, at: number) =>
        Leases.acquire(storage.database, "job", "shared", holder, duration, START + at);

    // take, refuse and renew a lease
    const taken = await acquire("first", 20, 0);
    const refused = await acquire("second", 20, 0);
    const renewed = await acquire("first", 20, 0);

    // take over a lapsed lease
    const takenOver = await acquire("second", 1000, 40);
    const late = await acquire("first", 1000, 40);
    expect([taken, "lapsesAt" in refused, renewed, takenOver, "lapsesAt" in late]).toEqual([
        { epoch: 1 },
        true,
        { epoch: 1 },
        { epoch: 2 },
        true,
    ]);
});

test("keep a lease of ours taken within the last third of its duration without writing, and renew it once that third passed", async () => {
    const storage = await TestDatabase.create("sqlite", [controllerLease], { isMigrated: true });
    onTestFinished(() => storage.close());
    const acquire = (at: number) =>
        Leases.acquire(storage.database, "job", "shared", "first", 300, START + at);
    const lapse = async () => {
        const [lease] = await storage.database
            .select({ at: controllerLease.expiresAt })
            .from(controllerLease);

        return present(lease, "the lease").at;
    };

    // take the lease and take it again at once
    await acquire(0);
    const taken = await lapse();
    await acquire(0);
    const kept = await lapse();

    // take it again once a third of its duration passed
    await acquire(120);
    const renewed = await lapse();
    expect([kept === taken, renewed > taken]).toEqual([true, true]);
});
