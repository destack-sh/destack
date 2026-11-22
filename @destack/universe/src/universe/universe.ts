import { InstallationOrigin, type UniverseLink, UniverseOrigin } from "@destack/account/address";
import { Directory, Issuer, Session } from "@destack/account/client";
import { defineDatabase } from "@destack/db";
import type { Alarm, DatabaseConnection, Snapshot } from "@destack/db/database";
import type { BlobStorage } from "@destack/resource";
import {
    AccountAuthentication,
    BetterAuthAuthenticator,
    type BetterAuthAuthenticatorOptions,
} from "@destack/account/better-auth";
import type { MailTransport } from "@destack/mail/transport";
import { IdentityKeystore, IdentityRegistry } from "@destack/account/identity";
import { OAuthError } from "@destack/account/authentication";
import { Bookmark } from "@destack/object/client";
import { Account, claim, Residency } from "@destack/account/object";
import registryPackage from "@destack/registry/package";
import { packageObject } from "@destack/registry/object";
import { RegistryError } from "@destack/registry/error";
import { RemoteBuildStore } from "@destack/build/store";
import type { BuildLayer } from "@destack/package/manifest";
import accountPackage from "@destack/account/package";
import { ACCESS_TOKEN_TYPE } from "@destack/account/oauth";
import {
    accountServer,
    DnsOverHttps,
    Platform,
    type ReleaseRegistry,
} from "@destack/account/server";
import type { Bucket } from "@destack/bucket";
import { DatabasePersonalKeyring, EventStore } from "@destack/event/store";
import type { Keyring } from "@destack/identity";
import { call, type CallKey, type Fetch, PackageMount, tablesOf } from "@destack/object";
import { ObjectConnectionHost, Health, Server } from "@destack/object/server";
import objectServer, { implementPackage, ObjectStore } from "@destack/object/server";
import { PackageVersion, PackageId, PackageRelease } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { Digest, present, schema } from "@destack/schema";
import { type Publisher, RequestId, Scope } from "@destack/sync";
import { settingServer } from "@destack/setting/server";
import settingPackage from "@destack/setting/package";
import objectPackage from "@destack/object/package";
import universePackage from "../package.ts";
import { Run } from "@destack/workflow";
import { apply } from "./apply.ts";
import { UniverseDefinition } from "./definition.ts";
import { UniverseError } from "../error/error.ts";
import { Bearer } from "@destack/access/token";
import { BasicCredential, type Environment } from "@destack/environment";

/** The schema the universe's tables live in, PostgreSQL's default. */
export const UNIVERSE_SCHEMA = "public";

/** The longest drain on shutdown, in milliseconds. */
const DRAIN_MILLISECONDS = 5000;

/** The realm a repository link challenges git clients for. */
const REALM = "destack";

/** The status of a request without valid credentials (RFC 9110 15.5.2). */
const UNAUTHORIZED_STATUS = 401;

/** What a host runs a universe with. */
export interface UniverseOptions {
    /** The universe's definition. */
    readonly definition: UniverseDefinition;
    /** The universe's address, below whose origin each package answers at its mount. */
    readonly origin: UniverseOrigin;
    /** The migrated account database. */
    readonly database: DatabaseConnection;
    /** The universe's root keys, which store its identity keys and derive its secrets. */
    readonly keyring: Keyring;
    /** The key the journals fingerprint sensitive inputs under. */
    readonly callKey: CallKey;
    /** The transport mailing sign-in codes. */
    readonly mail: MailTransport;
    /** The applications at the providers people sign in through, none where the host has none. */
    readonly providers: BetterAuthAuthenticatorOptions["providers"];
    /** The bucket with the segments of the universe's call events. */
    readonly events: Bucket;
    /** The storage of the universe's blobs, which its rows' blob columns reference. */
    readonly blobs: BlobStorage;
    /** The proxy and address headers the host's ingress sets. */
    readonly ipAddress: BetterAuthAuthenticatorOptions["ipAddress"];
    /** The clock the universe runs at, and the network it sends to other origins through. */
    readonly environment: Environment;
    /** The wake-up of a host that evicts the universe between requests. */
    readonly alarm?: Alarm;
    /** Report a failure the universe answers to no one. */
    readonly report: (error: unknown) => void;
    /** The releases the universe's distribution has, which it opens before any forge's. */
    readonly distribution: readonly PackageRelease[];
    /** The builds of the distribution's releases. */
    readonly layer: BuildLayer;
}

