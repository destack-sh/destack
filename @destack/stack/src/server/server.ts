import { PROCESS_RUNTIME } from "../object/index.ts";
import { principal } from "@destack/access";
import { hostname, machine } from "@destack/account/object";
import type { PackageId, PackageRelease } from "@destack/package";
import { type Identifier, present, schema } from "@destack/schema";
import { eq, Snapshot, type Database, type DatabaseConnection } from "@destack/db";
import { type Publisher, RequestId, type Subject, Scope, type Uplink } from "@destack/sync";
import { type CallKey, installation } from "@destack/object";
import type { ObjectPackage } from "@destack/object/server";
import type accountPackage from "@destack/account/package";
import type { ClientOf } from "@destack/object/client";
import { type IdentityKeystore } from "@destack/account/identity";
import {
    Follow,
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
import { Setting } from "@destack/setting";
import { settingServer } from "@destack/setting/server";
import type { BaseServerContext } from "@destack/object/workload";
import settingPackage from "@destack/setting/package";
import { Rollout, rollout } from "../object/index.ts";
import objectPackage from "@destack/object/package";
import stackPackage from "../package.ts";
import { implementRestore, implementSnapshot } from "./resource/snapshot.ts";
import type { ContentStore } from "@destack/resource";
import { Bindable, Binder, type ServedBindable } from "./resource/binding.ts";
import { implementDeployment, implementInstance } from "./deployment/deployment.ts";
import { implementInstallation, implementInstallations } from "./installation/installation.ts";
import { InstallationAssertion } from "./installation/token.ts";
import { implementInstallationRevision } from "./installation/revision.ts";
import {
    httpsProvider,
    installationProvider,
    ProviderIndex,
    type SpaceProvider,
} from "./resource/resource.ts";
import { implementSpace, type SpaceRole } from "./host/space.ts";
import { implementTransfer, type TransferOptions } from "./host/transfer.ts";
import { type Handoff, handoffAdapter } from "./host/handoff.ts";
import type { EventStore } from "@destack/event";
import { BuildStore, type Registry, implementBuild } from "./installation/build.ts";
import type { Forge } from "./installation/follow.ts";
import { AssignmentController } from "./deployment/assignment.ts";
import { PlacementController } from "./host/placement.ts";
import { TransferController } from "./host/transfer.ts";
import { ConsumerController } from "./resource/consumer.ts";
import { type ExternalGraphStorage, RetentionController } from "./resource/retention.ts";
import { hookAdapter } from "./installation/hook.ts";
import type { SocketRegistry } from "./host/socket.ts";
import { implementRoute } from "./access/route.ts";
import { implementRelationship, implementRole } from "./access/role.ts";
import type { Environment } from "@destack/environment";

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
    /** The base installation of each package the space is born with, by package. */
    readonly bases: ReadonlyMap<PackageId, BaseServerContext>;
    /** The principal the store's own work runs as. */
    readonly principal: Subject;
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
    /** Whether requests for the account's own hostnames reach the machine, so its routes serve. */
    readonly isServingHostnames: boolean;
    /** Open the publisher of a partition a follow reads at its home, such as a residency's home. */
    readonly publisherAt: ((home: string, partition: string) => Publisher) | null;
    /** The providers with the spaces' resources. */
    readonly providers: readonly SpaceProvider[];
    /** The space's own content-addressed store of its resources' snapshots. */
    readonly store: ContentStore;
    /** The keystore of the spaces, wrapping their keys under the machine's keyring. */
    readonly keystore: IdentityKeystore;
    /** The space's other package servers, such as its vaults and messages. */
    readonly servers: readonly PackageImplementation[];
    /** The sandboxes of the installations whose packages main serves without a workload. */
    readonly sandboxes: readonly HookSandbox[];
    /** The registry of published releases. */
    readonly registry: Registry;
    /** The release the machine ships of each package it serves in process, by package. */
    readonly bundled: ReadonlyMap<PackageId, PackageRelease>;
    /** The platform's declaration of every space's main database, with the platform's tables. */
    readonly main: Database;
    /** The graph indexes of the machine's spaces, swept once their builds go. */
    readonly graphs: Pick<ExternalGraphStorage, "sweep">;
    /** The host of the spaces' buckets. */
    readonly buckets: CatalogueBucketHost;
    /** The machine's store of every build. */
    readonly builds: BuildStore;
    /** The kinds of package objects that bindings target other than resources, such as secrets. */
    readonly bindables: readonly ServedBindable[];
    /** The runtimes the machine runs its instances on, in preference order. */
    readonly runtimes: readonly Runtime[];
    /** The events of the spaces, recording their finished runs. */
    readonly events: EventStore;
    /** The repositories installations select and follow, null on a machine following none. */
    readonly forge: Forge | null;
    /** The shapes the machine follows with the space's own, such as its repository references. */
    readonly follows: readonly Follow[];
    /** The host's own controllers of the served space: its serving lifecycle and its meters. */
    readonly controllers: NonNullable<ImplementationInput["controllers"]>;
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
    others: readonly {
        readonly objects: readonly ObjectType[];
        readonly providers?: ImplementationInput["providers"];
    }[],
): PackageImplementation {
    // index the machine's providers and bindable kinds, and the moves' options
    const listed = spaceProviders(options);
    const providers = new ProviderIndex(options.machine, listed, others);
    const binder = new Binder(
        providers.objects().map((type) => Bindable.resource(providers.kind(type), type)),
        options.bindables,
    );
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
            hook: hookAdapter(options.sandboxes),
        },
        // move the space in either role, run the primary's consumers and retention, and the host's own
        controllers: [
            (store) => new TransferController(transfers, store),
            ...(isPrimary ? primaryControllers(options, providers) : []),
            ...options.controllers,
        ],
        responseHeaders: { "Cache-Control": "no-store" },
    });
}

