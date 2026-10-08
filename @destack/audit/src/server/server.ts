import {
    implement,
    type ServiceContext,
    type ServiceImplementation,
} from "@destack/service/server";
import type { AuditHistory } from "../history/history.ts";
import { auditService } from "../service/index.ts";

/** Implement the audit service: take the ended calls admitted hosts' journals deliver into a history. */
export function implementAudit(options: AuditOptions): AuditImplementation {
    const implementation = implement(auditService.router).$context<ServiceContext>();

    return {
        service: auditService,
        history: options.history,
        router: implementation.router({
            ingest: implementation.ingest.handler(async ({ input, context }) => {
                // admit the sender for every scope the calls ran in
                context.requireAuthentication();
                const scopes = [
                    ...new Set(input.calls.map((each) => each.execution.context.scope)),
                ];
                await options.intake(context, scopes);

                return { stored: await options.history.ingest(input) };
            }),
        }),
    };
}

/** The history a host serves the audit service with, and who may deliver calls to it. */
export interface AuditOptions {
    /** The history the delivered calls are stored in. */
    readonly history: AuditHistory;
    /** Admit a caller delivering the ended calls of some scopes, such as another host's journal. */
    readonly intake: (context: ServiceContext, scopes: readonly string[]) => Promise<void>;
}

/** The audit service with the history it stores. */
export interface AuditImplementation extends ServiceImplementation {
    /** The history the delivered calls are stored in. */
    readonly history: AuditHistory;
}
