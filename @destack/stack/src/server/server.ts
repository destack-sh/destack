import { type Identifier, Clock, schema } from "@destack/schema";
import { Snapshot, type Database, type DatabaseConnection } from "@destack/db";
import { type Publisher, type Subject, Scope, type Uplink } from "@destack/sync";
import type { CallKey } from "@destack/runtime/request";
import type accountPackage from "@destack/account/package";
import type { ClientOf } from "@destack/runtime/client";
import { type IdentityKeystore } from "@destack/account/identity";
import { Follow, type ObjectType, type HookSandbox } from "@destack/object";
import {
    Account,
    placement,
    residency,
    ResidencyCode,
    residentShape,
    space,
    spaceShape,
    SpaceAllowance,
} from "@destack/account/object";
import {
    type ClaimHome,
    defineServer,
    type ExternalStorage,
    ObjectStore,
    type PackageServer,
    type ServerDefinition,
} from "@destack/object/server";
import { type PackageImplementation } from "@destack/runtime/server";
import { bucketProvider, type CatalogueBucketHost } from "@destack/bucket/provider";
import { implementBucket } from "./resource/bucket.ts";
import type { Runtime } from "../runtime/index.ts";
import { SettingCatalog } from "@destack/setting";
import { settingServer } from "@destack/setting/server";
import { branchType, installation } from "../object/index.ts";
import stackPackage from "../package.ts";
import { implementRestore, implementSnapshot } from "./resource/snapshot.ts";
import type { ContentStore } from "@destack/resource";
import { Bindable, Binder, type ServedBindable } from "./resource/binding.ts";
import { implementDeployment, implementInstance } from "./deployment/deployment.ts";
import { implementInstallation } from "./installation/installation.ts";
import { InstallationAssertion } from "./installation/token.ts";
import { implementInstallationRevision } from "./installation/revision.ts";
import {
    httpsProvider,
    installationProvider,
    ProviderIndex,
    type SpaceProvider,
} from "./resource/resource.ts";
import { implementSpace } from "./host/space.ts";
import { Scheduler } from "./host/scheduler.ts";
import { implementTransfer, type TransferOptions } from "./host/transfer.ts";
import { type Handoff, handoffAdapter } from "./host/handoff.ts";
import type { EventStore } from "@destack/event";
import { implementRun } from "./run/run.ts";
import { BuildStore, type Registry, implementBuild } from "./installation/build.ts";
import type { RepositoryOwner } from "./installation/selection.ts";
import { AssignmentController } from "./deployment/assignment.ts";
import { PlacementController } from "./host/placement.ts";
import { TransferController } from "./host/transfer.ts";
import { ConsumerController } from "./resource/consumer.ts";
import { RetentionController, type SweepCache } from "./resource/retention.ts";
import { implementTrigger } from "./run/trigger.ts";
import { hookAdapter } from "./installation/hook.ts";
import type { SocketRegistry } from "./host/socket.ts";
import { implementRoute } from "./access/route.ts";
import { implementRelationship, implementRole } from "./access/role.ts";

/** The account package's methods a machine calls about its spaces, as the machine. */
export interface SpaceAccounts {
    /** The residencies of accounts and the cloud machines serving them. */
    readonly residency: Pick<AccountClient["residency"], "machines">;
    /** The homes of people. */
    readonly user: Pick<AccountClient["user"], "home">;
}

/** The account package's client. */
type AccountClient = ClientOf<typeof accountPackage>;

