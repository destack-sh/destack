import { cp, mkdir, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Authentication, Caller, principal } from "@destack/access";
import { claim, ResidencyCode } from "@destack/account/object";
import type { AccountFixture } from "@destack/account/test";
import { LocalBucket } from "@destack/bucket/bun";
import { PackageClient, PackageStore } from "@destack/build/store";
import { type Select, Snapshot } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import type { EventStore } from "@destack/event";
import { ClaimLookup, claimTable } from "@destack/object";
import { defineServer, ObjectStore, RemoteStore } from "@destack/object/server";
import { testEvents } from "@destack/object/test";
import { Package, PackageName, type PackageId } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import { ResourceContext } from "@destack/resource/context";
import { createClient } from "@destack/runtime/client";
import type { Controller } from "@destack/runtime/control";
import { Health } from "@destack/runtime/health";
import { RequestId } from "@destack/runtime/request";
import { CallError } from "@destack/runtime/error";
import { Server } from "@destack/runtime/server";
import { reconciliation, testCallKey } from "@destack/runtime/test";
import { Clock } from "@destack/environment";
import { type Identifier, schema } from "@destack/schema";
import { ObjectReference, Scope, Subject, Uplink } from "@destack/sync";
import { LocalGitStorage } from "../local/index.ts";
import { packageObject, type Release } from "../object/index.ts";
import forgePackage from "../package.ts";
import { Forge } from "../server/index.ts";
import {
    bearHome,
    homeDatabase,
    homeReplicas,
    provisionVault,
    readHome,
    RESIDENCY,
    type ResidencyHome,
} from "./home.ts";

/** The file of the forge's database in its directory. */
const DATABASE_FILE = "forge.db";

/** How long a fixture bearer token lasts: a minute, longer than any call of a test. */
const TOKEN_MILLISECONDS = 60_000;

/** How long a server drains its requests on close: a second, above any test call's length. */
const DRAIN_MILLISECONDS = 1000;

/** How long a sweep runs at most: five seconds, above a sweep of a test's few builds. */
const SWEEP_MILLISECONDS = 5000;

/** What a forge fixture keeps across a close: its directory and the installation it serves as. */
export const ForgeState = schema.object({
    /** The directory with the database, the bucket and the repositories. */
    directory: schema.string(),
    /** The forge's installation in the residency's home. */
    installation: Subject,
    /** The residency whose home the forge is installed in. */
    residency: ResidencyCode,
    /** The residency's home the forge is installed in. */
    space: schema.identifier("space"),
    /** The credential vault the forge consumes in its home. */
    vault: ObjectReference,
    /** The account the fixture publishes packages under. */
    account: schema.identifier("account"),
    /** The user the fixture publishes as. */
    user: schema.identifier("user"),
});
/** What a forge fixture keeps across a close. */
export type ForgeState = schema.Infer<typeof ForgeState>;

/** A package the forge published: its build and release. */
export interface Published {
    /** The build the release was published from. */
    readonly reader: BuildReader;
    /** The package's identity. */
    readonly packageId: PackageId;
    /** The digest of the release's build manifest. */
    readonly manifest: string;
    /** The release as its creation returned it. */
    readonly release: Release;
}

/** The forge package installed in a residency's home of a universe, served in memory over HTTP. */
export class ForgeFixture implements AsyncDisposable {
    /** What the fixture keeps across a close. */
    readonly state: ForgeState;
    /** The forge's database with packages, releases and tags. */
    readonly storage: TestDatabase;
    /** The universe's account package, which the forge follows and calls. */
    readonly accounts: AccountFixture;
    /** The bucket with immutable package files and archives. */
    readonly bucket: LocalBucket;
    /** The store of builds and npm archives the forge serves. */
    readonly store: PackageStore;
    /** The directory's names the forge resolves packages through. */
    readonly claims: ClaimLookup;
    /** The residency home's store the forge extends. */
    readonly objects: ObjectStore;
    /** The forge's origin, with no trailing slash. */
    readonly origin: string;
    /** The residency's home the forge is installed in. */
    readonly space: Identifier<"space">;
    /** Aborted with the controllers' first failure, such as a refused replica, failing settles. */
    readonly #failure = new AbortController();
    /** The packages published so far. */
    readonly #created = new Set<string>();
    /** The controller sweeping the store, which tests run by hand. */
    readonly #sweep: Controller;
    /** The time the forge's clock reads during a sweep, absent while it reads the system's. */
    #sweptAt: number | undefined;
    /** The server serving the forge. */
    readonly #server: Server;
    /** The HTTP server passing requests to the server. */
    readonly #http: Bun.Server<undefined>;

