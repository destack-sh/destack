import { expect, test } from "@destack/test";
import { TEST_DIALECTS } from "@destack/db/test";
import type { Row, Table } from "@destack/db";
import { Feed } from "../feed/feed.ts";
import { ConditionAudience } from "../feed/test/audience.ts";
import { first, note, open, take } from "../test/fixture.ts";
import { Clock } from "@destack/environment";

/** An audience admitting every row and counting the rows it decides. */
class CountingAudience extends ConditionAudience {
    /** The rows the audience decided. */
    decided = 0;

    /** Admit every row and count it. */
    override async admits(table: Table, rows: readonly Row[]): Promise<ReadonlySet<number>> {
        this.decided += rows.length;

        return super.admits(table, rows);
    }
}

test.for(TEST_DIALECTS)(
    "decide only the changed rows meeting a query's condition on %s",
    async (dialect) => {
        const database = await open(dialect);
        const feed = new Feed(database, [note], { clock: Clock.system });
        const audience = new CountingAudience(new Map());
        const signal = AbortSignal.timeout(5000);
        const query = { notes: { table: note, scopes: ["inbox"], where: { title: "Open" } } };
        const start = await database.log.position();

        // add a note the query rejects, then one it keeps
        const pages = feed.subscribe(query, start, signal, { audience });
        const kept = take(pages, (page) => page.changes.some((change) => change.row["id"] === "b"));
        await database.insert(note).values({ ...first, id: "a", title: "Done" });
        await database.insert(note).values({ ...first, id: "b", title: "Open" });
        await kept;

        // ask the audience only about the kept note
        expect(audience.decided).toBe(1);
    },
);
