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
import { type DatabaseConnection, eq, Snapshot } from "@destack/db";
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
    HttpsBinding,
    type HttpsClient,
    InstallationBinding,
    type ProcedureRouter,
    type PackageHandle,
    PackageMount,
} from "@destack/object";
import { InstallationRoot, type InstalledServerContext } from "@destack/object/workload";
import { type Connector, ResourceDeclaration, ResourceId, ResourceRecord } from "@destack/resource";
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
import { type Digest, type Identifier, present, schema } from "@destack/schema";
import { type Publisher, Scope, Subject, type Subscription, Uplink } from "@destack/sync";
import { SPACE_KERNEL, type SpaceOptions } from "../server.ts";
import {
    binding,
    type Binding,
    bucket,
    Database,
    dependency,
    https,
    InstallationRevision,
    Rollout,
    rollout,
    type NamedInstallation,
} from "../../object/index.ts";
import type { ServedBindable } from "../resource/binding.ts";
import type { ServedProvider, SpaceProvider } from "../resource/resource.ts";
import stackPackage from "../../package.ts";
import type { BuildStore } from "../installation/build.ts";
import type { SpaceRole } from "./space.ts";
import { StackError } from "../../error/index.ts";
import type {
    EgressCaller,
    WorkloadInstallation,
    WorkloadResource,
} from "../../runtime/runtime.ts";

/** The origin in-store installations call their space's stack service at, in process. */
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

/** The origin below which in-store installations address other installations. */
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
    readonly builds: Pick<BuildStore, "packages">;
    /** The servers the machine runs in process, by their package. */
    readonly servers: readonly PackageImplementation[];
    /** The release the machine ships of each server's package. */
    readonly bundled: ReadonlyMap<PackageId, PackageRelease>;
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
    /** The bindables connecting to their bound targets through the store, such as secrets. */
    readonly bindables: readonly ServedBindable[];
    /** The providers of the space's resources, served ones connecting through the store. */
    readonly providers: readonly (SpaceProvider | ServedProvider)[];
    /** The connectors of the machine's resources, by provider code, such as Git storage. */
    readonly connectors: readonly Connector[];
    /** The host that stores the space's buckets. */
    readonly buckets: Pick<CatalogueBucketHost, "open">;
    /** Send an installation's call below the machine's egress, as it sends an instance's. */
    readonly egress: (caller: EgressCaller, request: Request) => Promise<Response>;
    /** Run the hooks of an installation whose package the machine ships no server of. */
    readonly sandbox: (
        row: Rollout & { readonly installation: NamedInstallation },
        server: InstalledServerContext,
        declarations: readonly graph.Declaration[],
    ) => HookSandbox;
}

/** The installations a space serves at one start: shipped ones in process, others sandboxed. */
export interface SpaceServers {
    /** The servers of the shipped installations. */
    readonly servers: readonly PackageImplementation[];
    /** The sandboxes of the others' hooks. */
    readonly sandboxes: readonly HookSandbox[];
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

