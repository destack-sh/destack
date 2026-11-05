import type { DatabaseConnection } from "@destack/db";
import { CallEvent, call } from "../index.ts";

/** Read the audited calls a database records as call events, in the order the log appended them. */
export async function auditedCalls(database: DatabaseConnection): Promise<CallEvent[]> {
    // read the call events
    const table = call.eventKind().table;
    const rows = await database.select({ id: table.id, data: table.data }).from(table);

    // read the events' whole log, noting where each was appended
    const positions = new Map<unknown, number>();
    for (let after = 0, isDone = false; !isDone;) {
        const page = await database.log.read({ tables: [table], after });
        for (const change of page.changes) {
            if (!positions.has(change.key["id"])) {
                positions.set(change.key["id"], positions.size);
            }
        }
        isDone = page.changes.length === 0 || page.sequence === after;
        after = page.sequence;
    }

    // order each call by its event's place in the log
    const ordered = rows.toSorted(
        (left, right) =>
            (positions.get(left.id) ?? Infinity) - (positions.get(right.id) ?? Infinity),
    );

    return ordered.map((row) => CallEvent.parse(row.data));
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
