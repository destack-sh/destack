import { PROCESS_RUNTIME } from "../object/index.ts";
import { KNOWN_TYPES } from "../stack/kernel.ts";
import { type BlobStore, type Database, type DatabaseConnection, eq, Snapshot } from "@destack/db";
import { principal } from "@destack/access";
import { hostname, machine } from "@destack/account/object";
import { type PackageId, PackageRelease } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import { type Digest, type Identifier, schema, type VersionRange } from "@destack/schema";
import { type Publisher, RequestId, type Subject, Scope, type Uplink } from "@destack/sync";
import { type CallKey, Provisioned } from "@destack/object";
import type accountPackage from "@destack/account/package";
import type { ClientOf } from "@destack/object/client";
import { type IdentityKeystore } from "@destack/account/identity";
import {
    type Follow,
    grantShape,
    type ObjectType,
    type HookSandbox,
    relationship,
} from "@destack/object";
import {
    Account,
    placement,
    profileShape,
    residency,
    residentsShape,
    space,
    residencyShape,
    spaceShape,
    user,
    Entitlement,
} from "@destack/account/object";
import {
    type ClaimPrimary,
    implementPackage,
    type ExternalStorage,
    ObjectStore,
    type PackageImplementation,
    type ImplementationInput,
} from "@destack/object/server";
import { type PackageHandler } from "@destack/object/server";
import { bucketProvider, type CatalogueBucketHost } from "@destack/bucket/provider";
import { implementBucket } from "./resource/bucket.ts";
import type { Runtime } from "../runtime/index.ts";
import { settingServer } from "@destack/setting/server";
import type { BootstrapPackageName } from "../declare/index.ts";
import { PackageContext } from "@destack/object/workload";
import { rollout } from "../object/index.ts";
import objectPackage from "@destack/object/package";
import stackPackage from "../package.ts";
import { implementRestore, implementSnapshot } from "./resource/snapshot.ts";
import { Binder, ResourceBindable, ObjectBindable } from "./resource/binding.ts";
import { implementDeployment, implementInstance } from "./deployment/deployment.ts";
import {
    type InstallationOptions,
    implementInstallations,
    implementRollouts,
} from "./installation/installation.ts";
import { implementInstallationRevision } from "./installation/revision.ts";
import { installationProvider, ProviderIndex, type SpaceProvider } from "./resource/resource.ts";
import { implementSpace, type SpaceRole } from "./host/space.ts";
import { implementTransfer, type TransferOptions } from "./host/transfer.ts";
import { type Handoff, handoffAdapter } from "./host/handoff.ts";
import type { EventStore } from "@destack/event";
import {
    type BuildOptions,
    SpaceBuildStore,
    type Registry,
    implementBuild,
} from "./installation/build.ts";
import type { Forge } from "./installation/follow.ts";
import { SpaceTransfer } from "./host/transfer.ts";
import type { SpaceHost } from "./host/host.ts";
import { hookAdapter } from "./installation/hook.ts";
import type { SocketRegistry } from "./host/socket.ts";
import { implementRoute } from "./access/route.ts";
import { implementRelationship, implementRole } from "./access/role.ts";
import type { Environment } from "@destack/environment";
import { RolloutTarget } from "./installation/target.ts";

/** The account package's methods a machine calls about its spaces, as the machine. */
export interface SpaceAccounts {
    /** The residencies of accounts and the cloud machines serving them. */
    readonly residency: Pick<AccountClient["residency"], "machines">;
    /** People's profiles, read for their home spaces. */
    readonly user: {
        /** Read a person's profile. */
        readonly get: (input: {
            readonly id: Identifier<"user">;
        }) => Promise<{ readonly homeId: Identifier<"space"> }>;
    };
}

/** The account package's client. */
type AccountClient = ClientOf<typeof accountPackage>;

