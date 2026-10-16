import { expect, onTestFinished, test } from "@destack/test";
import { TEST_DIALECTS, TestDatabase } from "../test/database.ts";
import { bucket, Bucket, type BucketRequest } from "./bucket.ts";

/** The time the scenarios start at, in UTC epoch milliseconds. */
const START = 1_800_000_000_000;

/** Ten tokens a second, three at once. */
const LIMIT = { name: "note.create", key: "space-1", rate: { count: 10, every: 1000 }, burst: 3 };

/** Open a database with the bucket table. */
async function open(dialect: (typeof TEST_DIALECTS)[number]) {
    const storage = await TestDatabase.create(dialect, [bucket], { isMigrated: true });
    onTestFinished(() => storage.close());

    return storage.database;
}

test.for(TEST_DIALECTS)(
    "take a burst at once, refuse the next token until its interval passed, and take it then, on %s",
    async (dialect) => {
        const database = await open(dialect);
        const take = (at: number, request: Partial<BucketRequest> = {}) =>
            Bucket.take(database, { ...LIMIT, now: START + at, ...request });

        // take the burst of three, refuse a fourth, and take it 100 ms later
        const burst = [await take(0), await take(0), await take(0)];
        const refused = await take(0);
        const later = await take(100);
        expect([...burst, refused, later]).toEqual([
            { kind: "taken" },
            { kind: "taken" },
            { kind: "taken" },
            { kind: "limited", retryAfter: 100 },
            { kind: "taken" },
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "take two tokens at once once the bucket refilled, refusing them while one is missing, on %s",
    async (dialect) => {
        const database = await open(dialect);
        const take = (at: number) =>
            Bucket.take(database, { ...LIMIT, tokens: 2, now: START + at });

        // take two of three, refuse two more at once with one left, and take them 100 ms later
        const first = await take(0);
        const early = await take(0);
        const refilled = await take(100);
        expect([first, early, refilled]).toEqual([
            { kind: "taken" },
            { kind: "limited", retryAfter: 100 },
            { kind: "taken" },
        ]);
    },
);

test("refuse more tokens at once than the burst allows", async () => {
    const database = await open("sqlite");

    // ask for four tokens of a burst of three
    await expect(Bucket.take(database, { ...LIMIT, tokens: 4, now: START })).rejects.toThrow(
        new RangeError("4 tokens exceed the burst of 3 of limit note.create"),
    );
});
