import { Caller, principal } from "@destack/access";
import { connectionShape, Runner } from "@destack/connection/server";
import { PackageStore } from "@destack/build/store";
import { eq, inArray, sql, type DatabaseConnection, Snapshot, Transaction } from "@destack/db";
import { Directory } from "@destack/account/address";
import { account, claimShape, spaceRecordShape } from "@destack/account/object";
import {
    Claim,
    claimTable,
    type MethodContext,
    type ObjectType,
    type Fetch,
    Follow,
    PackageMount,
    CallError,
    RequestBody,
} from "@destack/object";
import type { ObjectStore, ImplementationInput, RequestContext } from "@destack/object/server";
import { PackageId, PackageName } from "@destack/package";
import { PackageManifest } from "@destack/package/manifest";
import { type Digest, type Identifier, present, schema } from "@destack/schema";
import { RequestId, type ObjectReference, Scope, type Subject } from "@destack/sync";
import type { InstalledServerContext } from "@destack/object/workload";
import { GITHUB_API, GitHubApp, GitHubHosting } from "../github/index.ts";
import { packageObject, reference, release, repository, unit } from "../object/index.ts";
import forgePackage from "../package.ts";
import { github } from "../stack/setting.ts";
import {
    credentialVault,
    githubPrivateKey,
    githubWebhookSecret,
    packageBucket,
    repositoryStorage,
} from "../stack/index.ts";
import type { GitStorage } from "../storage/index.ts";
import { ForgeError, type ForgeReason } from "../error/index.ts";
import { NpmServer } from "./npm-server.ts";
import { implementPackageObject } from "./package.ts";
import { implementRelease } from "./release.ts";
import { implementRepository } from "./repository.ts";
import { SWEEP_INTERVAL_MILLISECONDS, SweepController } from "./sweep-controller.ts";
import { implementTag } from "./tag.ts";
import { implementIssuer } from "./issuer.ts";

/** How long the forge follows a read name: an hour, at most 2 renewals and 300 B per name. */
const CLAIM_TTL = { hours: 1 } as const;

/** How long an unreleased pushed build survives sweeps: two weeks, Git's gc.pruneExpire. */
export const UNRELEASED_MILLISECONDS = 14 * 24 * 60 * 60 * 1000;

/** The home, stores, URL and GitHub App the forge serves with. */
export interface ForgeOptions {
    /** The residency home the forge serves, which stores every package of its residency. */
    readonly home: Identifier<"space">;
    /** The store of builds and npm archives. */
    readonly store: PackageStore;
    /** The URL clients call the forge at, below which npm reads its endpoints and builds. */
    readonly endpoint: URL;
    /** The storage of platform repositories. */
    readonly storage: GitStorage;
    /** The GitHub App that opens repositories connected through its installations. */
    readonly github?: GitHubApp;
    /** The fetch calling Git remotes. */
    readonly fetch: Fetch;
    /** The forge's own vault, which stores the credentials of Git remotes. */
    readonly vault: ObjectReference;
    /** The forge's installation, the consumer its accounts bind their connections to. */
    readonly installation: Subject;
}

/** How long the forge follows a calling space's row: an hour, 2 renewals and 1 KB per space. */
const STANDING_TTL = { hours: 1 } as const;

/** Follow the row of each space calling the forge, which places it inside its account. */
export const standingFollow = Follow.of(spaceRecordShape, { onDemand: { ttl: STANDING_TTL } });

/** Follow the universe's claims of account handles and package names as the forge reads them. */
export const claimFollow = Follow.of(claimShape, { onDemand: { ttl: CLAIM_TTL } });

/** The reads a name lookup takes: claims it follows on demand, and the replicated rows. */
export interface ClaimReader {
    /** Follow a claim's key on demand, until its rows are here. */
    readonly acquire: MethodContext["acquire"];
    /** Read the replicated rows. */
    readonly read: Snapshot["read"];
}

/** The forge's repositories and packages as objects. */
export class Forge {
    /** The handled types the forge serves. */
    readonly handled: readonly ObjectType[];
    /** The store of builds and npm archives. */
    readonly store: PackageStore;
    /** The forge's options: its home, storage and endpoint. */
    readonly #options: ForgeOptions;
    /** The repositories hosted at GitHub, absent in a universe running no GitHub App. */
    readonly #github: GitHubHosting | undefined;