/** What a machine serves its spaces with. */
export interface SpaceOptions {
    /** The space's role on this machine, as its placement decides. */
    readonly role: SpaceRole;
    /** The space as this machine hosts it. */
    readonly host: SpaceHost;
    /** The principal the store's own work runs as. */
    readonly principal: Subject;
    /** Report an installation the space leaves unserved, such as a sandbox of unserved types. */
    readonly report: (error: unknown) => void;
    /** The partitions the space serves: itself, and its account when it is the home. */
    readonly partitions: readonly string[];
    /** The space's database. */
    readonly database: DatabaseConnection;
    /** The key sensitive call inputs are fingerprinted under in the journal. */
    readonly callKey: CallKey;
    /** The account package, called as the machine. */
    readonly accounts: SpaceAccounts;
    /** The claim type of unique index names, and the home that serves it. */
    readonly claims: ClaimPrimary;
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
    /** Whether the machine checks its spaces' use against their accounts' entitlements. */
    readonly isEnforcingEntitlements: boolean;
    /** The Destack versions the machine runs releases of: its own and the oldest it supports. */
    readonly destack: VersionRange;
    /** Whether requests for the account's own hostnames reach the machine, so its routes serve. */
    readonly isServingHostnames: boolean;
    /** Open the publisher of a partition a follow reads at its home, such as a residency's home. */
    readonly publisherAt: ((home: string, partition: string) => Publisher) | null;
    /** The providers with the spaces' resources. */
    readonly providers: readonly SpaceProvider[];
    /** The space's blobs, kept while its rows reference them, in the space's database. */
    readonly blobs: BlobStore;
    /** The keystore of the spaces, wrapping their keys under the machine's keyring. */
    readonly keystore: IdentityKeystore;
    /** The space's other package servers, such as its vaults and messages. */
    readonly servers: readonly PackageImplementation[];
    /** The sandboxes of the installations whose packages main serves without a workload. */
    readonly sandboxes: readonly HookSandbox[];
    /** The registry of published releases. */
    readonly registry: Registry;
    /** Publish a stored build and its account's unpublished dependencies for a caller. */
    readonly publish: BuildOptions["publish"];
    /** The release the machine links of each package, by package. */
    readonly linked: ReadonlyMap<PackageId, PackageRelease>;
    /** The releases the machine ships, by package. */
    readonly shipped: ReadonlyMap<PackageId, PackageRelease>;
    /** The manifests of the releases the machine runs in its process: those it ships or links. */
    readonly trusted: ReadonlySet<Digest>;
    /** The platform's declaration of every space's main database, with the platform's tables. */
    readonly main: Database;
    /** The host of the spaces' buckets. */
    readonly buckets: CatalogueBucketHost;
    /** The space's builds, read through to the machine's cache. */
    readonly builds: SpaceBuildStore;
    /** The runtimes the machine runs its instances on, in preference order. */
    readonly runtimes: readonly Runtime[];
    /** The events of the spaces, recording their finished runs. */
    readonly events: EventStore;
    /** The forge whose references and tags installations select, null without one. */
    readonly forge: Forge | null;
    /** The shapes the machine follows with the space's own, such as its repository references. */
    readonly follows: readonly Follow[];
    /** The host's own controllers of the served space: what it serves, and its meters. */
    readonly controllers: NonNullable<ImplementationInput["controllers"]>;
    /** The host's own cursors of the served space, such as its readings. */
    readonly cursors: NonNullable<ImplementationInput["cursors"]>;
    /** The storage of the machine's external object types: its builds' graphs. */
    readonly external: ExternalStorage;
    /** The live sockets of the machine's spaces. */
    readonly sockets: SocketRegistry;
    /** The environment the machine runs in: its clock, randomness, crypto and network. */
    readonly environment: Environment;
}

