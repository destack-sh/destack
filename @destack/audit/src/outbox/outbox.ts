import { v7 } from "uuid";
import { asc, eq, type DatabaseConnection } from "@destack/db";
import { canonicalize } from "@destack/schema/json";
import { wait } from "@destack/service/timer";
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

/** The name of the sender delivering to the configured history. */
const SENDER = "history";
/** The events one flush delivers and one read returns by default. */
const BATCH_SIZE = 100;
/** How long one delivery request may take, in milliseconds. */
const DELIVERY_TIMEOUT_MS = 30000;
/** The first retry delay after a failed delivery, in milliseconds. */
const DEFAULT_RETRY_DELAY_MS = 1000;
/** The longest retry delay, in milliseconds. */
const MAX_RETRY_DELAY_MS = 60000;

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
        // encode the event
        const { event, content } = encodeEvent(value);

        // insert into the caller's transaction, which must run on this physical database
        if (transaction) {
            if (!transaction.driver.transaction || transaction.state !== this.database.state) {
                throw new AuditError(
                    "INVALID_EVENT",
                    "audit recording requires a transaction on this database",
                );
            }
            await this.#insert(event, content, transaction);
        }
        // insert in a transaction of its own
        else {
            await this.database.transaction((transaction) =>
                this.#insert(event, content, transaction),
            );
        }
    }

    /** Read committed pending events for local inspection. */
    async read(limit = BATCH_SIZE): Promise<AuditEvent[]> {
        // read the oldest pending events
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
                        name: SENDER,
                        id: AuditProducerId.parse(`audit-producer-${v7()}`),
                        sequence: 0,
                    })
                    .onConflictDoUpdate({ target: auditSender.name, set: { name: SENDER } });
                const sender = await transaction
                    .select()
                    .from(auditSender)
                    .where(eq(auditSender.name, SENDER))
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

                // fail on a lost selected event, and report an empty outbox
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
                    .where(eq(auditSender.name, SENDER));

                return { producerId: sender.id, sequence: sender.sequence + 1, event: event.event };
            },
            { isolationLevel: "read committed" },
        );
    }

    /** Remove a delivery only after the destination durably accepted its exact position. */
    async acknowledge(delivery: AuditEntry, value: AuditAcknowledgement): Promise<void> {
        // require the acknowledgement of this exact delivery
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
                    .set({ name: SENDER })
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

                // skip a delivery a concurrent sender already acknowledged
                if (sender.sequence >= delivery.sequence) {
                    return;
                }

                // require the next position and its selected event
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
    async flush(
        destination: AuditDestination,
        limit = BATCH_SIZE,
        signal?: AbortSignal,
    ): Promise<number> {
        // deliver events until the limit or an empty outbox
        let delivered = 0;
        while (delivered < limit) {
            // preserve the selected delivery across network failures and host restarts
            signal?.throwIfAborted();
            const delivery = await this.next();
            if (!delivery) {
                break;
            }

            // bound the network request and persist acknowledgement before selecting another event
            const timeout = AbortSignal.timeout(DELIVERY_TIMEOUT_MS);
            const requestSignal = signal ? AbortSignal.any([signal, timeout]) : timeout;
            const accepted = await destination.ingest(delivery, { signal: requestSignal });
            await this.acknowledge(delivery, accepted);
            delivered++;
        }

        return delivered;
    }

    /** Deliver events as they commit, retrying failed deliveries with bounded backoff, until the host stops. */
    async run(destination: AuditDestination, options: AuditOutboxOptions): Promise<void> {
        // validate the first retry delay
        const retryDelay = options.retryDelay ?? DEFAULT_RETRY_DELAY_MS;
        if (!Number.isFinite(retryDelay) || retryDelay < 1 || retryDelay > MAX_RETRY_DELAY_MS) {
            throw new RangeError(
                `audit retry delay must be between 1 and ${MAX_RETRY_DELAY_MS} milliseconds`,
            );
        }

        // start each run of failures at the first retry delay
        let delay = retryDelay;
        while (!options.signal.aborted) {
            // deliver pending events, then wait for a commit that appends more
            try {
                const delivered = await this.flush(destination, BATCH_SIZE, options.signal);
                delay = retryDelay;
                if (delivered === 0) {
                    await this.database.log.until(() => this.#isPending(), options.signal);
                }
            } catch (error) {
                // stop quietly on shutdown
                if (options.signal.aborted) {
                    return;
                }

                // report the failure and back off, retaining every unacknowledged event
                options.report(error);
                await wait(delay, { signal: options.signal }).catch((error: unknown) => {
                    // stop retrying once the signal aborts
                    if (!options.signal.aborted) {
                        throw error;
                    }
                });
                delay = Math.min(delay * 2, MAX_RETRY_DELAY_MS);
            }
        }
    }

    /** Whether the outbox holds an event to deliver. */
    async #isPending(): Promise<boolean> {
        // look for any event
        const event = await this.database
            .select({ id: auditOutbox.id })
            .from(auditOutbox)
            .limit(1)
            .get();

        return event !== undefined;
    }

    /** Reject reuse of an event identity with different contents. */
    async #insert(
        event: AuditEvent,
        content: string,
        transaction: DatabaseConnection,
    ): Promise<void> {
        // insert the event, comparing an existing one of the same identity by its canonical contents
        const inserted = await transaction
            .insert(auditOutbox)
            .values({ id: event.id, event, recordedAt: Date.now() })
            .onConflictDoNothing()
            .returning({ id: auditOutbox.id });
        if (inserted.length > 0) {
            return;
        }

        // accept an existing event only with the same contents
        const row = await transaction
            .select({ event: auditOutbox.event })
            .from(auditOutbox)
            .where(eq(auditOutbox.id, event.id))
            .get();
        if (!row || canonicalize(row.event) !== content) {
            throw new AuditError("CONFLICT", "audit event identifier has conflicting contents");
        }
    }
}
