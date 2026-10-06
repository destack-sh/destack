import { expect, onTestFinished, test } from "@destack/test";
import { DatabaseError } from "../error/error.ts";
import { eq } from "../sql/index.ts";
import { integer, text } from "../table/column.ts";
import { defineTable } from "../table/table.ts";
import { TEST_DIALECTS, TestDatabase } from "../test/database.ts";

/** Counters by name. */
const tally = defineTable("retry_tally", {
    /** The counter's name. */
    name: text("name").primaryKey(),
    /** The counted value. */
    value: integer("value").notNull(),
});

/** Open a database keeping one counter at zero. */
async function open(dialect: (typeof TEST_DIALECTS)[number]) {
    const storage = await TestDatabase.create(dialect, [tally], { isMigrated: true });
    onTestFinished(() => storage.close());
    await storage.database.insert(tally).values({ name: "a", value: 0 });

    return storage.database;
}

test.for(TEST_DIALECTS)(
    "commit both of two concurrent transactions incrementing the same row, running the one losing the race again on %s",
    async (dialect) => {
        // read and increment the counter in two transactions at once, each reading before either writes
        const database = await open(dialect);
        const reads = [Promise.withResolvers<void>(), Promise.withResolvers<void>()];
        const increment = (index: number) =>
            database.transaction(async (transaction) => {
                const [row] = await transaction.select().from(tally).where(eq(tally.name, "a"));
                reads[index]?.resolve();
                await Promise.race([
                    Promise.all(reads.map((read) => read.promise)),
                    new Promise((resolve) => {
                        setTimeout(resolve, 100);
                    }),
                ]);
                await transaction
                    .update(tally)
                    .set({ value: (row?.value ?? 0) + 1 })
                    .where(eq(tally.name, "a"));
            });
        await Promise.all([increment(0), increment(1)]);

        // keep both increments
        expect(await database.select({ value: tally.value }).from(tally)).toEqual([{ value: 2 }]);
    },
);

test.for(TEST_DIALECTS)(
    "answer a transaction that loses every race as unavailable after five attempts, and a failure of another kind at once, on %s",
    async (dialect) => {
        // lose the race at each attempt, and fail otherwise once
        const database = await open(dialect);
        let losing = 0;
        const lost = await database
            .transaction(async () => {
                losing++;
                throw new Error("database is locked");
            })
            .catch((error: unknown) => error);
        let failing = 0;
        const failed = await database
            .transaction(async () => {
                failing++;
                throw new TypeError("not a race");
            })
            .catch((error: unknown) => error);

        expect({
            losing,
            code: lost instanceof DatabaseError ? lost.toServiceError().code : lost,
            failing,
            failed,
        }).toEqual({
            losing: 5,
            code: "SERVICE_UNAVAILABLE",
            failing: 1,
            failed: new TypeError("not a race"),
        });
    },
);
