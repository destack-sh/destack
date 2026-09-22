import { schema } from "@destack/schema";
import { implement, ServiceHandler } from "@destack/service/server";
import { ServiceError } from "@destack/service";
import type { Health } from "@destack/service/health";
import { AuditHistory } from "../history/history.ts";
import { AuditRecorder } from "../record/index.ts";
import { AuditEvent, type AuditResult } from "../event/index.ts";
import { defineAuditAction } from "../action/index.ts";
import { AuditError } from "../error/index.ts";
import { auditService, AuditScope } from "../service/index.ts";
import manifest from "../../package.json" with { type: "json" };
import definition from "../../destack.json" with { type: "json" };
import { PackageId } from "@destack/package";

/** Audited history access without copying filters or event contents into diagnostics. */
export const accessAudit = defineAuditAction({
    package: { id: PackageId.parse(definition.id), name: manifest.name, version: manifest.version },
    name: "audit.access",
    version: 1,
    targets: schema.record(
        schema.string(),
        schema.object({ type: schema.string(), id: schema.string() }),
    ),
    details: schema.object({ operation: schema.enum(["get", "list", "export", "prune"]) }),
});

/** Authorization requests interpreted by the hosting account and residency policy. */
export type AuditAccess =
    | { action: "ingest"; producerId: string; event: AuditEvent }
    | { action: "get" | "list" | "export" | "prune"; scope: AuditScope };

/** Verified request authority supplied by a local or regional host. */
export interface AuditRequestContext {
    /** Verify producer attestation on ingestion and collection permissions on reads and retention. */
    authorizeAudit(access: AuditAccess): Promise<void>;
    /** Record audit-history access under the authenticated request identity. */
    audit: Pick<AuditRecorder, "begin" | "complete" | "append">;
}

/** Host the shared service with mandatory producer and collection authorization. */
export function createAuditHandler(history: AuditHistory, health: Health) {
    const implementation = implement(auditService).$context<AuditRequestContext>();

    return new ServiceHandler(
        implementation.router({
            ingest: implementation.ingest.handler(async ({ input, context }) => {
                await context.authorizeAudit({
                    action: "ingest",
                    producerId: input.producerId,
                    event: input.event,
                });

                return history.ingest(input);
            }),
            get: implementation.get.handler(async ({ input, context }) => {
                return access(context, "get", input.scope, () =>
                    history.get(input.scope, input.id),
                );
            }),
            list: implementation.list.handler(async ({ input, context }) => {
                return access(context, "list", input.scope, () => history.list(input));
            }),
            export: implementation.export.handler(async function* ({ input, context, signal }) {
                const attempt = await beginAccess(context, "export", input.scope);
                let result: AuditResult = { outcome: "cancelled", errorCode: "CANCELLED" };
                let failureCause: unknown;
                try {
                    // recheck collection access while streaming bounded history pages
                    for await (const record of history.export(input, signal)) {
                        await context.authorizeAudit({ action: "export", scope: input.scope });
                        yield record;
                    }
                    result = { outcome: "success" };
                } catch (error) {
                    failureCause = error;
                    result = failure(error);
                    throw error;
                } finally {
                    await completeAccess(context, attempt, result, failureCause);
                }
            }),
            prune: implementation.prune.handler(async ({ input, context }) => {
                return access(context, "prune", input.scope, async () => ({
                    events: await history.prune(input.scope, input.before, input.limit),
                }));
            }),
        }),
        {
            health,
            authorize: async ({ context }) => {
                // validate authority before handing a request to any implementation
                if (typeof context.authorizeAudit !== "function" || !context.audit) {
                    throw new ServiceError("UNAUTHORIZED");
                }
            },
            clientInterceptors: [
                async ({ next }) => {
                    try {
                        return await next();
                    } catch (error) {
                        if (error instanceof AuditError) {
                            throw new ServiceError(
                                error.code === "INVALID_EVENT" ? "BAD_REQUEST" : error.code,
                                { message: error.message },
                            );
                        }
                        throw error;
                    }
                },
            ],
        },
    );
}

/** Record access and its result before returning history to the caller. */
async function access<Value>(
    context: AuditRequestContext,
    operation: "get" | "list" | "export" | "prune",
    scope: AuditScope,
    execute: () => Promise<Value>,
): Promise<Value> {
    const attempt = await beginAccess(context, operation, scope);
    let value: Value;

    // report application failure independently of audit completion failure
    try {
        value = await execute();
    } catch (error) {
        await completeAccess(context, attempt, failure(error), error);
        throw error;
    }
    await context.audit.append(context.audit.complete(attempt, { outcome: "success" }));

    return value;
}

/** Persist the request before checking the caller's collection permissions. */
async function beginAccess(
    context: AuditRequestContext,
    operation: "get" | "list" | "export" | "prune",
    scope: AuditScope,
): Promise<AuditEvent> {
    const id =
        scope.type === "space"
            ? scope.spaceId
            : scope.type === "account"
              ? scope.accountId
              : scope.type === "host"
                ? scope.hostId
                : "global";
    const attempt = context.audit.begin(accessAudit, {
        targets: { collection: { type: scope.type, id } },
        details: { operation },
    });
    await context.audit.append(attempt);
    try {
        await context.authorizeAudit({ action: operation, scope });
    } catch (error) {
        await completeAccess(context, attempt, failure(error), error);
        throw error;
    }

    return attempt;
}

/** Retain safe error codes and distinguish rejected access from execution failure. */
function failure(error: unknown): AuditResult {
    const errorCode =
        error instanceof ServiceError || error instanceof AuditError
            ? error.code
            : "INTERNAL_SERVER_ERROR";
    const outcome =
        errorCode === "FORBIDDEN" || errorCode === "UNAUTHORIZED" ? "denied" : "failure";

    return { outcome, errorCode };
}

/** Preserve an operation failure when recording its outcome also fails. */
async function completeAccess(
    context: AuditRequestContext,
    attempt: AuditEvent,
    result: AuditResult,
    cause?: unknown,
): Promise<void> {
    try {
        await context.audit.append(context.audit.complete(attempt, result));
    } catch (error) {
        if (cause !== undefined) {
            throw new AggregateError([cause, error], "audit access and result recording failed");
        }
        throw error;
    }
}
