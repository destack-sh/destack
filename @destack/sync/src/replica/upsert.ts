import {
    PARAMETER_BUDGET,
    sql,
    TABLE,
    type DatabaseConnection,
    type Row,
    type Table,
} from "@destack/db";

/**
 * Write held rows to a copied table as they are, a batch of rows per statement.
 *
 * An upsert fires the log's update trigger on held rows and its insert trigger on new ones.
 */
export async function upsert(
    database: DatabaseConnection,
    table: Table,
    rows: readonly Row[],
): Promise<void> {
    // update every written column but the key from the proposed row
    const key = table[TABLE].key;
    const columns = table[TABLE].columns;
    const written = [...new Set(rows.flatMap((row) => Object.keys(row)))];
    const set = Object.fromEntries(
        written
            .filter((name) => !key.includes(name))
            .map((name) => [name, sql`excluded.${sql.identifier(columns[name]!.definition.name)}`]),
    );

    // write rows with every written column, a batch within the parameter budget
    const size = Math.max(1, Math.floor(PARAMETER_BUDGET / Math.max(written.length, 1)));
    for (let start = 0; start < rows.length; start += size) {
        const batch = rows
            .slice(start, start + size)
            .map((row) => Object.fromEntries(written.map((name) => [name, row[name] ?? null])));
        const insert = database.insert(table).values(batch as never);
        await (Object.keys(set).length === 0
            ? insert.onConflictDoNothing()
            : insert.onConflictDoUpdate({
                  target: key.map((name) => columns[name]!) as never,
                  set: set as never,
              }));
    }
}