    /** Serve the forge over its home's storage, stores and GitHub App. */
    constructor(options: ForgeOptions) {
        // take the options and the server of published builds
        this.#options = options;
        this.store = options.store;

        // open GitHub repositories through the replicas of their connections
        const hosting =
            options.github === undefined ? undefined : new GitHubHosting(options.github);
        this.#github = hosting;

        // serve repositories at their origins, and releases packed from pushed builds
        this.handled = [
            implementRepository({
                storage: options.storage,
                github: hosting,
                fetch: options.fetch,
                vault: options.vault,
            }),
            reference,
            implementPackageObject(),
            implementRelease({ store: options.store }),
            implementTag(),
            unit,
            implementIssuer(options.github),
        ];
    }

    /** Run a read, reporting a refusal or a missing object as missing with a message. */
    static async conceal<Result>(
        reason: ForgeReason,
        message: string,
        read: () => Promise<Result>,
    ): Promise<Result> {
        try {
            return await read();
        } catch (error) {
            // rethrow other failures
            const known = CallError.of(error);
            if (known === undefined || (known.code !== "NOT_FOUND" && known.code !== "FORBIDDEN")) {
                throw error;
            }

            throw ForgeError.of(reason, message, { cause: known });
        }
    }

    /** Serve the forge as its installation: its bound storages, its GitHub App, and its URL. */
    static of(context: InstalledServerContext): Forge {
        // open the GitHub App the installation's setting selects
        const fetch = (request: Request) => context.fetch(request);
        const app = github.read(context.settings);
        const hosting =
            app.kind === "none"
                ? undefined
                : new GitHubApp({
                      id: app.appId,
                      key: context.resources.get(githubPrivateKey),
                      secret: context.resources.get(githubWebhookSecret),
                      api: GITHUB_API,
                      fetch,
                      environment: context.environment,
                  });

        // serve npm and builds below the forge's package on the installation's URL
        return new Forge({
            home: schema.identifier("space").parse(context.scope),
            store: new PackageStore(context.resources.get(packageBucket), context.environment),
            endpoint: new URL(`${context.url}${PackageMount.path(forgePackage.id)}`),
            storage: context.resources.get(repositoryStorage),
            ...(hosting === undefined ? {} : { github: hosting }),
            fetch,
            vault: context.resources.get(credentialVault).reference,
            installation: principal.installation.reference(context.scope, context.installation),
        });
    }

