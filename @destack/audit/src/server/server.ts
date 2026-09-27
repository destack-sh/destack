import { auditExport, auditPrune } from "../history/action.ts";
import {
    implement,
    type ServiceAccess,
    type ServiceContext,
    type ServiceImplementation,
} from "@destack/service/server";
import { event } from "../history/access.ts";
import { ServiceError } from "@destack/service";
import { AuditHistory } from "../history/history.ts";
import { AuditRecorder } from "../record/index.ts";
import { AuditError } from "../error/index.ts";
import { auditService, AuditScope } from "../service/index.ts";
import { Authorizer, GLOBAL_SCOPE } from "@destack/access";

/** Verified request authority supplied by a local or regional host. */
export interface AuditRequestContext {
    /** Require the caller to hold a history permission on a scope's history. */
    authorizeAudit(permission: "ingest" | "read" | "prune", scope: AuditScope): Promise<void>;
    /** Record audit-history access under the authenticated request identity. */
    audit: Pick<AuditRecorder, "attempt" | "stream">;
}

/** Bind history storage to authorization on the scope whose history it is. */
export function implementService(
    auditHistory: AuditHistory,
    options: AuditServerOptions,
): ServiceImplementation {
    // give each call its recorder and a check of history permissions
    const implementation = implement(auditService.router)
        .$context<ServiceContext>()
        .use(async ({ context, next }) =>
            next({
                context: {
                    audit: await options.record(context),
                    authorizeAudit: async (
                        permission: "ingest" | "read" | "prune",
                        scope: AuditScope,
                    ) => {
                        // require the permission on the scope object, denying an unknown scope
                        const [link] = await Authorizer.chain(options.access.database, scope);
                        if (link === undefined) {
                            throw new ServiceError("FORBIDDEN", {
                                message: `permission denied: ${permission}`,
                            });
                        }
                        await context.authorization!.require(
                            event.permission(permission),
                            link.object,
                        );
                    },
                },
            }),
        );

    return {
        service: auditService,
        access: options.access,
        router: implementation.router({
            ingest: implementation.ingest.handler(async ({ input, context }) => {
                // accept events of a scope other than the global one, whose administrators authorize the producer
                const scope = input.event.context.scope;
                if (scope === GLOBAL_SCOPE) {
                    throw new ServiceError("BAD_REQUEST", {
                        message: "audit events name a scope other than the global one",
                    });
                }
                await context.authorizeAudit("ingest", scope);

                return auditHistory.ingest(input);
            }),
            export: implementation.export.handler(({ input, context, signal }) =>
                // recheck access to the scope's history before each bounded page it streams
                context.audit.stream(auditExport, history(input.scope), async function* () {
                    // require read access before the first page is read
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
                        events: await auditHistory.prune(input),
                    };
                }),
            ),
        }),
        clientInterceptors: [
            async ({ next }) => {
                try {
                    return await next();
                } catch (error) {
                    // report audit failures as the service failures they mean
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

/** The access and request recording the hosting installation supplies. */
export interface AuditServerOptions {
    /** The host's policies, which include the history policy and the scope objects histories belong to. */
    access: ServiceAccess;
    /** Create the durable recorder for history access under the verified caller. */
    record(
        context: ServiceContext,
    ): AuditRequestContext["audit"] | Promise<AuditRequestContext["audit"]>;
}

/** Name a scope's history as the target of an action on it. */
function history(scope: AuditScope) {
    return { targets: { scope: { type: "scope" as const, id: scope } }, details: {} };
}
