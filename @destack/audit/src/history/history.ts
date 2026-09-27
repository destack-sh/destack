import { v7 } from "uuid";
import {
    and,
    or,
    eq,
    gt,
    lt,
    gte,
    inArray,
    notInArray,
    isNotNull,
    asc,
    type DatabaseConnection,
    type SQL,
} from "@destack/db";
import { AuditEvent, type AuditActor } from "../event/index.ts";
import { subjectKey } from "@destack/access";
import { canonicalize } from "@destack/schema/json";
import { identifier } from "@destack/schema/identifier";
import { encodeEvent } from "../event/encode.ts";
import { AuditProducerId, AuditEntry, type AuditAcknowledgement } from "../outbox/delivery.ts";
import { AuditError } from "../error/index.ts";
import { AuditPrune, AuditQuery, type AuditPage, type AuditScope } from "./query.ts";
import { auditEvent, auditTarget, auditProducer } from "./stack/index.ts";

/** Persist immutable history and query it within an authorized scope. */
export class AuditHistory {
    /** Prepared history database. */
    readonly database: DatabaseConnection;

    /** Bind storage whose scope is enforced by the hosting service. */
    constructor(database: DatabaseConnection) {
        this.database = database;
    }

    /** Accept an ordered event and acknowledge its durable producer position. */
    async ingest(value: AuditEntry): Promise<AuditAcknowledgement> {
        // encode the event and digest its content
        const entry = AuditEntry.parse(value);
        const { event, content } = encodeEvent(entry.event);
        const hash = await digest(content);

        return this.database.transaction(
            async (transaction) => {
                // reserve and lock producer progress independently of retained history
                await transaction
                    .insert(auditProducer)
                    .values({ id: entry.producerId, sequence: 0, digest: null })
                    .onConflictDoUpdate({
                        target: auditProducer.id,
                        set: { id: entry.producerId },
                    });
                const producer = await transaction
                    .select()
                    .from(auditProducer)
                    .where(eq(auditProducer.id, entry.producerId))
                    .get();
                if (!producer || producer.retiredAt !== null) {
                    throw new AuditError("FORBIDDEN", "audit producer is retired or unavailable");
                }

                // acknowledge unchanged retries even after history retention
                if (entry.sequence === producer.sequence) {
                    if (producer.digest !== hash) {
                        throw new AuditError(
                            "CONFLICT",
                            "audit delivery position has conflicting contents",
                        );
                    }

                    return { producerId: entry.producerId, sequence: entry.sequence };
                }

                // require the next position
                if (entry.sequence !== producer.sequence + 1) {
                    throw new AuditError(
                        "CONFLICT",
                        "audit delivery position is stale or skips events",
                    );
                }

                // index the event, accepting an identical one and rejecting other contents
                await this.#insert(event, transaction);

                // commit progress and event persistence atomically
                await transaction
                    .update(auditProducer)
                    .set({ sequence: entry.sequence, digest: hash })
                    .where(eq(auditProducer.id, entry.producerId));

                return { producerId: entry.producerId, sequence: entry.sequence };
            },
            { isolationLevel: "read committed" },
        );
    }

    /** Revoke a producer without releasing its identity for reuse. */
    async retire(producerId: string): Promise<void> {
        // mark the producer retired, reserving its identity when it never delivered
        const retiredAt = Date.now();
        await this.database
            .insert(auditProducer)
            .values({ id: AuditProducerId.parse(producerId), sequence: 0, digest: null, retiredAt })
            .onConflictDoUpdate({ target: auditProducer.id, set: { retiredAt } });
    }

    /** Index an event independently of live application tables. */
    async #insert(event: AuditEvent, transaction: DatabaseConnection): Promise<void> {
        // compare a result against its retained attempt and reject competing outcomes
        if (event.attemptId) {
            const attempt = await transaction
                .select({ event: auditEvent.event })
                .from(auditEvent)
                .where(eq(auditEvent.id, event.attemptId))
                .get();
            if (attempt) {
                const {
                    id: _attemptId,
                    occurredAt: _attemptTime,
                    result: _attemptResult,
                    ...origin
                } = attempt.event;
                const {
                    id: _eventId,
                    occurredAt: _eventTime,
                    result: _eventResult,
                    attemptId: _reference,
                    ...completion
                } = event;
                if (
                    attempt.event.result.stage !== "attempt" ||
                    canonicalize(origin) !== canonicalize(completion)
                ) {
                    throw new AuditError("CONFLICT", "audit result differs from its attempt");
                }
            }
        }

        // extract query columns and retain the complete historical event
        const actor = actorKey(event.context.actor);
        const scope = event.context.scope;
        const inserted = await transaction
            .insert(auditEvent)
            .values({
                id: event.id,
                scope,
                attemptId: event.attemptId ?? null,
                action: event.action.name,
                packageId: event.action.package.id,
                actor,
                stage: event.result.stage,
                outcome: "outcome" in event.result ? event.result.outcome : null,
                occurredAt: event.occurredAt,
                recordedAt: Date.now(),
                event,
            })
            .onConflictDoNothing()
            .returning({ id: auditEvent.id });

        // resolve concurrent uniqueness conflicts through the persisted event identity
        if (!inserted.length) {
            const existing = await transaction
                .select({ event: auditEvent.event })
                .from(auditEvent)
                .where(eq(auditEvent.id, event.id))
                .get();
            if (existing && canonicalize(existing.event) === canonicalize(event)) {
                return;
            }

            // reject a changed identity or a second result of the attempt
            throw new AuditError(
                "CONFLICT",
                existing
                    ? "audit event identifier has conflicting contents"
                    : "audit attempt already has a result",
            );
        }