/** Serve the stack package over the spaces' objects, next to the other packages' resources. */
function stackServer(
    options: SpaceOptions,
    providers: ProviderIndex,
    binder: Binder,
): PackageImplementation {
    // configure the moves
    const listed = providers.own;
    const transfers = configureTransfers(options, providers);

    // serve the space's objects with the external types, a standby only those its moves write
    const isPrimary = options.role === "primary";
    const own = isPrimary
        ? ownObjects(options, providers, binder, transfers)
        : stackObjects([implementRestore(transfers), implementTransfer(transfers)]);

    return implementPackage(stackPackage, {
        objects: [...own, ...externalObjects(options, own)],
        external: options.external,
        providers: listed,
        follows: [
            ...spaceFollows(options),
            // follow the account's entitlements, which enforcement reads
            ...(isPrimary ? [Entitlement.follow, ...options.follows] : []),
        ],
        // answer moving spaces' targets and installations' hook sessions
        adapters: {
            handoff: handoffAdapter({ ...options, providers }),
            push: options.builds.adapter(),
            hook: hookAdapter,
        },
        // receive moving spaces on a standby, run a primary's providers, and the host's own
        controllers: [
            ...(isPrimary
                ? options.providers.flatMap((provider) =>
                      (provider.controllers ?? []).map((controller) => () => controller),
                  )
                : [(store: ObjectStore) => SpaceTransfer.controller(transfers, store)]),
            ...options.controllers,
        ],
        // finish moves in either role, and project a primary's birth, assignments, resources and retention
        cursors: [
            (store) => SpaceTransfer.projection(transfers, store),
            ...(isPrimary
                ? [
                      (store: ObjectStore) => options.host.placement(store, options.main),
                      () => options.host.assignment(options.publisher),
                      (store: ObjectStore) =>
                          binder.specification(store, options.main, options.linked),
                      (store: ObjectStore) => options.builds.retention(store),
                  ]
                : []),
            ...options.cursors,
        ],
        responseHeaders: { "Cache-Control": "no-store" },
    });
}

/** A space's store: its stack, access and settings over its installed packages, as they change. */
export class SpaceStore {
    /** The store, the stack's package first. */
    readonly store: ObjectStore;
    /** The options the space serves with. */
    readonly #options: SpaceOptions;
    /** Each installed server read once, by the server: its types, its providers and its server. */
    #installed = new Map<
        PackageImplementation,
        {
            readonly served: {
                readonly objects: readonly ObjectType[];
                readonly providers: ImplementationInput["providers"];
            };
            readonly server: PackageImplementation;
            readonly isProviding: boolean;
        }
    >();
    /** The stack's, access's and settings' servers, with the providing servers they read. */
    #own:
        | {
              readonly over: readonly PackageImplementation[];
              readonly servers: readonly PackageImplementation[];
          }
        | undefined;

    /** Serve a space's stack and installed packages over its one database, the stack first. */
    constructor(options: SpaceOptions) {
        // read the installed servers, then serve them beside the stack, access and settings
        this.#options = options;
        const servers = this.#servers(options.servers);
        this.store = new ObjectStore({
            report: options.report,
            servers,
            principal: options.principal,
            partitions: options.partitions,
            database: options.database,
            blobs: options.blobs.storage,
            // know the account's hostnames, which its spaces' routes are declared on, as replicated rows
            policies: [hostname],
            claims: options.claims,
            callKey: options.callKey,
            environment: options.environment,
            events: options.events,
            ephemeral: options.sockets.storage,
            // keep a socket of each principal's stream, recording a machine leaving with its last one
            connect: (subject) =>
                linkSocket(
                    options.sockets,
                    schema.identifier("space").parse(options.principal.id),
                    subject,
                    () => this.store,
                ),
            origin: { package: stackPackage, machineId: options.machine },
            uplink: options.publisher,
            // open the release the space runs, or the version a value names
            release: (view, scope, packageId, version) =>
                this.release(view, schema.identifier("space").parse(scope), packageId, version),
            // root the chain above the space by its placement, as primary or move target
            chain: async (view) => {
                // read where the space is placed, and whether it moves here
                const served = schema.identifier("space").parse(options.principal.id);
                const row = await view
                    .read(placement.table)
                    .findFirst({ where: { spaceId: served } });
                const isFollowing = options.role === "primary" || row?.targetId === options.machine;

                // root the chain at the placement, wait for a primary's, and follow none elsewhere
                if (row !== undefined && isFollowing) {
                    return { kind: "home", parent: row.scope };
                } else if (row === undefined && options.role === "primary") {
                    return { kind: "pending" };
                }

                return undefined;
            },
            // send to the universe by the uplink, and to other scopes at their homes by the machine's route
            publisher: (scope) => homePublisher(options, scope),
            machine: options.machine,
            sandboxes: options.sandboxes,
        });
    }

