import { Simulation } from "@destack/environment/simulation";
import { expect, test } from "@destack/test";
import type { Row, Table } from "@destack/db";
import { Feed } from "../feed/feed.ts";
import { ConditionAudience } from "../feed/test/audience.ts";
import { first, note, open, take, START } from "../test/fixture.ts";

/** An audience showing every row and counting the rows it decides. */
class CountingAudience extends ConditionAudience {
    /** The rows the audience decided. */
    decided = 0;

    /** Show every row and count it. */
    override async visible(table: Table, rows: readonly Row[]): Promise<ReadonlySet<number>> {
        this.decided += rows.length;

        return super.visible(table, rows);
    }
}

test("decide only the changed rows meeting a query's condition", async () => {
    const simulation = new Simulation({ seed: 1, start: START });
    const database = await open({ simulation });
    const feed = new Feed(database, [note]);
    const audience = new CountingAudience(new Map());
    const signal = AbortSignal.timeout(5000);
    const query = { notes: { table: note, scopes: ["inbox"], where: { title: "Open" } } };
    const start = await database.log.position();

    // add a note the query rejects, then one it accepts
    const pages = feed.subscribe(query, start, signal, { audience });
    const accepted = take(pages, (page) => page.changes.some((change) => change.row["id"] === "b"));
    await database.transaction(
        async (transaction) =>
            await transaction.insert(note).values({ ...first, id: "a", title: "Done" }),
    );
    await database.transaction(
        async (transaction) =>
            await transaction.insert(note).values({ ...first, id: "b", title: "Open" }),
    );
    await accepted;

    // ask the audience only about the accepted note
    expect(audience.decided).toBe(1);
});