/** What a machine serves its spaces with. */
export interface SpaceOptions {
    /** The principal the store's own work runs as. */
    readonly principal: Subject;
    /** The space's database. */
    readonly database: DatabaseConnection;
    /** The key sensitive call inputs are fingerprinted under in the journal. */
    readonly callKey: CallKey;
    /** The account package, called as the machine. */
    readonly accounts: SpaceAccounts;
    /** The claim type keeping unique index names, and the home serving it. */
    readonly claims: ClaimHome;
    /** The account package's uplink, signed as the space or its installations. */
    readonly publisher: Uplink;
    /** Connect to the handoff of a machine moving a space here. */
    readonly handoff: (
        machine: Identifier<"machine">,
        spaceId: Identifier<"space">,
    ) => Promise<Handoff>;
    /** The machine serving the spaces. */
    readonly machine: Identifier<"machine">;
    /** The machine running the spaces' commit builds. */
    readonly buildRunner: Identifier<"machine"> | null;
    /** Whether the machine checks its spaces' use against their accounts' allowances. */
    readonly isEnforcingAllowances: boolean;
    /** Open the publisher at the home of a space a follow reads, such as a residency's home. */
    readonly publisherAt: ((scope: string) => Publisher) | null;
    /** The providers with the spaces' resources. */
    readonly providers: readonly SpaceProvider[];
    /** The machine's content-addressed store, keeping its resources' snapshots. */
    readonly store: ContentStore;
    /** The keystore of the spaces, wrapping their keys under the machine's keyring. */
    readonly keystore: IdentityKeystore;
    /** The space's other package servers, such as its vaults and messages. */
    readonly servers: readonly PackageServer[];
    /** The sandboxes of the installations whose packages main serves without a workload. */
    readonly sandboxes: readonly HookSandbox[];
    /** The registry of published releases. */
    readonly registry: Registry;
    /** The platform's declaration of every space's main database, with the platform's tables. */
    readonly main: Database;
    /** Delete what build retention drops from the machine's pull cache. */
    readonly sweepCache: SweepCache | null;
    /** The host keeping the spaces' buckets. */
    readonly buckets: CatalogueBucketHost;
    /** The machine's store of every build. */
    readonly builds: BuildStore;
    /** The kinds of package objects that bindings target beside resources, such as secrets. */
    readonly bindables: readonly ServedBindable[];
    /** The runtimes the machine runs its instances on, in preference order. */
    readonly runtimes: readonly Runtime[];
    /** The events of the spaces, keeping their finished runs. */
    readonly events: EventStore;
    /** Read the account owning a repository an installation selects. */
    readonly repositories: RepositoryOwner | null;
    /** The shapes the machine follows beside the space's own, such as its repository references. */
    readonly follows: readonly Follow[];
    /** The files with the machine's external object types, such as its builds' graphs. */
    readonly external: ExternalStorage | null;
    /** The live sockets of the machine's spaces. */
    readonly sockets: SocketRegistry;
    /** The clock the machine reads time at. */
    readonly clock: Clock;
}

/** Serve the stack package over the spaces' objects, beside the other packages' resources. */
function stackServer(
    options: SpaceOptions,
    others: readonly {
        readonly objects: readonly ObjectType[];
        readonly providers?: ServerDefinition["providers"];
    }[],
): PackageServer {
    // index the machine's providers and bindable kinds, and schedule and move the spaces through one scheduler
    const listed = spaceProviders(options);
    const providers = new ProviderIndex(options.machine, listed, others);
    const binder = new Binder(
        providers.objects().map((type) => Bindable.resource(providers.kind(type), type)),
        options.bindables,
    );
    const scheduler = new Scheduler(options);
    const transfers = configureTransfers(options, providers);

    // serve the space's objects with the external types
    const own = ownObjects(options, providers, binder, scheduler, transfers);

    return defineServer(stackPackage, {
        objects: [...own, ...externalObjects(options, own)],
        ...(options.external === null ? {} : { external: options.external }),
        providers: listed,
        follows: [
            ...spaceFollows(options),
            ...(options.isEnforcingAllowances ? [SpaceAllowance.follow] : []),
            ...options.follows,
        ],
        // answer moving spaces' targets and installations' hook sessions
        adapters: {
            handoff: handoffAdapter({ ...options, providers }),
            hook: hookAdapter(options.sandboxes),
        },
        // control the spaces, their consumers, retention and moves over the space's store
        controllers: [
            () =>
                new AssignmentController(
                    options.database,
                    options.publisher,
                    options.machine,
                    options.clock,
                ),
            (store) => new PlacementController(store, options.machine, options),
            (store) => new ConsumerController(store, providers.objects(), options.main),
            (store) =>
                new RetentionController(store, options.builds, options.sweepCache, options.store),
            (store) => new TransferController(transfers, store),
            ...options.providers.flatMap((provider) =>
                (provider.controllers ?? []).map((controller) => () => controller),
            ),
        ],
        responseHeaders: { "Cache-Control": "no-store" },
    });
}

