import { asc, type DatabaseConnection } from "@destack/db";
import { CallEvent, call } from "../../src/index.ts";

/** Read the audited calls a database records as call events, in start order. */
export async function auditedCalls(database: DatabaseConnection): Promise<CallEvent[]> {
    const table = call.eventKind().table;
    const rows = await database
        .select({ data: table.data })
        .from(table)
        .orderBy(asc(table.time), asc(table.id));

    return rows.map((row) => CallEvent.parse(row.data));
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
