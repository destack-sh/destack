import { AuditActor, AuditCaller } from "../record/actor.ts";
import { AuditCall } from "../record/call.ts";
import { v7 } from "uuid";
import {
    and,
    or,
    eq,
    gt,
    lt,
    gte,
    inArray,
    isNull,
    asc,
    type DatabaseConnection,
    type SQL,
} from "@destack/db";
import { Subject } from "@destack/sync";
import { canonicalize, schema } from "@destack/schema";
import { AuditError } from "../error/index.ts";
import { AuditBatch, AuditPrune, AuditQuery, type AuditPage, type AuditScope } from "./query.ts";
import { auditCall, auditTarget } from "../stack/index.ts";

/** The audited calls of each scope. */
export class AuditHistory {
    /** The history database. */
    readonly database: DatabaseConnection;

    /** Bind the history to its database. */
    constructor(database: DatabaseConnection) {
        this.database = database;
    }

    /** Store a batch's calls once in one transaction and return the stored count. */
    async ingest(value: AuditBatch): Promise<number> {
        // parse the batch, keeping no result values
        const batch = AuditBatch.parse(value);
        const calls = batch.calls.map((call) => AuditHistory.kept(call));
        await this.database.transaction(
            async (transaction) => {
                for (const call of calls) {
                    await this.#upsert(call, transaction);
                }
            },
            { isolationLevel: "read committed" },
        );

        return calls.length;
    }

    /** Insert a call, or the outcome of a running one. */
    async #upsert(call: AuditCall, transaction: DatabaseConnection): Promise<void> {
        // insert the call with its query columns, once
        const execution = call.execution;
        const scope = execution.context.scope;
        const inserted = await transaction
            .insert(auditCall)
            .values({
                id: execution.id,
                scope,
                method: call.method,
                packageId: execution.context.package.id,
                actor: actorKey(AuditCaller.actor(execution.context.caller)),
                category: execution.category,
                outcome: execution.outcome?.kind ?? null,
                startedAt: execution.startedAt,
                recordedAt: Date.now(),
                call,
            })
            .onConflictDoNothing()
            .returning({ id: auditCall.id });

        // index the named targets of a new call
        if (inserted.length > 0) {
            const targets = Object.entries(execution.targets).map(([role, target]) => ({
                id: schema.identifier("audit-target").parse(`audit-target-${v7()}`),
                callId: execution.id,
                scope,
                role,
                type: target.type,
                objectId: target.id,
            }));
            if (targets.length) {
                await transaction.insert(auditTarget).values(targets);
            }

            return;
        }

        // accept a repeat, or the outcome of a running call with the same origin
        const [existing] = await transaction
            .select({ call: auditCall.call })
            .from(auditCall)
            .where(eq(auditCall.id, execution.id));
        if (existing === undefined) {
            throw new TypeError(`audited call ${execution.id} conflicted but is missing`);
        }
        const isRepeat = canonicalize(existing.call) === canonicalize(call);
        const isOutcome =
            existing.call.execution.outcome === undefined &&
            canonicalize(AuditHistory.#origin(existing.call)) ===
                canonicalize(AuditHistory.#origin(call));
        if (!isRepeat && !isOutcome) {
            throw new AuditError("CONFLICT", "audited call has conflicting contents");
        }
        if (isOutcome) {
            await transaction
                .update(auditCall)
                .set({ outcome: execution.outcome?.kind ?? null, call })
                .where(eq(auditCall.id, execution.id));
        }
    }

    /** Copy a call without its input and result value. */
    static kept(value: AuditCall): AuditCall {
        // drop the input and a success's value
        const call = AuditCall.parse(value);
        const outcome = call.execution.outcome;
        const kept = outcome?.kind === "success" ? { kind: "success" as const } : outcome;

        return {
            ...call,
            input: {},
            execution: {
                ...call.execution,
                ...(kept === undefined ? {} : { outcome: kept }),
            },
        };
    }

    /** Read what a call's outcome may not change: everything but how and when it ended. */
    static #origin(call: AuditCall) {
        const {
            outcome: _outcome,
            finishedAt: _finishedAt,
            details: _details,
            ...origin
        } = call.execution;

        return { ...call, execution: origin };
    }

