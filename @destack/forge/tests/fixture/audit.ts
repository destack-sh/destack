import { asc, type DatabaseConnection } from "@destack/db";
import { CallEvent, call } from "@destack/object";

/** Read the audited calls a database stores as call events, in start order. */
export async function auditedCalls(database: DatabaseConnection): Promise<CallEvent[]> {
    const table = call.eventKind().table;
    const rows = await database
        .select({ data: table.data })
        .from(table)
        .orderBy(asc(table.time), asc(table.id));

    return rows.map((row) => CallEvent.parse(row.data));
}
