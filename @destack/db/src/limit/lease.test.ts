import { expect, onTestFinished, test } from "@destack/test";
import { TEST_DIALECTS, TestDatabase } from "../test/database.ts";
import { lease, Lease } from "./lease.ts";

/** The time the scenarios start at, in UTC epoch milliseconds. */
const START = 1_800_000_000_000;

/** Open a database with the lease table. */
async function open(dialect: (typeof TEST_DIALECTS)[number]) {
    const storage = await TestDatabase.create(dialect, [lease], { isMigrated: true });
    onTestFinished(() => storage.close());

    return storage.database;
}

test.for(TEST_DIALECTS)(
    "take over a lease at the next epoch once it lapses, refusing the old holder until it lapses again, on %s",
    async (dialect) => {
        const database = await open(dialect);
        const acquire = (holder: string, duration: number, at: number) =>
            Lease.acquire(database, {
                name: "job",
                key: "shared",
                holder,
                duration,
                now: START + at,
            });

        // take, refuse and renew a lease
        const taken = await acquire("first", 20, 0);
        const refused = await acquire("second", 20, 0);
        const renewed = await acquire("first", 20, 0);

        // take over the lease the moment it lapses, refusing its old holder
        const takenOver = await acquire("second", 1000, 20);
        const late = await acquire("first", 1000, 20);
        expect([taken, refused, renewed, takenOver, late]).toEqual([
            { kind: "acquired", slot: 0, epoch: 1, expiresAt: START + 20 },
            { kind: "busy", lapsesAt: START + 20 },
            { kind: "acquired", slot: 0, epoch: 1, expiresAt: START + 20 },
            { kind: "acquired", slot: 0, epoch: 2, expiresAt: START + 1020 },
            { kind: "busy", lapsesAt: START + 1020 },
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "reuse an own lease without writing while two thirds of it remain, and renew it once a third passed, on %s",
    async (dialect) => {
        const database = await open(dialect);
        const acquire = (at: number) =>
            Lease.acquire(database, {
                name: "job",
                key: "shared",
                holder: "first",
                duration: 300,
                now: START + at,
            });

        // take the lease, take it again at once, and again once a third of it passed
        expect([await acquire(0), await acquire(90), await acquire(120)]).toEqual([
            { kind: "acquired", slot: 0, epoch: 1, expiresAt: START + 300 },
            { kind: "acquired", slot: 0, epoch: 1, expiresAt: START + 300 },
            { kind: "acquired", slot: 0, epoch: 1, expiresAt: START + 420 },
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "lease a key to as many holders as it has slots, a released slot to the next holder, on %s",
    async (dialect) => {
        const database = await open(dialect);
        const acquire = (holder: string) =>
            Lease.acquire(database, {
                name: "review",
                key: "space-1",
                holder,
                duration: 100,
                now: START,
                slots: 2,
            });

        // fill both slots, refuse a third holder, then lease it the slot the first releases
        const filled = [await acquire("a"), await acquire("b"), await acquire("c")];
        await Lease.release(database, { holder: "a", name: "review", key: "space-1" });
        const reacquired = await acquire("c");
        expect([...filled, reacquired]).toEqual([
            { kind: "acquired", slot: 0, epoch: 1, expiresAt: START + 100 },
            { kind: "acquired", slot: 1, epoch: 1, expiresAt: START + 100 },
            { kind: "busy", lapsesAt: START + 100 },
            { kind: "acquired", slot: 0, epoch: 1, expiresAt: START + 100 },
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "release every key a holder has, leaving other holders' on %s",
    async (dialect) => {
        const database = await open(dialect);
        const acquire = (holder: string, key: string) =>
            Lease.acquire(database, { name: "job", key, holder, duration: 100, now: START });

        // take two keys as one holder and one as another, then release the first holder's
        await acquire("first", "a");
        await acquire("first", "b");
        await acquire("second", "c");
        await Lease.release(database, { holder: "first" });
        const rows = await database.select({ key: lease.key, holder: lease.holder }).from(lease);
        expect(rows).toEqual([{ key: "c", holder: "second" }]);
    },
);

test.for(TEST_DIALECTS)(
    "read when a key's held slots lapse, the latest of them, and none once they lapsed, on %s",
    async (dialect) => {
        const database = await open(dialect);
        const request = { name: "job", key: "shared" };

        // hold two slots of a key, then read their lapse before, at and after it
        await Lease.acquire(database, {
            ...request,
            holder: "first",
            duration: 20,
            now: START,
            slots: 2,
        });
        await Lease.acquire(database, {
            ...request,
            holder: "second",
            duration: 50,
            now: START,
            slots: 2,
        });
        expect([
            await Lease.lapsesAt(database, { name: "job", key: "other", now: START }),
            await Lease.lapsesAt(database, { ...request, now: START }),
            await Lease.lapsesAt(database, { ...request, now: START + 20 }),
            await Lease.lapsesAt(database, { ...request, now: START + 50 }),
        ]).toEqual([undefined, START + 50, START + 50, undefined]);
    },
);
