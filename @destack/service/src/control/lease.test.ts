import { expect, onTestFinished, test } from "@destack/test";
import { TestDatabase } from "@destack/db/test";
import { controllerLease, Leases } from "./lease.ts";

test("take over a lapsed lease at the next epoch, refusing the old holder's renewal", async () => {
    const storage = await TestDatabase.create("sqlite", [controllerLease], { isMigrated: true });
    onTestFinished(() => storage.close());
    const acquire = (holder: string, duration: number) =>
        Leases.acquire(storage.database, "job", "shared", holder, duration);

    // take, refuse and renew a lease
    const taken = await acquire("first", 20);
    const refused = await acquire("second", 20);
    const renewed = await acquire("first", 20);

    // take over a lapsed lease
    await new Promise((resolve) => {
        setTimeout(resolve, 40);
    });
    const takenOver = await acquire("second", 1000);
    const late = await acquire("first", 1000);
    expect([taken, "lapsesAt" in refused, renewed, takenOver, "lapsesAt" in late]).toEqual([
        { epoch: 1 },
        true,
        { epoch: 1 },
        { epoch: 2 },
        true,
    ]);
});
