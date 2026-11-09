import { type Authentication, CONTAINED, Grant, principal } from "@destack/access";
import { Exchange } from "@destack/access/token";
import type { Directory } from "@destack/account/address";
import {
    IdentityKeystore,
    type IdentityRegistry,
    type TokenSource,
} from "@destack/account/identity";
import * as accountObject from "@destack/account/object";
import accountPackage from "@destack/account/package";
import {
    and,
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
    Transaction,
} from "@destack/db";
import { AccessContext, Authorization, Caller } from "@destack/access";
import { MAIN_DATABASE } from "../../declare/index.ts";
import { STACK_INSTALLATION } from "../../package.ts";
import type { Environment, Network, SocketAddress } from "@destack/environment";
import type { Deriver, Keyring } from "@destack/identity";
import {
    CallKey,
    Egress,
    type Fetch,
    HookSandbox,
    Installation,
    InstallationMount,
    PackageDescription,
    InstallationBinding,
    installation,
    type ProcedureRouter,
    type PackageHandle,
    PackageMount,
} from "@destack/object";
import { InstallationRoot, type InstalledServerContext } from "@destack/object/workload";
import {
    type Connector,
    EGRESS_PROVIDER,
    type ExternalClient,
    ResourceDeclaration,
    ResourceId,
    ResourceRecord,
} from "@destack/resource";
import { BucketDeclaration } from "@destack/bucket";
import type { CatalogueBucketHost } from "@destack/bucket/provider";
import { ResourceContext } from "@destack/resource/context";
import { HostAddress } from "@destack/environment";
import {
    ObjectSink,
    type ObjectStore,
    type PackageImplementation,
    RemoteStore,
} from "@destack/object/server";
import { type ClientOf, createClient } from "@destack/object/client";
import objectPackage from "@destack/object/package";
import { type graph, type PackageId, type PackageRelease, Sandbox } from "@destack/package";
import { Digest, type Identifier, present, schema } from "@destack/schema";
import {
    ObjectReference,
    type Publisher,
    RequestId,
    Scope,
    Subject,
    type Subscription,
    Uplink,
} from "@destack/sync";
import { SPACE_BASE_PACKAGES, type SpaceOptions } from "../server.ts";
import {
    ACTIVE_BUILD_STATES,
    Binding,
    installationRevision,
    bucket,
    build,
    Database,
    database,
    deployment,
    Deployment,
    dependency,
    InstallationRevision,
    Rollout,
    rollout,
    SPACE_PROVIDER,
    type InstallationRollout,
} from "../../object/index.ts";
import { ObjectBindable } from "../resource/binding.ts";
import { StoreProvider, type SpaceProvider } from "../resource/resource.ts";
import stackPackage from "../../package.ts";
import type { SpaceBuildStore } from "../installation/build.ts";
import type { SpaceRole } from "./space.ts";
import { StackError } from "../../error/index.ts";
import type {
    EgressCaller,
    WorkloadInstallation,
    WorkloadResource,
} from "../../runtime/runtime.ts";
import { RolloutTarget } from "../installation/target.ts";

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

/** What a machine lends the spaces it hosts: its identity, directory, network and servers. */
export interface SpaceMachine {
    /** The machine. */
    readonly id: Identifier<"machine">;
    /** The clock, network and randomness the machine runs in. */
    readonly environment: Environment;
    /** The keyring that stores the keys of the spaces the machine hosts. */
    readonly keyring: Keyring;
    /** The origin the machine answers at: its relayed name, cloud URL or local endpoint. */
    readonly origin: string;
    /** The machine's replica of the universe's directory, locating spaces and machines. */
    readonly directory: Directory;
    /** The universe's registry of identities, which the spaces' keystores publish to. */
    readonly identities: IdentityRegistry;
    /** The account package's URL at the universe, and the universe's issuer. */
    readonly universe: { readonly accounts: string; readonly issuer: string };
    /** Create a fetch calling a universe package as the machine. */
    readonly fetch: (audience: PackageId) => Fetch;
    /** Send a request to another origin as it is. */
    readonly send: (request: Request) => Promise<Response>;
    /** The machine's builds, which installations' revisions read. */
    readonly builds: Pick<SpaceBuildStore, "contents">;
    /** The releases of the packages the machine links, by package. */
    readonly linked: ReadonlyMap<PackageId, PackageRelease>;
    /** The manifests of the releases the machine runs in its process: those it ships or links. */
    readonly trusted: ReadonlySet<Digest>;
}

