import type { DatabaseConnection, Table } from "@destack/db";
import type { Controller } from "@destack/service/control";
import { Outbox, outbox, type Address, type Destination } from "@destack/service/outbox";
import { AuditEvent } from "../event/index.ts";
import { encodeEvent } from "../event/encode.ts";
import type { AuditWriter } from "../record/index.ts";
import { AUDIT_BATCH_EVENTS, type AuditDestination } from "./delivery.ts";

/** The audit outbox tables. */
export const auditOutboxTables: readonly Table[] = [outbox as Table];

/** The outbox address of audit events. */
const AUDIT: Address<AuditEvent> = { name: "audit", message: AuditEvent };

/** The audit events waiting for delivery to a history. */
export class AuditOutbox implements AuditWriter<DatabaseConnection> {
    /** The outbox holding the pending events. */
    readonly outbox: Outbox;

    /** Bind the outbox to its database. */
    constructor(database: DatabaseConnection) {
        this.outbox = new Outbox(database);
    }

    /** The database holding the outbox. */
    get database(): DatabaseConnection {
        return this.outbox.database;
    }

    /** Append an event, inside a transaction when given. */
    async append(value: AuditEvent, transaction?: DatabaseConnection): Promise<void> {
        const { event } = encodeEvent(value);
        await this.outbox.append(AUDIT, event.id, event, transaction);
    }

    /** Read pending events, oldest first. */
    read(limit = AUDIT_BATCH_EVENTS): Promise<AuditEvent[]> {
        return this.outbox.read(AUDIT, limit);
    }

    /** Deliver the oldest pending events as one batch, returning how many. */
    deliver(history: AuditDestination, signal?: AbortSignal): Promise<number> {
        return this.outbox.deliver(delivering(history), signal);
    }

    /** Deliver pending events to a history as they commit. */
    controller(history: AuditDestination): Controller {
        return this.outbox.controller(delivering(history));
    }
}

/** Deliver audit events to a history. */
function delivering(history: AuditDestination): Destination<AuditEvent> {
    return {
        ...AUDIT,
        batch: AUDIT_BATCH_EVENTS,
        accept: (events, options) => history.ingest({ events: [...events] }, options),
    };
}