    /** Serve the forge over its opened database and bucket. */
    private constructor(
        state: ForgeState,
        storage: TestDatabase,
        accounts: AccountFixture,
        bucket: LocalBucket,
        home: ResidencyHome,
        events: EventStore,
    ) {
        // keep the state, the databases and the bucket
        this.state = state;
        this.space = state.space;
        this.storage = storage;
        this.accounts = accounts;
        this.bucket = bucket;

        // listen before serving, for the npm endpoints to know their URL
        this.#http = Bun.serve({
            hostname: "127.0.0.1",
            port: 0,
            fetch: (request) => this.#server.fetch(request),
        });
        this.origin = `http://127.0.0.1:${this.#http.port}`;

        // extend the residency home's store with the forge, with no repository connected through an account's connection
        const { installation } = state;
        const client = accounts.installation(installation);
        const uplink = Uplink.of(() => client.store);
        this.store = new PackageStore(bucket);
        this.claims = ClaimLookup.of(Snapshot.live(accounts.database).read(claimTable));
        const forge = new Forge({
            home: home.id,
            store: this.store,
            endpoint: new URL(this.origin),
            storage: new LocalGitStorage(join(state.directory, "repository")),
            vault: state.vault,
            consumer: installation,
        });

        // follow and call the account package as the installation, beside the home's own replicas
        const { follows, ...placed } = homeReplicas(home, state.residency, uplink);
        const definition = forge.definition();
        this.objects = new ObjectStore({
            servers: [
                defineServer(forgePackage, {
                    ...definition,
                    follows: [...(definition.follows ?? []), ...follows],
                }),
            ],
            clock: { ...Clock.system, now: () => this.#sweptAt ?? Date.now() },
            principal: installation,
            database: storage.database,
            callKey: testCallKey,
            events,
            origin: { package: Package.of(forgePackage) },
            claims: {
                object: claim,
                home: RemoteStore.of(client.store, Scope.universe.id),
            },
            uplink,
            ...placed,
        });

        // run every controller but the sweep, which tests run at their time
        const implementation = this.objects.package(forgePackage);
        const controllers = implementation.controllers ?? [];
        const sweep = controllers.find((controller) => controller.name === "forge-sweep");
        if (sweep === undefined) {
            throw new TypeError("the forge runs no sweep");
        }
        this.#sweep = sweep;

        // serve the forge to the user of a bearer token, and to anonymous callers
        this.#server = Server.start({
            clock: this.objects.clock,
            ...implementation,
            controllers: controllers.filter((controller) => controller !== sweep),
            resources: new ResourceContext(),
            health: new Health("forge"),
            authenticate: async (request) => ForgeFixture.#authenticate(request),
            authorizeMachine: async () => {},
            drainTimeout: DRAIN_MILLISECONDS,
            report: (error) => this.#failure.abort(error),
        });
    }

    /** Install the forge in a residency's home, publishing as a user under an account. */
    static async create(
        accounts: AccountFixture,
        options: {
            /** The user the fixture publishes as. */
            readonly user: Identifier<"user">;
            /** The account the fixture publishes packages under, the home's own by default. */
            readonly account?: Identifier<"account">;
            /** The residency whose home the forge is installed in, eu by default. */
            readonly residency?: ResidencyCode;
            /** The directory the forge keeps its files in, a fresh one by default. */
            readonly directory?: string;
        },
    ): Promise<ForgeFixture> {
        // install the forge in the residency's home, provisioning its vault
        const directory = options.directory ?? (await mkdtemp(join(tmpdir(), "destack-forge-")));
        await mkdir(directory, { recursive: true });
        const storage = await TestDatabase.at(join(directory, DATABASE_FILE), homeDatabase);
        const residency = options.residency ?? RESIDENCY;
        const installation = await accounts.install([], residency);
        const home = await readHome(accounts, installation.scope);
        const vault = await provisionVault(storage.database, home.id, installation);
        const state: ForgeState = {
            directory,
            installation,
            residency,
            space: home.id,
            vault,
            account: options.account ?? schema.identifier("account").parse(home.scope),
            user: options.user,
        };
        const forge = await ForgeFixture.#serve(state, storage, accounts, home);

        // replicate the home's account chain, then give birth to the home below it, as its machine does
        await forge.settle();
        await bearHome(forge.objects, home);
        await forge.settle();

        return forge;
    }

    /** Serve a copy of a closed forge's state in a fresh directory. */
    static async open(accounts: AccountFixture, state: ForgeState): Promise<ForgeFixture> {
        // copy the state's directory
        const directory = await mkdtemp(join(tmpdir(), "destack-forge-"));
        await cp(state.directory, directory, { recursive: true });
        const copied = { ...state, directory };

        // serve it over the copied database
        const storage = await TestDatabase.at(join(directory, DATABASE_FILE), homeDatabase);
        const home = await readHome(accounts, state.space);
        const forge = await ForgeFixture.#serve(copied, storage, accounts, home);
        await forge.settle();

        return forge;
    }

    /** Serve the forge over a state's database and bucket, with the events of its database. */
    static async #serve(
        state: ForgeState,
        storage: TestDatabase,
        accounts: AccountFixture,
        home: ResidencyHome,
    ): Promise<ForgeFixture> {
        const bucket = await LocalBucket.open(join(state.directory, "bucket"), "space-test");
        const events = await testEvents(storage.database);

        return new ForgeFixture(state, storage, accounts, bucket, home, events);
    }

    /** Connect to the forge as a user, or anonymously. */
    client(user?: Identifier<"user">) {
        return createClient(forgePackage, {
            url: this.origin,
            headers: user === undefined ? {} : { authorization: `Bearer ${user}` },
        });
    }

    /** Connect to the forge as a machine's builds and the account package read its releases. */
    get registry(): {
        readonly url: string;
        readonly space: string;
        readonly fetch: (request: Request) => Promise<Response>;
    } {
        return { url: this.origin, space: this.space, fetch: (request) => fetch(request) };
    }

    /** Publish a build as the fixture's user under its account, creating the package once. */
    async publish(
        reader: BuildReader,
        visibility: Select<typeof packageObject.table>["visibility"] = "private",
    ): Promise<Published> {
        // create the package under the build's identity and name once
        const { account, user } = this.state;
        const home = this.space;
        const publisher = this.client(user);
        const packageId = reader.manifest.package.id;
        const { local: name } = PackageName.split(reader.manifest.package.name);
        if (!this.#created.has(packageId)) {
            await publisher.package.create({
                spaceId: home,
                accountId: account,
                id: packageId,
                requestId: RequestId.create(Date.now()),
                name,
                visibility,
            });
            this.#created.add(packageId);
        }

        // push the build and release it
        const manifest = await this.push(reader);
        const release = await publisher.release.create({
            spaceId: home,
            requestId: RequestId.create(Date.now()),
            parentId: packageId,
            manifest,
        });

        return { reader, packageId, manifest, release };
    }

    /** Push a build as the fixture's user, returning its manifest's digest. */
    push(reader: BuildReader): Promise<string> {
        return this.builds(this.state.user, reader.manifest.package.id).push(reader);
    }

    /** Read the build of a package's release, by version or tag, as an anonymous reader. */
    async read(packageId: PackageId, release: string): Promise<BuildReader> {
        const resolved = await this.client().release.resolve({
            spaceId: this.space,
            parentId: packageId,
            release,
        });

        return this.store.contents(resolved.manifest);
    }

    /** Connect to the forge's uploads below a package as a user. */
    builds(user: Identifier<"user">, packageId: string): PackageClient {
        return new PackageClient(
            `${this.origin}${forgePackage.adapters.builds.path}/${packageId}/`,
            (request) => {
                const headers = new Headers(request.headers);
                headers.set("authorization", `Bearer ${user}`);

                return fetch(new Request(request, { headers }));
            },
        );
    }

    /** Write the npm configuration selecting this registry for a scope with the fixture's user. */
    npmrc(scope: string): string {
        const url = new URL(`${this.origin}${forgePackage.adapters.npm.path}/`);

        return `${scope}:registry=${url.href}\n//${url.host}${url.pathname}:_authToken=${this.state.user}\n`;
    }

    /** Sweep the forge's store through its controller at a time, in UTC epoch milliseconds. */
    async sweep(now: number): Promise<void> {
        // read the time on the forge's clock for the sweep alone
        this.#sweptAt = now;
        try {
            await this.#sweep.reconcile(
                "store",
                reconciliation(AbortSignal.timeout(SWEEP_MILLISECONDS)),
            );
        } finally {
            this.#sweptAt = undefined;
        }
    }

    /** Wait until the forge's replicas reflect the account package as of now. */
    async settle(): Promise<void> {
        await this.accounts.settle(this.objects, this.#failure.signal);
    }

    /** Stop serving, keeping the database and files in the state's directory. */
    async close(): Promise<ForgeState> {
        // stop serving, then write the database's log into its file for a copy
        await this.#http.stop(true);
        await this.#server.close();
        await this.bucket[Symbol.asyncDispose]();
        await this.storage.database.executeScript("PRAGMA wal_checkpoint(TRUNCATE)");
        await this.storage.close();

        return this.state;
    }

    /** Stop serving and remove the directory. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.close();
        await rm(this.state.directory, { recursive: true, force: true });
    }

    /** Verify a request's bearer token as the user it carries, for a minute. */
    static #authenticate(request: Request): Authentication | undefined {
        // read the user the token carries
        const token = /^Bearer (.+)$/u.exec(request.headers.get("authorization") ?? "")?.[1];
        if (token === undefined) {
            return undefined;
        }
        const user = schema.identifier("user").safeParse(token);
        if (!user.success) {
            throw new CallError("UNAUTHORIZED", { message: "the bearer token carries no user" });
        }

        // authenticate as the user
        const now = Date.now();

        return {
            caller: Caller.of(principal.user.reference(Scope.universe.id, user.data)),
            credential: {
                kind: "fixture",
                id: "fixture-1",
                audience: packageObject.policy.definition.packageId,
                verifiedAt: now,
                expiresAt: now + TOKEN_MILLISECONDS,
            },
        };
    }
}
