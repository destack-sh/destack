import { expect, onTestFinished, test } from "@destack/test";
import { TestDatabase } from "../test/database.ts";
import { lease, Lease } from "./lease.ts";
import { TEST_START, testSimulation } from "@destack/test/world";

/** Open a database with the lease table. */
async function open() {
    const storage = await TestDatabase.create([lease], {
        environment: testSimulation().environment("test"),
        isMigrated: true,
    });
    onTestFinished(() => storage.close());

    return storage.database;
}

test("take over a lease at the next epoch once it lapses, refusing the old holder until it lapses again", async () => {
    const database = await open();
    const acquire = (holder: string, duration: number, at: number) =>
        Lease.acquire(database, {
            name: "job",
            key: "shared",
            holder,
            duration,
            now: TEST_START + at,
        });

    // take, refuse and renew a lease
    const taken = await acquire("first", 20, 0);
    const refused = await acquire("second", 20, 0);
    const renewed = await acquire("first", 20, 0);

    // take over the lease the moment it lapses, refusing its old holder
    const takenOver = await acquire("second", 1000, 20);
    const late = await acquire("first", 1000, 20);
    expect([taken, refused, renewed, takenOver, late]).toEqual([
        { kind: "acquired", slot: 0, epoch: 1, expiresAt: TEST_START + 20 },
        { kind: "busy", lapsesAt: TEST_START + 20 },
        { kind: "acquired", slot: 0, epoch: 1, expiresAt: TEST_START + 20 },
        { kind: "acquired", slot: 0, epoch: 2, expiresAt: TEST_START + 1020 },
        { kind: "busy", lapsesAt: TEST_START + 1020 },
    ]);
});

test("reuse the holder's lease without writing while two thirds of it remain, and renew it once a third passed", async () => {
    const database = await open();
    const acquire = (at: number) =>
        Lease.acquire(database, {
            name: "job",
            key: "shared",
            holder: "first",
            duration: 300,
            now: TEST_START + at,
        });

    // take the lease, then again at once and once a third of it passed
    expect([await acquire(0), await acquire(90), await acquire(120)]).toEqual([
        { kind: "acquired", slot: 0, epoch: 1, expiresAt: TEST_START + 300 },
        { kind: "acquired", slot: 0, epoch: 1, expiresAt: TEST_START + 300 },
        { kind: "acquired", slot: 0, epoch: 1, expiresAt: TEST_START + 420 },
    ]);
});

test("lease a key to as many holders as it has slots, a released slot to the next holder", async () => {
    const database = await open();
    const acquire = (holder: string) =>
        Lease.acquire(database, {
            name: "review",
            key: "space-1",
            holder,
            duration: 100,
            now: TEST_START,
            slots: 2,
        });

    // fill both slots, refuse a third holder, then lease it the released slot
    const filled = [await acquire("a"), await acquire("b"), await acquire("c")];
    await Lease.release(database, { holder: "a", name: "review", key: "space-1" });
    const reacquired = await acquire("c");
    expect([...filled, reacquired]).toEqual([
        { kind: "acquired", slot: 0, epoch: 1, expiresAt: TEST_START + 100 },
        { kind: "acquired", slot: 1, epoch: 1, expiresAt: TEST_START + 100 },
        { kind: "busy", lapsesAt: TEST_START + 100 },
        { kind: "acquired", slot: 0, epoch: 1, expiresAt: TEST_START + 100 },
    ]);
});

test("release every key a holder has, leaving other holders'", async () => {
    const database = await open();
    const acquire = (holder: string, key: string) =>
        Lease.acquire(database, { name: "job", key, holder, duration: 100, now: TEST_START });

    // take two keys as one holder and one as another, then release the first holder's
    await acquire("first", "a");
    await acquire("first", "b");
    await acquire("second", "c");
    await Lease.release(database, { holder: "first" });
    const rows = await database.select({ key: lease.key, holder: lease.holder }).from(lease);
    expect(rows).toEqual([{ key: "c", holder: "second" }]);
});

test("read when a key's taken slots lapse, the latest of them, and none once they lapsed", async () => {
    const database = await open();
    const request = { name: "job", key: "shared" };

    // take two slots of a key, then read their lapse before, at and after it
    await Lease.acquire(database, {
        ...request,
        holder: "first",
        duration: 20,
        now: TEST_START,
        slots: 2,
    });
    await Lease.acquire(database, {
        ...request,
        holder: "second",
        duration: 50,
        now: TEST_START,
        slots: 2,
    });
    expect([
        await Lease.lapsesAt(database, { name: "job", key: "other", now: TEST_START }),
        await Lease.lapsesAt(database, { ...request, now: TEST_START }),
        await Lease.lapsesAt(database, { ...request, now: TEST_START + 20 }),
        await Lease.lapsesAt(database, { ...request, now: TEST_START + 50 }),
    ]).toEqual([undefined, TEST_START + 50, TEST_START + 50, undefined]);
});
