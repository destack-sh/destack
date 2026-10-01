import { Snapshot } from "@destack/db/log";
import { type ReplicaRequest, type ReplicaSource, Scope } from "@destack/sync";
import type { AuditRecorder } from "@destack/audit";
import type { DatabaseConnection } from "@destack/db";
import type { Directory } from "@destack/directory";
import type { ObjectType } from "@destack/object";
import { ObjectServer } from "@destack/object/server";
import type { PackageId } from "@destack/package";
import type { ResourceKind, Provider } from "@destack/resource";
import type { Identifier } from "@destack/schema";
import { Journal, type JournalKey } from "@destack/service/database";
import {
    implement,
    type ServiceContext,
    type ServiceImplementation,
} from "@destack/service/server";
import type { Runtime } from "@destack/host/runtime";
import { servedObjects } from "@destack/setting/server";
import { digest } from "@destack/schema/json";
import * as object from "../object/index.ts";
import { spaceObjects, spaceService } from "../service/service.ts";
import { SpaceKey } from "../service/key.ts";
import { spaceJournal } from "../stack/index.ts";
import { type Bindable, Binder, binding, resourceBindable } from "./binding.ts";
import { serveDeployments } from "./deployment.ts";
import { serveInstallations } from "./installation/installation.ts";
import { type OpenBuild, openRelease } from "./installation/release.ts";
import { serveRevisions } from "./installation/revision.ts";
import { networkPolicy, packagePolicy } from "./policy.ts";
import { ProviderIndex, serveResources } from "./resource.ts";
import { relationship, role } from "./role.ts";
import { serveSpaces } from "./space.ts";
import { serveTransfers, serveZones, type TransferOptions, type ZoneRelay } from "./transfer.ts";
import { type RunOptions, serveRuns } from "./run.ts";
import { serveSchedules } from "./schedule.ts";
import { zoneRouter } from "./zone.ts";

/** The global tier as a cell reaches it: the directory, the account service's copies of the universe, and its peers' zone relays. */
export interface GlobalTier {
    /** The directory: each space's cell, and the claims keeping unique indexes across databases. */
    readonly directory: Directory;
    /** The account service's source of the copies of the scopes above spaces and of the users they follow. */
    readonly replicas: ReplicaSource;
    /** Reach the zone relay of a cell moving a space here, proving this cell's identity. */
    relay(cell: string): Promise<ZoneRelay>;
}

/** What a host or region serves its spaces with: their database, the global tier, and the providers and object types it administers. */
export interface SpaceServiceOptions {
    /** The database keeping the spaces. */
    readonly database: DatabaseConnection;
    /** Read the deployment's key that sensitive inputs are fingerprinted under. */
    readonly journalKey: JournalKey;
    /** Record calls in a scope under the request's verified authority, or as the system without one. */
    readonly audit: (scope: string, context?: ServiceContext) => AuditRecorder<DatabaseConnection>;
    /** The global tier. */
    readonly global: GlobalTier;
    /** The host or region this service serves spaces for. */
    readonly cell: object.SpaceCell;
    /** The accounts whose chains the cell copies, empty for a region. */
    readonly served?: readonly Identifier<"account">[];
    /** The providers with the spaces' resources. */
    readonly providers: readonly Provider<ResourceKind, ObjectType>[];
    /** The object types stacks may declare on this cell beside the space's own, replacing the handlers of the same types. */
    readonly declared: readonly ObjectType[];
    /** Open a package's build through the cell's store of builds. */
    readonly openBuild: OpenBuild;
    /** The kinds of objects bindings target beside resources, such as secrets. */
    readonly bindables: readonly Bindable[];
    /** The runtimes the cell runs its instances on, in preference order. */
    readonly runtimes: readonly Runtime[];
    /** How the cell attempts its installations' runs. */
    readonly runs: RunOptions;
    /** Read the cell's current time in UTC epoch milliseconds, the system clock by default. */
    readonly now?: () => number;
    /** The object types of other services whose access this server decides. */
    readonly hosted?: readonly ObjectType[];
}

/** Implement the procedures and controllers of space administration, decided through the space's policies. */
export function implementService(options: SpaceServiceOptions): ServiceImplementation {
    // serve the objects, their copies and the spaces' zones, and receive the spaces moving here
    const { database, global } = options;
    const objects = serveObjects(options);
    const implementation = implement(spaceService.router).$context<ServiceContext>();

    return {
        ...objects.implement(spaceService),
        access: {
            ...objects.access,
            // decide on the space the call's input selects
            target: async ({ input }) => {
                const { spaceId } = SpaceKey.passthrough().parse(input);

                return Scope.object(Snapshot.live(database), spaceId);
            },
        },
        router: implementation.router({
            ...objects.router(),
            relay: zoneRouter({ ...options, directory: global.directory }),
        }),
    };
}