/** The packages of the universe itself, which its store serves acting as the universe. */
export const UNIVERSE_PACKAGES = [objectPackage, accountPackage, settingPackage, universePackage];

/** A running universe: its packages over one store, each below its mount. */
export class Universe implements AsyncDisposable {
    /** The universe's database: the packages its store serves. */
    static readonly database = defineDatabase({
        name: "account",
        ...tablesOf({
            packages: UNIVERSE_PACKAGES,
            follows: [],
            tables: [],
        }),
    });

    /** The universe's address: its origin and its domains. */
    readonly origin: UniverseOrigin;
    /** What the universe declares. */
    readonly definition: UniverseDefinition;
    /** The server of each package, by its identifier. */
    readonly servers: ReadonlyMap<string, Server>;
    /** The store serving the universe's packages, which runs their internal methods in process. */
    readonly store: ObjectStore;
    /** The account package's server, which answers as the universe's issuer. */
    readonly issuer: Server;
    /** The servers' connections, whose sockets the universe's listener keeps through it. */
    readonly connections: ObjectConnectionHost;
    /** Aborts the store's loop. */
    readonly #stopping: AbortController;
    /** The store's running loop. */
    readonly #running: Promise<void>;
    /** Closes the publishers of the residencies' homes the store follows. */
    readonly #publishers: DisposableStack;
    /** The universe's issuer as a client, exchanging callers' tokens for their packages'. */
    readonly #exchanger: Issuer;
    /** Sends to other origins, and to the universe's own in process. */
    readonly #send: Fetch;

    /** Create a started universe. */
    private constructor(
        origin: UniverseOrigin,
        definition: UniverseDefinition,
        servers: ReadonlyMap<string, Server>,
        store: ObjectStore,
        issuer: Server,
        connections: ObjectConnectionHost,
        loop: {
            readonly stopping: AbortController;
            readonly running: Promise<void>;
            readonly publishers: DisposableStack;
        },
        exchanger: Issuer,
        send: Fetch,
    ) {
        // keep the definition, the servers, the store, the issuer, the connections and the store's loop
        this.origin = origin;
        this.definition = definition;
        this.servers = servers;
        this.store = store;
        this.issuer = issuer;
        this.connections = connections;
        this.#stopping = loop.stopping;
        this.#running = loop.running;
        this.#publishers = loop.publishers;
        this.#exchanger = exchanger;
        this.#send = send;
    }

