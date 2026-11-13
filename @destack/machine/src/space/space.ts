import type * as packageRuntime from "@destack/package/runtime";
import type { defineDatabase } from "@destack/db";
import { openStore, ownServers } from "./store.ts";
import type { ClaimPrimary, ExternalStorage } from "@destack/object/server";
import type { Follow, ObjectType } from "@destack/object";
import {
    AccessContext,
    type Authentication,
    Authorization,
    Caller,
    CONTAINED,
    Grant,
    principal,
} from "@destack/access";
import { Exchange, PartitionId } from "@destack/access/token";
import { type Identity, IdentityKeystore, type IdentityRegistry } from "@destack/account/identity";
import accountPackage from "@destack/account/package";
import {
    and,
    BlobStore,
    Change,
    Condition,
    type Controller,
    type DatabaseConnection,
    eq,
    inArray,
    isNotNull,
    isNull,
    Projection,
    Snapshot,
    TABLE,
    Transaction,
} from "@destack/db";
import { MAIN_DATABASE } from "@destack/stack/declare";
import stackPackage, { STACK_INSTALLATION } from "@destack/stack/package";
import { HostAddress, type Network, type SocketAddress } from "@destack/environment";
import type { Deriver } from "@destack/identity";
import {
    CallKey,
    ClaimLookup,
    claimTable,
    type CursorPage,
    Egress,
    HookSandbox,
    installation,
    InstallationBinding,
    InstallationMount,
    ObjectDescription,
    Observation,
    type Package,
    PackageDescription,
    PackageMount,
    type ProcedureRouter,
    Provisioned,
    SpaceMount,
} from "@destack/object";
import {
    defineWorkload,
    InstallationRoot,
    PackageContext,
    type ServerContext,
    type WorkloadContext,
    WorkloadInstance,
} from "@destack/object/workload";
import { BuildReader, type Lock } from "@destack/package/manifest";
import {
    type Connector,
    EGRESS_PROVIDER,
    type ExternalClient,
    type Lease,
    ResourceDeclaration,
    ResourceHandle,
    ResourceId,
    ResourceRecord,
} from "@destack/resource";
import { BucketKind } from "@destack/bucket";
import type { CatalogueBucketHost } from "@destack/bucket/provider";
import { ResourceContext } from "@destack/resource/context";
import {
    Health,
    type ImplementationInput,
    implementPackage,
    ObjectSink,
    type ObjectStore,
    type PackageHandler,
    type PackageImplementation,
    RemoteStore,
} from "@destack/object/server";
import { Bookmark, type ClientOf, createClient } from "@destack/object/client";
import { type Credential, Session } from "@destack/account/client";
import objectPackage from "@destack/object/package";
import { type graph, PackageId, PackageRelease, Sandbox } from "@destack/package";
import { Digest, type Identifier, present, schema } from "@destack/schema";
import {
    ObjectReference,
    type Publisher,
    REQUEST_LIFETIME_MILLISECONDS,
    RequestId,
    Scope,
    Subject,
    type Subscription,
    Uplink,
} from "@destack/sync";
import {
    ACTIVE_BUILD_STATES,
    Binding,
    bucket,
    build,
    Build,
    database,
    Database,
    dependency,
    deployment,
    Deployment,
    installationRevision,
    PROCESS_RUNTIME,
    type InstallationRollout,
    ObjectBindable,
    Rollout,
    rollout,
    SPACE_PROVIDER,
    type SpaceProvider,
    StoreProvider,
} from "@destack/stack/object";
import { type PublishStore } from "../installation/build.ts";
import { StackError } from "@destack/stack/error";
import type {
    EgressCaller,
    InstanceSpec,
    WorkloadInstallation,
    WorkloadResource,
} from "../runtime/runtime.ts";
import { RolloutTarget } from "../installation/target.ts";
import { Forge, Registry } from "@destack/registry/client";
import { S3Credentials, type S3Server } from "@destack/bucket/s3";
import connectionPackage, { CONNECTION_INSTALLATION } from "@destack/connection/client";
import type { ConnectionLease } from "@destack/connection/object";
import { BuildStore, type ImportClosure } from "@destack/build/store";
import type { Volume } from "./storage.ts";
import { PackageError } from "@destack/package/error";
import { serveS3 } from "../resource/s3.ts";
import type { SpaceRole } from "./placement.ts";
import type { OtlpSignal } from "@destack/telemetry/otlp";
import { Gateway } from "../gateway/index.ts";
import { DRAIN_TIMEOUT_MILLISECONDS, type Machine } from "../machine/index.ts";
import { Reading, SpaceMeter } from "../meter/index.ts";
import { MachineError } from "../error/index.ts";
import type { GrantRequest } from "@destack/account/authentication";
import * as accountObject from "@destack/account/object";
import * as stackObject from "@destack/stack/object";

/** The origin in-process installations call their space's stack service at, in process. */
const STACK_ORIGIN = "http://stack.localhost";

/** The redirects a fetch follows at most, as the Fetch standard limits them. */
const MAX_REDIRECTS = 20;

/** The HTTP statuses a fetch follows as redirects. */
const REDIRECT_STATUSES: ReadonlySet<number> = new Set([301, 302, 303, 307, 308]);

/** The headers a redirect drops toward another origin, as the Fetch standard's credentials. */
const CREDENTIAL_HEADERS = ["authorization", "cookie", "proxy-authorization"];

/** The headers describing a request's body, which a redirect drops with the body (Fetch 4.4). */
const BODY_HEADERS = [
    "content-encoding",
    "content-language",
    "content-length",
    "content-location",
    "content-type",
];

/** The origin below which in-process installations address other installations. */
const EGRESS_ORIGIN = `http://egress.localhost${Egress.path}`;

/** The mount of OTLP/HTTP, which the installations taking a space's telemetry serve. */
const OTLP_MOUNT = "otlp";

/** A mount an installation of the space declares, at its path below its package's path. */
export interface Mount {
    /** The installation answering it. */
    readonly installationId: Identifier<"installation">;
    /** The installation's package. */
    readonly packageId: PackageId;
    /** The mount's name in its package. */
    readonly name: string;
    /** The mount's path below the package's path. */
    readonly path: string;
}

/** An installation with the build it runs: the revision in effect and the lock the machine runs. */
export interface InstallationBuild {
    /** The installation's rollout. */
    readonly rollout: InstallationRollout;
    /** The revision in effect. */
    readonly revisionId: Identifier<"installation-revision">;
    /** The lock the machine runs. */
    readonly lock: Digest;
    /** The lock the revision recorded, which differs while a host installation catches up. */
    readonly recorded: Digest;
    /** The declarations of the build the machine runs. */
    readonly declarations: graph.Declaration[];
}

/** An installation's server as one start serves it: its load, and its server or sandbox. */
interface ServedEntry {
    /** Its build's load: a trusted server, absent for a build without one, a sandbox, or none. */
    readonly load:
        | { readonly kind: "host"; readonly server: PackageImplementation | undefined }
        | { readonly kind: "sandbox" }
        | { readonly kind: "unserved" };
    /** The server or sandbox serving it, absent for one that failed to load. */
    readonly served:
        | { readonly kind: "server"; readonly server: PackageImplementation }
        | { readonly kind: "sandbox"; readonly sandbox: HookSandbox }
        | undefined;
}

/** One start of a space's workload: its role, the store and gateway it serves, and its buckets. */
export interface SpaceStart {
    /** The space's role on the machine this start serves. */
    readonly role: SpaceRole;
    /** The workload's context of this start. */
    readonly context: WorkloadContext;
    /** The partitions the start serves: the space, and its account when it is the home. */
    readonly partitions: readonly string[];
    /** The host of the space's buckets. */
    readonly buckets: CatalogueBucketHost;
    /** The S3 API over the space's buckets. */
    readonly s3: Pick<S3Server, "fetch" | "serve">;
    /** The forge whose references and tags installations select, null on a machine not building. */
    readonly forge: Forge | null;
    /** The account package's uplink of this start, signed as the space or its installations. */
    readonly uplink: Uplink;
    /** The claim type of unique index names, and the universe that serves it. */
    readonly claims: ClaimPrimary;
    /** The graphs of the space's builds, served as external objects. */
    readonly external: ExternalStorage;
    /** The shapes the start follows beside the space's own, such as the forge's references. */
    readonly follows: readonly Follow[];
    /** The start's own controllers: the space's reconcile and a rated primary's meter. */
    readonly controllers: NonNullable<ImplementationInput["controllers"]>;
    /** The start's own cursors, such as a rated primary's readings. */
    readonly cursors: NonNullable<ImplementationInput["cursors"]>;
    /** The installations a primary serves at the start, from the machine's builds. */
    readonly installed: readonly InstallationBuild[];
    /** The mounts the served installations declare. */
    mounts: readonly Mount[];
    /** The space's store, once it serves. */
    readonly store: PromiseWithResolvers<ObjectStore>;
    /** The gateway routing the installations' calls, once the store serves. */
    readonly gateway: PromiseWithResolvers<Gateway>;
}

/** An installed server as a start reads it once: its types, its providers and its server. */
interface ServerRead {
    /** The package's types, the handled ones in place, and the providers it serves. */
    readonly served: {
        readonly objects: readonly ObjectType[];
        readonly providers: ImplementationInput["providers"];
    };
    /** The resolved server. */
    readonly server: PackageImplementation;
    /** Whether the server provides, binds or provisions what the space's own servers read. */
    readonly isProviding: boolean;
}

/** A space a machine hosts: its identity, storage, placement, installations and workload. */
export class Space {
    /** The space. */
    readonly id: Identifier<"space">;
    /** The machine hosting the space. */
    readonly machine: Machine;
    /** The space's volume on its machine: its database, events, blobs, buckets and sockets. */
    readonly volume: Volume;
    /** The keystore of the space's keys under the machine's keyring. */
    readonly keystore: IdentityKeystore;
    /** The account package's URL at the universe, and the universe's issuer. */
    readonly universe: { readonly accounts: string; readonly issuer: string };
    /** The space's blobs, stored while its rows reference them. */
    readonly blobs: BlobStore;
    /** The space's own builds, reading through to its machine's builds. */
    readonly builds: BuildStore;
    /** The import maps of the space's locks, by lock, runtime and base, each mapped once. */
    readonly #closures = new Map<string, Promise<ImportClosure>>();
    /** Whether the space serves every installation at its revision, and why not. */
    readonly health: Health;
    /** The identities of the space and its installations, reusing their tokens, by subject. */
    readonly #identities = new Map<string, Identity>();
    /** The current start's servers, by installation and revision, kept while they stay served. */
    #served:
        | { readonly signal: AbortSignal; readonly entries: ReadonlyMap<string, ServedEntry> }
        | undefined;
    /** The space's running workload, once started. */
    #instance: WorkloadInstance | undefined;
    /** The workload's current start. */
    #start: SpaceStart | undefined;
    /** Each installed server the current start read, by the server. */
    #reads = new Map<PackageImplementation, ServerRead>();
    /** The current start's own servers, with the installed servers they read. */
    #own:
        | {
              readonly over: readonly PackageImplementation[];
              readonly servers: readonly PackageImplementation[];
          }
        | undefined;

