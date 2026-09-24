import { auditAction } from "../history/action.ts";
import { schema } from "@destack/schema";
import {
    implement,
    type ServiceContext,
    type ServiceImplementation,
} from "@destack/service/server";
import { ServiceError } from "@destack/service";
import { AuditHistory } from "../history/history.ts";
import { AuditRecorder } from "../record/index.ts";
import { AuditEvent, type AuditResult } from "../event/index.ts";
import { AuditError } from "../error/index.ts";
import { auditService, AuditScope } from "../service/index.ts";
import { AuditEntry } from "../outbox/delivery.ts";

/** Select the collection without decoding operation-specific fields. */
const historySelection = schema.object({ scope: AuditScope }).strip();

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

/** Bind history storage to authenticated producer and collection authorization. */
export function implementService(
    history: AuditHistory,
    options: AuditServerOptions,
): ServiceImplementation {
    const implementation = implement(auditService.router)
        .$context<ServiceContext>()
        .use(async ({ context, next }) => {
            return next({
                context: {
                    audit: await options.record(context),
                    authorizeAudit: (access: AuditAccess) => options.authorize(access, context),
                },
            });
        });

    return {
        service: auditService,
        target: async (call) => {
            // qualify credential restrictions by the selected history collection
            if (call.path.at(-1) === "ingest") {
                const selected = AuditEntry.safeParse(call.input);
                if (!selected.success) {
                    throw new ServiceError("BAD_REQUEST");
                }
                const { context } = selected.data.event;

                return {
                    scope: context.spaceId ?? context.accountId ?? context.hostId ?? "global",
                };
            }
            const selected = historySelection.safeParse(call.input);
            if (!selected.success) {
                throw new ServiceError("BAD_REQUEST");
            }

            return { scope: scopeId(selected.data.scope) };
        },
        router: implementation.router({
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
                // open the attempt and assume cancellation until the stream finishes
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
        authorize: async ({ context }) => {
            context.requireCaller();
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
    };
}

/** Domain authority and request recording supplied by the hosting installation. */
export interface AuditServerOptions {
    /** Verify producer attestation or current collection permissions for the caller. */
    authorize(access: AuditAccess, context: ServiceContext): Promise<void>;
    /** Create the durable recorder for history access under the verified caller. */
    record(
        context: ServiceContext,
    ): AuditRequestContext["audit"] | Promise<AuditRequestContext["audit"]>;
}

/** Record access and its result before returning history to the caller. */
async function access<Value>(
    context: AuditRequestContext,
    operation: "get" | "list" | "export" | "prune",
    scope: AuditScope,
    execute: () => Promise<Value>,
): Promise<Value> {
    // persist the attempt before running the operation
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
    // persist the attempt and complete it as failed when authorization fails
    const id = scopeId(scope);
    const attempt = context.audit.begin(auditAction[operation], {
        targets: { collection: { type: scope.type, id } },
        details: {},
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

/** Identify the authority administering a history collection. */
function scopeId(scope: AuditScope): string {
    switch (scope.type) {
        case "space":
            return scope.spaceId;
        case "account":
            return scope.accountId;
        case "host":
            return scope.hostId;
        case "global":
            return "global";
    }
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