/** Serve a space's stack and other packages over its one database, the stack first. */
export function serveSpace(options: SpaceOptions): {
    readonly store: ObjectStore;
    readonly packages: readonly PackageImplementation[];
} {
    // serve each other package once, its resources provisioned beside the stack's
    const others = options.servers.map((server) => {
        // read the package's definition once, serving it as a static one
        const definition = server.definition(server.installation);
        const resolved = defineServer(server.package, definition);

        // list the package's types, the handled ones in place, whose resources the stack provisions
        const handled = definition.objects ?? [];
        const objects = Object.values(server.package.objects).map(
            (type) => handled.find((each) => each.same(type)) ?? type,
        );

        return {
            served: {
                objects,
                ...(definition.providers === undefined ? {} : { providers: definition.providers }),
            },
            server: server.installation === undefined ? resolved : resolved.at(server.installation),
        };
    });

    // serve the stack, the settings and the other packages over one store
    const store = new ObjectStore({
        servers: [
            stackServer(
                options,
                others.map((each) => each.served),
            ),
            installedSettingsServer(options),
            ...others.map((each) => each.server),
        ],
        principal: options.principal,
        database: options.database,
        claims: options.claims,
        callKey: options.callKey,
        clock: options.clock,
        events: options.events,
        ephemeral: options.sockets.storage,
        origin: { package: stackPackage, machineId: options.machine },
        uplink: options.publisher,
        // follow no chain until the space is placed, then keep it above the space where placed, else follow it
        chain: async (view) => {
            const served = schema.identifier("space").parse(options.principal.id);
            const row = await view.read(placement.table).findFirst({ where: { id: served } });

            return row === undefined
                ? undefined
                : row.machineId === options.machine
                  ? { kind: "home", parent: row.scope }
                  : { kind: "follower" };
        },
        // reach the universe by the uplink, other scopes at their homes by the machine's route
        publisher: (scope) => homePublisher(options, scope),
        machine: options.machine,
        branch: branchType,
        sandboxes: options.sandboxes,
    });

    // verify the installations' assertions to each package beside the proofs they spend
    return {
        store,
        packages: store.packages.map((served) => {
            const verifier = InstallationAssertion.verifier(store, served.package.id);

            return {
                ...served,
                verify: (request: Request) =>
                    InstallationAssertion.verify(verifier, request, options.clock.now()),
            };
        }),
    };
}

/** Serve the space's own objects. */
function ownObjects(
    options: SpaceOptions,
    providers: ProviderIndex,
    binder: Binder,
    scheduler: Scheduler,
    transfers: TransferOptions,
) {
    // serve the space's types with their behaviour
    const { builds, registry } = options;
    const served = [
        implementSnapshot(transfers),
        implementRestore(transfers),
        implementSpace({
            keystore: options.keystore,
            scheduler,
            publisher: options.publisher,
            isEnforcingAllowances: options.isEnforcingAllowances,
        }),
        ...providers.provided(),
        implementInstallation({
            resources: providers.objects(),
            builds,
            registry,
            binder,
            runtimes: options.runtimes.map((runtime) => runtime.name),
            accounts: options.accounts,
            repositories: options.repositories,
            issuer: options.keystore.issuer(options.database, options.principal.id),
        }),
        implementInstallationRevision(builds, registry),
        binder.declare(),
        implementDeployment({
            providers,
            bindables: options.bindables,
            machine: options.machine,
            runtimes: new Map(options.runtimes.map((runtime) => [runtime.name, runtime])),
            builds,
        }),
        implementInstance(),
        implementRun(),
        implementBuild(options),
        implementTrigger(),
        implementTransfer(transfers),
        implementRoute(),
        implementRole(),
        implementRelationship(),
    ];

    // serve the package's stored types, each in place of its plain type, then the provided resources
    const own = Object.values(stackPackage.objects)
        .filter((type) => type.storage !== "external")
        .map((type) => served.find((each) => each.same(type)) ?? type);

    return [...own, ...served.filter((type) => !own.includes(type))];
}