/** Serve the space's objects: apply what stacks declare, reconcile them on this cell, and keep and relay the served spaces' copies. */
export function serveObjects(options: SpaceServiceOptions) {
    // open the builds of the installed releases, and index the cell's providers and bindable kinds
    const { database, global, cell, openBuild } = options;
    const now = options.now ?? Date.now;
    const release = (
        scope: string,
        packageId: PackageId,
        installationId?: Identifier<"installation">,
    ) => openRelease(database, openBuild, scope, packageId, installationId);
    const providers = new ProviderIndex(cell, options.providers);
    const binder = new Binder([resourceBindable, ...options.bindables]);
    const server = (): Pick<ObjectServer, "database" | "invoke"> => objects;
    const transfers = transferring(options, providers, server);

    // serve the space's objects, applying what stacks declare and keeping the served spaces' copies
    const objects = new ObjectServer({
        objects: {
            ...spaceObjects,
            space: serveSpaces({ cell, directory: global.directory }),
            resource: serveResources(providers),
            installation: serveInstallations({
                declared: (): readonly ObjectType[] => declared,
                directory: global.directory,
                openBuild,
                binder,
                runtimes: options.runtimes.map((runtime) => runtime.name),
            }),
            installationRevision: serveRevisions(openBuild),
            binding,
            networkPolicy,
            packagePolicy,
            ...serveDeployments({
                cell,
                runtimes: new Map(options.runtimes.map((runtime) => [runtime.name, runtime])),
                openBuild,
            }),
            ...providers.facets(),
            setting: servedObjects(release).setting,
            ...serveRuns(options.runs, now),
            ...serveSchedules(now),
            transfer: serveTransfers(transfers),
            zone: serveZones(transfers),
        },
        ...(options.hosted === undefined ? {} : { policies: options.hosted }),
        directory: global.directory,
        database,
        audit: options.audit,
        journal: new Journal(spaceJournal, options.journalKey),
        replicas: {
            source: global.replicas,
            requests: (): Promise<readonly Omit<ReplicaRequest, "after">[]> =>
                replicaRequests(objects, cell, options.served ?? []),
        },
    });
    const handled = [
        ...objects.objects.filter((type) => type.declaration !== undefined),
        role,
        relationship,
        ...options.declared,
    ];
    const declared: readonly ObjectType[] = [
        ...new Map(handled.map((type) => [type.table, type])).values(),
    ];

    return objects;
}

/** Move spaces through the directory, the tables with their rows, the sources' relays and the cell's providers. */
function transferring(
    options: SpaceServiceOptions,
    providers: ProviderIndex,
    server: () => Pick<ObjectServer, "database" | "invoke">,
): TransferOptions {
    return {
        cell: options.cell,
        directory: options.global.directory,
        source: (source) => options.global.relay(source),
        providers,
        server,
    };
}

/** List the replica requests of the copies a cell keeps, once each. */
async function replicaRequests(
    objects: Pick<ObjectServer, "database" | "authorizer" | "source">,
    cell: object.SpaceCell,
    served: readonly Identifier<"account">[],
): Promise<readonly Omit<ReplicaRequest, "after">[]> {
    // read the spaces the cell serves
    const spaces = await objects.database
        .select({ id: object.space.table.id })
        .from(object.space.table)
        .where(object.SpaceCell.served());

    // request each space's copies, each served account's chain and the cell's own rows of the universe
    const own = objects.source.universeRequest(object.SpaceCell.id(cell));
    const requests = [
        ...(
            await Promise.all(
                spaces.map(({ id }) => objects.source.replicaRequests(id, { isHome: true })),
            )
        ).flat(),
        ...(
            await Promise.all(
                served.map((account) =>
                    objects.authorizer.chain(objects.database, account, { isHome: false }),
                ),
            )
        ).flat(),
        ...(own === undefined ? [] : [own]),
    ];

    // keep each request once
    const keys = await Promise.all(requests.map((request) => digest(request)));
    const seen = new Set<string>();

    return requests.filter((_, index) => {
        const isNew = !seen.has(keys[index]!);
        seen.add(keys[index]!);

        return isNew;
    });
}