/** What a hosted space's start lends its installations' servers, as its host decides. */
export interface SpaceStart {
    /** Hand a request to one of the space's served packages as a verified caller. */
    readonly call: (
        packageId: PackageId,
        request: Request,
        authentication: Authentication,
    ) => Promise<Response>;
    /** The space's store once it serves, which bound secrets and vaults read through. */
    readonly store: Promise<ObjectStore>;
    /** The providers of the space's resources, store providers connecting through the store. */
    readonly providers: readonly (SpaceProvider | StoreProvider)[];
    /** The connectors of the machine's resources, by provider code, such as Git storage. */
    readonly connectors: readonly Connector[];
    /** The host that stores the space's buckets. */
    readonly buckets: Pick<CatalogueBucketHost, "open">;
    /** Send an installation's call below the machine's egress, as it sends an instance's. */
    readonly egress: (caller: EgressCaller, request: Request) => Promise<Response>;
    /** Load the server a build exports, absent for a build exporting none. */
    readonly server: (manifest: Digest) => Promise<PackageImplementation | undefined>;
    /** Report an installation the start leaves unserved, such as one whose server fails to load. */
    readonly report: (error: unknown) => void;
    /** Run the hooks of an installation served in its sandbox. */
    readonly sandbox: (
        row: InstallationRollout,
        server: InstalledServerContext,
        declarations: readonly graph.Declaration[],
    ) => HookSandbox;
}

/** The installations a space serves at one start: trusted ones in the store, others sandboxed. */
export interface SpaceServers {
    /** The servers of the installations the store runs, loaded from their builds. */
    readonly servers: readonly PackageImplementation[];
    /** The sandboxes of the others' hooks. */
    readonly sandboxes: readonly HookSandbox[];
    /** The revision of each installation the start serves, leaving out one that failed to load. */
    readonly revisions: ReadonlyMap<
        Identifier<"installation">,
        Identifier<"installation-revision">
    >;
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

/** A space a machine hosts: its identity, its link to the universe and its placement. */
export class SpaceHost {
    /** The space. */
    readonly id: Identifier<"space">;
    /** The machine hosting the space. */
    readonly machine: SpaceMachine;
    /** The space's database. */
    readonly database: DatabaseConnection;
    /** The keystore of the space's keys under the machine's keyring. */
    readonly keystore: IdentityKeystore;
    /** The token sources of the space and its installations, reusing their tokens, by subject. */
    readonly #sources = new Map<string, TokenSource>();
    /** The current start's servers, by installation and revision, kept while they stay served. */
    #served:
        | { readonly signal: AbortSignal; readonly entries: ReadonlyMap<string, ServedEntry> }
        | undefined;

    /** Host a space over its database on a machine. */
    constructor(machine: SpaceMachine, id: Identifier<"space">, connection: DatabaseConnection) {
        // store the space, its machine and database, and its keys under the machine's keyring
        this.id = id;
        this.machine = machine;
        this.database = connection;
        this.keystore = new IdentityKeystore(
            machine.keyring,
            machine.identities,
            machine.environment,
        );
    }

    /** The space as a principal. */
    get principal(): Subject {
        return principal.space.reference(Scope.universe.id, this.id);
    }

