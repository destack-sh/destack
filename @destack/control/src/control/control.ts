import { Directory, UniverseOrigin } from "@destack/account/address";
import { type Alarm, type DatabaseConnection, defineDatabase, Snapshot } from "@destack/db";
import type { BlobStorage } from "@destack/resource";
import {
    AccountAuthentication,
    BetterAuthAuthenticator,
    type BetterAuthAuthenticatorOptions,
} from "@destack/account/better-auth";
import type { MailTransport } from "@destack/mail/transport";
import { IdentityKeystore, IdentityRegistry } from "@destack/account/identity";
import { claim, Residency } from "@destack/account/object";
import accountPackage from "@destack/account/package";
import { TokenLifetime } from "@destack/account/oauth";
import {
    accountServer,
    DnsOverHttps,
    Platform,
    type ReleaseRegistry,
} from "@destack/account/server";
import type { Bucket } from "@destack/bucket";
import { DatabasePersonalKeyring, EventStore } from "@destack/event";
import { type Keyring } from "@destack/identity";
import {
    call,
    type CallKey,
    type Fetch,
    Installation,
    PackageMount,
    tablesOf,
} from "@destack/object";
import { Health, Server } from "@destack/object/server";
import objectServer, { implementPackage, ObjectStore } from "@destack/object/server";
import { Package, type PackageId, PackageRelease } from "@destack/package";
import { AccessContext, Authorization, Caller, type Policy } from "@destack/access";
import { ResourceContext } from "@destack/resource/context";
import { Digest, schema } from "@destack/schema";
import { RequestId, Scope } from "@destack/sync";
import { settingServer } from "@destack/setting/server";
import settingPackage from "@destack/setting/package";
import objectPackage from "@destack/object/package";
import controlPackage from "../package.ts";
import { Run } from "@destack/workflow";
import { apply } from "./apply.ts";
import { UniverseDefinition } from "./definition.ts";
import { ControlError } from "../error/error.ts";
import type { Environment } from "@destack/environment";

/** The packages the control's store serves. */
export const CONTROL_PACKAGES = [objectPackage, accountPackage, settingPackage, controlPackage];

/** The schema the universe's tables live in, PostgreSQL's default. */
export const UNIVERSE_SCHEMA = "public";

/** The longest drain on shutdown, in milliseconds. */
const DRAIN_MILLISECONDS = 5000;

/** What a host runs a universe with. */
export interface ControlOptions {
    /** The universe's definition. */
    readonly definition: UniverseDefinition;
    /** The universe's origin, below which each package answers at its mount. */
    readonly origin: string;
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
    /** Open the registry of packages' releases over the universe's directory and signed fetch. */
    readonly releases: (universe: ControlLink) => ReleaseRegistry;
    /** The policies of other packages the universe decides access for, such as the forge's. */
    readonly policies: readonly Policy[];
    /** The access token lifetimes longer than the default minute, by audience. */
    readonly lifetimes: ReadonlyMap<PackageId, number>;
}

/** What a universe lends a registry of releases: its directory and its fetch signed as itself. */
export interface ControlLink {
    /** The universe's directory, its own rows read live. */
    readonly directory: Directory;
    /** Create a fetch calling a package as the universe. */
    readonly connect: (audience: PackageId) => Fetch;
}

/** The control running a universe: its packages over one store, each below its mount. */
export class Control implements AsyncDisposable {
    /** The universe's database: the packages its store serves. */
    static readonly database = defineDatabase({
        name: "account",
        ...tablesOf({
            packages: CONTROL_PACKAGES,
            follows: [],
            tables: [],
        }),
    });

    /** What the universe declares. */
    readonly definition: UniverseDefinition;
    /** The server of each package, by its identifier. */
    readonly servers: ReadonlyMap<string, Server>;
    /** The store serving the universe's packages, which runs their internal methods in process. */
    readonly store: ObjectStore;
    /** The account package's server, which answers as the universe's issuer. */
    readonly issuer: Server;

    /** Create a started control. */
    private constructor(
        definition: UniverseDefinition,
        servers: ReadonlyMap<string, Server>,
        store: ObjectStore,
        issuer: Server,
    ) {
        // keep the definition, the servers, the store and the issuer
        this.definition = definition;
        this.servers = servers;
        this.store = store;
        this.issuer = issuer;
    }