/** Serve the space's installations' settings, read from the builds they install. */
function installedSettingsServer(options: SpaceOptions): PackageServer {
    const snapshot = Snapshot.live(options.database);
    const catalog = SettingCatalog.cached((written) =>
        BuildStore.installed(
            options.builds,
            snapshot,
            schema.identifier("space").parse(written.scope),
            written.package ?? written.packageId,
            written.installation,
        ),
    );

    return settingServer({ declared: catalog, after: [installation] });
}

/** Collect the external types, refusing one under a name the space serves. */
function externalObjects(options: SpaceOptions, own: readonly ObjectType[]): readonly ObjectType[] {
    // refuse an external type under a name the space serves
    const external = options.external?.objects ?? [];
    const taken = external.find((type) => own.some((each) => each.key === type.key));
    if (taken !== undefined) {
        throw new TypeError(`object ${taken.name} is served by the space`);
    }

    return external;
}

/** List the providers of a space's resources. */
function spaceProviders(options: SpaceOptions) {
    return [
        installationProvider,
        httpsProvider,
        bucketProvider(options.buckets, implementBucket(options.buckets)),
        ...options.providers,
    ];
}

/** Move spaces through the universe, the sources' handoffs, the providers and the spaces' keys. */
function configureTransfers(options: SpaceOptions, providers: ProviderIndex): TransferOptions {
    return {
        machine: options.machine,
        database: options.database,
        publisher: options.publisher,
        source: (source, spaceId) => options.handoff(source, spaceId),
        providers,
        store: options.store,
        builds: options.builds,
        keystore: options.keystore,
        runtimes: options.runtimes,
    };
}

/** Follow the served space's row and its residents. */
function spaceFollows(options: Pick<SpaceOptions, "principal">): readonly Follow[] {
    const served = schema.identifier("space").parse(options.principal.id);

    return [
        Follow.of(spaceShape, { keys: [{ key: { scope: Scope.universe.id, space: served } }] }),
        Follow.of(residentShape, {
            rows: () => residency.query(),
            key: async (row) =>
                row["home"] === served
                    ? {
                          key: {
                              scope: Scope.universe.id,
                              residency: ResidencyCode.parse(row["id"]),
                          },
                      }
                    : undefined,
        }),
    ];
}

/** Open a scope's publisher: the uplink for the universe, space and machine, else its home. */
function homePublisher(
    options: Pick<SpaceOptions, "database" | "publisher" | "publisherAt" | "principal" | "machine">,
    scope: string,
): Publisher {
    // follow the universe, and the chains of the served space and its machine, through the uplink
    const { publisherAt } = options;
    if ([Scope.universe.id, options.principal.id, options.machine].includes(scope)) {
        return options.publisher;
    }
    // refuse a machine without a route to other spaces
    else if (publisherAt === null) {
        throw new TypeError(`this machine follows nothing from the home of ${scope}`);
    }

    return {
        async *stream(subscription, signal) {
            // follow a space at itself, an account, organisation or person at their home
            const isSpace = space.identifies(scope);
            const home = isSpace
                ? scope
                : await Account.homeOf(Snapshot.live(options.database), scope);
            yield* publisherAt(home).stream(subscription, signal);
        },
    };
}
