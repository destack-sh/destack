import { Directory } from "@destack/account/address";
import {
    AccountAuthentication,
    BetterAuthAuthenticator,
    type BetterAuthAuthenticatorOptions,
} from "@destack/account/better-auth";
import { IdentityKeystore, IdentityRegistry } from "@destack/account/identity";
import { Account, claim, Residency } from "@destack/account/object";
import accountPackage, { OAUTH_PATH } from "@destack/account/package";
import { accountServer, DnsOverHttps, Operator, UniverseRole } from "@destack/account/server";
import { accountDatabase } from "@destack/account/stack";
import type { Bucket } from "@destack/bucket";
import { type DatabaseConnection, defineDatabase, Snapshot } from "@destack/db";
import { DatabasePersonalKeyring, EventStore } from "@destack/event";
import { Registry } from "@destack/forge/client";
import forgePackage from "@destack/forge/package";
import { type Keyring } from "@destack/identity";
import type { MailTransport } from "@destack/message/email";
import { call } from "@destack/object";
import objectServer, { defineServer, ObjectStore } from "@destack/object/server";
import { Package, type PackageId } from "@destack/package";
import relayPackage from "@destack/relay/package";
import { RELAY_TOKEN_LIFETIME_MILLISECONDS } from "@destack/relay/server";
import { ResourceContext } from "@destack/resource/context";
import { Digest, type Identifier, present, schema } from "@destack/schema";
import { type Fetch, PackageMount } from "@destack/runtime";
import type { Alarm } from "@destack/runtime/control";
import { Health } from "@destack/runtime/health";
import { type CallKey, RequestId } from "@destack/runtime/request";
import { Server } from "@destack/runtime/server";
import { Setting } from "@destack/setting";
import { settingServer } from "@destack/setting/server";
import { Scope } from "@destack/sync";
import objectPackage from "@destack/object/package";
import universePackage from "../package.ts";
import { Run } from "@destack/workflow";
import { apply } from "./apply.ts";
import { UniverseDefinition } from "./definition.ts";
import type { Environment } from "@destack/environment";

/** The universe's database: the account database and the runs applying its definition. */
export const universeDatabase = defineDatabase({
    name: "account",
    tables: [
        ...accountDatabase.tables,
        ...Object.values(apply.objects).flatMap((type) => type.tables),
    ],
});

/** The longest drain on shutdown, in milliseconds. */
const DRAIN_MILLISECONDS = 5000;

/** The access token lifetimes longer than the default minute: the relay's tunnel renewals. */
const LIFETIMES: ReadonlyMap<PackageId, number> = new Map([
    [relayPackage.id, RELAY_TOKEN_LIFETIME_MILLISECONDS],
]);

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
}

/** A running universe: the account and setting packages, each answering below its mount. */
export class Universe implements AsyncDisposable {
    /** What the universe declares. */
    readonly definition: UniverseDefinition;
    /** The server of each package, by its identifier. */
    readonly servers: ReadonlyMap<string, Server>;

    /** Create a started universe. */
    private constructor(definition: UniverseDefinition, servers: ReadonlyMap<string, Server>) {
        this.definition = definition;
        this.servers = servers;
    }

