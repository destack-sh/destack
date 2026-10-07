import { auditExport, auditList, auditPrune } from "../history/action.ts";
import { Scope } from "@destack/sync";
import { Snapshot } from "@destack/db";
import {
    implement,
    type ServiceAccess,
    type ServiceContext,
    type ServiceImplementation,
} from "@destack/service/server";
import { call } from "../record/access.ts";
import { ServiceError } from "@destack/service";
import { AuditHistory } from "../history/history.ts";
import { AuditRecorder } from "./recorder.ts";
import { auditService, AuditScope } from "../service/index.ts";

/** The request authority a host supplies. */
export interface AuditRequestContext {
    /** Require a history permission on a scope. */
    authorizeAudit(permission: AuditPermission, scope: AuditScope): Promise<void>;
    /** The recorder of history access. */
    audit: Pick<AuditRecorder, "attempt" | "read" | "record" | "stream">;
}

/** Implement the audit service on a history. */
export function implementAudit(options: AuditOptions): AuditImplementation {
    // add a recorder and a permission check to each call
    const implementation = implement(auditService.router)
        .$context<ServiceContext>()
        .use(async ({ context, next }) =>
            next({
                context: {
                    audit: await options.record(context),
                    authorizeAudit: (permission: AuditPermission, scope: AuditScope) =>
                        authorizeAudit(options.access, context, permission, scope),
                },
            }),
        );

    // serve the history
    const auditHistory = options.history;

    return {
        service: auditService,
        history: auditHistory,
        access: options.access,
        audit: AuditRecorder.procedure(({ context }) => options.record(context)),
        router: implementation.router({
            list: implementation.list.handler(({ input, context }) =>
                context.audit.read(auditList, history(input.scope), async () => {
                    await context.authorizeAudit("read", input.scope);

                    return auditHistory.list(input);
                }),
            ),
            export: implementation.export.handler(({ input, context, signal }) =>
                // recheck read access before each page
                context.audit.stream(auditExport, history(input.scope), async function* () {
                    // require read access before the first page
                    await context.authorizeAudit("read", input.scope);
                    for await (const page of auditHistory.export(input, signal)) {
                        await context.authorizeAudit("read", input.scope);
                        for (const record of page) {
                            signal?.throwIfAborted();
                            yield record;
                        }
                    }
                }),
            ),
            prune: implementation.prune.handler(({ input, context }) =>
                context.audit.attempt(auditPrune, history(input.scope), async () => {
                    await context.authorizeAudit("prune", input.scope);

                    return {
                        calls: await auditHistory.prune(input),
                    };
                }),
            ),
        }),
    };
}

/** A history permission of a scope. */
type AuditPermission = "read" | "prune";

/** Require a history permission on a scope's object. */
async function authorizeAudit(
    access: ServiceAccess,
    context: ServiceContext,
    permission: AuditPermission,
    scope: AuditScope,
): Promise<void> {
    // find the scope object
    const [link] = await Scope.chain(Snapshot.live(access.database), scope);
    if (link === undefined) {
        throw new ServiceError("FORBIDDEN", { message: `permission denied: ${permission}` });
    }

    // require the permission on it
    await context.requireAuthorization().require(call.permission(permission), link.object);
}

/** The history, access and recording a host serves the audit service with. */
export interface AuditOptions {
    /** The history the service stores and reads. */
    readonly history: AuditHistory;
    /** The host's policies. */
    access: ServiceAccess;
    /** Create the recorder of history access. */
    record(
        context: ServiceContext,
    ): AuditRequestContext["audit"] | Promise<AuditRequestContext["audit"]>;
}

/** The audit service with the history it serves. */
export interface AuditImplementation extends ServiceImplementation {
    /** The history the service stores and reads. */
    readonly history: AuditHistory;
}

/** Name a scope's history as a target. */
function history(scope: AuditScope) {
    return { targets: { scope: { type: "scope" as const, id: scope } }, details: {} };
}