    /** Host a space over its database on a machine. */
    constructor(machine: SpaceMachine, id: Identifier<"space">, database: DatabaseConnection) {
        // store the space, its machine and database, and its keys under the machine's keyring
        this.id = id;
        this.machine = machine;
        this.database = database;
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
        const { keystore, database } = this;
        const isRetained = await keystore.hasKeys(database, this.id);
        if ((await this.role()) === "primary" && !isRetained) {
            await keystore.generate(database, this.id);
        }

        // key the space's sensitive columns with its identity's column key, once its keys live here
        if (await keystore.hasKeys(database, this.id)) {
            await keystore.key(database, this.id);
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
            confirm: async (subscription, watermark) => {
                await (await followerOf(subscription)).confirm({ subscription, watermark });
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
        // sign as the installation a follow is decided for, else as the space
        const signer = (subject: Subject | undefined) =>
            subject !== undefined && principal.installation.is(subject) ? subject : this.principal;
        const at = (subscription: Subscription) =>
            this.machine.directory.publisher(
                home,
                this.source(signer(subscription.subject)).fetchSigned(objectPackage.id, partition),
                this.machine.environment,
            );

        return {
            stream: (subscription, signal) => at(subscription).stream(subscription, signal),
            confirm: (subscription, watermark) => at(subscription).confirm(subscription, watermark),
        };
    }

    /** Fill the options of serving the space its host decides: its identity, links and claims. */
    async options(): Promise<
        Pick<
            SpaceOptions,
            | "bases"
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
            | "bundled"
        >
    > {
        // call the universe as the machine, and claim names at the universe through it
        const accounts = this.accounts();

        // derive the base installation of each package the space is born with
        const bases = await Promise.all(
            SPACE_KERNEL.map(
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
            bundled: this.machine.bundled,
            publisher: this.uplink(),
            publisherAt: (home, partition) => this.publisherAt(home, partition),
            keystore: this.keystore,
        };
    }

    /** Serve each installation main applied: a shipped one in process, another in its sandbox. */
    async servers(
        installed: Awaited<ReturnType<typeof Database.installed>>,
        start: SpaceStart,
        signal: AbortSignal,
    ): Promise<SpaceServers> {
        // serve each installation in process or in its sandbox
        const servers: PackageImplementation[] = [];
        const sandboxes: HookSandbox[] = [];
        for (const { rollout: row, revisionId, manifest, declarations } of installed) {
            // serve a shipped server in process, with an activation that waits for its revision and any lent grant
            const shipped = this.#shipped(row, manifest);
            if (shipped !== undefined) {
                const caller = this.#callerOnce(row, shipped.package, start);
                const resources = await this.#resources(row, shipped.package, start, caller);
                const context = await this.#context(row, revisionId, resources, start, caller);
                const requested = Rollout.permissions(
                    row.installation,
                    shipped.package,
                    "installation",
                );
                const isLent = Object.entries(requested).some(
                    ([scope, each]) => scope !== "space" && each.length > 0,
                );
                const activation = (stopped: AbortSignal) =>
                    this.#activate(row, revisionId, isLent, AbortSignal.any([signal, stopped]));
                servers.push(shipped.at({ ...context, activation }));
            }
            // run another package's hooks in its sandbox, which binds its resources itself
            else {
                const caller = this.#callerOnce(row, { resources: {} }, start);
                const context = await this.#context(
                    row,
                    revisionId,
                    new ResourceContext(),
                    start,
                    caller,
                );
                sandboxes.push(start.sandbox(row, context, declarations));
            }
        }

        return { servers, sandboxes };
    }

    /** Activate an installation once it applied its revision and has any grant it borrows by. */
    async #activate(
        target: Rollout & { readonly installation: NamedInstallation },
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
        target: Rollout & { readonly installation: NamedInstallation },
        revisionId: Identifier<"installation-revision">,
        isLent: boolean,
        signal: AbortSignal,
    ): Promise<void> {
        // wait for the applied revision and for the replica of the grant its space lends it
        const parties = {
            delegator: principal.space.reference(Scope.universe.id, target.scope),
            delegate: principal.installation.reference(target.scope, target.id),
        };
        await this.database.log.until(async () => {
            // read the rollout's applied revision and the grant it borrows by
            const snapshot = Snapshot.live(this.database);
            const [row] = await snapshot.read(rollout.table).findMany({ where: { id: target.id } });
            const isApplied = row?.appliedRevisionId === revisionId;
            const isGranted =
                !isLent ||
                (await Grant.find(snapshot, parties, this.machine.environment.clock.now())) !==
                    undefined;

            return isApplied && isGranted;
        }, signal);
    }

    /** Find the server the machine ships for an installation, refusing an unshipped release. */
    #shipped(
        row: Rollout & { readonly installation: NamedInstallation },
        manifest: Digest,
    ): PackageImplementation | undefined {
        // find the server the machine runs for the installation's package
        const { servers, bundled } = this.machine;
        const server = servers.find((each) => each.package.id === row.installation.packageId);
        if (server === undefined) {
            return undefined;
        }

        // require the revision to install the release the machine ships with the server
        const { name } = server.package;
        const shipped = bundled.get(row.installation.packageId);
        if (shipped === undefined) {
            throw StackError.of("RELEASE_NOT_SHIPPED", `this machine has no release of ${name}`);
        } else if (shipped.manifest !== manifest) {
            throw StackError.of(
                "RELEASE_MISMATCH",
                `${name} only has release ${shipped.manifest}, not ${manifest}`,
            );
        }

        return server;
    }

