import type { DatabaseConnection } from "@destack/db";
import { CallEvent, call, journal } from "../../src/index.ts";

/** Read the audited calls a database records as call events, in journal order. */
export async function auditedCalls(database: DatabaseConnection): Promise<CallEvent[]> {
    // read the call events
    const table = call.eventKind().table;
    const rows = await database.select({ data: table.data }).from(table);

    // read the journal's whole log, noting where each entry was first written
    const positions = new Map<unknown, number>();
    for (let after = 0, isDone = false; !isDone;) {
        const page = await database.log.read({ tables: [journal], after });
        for (const change of page.changes) {
            if (!positions.has(change.key.id)) {
                positions.set(change.key.id, positions.size);
            }
        }
        isDone = page.changes.length === 0 || page.sequence === after;
        after = page.sequence;
    }

    // order each call by its entry's place in the log
    const calls = rows.map((row) => CallEvent.parse(row.data));
    const position = (ended: CallEvent) => positions.get(ended.execution.id) ?? Infinity;

    return calls.toSorted((left, right) => position(left) - position(right));
}

/** Read the methods a database's audited calls ran, a failed one with its error code. */
export async function auditedActions(
    database: DatabaseConnection,
    actor: "caller" | "component",
): Promise<string[]> {
    const calls = await auditedCalls(database);

    return calls
        .filter(
            (ended) =>
                (ended.execution.context.component !== undefined) === (actor === "component"),
        )
        .map((ended) => {
            const outcome = ended.execution.outcome;

            return outcome.kind !== "success"
                ? `${ended.method} ${outcome.error.code}`
                : ended.method;
        });
}
