import { AccountFixture, ids } from "@destack/account/test";
import { cp, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import {
    type BuildOptions,
    PackageBuild,
    Compiler,
    readDependencies,
    readOutputs,
} from "@destack/build";
import { PackageClient, PackageStore } from "@destack/build/store";
import { Authentication, Caller, principal } from "@destack/access";
import { ClaimLookup, claimTable } from "@destack/object";
import { testEvents } from "@destack/object/test";
import type { EventStore } from "@destack/event";
import { LocalBucket } from "@destack/bucket/bun";
import { type ObjectReference, Scope, type Subject, Uplink } from "@destack/sync";
import { TestDatabase } from "@destack/db/test";
import { Package, PackageName, type PackageId } from "@destack/package";
import { type Select, Snapshot } from "@destack/db";
import { History, Vocabulary } from "@destack/resource";
import { ResourceContext } from "@destack/resource/context";
import { defineServer, ObjectStore, RemoteStore } from "@destack/object/server";
import { claim } from "@destack/account/object";
import { nth, Commit, found, type Identifier, present, schema, Version } from "@destack/schema";
import { Health } from "@destack/runtime/health";
import { RequestId } from "@destack/runtime/request";
import { Server } from "@destack/runtime/server";
import { createClient } from "@destack/runtime/client";
import { reconciliation, testCallKey } from "@destack/runtime/test";
import type { Controller } from "@destack/runtime/control";
import { LocalGitStorage } from "../../src/local/index.ts";
import { packageObject, type Release } from "../../src/object/index.ts";
import { Forge } from "../../src/server/index.ts";
import forgePackage from "../../src/package.ts";
import {
    bearHome,
    homeReplicas,
    homeDatabase,
    provisionVault,
    readHome,
    RESIDENCY,
    type ResidencyHome,
} from "./home.ts";
import { auditedCalls } from "./audit.ts";
import { Clock } from "@destack/environment";

/** The account that publishes the fixture packages and scopes their npm names. */
export const ACCOUNT_ID = schema
    .identifier("account")
    .parse("account-00000000-0000-7000-8000-000000000001");

/** The bearer token of the account's owner. */
const OWNER = "owner";

/** The users of the bearer tokens: the account's owner and a stranger to it. */
const USERS: ReadonlyMap<string, string> = new Map([
    [OWNER, ids.owner],
    ["stranger", ids.stranger],
]);

/** The version the fixture sources declare. */
export const FIXTURE_VERSION = "2026.9.0";

/** The fixture releases, each dependency before its dependents. */
export const FIXTURE_RELEASES = [
    ["answer", [FIXTURE_VERSION, "2026.9.1"]],
    ["greeting", [FIXTURE_VERSION]],
] as const satisfies readonly (readonly [FixturePackage, readonly string[]])[];

/** The dev prerelease of the answer package after its releases, built without a commit. */
export const PRERELEASE_VERSION = "2026.9.2-dev.1";

/** The commit every fixture release is built from. */
export const COMMIT = "a".repeat(40);

/** The fixture packages: real package sources under tests/fixture called `@example/<name>`. */
export type FixturePackage = "answer" | "greeting";

/** A package the forge published: its build and release. */
export interface Published {
    /** The Destack build the release was packed from. */
    readonly build: PackageBuild;
    /** The package's identity. */
    readonly packageId: PackageId;
    /** The digest of the release's build manifest. */
    readonly manifest: string;
    /** The release as its creation returned it. */
    readonly release: Release;
}

/** A forge over its database, the account package and a local bucket, served over HTTP. */
export class PackageFixture implements AsyncDisposable {
    /** The temporary directory with the bucket, sources and installations. */
    readonly directory: string;
    /** The forge's database with packages, releases and tags. */
    readonly storage: TestDatabase;
    /** The account package with the account and the directory. */
    readonly accounts: AccountFixture;
    /** The bucket with immutable package files and archives. */
    readonly bucket: LocalBucket;
    /** The store of builds and npm archives the forge serves. */
    readonly store: PackageStore;
    /** The directory's names the forge resolves packages through. */
    readonly claims: ClaimLookup;
    /** The residency home's store the forge extends. */
    readonly objects: ObjectStore;
    /** The forge's installation in the residency home. */
    readonly installation: Identifier<"installation">;
    /** The forge installation's space, below which it stores its replicas. */
    readonly space: string;
    /** The forge's origin, with no trailing slash. */
    readonly origin: string;
    /** Aborted with the controllers' first failure, such as a refused replica, failing settles. */
    readonly #failure = new AbortController();
    /** The packages created so far. */
    readonly #created = new Set<string>();
    /** The controller sweeping the store, which each test runs by hand. */
    readonly #sweep: Controller;
    /** The time the forge's clock reads during a sweep, absent while it reads the system's. */
    #sweptAt: number | undefined;
    /** The server serving the forge. */
    readonly #server: Server;
    /** The HTTP server passing requests to the server. */
    readonly #http: Bun.Server<undefined>;

    /** Serve the opened databases and bucket. */
    private constructor(
        directory: string,
        storage: TestDatabase,
        accounts: AccountFixture,
        bucket: LocalBucket,
        installation: Subject,
        home: ResidencyHome,
        events: EventStore,
        credentials: ObjectReference,
    ) {
        // store releases in the forge's database and names in the account package's directory
        this.directory = directory;
        this.storage = storage;
        this.accounts = accounts;
        this.bucket = bucket;
        this.installation = schema.identifier("installation").parse(installation.id);

        // listen before serving, for the npm endpoints to know their URL
        this.#http = Bun.serve({
            hostname: "127.0.0.1",
            port: 0,
            fetch: (request) => this.#server.fetch(request),
        });
        this.origin = `http://127.0.0.1:${this.#http.port}`;

        // extend the residency home's store with the forge, with no repository connected through an account's connection
        const client = accounts.installation(installation);
        const uplink = Uplink.of(() => client.store);
        this.store = new PackageStore(bucket);
        const universe = Snapshot.live(accounts.database);
        this.claims = ClaimLookup.of(universe.read(claimTable));
        const forge = new Forge({
            home: home.id,
            store: this.store,
            endpoint: new URL(this.origin),
            storage: new LocalGitStorage(join(directory, "repository")),
            vault: credentials,
            consumer: installation,
        });

        // follow and call the account package as the installation, beside the home's own replicas
        const { follows, ...placed } = homeReplicas(home, RESIDENCY, uplink);
        const definition = forge.definition();
        const composed = new ObjectStore({
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
        this.objects = composed;
        this.space = home.id;

        // run every controller but the sweep, which each test runs at its time
        const implementation = composed.package(forgePackage);
        const controllers = implementation.controllers ?? [];
        this.#sweep = present(
            controllers.find((controller) => controller.name === "forge-sweep"),
            "the forge's sweep",
        );

        // serve the forge to the user of a bearer token, and to anonymous callers
        this.#server = Server.start({
            clock: composed.clock,
            ...implementation,
            controllers: controllers.filter((controller) => controller !== this.#sweep),
            resources: new ResourceContext(),
            health: new Health("forge"),
            authenticate: async (request) => PackageFixture.#authenticate(request),
            authorizeMachine: async () => {},
            drainTimeout: 1000,
            report: (error) => this.#failure.abort(error),
        });
    }

    /** Open an empty forge on sqlite with the owner's account. */
    static async open(): Promise<PackageFixture> {
        // open the databases and bucket in a fresh directory
        const directory = await mkdtemp(join(tmpdir(), "destack-forge-"));
        const storage = await TestDatabase.create("sqlite", homeDatabase, {
            isMigrated: true,
        });
        const accounts = await AccountFixture.open({
            dialect: "sqlite",
            policies: [packageObject.policy],
        });
        const bucket = await LocalBucket.open(join(directory, "bucket"), "space-test");

        // create the owner's account through the account package
        await accounts.user(ids.owner).account.create({
            scope: ids.owner,
            id: ACCOUNT_ID,
            requestId: RequestId.create(Date.now()),
            handle: "example",
            name: "Example",
            kind: "shared",
        });

        // install the forge in the residency's home, following and calling the account package as the installation
        const installation = await accounts.install([], RESIDENCY);
        const home = await readHome(accounts, installation.scope);
        const fixture = new PackageFixture(
            directory,
            storage,
            accounts,
            bucket,
            installation,
            home,
            await testEvents(storage.database),
            await provisionVault(storage.database, home.id, installation),
        );

        // replicate the home's account chain, then give birth to the home below it, as its machine does
        await fixture.settle();
        await bearHome(fixture.objects, home);
        await fixture.settle();

        return fixture;
    }

    /** Read each change the call events recorded, with how it ended. */
    async audited(): Promise<[string, string][]> {
        return (await auditedCalls(this.storage.database))
            .filter((ended) => ended.execution.category === "activity")
            .map((ended) => [ended.method, ended.execution.outcome.kind]);
    }

    /** Connect to the forge as a user, or anonymously. */
    client(user?: string) {
        return createClient(forgePackage, {
            url: this.origin,
            headers: user === undefined ? {} : { authorization: `Bearer ${user}` },
        });
    }

    /** Publish a build of a fixture package as the owner, creating the package the first time. */
    async publish(
        build: PackageBuild,
        visibility: Select<typeof packageObject.table>["visibility"] = "private",
    ): Promise<Published> {
        // create the package under the build's identity and name once
        const owner = this.client(OWNER);
        const packageId = build.manifest.package.id;
        const { local: name } = PackageName.split(build.manifest.package.name);
        if (!this.#created.has(packageId)) {
            await owner.package.create({
                spaceId: this.space,
                accountId: ACCOUNT_ID,
                id: packageId,
                requestId: RequestId.create(Date.now()),
                name,
                visibility,
            });
            this.#created.add(packageId);
        }

        // push the build and release it
        const manifest = await this.push(build);
        const release = await owner.release.create({
            spaceId: this.space,
            requestId: RequestId.create(Date.now()),
            parentId: packageId,
            manifest,
        });

        return { build, packageId, manifest, release };
    }

    /** Push a build to the forge as the owner, returning its manifest's digest. */
    push(build: PackageBuild): Promise<string> {
        return this.builds(OWNER, build.reader.manifest.package.id).push(build.reader);
    }

    /** Connect to the forge's uploads below a package as a user. */
    builds(user: string, packageId: string): PackageClient {
        return new PackageClient(
            `${this.origin}${forgePackage.adapters.builds.path}/${packageId}/`,
            (request) => {
                const headers = new Headers(request.headers);
                headers.set("authorization", `Bearer ${user}`);

                return fetch(new Request(request, { headers }));
            },
        );
    }

    /** Write the npm configuration selecting this registry with the owner's token. */
    npmrc(): string {
        const url = new URL(`${this.origin}${forgePackage.adapters.npm.path}/`);

        return `@example:registry=${url.href}\n//${url.host}${url.pathname}:_authToken=${OWNER}\n`;
    }

    /** Stop serving and remove every database and file. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#http.stop(true);
        await this.#server.close();
        await this.bucket[Symbol.asyncDispose]();
        await this.storage.close();
        await this.accounts[Symbol.asyncDispose]();
        await rm(this.directory, { recursive: true });
    }

    /** Sweep the forge's store through its controller at a time, in UTC epoch milliseconds. */
    async sweep(now: number): Promise<void> {
        // read the time on the forge's clock for the sweep alone
        this.#sweptAt = now;
        try {
            await this.#sweep.reconcile("store", reconciliation(AbortSignal.timeout(5000)));
        } finally {
            this.#sweptAt = undefined;
        }
    }

    /** Wait until the forge's replicas reflect the account package as of now. */
    async settle(): Promise<void> {
        await this.accounts.settle(this.objects, this.#failure.signal);
    }

    /** Verify a request's bearer token as its user, for a minute. */
    static #authenticate(request: Request): Authentication | undefined {
        const token = /^Bearer (.+)$/u.exec(request.headers.get("authorization") ?? "")?.[1];
        if (token === undefined) {
            return undefined;
        }
        const subject = principal.user.reference("universe", found(USERS, token));

        return {
            caller: Caller.of(subject),
            credential: {
                kind: "fixture",
                id: "fixture-1",
                audience: packageObject.policy.definition.packageId,
                verifiedAt: Date.now(),
                expiresAt: Date.now() + 60_000,
            },
        };
    }
}

