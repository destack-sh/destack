import { v7 } from "uuid";
import { asc, eq, type DatabaseConnection } from "@destack/db";
import { AuditEvent } from "../event/index.ts";
import { encodeEvent } from "../event/encode.ts";
import type { AuditWriter } from "../record/index.ts";
import { AuditError } from "../error/index.ts";
import { auditOutbox, auditSender } from "./stack/index.ts";
import {
    AuditProducerId,
    AuditAcknowledgement,
    type AuditEntry,
    type AuditOutboxOptions,
    type AuditDestination,
} from "./delivery.ts";

/** Persist application audit events and deliver them in committed order. */
export class AuditOutbox implements AuditWriter<DatabaseConnection> {
    /** Prepared application database containing the outbox schema. */
    readonly database: DatabaseConnection;

    /** Bind storage migrated by the host. */
    constructor(database: DatabaseConnection) {
        this.database = database;
    }

    /** Append an immutable event directly or inside an application transaction. */
    async append(value: AuditEvent, transaction?: DatabaseConnection): Promise<void> {
        const { event, content } = encodeEvent(value);

        // require the active transaction of this physical database
        if (transaction) {
            if (!transaction.connection.transaction || transaction.state !== this.database.state) {
                throw new AuditError(
                    "INVALID_EVENT",
                    "audit recording requires a transaction on this database",
                );
            }
            await this.#insert(event, content, transaction);
        } else {
            await this.database.transaction((transaction) =>
                this.#insert(event, content, transaction),
            );
        }
    }

    /** Read committed pending events for local inspection. */
    async read(limit = 100): Promise<AuditEvent[]> {
        checkLimit(limit);
        const rows = await this.database
            .select({ event: auditOutbox.event })
            .from(auditOutbox)
            .orderBy(asc(auditOutbox.recordedAt), asc(auditOutbox.id))
            .limit(limit);

        return rows.map((row) => row.event);
    }

    /** Persist one delivery position after its application transaction has committed. */
    async next(): Promise<AuditEntry | null> {
        return this.database.transaction(
            async (transaction) => {
                // serialize delivery selection across connections without holding locks during transport
                await transaction
                    .insert(auditSender)
                    .values({
                        name: "history",
                        id: AuditProducerId.parse(`audit-producer-${v7()}`),
                        sequence: 0,
                    })
                    .onConflictDoUpdate({ target: auditSender.name, set: { name: "history" } });
                const sender = await transaction
                    .select()
                    .from(auditSender)
                    .where(eq(auditSender.name, "history"))
                    .get();
                if (!sender) {
                    throw new AuditError("CONFLICT", "audit sender is missing");
                }

                // retry exactly the selected event until its position is acknowledged
                const event = sender.eventId
                    ? await transaction
                          .select()
                          .from(auditOutbox)
                          .where(eq(auditOutbox.id, sender.eventId))
                          .get()
                    : await transaction
                          .select()
                          .from(auditOutbox)
                          .orderBy(asc(auditOutbox.recordedAt), asc(auditOutbox.id))
                          .get();
                if (!event && sender.eventId) {
                    throw new AuditError("CONFLICT", "pending audit delivery is missing");
                }
                if (!event) {
                    return null;
                }

                // persist selection before releasing the database lock
                await transaction
                    .update(auditSender)
                    .set({ eventId: event.id })
                    .where(eq(auditSender.name, "history"));

                return { producerId: sender.id, sequence: sender.sequence + 1, event: event.event };
            },
            { isolationLevel: "read committed" },
        );
    }