    /** Migrate the universe's database to this release, then start its packages over it. */
    static async start(options: UniverseOptions): Promise<Universe> {
        // read the universe's options at the start time
        const { database, keyring, callKey, definition } = options;
        const { origin } = options.origin;
        const { clock, network } = options.environment;
        const now = clock.now();

        // migrate the universe's tables
        await database.migrate(Universe.database.tables);

        // record the universe's calls as call events, their segments in the bucket
        const events = new EventStore({
            database,
            kinds: [call.eventKind()],
            files: () => options.events,
            personal: new DatabasePersonalKeyring(database, keyring, options.environment),
            environment: options.environment,
        });

        // resolve the sign-in, the session and the universe once the store exists
        const signIn = Promise.withResolvers<BetterAuthAuthenticator>();
        const connected = Promise.withResolvers<Session>();
        const serving = Promise.withResolvers<Universe>();

        // call residency homes as the universe once its identity exists
        const connect =
            (audience: PackageId): Fetch =>
            async (request) =>
                (await connected.promise).fetch(audience)(request);
        const directory = new Directory(database.snapshot());

        // follow each residency home over one publisher, closed with the universe
        const publishers = new Map<string, Publisher>();
        const following = new DisposableStack();
        const releases: ReleaseRegistry = {
            open: async (packageId, named) => {
                // open a release of the distribution from its layer
                const distributed = PackageRelease.select(options.distribution, packageId, named);
                if (distributed !== undefined) {
                    const image = present(
                        options.layer.image(distributed.image),
                        `image ${distributed.image}`,
                    );

                    return await present(options.layer.build(image.root), `build ${image.root}`);
                }

                // find the forge in the home of the package's publisher, refusing a package none claims
                const owner = await directory.owner(packageObject, packageId);
                if (owner === undefined) {
                    throw RegistryError.of(
                        "PUBLISHER_NOT_FOUND",
                        `owning account for '${packageId}' not found`,
                    );
                }
                const home = await Account.locate(directory.snapshot, owner);
                const fetch = connect(registryPackage.id);

                // resolve the release there as the universe, then open its root build
                const registry = directory.client(
                    registryPackage,
                    home,
                    fetch,
                    options.environment,
                );
                const found = await registry.release.resolve({
                    spaceId: home,
                    parentId: packageId,
                    release: named,
                });
                const mount = await directory.package(home, registryPackage.id, clock.now());
                const builds = `${mount}${registryPackage.mounts.builds.path}`;

                return await RemoteBuildStore.open(
                    builds,
                    fetch,
                    found.image,
                    options.environment.crypto,
                );
            },
            distribution: options.distribution,
        };

        // apply the residencies, then each step of the platform's definition, once per run
        const applied = apply.implement({
            run: async (step) => {
                const steps = [
                    {
                        name: "residencies",
                        apply: () => Residency.apply(store, definition.residencies),
                    },
                    ...platform.steps(),
                ];
                for (const each of steps) {
                    await step.run(each.name, schema.null(), async () => {
                        await each.apply(clock.now());

                        return null;
                    });
                }
            },
        });

        // serve the object, account, setting and universe packages over one store, acting as the universe
        const implementations = [
            objectServer,
            accountServer({
                universe: options.origin,
                resolver: new DnsOverHttps((request) => network.fetch(request)),
                stacks: definition.platform.stacks,
                signIn: signIn.promise,
                releases,
            }),
            settingServer({ after: [] }),
            implementPackage(universePackage, { objects: applied.objects }),
        ];
        const store: ObjectStore = new ObjectStore({
            // open the version a value names, else the release the universe runs: the distribution's
            release: async (_view, _scope, packageId, version) => {
                const runs = PackageRelease.newest(releases.distribution, packageId);
                const named = version ?? runs?.package.version;

                return named === undefined ? undefined : await releases.open(packageId, named);
            },
            servers: implementations,
            // follow no chain at the universe, the root of every chain
            chain: async () => undefined,
            principal: Scope.universe,
            partitions: [Scope.universe.id],
            database,
            blobs: options.blobs,
            claims: { object: claim },
            environment: options.environment,
            callKey,
            events,
            origin: { package: PackageVersion.of(accountPackage) },
            // follow the residencies' homes where their machines answer
            publisher: (scope) => {
                // follow each home over one publisher, closed with the universe
                const known = publishers.get(scope);
                if (known !== undefined) {
                    return known;
                }
                const publisher = following.use(
                    directory.publisher(
                        scope,
                        async (request) =>
                            (await connected.promise).credential.authorize(
                                request,
                                objectPackage.id,
                                {},
                            ),
                        options.environment,
                    ),
                );
                publishers.set(scope, publisher);

                return publisher;
            },
        });

        // apply the platform through the universe's store
        const platform = new Platform(store, definition.platform);

        // apply the definition unless its latest run completed and the operator stays invited
        const digest = await Digest.json(definition, options.environment.crypto);
        const [latest] = await database
            .snapshot()
            .read(apply.run.table)
            .findMany({ where: { key: digest }, orderBy: { createdAt: "desc" }, limit: 1 });
        const isApplied = latest?.state === "completed" && (await platform.isOperatorInvited(now));

        // follow the applied run, else the run the start answers: a new one or the live one
        const applier = store.as(apply.run);
        const [current] = isApplied
            ? [latest]
            : await store.mutate(applier, {
                  id: RequestId.create(store.environment),
                  calls: [apply.start({ digest }, { key: digest, actor: applier.subject })],
              });
        const applying = new Run(apply, current);

        // sign in with the universe's identity
        const keystore = await IdentityKeystore.universe(
            keyring,
            database,
            IdentityRegistry.of(store),
            options.environment,
        );

        // key the universe's sensitive columns with its identity's column key
        await keystore.key(database, Scope.universe.id);
        const { issuer } = options.origin;
        signIn.resolve(
            await BetterAuthAuthenticator.create({
                keystore,
                callKey,
                universe: options.origin,
                ipAddress: options.ipAddress,
                database,
                providers: options.providers,
                objects: store,
                events,
                mail: options.mail,
            }),
        );

        // call packages in spaces as the universe, with tokens its identity signs, its own origin on loopback
        const identity = keystore.identityOf(database, Scope.universe, {
            issuer,
            environment: options.environment,
            send: async (request) =>
                request.url.startsWith(origin)
                    ? await (await serving.promise).fetch(request)
                    : await network.fetch(request),
        });
        const session = new Session(identity, {
            universe: origin,
            send: identity.send,
            bookmark: new Bookmark(),
            environment: options.environment,
        });
        connected.resolve(session);

        // serve once the run applied the definition, refusing a run that did not complete
        const ready = async (signal: AbortSignal) => {
            const run = await applying.ended(database, signal);
            if (run.row.state !== "completed") {
                throw UniverseError.of(
                    "DEFINITION_UNAVAILABLE",
                    `definition '${digest}' is unavailable: its run is '${run.row.state}'`,
                    { cause: run.row.failure },
                );
            }
        };

        // answer each package to bearers of the universe's access tokens
        const verifier = AccountAuthentication.universe(
            database.snapshot(),
            issuer,
            options.environment,
        );
        const connections = new ObjectConnectionHost(options.environment.telemetry);
        const servers = new Map(
            store.packages.map((implementation) => [
                implementation.package.id,
                Server.start({
                    ...implementation,
                    hibernation: connections.hibernation,
                    environment: options.environment,
                    health: new Health(implementation.package.name),
                    resources: new ResourceContext(),
                    authenticate: (request) =>
                        AccountAuthentication.bearer(request, verifier, {
                            audience: implementation.package.id,
                            now: clock.now(),
                        }),
                    authorizeMachine: async () => {},
                    drainTimeout: DRAIN_MILLISECONDS,
                    ready,
                }),
            ]),
        );

        // serve the issuer through the account package's server, which every universe serves
        const accounts = servers.get(accountPackage.id);
        if (accounts === undefined) {
            throw UniverseError.of("ISSUER_BROKEN", "the universe serves no account package");
        }
        // run the store's jobs in its loop
        const stopping = new AbortController();
        const running = store
            .run(stopping.signal, options.alarm === undefined ? {} : { alarm: options.alarm })
            .catch((error: unknown) => options.report(error));
        // take each server's connections, then start
        for (const [packageId, server] of servers) {
            connections.serve(PackageId.parse(packageId), server);
        }
        const exchanger = new Issuer(issuer, identity.send, options.environment, new Bookmark());
        const universe = new Universe(
            options.origin,
            definition,
            servers,
            store,
            accounts,
            connections,
            { stopping, running, publishers: following },
            exchanger,
            identity.send,
        );
        serving.resolve(universe);

        return universe;
    }