    /** Open a package's release the space runs, or a named version, absent for none. */
    async release(
        view: Snapshot,
        spaceId: Identifier<"space">,
        packageId: PackageId,
        version?: string,
    ): Promise<BuildReader | undefined> {
        // read the installed release and the distribution's
        const options = this.#options;
        const { registry, builds } = options;
        const installed = await RolloutTarget.reader(view, spaceId, packageId, undefined, options);
        const distributed = PackageRelease.newest(registry.distribution, packageId);
        const isInstalled =
            installed !== undefined &&
            (version === undefined || installed.manifest.package.version === version);

        // open the installed release, unless a value names another version
        if (isInstalled) {
            return installed;
        }
        // pull a named version
        else if (version !== undefined) {
            return builds.pull(packageId, await registry.find(packageId, version));
        }
        // pull the distribution's release of a package the space does not install
        else if (distributed !== undefined) {
            return builds.pull(packageId, distributed.manifest);
        }
        // run nothing of a package outside the distribution
        else {
            return undefined;
        }
    }

    /** Serve a changed set of installed servers and sandboxes, the unchanged ones kept. */
    apply(
        servers: readonly PackageImplementation[],
        sandboxes: readonly HookSandbox[],
    ): readonly PackageHandler[] {
        return this.store.apply(this.#servers(servers), sandboxes);
    }

    /** List the space's servers: the stack's, access's and settings', then the installed ones. */
    #servers(installed: readonly PackageImplementation[]): readonly PackageImplementation[] {
        // read each installed server once, serving its definition as a static one
        const options = this.#options;
        const reads = installed.map(
            (server) => [server, this.#installed.get(server) ?? this.#read(server)] as const,
        );
        this.#installed = new Map(reads);

        // build the own servers again once the servers providing, binding or provisioning change
        const over = reads.filter(([, read]) => read.isProviding).map(([server]) => server);
        const earlier = this.#own;
        const own =
            earlier !== undefined &&
            earlier.over.length === over.length &&
            earlier.over.every((server, index) => server === over[index])
                ? earlier.servers
                : this.#build(
                      installed,
                      reads.map(([, read]) => read.served),
                  );
        this.#own = { over, servers: own };

        // serve a primary's installed packages beside its own, a standby its own alone
        return options.role === "primary" ? [...own, ...reads.map(([, read]) => read.server)] : own;
    }

    /** Read an installed server: its types, the handled ones in place, and its resolved server. */
    #read(server: PackageImplementation) {
        // read the package's definition once, serving it as a static one
        const definition = server.definition(
            PackageContext.of(server.installation, this.#options.blobs),
        );
        const resolved = implementPackage(server.package, definition);

        // list the package's types, the handled ones in place, whose resources the stack provisions
        const handled = definition.objects ?? [];
        const objects = Object.values(server.package.objects).map(
            (type) => handled.find((each) => each.same(type)) ?? type,
        );

        return {
            served: { objects, providers: server.providers },
            server: server.installation === undefined ? resolved : resolved.at(server.installation),
            isProviding:
                server.providers.length > 0 ||
                server.bindables.length > 0 ||
                objects.some((type) => Provisioned.is(type)),
        };
    }