/** List a primary's controllers: assignments, placement, consumers and retention. */
function primaryControllers(
    options: SpaceOptions,
    providers: ProviderIndex,
): NonNullable<ImplementationInput["controllers"]> {
    return [
        () =>
            new AssignmentController(
                options.database,
                options.publisher,
                options.machine,
                options.environment,
            ),
        (store) =>
            new PlacementController(
                store,
                schema.identifier("space").parse(options.principal.id),
                options.machine,
                options,
            ),
        (store) => new ConsumerController(store, providers.objects(), options.main),
        (store) => new RetentionController(store, options.builds, options.graphs, options.store),
        ...options.providers.flatMap((provider) =>
            (provider.controllers ?? []).map((controller) => () => controller),
        ),
    ];
}

/** The packages every space is born with as base installations: the installer and what it uses. */
export const SPACE_KERNEL = [stackPackage, objectPackage, settingPackage];

/** Serve a space's stack and other packages over its one database, the stack first. */
export function serveSpace(options: SpaceOptions): {
    readonly store: ObjectStore;
    readonly packages: readonly PackageHandler[];
} {
    // serve each other package once, with its resources provisioned next to the stack's
    const others = options.servers.map((server) => {
        // read the package's definition once, serving it as a static one
        const definition = server.definition(server.installation);
        const resolved = implementPackage(server.package, definition);

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

    // serve the stack with its access objects, then settings and packages, a standby the stack and access only
    const stack = stackServer(
        options,
        others.map((each) => each.served),
    );
    const access = implementPackage(objectPackage, {
        objects: [
            implementRole(),
            implementRelationship(),
            implementInstallations({
                issuer: options.keystore.issuer(options.database, options.principal.id),
            }),
        ],
    });
    const base = (handle: Pick<ObjectPackage, "id" | "name">) =>
        present(options.bases.get(handle.id), `the base installation of ${handle.name}`);
    const born = [stack.at(base(stackPackage)), access.at(base(objectPackage))];
    const store = new ObjectStore({
        servers:
            options.role === "primary"
                ? [
                      ...born,
                      installedSettingsServer(options).at(base(settingPackage)),
                      ...others.map((each) => each.server),
                  ]
                : born,
        principal: options.principal,
        partitions: options.partitions,
        database: options.database,
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
                () => store,
            ),
        origin: { package: stackPackage, machineId: options.machine },
        uplink: options.publisher,
        // root the chain above the space by its placement, as primary or move target
        chain: async (view) => {
            // read where the space is placed, and whether it moves here
            const served = schema.identifier("space").parse(options.principal.id);
            const row = await view.read(placement.table).findFirst({ where: { id: served } });
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

    // verify the installations' assertions to each package with the proofs they spend
    return {
        store,
        packages: store.packages.map((served) => {
            const verifier = InstallationAssertion.verifier(store, served.package.id);

            return {
                ...served,
                verify: (request: Request) =>
                    InstallationAssertion.verify(
                        verifier,
                        request,
                        options.environment.clock.now(),
                    ),
            };
        }),
    };
}

/** Serve the space's own objects. */
function ownObjects(
    options: SpaceOptions,
    providers: ProviderIndex,
    binder: Binder,
    transfers: TransferOptions,
) {
    // serve the space's types with their behaviour
    const { builds, registry } = options;
    const served = [
        implementSnapshot(transfers),
        implementRestore(transfers),
        implementSpace({
            keystore: options.keystore,
            builds,
            publisher: options.publisher,
            isEnforcingEntitlements: options.isEnforcingEntitlements,
        }),
        ...providers.provided(),
        implementInstallation({
            resources: providers.objects(),
            builds,
            registry,
            binder,
            runtimes: options.runtimes.flatMap((runtime) =>
                runtime.name === PROCESS_RUNTIME ? [] : [runtime.name],
            ),
            accounts: options.accounts,
            forge: options.forge,
            issuer: options.keystore.issuer(options.database, options.principal.id),
            isEnforcingEntitlements: options.isEnforcingEntitlements,
            database: options.database,
            bundled: options.bundled,
        }),
        implementInstallationRevision(builds, registry),
        binder.serve(),
        implementDeployment({
            providers,
            bindables: options.bindables,
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

/** Serve the stack's stored types, each implemented one in place, then the other served types. */
function stackObjects(served: readonly ObjectType[]): ObjectType[] {
    const own = Object.values(stackPackage.objects)
        .filter((type) => type.storage !== "external")
        .map((type) => served.find((each) => each.same(type)) ?? type);

    return [...own, ...served.filter((type) => !own.includes(type))];
}

/** Serve the space's installations' settings, read from the builds they install. */
function installedSettingsServer(options: SpaceOptions): PackageImplementation {
    const declared = Setting.declared(async (written, view) => {
        // read the declaring package's build from the release serving it: an installation's revision, else the machine's shipped release
        const spaceId = schema.identifier("space").parse(written.scope);
        const shipped = options.bundled.get(written.packageId);
        const installed = await view
            .read(installation.table)
            .findMany({ where: { scope: spaceId, packageId: written.packageId } });
        const following = await view.read(rollout.table).findMany({
            where: { id: { in: installed.map((row) => row.id) }, revisionId: { isNotNull: true } },
        });

        return following.length > 0 || shipped === undefined
            ? Rollout.reader(
                  view,
                  spaceId,
                  written.packageId,
                  written.installation,
                  options.builds.packages,
              )
            : options.builds.pull(options.registry, written.packageId, shipped.manifest);
    });

    return settingServer({ declared, after: [rollout] });
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
        httpsProvider,
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
        store: options.store,
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
        Follow.of(spaceShape, {
            keys: [
                {
                    key: { scope: Scope.universe.id, space: served, machine: options.machine },
                    subject: host,
                },
            ],
        }),
    ];

    // follow nothing the space signs for on a standby, which has none of its keys
    if (options.role !== "primary") {
        return follows;
    }

    return [
        ...follows,
        // follow the residencies and its residency's residents as the space, a resident itself, so it decides on them
        Follow.of(residencyShape, { keys: [{ key: { scope: Scope.universe.id } }] }),
        residency.follow(residentsShape, {
            key: async (row) =>
                row.space === served
                    ? { key: { scope: Scope.universe.id, residency: row.id } }
                    : undefined,
        }),
        // follow the profiles of the people related to the space once it is placed here
        relationship.follow(profileShape, {
            where: { subjectPackageId: user.typeReference.packageId, subjectType: user.name },
            key: async (row, view) => {
                const placed = await view
                    .read(placement.table)
                    .findFirst({ where: { id: served } });
                const isPerson = row.subjectRelation === null && user.identifies(row.subjectId);

                return isPerson && placed?.machineId === options.machine
                    ? { key: { scope: Scope.universe.id, user: user.identifier(row.subjectId) } }
                    : undefined;
            },
        }),
        // follow the grants people delegate to the space once it is placed here
        placement.follow(grantShape, {
            key: async (row) =>
                row.id === served && row.machineId === options.machine
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
    if (store.readServer(machine) === undefined || store.replicates(machine)) {
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