    /** Serve a request at the universe's origin: the issuer, a link, else a package's path. */
    fetch(request: Request): Promise<Response> {
        // serve the issuer through the account package's oauth mount
        const { pathname } = new URL(request.url);
        const link = UniverseOrigin.link(pathname);
        if (UniverseOrigin.path(pathname) === "issuer") {
            return this.issuer.fetch(request);
        } else if (link !== undefined) {
            return this.#open(link, request);
        }

        // answer the package mounted at the path, nothing elsewhere
        const mounted = PackageMount.of(request);
        const server = mounted === undefined ? undefined : this.servers.get(mounted.packageId);

        return mounted === undefined || server === undefined
            ? Promise.resolve(new Response(null, { status: 404 }))
            : server.fetch(mounted.request);
    }

    /** Redirect an app link to its installation, or forward a repository link to its registry. */
    async #open(link: UniverseLink, request: Request): Promise<Response> {
        // redirect an app by rewriting its path into its installation's hostname
        if (link.kind === "app") {
            const { domains } = this.origin;
            const hostname = InstallationOrigin.format(link.installation, domains.space);

            return new Response(null, {
                status: 303,
                headers: { location: `${this.origin.url(hostname)}/` },
            });
        }

        // forward a repository to its owner's registry, refusing an unclaimed handle
        const snapshot = this.store.database.snapshot();
        const owner = await Account.byHandle(snapshot, link.handle);
        if (owner === undefined) {
            return new Response(null, { status: 404 });
        }
        const { pathname } = new URL(request.url);