        // index named object references without creating live foreign keys
        const targets = Object.entries(event.targets).map(([role, target]) => ({
            id: identifier("audit-target").parse(`audit-target-${v7()}`),
            event: event.id,
            scope,
            role,
            type: target.type,
            objectId: target.id,
        }));
        if (targets.length) {
            await transaction.insert(auditTarget).values(targets);
        }
    }

    /** Read one event of a scope's history. */
    async get(scope: AuditScope, id: AuditEvent["id"]) {
        // read the event within its scope
        const row = await this.database
            .select({ event: auditEvent.event, recordedAt: auditEvent.recordedAt })
            .from(auditEvent)
            .where(and(eq(auditEvent.scope, scope), eq(auditEvent.id, id)))
            .get();
        if (!row) {
            throw new AuditError("NOT_FOUND", "audit event not found");
        }

        return row;
    }

    /** Read a bounded page in durable acceptance order. */
    async list(request: AuditQuery): Promise<AuditPage> {
        // parse the query and filter by scope
        const query = AuditQuery.parse(request);
        const filters: (SQL | undefined)[] = [eq(auditEvent.scope, query.scope)];

        // select declared actions and producers
        if (query.action) {
            filters.push(eq(auditEvent.action, query.action));
        }
        if (query.packageId) {
            filters.push(eq(auditEvent.packageId, query.packageId));
        }

        // select an actor or occurrence
        if (query.actor) {
            filters.push(eq(auditEvent.actor, actorKey(query.actor)));
        }
        if (query.attemptId) {
            filters.push(
                or(eq(auditEvent.id, query.attemptId), eq(auditEvent.attemptId, query.attemptId)),
            );
        }

        // select observed outcomes
        if (query.outcome) {
            filters.push(eq(auditEvent.outcome, query.outcome));
        }

        // find attempts whose result has not arrived, among the results of the same scope
        if (query.unresolved) {
            const completed = this.database
                .select({ id: auditEvent.attemptId })
                .from(auditEvent)
                .where(and(eq(auditEvent.scope, query.scope), isNotNull(auditEvent.attemptId)));
            filters.push(eq(auditEvent.stage, "attempt"), notInArray(auditEvent.id, completed));
        }

        // restrict durable acceptance time
        if (query.from !== undefined) {
            filters.push(gte(auditEvent.recordedAt, query.from));
        }
        if (query.before !== undefined) {
            filters.push(lt(auditEvent.recordedAt, query.before));
        }

        // resume after the last accepted record
        if (query.cursor) {
            filters.push(
                or(
                    gt(auditEvent.recordedAt, query.cursor.recordedAt),
                    and(
                        eq(auditEvent.recordedAt, query.cursor.recordedAt),
                        gt(auditEvent.id, query.cursor.id),
                    ),
                ),
            );
        }

        // search affected objects through their target index
        if (query.target) {
            const targets = this.database
                .select({ id: auditTarget.event })
                .from(auditTarget)
                .where(
                    and(
                        eq(auditTarget.type, query.target.type),
                        eq(auditTarget.objectId, query.target.id),
                    ),
                );
            filters.push(inArray(auditEvent.id, targets));
        }

        // fetch one extra record to distinguish exhaustion from a full page
        const rows = await this.database
            .select({ event: auditEvent.event, recordedAt: auditEvent.recordedAt })
            .from(auditEvent)
            .where(and(...filters))
            .orderBy(asc(auditEvent.recordedAt), asc(auditEvent.id))
            .limit(query.limit + 1);
        const items = rows.slice(0, query.limit);
        const last = items.at(-1);

        return {
            items,
            cursor:
                rows.length > query.limit && last
                    ? { recordedAt: last.recordedAt, id: last.event.id }
                    : null,
        };
    }

    /** Remove expired event contents while retaining producer progress. */
    async prune(request: AuditPrune): Promise<number> {
        // validate the scope, cutoff and bound
        const { scope, before, limit } = AuditPrune.parse(request);

        // remove the oldest expired events, cascading to their target indexes
        const expired = this.database
            .select({ id: auditEvent.id })
            .from(auditEvent)
            .where(and(eq(auditEvent.scope, scope), lt(auditEvent.recordedAt, before)))
            .orderBy(asc(auditEvent.recordedAt), asc(auditEvent.id))
            .limit(limit);
        const removed = await this.database
            .delete(auditEvent)
            .where(inArray(auditEvent.id, expired))
            .returning({ id: auditEvent.id });

        return removed.length;
    }

    /** Stream the bounded pages of events accepted before the export started. */
    async *export(request: AuditQuery, signal?: AbortSignal): AsyncGenerator<AuditPage["items"]> {
        // fix the end of the export at its start
        const query = AuditQuery.parse({
            ...request,
            before:
                request.before === undefined ? Date.now() : Math.min(request.before, Date.now()),
        });
        while (true) {
            // keep memory bounded and stop before fetching another page on cancellation
            signal?.throwIfAborted();
            const page = await this.list(query);
            yield page.items;
            if (!page.cursor) {
                return;
            }
            query.cursor = page.cursor;
        }
    }
}

/** Hash validated JSON before taking the database writer lock. */
async function digest(content: string): Promise<string> {
    // hash the content bytes as hexadecimal
    const bytes = new TextEncoder().encode(content);
    const hash = await crypto.subtle.digest("SHA-256", bytes);

    return new Uint8Array(hash).toHex();
}

/** Encode the complete identity for indexed history selection. */
function actorKey(actor: AuditActor): string {
    // key a subject as access keys it
    if (actor.type === "subject") {
        return subjectKey(actor.subject);
    }
    // key a system component by its name
    else if (actor.type === "system") {
        return JSON.stringify([actor.type, actor.name]);
    }
    // key an anonymous caller by its type
    else {
        return JSON.stringify([actor.type]);
    }
}