    /** Define the forge's server: its objects, follows, controllers and adapters. */
    definition(): ImplementationInput<typeof forgePackage> {
        return {
            objects: this.handled,
            follows: [this.#connectionFollow(), claimFollow],
            standing: [standingFollow],
            // sweep the store
            controllers: [(store) => new SweepController(this, store)],
            // answer npm reads, builds and the GitHub App's deliveries
            adapters: {
                npm: (request, _caller, context, store) =>
                    new NpmServer(this.#options.endpoint, this, store).answer(request, context),
                builds: (request, _caller, context, store) => this.#builds(request, context, store),
                github: (request, _caller, _context, store) => this.#delivered(request, store),
            },
        };
    }

    /** Follow the connection of each live repository from its account's home, as its consumer. */
    #connectionFollow() {
        const { installation } = this.#options;

        return repository.follow(connectionShape, {
            where: { connectionId: { isNotNull: true }, deletedAt: { isNull: true } },
            key: async (row) => ({
                key: { scope: row.accountId, id: present(row.connectionId, "a connection") },
                subject: installation,
            }),
        });
    }

    /** Delete the pushed builds and archives no release in a database lists. */
    async sweep(now: number, database: DatabaseConnection): Promise<void> {
        // read the manifests and archives releases reference, by the digests the store lists
        const table = release.table;
        const archive = sql<string>`${table.distribution} ->> 'digest'`;
        const manifests = async (digests: readonly string[]) => {
            const released = await database
                .select({ digest: table.manifest })
                .from(table)
                .where(inArray(table.manifest, digests));
            const units = await database
                .select({ digest: unit.table.manifest })
                .from(unit.table)
                .where(inArray(unit.table.manifest, digests));

            return new Set([...released, ...units].map((row) => row.digest));
        };
        const archives = async (digests: readonly string[]) => {
            const rows = await database
                .select({ digest: archive })
                .from(table)
                .where(inArray(archive, digests));

            return new Set(rows.map((row) => row.digest));
        };

        // delete the rest stored before the window and marked an interval ago, and mark what is newly dropped
        await this.store.sweep(
            manifests,
            new Date(now - UNRELEASED_MILLISECONDS),
            archives,
            new Date(now),
            new Date(now - SWEEP_INTERVAL_MILLISECONDS),
        );
    }

    /** Find the package claiming a global name in a forge's home, following the claims it needs. */
    static async package(
        name: string,
        reader: ClaimReader,
        home: Identifier<"space">,
    ): Promise<ObjectReference> {
        // resolve the account handle, refusing an unclaimed one
        const { handle, local } = PackageName.split(name);
        await Forge.#acquire(reader, Claim.key(account, "handle", [handle]));
        const accountId = await new Directory(reader).account(handle);
        if (accountId === undefined) {
            throw ForgeError.of("PACKAGE_NOT_FOUND", "package not found");
        }

        // find the package claiming the name within the account
        const keyed = Claim.key(packageObject, "name", [accountId, local]);
        await Forge.#acquire(reader, keyed);
        const found = await Claim.owner(reader.read(claimTable), keyed);
        if (found === undefined) {
            throw ForgeError.of("PACKAGE_NOT_FOUND", "package not found");
        }

        // refer to the package in the forge's home
        return { ...packageObject.reference(home, found.objectId), id: found.objectId };
    }

    /** Follow a claim's key at the universe on demand. */
    static #acquire(reader: ClaimReader, claim: ReturnType<typeof Claim.key>): Promise<void> {
        return reader.acquire(claimFollow, { key: { scope: Scope.universe.id, ...claim } });
    }

    /** Find the package of a global name, reporting one the caller may not read as missing. */
    async find(
        name: string,
        context: RequestContext,
        objects: ObjectStore,
    ): Promise<ObjectReference> {
        // read the package as the caller, concealing a refusal
        const view = Snapshot.live(objects.database);
        const reader: ClaimReader = {
            acquire: (follow, at) => objects.acquire(follow, at, context.signal),
            read: (table) => view.read(table),
        };
        const found = await Forge.package(name, reader, this.#options.home);
        await Forge.conceal("PACKAGE_NOT_FOUND", "package not found", () =>
            objects.query(
                context.access(found.scope),
                packageObject.call("get", { spaceId: found.scope, id: found.id }),
            ),
        );

        return found;
    }

    /** Answer a delivery of the forge's GitHub App, refusing one where the universe runs none. */
    async #delivered(request: Request, objects: ObjectStore): Promise<Response> {
        // refuse a delivery without a GitHub App, and one that is no POST
        const hosting = this.#github;
        if (hosting === undefined) {
            throw ForgeError.of("GITHUB_WEBHOOK_MISSING", "the forge has no GitHub App");
        } else if (request.method !== "POST") {
            return new Response(null, { status: 405 });
        }

        // refresh the repositories the delivery changes
        await this.#receive(request, objects, hosting);

        return new Response(null, { status: 204 });
    }

    /** Receive a delivery of the forge's GitHub App signed with its webhook secret. */
    async #receive(request: Request, objects: ObjectStore, hosting: GitHubHosting): Promise<void> {
        // read the repositories the delivery changes, none for a ping
        const changed = await hosting.receive(request, Runner.of(objects));
        if (changed.length === 0) {
            return;
        }