    /** Start the account and setting packages over the universe's database. */
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
            personal: new DatabasePersonalKeyring(database, keyring),
            clock,
        });

        // call residency homes as the universe once its identity exists, its own requests on loopback
        let universe: Universe | undefined;
        const connected = Promise.withResolvers<(audience: PackageId) => Fetch>();
        const connect =
            (audience: PackageId): Fetch =>
            async (request) =>
                (await connected.promise)(audience)(request);
        const directory = new Directory(Snapshot.live(database));
        const home = (account: Identifier<"account">) =>
            Account.locate(directory.snapshot, account);
        const registry = Registry.of(directory, home, connect(forgePackage.id), clock);

        // apply the residencies, then each step of the platform's definition, once per run
        const applied = apply.implement(async (step) => {
            const steps = [
                {
                    name: "residencies",
                    apply: (objects: ObjectStore, at: number) =>
                        Residency.apply(objects, definition.residencies, at),
                },
                ...new Operator(definition.platform, definition.stacks.residency).steps(),
            ];
            for (const each of steps) {
                await step.run(each.name, schema.null(), async () => {
                    await each.apply(store, clock.now());

                    return null;
                });
            }
        });

        // serve the account, setting and universe packages over one store, signing in once it exists
        const signIn = Promise.withResolvers<BetterAuthAuthenticator>();
        const store: ObjectStore = new ObjectStore({
            servers: [
                objectServer,
                accountServer({
                    resolver: new DnsOverHttps(),
                    stacks: definition.stacks,
                    signIn: signIn.promise,
                }),
                settingServer({
                    declared: Setting.declared((value) =>
                        registry.open(value.packageId, value.release),
                    ),
                    after: [],
                }),
                defineServer(universePackage, {
                    objects: applied.objects,
                    controllers: applied.controllers,
                }),
            ],
            // follow no chain at the universe, the root of every chain
            chain: async () => undefined,
            principal: Scope.universe,
            partition: Scope.universe.id,
            policies: Object.values(forgePackage.objects).map((object) => object.policy),
            database,
            claims: { object: claim },
            clock,
            callKey,
            events,
            origin: { package: Package.of(accountPackage) },
            // follow the residencies' homes where their machines answer
            publisher: (scope) =>
                directory.publisher(scope, connect(objectPackage.id), () => clock.now()),
        });

        // trust the universe as its own owner, and start applying its definition unless a run applies it
        await UniverseRole.trust(store, now);
        const digest = await Digest.json(definition);
        await store.mutate(store.serving(apply.run), {
            id: RequestId.create(now),
            calls: [apply.start({ digest }, { key: digest, actor: Scope.universe })],
        });
        const [started] = await Snapshot.live(database)
            .read(apply.run.table)
            .findMany({ where: { key: digest }, orderBy: { createdAt: "desc" }, limit: 1 });
        const applying = new Run(apply, present(started, `the run applying ${digest}`));

        // sign in with the universe's identity
        const keystore = await IdentityKeystore.universe(
            keyring,
            database,
            IdentityRegistry.of(store),
            clock,
        );

        // key the universe's sensitive columns with its identity's column key
        await keystore.key(database, Scope.universe.id);
        const issuer = `${PackageMount.url(origin, accountPackage.id)}${OAUTH_PATH}`;
        signIn.resolve(
            await BetterAuthAuthenticator.create({
                keystore,
                clock,
                callKey,
                issuer,
                trustedOrigins: [origin],
                ipAddress: options.ipAddress,
                database,
                providers: {},
                objects: store,
                events,
                lifetimes: LIFETIMES,
                mail: options.mail,
            }),
        );

        // call packages in spaces as the universe, with tokens its identity signs
        const source = keystore.source(database, Scope.universe, {
            issuer,
            send: (request) =>
                request.url.startsWith(origin)
                    ? present(universe, "the universe").fetch(request)
                    : network.fetch(request),
        });
        connected.resolve((audience) => source.fetchSigned(audience));

        // serve once the run applied the definition, refusing a run that did not complete
        const ready = async (signal: AbortSignal) => {
            const run = await applying.ended(database, signal);
            if (run.state !== "completed") {
                throw new Error(`the run applying the definition ${digest} is ${run.state}`, {
                    cause: run.failure,
                });
            }
        };

        // answer each package to bearers of the universe's access tokens, the first running the controllers
        const servers = new Map(
            store.packages.map((implementation, index) => [
                implementation.package.id,
                Server.start({
                    ...implementation,
                    clock,
                    health: new Health(implementation.package.name),
                    resources: new ResourceContext(),
                    authenticate: (request) =>
                        AccountAuthentication.bearer(request, {
                            database,
                            issuer,
                            audience: implementation.package.id,
                            now: clock.now(),
                        }),
                    authorizeMachine: async () => {},
                    drainTimeout: DRAIN_MILLISECONDS,
                    ready,
                    ...(options.alarm === undefined || index > 0 ? {} : { alarm: options.alarm }),
                    report: (error) => options.report(error),
                }),
            ]),
        );
        universe = new Universe(definition, servers);

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
