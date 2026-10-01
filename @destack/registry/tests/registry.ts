import { DirectoryStore } from "@destack/directory";
import { Resolver } from "@destack/account/directory";
import { directoryTables } from "@destack/directory";
import { account } from "@destack/account/object";
import { accountTables } from "@destack/account/stack";
import { cp, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { inject } from "vitest";
import { PackageBuild, PackageBuilder } from "@destack/build";
import { readDependencies } from "@destack/build/local";
import { PackageStore } from "@destack/build/store";
import { principal } from "@destack/access";
import { copyOwner, copyScope } from "@destack/access/test";
import { Journal } from "@destack/audit";
import { LocalBucket } from "@destack/bucket/local";
import type { Dialect } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { PackageDefinition, type PackageId } from "@destack/package";
import { History, Vocabulary } from "@destack/resource";
import { ResourceContext } from "@destack/resource/context";
import { identifier } from "@destack/schema";
import { Authentication } from "@destack/service/authentication";
import { Health } from "@destack/service/health";
import { RequestId } from "@destack/service/request";
import { Server } from "@destack/service/server";

import { connect } from "../src/client/index.ts";
import { packageObject, type Release } from "../src/object/index.ts";
import { RegistryServer } from "../src/server/index.ts";
import { registryTables } from "../src/stack/index.ts";
import { testCallKey } from "@destack/service/test";

/** The account that publishes the fixture packages and scopes their npm names. */
export const ACCOUNT_ID = identifier("account").parse(
    "account-00000000-0000-7000-8000-000000000001",
);

/** The user that owns the account, with a bearer token. */
const OWNER = "owner";

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

/** A package the registry published: its build and release. */
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

/**
 * A registry over a regional and a global database and a local bucket, serving its service and npm endpoints over HTTP.
 *
 * Bearer tokens are the calling user's id, and requests without one call anonymously.
 */
export class RegistryFixture implements AsyncDisposable {
    /** The temporary directory with the bucket, sources and installations. */
    readonly directory: string;
    /** The regional database with packages, releases and tags. */
    readonly regional: TestDatabase;
    /** The global database with the account and the key index. */
    readonly global: TestDatabase;
    /** The bucket with immutable package files and archives. */
    readonly bucket: LocalBucket;
    /** The registry served. */
    readonly server: RegistryServer;
    /** The journal of the regional database's calls. */
    readonly journal: Journal;
    /** The registry's origin, with no trailing slash. */
    readonly origin: string;
    /** The packages created so far. */
    readonly #created = new Set<string>();
    /** The service serving the registry. */
    readonly #service: Server;
    /** The HTTP server passing requests to the service. */
    readonly #http: Bun.Server<undefined>;

    /** Serve the opened databases and bucket. */
    private constructor(
        directory: string,
        regional: TestDatabase,
        global: TestDatabase,
        bucket: LocalBucket,
    ) {
        // keep releases in the regional database and names in the global one
        this.directory = directory;
        this.regional = regional;
        this.global = global;
        this.bucket = bucket;

        // listen before serving, since the npm endpoints name their own URL
        let service: Server | undefined;
        this.#http = Bun.serve({
            hostname: "127.0.0.1",
            port: 0,
            fetch: (request) => service!.fetch(request),
        });
        this.origin = `http://127.0.0.1:${this.#http.port}`;

        // serve the registry to the user a bearer token names, and to anonymous callers
        this.server = new RegistryServer({
            callKey: testCallKey,
            database: regional.database,
            resolver: new Resolver(new DirectoryStore(global.database), (handle) =>
                Resolver.account(global.database, handle),
            ),
            store: new PackageStore(bucket),
            npm: new URL(`${this.origin}/npm/`),
        });
        this.journal = this.server.objects.journal;
        service = Server.start({
            ...this.server.service(),
            audience: packageObject.policy.definition.packageId,
            resources: new ResourceContext(),
            health: new Health("registry"),
            authenticate: async (request) => RegistryFixture.#authenticate(request),
            authorizeHost: async () => {},
            drainTimeout: 1000,
        });
        this.#service = service;
    }

    /** Open an empty registry of a dialect with its account. */
    static async open(dialect: Dialect): Promise<RegistryFixture> {
        // open the databases and bucket in a fresh directory
        const directory = await mkdtemp(join(tmpdir(), "destack-registry-"));
        const regional = await TestDatabase.create(dialect, [...registryTables], {
            isMigrated: true,
        });
        const global = await TestDatabase.create(dialect, [...accountTables, ...directoryTables], {
            isMigrated: true,
        });
        const bucket = await LocalBucket.open(join(directory, "bucket"), "space-test");

        // create the account in the global database, and copy its owner to the regional one
        const owner = principal.user.reference("universe", OWNER);
        await global.database.insert(account.table).values({
            id: ACCOUNT_ID,
            scope: owner.id,
            handle: "example",
            name: "Example",
            defaultResidency: "eu",
            createdAt: 1,
            updatedAt: 1,
        });
        const scope = { ...account.reference(owner.id, ACCOUNT_ID), scope: "universe" };
        await copyScope(regional.database, scope);
        await copyOwner(regional.database, scope, owner);

        return new RegistryFixture(directory, regional, global, bucket);
    }

    /** Read each change the journal recorded, with how it ended. */
    async audited(): Promise<[string, string][]> {
        return (await this.journal.read())
            .filter((call) => call.execution!.category === "activity")
            .map((call) => [call.method, call.execution!.outcome?.kind ?? "running"]);
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

        // store the build, then release it
        const manifest = await this.server.store.put(build);
        const release = await owner.release.create({
            accountId: ACCOUNT_ID,
            requestId: RequestId.create(),
            parentId: packageId,
            commit: COMMIT,
            manifest,
        });

        return { build, packageId, manifest, release };
    }

    /** Write the npm configuration selecting this registry for the fixture scope with the owner's token. */
    npmrc(): string {
        const url = new URL("/npm/", this.origin);

        return `@example:registry=${url.href}\n//${url.host}${url.pathname}:_authToken=${OWNER}\n`;
    }

    /** Stop serving and remove every database and file. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#http.stop(true);
        await this.#service.close();
        await this.bucket[Symbol.asyncDispose]();
        await this.regional.close();
        await this.global.close();
        await rm(this.directory, { recursive: true });
    }

    /** Verify a request's bearer token as the user it names, for a minute. */
    static #authenticate(request: Request): Authentication | null {
        const token = /^Bearer (.+)$/.exec(request.headers.get("authorization") ?? "")?.[1];
        if (token === undefined) {
            return null;
        }
        const subject = principal.user.reference("universe", token);

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

    /** Keep a copy and its started builder. */
    private constructor(name: FixturePackage, directory: string, builder: PackageBuilder) {
        this.name = name;
        this.directory = directory;
        this.#builder = builder;
    }

    /** Copy a fixture package's source into a directory and start its builder. */
    static async open(name: FixturePackage, parent: string): Promise<FixtureSource> {
        const directory = join(parent, name);
        await cp(fileURLToPath(new URL(`./fixture/${name}/`, import.meta.url)), directory, {
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
        const declaration = (await readJson(join(this.directory, "package.json"))) as object;
        const dependencies = await readDependencies(this.directory);

        // build the module outputs destack.json publishes, as a build worker does
        const definition = PackageDefinition.parse(
            await readJson(join(this.directory, "destack.json")),
        );
        const outputs = Object.fromEntries(
            Object.entries(definition.publication!.outputs).map(([name, output]) => [
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
            const history = latest && (await History.read(latest.reader, vocabulary));
            const build = await this.#builder.build({ outputs, dependencies, history });
            vocabulary = Vocabulary.advance(vocabulary, await build.reader.declarations(), version);
            builds.push(build);
        }

        return builds;
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
