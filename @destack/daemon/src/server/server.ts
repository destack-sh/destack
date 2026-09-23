import { daemonPackage } from "../audit/index.ts";
import {
    implement,
    type ServiceContext,
    type ServiceImplementation,
} from "@destack/service/server";
import { daemonService } from "../service/index.ts";
import type { DaemonStore } from "../daemon/index.ts";
import { instanceRouter } from "./instance.ts";
import { deploymentRouter } from "./deployment.ts";
import { hostRouter } from "./host.ts";
import { implementCheckout } from "./checkout.ts";
import { previewRouter } from "./preview.ts";
import { loginRouter } from "./login.ts";
import * as audit from "@destack/audit/server";
import { ServiceError } from "@destack/service/error";
import { Host } from "../host/index.ts";
import { CheckoutStore } from "../checkout/index.ts";
import type { CredentialRegistry } from "../authentication/index.ts";

/** Host state retained while the daemon service runs. */
export interface DaemonOptions {
    /** Installed software version. */
    version: string;
    /** Daemon startup time. */
    started: string;
    /** Request graceful shutdown. */
    shutdown: () => void;
    /** End long-lived observations when daemon shutdown begins. */
    shutdownSignal: AbortSignal;
    /** Active restricted credentials issued by this daemon. */
    credentials: CredentialRegistry;
}

/** Implement local host administration and its audit-history procedures. */
export async function implementService(
    storage: DaemonStore,
    options: DaemonOptions,
): Promise<ServiceImplementation> {
    // compose domain operations over the prepared local database
    const host = await storage.host.get();
    const origin = {
        package: daemonPackage,
        service: "daemon",
        hostId: host.hostId,
        deviceId: host.deviceId ?? undefined,
    };
    const record = (context: ServiceContext) =>
        audit.createRecorder(context, storage.outbox, origin);
    const checkouts = new CheckoutStore(storage.database);
    const administration = new Host(storage.host, {
        version: options.version,
        started: options.started,
        pid: process.pid,
        host,
    });

    // expose only this host's audit history under its current authority
    const history = audit.implementService(storage.history, {
        record,
        authorize: async (access) => {
            if (
                access.action === "ingest" ||
                access.scope.type !== "host" ||
                access.scope.hostId !== host.hostId
            ) {
                throw new ServiceError("FORBIDDEN");
            }
        },
    });

    // use the shared request context directly in domain handlers
    const implementation = implement(daemonService).$context<ServiceContext>();

    return {
        ...history,
        target: async () => ({ scope: host.hostId }),
        audit: audit.createProcedureAudit(({ context }) => record(context)),
        router: {
            audit: history.router,
            ...implementation.router({
                credential: {
                    create: implementation.credential.create.handler(({ input, context }) =>
                        options.credentials.create(input, record(context)),
                    ),
                    revoke: implementation.credential.revoke.handler(async ({ input, context }) => {
                        await options.credentials.revoke(input.token, record(context));

                        return {};
                    }),
                },
                instance: instanceRouter,
                deployment: deploymentRouter,
                checkout: implementCheckout(checkouts, record),
                preview: previewRouter,
                login: loginRouter,
                status: implementation.status.handler(() => administration.status.value),
                watch: implementation.watch.handler(({ context }) =>
                    administration.status.watch(
                        AbortSignal.any([context.signal, options.shutdownSignal]),
                    ),
                ),
                stop: implementation.stop.handler(() => {
                    setTimeout(options.shutdown, 0);

                    return {};
                }),
                host: {
                    ...hostRouter,
                    rename: implementation.host.rename.handler(({ input, context }) =>
                        administration.rename(input.name, record(context)),
                    ),
                },
            }),
        },
    };
}