    /** Build the stack's, access's and settings' servers over the installed providers and kinds. */
    #build(
        installed: readonly PackageImplementation[],
        served: ConstructorParameters<typeof ProviderIndex>[2],
    ): readonly PackageImplementation[] {
        // index the machine's providers and bindable kinds, which the stack and the installations bind with
        const options = this.#options;
        const bindables = ObjectBindable.of(installed);
        const providers = new ProviderIndex(options.machine, spaceProviders(options), served);
        const binder = new Binder(
            providers.objects().map((type) => ResourceBindable.of(providers.kind(type), type)),
            bindables,
        );

        // serve the space's own packages on a standby: the stack with its access objects
        const standby: {
            readonly [Name in Exclude<BootstrapPackageName, "setting">]: PackageImplementation;
        } = {
            stack: stackServer(options, providers, binder),
            object: implementPackage(objectPackage, {
                objects: [
                    implementRole(),
                    implementRelationship(),
                    implementInstallations(installationOptions(options, providers, binder)),
                ],
            }),
        };

        // serve every own package on a primary, its settings after rollout
        const primary: { readonly [Name in BootstrapPackageName]: PackageImplementation } = {
            ...standby,
            setting: settingServer({ after: [rollout] }),
        };

        return Object.values(options.role === "primary" ? primary : standby);
    }
}

/** Serve the space's own objects. */
function ownObjects(
    options: SpaceOptions,
    providers: ProviderIndex,
    binder: Binder,
    transfers: TransferOptions,
) {
    // serve the space's types with their behaviour
    const { builds } = options;
    const served = [
        implementSnapshot(transfers),
        implementRestore(transfers),
        implementSpace({
            keystore: options.keystore,
            builds,
            blobs: options.blobs,
            publisher: options.publisher,
            isEnforcingEntitlements: options.isEnforcingEntitlements,
        }),
        ...providers.provided(),
        implementRollouts(installationOptions(options, providers, binder)),
        implementInstallationRevision(),
        implementDeployment({
            providers,
            bindables: binder.objects(),
            machine: options.machine,
            runtimes: new Map(options.runtimes.map((runtime) => [runtime.name, runtime])),
            builds,
        }),
        implementInstance(),
        implementBuild(options),
        implementTransfer(transfers),
        implementRoute(options.isServingHostnames),
    ];

    return stackObjects(served);
}

/** Read what the space's installations run with on this machine. */
function installationOptions(
    options: SpaceOptions,
    providers: ProviderIndex,
    binder: Binder,
): InstallationOptions {
    return {
        resources: providers.objects(),
        builds: options.builds,
        blobs: options.blobs,
        registry: options.registry,
        binder,
        runtimes: options.runtimes.filter((runtime) => runtime.name !== PROCESS_RUNTIME),
        destack: options.destack,
        accounts: options.accounts,
        forge: options.forge,
        known: KNOWN_TYPES,
        issuer: options.keystore.issuer(options.database, options.principal.id),
        isEnforcingEntitlements: options.isEnforcingEntitlements,
        database: options.database,
        linked: options.linked,
        shipped: options.shipped,
        trusted: options.trusted,
    };
}

/** Serve the stack's stored types, each implemented one in place, then the other served types. */
function stackObjects(served: readonly ObjectType[]): ObjectType[] {
    const own = Object.values(stackPackage.objects)
        .filter((type) => type.storage !== "external")
        .map((type) => served.find((each) => each.same(type)) ?? type);

    return [...own, ...served.filter((type) => !own.includes(type))];
}

/** Collect the external types, refusing one under a name the space serves. */
function externalObjects(options: SpaceOptions, own: readonly ObjectType[]): readonly ObjectType[] {
    // refuse an external type under a name the space serves
    const external = options.external.objects;
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
        bucketProvider(options.buckets, implementBucket(options.buckets)),
        ...options.providers,
    ];
}

/** Move spaces through the universe, the sources' handoffs, the providers and the spaces' keys. */
function configureTransfers(options: SpaceOptions, providers: ProviderIndex): TransferOptions {
    return {
        role: options.role,
        machine: options.machine,
        database: options.database,
        publisher: options.publisher,
        source: (source, spaceId) => options.handoff(source, spaceId),
        providers,
        blobs: options.blobs,
        builds: options.builds,
        keystore: options.keystore,
        runtimes: options.runtimes,
    };
}

