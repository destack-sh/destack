import type { DatabaseConnection } from "@destack/db";
import { testCallKey } from "@destack/service/test";
import { Journal, AuditCaller } from "@destack/audit";

/** The most calls a test reads from a journal. */
const READ_CALLS = 1000;

/** Read the methods a database's journal recorded, by callers or by the system, a failed one with its error code. */
export async function auditedActions(
    database: DatabaseConnection,
    actor: "caller" | "system",
): Promise<string[]> {
    const calls = await new Journal(database, testCallKey).read({ limit: READ_CALLS });

    return calls
        .filter(
            (call) =>
                (AuditCaller.actor(call.execution.context.caller).type === "system") ===
                (actor === "system"),
        )
        .map((call) => {
            const outcome = call.execution.outcome;

            return outcome !== undefined && outcome.kind !== "success"
                ? `${call.method} ${outcome.error.code}`
                : call.method;
        });
}
