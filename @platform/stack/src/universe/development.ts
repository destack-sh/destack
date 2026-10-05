import { join } from "node:path";
import { type Authenticator, createAuthenticator } from "@destack/account/better-auth";
import {
    connect,
    DirectoryClient,
    WorkloadIdentity,
    workloadIdentity,
} from "@destack/account/client";
import { Host } from "@destack/account/object";
import { accountService } from "@destack/account/service";
import { accountConfiguration, accountWorkload } from "@destack/account/workload";
import { AuditHistory } from "@destack/audit/history";
import { LocalBucket } from "@destack/bucket/local";
import { PackageStore } from "@destack/build/store";
import { type DatabaseConnection, sql } from "@destack/db";
import * as postgresql from "@destack/db/postgres";
import type { PackageId } from "@destack/package";
import { LocalGitStorage } from "@destack/forge/local";
import { forgeConfiguration } from "@destack/forge/workload";
import { BunRelay } from "@destack/relay/bun";
import { RELAY_PACKAGE } from "@destack/relay/server";
import { RELAY_ROLE, relayConfiguration } from "@destack/relay/workload";
import { ResourceContext } from "@destack/resource/context";
import { type Service, ServiceMount } from "@destack/service";
import type { Authentication } from "@destack/service/authentication";
import { CallKey } from "@destack/service/request";
import { type Workload, WorkloadInstance } from "@destack/service/workload";
import { Scope } from "@destack/sync";
import { serve, type Server } from "bun";
import { PlatformAuthentication } from "../authentication/index.ts";
import { type Platform, PLATFORM, Placement, PlatformAccount } from "../placement/index.ts";
import type { SignInMail } from "../mail/index.ts";
import { Operator, type RegionDefinition } from "./operator.ts";
import { RESIDENCIES } from "./residency.ts";

/** The longest drain on shutdown, in milliseconds. */
const DRAIN_MILLISECONDS = 5000;

/** Where and with what the development universe runs. */
export interface DevelopmentUniverseOptions {
    /** The PostgreSQL server's URL, without a database. */
    readonly server: string;
    /** The database keeping each service's schema. */
    readonly database: string;
    /** The origin answering for the account service and the region's services, the universe's issuer. */
    readonly origin: string;
    /** Where the relay listens, and the origin hosts open their tunnels at. */
    readonly relay: {
        /** The relay's own origin. */
        readonly origin: string;
        /** The network interface. */
        readonly hostname: string;
        /** The port. */
        readonly port: number;
    };
    /** The code of the region the universe serves at its own origin, one of its residencies' regions. */
    readonly region: string;
    /** The directory with the services' files: the forge's builds and platform repositories. */
    readonly directory: string;
    /** The secret protecting the universe's cookies and tokens. */
    readonly secret: string;
    /** The operator's email, invited as the platform organisation's owner and signed in through the links the universe mails itself. */
    readonly operator: string;
    /** How sign-in links and codes reach people. */
    readonly mail: SignInMail;
    /** Report a failure no request answers. */
    report(error: unknown): void;
}

/** The sign-in, account database and journal key every workload of the universe runs with. */
interface Shared {
    /** The universe's sign-in. */
    readonly signIn: Authenticator;
    /** The account service's database, with the hosts' keys and the directory. */
    readonly database: DatabaseConnection;
    /** The key the journals fingerprint sensitive inputs under. */
    readonly callKey: CallKey;
}

/** The development universe, which runs every platform workload in one process over its own PostgreSQL schema. */
export class DevelopmentUniverse implements AsyncDisposable {
    /** The URL the universe answers at. */
    readonly url: URL;
    /** The running workloads, in start order. */
    readonly #instances: WorkloadInstance[] = [];
    /** The service each mounted package serves, with its workload. */
    readonly #mounts = new Map<
        string,
        { readonly instance: WorkloadInstance; readonly service: Service }
    >();
    /** The verification of the workloads' callers. */
    readonly #authentication: PlatformAuthentication;
    /** The databases opened, closed after the workloads. */
    readonly #databases: postgresql.PostgresDatabase[] = [];
    /** The buckets opened, closed after the workloads. */
    readonly #buckets: LocalBucket[] = [];
    /** The sign-in links the universe waits for, by the address they are mailed to. */
    readonly #inbox = new Map<string, PromiseWithResolvers<string>>();
    /** The listener. */
    readonly #server: Server<undefined>;