/** Follow the space's row and residents, and on its primary its people's profiles and grants. */
function spaceFollows(
    options: Pick<SpaceOptions, "principal" | "machine" | "role">,
): readonly Follow[] {
    // follow the space's row as the machine wherever it is served, which it needs before acting as the space
    const served = schema.identifier("space").parse(options.principal.id);
    const host = principal.machine.reference(Scope.universe.id, options.machine);
    const follows: Follow[] = [
        spaceShape.at({
            key: { scope: Scope.universe.id, space: served, machine: options.machine },
            subject: host,
        }),
    ];

    // follow nothing the space signs for on a standby, which has none of its keys
    if (options.role !== "primary") {
        return follows;
    }

    return [
        ...follows,
        // follow the residencies and its residency's residents as the space, a resident itself, so it decides on them
        residencyShape.at({ key: { scope: Scope.universe.id } }),
        residentsShape.per(residency, {
            key: async (row) =>
                row.space === served
                    ? { key: { scope: Scope.universe.id, residency: row.code } }
                    : undefined,
        }),
        // follow the profiles of the people related to the space once it is placed here
        profileShape.per(relationship, {
            where: { subjectPackageId: user.typeReference.packageId, subjectType: user.name },
            key: async (row, view) => {
                const placed = await view
                    .read(placement.table)
                    .findFirst({ where: { spaceId: served } });
                const isPerson = row.subjectRelation === null && user.identifies(row.subjectId);

                return isPerson && placed?.machineId === options.machine
                    ? { key: { scope: Scope.universe.id, user: user.identifier(row.subjectId) } }
                    : undefined;
            },
        }),
        // follow the grants people delegate to the space once it is placed here
        grantShape.per(placement, {
            key: async (row) =>
                row.spaceId === served && row.machineId === options.machine
                    ? {
                          key: {
                              scope: Scope.universe.id,
                              delegatePackageId: options.principal.packageId,
                              delegateType: options.principal.type,
                              delegateScope: options.principal.scope,
                              delegateId: options.principal.id,
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

    // follow a space at itself, and another scope in its account's partition at the account's home
    const homed = async () => {
        if (space.identifies(scope)) {
            return publisherAt(scope, scope);
        }
        const { id } = await Account.of(Snapshot.live(options.database), scope);

        return publisherAt(Account.home(id), id);
    };

    return {
        async *stream(subscription, signal) {
            yield* (await homed()).stream(subscription, signal);
        },
        request: async (subscription) => (await homed()).request(subscription),
    };
}

/** Open a principal's socket to a space, recording a machine's leave at its last close. */
async function linkSocket(
    sockets: SocketRegistry,
    spaceId: Identifier<"space">,
    subject: Subject,
    store: () => ObjectStore,
): Promise<() => Promise<void>> {
    const lease = await sockets.link(spaceId, subject);

    return async () => {
        await lease.close();
        if (principal.machine.is(subject) && !(await sockets.isConnected(subject))) {
            await leaveMachine(store(), subject);
        }
    };
}

/** Record a machine leaving its home, where this store serves the machine's row. */
async function leaveMachine(store: ObjectStore, subject: Subject): Promise<void> {
    // leave a machine whose row lives elsewhere, or replicated here
    if (store.findImplementation(machine) === undefined || store.replicates(machine)) {
        return;
    }
    const [row] = await store.database
        .select({ scope: machine.table.scope, id: machine.table.id })
        .from(machine.table)
        .where(eq(machine.table.id, schema.identifier("machine").parse(subject.id)));
    if (row === undefined) {
        return;
    }

    // record the leave as the store
    await store.mutate(
        store.as(machine),
        {
            id: RequestId.create(store.environment),
            calls: [machine.call("leave", machine.target(row))],
        },
        { component: "presence" },
    );
}