    /** Bind a package's declared resources and secrets to the host's clients for it. */
    async #resources(
        row: Rollout & { readonly installation: NamedInstallation },
        handle: Pick<PackageHandle, "resources" | "secrets">,
        start: SpaceStart,
        caller: () => Promise<EgressCaller>,
    ): Promise<ResourceContext> {
        const bound = new ResourceContext();
        for (const declaration of [
            ...Object.values(handle.secrets),
            ...Object.values(handle.resources),
        ]) {
            // refuse a requirement another package declares
            if (!ResourceDeclaration.is(declaration)) {
                throw new TypeError("an in-process package can only bind its own declarations");
            }
            bound.bind(declaration, await this.#open(row, declaration, start, caller));
        }

        return bound;
    }

    /** Open a declaration an installation binds in this process, through what opens its kind. */
    async #open(
        row: Rollout & { readonly installation: NamedInstallation },
        declaration: ResourceDeclaration<unknown>,
        start: SpaceStart,
        caller: () => Promise<EgressCaller>,
    ): Promise<unknown> {
        // read the target the installation binds the declaration to
        const { scope, id } = row;
        const { packageId } = row.installation;
        const [bound] = await Snapshot.live(this.database)
            .read(binding.table)
            .findMany({ where: { scope, installationId: id, packageId, name: declaration.name } });
        const found = present(bound, `the binding of ${declaration.name} by ${id}`);
        const { target } = found;
        const served = start.bindables.find((each) => each.declaration === declaration.kind);
        const provider = start.providers.find((each) => each.kind.name === declaration.kind);

        // connect to a served object, such as a secret, or a served resource, such as a vault, through the store
        if (served !== undefined) {
            return served.connect(start.store, row, found);
        } else if (provider !== undefined && "connect" in provider) {
            return provider.connect(start.store, row, found);
        }
        // call a bound installation or HTTPS API through the machine's egress, as an instance's binding does
        else if (InstallationBinding.is(declaration) || declaration instanceof HttpsBinding) {
            return this.#client(declaration, found, start, caller);
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

    /** Open a bound installation's or HTTPS API's client, calling through the machine's egress. */
    async #client(
        declaration: InstallationBinding | HttpsBinding,
        bound: Pick<Binding, "target" | "targetSpaceId">,
        start: SpaceStart,
        caller: () => Promise<EgressCaller>,
    ): Promise<unknown> {
        // send each call as the installation's in-store caller
        const send = async (request: Request) => start.egress(await caller(), request);

        // call an HTTPS API below its address
        if (declaration instanceof HttpsBinding) {
            const url = Egress.url(EGRESS_ORIGIN, await this.#reference(bound));
            const client: HttpsClient & AsyncDisposable = {
                url,
                fetch: (path, init = {}) => send(new Request(`${url}${path}`, init)),
                [Symbol.asyncDispose]: () => Promise.resolve(),
            };

            return client;
        }

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

    /** Read the reference a local binding targets: a dependency's address or an API's id. */
    async #reference(bound: Pick<Binding, "target" | "targetSpaceId">): Promise<string> {
        // take another space's installation by its name there
        if (bound.targetSpaceId !== null) {
            return bound.target;
        }

        // read the reference of the dependency or the API the binding targets
        const view = Snapshot.live(this.database);
        const { target } = bound;
        const found = dependency.identifies(target)
            ? await view
                  .read(dependency.table)
                  .findFirst({ where: { id: dependency.identifier(target) } })
            : await view.read(https.table).findFirst({ where: { id: https.identifier(target) } });

        return ResourceRecord.reference(present(found, `the resource ${target}`));
    }

    /** Build an installation's in-store caller once per start, at its first call. */
    #callerOnce(
        row: Rollout & { readonly installation: NamedInstallation },
        handle: Pick<PackageHandle, "resources">,
        start: SpaceStart,
    ): () => Promise<EgressCaller> {
        let built: Promise<EgressCaller> | undefined;

        return () => (built ??= this.#caller(row, handle, start));
    }

    /** Read an in-store installation as its egress's caller, with what it binds. */
    async #caller(
        row: Rollout & { readonly installation: NamedInstallation },
        handle: Pick<PackageHandle, "resources">,
        start: SpaceStart,
    ): Promise<EgressCaller> {
        // read the installation's bindings and the resources here they target
        const view = Snapshot.live(this.database);
        const bindings = await view
            .read(binding.table)
            .findMany({ where: { scope: row.scope, installationId: row.id } });
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
                            ...(found.connection === null ? {} : { connection: found.connection }),
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
            installationId: row.id,
            resources: resources.flat(),
            installations,
            runs: { kind: "store" },
        };
    }

    /** Connect a resource where its provider placed it, through the machine's connector for it. */
    async #connect(
        row: Rollout & { readonly installation: NamedInstallation },
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
        row: Rollout & { readonly installation: NamedInstallation },
        revisionId: Identifier<"installation-revision">,
        resources: ResourceContext,
        start: SpaceStart,
        caller: () => Promise<EgressCaller>,
    ): Promise<InstalledServerContext> {
        // call the space's stack package in process as the installation
        const { database, machine } = this;
        const { id, scope } = row;
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
        const build = await InstallationRevision.reader(
            Snapshot.live(database),
            revisionId,
            machine.builds.packages,
        );
        const description = await PackageDescription.of(build);
        const identity = present(
            await this.keystore.deriver(database, scope),
            `the keys of ${scope}, which this machine keeps`,
        );

        // serve below the installation's mount at this machine's origin, on the network its sandbox lists
        const named = row.installation;
        const network = this.#network(named.name, Rollout.sandbox(row.installation, description));
        const { clock, random, crypto, compression, telemetry, locale, timeZone } =
            machine.environment;

        return {
            origin: named.origin,
            installation: id,
            name: named.name,
            scope,
            url: InstallationMount.url(machine.origin, scope, id),
            build,
            resources,
            capabilities: Rollout.enabled(row.installation, description),
            keys: await InstallationRoot.open(
                await InstallationRoot.derive(identity, id),
                machine.environment.crypto,
            ),
            fetch: (request) => network.fetch(request),
            environment: {
                clock,
                random,
                crypto,
                network,
                compression,
                telemetry,
                locale,
                timeZone,
            },
            settings: await Rollout.settings(database, build, { scope, installation: id }),
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

    /** Build the network an in-store installation reaches: the hosts its sandbox lists. */
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
}
