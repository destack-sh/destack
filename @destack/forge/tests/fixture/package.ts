import type { WorkloadIdentity } from "@destack/account/client";
import { AccountFixture, ids } from "@destack/host/test";
import { cp, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { inject } from "vitest";
import { type BuildOptions, PackageBuild, PackageBuilder, readDependencies } from "@destack/build";
import { PackageStore } from "@destack/build/store";
import { principal } from "@destack/access";
import { Journal } from "@destack/audit";
import { LocalBucket } from "@destack/bucket/local";
import type { Dialect } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { PackageDefinition, type PackageId } from "@destack/package";
import { History, Vocabulary } from "@destack/resource";
import { ResourceContext } from "@destack/resource/context";
import { Commit, found, type Identifier, schema } from "@destack/schema";
import { Authentication } from "@destack/service/authentication";
import { Health } from "@destack/service/health";
import { RequestId } from "@destack/service/request";
import { Server } from "@destack/service/server";

import { type PackageDistribution } from "@destack/package/manifest";
import { connect } from "../../src/client/index.ts";
import { LocalGitStorage } from "../../src/local/index.ts";
import { packageObject, type Release } from "../../src/object/index.ts";
import { ForgeServer } from "../../src/server/index.ts";
import { BUILDS_PATH, forgeService } from "../../src/service/index.ts";
import { forgeDatabase } from "../../src/stack/index.ts";
import { testCallKey } from "@destack/service/test";

/** The account that publishes the fixture packages and scopes their npm names. */
export const ACCOUNT_ID = schema
    .identifier("account")
    .parse("account-00000000-0000-7000-8000-000000000001");

/** The account's owner, by the bearer token naming them. */
const OWNER = "owner";

/** The users bearer tokens name: the account's owner and a stranger to it. */
const USERS: ReadonlyMap<string, string> = new Map([
    [OWNER, ids.owner],
    ["stranger", ids.stranger],
]);

/** The version the fixture sources declare. */
const FIXTURE_VERSION = "2026.9.0";

/** The fixture releases the run builds once, in an order that publishes each dependency before its dependents. */
export const FIXTURE_RELEASES = [
    ["answer", [FIXTURE_VERSION, "2026.9.1"]],
    ["greeting", [FIXTURE_VERSION]],
] as const satisfies readonly (readonly [FixturePackage, readonly string[]])[];

/** The commit every fixture release is built from. */
export const COMMIT = "a".repeat(40);

/** The fixture packages: real package sources under tests/fixture called `@example/<name>`. */
export type FixturePackage = "answer" | "greeting";

/** Who may read a package, and whether it is listed to them. */
export type Visibility = "public" | "unlisted" | "private";

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

/** A forge over its database, an account service and a local bucket, serving its endpoints over HTTP. */
export class PackageFixture implements AsyncDisposable {
    /** The temporary directory with the bucket, sources and installations. */
    readonly directory: string;
    /** The forge's database with packages, releases and tags. */
    readonly regional: TestDatabase;
    /** The account service with the account and the directory. */
    readonly accounts: AccountFixture;
    /** The bucket with immutable package files and archives. */
    readonly bucket: LocalBucket;
    /** The publisher's store of the builds it pushes. */
    readonly published: PackageStore;
    /** The bucket with the publisher's builds. */
    readonly publisher: LocalBucket;
    /** The forge served. */
    readonly server: ForgeServer;
    /** The forge's placement in the platform's region. */
    readonly placement: Identifier<"placement">;
    /** The journal of the forge database's calls. */
    readonly journal: Journal;
    /** The forge's origin, with no trailing slash. */
    readonly origin: string;
    /** The packages created so far. */
    readonly #created = new Set<string>();
    /** The service serving the forge. */
    readonly #service: Server;
    /** The HTTP server passing requests to the service. */
    readonly #http: Bun.Server<undefined>;

    /** Serve the opened databases and bucket. */
    private constructor(
        directory: string,
        regional: TestDatabase,
        accounts: AccountFixture,
        bucket: LocalBucket,
        publisher: LocalBucket,
        identity: WorkloadIdentity,
    ) {
        // keep releases in the forge's database and names in the account service's directory
        this.directory = directory;
        this.regional = regional;
        this.accounts = accounts;
        this.bucket = bucket;
        this.publisher = publisher;
        this.published = new PackageStore(publisher);

        // listen before serving, since the npm endpoints name their URL
        this.#http = Bun.serve({
            hostname: "127.0.0.1",
            port: 0,
            fetch: (request) => this.#service.fetch(request),
        });
        this.origin = `http://127.0.0.1:${this.#http.port}`;

        // serve the forge to the user a bearer token names, and to anonymous callers
        this.server = new ForgeServer({
            identity,
            callKey: testCallKey,
            database: regional.database,
            store: new PackageStore(bucket),
            endpoint: new URL(this.origin),
            storage: new LocalGitStorage(join(directory, "repository")),
        });
        this.placement = identity.placementId;
        this.journal = this.server.objects.journal;
        this.#service = Server.start({
            ...this.server.service(),
            audience: packageObject.policy.definition.packageId,
            resources: new ResourceContext(),
            health: new Health("forge"),
            authenticate: async (request) => PackageFixture.#authenticate(request),
            authorizeHost: async () => {},
            drainTimeout: 1000,
        });
    }

    /** Open an empty forge of a dialect, following the account service as its placement, with the owner's account. */
    static async open(dialect: Dialect): Promise<PackageFixture> {
        // open the databases and bucket in a fresh directory
        const directory = await mkdtemp(join(tmpdir(), "destack-forge-"));
        const regional = await TestDatabase.create(dialect, forgeDatabase, {
            isMigrated: true,
        });
        const accounts = await AccountFixture.open({ policies: [packageObject.policy] });
        const bucket = await LocalBucket.open(join(directory, "bucket"), "space-test");
        const publisher = await LocalBucket.open(join(directory, "publisher"), "space-test");

        // create the owner's account through the account service, and place the forge in the platform's region
        await accounts.user(ids.owner).account.create({
            scope: ids.owner,
            id: ACCOUNT_ID,
            requestId: RequestId.create(),
            handle: "example",
            name: "Example",
            kind: "shared",
        });
        const placement = await accounts.place(forgeService.package.id);

        // follow and reach the account service as the forge's workload through a host of the platform's region, once copied
        const region = await accounts.enroll(ids.platform);
        const fixture = new PackageFixture(
            directory,
            regional,
            accounts,
            bucket,
            publisher,
            accounts.identity(region, placement),
        );
        await fixture.settle();

        return fixture;
    }

    /** Read each change the journal recorded, with how it ended. */
    async audited(): Promise<[string, string][]> {
        return (await this.journal.read())
            .filter((call) => call.execution.category === "activity")
            .map((call) => [call.method, call.execution.outcome?.kind ?? "running"]);
    }

    /** Connect to the service as a user, or anonymously. */
    client(user?: string) {
        return connect({
            url: this.origin,
            headers: user === undefined ? {} : { authorization: `Bearer ${user}` },
        });
    }

    /** Publish a fixture package at a version from the run's build of it, creating it as the owner the first time. */
    async publish(
        name: FixturePackage,
        options: { readonly visibility?: Visibility; readonly version?: string } = {},
    ): Promise<Published> {
        const build = await fixtureBuild(name, options.version);

        return this.release(name, build, options.visibility);
    }

    /** Publish a build of a fixture package as the owner, creating the package the first time. */
    async release(
        name: FixturePackage,
        build: PackageBuild,
        visibility: Visibility = "private",
    ): Promise<Published> {
        // create the package under the build's identity once
        const owner = this.client(OWNER);
        const packageId = build.manifest.package.id;
        if (!this.#created.has(packageId)) {
            await owner.package.create({
                accountId: ACCOUNT_ID,
                id: packageId,
                requestId: RequestId.create(),
                name,
                visibility,
            });
            this.#created.add(packageId);
        }

        // push the build, then release it
        const manifest = await this.push(build);
        const release = await owner.release.create({
            accountId: ACCOUNT_ID,
            requestId: RequestId.create(),
            parentId: packageId,
            manifest,
        });

        return { build, packageId, manifest, release };
    }

    /** Push a build to the forge as the owner, returning its manifest's digest. */
    async push(build: PackageDistribution): Promise<string> {
        const manifest = await this.published.put(build);
        await this.published.push(manifest, `${this.origin}${BUILDS_PATH}`, (request) => {
            const headers = new Headers(request.headers);
            headers.set("authorization", `Bearer ${OWNER}`);

            return fetch(new Request(request, { headers }));
        });

        return manifest;
    }

    /** Write the npm configuration selecting this registry for the fixture scope with the owner's token. */
    npmrc(): string {
        const url = this.server.npm.url;

        return `@example:registry=${url.href}\n//${url.host}${url.pathname}:_authToken=${OWNER}\n`;
    }

    /** Stop serving and remove every database and file. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#http.stop(true);
        await this.#service.close();
        await this.bucket[Symbol.asyncDispose]();
        await this.publisher[Symbol.asyncDispose]();
        await this.regional.close();
        await this.accounts[Symbol.asyncDispose]();
        await rm(this.directory, { recursive: true });
    }

    /** Wait until the forge's copies reflect the account service as of now. */
    async settle(): Promise<void> {
        await this.accounts.settle(this.server.objects, this.placement);
    }

    /** Verify a request's bearer token as the user it names, for a minute. */
    static #authenticate(request: Request): Authentication | null {
        const token = /^Bearer (.+)$/u.exec(request.headers.get("authorization") ?? "")?.[1];
        if (token === undefined) {
            return null;
        }
        const subject = principal.user.reference("universe", found(USERS, token));

        return new Authentication({
            credential: { kind: "fixture", id: "fixture-1" },
            audience: packageObject.policy.definition.packageId,
            subject,
            subjects: [subject],
            verifiedAt: Date.now(),
            expiresAt: Date.now() + 60_000,
        });
    }
}