    /** Remove a delivery only after the destination durably accepted its exact position. */
    async acknowledge(delivery: AuditEntry, value: AuditAcknowledgement): Promise<void> {
        const accepted = AuditAcknowledgement.parse(value);
        if (
            accepted.producerId !== delivery.producerId ||
            accepted.sequence !== delivery.sequence
        ) {
            throw new AuditError("CONFLICT", "audit acknowledgement does not match the delivery");
        }

        await this.database.transaction(
            async (transaction) => {
                // lock the producer before changing progress or removing an event
                await transaction
                    .update(auditSender)
                    .set({ name: "history" })
                    .where(eq(auditSender.id, delivery.producerId));
                const sender = await transaction
                    .select()
                    .from(auditSender)
                    .where(eq(auditSender.id, delivery.producerId))
                    .get();
                if (!sender) {
                    throw new AuditError(
                        "CONFLICT",
                        "audit acknowledgement has an unknown producer",
                    );
                }

                // concurrent senders may acknowledge the same already completed delivery
                if (sender.sequence >= delivery.sequence) {
                    return;
                }
                if (
                    sender.sequence + 1 !== delivery.sequence ||
                    sender.eventId !== delivery.event.id
                ) {
                    throw new AuditError("CONFLICT", "audit acknowledgement skips pending events");
                }

                // advance progress and release its event reference before deleting the outbox record
                await transaction
                    .update(auditSender)
                    .set({ sequence: delivery.sequence, eventId: null })
                    .where(eq(auditSender.id, sender.id));
                await transaction.delete(auditOutbox).where(eq(auditOutbox.id, delivery.event.id));
            },
            { isolationLevel: "read committed" },
        );
    }

    /** Deliver a bounded number of events without holding application transactions open. */
    async flush(destination: AuditDestination, limit = 100, signal?: AbortSignal): Promise<number> {
        // deliver events until the limit or an empty outbox
        checkLimit(limit);
        let delivered = 0;
        while (delivered < limit) {
            // preserve the selected delivery across network failures and host restarts
            signal?.throwIfAborted();
            const delivery = await this.next();
            if (!delivery) {
                break;
            }

            // bound the network request and persist acknowledgement before selecting another event
            const timeout = AbortSignal.timeout(30000);
            const requestSignal = signal ? AbortSignal.any([signal, timeout]) : timeout;
            const accepted = await destination.ingest(delivery, { signal: requestSignal });
            await this.acknowledge(delivery, accepted);
            delivered++;
        }

        return delivered;
    }

    /** Retry delivery with bounded backoff until the host stops. */
    async run(destination: AuditDestination, options: AuditOutboxOptions): Promise<void> {
        // validate the retry interval and start backoff at it
        const interval = options.interval ?? 1000;
        if (!Number.isFinite(interval) || interval < 1 || interval > 60000) {
            throw new AuditError(
                "INVALID_EVENT",
                "audit delivery interval must be between 1 and 60000 milliseconds",
            );
        }
        let delay = interval;
        while (!options.signal.aborted) {
            // report failures while retaining every unacknowledged event
            try {
                const delivered = await this.flush(destination, 100, options.signal);
                delay = interval;
                if (delivered) {
                    continue;
                }
            } catch (error) {
                if (options.signal.aborted) {
                    return;
                }
                options.report(error);
                delay = Math.min(delay * 2, 60000);
            }

            // release retry timers immediately during shutdown
            await new Promise<void>((resolve) => {
                // resolve once on timeout or abort
                const finish = () => {
                    clearTimeout(timer);
                    options.signal.removeEventListener("abort", finish);
                    resolve();
                };
                const timer = setTimeout(finish, delay);
                options.signal.addEventListener("abort", finish, { once: true });
                if (options.signal.aborted) {
                    finish();
                }
            });
        }
    }

    /** Reject reuse of an event identity with different contents. */
    async #insert(
        event: AuditEvent,
        content: string,
        transaction: DatabaseConnection,
    ): Promise<void> {
        await transaction
            .insert(auditOutbox)
            .values({ id: event.id, event, content, recordedAt: Date.now() })
            .onConflictDoNothing();
        const row = await transaction
            .select({ content: auditOutbox.content })
            .from(auditOutbox)
            .where(eq(auditOutbox.id, event.id))
            .get();
        if (!row || row.content !== content) {
            throw new AuditError("CONFLICT", "audit event identifier has conflicting contents");
        }
    }
}

/** Bound work and memory per delivery or inspection request. */
function checkLimit(limit: number): void {
    if (!Number.isInteger(limit) || limit < 1 || limit > 1000) {
        throw new AuditError("INVALID_EVENT", "audit limit must be between 1 and 1000");
    }
}