    /** Listen at the universe's origin, answering mounts as their workloads start. */
    private constructor(origin: string) {
        // listen, and verify callers against the universe's own key set
        const url = new URL(origin);
        this.#server = serve({
            hostname: url.hostname,
            port: Number(url.port),
            fetch: (request) => this.fetch(request),
        });
        this.url = this.#server.url;
        this.#authentication = new PlatformAuthentication(this.url.origin, (request) =>
            this.fetch(request),
        );
    }

    /** Start the development universe with its databases, account service, operator rows and region workloads. */
    static async start(options: DevelopmentUniverseOptions): Promise<DevelopmentUniverse> {
        const universe = new DevelopmentUniverse(options.origin);
        try {
            await universe.#start(options);
        } catch (error) {
            // close what started, keeping the start's failure beside a failure to close
            try {
                await universe.close();
            } catch (closing) {
                throw new SuppressedError(closing, error, "the universe failed to start and close");
            }
            throw error;
        }

        return universe;
    }

    /** The URL the universe's account service answers at. */
    get accounts(): string {
        return ServiceMount.url(this.url.origin, accountService.package.id);
    }

    /** Hand a request to the service its mount names, or to the account service at the issuer's paths. */
    async fetch(request: Request): Promise<Response> {
        // find the mounted service, the account service at the issuer's paths
        const routed = ServiceMount.route(request);
        const mount = this.#mounts.get(routed?.packageId ?? accountService.package.id);

        // refuse a mount of no running service, and answer unavailable while the account service starts
        if (mount === undefined) {
            return new Response(null, { status: routed === undefined ? 503 : 404 });
        }

        return mount.instance.fetch(mount.service, routed?.request ?? request);
    }

    /** Stop every workload in reverse start order and close what they ran over. */
    async close(): Promise<void> {
        // stop the workloads and release what they ran over
        for (const instance of this.#instances.toReversed()) {
            await instance.close();
        }
        for (const database of this.#databases) {
            await database.close();
        }
        for (const bucket of this.#buckets) {
            await bucket[Symbol.asyncDispose]();
        }
        await this.#server.stop(true);
    }

    /** Stop the universe when its owner's scope ends. */
    [Symbol.asyncDispose](): Promise<void> {
        return this.close();
    }

    /** Start the databases, account service, operator rows and region workloads. */
    async #start(options: DevelopmentUniverseOptions): Promise<void> {
        // place every platform workload in this one process, keyed by the universe's secret
        const [placed] = Placement.single("universe", PLATFORM).processes;
        const [account, ...placedInRegion] = placed?.packages ?? [];
        if (account?.workload !== accountWorkload) {
            throw new TypeError("the universe places the account service first");
        }
        const key = CallKey.import(
            new Uint8Array(
                await crypto.subtle.digest(
                    "SHA-256",
                    new TextEncoder().encode(`journal:${options.secret}`),
                ),
            ),
        );
        const callKey: CallKey = () => key;

        // sign in through the account service over its own schema, mailing links and codes
        await DevelopmentUniverse.#create(options);
        const database = await this.#open(options, account);
        const origin = this.url.origin;
        const signIn = createAuthenticator({
            callKey,
            origin,
            trustedOrigins: [origin],
            ipAddress: { disableIpTracking: true },
            secret: options.secret,
            database,
            providers: {},
            ...PlatformAccount.pages(origin),
            service: (request) => this.fetch(request),
            sendMagicLink: (message) => this.#deliver(message, options.mail),
            sendCode: options.mail.sendCode,
        });

        // start the account service with the platform organisation and the placed packages' permissions
        const shared: Shared = { signIn, database, callKey };
        await this.#run(
            accountWorkload,
            new ResourceContext().bind(
                accountConfiguration,
                PlatformAccount.configuration(signIn, options.operator),
            ),
            shared,
            options,
            (request, audience) =>
                this.#authentication.account(request, audience, signIn, database),
            new AuditHistory(database),
        );

        // sign the operator in and enroll the served region's host
        const service = { url: this.accounts, fetch: (request: Request) => this.fetch(request) };
        const served = DevelopmentUniverse.#served(options.region);
        const operator = await Operator.open(
            { ...service, origin, link: (email) => this.#link(email) },
            options.operator,
        );
        await DevelopmentUniverse.#bootstrap(operator);
        const regionId = await operator.region(served);
        const region = await operator.host(regionId);

        // publish the region's origin as its host, the way a cell publishes its endpoint
        const hostClient = connect({
            url: this.accounts,
            fetch: region.fetch(service.fetch, accountService.package.id, this.accounts),
        });
        await new DirectoryClient(hostClient).publish(regionId, Scope.universe.id, origin);

        // start the region's workloads, each as its placement through the region's host
        for (const entry of placedInRegion) {
            const { workload } = entry;
            const placementId = await operator.place(workload.package.id, regionId);
            const identity = new WorkloadIdentity(
                placementId,
                connect({
                    url: this.accounts,
                    fetch: region.fetch(service.fetch, accountService.package.id, this.accounts, {
                        placementId,
                    }),
                }),
            );
            const connection = await this.#open(options, entry);
            const resources = await this.#bind(workload, operator, identity, options);
            await this.#run(
                workload,
                resources.bind(entry.database, connection).bind(workloadIdentity, identity),
                shared,
                options,
                (request, audience) =>
                    this.#authentication.workload(request, audience, identity, connection),
            );
        }
    }

    /** Find the served region among the residencies' regions, refusing an unknown code. */
    static #served(code: string): RegionDefinition {
        // find the region with its residency
        const found = RESIDENCIES.flatMap((residency) =>
            residency.regions.map((region) => ({ ...region, residencyId: residency.code })),
        ).find((region) => region.code === code);
        if (found === undefined) {
            throw new TypeError(`the universe keeps no region ${code}`);
        }

        return found;
    }

    /** Keep every residency's regions once, creating only what is missing. */
    static async #bootstrap(operator: Operator): Promise<void> {
        for (const residency of RESIDENCIES) {
            for (const region of residency.regions) {
                await operator.region({ ...region, residencyId: residency.code });
            }
        }
    }

    /** Bind a residency workload's configuration, refusing a workload the universe does not know. */
    async #bind(
        workload: Workload,
        operator: Operator,
        identity: WorkloadIdentity,
        options: DevelopmentUniverseOptions,
    ): Promise<ResourceContext> {
        // keep the forge's builds in a local bucket and platform repositories in a local directory
        if (workload.name === "forge") {
            const bucket = await LocalBucket.open(join(options.directory, "builds"), "forge");
            this.#buckets.push(bucket);

            return new ResourceContext().bind(forgeConfiguration, {
                store: new PackageStore(bucket),
                endpoint: new URL(ServiceMount.url(this.url.origin, workload.package.id)),
                storage: new LocalGitStorage(join(options.directory, "repositories")),
            });
        }
        // serve the relay on its own listener, bound to the role operators give it
        else if (workload.name === "relay") {
            await operator.grant(RELAY_ROLE, identity.placementId);

            return new ResourceContext().bind(relayConfiguration, {
                origin: options.relay.origin,
                tokens: this.#authentication.verifier(RELAY_PACKAGE.id),
                serve: (relay) =>
                    BunRelay.listen(relay, {
                        hostname: options.relay.hostname,
                        port: options.relay.port,
                    }),
            });
        }

        throw new TypeError(`the universe binds no workload ${workload.name}`);
    }

    /** Start a workload over its resources and mount the services it serves. */
    async #run(
        workload: Workload,
        resources: ResourceContext,
        shared: Shared,
        options: DevelopmentUniverseOptions,
        authenticate: (request: Request, audience: PackageId) => Promise<Authentication | null>,
        history?: AuditHistory,
    ): Promise<void> {
        // start the workload
        const instance = await WorkloadInstance.start(workload, {
            report: (error) => options.report(error),
            resources,
            callKey: shared.callKey,
            ...(history === undefined ? {} : { history }),
            service: (service) => ({
                audience: service.package.id,
                authenticate: (request) => authenticate(request, service.package.id),
                authorizeHost: (call) => Host.authorize(call),
                drainTimeout: DRAIN_MILLISECONDS,
                instance: "universe",
            }),
        });
        this.#instances.push(instance);

        // mount each service it serves
        for (const service of instance.services) {
            this.#mounts.set(service.package.id, { instance, service });
        }
    }

    /** Hand a sign-in link to the universe when it waits for its address, mailing it otherwise. */
    async #deliver(message: { email: string; url: string }, mail: SignInMail): Promise<void> {
        const waiting = this.#inbox.get(message.email);
        if (waiting === undefined) {
            await mail.sendMagicLink(message);
        } else {
            this.#inbox.delete(message.email);
            waiting.resolve(message.url);
        }
    }

    /** Wait for the next sign-in link mailed to an address, keeping it from the mail. */
    #link(email: string): Promise<string> {
        const waiting = Promise.withResolvers<string>();
        this.#inbox.set(email, waiting);

        return waiting.promise;
    }

    /** Open a platform package's database in a schema named after its workload, migrated to its declaration. */
    async #open(
        options: DevelopmentUniverseOptions,
        platform: Platform,
    ): Promise<DatabaseConnection> {
        // create and migrate the schema once
        const { database: declaration, workload } = platform;
        const schema = workload.name;
        const url = `${options.server}/${options.database}?search_path=${schema}`;
        const database = await postgresql.connect(url, declaration);
        this.#databases.push(database);
        await database.execute(sql.raw(`CREATE SCHEMA IF NOT EXISTS "${schema}"`));
        await database.migrate(declaration.tables);

        return database;
    }

    /** Create the universe's database once. */
    static async #create(options: DevelopmentUniverseOptions): Promise<void> {
        const server = await postgresql.connect(`${options.server}/postgres`);
        try {
            const [existing] = await server.execute(
                sql`SELECT 1 AS found FROM pg_database WHERE datname = ${options.database}`,
            );
            if (existing === undefined) {
                await server.execute(sql.raw(`CREATE DATABASE "${options.database}"`));
            }
        } finally {
            await server.close();
        }
    }
}