    /** Start the universe's packages over its database. */
    static async start(options: ControlOptions): Promise<Control> {
        // read the universe's options at the start time
        const { origin, database, keyring, callKey, definition } = options;
        const { clock, network } = options.environment;
        const now = clock.now();

        // record the universe's calls as call events, their segments in the bucket
        const events = new EventStore({
            database,
            kinds: [call.eventKind()],
            files: () => options.events,
            personal: new DatabasePersonalKeyring(database, keyring, options.environment),
            clock,
        });

        // bind the parts that need the store once it exists: the account server's sign-in stores
        //  through the store, and the universe's signed fetch registers its identity there
        const signIn = Promise.withResolvers<BetterAuthAuthenticator>();
        const connected = Promise.withResolvers<(audience: PackageId) => Fetch>();
        const serving = Promise.withResolvers<Control>();

        // call residency homes as the universe once its identity exists
        const connect =
            (audience: PackageId): Fetch =>
            async (request) =>
                (await connected.promise)(audience)(request);
        const directory = new Directory(Snapshot.live(database));
        const releases = options.releases({ directory, connect });

        // apply the residencies, then each step of the platform's definition, once per run
        const applied = apply.implement(async (step) => {
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
        });

        // serve the account, setting and control packages over one store, each as its base installation
        const implementations = [
            objectServer,
            accountServer({
                resolver: new DnsOverHttps((request) => network.fetch(request)),
                stacks: definition.platform.stacks,
                signIn: signIn.promise,
                releases,
            }),
            settingServer({ after: [] }),
            implementPackage(controlPackage, {
                objects: applied.objects,
                cursors: applied.cursors,
            }),
        ];
        const { crypto } = options.environment;
        const store: ObjectStore = new ObjectStore({
            // open the version a value names, else the release the universe runs: the distribution's
            release: async (_view, _scope, packageId, version) => {
                const runs = PackageRelease.newest(releases.distribution, packageId);
                const named = version ?? runs?.package.version;

                return named === undefined ? undefined : await releases.open(packageId, named);
            },
            servers: await Promise.all(
                implementations.map((server) => server.base(Scope.universe, crypto)),
            ),
            // follow no chain at the universe, the root of every chain
            chain: async () => undefined,
            principal: Scope.universe,
            partitions: [Scope.universe.id],
            policies: options.policies,
            database,
            blobs: options.blobs,
            claims: { object: claim },
            environment: options.environment,
            callKey,
            events,
            origin: { package: Package.of(accountPackage) },
            // follow the residencies' homes where their machines answer
            publisher: (scope) =>
                directory.publisher(scope, connect(objectPackage.id), options.environment),
        });

        // apply the platform through the universe's store
        const platform = new Platform(store, definition.platform);

        // receive the universe at its first start
        await Authorization.run(
            store.authorizer,
            store.database,
            () => AccessContext.of(Caller.of(Scope.universe), now),
            store.environment,
            (authorization) =>
                Installation.receive(authorization, Scope.universe, [], CONTROL_PACKAGES),
        );

        // apply the definition unless its latest run completed and the operator stays invited
        const digest = await Digest.json(definition, options.environment.crypto);
        const [latest] = await Snapshot.live(database)
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
        const issuer = UniverseOrigin.issuer(origin);
        signIn.resolve(
            await BetterAuthAuthenticator.create({
                keystore,
                callKey,
                issuer,
                trustedOrigins: [origin],
                ipAddress: options.ipAddress,
                database,
                providers: options.providers,
                objects: store,
                events,
                lifetime: new TokenLifetime(options.lifetimes),
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
        connected.resolve((audience) => identity.fetchSigned(audience));

        // serve once the run applied the definition, refusing a run that did not complete
        const ready = async (signal: AbortSignal) => {
            const run = await applying.ended(database, signal);
            if (run.row.state !== "completed") {
                throw ControlError.of(
                    "DEFINITION_NOT_APPLIED",
                    `the run applying the definition ${digest} is ${run.row.state}`,
                    { cause: run.row.failure },
                );
            }
        };

        // answer each package to bearers of the universe's access tokens, the first running the controllers
        const servers = new Map(
            store.packages.map((implementation, index) => [
                implementation.package.id,
                Server.start({
                    ...implementation,
                    environment: options.environment,
                    health: new Health(implementation.package.name),
                    resources: new ResourceContext(),
                    authenticate: (request) =>
                        AccountAuthentication.bearer(request, {
                            database,
                            issuer,
                            audience: implementation.package.id,
                            now: clock.now(),
                            environment: options.environment,
                        }),
                    authorizeMachine: async () => {},
                    drainTimeout: DRAIN_MILLISECONDS,
                    ready,
                    ...(options.alarm === undefined || index > 0 ? {} : { alarm: options.alarm }),
                    report: (error) => options.report(error),
                }),
            ]),
        );

        // serve the issuer through the account package's server, which every universe serves
        const accounts = servers.get(accountPackage.id);
        if (accounts === undefined) {
            throw ControlError.of("ISSUER_MISSING", "the universe serves no account package");
        }
        const control = new Control(definition, servers, store, accounts);
        serving.resolve(control);

        return control;
    }

    /** Serve a request at the universe's origin: the issuer, else a package's mount. */
    fetch(request: Request): Promise<Response> {
        // serve the issuer through the account package's oauth adapter
        if (UniverseOrigin.path(new URL(request.url).pathname) === "issuer") {
            return this.issuer.fetch(request);
        }

        // answer the package mounted at the path, nothing elsewhere
        const mounted = PackageMount.of(request);
        const server = mounted === undefined ? undefined : this.servers.get(mounted.packageId);

        return mounted === undefined || server === undefined
            ? Promise.resolve(new Response(null, { status: 404 }))
            : server.fetch(mounted.request);
    }

    /** Wait until every package's controllers are idle, or until a deadline. */
    async idle(deadline?: number): Promise<void> {
        await Promise.all([...this.servers.values()].map((server) => server.idle(deadline)));
    }

    /** Drain every package. */
    async [Symbol.asyncDispose](): Promise<void> {
        for (const server of this.servers.values()) {
            await server.close();
        }
    }
}
