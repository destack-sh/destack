import { AuditOutbox } from "@destack/audit/outbox";
import type { DatabaseConnection } from "@destack/db";

/** The most events a test reads from an outbox. */
const READ_EVENTS = 1000;

/** Read the actions a database's outbox holds, by callers or by the system, a failed one with its error code. */
export async function auditedActions(
    database: DatabaseConnection,
    actor: "caller" | "system",
): Promise<string[]> {
    const events = await new AuditOutbox(database).read(READ_EVENTS);

    return events
        .filter((event) => (event.context.actor.type === "system") === (actor === "system"))
        .map((event) =>
            event.result.stage === "result" && event.result.outcome !== "success"
                ? `${event.action.name} ${event.result.errorCode}`
                : event.action.name,
        );
}
