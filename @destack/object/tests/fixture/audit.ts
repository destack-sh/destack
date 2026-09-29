import { AuditOutbox } from "@destack/audit/outbox";
import type { DatabaseConnection } from "@destack/db";

/** The most events a test reads from an outbox. */
const READ_EVENTS = 1000;

/** Read the action names of the events a database's outbox holds, by callers or by the system. */
export async function auditedActions(
    database: DatabaseConnection,
    actor: "caller" | "system",
): Promise<string[]> {
    const events = await new AuditOutbox(database).read(READ_EVENTS);

    return events
        .filter((event) => (event.context.actor.type === "system") === (actor === "system"))
        .map((event) => event.action.name);
}
