import {
    and,
    or,
    eq,
    gt,
    lt,
    gte,
    isNull,
    inArray,
    notInArray,
    isNotNull,
    asc,
    type DatabaseConnection,
    type SQL,
} from "@destack/db";
import { AuditEvent, actorKey } from "../event/index.ts";
import { encodeEvent, canonical } from "../event/encode.ts";
import { AuditProducerId, AuditEntry, type AuditAcknowledgement } from "../outbox/delivery.ts";
import { AuditError } from "../error/index.ts";
import { AuditQuery, type AuditPage, type AuditScope } from "./query.ts";
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
                    .values({
                        id: entry.producerId,
                        sequence: 0,
                        digest: "",
                    })
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
                if (entry.sequence !== producer.sequence + 1) {
                    throw new AuditError(
                        "CONFLICT",
                        "audit delivery position is stale or skips events",
                    );
                }

                // reject changed identities and index newly accepted events
                const existing = await transaction
                    .select()
                    .from(auditEvent)
                    .where(eq(auditEvent.id, event.id))
                    .get();
                if (existing && canonical(existing.event) !== content) {
                    throw new AuditError(
                        "CONFLICT",
                        "audit event identifier has conflicting contents",
                    );
                }
                if (!existing) {
                    await this.#insert(event, transaction);
                }

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
        await this.database
            .insert(auditProducer)
            .values({
                id: AuditProducerId.parse(producerId),
                sequence: 0,
                digest: "",
                retiredAt: Date.now(),
            })
            .onConflictDoUpdate({ target: auditProducer.id, set: { retiredAt: Date.now() } });
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
                    canonical(origin) !== canonical(completion)
                ) {
                    throw new AuditError("CONFLICT", "audit result differs from its attempt");
                }
            }
        }

        // extract query columns and retain the complete historical event
        const actor = actorKey(event.context.actor);
        const inserted = await transaction
            .insert(auditEvent)
            .values({
                id: event.id,
                attemptId: event.attemptId ?? null,
                action: event.action.name,
                packageId: event.action.package.id,
                accountId: event.context.accountId ?? null,
                spaceId: event.context.spaceId ?? null,
                hostId: event.context.hostId ?? null,
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
            if (existing && canonical(existing.event) === canonical(event)) {
                return;
            }
            throw new AuditError(
                "CONFLICT",
                existing
                    ? "audit event identifier has conflicting contents"
                    : "audit attempt already has a result",
            );
        }

        // index named object references without creating live foreign keys
        const targets = Object.entries(event.targets).map(([role, target]) => ({
            eventId: event.id,
            role,
            type: target.type,
            id: target.id,
        }));
        if (targets.length) {
            await transaction.insert(auditTarget).values(targets);
        }
    }

    /** Read one event only when it belongs to the requested collection. */
    async get(scope: AuditScope, id: AuditEvent["id"]) {
        const row = await this.database
            .select({ event: auditEvent.event, recordedAt: auditEvent.recordedAt })
            .from(auditEvent)
            .where(and(scopeFilter(scope), eq(auditEvent.id, id)))
            .get();
        if (!row) {
            throw new AuditError("NOT_FOUND", "audit event not found");
        }

        return row;
    }

    /** Read a bounded page in durable acceptance order. */
    async list(request: AuditQuery): Promise<AuditPage> {
        // parse the query and filter by collection
        const query = AuditQuery.parse(request);
        const filters: (SQL | undefined)[] = [scopeFilter(query.scope)];

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

        // find attempts whose result has not arrived
        if (query.unresolved) {
            const completed = this.database
                .select({ id: auditEvent.attemptId })
                .from(auditEvent)
                .where(isNotNull(auditEvent.attemptId));
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
                .select({ id: auditTarget.eventId })
                .from(auditTarget)
                .where(
                    and(
                        eq(auditTarget.type, query.target.type),
                        eq(auditTarget.id, query.target.id),
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
    async prune(scope: AuditScope, before: number, limit: number): Promise<number> {
        AuditQuery.parse({ scope, before, limit });

        return this.database.transaction(
            async (transaction) => {
                // bound retention work and cascade deletion to target indexes
                const expired = await transaction
                    .select({ id: auditEvent.id })
                    .from(auditEvent)
                    .where(and(scopeFilter(scope), lt(auditEvent.recordedAt, before)))
                    .orderBy(asc(auditEvent.recordedAt), asc(auditEvent.id))
                    .limit(limit);
                if (!expired.length) {
                    return 0;
                }
                const removed = await transaction
                    .delete(auditEvent)
                    .where(
                        inArray(
                            auditEvent.id,
                            expired.map((event) => event.id),
                        ),
                    )
                    .returning({ id: auditEvent.id });

                return removed.length;
            },
            { isolationLevel: "read committed" },
        );
    }

    /** Stream bounded pages accepted before the export started. */
    async *export(request: AuditQuery, signal?: AbortSignal) {
        const query = AuditQuery.parse({
            ...request,
            before:
                request.before === undefined ? Date.now() : Math.min(request.before, Date.now()),
        });
        while (true) {
            // keep memory bounded and stop before fetching another page on cancellation
            signal?.throwIfAborted();
            const page = await this.list(query);
            for (const record of page.items) {
                signal?.throwIfAborted();
                yield record;
            }
            if (!page.cursor) {
                return;
            }
            query.cursor = page.cursor;
        }
    }
}

/** Apply collection membership independently of the host's permission check. */
function scopeFilter(scope: AuditScope): SQL {
    switch (scope.type) {
        case "global":
            return and(
                isNull(auditEvent.accountId),
                isNull(auditEvent.spaceId),
                isNull(auditEvent.hostId),
            )!;
        case "account":
            return eq(auditEvent.accountId, scope.accountId);
        case "space":
            return eq(auditEvent.spaceId, scope.spaceId);
        case "host":
            return eq(auditEvent.hostId, scope.hostId);
    }
}

/** Hash validated JSON before taking the database writer lock. */
async function digest(content: string): Promise<string> {
    const bytes = new TextEncoder().encode(content);
    const hash = await crypto.subtle.digest("SHA-256", bytes);

    return new Uint8Array(hash).toHex();
}
