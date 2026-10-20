import { expect, test } from "@destack/test";
import { render } from "./render.ts";
import { sql } from "./sql.ts";

test("read a statement as a read when it selects without a writing verb outside its quotes", () => {
    // classify selects, common tables and writes
    const statements = {
        select: sql`SELECT 1`,
        common: sql`WITH bounds AS (SELECT 1 AS logged) SELECT logged FROM bounds`,
        quoted: sql`SELECT 'delete' AS "update"`,
        commonWrite: sql`WITH gone AS (SELECT 1) DELETE FROM task WHERE id IN (SELECT * FROM gone)`,
        writableCommon: sql`WITH moved AS (UPDATE task SET priority = 1 RETURNING id) SELECT id FROM moved`,
        locking: sql`SELECT id FROM task FOR UPDATE`,
        insert: sql`INSERT INTO task (id) VALUES ('a')`,
        replacing: sql`SELECT replace(name, 'a', 'b') FROM task`,
        replace: sql`REPLACE INTO task (id) VALUES ('a')`,
        pragma: sql`PRAGMA defer_foreign_keys = ON`,
    };
    const reads = Object.fromEntries(
        Object.entries(statements).map(([name, statement]) => [
            name,
            render(statement, "sqlite").isRead,
        ]),
    );

    expect(reads).toEqual({
        select: true,
        common: true,
        quoted: true,
        commonWrite: false,
        writableCommon: false,
        locking: false,
        insert: false,
        replacing: true,
        replace: false,
        pragma: false,
    });
});
