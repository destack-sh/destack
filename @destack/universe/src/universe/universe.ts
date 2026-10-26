import { Directory } from "@destack/account/address";
import {
    AccountAuthentication,
    BetterAuthAuthenticator,
    type BetterAuthAuthenticatorOptions,
} from "@destack/account/better-auth";
import { IdentityKeystore, IdentityRegistry } from "@destack/account/identity";
import { claim, Residency } from "@destack/account/object";
import accountPackage, { OAUTH_PATH } from "@destack/account/package";
import { TokenLifetime } from "@destack/account/oauth";
import {
    accountServer,
    DnsOverHttps,
    Platform,
    type ReleaseRegistry,
} from "@destack/account/server";
import type { Bucket } from "@destack/bucket";
import { type DatabaseConnection, defineDatabase, Snapshot } from "@destack/db";
import { DatabasePersonalKeyring, EventStore } from "@destack/event";
import { type Keyring } from "@destack/identity";
import type { MailTransport } from "@destack/account/better-auth";
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
import { Package, type PackageId } from "@destack/package";
import { AccessContext, Authorization, Caller, type Policy } from "@destack/access";
import { ResourceContext } from "@destack/resource/context";
import { Digest, present, schema } from "@destack/schema";
import type { Alarm } from "@destack/db";
import { RequestId, Scope } from "@destack/sync";
import { Setting } from "@destack/setting";
import { settingServer } from "@destack/setting/server";
import settingPackage from "@destack/setting/package";
import objectPackage from "@destack/object/package";
import universePackage from "../package.ts";
import { Run } from "@destack/workflow";
import { apply } from "./apply.ts";
import { UniverseDefinition } from "./definition.ts";
import { UniverseError } from "../error/error.ts";
import type { Environment } from "@destack/environment";

/** The packages a universe's store serves. */
export const UNIVERSE_PACKAGES = [objectPackage, accountPackage, settingPackage, universePackage];

/** The schema the universe's tables live in, PostgreSQL's default. */
export const UNIVERSE_SCHEMA = "public";

/** The universe's database: the packages its store serves. */
export const universeDatabase = defineDatabase({
    name: "account",
    ...tablesOf({
        packages: UNIVERSE_PACKAGES,
        follows: [],
        tables: [],
    }),
});

/** The longest drain on shutdown, in milliseconds. */
const DRAIN_MILLISECONDS = 5000;

/** What a host runs a universe with. */
export interface UniverseOptions {
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
    /** The transport mailing sign-in links and codes. */
    readonly mail: MailTransport;
    /** The bucket with the segments of the universe's call events. */
    readonly events: Bucket;
    /** The proxy and address headers the host's ingress sets. */
    readonly ipAddress: BetterAuthAuthenticatorOptions["ipAddress"];
    /** The clock the universe runs at, and the network it sends to other origins through. */
    readonly environment: Environment;
    /** The wake-up of a host that evicts the universe between requests. */
    readonly alarm?: Alarm;
    /** Report a failure the universe answers to no one. */
    readonly report: (error: unknown) => void;
    /** Open the registry of packages' releases over the universe's directory and signed fetch. */
    readonly releases: (universe: UniverseLink) => ReleaseRegistry;
    /** The policies of other packages the universe decides access for, such as the forge's. */
    readonly policies: readonly Policy[];
    /** The access token lifetimes longer than the default minute, by audience. */
    readonly lifetimes: ReadonlyMap<PackageId, number>;
}

/** What a universe lends a registry of releases: its directory and its fetch signed as itself. */
export interface UniverseLink {
    /** The universe's directory, its own rows read live. */
    readonly directory: Directory;
    /** Create a fetch calling a package as the universe. */
    readonly connect: (audience: PackageId) => Fetch;
}

/** A running universe: the object, account, setting and universe packages, each below its mount. */
export class Universe implements AsyncDisposable {
    /** What the universe declares. */
    readonly definition: UniverseDefinition;
    /** The server of each package, by its identifier. */
    readonly servers: ReadonlyMap<string, Server>;
    /** The store serving the universe's packages, which runs their internal methods in process. */
    readonly store: ObjectStore;

    /** Create a started universe. */
    private constructor(
        definition: UniverseDefinition,
        servers: ReadonlyMap<string, Server>,
        store: ObjectStore,
    ) {
        this.definition = definition;
        this.servers = servers;
        this.store = store;
    }

    /** Start the universe's packages over its database. */
    static async start(options: UniverseOptions): Promise<Universe> {
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
        const serving = Promise.withResolvers<Universe>();

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

        // serve the account, setting and universe packages over one store, each as its base installation
        const base = (handle: (typeof UNIVERSE_PACKAGES)[number]) =>
            Installation.base(Scope.universe, handle, options.environment.crypto);
        const store: ObjectStore = new ObjectStore({
            servers: [
                objectServer.at(await base(objectPackage)),
                accountServer({
                    resolver: new DnsOverHttps((request) => network.fetch(request)),
                    stacks: definition.stacks,
                    signIn: signIn.promise,
                    releases,
                }).at(await base(accountPackage)),
                settingServer({
                    declared: Setting.declared((value) =>
                        releases.open(
                            value.packageId,
                            present(value.release, `the release of ${value.name}`),
                        ),
                    ),
                    after: [],
                }).at(await base(settingPackage)),
                implementPackage(universePackage, {
                    objects: applied.objects,
                    controllers: applied.controllers,
                }).at(await base(universePackage)),
            ],
            // follow no chain at the universe, the root of every chain
            chain: async () => undefined,
            principal: Scope.universe,
            partitions: [Scope.universe.id],
            policies: options.policies,
            database,
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
        const platform = new Platform(store, definition.platform, definition.stacks.residency);

        // receive the universe at its first start
        await Authorization.run(
            store.authorizer,
            store.database,
            () => AccessContext.of(Caller.of(Scope.universe), now),
            store.environment,
            (authorization) =>
                Installation.receive(authorization, Scope.universe, [], UNIVERSE_PACKAGES, store),
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
        const issuer = `${PackageMount.url(origin, accountPackage.id)}${OAUTH_PATH}`;
        signIn.resolve(
            await BetterAuthAuthenticator.create({
                keystore,
                callKey,
                issuer,
                trustedOrigins: [origin],
                ipAddress: options.ipAddress,
                database,
                providers: {},
                objects: store,
                events,
                lifetime: new TokenLifetime(options.lifetimes),
                mail: options.mail,
            }),
        );

        // call packages in spaces as the universe, with tokens its identity signs, its own origin on loopback
        const source = keystore.source(database, Scope.universe, {
            issuer,
            environment: options.environment,
            send: async (request) =>
                request.url.startsWith(origin)
                    ? await (await serving.promise).fetch(request)
                    : await network.fetch(request),
        });
        connected.resolve((audience) => source.fetchSigned(audience));

        // serve once the run applied the definition, refusing a run that did not complete
        const ready = async (signal: AbortSignal) => {
            const run = await applying.ended(database, signal);
            if (run.row.state !== "completed") {
                throw UniverseError.of(
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
        const universe = new Universe(definition, servers, store);
        serving.resolve(universe);

        return universe;
    }

    /** Answer a request at the universe's origin: the package mounted at its path. */
    fetch(request: Request): Promise<Response> {
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
