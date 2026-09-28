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
import { AuditBatch } from "../outbox/delivery.ts";
import { AuditError } from "../error/index.ts";
import { AuditPrune, AuditQuery, type AuditPage, type AuditScope } from "./query.ts";
import { auditEvent, auditTarget } from "./stack/index.ts";

/** The stored audit events of each scope. */
export class AuditHistory {
    /** The history database. */
    readonly database: DatabaseConnection;

    /** Bind the history to its database. */
    constructor(database: DatabaseConnection) {
        this.database = database;
    }

    /** Store a batch's events in one transaction, each once, returning how many it holds. */
    async ingest(value: AuditBatch): Promise<number> {
        // parse the batch and insert each event
        const batch = AuditBatch.parse(value);
        const events = batch.events.map((event) => encodeEvent(event).event);
        await this.database.transaction(
            async (transaction) => {
                for (const event of events) {
                    await this.#insert(event, transaction);
                }
            },
            { isolationLevel: "read committed" },
        );

        return events.length;
    }

    /** Insert one event. */
    async #insert(event: AuditEvent, transaction: DatabaseConnection): Promise<void> {
        // reject a second outcome of an attempt
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

        // insert the event with its query columns
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

        // resolve a conflict against the stored event
        if (!inserted.length) {
            const existing = await transaction
                .select({ event: auditEvent.event })
                .from(auditEvent)
                .where(eq(auditEvent.id, event.id))
                .get();
            if (existing && canonicalize(existing.event) === canonicalize(event)) {
                return;
            }

            // reject a changed event or a second result
            throw new AuditError(
                "CONFLICT",
                existing
                    ? "audit event identifier has conflicting contents"
                    : "audit attempt already has a result",
            );
        }

        // index the named targets
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

    /** Read one page in acceptance order. */
    async list(request: AuditQuery): Promise<AuditPage> {
        // parse the query and filter by scope
        const query = AuditQuery.parse(request);
        const filters: (SQL | undefined)[] = [eq(auditEvent.scope, query.scope)];

        // filter by action and package
        if (query.action) {
            filters.push(eq(auditEvent.action, query.action));
        }
        if (query.packageId) {
            filters.push(eq(auditEvent.packageId, query.packageId));
        }

        // filter by actor or attempt
        if (query.actor) {
            filters.push(eq(auditEvent.actor, actorKey(query.actor)));
        }
        if (query.attemptId) {
            filters.push(
                or(eq(auditEvent.id, query.attemptId), eq(auditEvent.attemptId, query.attemptId)),
            );
        }

        // filter by outcome
        if (query.outcome) {
            filters.push(eq(auditEvent.outcome, query.outcome));
        }

        // find attempts without a result
        if (query.unresolved) {
            const completed = this.database
                .select({ id: auditEvent.attemptId })
                .from(auditEvent)
                .where(and(eq(auditEvent.scope, query.scope), isNotNull(auditEvent.attemptId)));
            filters.push(eq(auditEvent.stage, "attempt"), notInArray(auditEvent.id, completed));
        }

        // filter by acceptance time
        if (query.from !== undefined) {
            filters.push(gte(auditEvent.recordedAt, query.from));
        }
        if (query.before !== undefined) {
            filters.push(lt(auditEvent.recordedAt, query.before));
        }

        // continue after the cursor
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

        // filter by target
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

        // fetch one extra record to detect the last page
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

    /** Remove a scope's oldest events accepted before a time, returning how many. */
    async prune(request: AuditPrune): Promise<number> {
        // parse the request
        const { scope, before, limit } = AuditPrune.parse(request);

        // remove the oldest expired events with their targets
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

    /** Stream the pages of events accepted before the export started. */
    async *export(request: AuditQuery, signal?: AbortSignal): AsyncGenerator<AuditPage["items"]> {
        // fix the end of the export
        const query = AuditQuery.parse({
            ...request,
            before:
                request.before === undefined ? Date.now() : Math.min(request.before, Date.now()),
        });
        while (true) {
            // stop on cancellation
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

/** Encode an actor as its query key. */
function actorKey(actor: AuditActor): string {
    // key a subject by its access key
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