/** A copy of a fixture package's source with a started builder. */
export class FixtureSource implements AsyncDisposable {
    /** The fixture package. */
    readonly name: FixturePackage;
    /** The directory with the replica. */
    readonly directory: string;
    /** The builder retained across the package's versions. */
    readonly #builder: Compiler;
    /** The outputs and dependencies the last build compiled, and what it published. */
    #compiled: (Pick<BuildOptions, "outputs" | "dependencies"> & { history: History }) | undefined;

    /** Take a replica and its started builder. */
    private constructor(name: FixturePackage, directory: string, builder: Compiler) {
        this.name = name;
        this.directory = directory;
        this.#builder = builder;
    }

    /** Replicate a fixture package's source into a directory and start its builder. */
    static async open(name: FixturePackage, parent: string): Promise<FixtureSource> {
        const directory = join(parent, name);
        await cp(fileURLToPath(new URL(`./${name}/`, import.meta.url)), directory, {
            recursive: true,
        });

        return new FixtureSource(name, directory, await Compiler.start(directory));
    }

    /** Install the dependencies and build each version's published outputs. */
    async build(npmrc: string, versions: readonly string[]): Promise<PackageBuild[]> {
        // install the dependencies
        await writeFile(join(this.directory, ".npmrc"), npmrc);
        await run(
            [
                process.execPath,
                "install",
                "--cache-dir",
                join(this.directory, "..", "cache", this.name),
            ],
            this.directory,
        );
        const declaration = schema
            .jsonObject()
            .parse(await readJson(join(this.directory, "package.json")));
        const dependencies = await readDependencies(this.directory);

        // build one unbundled module per runtime the package ships, as a build worker does
        const outputs = Object.fromEntries(
            Object.entries(await readOutputs(this.directory)).map(([name, { kind, runtime }]) => [
                name,
                { kind, runtime },
            ]),
        );

        // build each version with the retained builder, planned from the version before
        const builds: PackageBuild[] = [];
        let vocabulary: Vocabulary = {};
        for (const version of versions) {
            await writeFile(
                join(this.directory, "package.json"),
                JSON.stringify({ ...declaration, version }),
            );
            const latest = builds.at(-1);
            const history =
                latest === undefined ? undefined : await History.read(latest.reader, vocabulary);
            const build = await this.#builder.build({
                outputs,
                dependencies,
                commit: Commit.parse(COMMIT),
                ...(history === undefined ? {} : { history }),
            });
            vocabulary = Vocabulary.advance(vocabulary, await build.reader.declarations(), version);
            builds.push(build);
        }

        const latest = nth(builds, -1);
        this.#compiled = {
            outputs,
            dependencies,
            history: await History.read(latest.reader, vocabulary),
        };

        return builds;
    }

    /** Build a working tree without a commit: the last version, or a prerelease of the next. */
    async uncommitted(prerelease?: Version): Promise<PackageBuild> {
        if (this.#compiled === undefined) {
            throw new TypeError(`fixture ${this.name} has not been built`);
        }

        // build the last version again
        const { outputs, dependencies, history } = this.#compiled;
        if (prerelease === undefined) {
            return await this.#builder.build({ outputs, dependencies });
        }

        // author the next version, and build its prerelease planned from the last version
        const path = join(this.directory, "package.json");
        const declaration = schema.jsonObject().parse(await readJson(path));
        const version = Version.next(Version.parse(declaration["version"]));
        await writeFile(path, JSON.stringify({ ...declaration, version }));
        const name = schema.string().parse(declaration["name"]);

        return await this.#builder.build({
            outputs,
            dependencies,
            history,
            versions: { [name]: prerelease },
        });
    }

    /** Stop the builder. */
    [Symbol.asyncDispose](): Promise<void> {
        return this.#builder[Symbol.asyncDispose]();
    }
}

/** Run a command in a directory, failing with its error output unless it succeeds. */
export async function run(command: readonly string[], directory: string): Promise<string> {
    // run the command, collecting both outputs
    const child = Bun.spawn([...command], {
        cwd: directory,
        stdout: "pipe",
        stderr: "pipe",
        timeout: 60_000,
    });
    const [code, stdout, stderr] = await Promise.all([
        child.exited,
        new Response(child.stdout).text(),
        new Response(child.stderr).text(),
    ]);
    if (code !== 0) {
        throw new Error(`${command.join(" ")} failed with ${code}: ${stderr}`);
    }

    return stdout;
}

/** Read a JSON file. */
export async function readJson(path: string): Promise<unknown> {
    return JSON.parse(await readFile(path, "utf8"));
}