    /** Start the space's identity where the directory places it here, then key its columns. */
    async open(): Promise<void> {
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
        this.#sources.clear();
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

    /** Read the token source of the space or an installation, reusing its tokens. */
    source(subject: Subject): TokenSource {
        // reuse the subject's source and its tokens
        const key = Subject.key(subject);
        const existing = this.#sources.get(key);
        if (existing !== undefined) {
            return existing;
        }

        // open it with the space's keys toward the universe's issuer
        const opened = this.keystore.source(this.database, subject, {
            issuer: this.machine.universe.issuer,
            send: (request) => this.machine.send(request),
            environment: this.machine.environment,
        });
        this.#sources.set(key, opened);

        return opened;
    }

    /** Call the account package as the machine, or as a principal of the space. */
    accounts(subject?: Subject): ClientOf<typeof accountPackage> {
        return createClient(
            accountPackage,
            {
                url: this.machine.universe.accounts,
                fetch:
                    subject === undefined
                        ? this.machine.fetch(accountPackage.id)
                        : this.source(subject).fetchGranted(accountPackage.id),
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

            // follow the space's chain as the machine on a move's target without the space's keys yet
            const isReceiving =
                subscription.name === ObjectSink.chainName(this.id) &&
                !(await this.keystore.hasKeys(this.database, this.id));

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
        const fetchOf = (subject: Subject | undefined) =>
            subject !== undefined && principal.installation.is(subject)
                ? this.source(subject).fetchSigned(objectPackage.id, partition)
                : this.source(this.principal).fetchGranted(objectPackage.id, partition);
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

    /** Fill the options of serving the space its host decides: its identity, links and claims. */
    async options(): Promise<
        Pick<
            SpaceOptions,
            | "bases"
            | "host"
            | "role"
            | "principal"
            | "partitions"
            | "machine"
            | "database"
            | "callKey"
            | "accounts"
            | "claims"
            | "environment"
            | "publisher"
            | "publisherAt"
            | "keystore"
            | "linked"
            | "trusted"
        >
    > {
        // call the universe as the machine, and claim names at the universe through it
        const accounts = this.accounts();

        // derive the base installation of each package the space is born with
        const bases = await Promise.all(
            SPACE_BASE_PACKAGES.map(
                async (handle) =>
                    [
                        handle.id,
                        await Installation.base(
                            this.principal,
                            handle,
                            this.machine.environment.crypto,
                        ),
                    ] as const,
            ),
        );

        return {
            bases: new Map(bases),
            host: this,
            role: await this.role(),
            principal: this.principal,
            partitions: await this.partitions(),
            machine: this.machine.id,
            database: this.database,
            callKey: () => this.callKey(),
            accounts,
            claims: {
                object: accountObject.claim,
                home: RemoteStore.of(accounts.store, Scope.universe.id),
            },
            environment: this.machine.environment,
            linked: this.machine.linked,
            trusted: this.machine.trusted,
            publisher: this.uplink(),
            publisherAt: (home, partition) => this.publisherAt(home, partition),
            keystore: this.keystore,
        };
    }

    /** Serve each installation main applied: a trusted one in the store, another in its sandbox. */
    async servers(
        installed: Awaited<ReturnType<typeof Database.installed>>,
        start: SpaceStart,
        signal: AbortSignal,
    ): Promise<SpaceServers> {
        // keep this start's servers of installations at the revision served before, by installation and revision
        const earlier: ReadonlyMap<string, ServedEntry> =
            this.#served?.signal === signal ? this.#served.entries : new Map();
        const keys = installed.map(({ rollout: row, revisionId }) =>
            SpaceHost.#key(row.installationId, revisionId),
        );

        // load the server of each other installation the store runs from its build
        const loaded = await Promise.all(
            installed.map(async ({ rollout: row, manifest }, index) => {
                // take the earlier load, else load a trusted one, reporting a server that fails to load
                const kept = earlier.get(present(keys[index], "the installation's key"));
                const isLinked =
                    this.machine.linked.get(row.installation.packageId)?.manifest === manifest;
                if (kept !== undefined) {
                    return kept.load;
                } else if (row.installation.isolation !== "host" && !isLinked) {
                    return { kind: "sandbox" } as const;
                }
                try {
                    return { kind: "host", server: await start.server(manifest) } as const;
                } catch (error) {
                    const name = row.installation.name ?? row.installationId;
                    start.report(
                        new Error(
                            `installation ${name} serves nothing: its server failed to load`,
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
                (await this.#serve(entry, load, { start, signal, trusted }));
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
    controller(reconcile: () => Promise<void>): Controller<typeof Scope.table> {
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
            reconcile,
        };
    }

    /** Apply an installed set to a start: load its servers, hand a changed set over, record it. */
    async apply(
        installed: Awaited<ReturnType<typeof Database.installed>>,
        start: SpaceStart,
        signal: AbortSignal,
        handover: (
            servers: readonly PackageImplementation[],
            sandboxes: readonly HookSandbox[],
        ) => Promise<void> | void,
    ): Promise<void> {
        // load the set's servers, reusing this start's, and hand them over unless the start serves the set already
        const served = this.#served?.signal === signal ? this.#served.entries : undefined;
        const isServed =
            served?.size === installed.length &&
            installed.every(({ rollout: row, revisionId }) =>
                served.has(SpaceHost.#key(row.installationId, revisionId)),
            );
        const { servers, sandboxes, revisions } = await this.servers(installed, start, signal);
        if (!isServed) {
            await handover(servers, sandboxes);
        }

        // record the revision of each installation the start serves
        await Rollout.serve(await start.store, this.id, revisions);
    }

    /** Key an installation's server by the installation and the revision it serves. */
    static #key(
        installationId: Identifier<"installation">,
        revisionId: Identifier<"installation-revision">,
    ): string {
        return `${installationId}@${revisionId}`;
    }

    /** Serve one installation: a loaded server in the store, else its hooks in a sandbox. */
    async #serve(
        {
            rollout: row,
            revisionId,
            declarations,
        }: Awaited<ReturnType<typeof Database.installed>>[number],
        load: ServedEntry["load"],
        run: {
            readonly start: SpaceStart;
            readonly signal: AbortSignal;
            readonly trusted: readonly PackageImplementation[];
        },
    ): Promise<ServedEntry["served"]> {
        // serve a loaded server in the store, activated once its revision and any lent grant exist
        const { start, signal, trusted } = run;
        const stored = load.kind === "host" ? load.server : undefined;
        if (load.kind === "unserved") {
            return undefined;
        }
        const revision = await Snapshot.live(this.database)
            .read(installationRevision.table)
            .findFirst({ where: { id: revisionId } });
        const { bindings } = present(revision, `revision ${revisionId}`);
        if (stored !== undefined) {
            const caller = this.#callerOnce(row, stored.package, start, bindings);
            const resources = await this.#resources(
                row,
                stored.package,
                start,
                caller,
                trusted,
                bindings,
            );
            const context = await this.#context(row, revisionId, resources, start, caller);
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
            const caller = this.#callerOnce(row, { resources: {} }, start, bindings);
            const context = await this.#context(
                row,
                revisionId,
                new ResourceContext(),
                start,
                caller,
            );

            return { kind: "sandbox", sandbox: start.sandbox(row, context, declarations) };
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
        handle: Pick<PackageHandle, "resources" | "secrets">,
        start: SpaceStart,
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
                    `installation ${row.installationId} binds ${declaration.name}, which is no declaration of this process's resource kinds`,
                );
            }
            bound.bind(
                declaration,
                await this.#open(row, declaration, start, caller, trusted, bindings),
            );
        }

        return bound;
    }

    /** Open a declaration an installation binds in this process, through what opens its kind. */
    async #open(
        row: InstallationRollout,
        declaration: ResourceDeclaration<unknown>,
        start: SpaceStart,
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
        const bindable = ObjectBindable.of(trusted).find(
            (each) => each.declaration.name === declaration.kind,
        );
        const provider = [...start.providers, ...StoreProvider.of(trusted)].find(
            (each) => each.kind.name === declaration.kind,
        );

        // connect to a bound object, such as a secret, or a store resource, such as a vault, through the store
        if (bindable !== undefined) {
            return bindable.connect(start.store, row, found);
        } else if (provider !== undefined && "connect" in provider) {
            return provider.connect(start.store, row, found);
        }
        // call a bound installation through the machine's egress, as an instance's binding does
        else if (InstallationBinding.is(declaration)) {
            return this.#client(declaration, found, start, caller);
        }
        // call an external resource, such as a connection's API, through the egress below its identifier
        else if (declaration.connectors[EGRESS_PROVIDER] !== undefined) {
            const url = Egress.url(EGRESS_ORIGIN, found.target);
            const send = async (request: Request) => start.egress(await caller(), request);
            const client: ExternalClient & AsyncDisposable = {
                url,
                fetch: (path, init = {}) => send(new Request(`${url}${path}`, init)),
                [Symbol.asyncDispose]: () => Promise.resolve(),
            };

            return client;
        }
        // open a bucket from the space's buckets
        else if (declaration instanceof BucketDeclaration) {
            return start.buckets.open({ bucketId: bucket.identifier(target) }, scope);
        }
        // connect another resource where its provider placed it, through the connector of that provider
        else if (provider !== undefined) {
            return this.#connect(row, declaration, provider, target, start.connectors);
        }
        // refuse a kind nothing here opens
        else {
            throw StackError.of(
                "RESOURCE_UNSUPPORTED",
                `this machine cannot open ${declaration.kind} in process`,
            );
        }
    }

    /** Open a bound installation's client, calling through the machine's egress. */
    async #client(
        declaration: InstallationBinding,
        bound: Pick<Binding, "target" | "targetSpaceId">,
        start: SpaceStart,
        caller: () => Promise<EgressCaller>,
    ): Promise<unknown> {
        // send each call as the installation's in-store caller
        const send = async (request: Request) => start.egress(await caller(), request);

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
        handle: Pick<PackageHandle, "resources">,
        start: SpaceStart,
        bindings: readonly Binding[],
    ): () => Promise<EgressCaller> {
        let built: Promise<EgressCaller> | undefined;

        return () => (built ??= this.#caller(row, handle, start, bindings));
    }

    /** Read an in-process installation as its egress's caller, with what it binds. */
    async #caller(
        row: InstallationRollout,
        handle: Pick<PackageHandle, "resources">,
        start: SpaceStart,
        bindings: readonly Binding[],
    ): Promise<EgressCaller> {
        // take the served revision's bindings to targets here, and the resources they target
        const local = bindings.filter((each) => each.targetSpaceId === null);
        const provided = start.providers.filter(
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
                            `${entry.name} of ${row.installation.name} binds ${found.id}, which its provider has not placed`,
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
            runs: { kind: "store" },
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
                "PROVIDER_MISMATCH",
                `${resource} belongs to provider ${code}, which this machine does not connect to`,
            );
        }

        return connector.connect(
            {
                resource: ResourceId.parse(resource),
                kind: declaration.kind,
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
        revisionId: Identifier<"installation-revision">,
        resources: ResourceContext,
        start: SpaceStart,
        caller: () => Promise<EgressCaller>,
    ): Promise<InstalledServerContext> {
        // call the space's stack package in process as the installation
        const { machine } = this;
        const { installationId: id, scope } = row;
        const signer = principal.installation.reference(scope, id);
        const stack = createClient(
            stackPackage,
            {
                url: STACK_ORIGIN,
                fetch: (request) =>
                    start.call(
                        stackPackage.id,
                        request,
                        this.source(signer).authentication(stackPackage.id, scope),
                    ),
            },
            machine.environment,
        );
        const at = (address: string) => ({
            url: Egress.url(EGRESS_ORIGIN, address),
            fetch: async (request: Request) => start.egress(await caller(), request),
        });

        // describe its build and package, settings, keys rooted in its space's identity, replicas and peers
        const reader = await InstallationRevision.reader(
            Snapshot.live(this.database),
            revisionId,
            machine.builds,
        );
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
                    "NETWORK_UNDECLARED",
                    `installation ${name} declares no network access to ${target}`,
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
                    sent = SpaceHost.#redirected(sent, response.status, location);
                }

                throw StackError.of(
                    "REDIRECTS_EXCEEDED",
                    `${request.url} redirected more than ${MAX_REDIRECTS} times`,
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
                        "NETWORK_UNDECLARED",
                        `installation ${name} lists no name ${address.hostname} for a WebSocket`,
                    );
                }

                return network.websocket(url, protocols);
            },
            resolve: async (hostname) => {
                throw StackError.of(
                    "NETWORK_UNDECLARED",
                    `installation ${name} resolves no name such as ${hostname}: its fetches and connections do`,
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
                "REDIRECT_REFUSED",
                `${sent.url} redirected, which the request refuses`,
            );
        } else if (!isGet && sent.body !== null) {
            throw StackError.of(
                "REDIRECT_REFUSED",
                `${sent.url} redirected with ${status}, which would send its body again`,
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
                "FOREIGN_PARTITION_TOKEN",
                "the universe token is bound to another partition",
            );
        }
    }

    /** Place another space's verified caller inside its account's chain, as this space records. */
    async contain(verified: Authentication): Promise<Authentication> {
        // find the account the universe places the signing space in: the space or its installation's
        const { caller } = verified;
        const { subject } = caller;
        const signer = principal.installation.is(subject) ? subject.scope : subject.id;
        const space = schema.identifier("space").parse(signer);
        const [placement] = await accountObject.Placement.find(this.machine.directory.snapshot, [
            space,
        ]);
        if (placement === undefined) {
            throw StackError.of("SPACE_NOT_PLACED", `${space} has no placement`);
        }

        // add the account's chain from this space's records
        const links = await Scope.chain(Snapshot.live(this.database), placement.scope);
        const inside = links.flatMap((link) =>
            link.object.id === Scope.universe.id ? [] : [{ ...link.object, relation: CONTAINED }],
        );

        return { ...verified, caller: { ...caller, groups: [...caller.groups, ...inside] } };
    }

    /** Declare the projection giving birth to the space once placed here, following its stack. */
    placement(store: ObjectStore, main: SpaceOptions["main"]): Projection {
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
    async #place(key: string, store: ObjectStore, main: SpaceOptions["main"]): Promise<void> {
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

    /** Receive a placed space at its primary below its ancestors, with its base installations. */
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
        // record its scope here, its primary, born with its base installations
        const scope = accountObject.space.reference(replicated.scope, spaceId);
        await Authorization.run(
            store.authorizer,
            this.database,
            () => AccessContext.of(Caller.of(store.principal), now),
            store.environment,
            (authorization) =>
                Installation.receive(authorization, scope, ancestors, SPACE_BASE_PACKAGES),
        );
    }

    /** Create a space's main database where its stack places it. */
    async #createMain(
        store: ObjectStore,
        main: SpaceOptions["main"],
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
                            ...main.declare({
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
                `the stack of ${spaceId} moved from ${installedPackage} to ${stack.packageId}`,
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
}
