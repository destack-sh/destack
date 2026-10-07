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
import { canonicalize, type Identifier, schema } from "@destack/schema";
import type { PackageId } from "@destack/package";
import { AuditError } from "../error/index.ts";
import {
    AuditBatch,
    AuditPrune,
    AuditQuery,
    type AuditPage,
    type AuditScope,
} from "../service/query.ts";
import { auditCall, auditEnclosure, auditTarget } from "../record/table.ts";

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

    /** Store a batch of an instance's journal its host relays, recording the instance as each call's provenance. */
    async relay(value: unknown, provenance: AuditProvenance): Promise<number> {
        // parse the batch, refusing one no journal writes
        const parsed = AuditBatch.safeParse(value);
        if (!parsed.success) {
            throw new AuditError("INVALID_EVENT", "invalid journal batch", { cause: parsed.error });
        }

        // refuse calls of another space or package, and calls naming a provenance of their own
        const batch = parsed.data;
        for (const call of batch.calls) {
            AuditHistory.#requireOrigin(call, provenance);
        }

        // record the installation and the instance on each call
        const { installationId, instanceId } = provenance;
        const calls = batch.calls.map((call) => {
            const context = { ...call.execution.context, installationId, instanceId };

            return { ...call, execution: { ...call.execution, context } };
        });

        return this.ingest({ calls });
    }

    /** Require a relayed call to run in the instance's space and package, naming no provenance, which only its host records. */
    static #requireOrigin(call: AuditCall, provenance: AuditProvenance): void {
        // refuse a call of another space or package
        const { context } = call.execution;
        if (context.scope !== provenance.scope || context.package.id !== provenance.packageId) {
            throw new AuditError(
                "FORBIDDEN",
                `installation ${provenance.installationId} records no calls of ${context.package.id} in ${context.scope}`,
            );
        }

        // refuse a call naming where it ran
        const isProvenanced =
            context.installationId !== undefined ||
            context.instanceId !== undefined ||
            context.machineId !== undefined;
        if (isProvenanced) {
            throw new AuditError("FORBIDDEN", "only the host records a call's provenance");
        }
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

        // index the named targets and the enclosing scopes of a new call
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
            const enclosures = (execution.context.chain ?? []).map((enclosing) => ({
                id: schema.identifier("audit-enclosure").parse(`audit-enclosure-${v7()}`),
                callId: execution.id,
                scope: enclosing,
            }));
            if (enclosures.length > 0) {
                await transaction.insert(auditEnclosure).values(enclosures);
            }

            return;
        }

        // accept a repeat, or the outcome of a running call with the same origin
        await AuditHistory.#settle(call, transaction);
    }

    /** Accept a call conflicting with a stored one as a repeat, or as the outcome of the running call it stores. */
    static async #settle(call: AuditCall, transaction: DatabaseConnection): Promise<void> {
        // read the stored call
        const { execution } = call;
        const [existing] = await transaction
            .select({ call: auditCall.call })
            .from(auditCall)
            .where(eq(auditCall.id, execution.id));
        if (existing === undefined) {
            throw new TypeError(`audited call ${execution.id} conflicted but is missing`);
        }

        // refuse other contents, and record an outcome
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
        // parse the query
        const query = AuditQuery.parse(request);
        const filters = this.#filters(query);

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

    /** Match the calls a query selects: its scope or the scopes inside it, method, package, actor, category, outcome, time range, cursor and target. */
    #filters(query: AuditQuery): (SQL | undefined)[] {
        // match the call's columns the query names
        const { cursor, target } = query;
        const columns = [
            query.within === true
                ? or(
                      eq(auditCall.scope, query.scope),
                      inArray(
                          auditCall.id,
                          this.database
                              .select({ id: auditEnclosure.callId })
                              .from(auditEnclosure)
                              .where(eq(auditEnclosure.scope, query.scope)),
                      ),
                  )
                : eq(auditCall.scope, query.scope),
            query.method === undefined ? undefined : eq(auditCall.method, query.method),
            query.packageId === undefined ? undefined : eq(auditCall.packageId, query.packageId),
            query.actor === undefined ? undefined : eq(auditCall.actor, actorKey(query.actor)),
            query.category === undefined ? undefined : eq(auditCall.category, query.category),
            query.outcome === undefined ? undefined : eq(auditCall.outcome, query.outcome),
            query.isRunning === true ? isNull(auditCall.outcome) : undefined,
        ];

        // match the acceptance times from the start, before the end, and after the cursor
        const times = [
            query.from === undefined ? undefined : gte(auditCall.recordedAt, query.from),
            query.before === undefined ? undefined : lt(auditCall.recordedAt, query.before),
            cursor === undefined || cursor === null
                ? undefined
                : or(
                      gt(auditCall.recordedAt, cursor.recordedAt),
                      and(eq(auditCall.recordedAt, cursor.recordedAt), gt(auditCall.id, cursor.id)),
                  ),
        ];

        // match the calls naming the target
        const targeted =
            target === undefined || target === null
                ? undefined
                : inArray(
                      auditCall.id,
                      this.database
                          .select({ id: auditTarget.callId })
                          .from(auditTarget)
                          .where(
                              and(
                                  eq(auditTarget.type, target.type),
                                  eq(auditTarget.objectId, target.id),
                              ),
                          ),
                  );

        return [...columns, ...times, targeted];
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

/** The instance whose journal a host relays, as the host verified it from the instance's secret. */
export interface AuditProvenance {
    /** The space the instance serves. */
    readonly scope: Identifier<"space">;
    /** The package the instance runs. */
    readonly packageId: PackageId;
    /** The installation the instance runs. */
    readonly installationId: Identifier<"installation">;
    /** The instance. */
    readonly instanceId: Identifier<"instance">;
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
