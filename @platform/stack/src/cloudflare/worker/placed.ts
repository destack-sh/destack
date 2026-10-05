import {
    connect,
    DirectoryClient,
    WorkloadIdentity,
    workloadIdentity,
} from "@destack/account/client";
import { accountService } from "@destack/account/service";
import { HostIdentity } from "@destack/host/identity";
import { MemoryKeychain } from "@destack/host/keychain";
import type { ResourceContext } from "@destack/resource/context";
import { schema } from "@destack/schema";
import { ServiceMount } from "@destack/service";
import type { Alarm } from "@destack/service/control";
import { Scope } from "@destack/sync";
import { PlatformAuthentication } from "../../authentication/index.ts";
import type { Platform } from "../../placement/index.ts";
import { type ProcessEnvironment, WorkerProcess } from "./process.ts";

/** A placed process's bindings to the account process, its origin, its placement and its region's host. */
export interface PlacedEnvironment extends ProcessEnvironment {
    /** The account process's Worker, through a service binding. */
    readonly ACCOUNT: { fetch(request: Request): Promise<Response> };
    /** The origin the process answers at. */
    readonly DESTACK_ORIGIN: string;
    /** The placement the process's workload runs as. */
    readonly DESTACK_PLACEMENT_ID: string;
    /** The region's host. */
    readonly DESTACK_HOST_ID: string;
    /** The region's host's private key, a P-256 JSON Web Key. */
    readonly DESTACK_HOST_KEY: string;
}

/** A residency process's bindings: the region whose origin it publishes. */
export interface ResidencyEnvironment extends PlacedEnvironment {
    /** The region the process serves at its origin. */
    readonly DESTACK_REGION_ID: string;
}

/** A placed process on workerd: its workload as the placement the region's host represents, following the account service. */
export const PlacedProcess = {
    /** Start a workload as its placement over its database and configuration, publishing its region's origin first for a residency process. */
    async start(
        environment: PlacedEnvironment | ResidencyEnvironment,
        platform: Pick<Platform, "workload" | "database">,
        configuration: ResourceContext,
        alarm: Alarm,
    ): Promise<WorkerProcess> {
        // prove the region's host with the key the operator enrolled
        const hostId = schema.identifier("host").parse(environment.DESTACK_HOST_ID);
        const keychain = new MemoryKeychain();
        await keychain.save(hostId, environment.DESTACK_HOST_KEY);
        const host = new HostIdentity(hostId, keychain);
        const accounts = ServiceMount.url(environment.DESTACK_ISSUER, accountService.package.id);
        const send = (request: Request) => environment.ACCOUNT.fetch(request);

        // publish a region's origin as its host, the way a cell publishes its endpoint
        if ("DESTACK_REGION_ID" in environment) {
            const hostClient = connect({
                url: accounts,
                fetch: host.fetch(send, accountService.package.id, accounts),
            });
            await new DirectoryClient(hostClient).publish(
                schema.identifier("region").parse(environment.DESTACK_REGION_ID),
                Scope.universe.id,
                environment.DESTACK_ORIGIN,
            );
        }

        // run the workload as its placement, calling the account service with the placement's tokens
        const placementId = schema.identifier("placement").parse(environment.DESTACK_PLACEMENT_ID);
        const identity = new WorkloadIdentity(
            placementId,
            connect({
                url: accounts,
                fetch: host.fetch(send, accountService.package.id, accounts, { placementId }),
            }),
        );

        // serve it over its database, verifying callers against its copies and by introspection
        const database = await WorkerProcess.database(environment, platform.database);
        const authentication = new PlatformAuthentication(environment.DESTACK_ISSUER, send);

        return WorkerProcess.start(platform.workload, {
            resources: configuration
                .bind(platform.database, database)
                .bind(workloadIdentity, identity),
            callKey: WorkerProcess.callKey(environment),
            alarm,
            authenticate: (request, audience) =>
                authentication.workload(request, audience, identity, database),
        });
    },
};