    /** Read one call of a scope's history. */
    async get(scope: AuditScope, id: string) {
        // read the call within its scope
        const row = await this.database
            .select({ call: auditCall.call, recordedAt: auditCall.recordedAt })
            .from(auditCall)
            .where(
                and(
                    eq(auditCall.scope, scope),
                    eq(auditCall.id, schema.identifier("call").parse(id)),
                ),
            )
            .get();
        if (!row) {
            throw new AuditError("NOT_FOUND", "audited call not found");
        }

        return row;
    }

    /** Read one page in acceptance order. */
    async list(request: AuditQuery): Promise<AuditPage> {
        // parse the query and filter by scope
        const query = AuditQuery.parse(request);
        const filters: (SQL | undefined)[] = [eq(auditCall.scope, query.scope)];

        // filter by method and package
        if (query.method !== undefined) {
            filters.push(eq(auditCall.method, query.method));
        }
        if (query.packageId !== undefined) {
            filters.push(eq(auditCall.packageId, query.packageId));
        }

        // filter by actor
        if (query.actor !== undefined) {
            filters.push(eq(auditCall.actor, actorKey(query.actor)));
        }

        // filter by category
        if (query.category !== undefined) {
            filters.push(eq(auditCall.category, query.category));
        }

        // filter by outcome
        if (query.outcome !== undefined) {
            filters.push(eq(auditCall.outcome, query.outcome));
        }

        // find calls still running
        if (query.isRunning === true) {
            filters.push(isNull(auditCall.outcome));
        }

        // filter by acceptance time
        if (query.from !== undefined) {
            filters.push(gte(auditCall.recordedAt, query.from));
        }
        if (query.before !== undefined) {
            filters.push(lt(auditCall.recordedAt, query.before));
        }

        // continue after the cursor
        if (query.cursor) {
            filters.push(
                or(
                    gt(auditCall.recordedAt, query.cursor.recordedAt),
                    and(
                        eq(auditCall.recordedAt, query.cursor.recordedAt),
                        gt(auditCall.id, query.cursor.id),
                    ),
                ),
            );
        }

        // filter by target
        if (query.target) {
            const targets = this.database
                .select({ id: auditTarget.callId })
                .from(auditTarget)
                .where(
                    and(
                        eq(auditTarget.type, query.target.type),
                        eq(auditTarget.objectId, query.target.id),
                    ),
                );
            filters.push(inArray(auditCall.id, targets));
        }

        // fetch one extra record to detect the last page
        const rows = await this.database
            .select({ call: auditCall.call, recordedAt: auditCall.recordedAt })
            .from(auditCall)
            .where(and(...filters))
            .orderBy(asc(auditCall.recordedAt), asc(auditCall.id))
            .limit(query.limit + 1);
        const items = rows.slice(0, query.limit);
        const last = items.at(-1);

        return {
            items,
            cursor:
                rows.length > query.limit && last
                    ? { recordedAt: last.recordedAt, id: last.call.execution.id }
                    : null,
        };
    }

    /** Remove a scope's oldest calls accepted before a time, returning how many. */
    async prune(request: AuditPrune): Promise<number> {
        // parse the request
        const { scope, before, limit } = AuditPrune.parse(request);

        // remove the oldest expired calls with their targets
        const expired = this.database
            .select({ id: auditCall.id })
            .from(auditCall)
            .where(and(eq(auditCall.scope, scope), lt(auditCall.recordedAt, before)))
            .orderBy(asc(auditCall.recordedAt), asc(auditCall.id))
            .limit(limit);
        const removed = await this.database
            .delete(auditCall)
            .where(inArray(auditCall.id, expired))
            .returning({ id: auditCall.id });

        return removed.length;
    }

    /** Stream the pages of calls accepted before the export started. */
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
        return Subject.key(actor.subject);
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