        return await this.#forward(request, owner, snapshot, registryPackage.id, {
            mount: registryPackage.mounts.git.path,
            path: pathname,
        });
    }

    /** Forward a request below a package's mount in the owner's residency home, as its caller. */
    async #forward(
        request: Request,
        owner: Account,
        snapshot: Snapshot,
        packageId: PackageId,
        target: { readonly mount: string; readonly path: string },
    ): Promise<Response> {
        // locate the package in the owner's home, on its machine now
        const home = await Account.locate(snapshot, owner.id);
        const now = this.store.environment.clock.now();
        const endpoint = await new Directory(snapshot).package(home, packageId, now);
        const { search } = new URL(request.url);
        const url = `${endpoint}${target.mount}${target.path}${search}`;

        // exchange the caller's account token for the package's in the home, an anonymous caller going on as one
        const headers = new Headers(request.headers);
        const authorization = headers.get("authorization");
        const presented =
            Bearer.token(authorization) ?? BasicCredential.read(authorization)?.password;
        if (presented !== undefined) {
            const exchanged = await this.#exchanger
                .exchange({ token: presented, type: ACCESS_TOKEN_TYPE }, packageId, {
                    partition: home,
                })
                .then(
                    (granted) => ({ granted }),
                    (error: unknown) => {
                        // answer the issuer's refusal as a 401, failing loudly on anything else
                        if (!OAuthError.is(error)) {
                            throw error;
                        }

                        return { refused: error.message };
                    },
                );
            if ("refused" in exchanged) {
                return new Response(exchanged.refused, {
                    status: 401,
                    headers: { "www-authenticate": BasicCredential.challenge(REALM) },
                });
            }
            headers.set("authorization", `Bearer ${exchanged.granted.accessToken}`);
        }

        // send it on to the home
        const answer = await this.#send(
            new Request(url, {
                method: request.method,
                headers,
                body: request.body,
                redirect: "manual",
            }),
        );

        // challenge an unauthenticated caller for the credentials git sends
        if (answer.status === UNAUTHORIZED_STATUS && !answer.headers.has("www-authenticate")) {
            const challenged = new Headers(answer.headers);
            challenged.set("www-authenticate", BasicCredential.challenge(REALM));

            return new Response(answer.body, { status: answer.status, headers: challenged });
        }

        return answer;
    }

    /** Wait until the store's loop is idle, or until a deadline. */
    async idle(deadline?: number): Promise<void> {
        await this.store.idle(deadline);
    }

    /** Stop the store's loop, drain every package, then close the homes' publishers. */
    async [Symbol.asyncDispose](): Promise<void> {
        // stop the loop and drain the packages
        this.#stopping.abort();
        await this.#running;
        for (const server of this.servers.values()) {
            await server.close();
        }

        // close the publishers the store followed through
        this.#publishers.dispose();
    }
}