/** Read the global setup's build of the answer package's last version compiled without a commit. */
export function uncommittedBuild(): Promise<PackageBuild> {
    return PackageBuild.read(join(inject("builds"), "answer", "uncommitted"));
}

/** Read the global setup's build of a fixture package at a version. */
export function fixtureBuild(
    name: FixturePackage,
    version = FIXTURE_VERSION,
): Promise<PackageBuild> {
    return PackageBuild.read(join(inject("builds"), name, version));
}

/** A copy of a fixture package's source with a started builder. */
export class FixtureSource implements AsyncDisposable {
    /** The fixture package. */
    readonly name: FixturePackage;
    /** The directory with the copy. */
    readonly directory: string;
    /** The builder retained across the package's versions. */
    readonly #builder: PackageBuilder;
    /** The outputs and dependencies the last build compiled. */
    #compiled: Pick<BuildOptions, "outputs" | "dependencies"> | undefined;

    /** Keep a copy and its started builder. */
    private constructor(name: FixturePackage, directory: string, builder: PackageBuilder) {
        this.name = name;
        this.directory = directory;
        this.#builder = builder;
    }

    /** Copy a fixture package's source into a directory and start its builder. */
    static async open(name: FixturePackage, parent: string): Promise<FixtureSource> {
        const directory = join(parent, name);
        await cp(fileURLToPath(new URL(`./${name}/`, import.meta.url)), directory, {
            recursive: true,
        });

        return new FixtureSource(name, directory, await PackageBuilder.start(directory));
    }

    /** Install the dependencies through an npm configuration, then build each version's published outputs, rebuilding only what the version changes. */
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
            .record(schema.string(), schema.json())
            .parse(await readJson(join(this.directory, "package.json")));
        const dependencies = await readDependencies(this.directory);

        // build the module outputs destack.json publishes, as a build worker does
        const definition = PackageDefinition.parse(
            await readJson(join(this.directory, "destack.json")),
        );
        if (definition.publication === undefined) {
            throw new TypeError(`fixture ${this.name} declares no publication`);
        }
        const outputs = Object.fromEntries(
            Object.entries(definition.publication.outputs).map(([name, output]) => [
                name,
                { kind: "module" as const, ...output },
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

        this.#compiled = { outputs, dependencies };

        return builds;
    }

    /** Build the last version again without a commit, as a working tree with uncommitted changes compiles. */
    uncommitted(): Promise<PackageBuild> {
        if (this.#compiled === undefined) {
            throw new TypeError(`fixture ${this.name} has not been built`);
        }

        return this.#builder.build(this.#compiled);
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