    /** Host a space over its volume on a machine, its keys under the machine's keyring. */
    constructor(
        fields: Pick<Space, "id" | "machine" | "volume" | "universe" | "blobs"> & {
            /** The universe's registry of identities, which the space's keystore publishes to. */
            readonly identities: IdentityRegistry;
        },
    ) {
        // set the space, its machine and storage, and its keys under the machine's keyring
        this.id = fields.id;
        this.machine = fields.machine;
        this.volume = fields.volume;
        this.universe = fields.universe;
        this.blobs = fields.blobs;
        this.builds = new BuildStore({
            blobs: fields.blobs,
            environment: fields.machine.environment,
            sources: [fields.machine.builds],
            pullFile: (built, file) => fields.machine.pullFile(built, file),
        });
        this.keystore = new IdentityKeystore(
            fields.machine.keyring,
            fields.identities,
            fields.machine.environment,
        );
        this.health = new Health(fields.id);
    }

    /** The space's database. */
    get database(): DatabaseConnection {
        return this.volume.database;
    }

    /** Open a space on its machine and start its workload. */
    static start(machine: Machine, id: Identifier<"space">): Promise<Space> {
        return machine.scope.span("destack.space.start", { "destack.space.id": id }, async () => {
            // open the space's volume and its blobs
            const volume = await machine.volumes.open(id);
            try {
                const { environment } = machine;
                const blobs = new BlobStore({
                    storage: volume.blobs,
                    database: volume.database,
                    environment,
                });
                const space = new Space({
                    id,
                    machine,
                    volume,
                    universe: await machine.universe(),
                    identities: await machine.identities(),
                    blobs,
                });

                // start the machine workload serving the space
                space.#instance = await WorkloadInstance.start(spaceWorkload, {
                    environment,
                    resources: new ResourceContext().bind(machineSpace, space),
                    callKey: () => space.callKey(),
                    ...(machine.alarm === undefined ? {} : { alarm: machine.alarm }),
                    report: (error) => machine.report(error),
                    serverOptions: (handle) => ({
                        environment,
                        authenticate: (request) => machine.authenticate(request, handle.id, id),
                        authorizeMachine: (call) => machine.authorizeSpace(call),
                        drainTimeout: DRAIN_TIMEOUT_MILLISECONDS,
                        instance: id,
                    }),
                });

                return space;
            }
            // close the space's volume when its start fails, as a close does
            catch (error) {
                await volume.close();
                throw error;
            }
        });
    }

    /** Prepare a start of the space's workload: its role, buckets, forge, uplink and jobs. */
    prepare(context: WorkloadContext): Promise<SpaceStart> {
        return this.machine.scope.span(
            "destack.space.prepare",
            { "destack.space.id": this.id },
            async () => {
                // open the space's keys, and serve its buckets over S3 with credentials its root secret derives
                const { volume, machine } = this;
                await this.open();
                const buckets = volume.buckets(async (scope) => {
                    const root =
                        scope === this.id
                            ? await this.keystore.deriver(this.database, scope)
                            : undefined;

                    return root === undefined ? undefined : S3Credentials.derive(root, scope);
                });

                // start serving a primary's applied installations, following the forge's references on a building machine
                const role = await this.role();
                const { installed, unavailable } =
                    role === "primary"
                        ? await this.installed("machine")
                        : { installed: [], unavailable: [] };
                await this.#root(installed, unavailable);
                const forge = this.forge();
                const accounts = this.accounts();
                const isMetered = machine.skus !== undefined && role !== "standby";
                const start: SpaceStart = {
                    role,
                    context,
                    partitions: await this.partitions(),
                    buckets,
                    s3: serveS3(buckets, this.database, machine.environment),
                    forge,
                    uplink: this.uplink(),
                    claims: {
                        object: accountObject.claim,
                        home: RemoteStore.of(accounts.store, Scope.universe.id),
                    },
                    external: volume.graphs(this.builds),
                    follows: forge === null ? [] : forge.follows(),
                    controllers: [
                        () => this.controller(),
                        ...(isMetered
                            ? [
                                  (store: ObjectStore) =>
                                      new SpaceMeter(store, {
                                          space: this.id,
                                          measure: (from, to) => this.measure(from, to),
                                          skus: present(machine.skus, "the SKUs rating the space"),
                                          account: () => this.account(),
                                          traffic: (signal) => machine.traffic.next(signal),
                                      }).controller(),
                              ]
                            : []),
                    ],
                    cursors: isMetered
                        ? [
                              // append each account's uses of a page an hour at a time, kept while a billing call may retry
                              stackObject.reading.cursor("metering", (page) => this.append(page), {
                                  hold: { milliseconds: REQUEST_LIFETIME_MILLISECONDS },
                                  batch: { hours: 1 },
                              }),
                          ]
                        : [],
                    installed,
                    mounts: Space.#mounts(installed),
                    store: Promise.withResolvers(),
                    gateway: Promise.withResolvers(),
                };
                this.#start = start;
                this.#reads = new Map();
                this.#own = undefined;

                return start;
            },
        );
    }

    /** Open the forge installations follow, none on a machine that builds nothing. */
    forge(): Forge | null {
        const { machine } = this;
        const lookup = ClaimLookup.of(machine.directory.snapshot.read(claimTable));

        return machine.isBuilding
            ? new Forge(lookup, (account) => this.#residencyHome(account), machine.id)
            : null;
    }

    /** Serve one start of the space's workload: its store, installations, gateway and meters. */
    serve(context: WorkloadContext): Promise<{ readonly packages: readonly PackageHandler[] }> {
        return this.machine.scope.span(
            "destack.space.serve",
            { "destack.space.id": this.id },
            async () => {
                // prepare the start, then serve the store and route the installations' calls through the gateway
                const start = await this.prepare(context);
                try {
                    const { servers, sandboxes } = await this.#servers(
                        start.installed,
                        context.signal,
                    );
                    const store = openStore(this, start, this.#storeServers(servers), sandboxes);
                    start.store.resolve(store);
                    start.gateway.resolve(new Gateway(this, start, store));

                    return { packages: store.packages };
                }
                // reject the waits, each handled here as well as by its callers
                catch (error) {
                    start.store.reject(error);
                    start.gateway.reject(error);
                    start.store.promise.catch(() => undefined);
                    start.gateway.promise.catch(() => undefined);
                    throw error;
                }
            },
        );
    }

    /** Read the current start's store, once it serves. */
    store(): Promise<ObjectStore> {
        return this.#current().store.promise;
    }

    /** Serve an installed set in this start: load, apply and record its changed servers. */
    async #apply(installed: readonly InstallationBuild[]): Promise<void> {
        // apply the changed installations' servers to the store and serve their packages
        const start = this.#current();
        const store = await start.store.promise;
        await this.#applyServers(installed, start.context.signal, (servers, sandboxes) =>
            start.context.serve(store.apply(this.#storeServers(servers), sandboxes)),
        );

        // read the mounts the set declares
        start.mounts = Space.#mounts(installed);
    }

    /** Open a package's release the space runs, or a named version, absent for none. */
    async release(
        view: Snapshot,
        spaceId: Identifier<"space">,
        packageId: PackageId,
        version?: string,
    ): Promise<BuildReader | undefined> {
        // read the installed release and the distribution's
        const { releases, linked, shipped } = this.machine;
        const installed = await RolloutTarget.reader(view, spaceId, packageId, undefined, {
            forge: this.#current().forge,
            registry: releases,
            linked,
            shipped,
            space: this,
        });
        const distributed = PackageRelease.newest(releases.distribution, packageId);
        const isInstalled =
            installed !== undefined &&
            (version === undefined || installed.manifest.package.version === version);

        // open the installed release, unless a value names another version
        if (isInstalled) {
            return installed;
        }
        // pull a named version
        else if (version !== undefined) {
            return this.root(packageId, await releases.find(packageId, version));
        }
        // pull the distribution's release of a package the space does not install
        else if (distributed !== undefined) {
            return this.root(packageId, distributed.lock);
        }
        // run nothing of a package outside the distribution
        else {
            return undefined;
        }
    }

    /** List the start's servers: the stack's, access's and settings', then the installed ones. */
    #storeServers(installed: readonly PackageImplementation[]): readonly PackageImplementation[] {
        // read each installed server once, serving its definition as a static one
        const start = this.#current();
        const reads = installed.map(
            (server) => [server, this.#reads.get(server) ?? this.#readServer(server)] as const,
        );
        this.#reads = new Map(reads);

        // build the own servers again once the servers providing, binding or provisioning change
        const over = reads.filter(([, read]) => read.isProviding).map(([server]) => server);
        const earlier = this.#own;
        const own =
            earlier !== undefined &&
            earlier.over.length === over.length &&
            earlier.over.every((server, index) => server === over[index])
                ? earlier.servers
                : ownServers(
                      this,
                      start,
                      installed,
                      reads.map(([, read]) => read.served),
                  );
        this.#own = { over, servers: own };

        // serve a primary's installed packages beside its own, a standby its own alone
        return start.role === "primary" ? [...own, ...reads.map(([, read]) => read.server)] : own;
    }

    /** Read an installed server: its types, the handled ones in place, and its resolved server. */
    #readServer(server: PackageImplementation): ServerRead {
        // read the package's definition once, serving it as a static one
        const definition = server.definition(PackageContext.of(server.installation, this.blobs));
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

    /** Read what the space used over an interval, with the traffic the machine counted for it. */
    async measure(from: number, to: number): Promise<Reading[]> {
        return this.volume.measure(this.buckets, from, to, this.machine.traffic);
    }

    /** Send an installation's call out through the current start's gateway. */
    async #egress(caller: EgressCaller, request: Request): Promise<Response> {
        return (await this.#current().gateway.promise).send(caller, request);
    }

    /** Run an installation's hooks in a sandbox, their calls sent through the gateway. */
    #sandbox(
        row: InstallationRollout,
        server: ServerContext,
        declarations: readonly graph.Declaration[],
    ): HookSandbox {
        return new HookSandbox({
            server,
            declarations,
            send: async (request) =>
                (await this.#current().gateway.promise).hook(row.installationId, request),
        });
    }

    /** The space as a principal. */
    get principal(): Subject {
        return principal.space.reference(Scope.universe.id, this.id);
    }

    /** Start the space's identity where the directory places it here, then key its columns. */
    open(): Promise<void> {
        // open the space's keys and replicas in its span
        return this.machine.scope.span(
            "destack.space.open",
            { "destack.space.id": this.id },
            async () => {
                // start the identity of a space placed here without keys on the machine yet
                const { keystore } = this;
                const isRetained = await keystore.hasKeys(this.database, this.id);
                if ((await this.role()) === "primary" && !isRetained) {
                    await keystore.generate(this.database, this.id);
                }

                // key the space's sensitive columns with its identity's column key, once its keys live here
                if (await keystore.hasKeys(this.database, this.id)) {
                    await keystore.key(this.database, this.id);
                }
                this.#identities.clear();
            },
        );
    }

    /** Read the space's identity, refusing a space without keys on this machine. */
    async identity(): Promise<Deriver> {
        const kept = await this.keystore.deriver(this.database, this.id);

        return present(kept, `the keys of ${this.id}, which this machine keeps`);
    }

    /** Read the key the space's identity derives for fingerprinting sensitive call inputs. */
    async callKey(): Promise<CryptoKey> {
        return CallKey.of(await this.identity(), this.machine.environment.crypto);
    }

    /** Pull a lock with its closures into the machine's builds, from the space or its forge. */
    async pull(packageId: PackageId, lock: Digest): Promise<Lock> {
        // pull a published lock from its forge
        if (!(await this.builds.contains(lock))) {
            return await this.machine.pull(packageId, lock);
        }

        // pull the space's own lock from the space's builds
        const { builds, targets } = this.machine;
        await builds.pull(this.builds, { kind: "lock", digest: lock }, targets);

        return await builds.lock(lock);
    }

    /** Open a lock's root build, pulling a published one with its closures first. */
    async root(packageId: PackageId, lock: Digest): Promise<BuildReader> {
        return await this.builds.contents((await this.pull(packageId, lock)).root);
    }

    /** Read a package's build: the space's own, else a published one listed from its forge. */
    async read(packageId: PackageId, manifest: Digest): Promise<BuildReader> {
        // read the space's own build
        if (await this.builds.contains(manifest)) {
            return await this.builds.contents(manifest);
        }

        // list a published build's manifest once, the linked release's in place of another
        const { builds, linked, releases } = this.machine;
        const release = linked.get(packageId);
        const resolved = release === undefined ? manifest : (await builds.lock(release.lock)).root;
        if ((await builds.find(resolved)) === undefined) {
            const root = { kind: "manifest", digest: resolved } as const;
            await builds.pull(await releases.store(packageId, root), root, []);
        }

        return await this.builds.contents(resolved);
    }

    /** Map a lock's imports for a runtime once, with the builtins the machine links. */
    imports(lock: Digest, runtime: packageRuntime.Runtime, base: string): Promise<ImportClosure> {
        // map each lock once per runtime and base, forgetting a failed map
        const key = `${lock} ${runtime} ${base}`;
        const known = this.#closures.get(key);
        if (known !== undefined) {
            return known;
        }
        const mapping = (async () => {
            const read = await this.builds.lock(lock);
            const linked = await this.machine.linkedLocks();

            return await this.machine.builds.imports(read, runtime, base, linked, (digest) =>
                this.builds.get(digest),
            );
        })();
        this.#closures.set(key, mapping);
        mapping.catch(() => this.#closures.delete(key));

        return mapping;
    }

    /** Read the space's role here: primary where the directory places it on this machine. */
    async role(): Promise<SpaceRole> {
        const [placed] = await accountObject.Placement.find(this.machine.directory.snapshot, [
            this.id,
        ]);

        return present(placed, `the placement of ${this.id}`).machine === this.machine.id
            ? "primary"
            : "standby";
    }

    /** Read the account the space is placed in. */
    async account(): Promise<Identifier<"account">> {
        const [placed] = await accountObject.Placement.find(this.machine.directory.snapshot, [
            this.id,
        ]);

        return schema
            .identifier("account")
            .parse(present(placed, `the placement of ${this.id}`).scope);
    }

    /** Act as the space or one of its installations, calling out as it, reusing its tokens. */
    identityOf(subject: Subject): Identity {
        // reuse the subject's identity and its tokens
        const key = Subject.key(subject);
        const existing = this.#identities.get(key);
        if (existing !== undefined) {
            return existing;
        }

        // open it with the space's keys toward the universe's issuer
        const opened = this.keystore.identityOf(this.database, subject, {
            issuer: this.universe.issuer,
            send: (request) => this.machine.send(request),
            environment: this.machine.environment,
        });
        this.#identities.set(key, opened);

        return opened;
    }

    /** Call packages with a credential through the machine's network, toward its universe. */
    session(credential: Credential): Session {
        return new Session(credential, {
            universe: new URL(this.universe.issuer).origin,
            send: (request) => this.machine.send(request),
            bookmark: new Bookmark(),
            environment: this.machine.environment,
        });
    }

    /** Call the account package as the machine, or as a principal of the space. */
    accounts(subject?: Subject): ClientOf<typeof accountPackage> {
        return createClient(
            accountPackage,
            {
                url: this.universe.accounts,
                fetch:
                    subject === undefined
                        ? this.machine.fetch(accountPackage.id)
                        : this.session(this.identityOf(subject).granted).fetch(accountPackage.id),
            },
            this.machine.environment,
        );
    }

    /** Replicate account rows: the machine's own as the machine, the rest as the space. */
    uplink(): Uplink {
        // open the account store as the space or a signer, and as the machine
        const signed = (signer?: Subject) => this.accounts(signer ?? this.principal).store;
        const machine = this.accounts().store;

        // follow as the machine what is decided for it
        const followerOf = async (subscription: Subscription) => {
            // read whether the follow is decided for the machine
            const { subject } = subscription;
            const isMachine = subject !== undefined && principal.machine.is(subject);

            // follow the space's chain as the machine on a standby
            const isReceiving =
                subscription.name === ObjectSink.chainName(this.id) &&
                (await this.role()) === "standby";

            return isMachine || isReceiving ? machine : signed();
        };

        return {
            async *stream(subscription, signal) {
                yield* await (await followerOf(subscription)).stream(subscription, { signal });
            },
            request: async (subscription) => {
                await (await followerOf(subscription)).request({ subscription });
            },
            receive: async (mutation, signer) => {
                // send the machine's universe writes as the machine, and the space's changes as their signer
                const sender = signer === undefined ? machine : signed(signer);

                return (await sender.receive({ mutation })).results;
            },
            call: (call, scope, signer) => {
                // ask the universe as the machine, and for the space as the query's signer
                const sender = signer === undefined ? machine : signed(signer);

                return sender.call({ scope, call });
            },
        };
    }

    /** Follow a partition at its home, where the directory locates it, as each follow's subject. */
    publisherAt(home: string, partition: string): Publisher {
        // sign as the installation a follow is decided for, else as the space with a universe grant
        const bound = PartitionId.parse(partition);
        const fetchOf = (subject: Subject | undefined) =>
            subject !== undefined && principal.installation.is(subject)
                ? this.session(this.identityOf(subject)).fetch(objectPackage.id, {
                      partition: bound,
                  })
                : this.session(this.identityOf(this.principal).granted).fetch(objectPackage.id, {
                      partition: bound,
                  });
        const at = (subscription: Subscription) =>
            this.machine.directory.publisher(
                home,
                fetchOf(subscription.subject),
                this.machine.environment,
            );

        return {
            stream: (subscription, signal) => at(subscription).stream(subscription, signal),
            request: (subscription) => at(subscription).request(subscription),
        };
    }

    /** Serve each installation main applied: a trusted one in the store, another in its sandbox. */
    async #servers(
        installed: readonly InstallationBuild[],
        signal: AbortSignal,
    ): Promise<{
        readonly servers: readonly PackageImplementation[];
        readonly sandboxes: readonly HookSandbox[];
        readonly revisions: ReadonlyMap<
            Identifier<"installation">,
            Identifier<"installation-revision">
        >;
    }> {
        // keep this start's servers served before, by installation, revision and lock
        const earlier: ReadonlyMap<string, ServedEntry> =
            this.#served?.signal === signal ? this.#served.entries : new Map();
        const keys = installed.map(({ rollout: row, revisionId, lock }) =>
            Space.#key(row.installationId, revisionId, lock),
        );

        // load the server of each other installation the store runs from its build
        const loaded = await Promise.all(
            installed.map(async ({ rollout: row, lock }, index) => {
                // take the earlier load, else load a trusted one, reporting a server that fails to load
                const kept = earlier.get(present(keys[index], "the installation's key"));
                const isLinked = this.machine.linked.get(row.installation.packageId)?.lock === lock;
                if (kept !== undefined) {
                    return kept.load;
                } else if (row.installation.isolation !== "host" && !isLinked) {
                    return { kind: "sandbox" } as const;
                }
                try {
                    return { kind: "host", server: await this.machine.server(lock) } as const;
                } catch (error) {
                    const name = row.installation.name ?? row.installationId;
                    this.#current().context.report(
                        new Error(
                            `installation '${name}' serves nothing: its server did not load`,
                            {
                                cause: error,
                            },
                        ),
                    );

                    return { kind: "unserved" } as const;
                }
            }),
        );
        const trusted = loaded.flatMap((each) =>
            each.kind === "host" && each.server !== undefined ? [each.server] : [],
        );

        // serve each installation in the store or in its sandbox, keeping the earlier ones
        const entries = new Map<string, ServedEntry>();
        const servers: PackageImplementation[] = [];
        const sandboxes: HookSandbox[] = [];
        const revisions = new Map<
            Identifier<"installation">,
            Identifier<"installation-revision">
        >();
        for (const [index, entry] of installed.entries()) {
            const key = present(keys[index], "the installation's key");
            const load = present(
                loaded[index],
                `the load of installation ${entry.rollout.installationId}`,
            );
            const served =
                earlier.get(key)?.served ??
                (await this.#serve(entry, load, { signal, trusted }).catch((error: unknown) => {
                    // report an installation that fails to start, and start the space without it
                    const name = entry.rollout.installation.name ?? entry.rollout.installationId;
                    this.#current().context.report(
                        new Error(
                            `installation '${name}' serves nothing: its server did not start`,
                            { cause: error },
                        ),
                    );

                    return undefined;
                }));
            entries.set(key, { load, served });

            // record the revision of each installation the start serves, in its store or its sandbox
            if (served !== undefined) {
                revisions.set(entry.rollout.installationId, entry.revisionId);
            }
            if (served?.kind === "server") {
                servers.push(served.server);
            } else if (served?.kind === "sandbox") {
                sandboxes.push(served.sandbox);
            }
        }
        this.#served = { signal, entries };

        return { servers, sandboxes, revisions };
    }

    /** Build the controller applying the space's installed set at each start and change of it. */
    controller(): Controller<typeof Scope.table> {
        return {
            name: "space",
            table: Scope.table,
            pending: { scope: this.id },
            // apply again as the installed set, its rollouts or the main database's status move
            watches: [installation.table, rollout.table, database.table].map((table) => ({
                table,
                isObserving: true,
                keys: () => [{ scope: this.id }],
            })),
            reconcile: () => this.reconcile(),
        };
    }

    /** Apply an installed set to a start: load its servers, hand a changed set over, record it. */
    async #applyServers(
        installed: readonly InstallationBuild[],
        signal: AbortSignal,
        handover: (
            servers: readonly PackageImplementation[],
            sandboxes: readonly HookSandbox[],
        ) => Promise<void> | void,
    ): Promise<void> {
        // load the set's servers, reusing this start's, and hand them over once the served set changes
        const before = this.#served?.signal === signal ? this.#serving() : undefined;
        const { servers, sandboxes, revisions } = await this.#servers(installed, signal);
        const after = this.#serving();
        if (before === undefined || before.symmetricDifference(after).size > 0) {
            await handover(servers, sandboxes);
        }

        // record the revision of each installation the start serves
        await Rollout.serve(await this.#current().store.promise, this.id, revisions);
    }

    /** List the keys of the installations this start serves. */
    #serving(): Set<string> {
        const entries = this.#served?.entries ?? new Map<string, ServedEntry>();

        return new Set(
            [...entries].flatMap(([key, entry]) => (entry.served === undefined ? [] : [key])),
        );
    }

    /** Read the build each installation of the space runs, at its revision in effect. */
    async installations(
        open: (packageId: PackageId, lock: Digest) => Promise<BuildReader | undefined>,
    ): Promise<InstallationBuild[]> {
        // read no installations of a space before its birth gives it main
        const snapshot = Snapshot.live(this.database);
        const spaceId = this.id;
        const main = await Database.mainMaybe(snapshot, spaceId);
        if (main === undefined) {
            return [];
        }

        // read the rollouts with their installations, and their revisions with their bindings and builds
        const installations = await Rollout.read(
            snapshot,
            await snapshot.read(rollout.table).findMany({ where: { scope: spaceId } }),
        );
        const revisionIds = installations.flatMap((entry) =>
            [entry.revisionId, entry.appliedRevisionId].filter((id) => id !== null),
        );
        const revisions = await snapshot
            .read(installationRevision.table)
            .findMany({ where: { scope: spaceId, id: { in: revisionIds } } });
        const builds = await snapshot.read(build.table).findMany({
            where: { scope: spaceId, id: { in: revisions.map((entry) => entry.buildId) } },
        });

        // read each revision's build and its declarations, once
        const declared = new Map(
            await Promise.all(
                revisions.map(async (revision) => {
                    // open the revision's build by its installation's package and its lock
                    const built = present(
                        builds.find((each) => each.id === revision.buildId),
                        `the build of revision ${revision.id}`,
                    );
                    const installed = present(
                        installations.find(
                            (entry) =>
                                entry.revisionId === revision.id ||
                                entry.appliedRevisionId === revision.id,
                        ),
                        `the installation of revision ${revision.id}`,
                    );
                    const recorded = Build.lock(built);
                    const lock = RolloutTarget.running(
                        installed.installation,
                        recorded,
                        this.machine,
                    );
                    const reader =
                        lock === undefined
                            ? undefined
                            : await open(installed.installation.packageId, lock);
                    if (lock === undefined || reader === undefined) {
                        return [];
                    }

                    return [
                        [
                            revision.id,
                            {
                                lock,
                                recorded,
                                declarations: await reader.declarations(),
                                objects: await ObjectDescription.read(reader),
                            },
                        ] as const,
                    ];
                }),
            ).then((entries) => entries.flat()),
        );

        // serve each installation at its revision once its targets are bound and main applied its tables, else at the revision it deployed last
        const applied = new Set(main.appliedStates);
        const served = await Promise.all(
            installations.map(async (entry) => {
                // serve a bound revision declaring no object types at once, and one declaring some once main applied them
                const latest =
                    entry.revisionId === null ? undefined : declared.get(entry.revisionId);
                const tables =
                    entry.revisionId === null
                        ? undefined
                        : revisions.find((each) => each.id === entry.revisionId)?.states[
                              MAIN_DATABASE
                          ];
                const isBound =
                    Observation.isCurrent(entry.installation, "Bound") &&
                    entry.installation.conditions["Bound"]?.status === "true";
                const isApplied =
                    latest !== undefined &&
                    isBound &&
                    (latest.objects.length === 0 || (tables !== undefined && applied.has(tables)));
                const revisionId = isApplied ? entry.revisionId : entry.appliedRevisionId;

                return revisionId === null ? [] : [{ rollout: entry, revisionId }];
            }),
        );

        // select each one whose revision binds its own resources, since servers open them at start and read secrets on use
        return served.flat().flatMap(({ rollout: entry, revisionId }) => {
            // read the served revision's build and declarations, leaving out one whose build is absent
            const opened = declared.get(revisionId);
            if (opened === undefined) {
                return [];
            }
            const { lock, recorded, declarations } = opened;

            // serve a stack at once, which runs nothing and whose resources its document instantiates
            const isStack = entry.installation.export !== null;
            const revision = revisions.find((each) => each.id === revisionId);
            const { packageId } = entry.installation;
            const hasBindings = declarations
                .filter((each) => each.provisioned === true)
                .every(
                    (each) =>
                        revision !== undefined &&
                        revision.bindings.some(
                            (bound) => bound.packageId === packageId && bound.name === each.name,
                        ),
                );

            return isStack || hasBindings
                ? [{ rollout: entry, revisionId, lock, recorded, declarations }]
                : [];
        });
    }

    /** Key an installation's server by the installation, its served revision and its lock. */
    static #key(
        installationId: Identifier<"installation">,
        revisionId: Identifier<"installation-revision">,
        lock: Digest,
    ): string {
        return `${installationId}@${revisionId}@${lock}`;
    }

    /** Serve one installation: a loaded server in the store, else its hooks in a sandbox. */
    async #serve(
        { rollout: row, revisionId, lock, declarations }: InstallationBuild,
        load: ServedEntry["load"],
        run: {
            readonly signal: AbortSignal;
            readonly trusted: readonly PackageImplementation[];
        },
    ): Promise<ServedEntry["served"]> {
        // serve a loaded server in the store, activated once its revision and any lent grant exist
        const { signal, trusted } = run;
        const stored = load.kind === "host" ? load.server : undefined;
        if (load.kind === "unserved") {
            return undefined;
        }
        const revision = await Snapshot.live(this.database)
            .read(installationRevision.table)
            .findFirst({ where: { id: revisionId } });
        const { bindings } = present(revision, `revision ${revisionId}`);
        if (stored !== undefined) {
            const caller = this.#callerOnce(row, stored.package, bindings);
            const resources = await this.#resources(row, stored.package, caller, trusted, bindings);
            const context = await this.#context(row, lock, resources, caller);
            const requested = Rollout.permissions(row.installation, stored.package, "space");
            const isLent = Object.entries(requested).some(
                ([scope, each]) => scope !== "space" && each.length > 0,
            );
            const activation = (stopped: AbortSignal) =>
                this.#activate(row, revisionId, isLent, AbortSignal.any([signal, stopped]));

            return { kind: "server", server: stored.at({ ...context, activation }) };
        }
        // run another package's hooks in its sandbox, which binds its resources itself
        else {
            const caller = this.#callerOnce(row, { resources: {} }, bindings);
            const context = await this.#context(row, lock, new ResourceContext(), caller);

            return { kind: "sandbox", sandbox: this.#sandbox(row, context, declarations) };
        }
    }

    /** Activate an installation once it applied its revision and has any grant it borrows by. */
    async #activate(
        target: InstallationRollout,
        revisionId: Identifier<"installation-revision">,
        isLent: boolean,
        signal: AbortSignal,
    ): Promise<void> {
        // wait for both, leaving the wait once the start stops
        await this.#applied(target, revisionId, isLent, signal).catch((error: unknown) => {
            if (!signal.aborted) {
                throw error;
            }
        });
    }

    /** Wait until an installation applied a revision and the grant it borrows by arrived. */
    async #applied(
        target: InstallationRollout,
        revisionId: Identifier<"installation-revision">,
        isLent: boolean,
        signal: AbortSignal,
    ): Promise<void> {
        // wait for the applied revision and for the replica of the grant its space lends it
        const parties = {
            delegator: principal.space.reference(Scope.universe.id, target.scope),
            delegate: principal.installation.reference(target.scope, target.installationId),
        };
        await this.database.log.waitUntil(async () => {
            // read the rollout's applied revision and the grant it borrows by
            const snapshot = Snapshot.live(this.database);
            const [row] = await snapshot
                .read(rollout.table)
                .findMany({ where: { installationId: target.installationId } });
            const isApplied = row?.appliedRevisionId === revisionId;
            const isGranted =
                !isLent ||
                (await Grant.find(snapshot, parties, this.machine.environment.clock.now())) !==
                    undefined;

            return isApplied && isGranted;
        }, signal);
    }

    /** Bind a package's declared resources and secrets to the host's clients for it. */
    async #resources(
        row: InstallationRollout,
        handle: Pick<Package, "resources" | "secrets">,
        caller: () => Promise<EgressCaller>,
        trusted: readonly PackageImplementation[],
        bindings: readonly Binding[],
    ): Promise<ResourceContext> {
        const bound = new ResourceContext();
        for (const declaration of [
            ...Object.values(handle.secrets),
            ...Object.values(handle.resources),
        ]) {
            // refuse a requirement another package declares
            if (!ResourceDeclaration.is(declaration)) {
                throw new TypeError(
                    `installation '${row.installationId}' binds '${declaration.name}', which is no declaration of this process's resource kinds`,
                );
            }
            bound.bind(declaration, await this.#open(row, declaration, caller, trusted, bindings));
        }

        return bound;
    }

    /** Open a declaration an installation binds in this process, through what opens its kind. */
    async #open(
        row: InstallationRollout,
        declaration: ResourceDeclaration<unknown>,
        caller: () => Promise<EgressCaller>,
        trusted: readonly PackageImplementation[],
        bindings: readonly Binding[],
    ): Promise<unknown> {
        // take the target the served revision binds the declaration to
        const { scope } = row;
        const { packageId } = row.installation;
        const found = present(
            bindings.find((each) => each.packageId === packageId && each.name === declaration.name),
            `the binding of ${declaration.name} by ${row.installationId}`,
        );
        const { target } = found;
        const bindable = ObjectBindable.of(trusted).find((each) =>
            each.kind.is(declaration.kind.name, declaration.kind.packageId),
        );
        const provider = [...this.volume.providers, ...StoreProvider.of(trusted)].find((each) =>
            each.kind.is(declaration.kind.name, declaration.kind.packageId),
        );

        // connect to a bound object, such as a secret, or a store resource, such as a vault, through the store
        if (bindable !== undefined) {
            return bindable.connect(this.#current().store.promise, row, found);
        } else if (provider !== undefined && "connect" in provider) {
            return provider.connect(this.#current().store.promise, row, found);
        }
        // call a bound installation through the machine's egress, as an instance's binding does
        else if (InstallationBinding.is(declaration)) {
            return this.#client(declaration, found, caller);
        }
        // call an external resource, such as a connection's API, through the egress below its identifier
        else if (declaration.connectors[EGRESS_PROVIDER] !== undefined) {
            const url = Egress.url(EGRESS_ORIGIN, found.target);
            const send = async (request: Request) => this.#egress(await caller(), request);
            const client: ExternalClient & AsyncDisposable = {
                url,
                fetch: (path, init = {}) => send(new Request(`${url}${path}`, init)),
                [Symbol.asyncDispose]: () => Promise.resolve(),
            };

            return client;
        }
        // open a bucket from the space's buckets
        else if (declaration.kind.is(BucketKind.name, BucketKind.packageId)) {
            return this.#current().buckets.open({ bucketId: bucket.identifier(target) }, scope);
        }
        // connect another resource where its provider placed it, through the connector of that provider
        else if (provider !== undefined) {
            return this.#connect(row, declaration, provider, target, this.volume.connectors);
        }
        // refuse a kind nothing here opens
        else {
            throw StackError.of(
                "RESOURCE_UNSUPPORTED",
                `in-process '${declaration.kind.name}' on this machine is not supported`,
            );
        }
    }

    /** Open a bound installation's client, calling through the machine's egress. */
    async #client(
        declaration: InstallationBinding,
        bound: Pick<Binding, "target" | "targetSpaceId">,
        caller: () => Promise<EgressCaller>,
    ): Promise<unknown> {
        // send each call as the installation's in-store caller
        const send = async (request: Request) => this.#egress(await caller(), request);

        // call a bound installation's procedures below its address
        const mounted = PackageMount.path(declaration.handle.id);
        const url =
            bound.targetSpaceId === null
                ? Egress.url(EGRESS_ORIGIN, await this.#reference(bound))
                : `${Egress.url(EGRESS_ORIGIN, Egress.address(bound.target, bound.targetSpaceId))}${mounted}`;
        const client = createClient<ProcedureRouter, { readonly authentication?: Authentication }>(
            declaration.handle,
            {
                url,
                fetch: send,
                headers: (options) =>
                    Exchange.ask(Exchange.of(options.context?.authentication?.caller)),
            },
            this.machine.environment,
        );

        return Object.assign(client, { [Symbol.asyncDispose]: () => Promise.resolve() });
    }

    /** Read the reference a local binding targets: a dependency's address. */
    async #reference(bound: Pick<Binding, "target" | "targetSpaceId">): Promise<string> {
        // take another space's installation by its name there
        if (bound.targetSpaceId !== null) {
            return bound.target;
        }

        // read the reference of the dependency the binding targets
        const view = Snapshot.live(this.database);
        const { target } = bound;
        const found = await view
            .read(dependency.table)
            .findFirst({ where: { id: dependency.identifier(target) } });

        return ResourceRecord.reference(present(found, `the resource ${target}`));
    }

    /** Build an installation's in-store caller once per start, at its first call. */
    #callerOnce(
        row: InstallationRollout,
        handle: Pick<Package, "resources">,
        bindings: readonly Binding[],
    ): () => Promise<EgressCaller> {
        let built: Promise<EgressCaller> | undefined;

        return () => (built ??= this.#caller(row, handle, bindings));
    }

    /** Read an in-process installation as its egress's caller, with what it binds. */
    async #caller(
        row: InstallationRollout,
        handle: Pick<Package, "resources">,
        bindings: readonly Binding[],
    ): Promise<EgressCaller> {
        // read the process deployment the store runs the installation's server as
        const [deployed] = await Snapshot.live(this.database)
            .read(deployment.table)
            .findMany({
                where: {
                    ...Deployment.live(),
                    scope: row.scope,
                    installationId: row.installationId,
                    revisionId: present(row.revisionId, `the revision of ${row.installation.name}`),
                    runtime: PROCESS_RUNTIME,
                },
                limit: 1,
            });
        const served = present(deployed, `the process deployment of ${row.installation.name}`);

        // take the served revision's bindings to targets here, and the resources they target
        const local = bindings.filter((each) => each.targetSpaceId === null);
        const provided = this.volume.providers.filter(
            (each): each is SpaceProvider => !("connect" in each),
        );
        const resources = await Promise.all(
            provided.map(async (provider) => {
                // read the provider's resources the bindings target, as a deployment captures them
                const { object } = provider;
                const targets = local.filter((each) => object.identifies(each.target));
                const rows =
                    targets.length === 0
                        ? []
                        : await this.database
                              .select()
                              .from(object.table)
                              .where(object.withIds(targets.map((each) => each.target)));

                return rows.flatMap((found): WorkloadResource[] => {
                    const entry = present(
                        local.find((each) => each.target === found.id),
                        `the binding of ${found.id}`,
                    );

                    // refuse a target bound before its provider placed it, which activation rules out
                    if (found.reference === null || found.provider === null) {
                        throw StackError.of(
                            "RESOURCE_NOT_FOUND",
                            `resource '${found.id}' that '${entry.name}' of '${row.installation.name}' binds not found`,
                        );
                    }

                    return [
                        {
                            id: ResourceId.parse(found.id),
                            scope: found.scope,
                            spec: found.spec,
                            reference: found.reference,
                            packageId: entry.packageId,
                            name: entry.name,
                            kind: provider.kind.name,
                            provider: found.provider,
                        },
                    ];
                });
            }),
        );

        // take the installations of other spaces it binds, by the package each declaration calls
        const installations = bindings.flatMap((each): WorkloadInstallation[] => {
            const declared = Object.values(handle.resources).find(
                (declaration) => declaration.name === each.name,
            );

            return each.targetSpaceId === null || !InstallationBinding.is(declared)
                ? []
                : [
                      {
                          packageId: each.packageId,
                          name: each.name,
                          package: declared.handle.id,
                          installation: each.target,
                          scope: each.targetSpaceId,
                      },
                  ];
        });

        return {
            scope: row.scope,
            installationId: row.installationId,
            resources: resources.flat(),
            installations,
            runs: { kind: "store", deploymentId: served.id },
        };
    }

    /** Connect a resource where its provider placed it, through the machine's connector for it. */
    async #connect(
        row: InstallationRollout,
        declaration: ResourceDeclaration<unknown>,
        provider: SpaceProvider,
        target: string,
        connectors: readonly Connector[],
    ): Promise<unknown> {
        // read the resource's row
        const resource = provider.object.identifier(target);
        const found = present(
            await Snapshot.live(this.database)
                .read(provider.object.table)
                .findFirst({ where: { id: resource } }),
            `the resource ${resource}`,
        );
        const code = present(found.provider, `the provider of ${resource}`);

        // refuse a resource no connector here connects to
        const connector = connectors.find((each) => each.code === code);
        if (connector === undefined) {
            throw StackError.of(
                "PROVIDER_BLOCKED",
                `cannot open resource '${resource}': this machine does not connect to its provider '${code}'`,
            );
        }

        return connector.connect(
            {
                resource: ResourceId.parse(resource),
                kind: declaration.kind.name,
                provider: code,
                reference: present(found.reference, `the reference of ${resource}`),
            },
            declaration,
            this.machine.environment,
        );
    }

    /** Describe an installation to its code serving in the store, calling the platform as it. */
    async #context(
        row: InstallationRollout,
        lock: Digest,
        resources: ResourceContext,
        caller: () => Promise<EgressCaller>,
    ): Promise<ServerContext> {
        // call the space's stack package in process as the installation
        const { machine } = this;
        const { installationId: id, scope } = row;
        const signer = principal.installation.reference(scope, id);
        const stack = createClient(
            stackPackage,
            {
                url: STACK_ORIGIN,
                fetch: (request) =>
                    this.call(
                        stackPackage.id,
                        request,
                        this.identityOf(signer).authentication(stackPackage.id, scope),
                    ),
            },
            machine.environment,
        );
        const at = (address: string) => ({
            url: Egress.url(EGRESS_ORIGIN, address),
            fetch: async (request: Request) => this.#egress(await caller(), request),
        });

        // describe its build and package, settings, keys rooted in its space's identity, replicas and peers
        const reader = await this.root(row.installation.packageId, lock);
        const description = await PackageDescription.of(reader);
        const identity = present(
            await this.keystore.deriver(this.database, scope),
            `the keys of ${scope}, which this machine keeps`,
        );

        // serve below the installation's mount at this machine's origin, on the network its sandbox lists
        const named = row.installation;
        const network = this.#network(named.name, Rollout.sandbox(row.installation, description));
        const { clock, scheduler, random, crypto, compression, telemetry, locale, timeZone } =
            machine.environment;

        return {
            installation: id,
            name: named.name,
            scope,
            url: InstallationMount.url(machine.origin, scope, id),
            build: reader,
            resources,
            capabilities: Rollout.enabled(row.installation, description),
            keys: await InstallationRoot.open(
                await InstallationRoot.derive(identity, id),
                machine.environment.crypto,
            ),
            fetch: (request) => network.fetch(request),
            environment: {
                clock,
                scheduler,
                random,
                crypto,
                network,
                compression,
                telemetry,
                locale,
                timeZone,
            },
            distribution: this.machine.builds.layer,
            settings: await RolloutTarget.settings(this.database, reader, {
                scope,
                installation: id,
            }),
            publisher: Uplink.of(() => stack.store),
            publisherAt: (address, packageId) => {
                // follow the stack package at the address
                const published = createClient(
                    stackPackage,
                    {
                        url: `${at(address).url}${PackageMount.path(packageId)}`,
                        fetch: (request) => at(address).fetch(request),
                    },
                    machine.environment,
                );

                return Uplink.of(() => published.store);
            },
            at,
        };
    }

    /** Build the network an in-process installation reaches: the hosts its sandbox lists. */
    #network(name: string, sandbox: Sandbox): Network {
        const { network } = this.machine.environment;
        const allow = (address: SocketAddress) => {
            // refuse a Unix socket and a host the sandbox does not list, else read whether a wildcard matched it
            const matched = "unix" in address ? undefined : Sandbox.match(sandbox, address);
            if (matched === undefined) {
                const target =
                    "unix" in address ? address.unix : `${address.hostname}:${address.port}`;
                throw StackError.of(
                    "NETWORK_DENIED",
                    `network access to '${target}' is forbidden: installation '${name}' declares no access to it`,
                );
            }

            return matched === "wildcard";
        };

        return {
            fetch: async (request, options) => {
                // send each hop unfollowed, a wildcard's host on public addresses alone
                let sent = request;
                for (let hop = 0; hop <= MAX_REDIRECTS; hop += 1) {
                    const isPublicOnly = allow(HostAddress.of(new URL(sent.url)));
                    const response = await network.fetch(
                        new Request(sent, { redirect: "manual" }),
                        {
                            ...options,
                            isPublicOnly,
                        },
                    );

                    // answer a response that is no redirect, or that the request takes as it is
                    const location = response.headers.get("location");
                    if (
                        !REDIRECT_STATUSES.has(response.status) ||
                        location === null ||
                        sent.redirect === "manual"
                    ) {
                        return response;
                    }

                    // follow the redirect as the Fetch standard does
                    await response.body?.cancel();
                    sent = Space.#redirected(sent, response.status, location);
                }

                throw StackError.of(
                    "REDIRECTS_UNREACHABLE",
                    `could not fetch '${request.url}' within ${MAX_REDIRECTS} redirects`,
                );
            },
            connect: (address, options) => {
                const isPublicOnly = allow(address);

                return network.connect(address, { ...options, isPublicOnly });
            },
            websocket: (url, protocols) => {
                // open a WebSocket to a listed name alone, which a wildcard's public check cannot cover
                const address = HostAddress.of(new URL(url));
                if (allow(address)) {
                    throw StackError.of(
                        "NETWORK_DENIED",
                        `WebSocket to '${address.hostname}' is forbidden: installation '${name}' lists no such name`,
                    );
                }

                return network.websocket(url, protocols);
            },
            resolve: async (hostname) => {
                throw StackError.of(
                    "NETWORK_DENIED",
                    `name resolution of '${hostname}' is forbidden: installation '${name}' resolves no names, only its fetches and connections do`,
                );
            },
        };
    }

    /** Read the request a redirect sends next, as the Fetch standard follows one. */
    static #redirected(sent: Request, status: number, location: string): Request {
        // refuse a redirect the request refuses, and one replaying a body already sent
        const isGet =
            status === 303 || ((status === 301 || status === 302) && sent.method === "POST");
        if (sent.redirect === "error") {
            throw StackError.of(
                "REDIRECT_UNREACHABLE",
                `could not fetch '${sent.url}': it redirected and the request refuses redirects`,
            );
        } else if (!isGet && sent.body !== null) {
            throw StackError.of(
                "REDIRECT_UNREACHABLE",
                `could not follow the ${status} redirect of '${sent.url}': it would send the body again`,
            );
        }

        // drop the body's headers with a GET, and the credentials toward another origin
        const url = new URL(location, sent.url);
        const headers = new Headers(sent.headers);
        if (isGet) {
            BODY_HEADERS.forEach((header) => headers.delete(header));
        }
        if (url.origin !== new URL(sent.url).origin) {
            CREDENTIAL_HEADERS.forEach((header) => headers.delete(header));
        }

        return new Request(url.href, {
            method: isGet && sent.method !== "HEAD" ? "GET" : sent.method,
            headers,
            signal: sent.signal,
            redirect: sent.redirect,
        });
    }

    /** List the partitions the space serves: itself, and its account when it is the home. */
    async partitions(): Promise<readonly string[]> {
        // read the account the space's row places it in
        const [row] = await this.database
            .select({ account: accountObject.space.table.scope })
            .from(accountObject.space.table)
            .where(eq(accountObject.space.table.id, this.id));

        return row !== undefined && accountObject.Account.home(row.account) === this.id
            ? [this.id, row.account]
            : [this.id];
    }

    /** Require a token bound to one of the partitions the space serves, or to none. */
    async requirePartition(verified: Authentication): Promise<void> {
        const { scope } = verified.credential;
        if (scope !== undefined && !(await this.partitions()).includes(scope)) {
            throw StackError.of(
                "PARTITION_TOKEN_DENIED",
                "partition is forbidden: the universe token is bound to another partition",
            );
        }
    }

    /** Place another space's verified caller inside its account's chain, as this space records. */
    async contain(verified: Authentication): Promise<Authentication> {
        // find the account the universe places the signing space in: the acting space or its installation's
        const { caller } = verified;
        const acting = Caller.principal(caller);
        const signer = principal.installation.is(acting) ? acting.scope : acting.id;
        const space = schema.identifier("space").parse(signer);
        const [placement] = await accountObject.Placement.find(this.machine.directory.snapshot, [
            space,
        ]);
        if (placement === undefined) {
            throw StackError.of("PLACEMENT_NOT_FOUND", `placement of '${space}' not found`);
        }

        // add the account's chain from this space's records
        const links = await Scope.chain(Snapshot.live(this.database), placement.scope);
        const inside = links.flatMap((link) =>
            link.object.id === Scope.universe.id ? [] : [{ ...link.object, relation: CONTAINED }],
        );

        return { ...verified, caller: { ...caller, groups: [...caller.groups, ...inside] } };
    }

    /** Declare the projection giving birth to the space once placed here, following its stack. */
    placement(store: ObjectStore, main: ReturnType<typeof defineDatabase>): Projection {
        return new Projection(this.database, {
            name: "placement",
            tables: [
                accountObject.placement.table,
                accountObject.space.table,
                rollout.table,
                Scope.table,
            ],
            keys: async (change) => this.#placementKeys(change),
            all: async () => this.#placed(),
            project: async (key) => this.#place(key, store, main),
        });
    }

    /** Declare the projection assigning the space's work to the other machines doing it. */
    assignment(publisher: Uplink): Projection {
        return new Projection(this.database, {
            name: "assignment",
            tables: [
                build.table,
                deployment.table,
                accountObject.placement.table,
                accountObject.assignment.table,
            ],
            keys: async (change) => this.#assignmentKeys(change),
            all: async () => this.#spaces(),
            project: async (key) => this.#assign(key, publisher),
        });
    }

    /** List the store's space when a change touches it or an account's scope arrived. */
    #placementKeys(change: Change): readonly string[] {
        // take the space's placement
        if (Change.of(change, accountObject.placement.table)) {
            return this.#touching(Change.image(change).spaceId);
        }
        // take the space's replicated row
        else if (Change.of(change, accountObject.space.table)) {
            return this.#touching(Change.image(change).id);
        }
        // take the space's rollouts, its stack's among them
        else if (Change.of(change, rollout.table)) {
            return this.#touching(Change.image(change).scope);
        }
        // take an account whose scope arrived, inside which the space may now be born
        else if (Change.of(change, Scope.table)) {
            return Change.image(change).type === "account" ? [this.id] : [];
        }
        // select nothing for any other change
        else {
            return [];
        }
    }

    /** List the store's space if a change's space is it, else nothing. */
    #touching(changed: string): readonly string[] {
        return changed === this.id ? [this.id] : [];
    }

    /** List the space while placed here awaiting birth, a main database, or its changed stack. */
    async #placed(): Promise<readonly string[]> {
        // read the space while placed here with its stack and the stack its installation last followed
        const rows = await this.database
            .select({
                id: accountObject.placement.table.spaceId,
                born: Scope.table.scope,
                main: database.table.id,
                stack: accountObject.space.table.stack,
                followed: rollout.table.spaceStack,
            })
            .from(accountObject.placement.table)
            .innerJoin(
                accountObject.space.table,
                eq(accountObject.space.table.id, accountObject.placement.table.spaceId),
            )
            .leftJoin(Scope.table, eq(Scope.table.scope, accountObject.space.table.id))
            .leftJoin(
                database.table,
                and(
                    eq(database.table.scope, accountObject.placement.table.spaceId),
                    eq(database.table.name, MAIN_DATABASE),
                ),
            )
            .leftJoin(
                installation.table,
                and(
                    eq(installation.table.scope, accountObject.placement.table.spaceId),
                    isNotNull(installation.table.export),
                ),
            )
            .leftJoin(rollout.table, eq(rollout.table.installationId, installation.table.id))
            .where(
                and(
                    eq(accountObject.placement.table.spaceId, this.id),
                    eq(accountObject.placement.table.machineId, this.machine.id),
                    isNull(accountObject.space.table.deletedAt),
                ),
            );

        // select it unborn, without a main database, or with a stack changed since its installation followed it
        const { crypto } = this.database.state.environment;
        const listed = await Promise.all(
            rows.map(async (row) =>
                row.born === null ||
                row.main === null ||
                (await Digest.json(row.stack, crypto)) !== row.followed
                    ? [row.id]
                    : [],
            ),
        );

        return listed.flat();
    }

    /** Give birth to the placed space from its replicated row, and install its stack. */
    async #place(
        key: string,
        store: ObjectStore,
        main: ReturnType<typeof defineDatabase>,
    ): Promise<void> {
        // leave a space not placed here, a deleting one, and one whose row or parent scope has not arrived
        const spaceId = accountObject.space.identifier(key);
        const [placed] = await this.database
            .select({ born: Scope.table.scope, parent: accountObject.space.table.scope })
            .from(accountObject.placement.table)
            .innerJoin(
                accountObject.space.table,
                eq(accountObject.space.table.id, accountObject.placement.table.spaceId),
            )
            .leftJoin(Scope.table, eq(Scope.table.scope, accountObject.space.table.id))
            .where(
                and(
                    eq(accountObject.placement.table.spaceId, spaceId),
                    eq(accountObject.placement.table.machineId, this.machine.id),
                    isNull(accountObject.space.table.deletedAt),
                ),
            );
        if (placed === undefined) {
            return;
        }

        // open the space once its parent's scope arrived
        if (placed.born === null) {
            const [parent] = await this.database
                .select({ ancestors: Scope.table.ancestors })
                .from(Scope.table)
                .where(eq(Scope.table.scope, placed.parent));
            if (parent === undefined) {
                return;
            }
            await this.#receive(store, spaceId, [placed.parent, ...parent.ancestors]);
        }

        // create its main database as the space
        const [created] = await this.database
            .select({ id: database.table.id })
            .from(database.table)
            .where(and(eq(database.table.scope, spaceId), eq(database.table.name, MAIN_DATABASE)));
        if (created === undefined) {
            await this.#createMain(store, main, spaceId);
        }

        // follow the space's stack once it changed, leaving a stack its owner applied since
        const [received] = await this.database
            .select({
                stack: accountObject.space.table.stack,
                installed: installation.table.id,
                installedPackage: installation.table.packageId,
                followed: rollout.table.spaceStack,
            })
            .from(accountObject.space.table)
            .leftJoin(
                installation.table,
                and(
                    eq(installation.table.scope, accountObject.space.table.id),
                    isNotNull(installation.table.export),
                ),
            )
            .leftJoin(rollout.table, eq(rollout.table.installationId, installation.table.id))
            .where(eq(accountObject.space.table.id, spaceId));
        const row = present(received, `the row of ${spaceId}`);
        const digest = await Digest.json(row.stack, this.database.state.environment.crypto);
        if (digest !== row.followed) {
            await this.#followStack(
                store,
                spaceId,
                row.stack,
                digest,
                row.installed,
                row.installedPackage,
            );
        }
    }

    /** Receive a placed space at its primary below its ancestors. */
    async #receive(
        store: ObjectStore,
        spaceId: Identifier<"space">,
        ancestors: readonly string[],
    ): Promise<void> {
        // read the account the space lives in
        const [row] = await this.database
            .select({ scope: accountObject.space.table.scope })
            .from(accountObject.space.table)
            .where(eq(accountObject.space.table.id, spaceId));
        const replicated = present(row, `the row of ${spaceId}`);
        const now = store.clock.now();
        // record its scope here, its primary
        const scope = accountObject.space.reference(replicated.scope, spaceId);
        await Authorization.run(
            store.authorizer,
            this.database,
            () => AccessContext.of(Caller.of(store.principal), now),
            store.environment,
            (authorization) => authorization.receive(scope, ancestors),
        );
    }

    /** Create a space's main database where its stack places it. */
    async #createMain(
        store: ObjectStore,
        main: ReturnType<typeof defineDatabase>,
        spaceId: Identifier<"space">,
    ): Promise<void> {
        // describe the main database every space has in its own storage
        await Transaction.run(this.database, async (transaction) => {
            // create the main database as the space
            await store.mutate(
                store.as(database),
                {
                    id: RequestId.create(store.environment),
                    calls: [
                        database.call("create", {
                            ...database.in(spaceId),
                            ...ObjectReference.columns(
                                "owner",
                                principal.space.reference(Scope.universe.id, spaceId),
                            ),
                            name: MAIN_DATABASE,
                            ...main.provision({
                                placement: { provider: SPACE_PROVIDER },
                                retention: "forever",
                            }),
                        }),
                    ],
                },
                { transaction, scope: spaceId },
            );
        });
    }

    /** Create the space's stack installation following its stack, or point it at a changed one. */
    async #followStack(
        store: ObjectStore,
        spaceId: Identifier<"space">,
        stack: accountObject.StackRelease,
        digest: Digest,
        installed: Identifier<"installation"> | null,
        installedPackage: string | null,
    ): Promise<void> {
        // refuse moving a space to another stack package, which its installation cannot follow
        if (installedPackage !== null && installedPackage !== stack.packageId) {
            throw new TypeError(
                `the stack of '${spaceId}' moved from '${installedPackage}' to '${stack.packageId}'`,
            );
        }

        // install the stack package once, else follow the changed release and export
        const followed = { selection: stack.selection, export: stack.export };
        const [created] = await store.mutate(store.as(rollout), {
            id: RequestId.create(store.environment),
            calls: [
                installed === null
                    ? installation.call("create", {
                          ...installation.in(spaceId),
                          packageId: stack.packageId,
                          ...followed,
                          parameters: {},
                          name: STACK_INSTALLATION,
                      })
                    : installation.call("update", {
                          ...installation.target({ scope: spaceId, id: installed }),
                          ...followed,
                      }),
            ],
        });

        // record the stack followed and drop an owner's earlier apply, so one its owner applies later stays
        const id = installed ?? present(created, `the stack of ${spaceId}`).id;
        await store.mutate(store.as(rollout), {
            id: RequestId.create(store.environment),
            calls: [
                rollout.call("follow", {
                    ...rollout.target({ scope: spaceId, id }),
                    spaceStack: digest,
                }),
            ],
        });
    }

    /** List the space a change of its work, its placement or its assignments touches. */
    async #assignmentKeys(change: Change): Promise<readonly string[]> {
        // take a placement's space
        if (Change.of(change, accountObject.placement.table)) {
            return [Change.image(change).spaceId];
        }
        // take the space of an assignment's placement
        else if (Change.of(change, accountObject.assignment.table)) {
            const placed = await Snapshot.live(this.database)
                .read(accountObject.placement.table)
                .findFirst({ where: { id: Change.image(change).placementId } });

            return placed === undefined ? [] : [placed.spaceId];
        }
        // take the space of its work otherwise
        else {
            return [change.scope];
        }
    }

    /** List the spaces this machine serves. */
    async #spaces(): Promise<readonly string[]> {
        const rows = await this.database
            .select({ id: accountObject.space.table.id })
            .from(accountObject.space.table)
            .where(accountObject.Space.served());

        return rows.map((row) => row.id);
    }

    /** Assign the machines with work in a space this machine serves and withdraw the others. */
    async #assign(key: string, publisher: Uplink): Promise<void> {
        // act only for a space the replicated placement places on this machine
        const spaceId = accountObject.space.identifier(key);
        const [placed] = await this.database
            .select()
            .from(accountObject.placement.table)
            .where(
                and(
                    eq(accountObject.placement.table.spaceId, spaceId),
                    eq(accountObject.placement.table.machineId, this.machine.id),
                ),
            );
        if (placed === undefined) {
            return;
        }

        // assign the other machines with active builds of their checkouts or live deployments
        const working = new Set(await this.#workers(spaceId));
        working.delete(placed.machineId);
        const assigned = new Set(await this.#assigned(spaceId));
        const work = (name: "assign" | "unassign", machine: Identifier<"machine">) =>
            accountObject.placement.call(name, {
                accountId: placed.scope,
                id: placed.id,
                epoch: placed.epoch,
                machine,
            });

        // assign the new work and withdraw the ended work in one mutation
        const calls = [
            ...[...working.difference(assigned)].map((machine) => work("assign", machine)),
            ...[...assigned.difference(working)].map((machine) => work("unassign", machine)),
        ];
        if (calls.length > 0) {
            await publisher.receive({
                id: RequestId.create(this.machine.environment),
                calls,
            });
        }
    }

    /** List the machines a space gave work, from the replicated assignments. */
    async #assigned(spaceId: Identifier<"space">): Promise<Identifier<"machine">[]> {
        // read the space's placement, then the machines its assignments give work
        const reader = Snapshot.live(this.database);
        const placed = await reader
            .read(accountObject.placement.table)
            .findFirst({ where: { spaceId } });
        const rows =
            placed === undefined
                ? []
                : await reader
                      .read(accountObject.assignment.table)
                      .findMany({ where: { placementId: placed.id } });

        return rows.map((row) => row.machineId);
    }

    /** List the machines a space's active builds and live deployments are addressed to. */
    async #workers(scope: Identifier<"space">): Promise<Identifier<"machine">[]> {
        const [builds, deployments] = await Promise.all([
            this.database
                .select({ source: build.table.source })
                .from(build.table)
                .where(
                    and(
                        eq(build.table.scope, scope),
                        inArray(build.table.state, [...ACTIVE_BUILD_STATES]),
                    ),
                ),
            this.database
                .select({ machineId: deployment.table.machineId })
                .from(deployment.table)
                .where(
                    and(
                        eq(deployment.table.scope, scope),
                        Condition.render(Deployment.live(), deployment.table),
                        isNotNull(deployment.table.machineId),
                    ),
                ),
        ]);

        return [
            ...builds.flatMap((row) =>
                row.source.kind === "checkout" ? [row.source.machine] : [],
            ),
            ...deployments.flatMap((row) => (row.machineId === null ? [] : [row.machineId])),
        ];
    }

    /** The space's running workload. */
    get instance(): WorkloadInstance {
        return present(this.#instance, "the space's workload");
    }

    /** The host of the space's buckets, which the workload's current start opened. */
    get buckets(): CatalogueBucketHost {
        return this.#current().buckets;
    }

    /** Hand a request to one of the space's packages as a verified caller. */
    async call(
        packageId: PackageId,
        request: Request,
        authentication?: Authentication,
    ): Promise<Response> {
        const served = this.instance.packages.find((handle) => handle.id === packageId);
        if (served === undefined) {
            throw MachineError.of("PACKAGE_NOT_FOUND", `package '${packageId}' not found`);
        }

        return this.instance.fetch(served, request, authentication);
    }

    /** Serve a request to the space: its health's probes, else through its gateway. */
    async fetch(request: Request, authentication?: Authentication): Promise<Response> {
        // answer the space's liveness and readiness, as a server answers its own, while it starts too
        const mounted = SpaceMount.parse(new URL(request.url).pathname);
        const probed =
            mounted === undefined
                ? undefined
                : this.health.probe(
                      new Request(new URL(mounted.path, request.url).href, {
                          method: request.method,
                      }),
                  );
        if (probed !== undefined) {
            return probed;
        }

        // serve anything else through the gateway once it serves
        const gateway = await this.#current().gateway.promise;

        return gateway.serve(request, authentication);
    }

    /** Stop the space's workload, then release what the machine opened for it. */
    close(): Promise<void> {
        // close the space in its span
        return this.machine.scope.span(
            "destack.space.close",
            { "destack.space.id": this.id },
            async () => {
                try {
                    await this.#instance?.close();
                } finally {
                    await this.volume.close();
                    this.health.set("stopped", "the space closed");
                }
            },
        );
    }

    /** Retain the builds the space runs, records and waits for in the machine's builds. */
    async #root(
        installed: readonly InstallationBuild[],
        unavailable: readonly Digest[],
    ): Promise<void> {
        const locks = installed.flatMap((each) => [each.lock, each.recorded]);
        await this.machine.retain(this.id, [...locks, ...unavailable]);
    }

    /** Apply a primary's installed set in place, then report whether it serves each revision. */
    async reconcile(): Promise<void> {
        // apply a primary's installed set in this start, recording its served revisions
        const serving = present(this.#start, `the serving of ${this.id}`);
        let unavailable: readonly Digest[] = [];
        if (serving.role === "primary") {
            const { installed, unavailable: waiting } = await this.installed("forges");
            unavailable = waiting;
            await this.#root(installed, waiting);
            await this.#apply(installed);
        }

        // report whether every installation serves its revision, and tell the universe a primary serves the space
        const [status, reason] = await this.#readiness(serving.role);
        this.health.set(status, reason);
        if (serving.role === "primary") {
            await this.#reportServing();
        }

        // retry once a build the space's forges cannot answer yet arrives, as an image pull backs off
        if (unavailable.length > 0) {
            throw new Error(`space '${this.id}' waits for builds '${unavailable.join(", ")}'`);
        }
    }

    /** Read a primary's installed set from the machine's builds or forges, without absent ones. */
    async installed(source: "machine" | "forges"): Promise<{
        readonly installed: readonly InstallationBuild[];
        readonly unavailable: readonly Digest[];
    }> {
        // open the lock each installation runs, noting an unavailable one
        const unavailable: Digest[] = [];
        const open = async (packageId: PackageId, lock: Digest) => {
            // open the root of a lock the machine keeps
            if (source === "machine") {
                const isKept = (await this.builds.storeOf(lock)) !== undefined;
                if (!isKept) {
                    unavailable.push(lock);
                }

                return isKept ? await this.builds.root(lock) : undefined;
            }
            // pull a lock
            else {
                return await this.root(packageId, lock).catch((error: unknown) => {
                    if (!PackageError.is(error, "FILE_UNAVAILABLE")) {
                        throw error;
                    }
                    unavailable.push(lock);

                    return undefined;
                });
            }
        };
        const installed = await this.installations(open);

        return { installed, unavailable };
    }

    /** Report to the universe that this machine serves the space, unless its placement says so. */
    async #reportServing(): Promise<void> {
        // read the placement, reporting at its current epoch
        const { placement, status } = present(
            await this.machine.directory.placement(this.id),
            `the placement of ${this.id}`,
        );
        if (status === "serving") {
            return;
        }

        await this.accounts().placement.report({
            accountId: placement.scope,
            id: this.id,
            requestId: RequestId.create(this.machine.environment),
            epoch: placement.epoch,
            status: "serving",
        });
    }

    /** Read whether the space serves each installation at its revision, else the lagging one. */
    async #readiness(role: SpaceRole): Promise<["starting" | "up", string]> {
        // serve a follower as its primary does
        if (role !== "primary") {
            return ["up", `it follows its primary as ${role}`];
        }

        // read the rollouts with their installations' conditions
        const snapshot = Snapshot.live(this.database);
        const rows = await Rollout.read(
            snapshot,
            await snapshot.read(rollout.table).findMany({ where: { scope: this.id } }),
        );
        const lagging = rows.find(
            (row) =>
                row.revisionId === null ||
                row.appliedRevisionId !== row.revisionId ||
                row.servedRevisionId !== row.revisionId,
        );

        // wait for a first installation, then for each to serve its revision, naming its open condition
        if (rows.length === 0) {
            return ["starting", "no installation is applied yet"];
        } else if (lagging !== undefined) {
            const installed = lagging.installation.name;
            const open = Object.entries(lagging.installation.conditions).find(
                ([, condition]) => condition.status !== "true",
            );
            const condition =
                open === undefined ? "" : `: ${open[0]} ${open[1].reason}, ${open[1].message}`;

            return [
                "starting",
                `installation ${installed} does not serve its revision yet${condition}`,
            ];
        } else {
            return ["up", "every installation serves its revision"];
        }
    }

    /** Wait until an installation of the space is current and served, refusing once aborted. */
    async waitServed(name: string, signal: AbortSignal): Promise<void> {
        // wake on changes of installations and their rollouts
        const installations = installation.table[TABLE].sqlName;
        const rollouts = rollout.table[TABLE].sqlName;
        const isServed = await this.database.log.waitUntil(async (tables) => {
            // skip commits that change no installation and no rollout
            if (!tables.has(installations) && !tables.has(rollouts)) {
                return false;
            }

            // read the installation by its name, current once this start serves its latest revision
            const snapshot = Snapshot.live(this.database);
            const installed = await snapshot
                .read(installation.table)
                .findFirst({ where: { scope: this.id, name } });
            const served =
                installed === undefined
                    ? undefined
                    : await snapshot
                          .read(rollout.table)
                          .findFirst({ where: { installationId: installed.id } });

            return (
                installed !== undefined &&
                Observation.progress(installed) === "current" &&
                served?.servedRevisionId === installed.revisionId
            );
        }, signal);
        if (!isServed) {
            const { status, reason } = this.health;
            throw MachineError.of(
                "INSTALLATION_UNAVAILABLE",
                `installation '${name}' of space '${this.id}' is unavailable: it serves nothing yet, as the space is ${status} ('${reason}')`,
            );
        }
    }

    /** Restart the workload once the directory places the space here in another role. */
    async restartIfRoleChanged(): Promise<void> {
        const serving = this.#start;
        if (serving !== undefined && (await this.role()) !== serving.role) {
            serving.context.reload().catch((error: unknown) => serving.context.report(error));
        }
    }

    /** Read the mounts the installed set's packages declare. */
    static #mounts(installed: readonly InstallationBuild[]): Mount[] {
        return installed.flatMap((entry) => {
            // read the mounts the package declares, none for a library
            const { installationId } = entry.rollout;
            const packageId = PackageId.parse(entry.rollout.installation.packageId);
            const mounts = PackageDescription.find(entry.declarations)?.mounts ?? {};

            return Object.entries(mounts).map(([name, mount]) => ({
                installationId,
                packageId,
                name,
                path: mount.path,
            }));
        });
    }

    /** List the mounts an installation the space serves declares. */
    mounts(installationId: Identifier<"installation">): readonly Mount[] {
        return this.#current().mounts.filter((each) => each.installationId === installationId);
    }

    /** Publish each of an account's packages in a stored lock for a caller. */
    async publish(
        store: PublishStore,
        lock: Digest,
        account: Identifier<"account">,
        tag: string,
        commit: string | undefined,
        caller: Caller,
        requested: GrantRequest,
    ): Promise<readonly PackageRelease[]> {
        // act as the space for the caller, refusing a caller it cannot act for
        const space = principal.space.reference(Scope.universe.id, this.id);
        const { environment } = this.machine;
        const acting = Exchange.of(caller);
        if (acting === undefined) {
            throw MachineError.of(
                "PUBLISHER_DENIED",
                "publish is forbidden: a publish acts for a person or a delegate",
            );
        }

        // publish through the forges in the accounts' residency homes, calling each for the caller under the grant the space asks
        const forges = Registry.of(
            this.machine.directory,
            (owner) => this.#residencyHome(owner),
            (audience) =>
                this.session(this.identityOf(space).acting(acting, requested)).fetch(audience),
            environment,
            this.machine.registry,
            [...this.machine.shipped.values()],
        );

        return await forges.publish(store, lock, account, tag, commit);
    }

    /** Lease a repository of the space's account to read, at the forge in its residency's home. */
    async lease(repositoryId: Identifier<"repository">): Promise<Lease> {
        // call the forge of the space's account as the space, a peer call to its home's stack
        const self = principal.space.reference(Scope.universe.id, this.id);
        const forges = Registry.of(
            this.machine.directory,
            (account) => this.#residencyHome(account),
            () => this.session(this.identityOf(self)).fetch(stackPackage.id),
            this.machine.environment,
            this.machine.registry,
        );

        return forges.lease(await this.account(), repositoryId);
    }

    /** Forward an instance's telemetry export, stamped with its installation and instance. */
    async telemetry(spec: InstanceSpec, signal: OtlpSignal, body: unknown): Promise<void> {
        const gateway = await this.#current().gateway.promise;
        await gateway.telemetry(spec, signal, body);
    }

    /** Forward a telemetry export to each installation taking it, as the space itself. */
    async observe(signal: OtlpSignal, body: unknown): Promise<void> {
        const self = principal.space.reference(Scope.universe.id, this.id);
        const receivers = this.#current().mounts.filter((each) => each.name === OTLP_MOUNT);
        for (const receiver of receivers) {
            // post the export below the receiver's mount, refusing a failed answer
            const url = new URL(`${receiver.path}/${signal}`, this.machine.origin);
            const request = new Request(url.href, {
                method: "POST",
                headers: { "content-type": "application/json" },
                body: JSON.stringify(body),
            });
            const authentication = this.identityOf(self).authentication(
                receiver.packageId,
                this.id,
            );
            const response = await this.call(receiver.packageId, request, authentication);
            const answer = await response.text();
            if (!response.ok) {
                throw MachineError.of(
                    "TELEMETRY_UNAVAILABLE",
                    `the telemetry receiver is unavailable: it refused a '${signal}' export with ${response.status} '${answer}'`,
                );
            }
        }
    }

    /** Borrow a connection's credential and its resource at its account's home, as the consumer. */
    async borrow(
        lent: { readonly scope: string; readonly id: Identifier<"connection"> },
        consumer: Subject,
    ): Promise<ConnectionLease> {
        // call the connections' primary in the home of the scope's account as the consumer
        const { environment } = this.machine;
        const home = await accountObject.Account.homeOf(Snapshot.live(this.database), lent.scope);
        const identity = this.identityOf(consumer);
        const primary = this.machine.directory.client(
            connectionPackage,
            home,
            CONNECTION_INSTALLATION,
            this.session(identity).fetch(connectionPackage.id),
            environment,
        );

        return primary.connection.lease({ scope: lent.scope, id: lent.id });
    }

    /** Append an account's uses of a page of readings at its payer's home, as the machine. */
    async append(page: CursorPage): Promise<void> {
        const { directory, environment } = this.machine;
        for (const use of await Reading.uses(page, environment.crypto)) {
            // bill the payer of the space this machine serves, at the stack in the home of the payer's residency
            const payer = await this.accounts().placement.payer({
                accountId: use.accountId,
                id: this.id,
            });
            const home = await this.#residencyHome(payer);
            const fetch = this.machine.fetch(stackPackage.id);
            const stack = directory.client(
                stackPackage,
                home,
                STACK_INSTALLATION,
                fetch,
                environment,
            );
            await stack.usage.append({ ...use, accountId: payer });
        }
    }

    /** Read what the current start of the workload serves with, refusing before it serves. */
    #current(): SpaceStart {
        return present(this.#start, `the workload of ${this.id}`);
    }

    /** Read the home of an account's residency through the universe, as the machine. */
    async #residencyHome(accountId: Identifier<"account">): Promise<Identifier<"space">> {
        return this.accounts().residency.locate({ account: accountId });
    }
}

/** A machine's space, which the process running the machine binds for its workload. */
export const machineSpace = new ResourceHandle<Space>(import.meta.destack.package, "space");

/** The workload of one space a machine serves. */
export const spaceWorkload = defineWorkload({
    name: "machine",
    start: (context) => machineSpace.get(context.resources).serve(context),
});
