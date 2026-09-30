import type { AuditHistory } from "@destack/audit/history";
import * as audit from "@destack/audit/server";
import { auditService } from "@destack/audit/service";
import type { PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import type { Caller } from "@destack/service/authentication";
import { Health } from "@destack/service/health";
import { type ProcedureCall, Server, type ServiceContext } from "@destack/service/server";
import { Scope } from "@destack/sync";
import { spaceService } from "../service/service.ts";
import { implementService, type SpaceServiceOptions } from "./server.ts";

/** How long a cell's services drain accepted requests when it stops, in milliseconds. */
const DRAIN_TIMEOUT_MILLISECONDS = 10000;

/** What a cell serves its spaces with: the space service's options, the audit history, and how requests authenticate. */
export interface CellOptions extends SpaceServiceOptions {
    /** The audit history the spaces' events go to. */
    readonly history: AuditHistory;
    /** Authenticate a request to a service the cell serves. */
    authenticate(request: Request, audience: PackageId): Promise<Caller>;
    /** Refuse host procedures to clients. */
    authorize(call: ProcedureCall<ServiceContext>): Promise<void>;
}

/** The servers of a running cell: the space service and the audit history of its spaces, over one cell database. */
export class CellServer {
    /** The space service. */
    readonly space: Server;
    /** The audit service of the cell's spaces, decided by the space's policies. */
    readonly audit: Server;

    /** Keep a cell's running services. */
    private constructor(space: Server, audits: Server) {
        this.space = space;
        this.audit = audits;
    }

    /** Start a cell's services over its database. */
    static start(options: CellOptions): CellServer {
        // serve the spaces
        const implementation = implementService(options);
        const serve = (audience: PackageId, name: string) => ({
            audience,
            drainTimeout: DRAIN_TIMEOUT_MILLISECONDS,
            authenticate: (request: Request) => options.authenticate(request, audience),
            authorizeHost: (call: ProcedureCall<ServiceContext>) => options.authorize(call),
            health: new Health(name),
            resources: new ResourceContext(),
        });
        const space = Server.start({
            ...implementation,
            ...serve(spaceService.package.id, spaceService.name),
        });

        // serve the spaces' audit history, decided by space's policies, beside it
        const audits = Server.start({
            ...audit.implementService(options.history, {
                access: implementation.access!,
                record: (context) => options.audit(context.scope ?? Scope.universe.id, context),
            }),
            ...serve(auditService.package.id, auditService.name),
        });

        return new CellServer(space, audits);
    }

    /** Stop the cell's services, draining accepted requests. */
    async close(): Promise<void> {
        await Promise.all([this.space.close(), this.audit.close()]);
    }
}