        // refresh them, observing references as they are now
        request.signal.throwIfAborted();
        await objects.mutate(
            objects.as(repository),
            {
                id: RequestId.create(objects.environment),
                calls: changed.map((row) => repository.call("refresh", repository.target(row))),
            },
            { component: "github" },
        );
    }

    /** Serve published builds by digest, and uploads below a package to its publishers. */
    async #builds(
        request: Request,
        context: RequestContext,
        objects: ObjectStore,
    ): Promise<Response> {
        return this.store.fetch(request, new URL("/", this.#options.endpoint), {
            authorize: async (_request, digest) => {
                await this.#readable(digest, context, objects);
            },
            upload: async (upload, scope) => this.#authorizeUpload(upload, scope, context, objects),
        });
    }

    /** Authorize an upload below a package to its publishers, as OCI authorizes a repository. */
    async #authorizeUpload(
        request: Request,
        scope: string | undefined,
        context: RequestContext,
        objects: ObjectStore,
    ): Promise<void> {
        // require an identified caller uploading below a package
        if (Caller.isAnonymous(context.access())) {
            throw ForgeError.of("CREDENTIAL_MISSING", "missing caller credential");
        }
        const packageId = PackageId.safeParse(scope);
        if (!packageId.success) {
            throw ForgeError.of("UPLOAD_OUTSIDE_PACKAGE", "uploads must target a package");
        }

        // require publish on the package, as a release's creation does
        if (!(await this.#isPublisher(packageId.data, context, objects))) {
            throw ForgeError.of("PACKAGE_NOT_FOUND", "package not found");
        }
        await Forge.#requirePushedPackage(request, packageId.data);
    }

    /** Decide whether the caller may publish a package, denying a missing package. */
    async #isPublisher(
        packageId: PackageId,
        context: RequestContext,
        objects: ObjectStore,
    ): Promise<boolean> {
        // find the package's account
        const table = packageObject.table;
        const [found] = await objects.database
            .select({ scope: table.scope })
            .from(table)
            .where(eq(table.id, packageId))
            .limit(1);
        if (found === undefined) {
            return false;
        }

        // check publish as the caller in the account
        const authorization = await objects.authorize(
            Transaction.reading(objects.database),
            found.scope,
            context.access(found.scope),
        );
        const decision = await authorization.check(
            packageObject.permission("publish"),
            packageObject.reference(found.scope, packageId),
        );

        return decision.isAllowed;
    }

    /** Require a readable release of a build, or one listing it as a unit, yanked ones included. */
    async #readable(digest: Digest, context: RequestContext, objects: ObjectStore): Promise<void> {
        // find the releases of the build, and those listing it as a unit
        const table = release.table;
        const released = await objects.database
            .select({ scope: table.scope, id: table.id })
            .from(table)
            .where(eq(table.manifest, digest));
        const listing = await objects.database
            .select({ scope: unit.table.scope, id: unit.table.parentId })
            .from(unit.table)
            .where(eq(unit.table.manifest, digest));

        // pass at the first release the caller reads, concealing every refusal
        for (const published of [...released, ...listing]) {
            const isReadable = await Forge.conceal("BUILD_NOT_FOUND", "build not found", () =>
                objects.query(
                    context.access(published.scope),
                    release.call("get", { spaceId: published.scope, id: published.id }),
                ),
            ).then(
                () => true,
                (error: unknown) => {
                    if (!ForgeError.is(error, "BUILD_NOT_FOUND")) {
                        throw error;
                    }

                    return false;
                },
            );
            if (isReadable) {
                return;
            }
        }
        throw ForgeError.of("BUILD_NOT_FOUND", "build not found");
    }

    /** Require a pushed manifest to belong to the package it is uploaded below. */
    static async #requirePushedPackage(request: Request, packageId: PackageId): Promise<void> {
        // pass files, which any build of the package may list
        if (request.method !== "PUT" || !request.url.endsWith("/manifest.json")) {
            return;
        }

        // read a clone of the body within the limit, leaving the body to the store
        const value = await RequestBody.json(request.clone(), "the pushed manifest");
        const parsed = PackageManifest.safeParse(value);
        if (!parsed.success) {
            throw ForgeError.of("MANIFEST_INVALID", "the pushed manifest is invalid");
        }
        // refuse a Destack manifest of another package, passing an npm unit's for the release that lists it
        const pushed = parsed.data.package;
        if ("id" in pushed && pushed.id !== packageId) {
            throw ForgeError.of(
                "FOREIGN_MANIFEST",
                `the pushed manifest belongs to ${pushed.id}, not ${packageId}`,
            );
        }
    }
}
