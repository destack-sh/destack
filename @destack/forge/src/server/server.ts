import { AccessContext } from "@destack/access";
import { account, connection, host, hostKey, zone } from "@destack/account/object";
import type { WorkloadIdentity } from "@destack/account/client";
import type { Resolver } from "@destack/account/directory";
import { PackageServer, type PackageStore } from "@destack/build/store";
import { and, eq, isNull, type DatabaseConnection, type Select } from "@destack/db";
import type { CallOf } from "@destack/object";
import { SystemCall } from "@destack/object";
import { ObjectServer, Subscriber } from "@destack/object/server";
import { PackageName, type PackageId } from "@destack/package";
import { PackageManifest } from "@destack/package/manifest";
import { LeaseMode, Upgrade, type Lease } from "@destack/resource";
import { schema } from "@destack/schema";
import { conceal, isServiceError, ServiceError } from "@destack/service/error";
import type { CallKey } from "@destack/service/request";
import {
    implement,
    type ServiceContext,
    type ServiceImplementation,
} from "@destack/service/server";
import { Subject, type ObjectReference } from "@destack/sync";
import { GitHubHosting, type GitHubApp } from "../github/index.ts";
import {
    dependency,
    ORIGIN_FIELDS,
    type OriginColumns,
    type OriginMove,
    packageObject,
    type Pack,
    reference,
    referenceShape,
    release,
    repository,
    RepositoryOrigin,
    TagName,
} from "../object/index.ts";
import { PackageArchive } from "../pack/index.ts";
import { BUILDS_PATH, forgeService, manifests } from "../service/index.ts";
import { GitAdvertisement, GitListing, type Fetch, type GitStorage } from "../storage/index.ts";
import { NpmServer } from "./npm.ts";
import { Release } from "./release.ts";
import { Repository } from "./repository.ts";
import { SWEEP_INTERVAL_MILLISECONDS, SweepController } from "./sweep.ts";
import { requireTagName, tag } from "./tag.ts";

/** Typed implementations of the forge's procedures. */
const implementation = implement(manifests).$context<ServiceContext>();

/** The most bytes of a pushed manifest the forge reads to authorize it: a manifest names its lists by digest, so it stays within kilobytes. */
const MAX_MANIFEST_BYTES = 1024 * 1024;

/** How long a pushed build stays without a release before a sweep drops it: a day, far longer than pushing a build and publishing it takes. */
export const UNRELEASED_MILLISECONDS = 24 * 60 * 60 * 1000;

/** How long a release's creation leases its build and archive: two sweep intervals, far longer than packing a build and committing its release takes. */
const LEASE_MILLISECONDS = 2 * SWEEP_INTERVAL_MILLISECONDS;

/** The hostings an update may move an origin between. */
const MOVABLE_HOSTINGS: ReadonlySet<OriginColumns["hosting"]> = new Set(["github", "git"]);

/** The origin columns no origin names, which the columns of each origin overwrite. */
const BLANK_COLUMNS = {
    host: null,
    remote: null,
    authentication: null,
    connectedAccountId: null,
    secretSpaceId: null,
    secretId: null,
    provider: null,
    providerRepositoryId: null,
} as const satisfies Omit<OriginColumns, "hosting">;

/** The database, workload identity, stores, URL and GitHub App the forge serves with. */
export interface ForgeOptions {
    /** The forge's database with its objects, and copies of its residency's accounts and connections. */
    readonly database: DatabaseConnection;
    /** The forge's placement in its region, which follows the account service's copies of its residency's accounts and resolves names through its directory. */
    readonly identity: WorkloadIdentity;
    /** The key sensitive call inputs are fingerprinted under in the journal. */
    readonly callKey: CallKey;
    /** The store of builds and npm archives. */
    readonly store: PackageStore;
    /** The URL clients call the forge at, below which npm reads its endpoints and builds. */
    readonly endpoint: URL;
    /** The storage of platform repositories. */
    readonly storage: GitStorage;
    /** The GitHub App opening repositories connected through its installations, absent in a universe running none. */
    readonly github?: GitHubApp;
    /** The fetch calling Git remotes. */
    readonly fetch?: Fetch;
    /** Report committed external work that fails to settle. */
    readonly report?: (error: unknown) => void;
}

/** The forge service with its object server and the forge receiving webhooks and sweeping builds. */
export interface ForgeImplementation extends ServiceImplementation {
    /** The object server executing the forge's methods. */
    readonly objects: Forge["objects"];
    /** The forge serving the service. */
    readonly forge: Forge;
}

/** Implement the forge service with the npm endpoints, the builds of published releases and the sweep of unreleased builds. */
export function implementForge(options: ForgeOptions): ForgeImplementation {
    // serve the objects with the sweep, the manifests, the npm endpoints and the builds
    const forge = new Forge(options);

    return {
        ...forge.objects.implement(forgeService, [new SweepController(forge)]),
        router: { ...forge.objects.router(), manifests: forge.manifests() },
        route: async (request, context) =>
            (await forge.npm.route(request, context)) ?? (await forge.builds(request, context)),
        objects: forge.objects,
        forge,
    };
}

/** The forge's repositories and packages as objects, with the npm endpoints, the builds and the GitHub webhook. */
export class Forge {
    /** The object server executing the forge's methods. */
    readonly objects: ObjectServer<{
        readonly repository: typeof repository;
        readonly reference: typeof reference;
        readonly package: typeof packageObject;
        readonly release: typeof release;
        readonly tag: typeof tag;
        readonly dependency: typeof dependency;
    }>;
    /** The resolver of account handles with the directory that claims names. */
    readonly resolver: Resolver;
    /** The store of builds and npm archives. */
    readonly store: PackageStore;
    /** The npm endpoints. */
    readonly npm: NpmServer;
    /** The builds of published releases, served by their manifest's digest. */
    readonly served: PackageServer;
    /** The repository object with this server's handlers. */
    readonly #repository: typeof repository;
    /** The region's database, storage and GitHub App. */
    readonly #options: ForgeOptions;
    /** The fetch calling Git remotes. */
    readonly #fetch: Fetch;
    /** The repositories kept at GitHub, absent in a universe running no GitHub App. */
    readonly #github: GitHubHosting | undefined;

    /** Serve the forge over a region's database, stores and GitHub App. */
    constructor(options: ForgeOptions) {
        // keep the region's options and the directory resolving account handles
        const { identity } = options;
        this.#options = options;
        this.#fetch = options.fetch ?? globalThis.fetch;
        this.#github =
            options.github === undefined
                ? undefined
                : new GitHubHosting(options.github, options.database);
        this.resolver = identity.directory().resolver();
        this.store = options.store;
        this.#repository = this.#handleRepository();

        // publish releases by packing pushed builds before the transaction
        const releases = release.handle({
            create: { prepare: (call) => this.#pack(call), handler: Release.publish },
            unpublish: Release.unpublish,
        });

        // serve the forge's objects
        this.objects = new ObjectServer({
            objects: {
                repository: this.#repository,
                reference,
                package: packageObject,
                release: releases,
                tag,
                dependency,
            },
            // decide methods by the copied accounts' roles
            policies: [account, connection, host, hostKey, zone],
            database: options.database,
            // claim names in the directory
            directory: this.resolver.directory,
            callKey: options.callKey,
            origin: { package: forgeService.package, service: forgeService.name },
            subscriber: Subscriber.of(identity.publisher(), () =>
                this.objects.source.workloadSubscriptions(identity.placementId),
            ),
            // stream selected references to the spaces copied zones stand for
            standing: [zone],
            shapes: [referenceShape],
            context: (context, scope) => AccessContext.withCells(context.access(scope)),
            ...(options.report === undefined ? {} : { report: options.report }),
        });
        this.npm = new NpmServer(options.endpoint, this);
        this.served = new PackageServer(new URL(BUILDS_PATH, options.endpoint), options.store);
    }

    /** Receive a delivery of the forge's GitHub App signed with its webhook secret, refreshing as the system the repositories it changes. */
    async receive(request: Request, secret: string): Promise<void> {
        // read the repositories the delivery changes, ignoring the ping
        const change = await this.#requireGitHub().receive(request, secret);
        if (change === null) {
            return;
        }

        // refresh them, observing references as they are now
        request.signal.throwIfAborted();
        if (change.repositories.length > 0) {
            await this.objects.executeAsSystem(
                this.#repository,
                "refresh",
                change.repositories.map((row) => SystemCall.of(row)),
                change.receivedAt,
            );
        }
    }

    /** Delete the pushed builds and archives no release names, stored longer than the window before a moment and marked by a sweep an interval earlier. */
    async sweep(now: number): Promise<void> {
        // keep every manifest and archive a release names
        const releases = await this.objects.database
            .select({ manifest: release.table.manifest, distribution: release.table.distribution })
            .from(release.table);
        const released = new Set(releases.map((each) => each.manifest));
        const archives = new Set(releases.map((each) => each.distribution.digest));

        // delete the rest stored before the window and marked an interval ago, and mark what is newly dropped
        await this.store.sweep(
            released,
            new Date(now - UNRELEASED_MILLISECONDS),
            archives,
            new Date(now),
            new Date(now - SWEEP_INTERVAL_MILLISECONDS),
        );
    }

    /** Find the package a global name names, reporting one the caller may not read as missing. */
    async find(name: string, context: ServiceContext): Promise<ObjectReference> {
        // read the named package as the caller, concealing a refusal
        const found = await this.#named(name);
        await concealed("package not found", () =>
            this.objects.query(
                packageObject,
                "get",
                { accountId: found.scope, id: found.id },
                context,
            ),
        );

        return found;
    }

    /** Bind the repository object to this server's origins and storage, calling them outside each transaction. */
    #handleRepository(): typeof repository {
        return repository.handle({
            create: {
                prepare: (call) =>
                    this.#provide(
                        Repository.createdId(call),
                        call.scope,
                        Repository.readOrigin(call.input),
                    ),
                handler: (call, next) =>
                    next(call.with({ input: { ...call.input, ...call.prepared } })),
                commit: async (call, prepared) => {
                    await this.#createStorage(call.requireId(), prepared);
                },
            },
            update: { prepare: (call) => this.#move(call), handler: Repository.update },
            refresh: {
                prepare: (call) => this.#list(call.target),
                handler: (call) => Repository.record(call, call.prepared),
            },
            report: (call) => Repository.record(call, call.input),
            open: {
                authorize: Repository.authorizeWrite,
                prepare: (call) => this.#lease(call.target, call.input.mode),
                handler: async (call) => call.prepared,
            },
            purge: {
                prepare: async (call) => Repository.readColumns(call.target),
                commit: async (_call, prepared) => {
                    await this.#deleteStorage(prepared);
                },
            },
        });
    }

    /** Answer the manifest of a published release a caller reads, by version or distribution tag. */
    manifests() {
        return implementation.router({
            find: implementation.find.handler(async ({ input, context }) => {
                // read the version a distribution tag points at
                const [tagged] = TagName.safeParse(input.release).success
                    ? await this.objects.database
                          .select({ version: tag.table.version })
                          .from(tag.table)
                          .where(
                              and(
                                  eq(tag.table.parentId, input.packageId),
                                  eq(tag.table.name, input.release),
                              ),
                          )
                    : [];
                const version = tagged?.version ?? input.release;

                // find the published release of the version
                const table = release.table;
                const [published] = await this.objects.database
                    .select({ manifest: table.manifest })
                    .from(table)
                    .where(
                        and(
                            eq(table.parentId, input.packageId),
                            eq(table.version, version),
                            isNull(table.unpublishedAt),
                        ),
                    )
                    .limit(1);
                if (published === undefined) {
                    throw new ServiceError("NOT_FOUND", {
                        message: `${input.packageId} has no release ${input.release}`,
                    });
                }

                // read it as the caller would its build
                await this.#readable(published.manifest, context);

                return { manifest: published.manifest };
            }),
        });
    }

    /** Serve the build of a published release below the builds path as the caller reads its package, and take the builds its publishers push. */
    async builds(request: Request, context: ServiceContext): Promise<Response | undefined> {
        // leave paths outside the builds
        if (!new URL(request.url).pathname.startsWith(BUILDS_PATH)) {
            return undefined;
        }

        return this.served.fetch(request, {
            authorize: async (_request, digest) => {
                await this.#readable(digest, context);

                return {};
            },
            upload: async (upload) => this.#authorizeUpload(upload, context),
        });
    }

    /** Authorize an upload from a signed-in caller, and a pushed manifest only to a publisher of the package it names. */
    async #authorizeUpload(request: Request, context: ServiceContext): Promise<void> {
        // take files from any signed-in caller, since a sweep drops the files no release keeps
        context.requireAuthentication();
        if (request.method !== "PUT" || !request.url.endsWith("/manifest.json")) {
            return;
        }

        // require publish on the package the manifest names, as a release's creation does
        const packageId = await pushedPackage(request);
        if (!(await this.#isPublisher(packageId, context))) {
            throw new ServiceError("NOT_FOUND", { message: "package not found" });
        }
    }

    /** Decide whether the caller holds publish on a package, which a missing package denies. */
    async #isPublisher(packageId: PackageId, context: ServiceContext): Promise<boolean> {
        // find the package's account
        const table = packageObject.table;
        const [found] = await this.objects.database
            .select({ scope: table.scope })
            .from(table)
            .where(eq(table.id, packageId))
            .limit(1);
        if (found === undefined) {
            return false;
        }

        // check publish as the caller in the account
        const authorization = await this.objects.authorize(
            this.objects.database,
            found.scope,
            context,
        );
        const decision = await authorization.check(
            packageObject.permission("publish"),
            packageObject.reference(found.scope, packageId),
        );

        return decision.isAllowed;
    }

    /** Require a published release of a build whose package the caller reads, reporting any other build as missing. */
    async #readable(digest: string, context: ServiceContext): Promise<void> {
        // find a published release of the build
        const table = release.table;
        const [published] = await this.objects.database
            .select({ scope: table.scope, id: table.id })
            .from(table)
            .where(and(eq(table.manifest, digest), isNull(table.unpublishedAt)))
            .limit(1);
        if (published === undefined) {
            throw new ServiceError("NOT_FOUND", { message: "build not found" });
        }

        // read the release as the caller, concealing a refusal
        await concealed("build not found", () =>
            this.objects.query(
                release,
                "get",
                { accountId: published.scope, id: published.id },
                context,
            ),
        );
    }

    /** Find the package with a global name `@<account handle>/<name>` through the directory. */
    async #named(name: string): Promise<ObjectReference> {
        // resolve the account handle and the name within the account
        const [handle, local] = PackageName.parse(name).slice(1).split("/");
        if (handle === undefined || local === undefined) {
            throw new TypeError(`package name has no account handle: ${name}`);
        }
        const accountId = await this.resolver.account(handle);
        const found =
            accountId === undefined
                ? undefined
                : await packageObject.lookup(this.resolver.directory, "name", [local], accountId);
        if (found === undefined) {
            throw new ServiceError("NOT_FOUND", { message: "package not found" });
        }

        return found;
    }

    /** Pack a pushed build of a commit under the package's global name, and store its npm archive. */
    async #pack(call: CallOf<typeof release, "create">): Promise<Pack> {
        // refuse a tag a range reads, and lease the build against sweeps until its release keeps it
        requireTagName(Release.tagName(call));
        const { parentId, manifest } = call.input;
        const until = new Date(Date.now() + LEASE_MILLISECONDS);
        await this.store.lease(manifest, until);

        // require a pushed build of a commit
        if (!(await this.store.contains(manifest))) {
            throw new ServiceError("NOT_FOUND", {
                message: `build ${manifest} was not pushed to the forge`,
            });
        }
        const contents = await this.store.contents(manifest);
        const commit = contents.manifest.commit;
        if (commit === undefined) {
            throw new ServiceError("CONFLICT", {
                message: `build ${manifest} compiled a working tree with uncommitted changes`,
            });
        }

        // pack the build
        const archive = await PackageArchive.pack(contents, commit);
        if (archive.manifest !== manifest) {
            throw new ServiceError("BAD_REQUEST", {
                message: "archived manifest differs from the build",
            });
        }

        // require the build to be the package's, under the package's global name
        if (archive.packageId !== parentId) {
            throw new ServiceError("CONFLICT", { message: "the build is another package's" });
        }
        const named = await this.#named(archive.metadata.name);
        if (named.id !== parentId) {
            throw new ServiceError("CONFLICT", {
                message: "the npm name belongs to another package",
            });
        }

        // lease and store the archive under its digest before any release refers to it
        await this.store.lease(archive.file.digest, until);
        await this.store.putFile(archive.file, async () => archive.open());

        // read the build's declarations and the upgrade it planned
        const upgrade = contents.manifest.upgrade;

        return {
            manifest,
            commit,
            version: archive.metadata.version,
            distribution: archive.distribution,
            metadata: archive.metadata,
            dependencies: archive.dependencies,
            ...(upgrade === undefined
                ? {}
                : { upgrade: await contents.reader.read(upgrade.file, Upgrade) }),
            declarations: await contents.reader.declarations(),
        };
    }

    /** Prepare the origin an update names, none when it names none of its fields. */
    async #move(call: CallOf<typeof repository, "update">): Promise<OriginMove> {
        // keep the origin when the update names none of its fields
        const target = call.target;
        const isMoved = ORIGIN_FIELDS.some((name) => call.input[name] !== undefined);
        if (!isMoved) {
            return null;
        }

        // move GitHub and Git repositories only, between GitHub and Git origins
        if (!MOVABLE_HOSTINGS.has(target.hosting)) {
            throw new ServiceError("BAD_REQUEST", {
                message: `${target.hosting} repositories do not move`,
            });
        }
        const merged = Object.fromEntries(
            ORIGIN_FIELDS.map((name) => [
                name,
                call.input[name] === undefined ? target[name] : call.input[name],
            ]),
        );
        const moved = Repository.readOrigin(merged);
        if (!MOVABLE_HOSTINGS.has(moved.hosting)) {
            throw new ServiceError("BAD_REQUEST", {
                message: `${target.hosting} repositories do not move to ${moved.hosting} origins`,
            });
        }

        return {
            revision: target.revision,
            columns: await this.#provide(target.id, call.scope, moved),
        };
    }

    /** Read the columns of an origin and its identity in storage or at GitHub. */
    async #provide(id: string, scope: string, origin: RepositoryOrigin): Promise<OriginColumns> {
        const { storage } = this.#options;

        // derive the platform repository the region's storage creates once the repository commits
        if (origin.hosting === "platform") {
            return {
                ...BLANK_COLUMNS,
                hosting: "platform",
                provider: storage.provider,
                providerRepositoryId: id,
                remote: storage.remote(id),
            };
        }
        // identify a GitHub repository through the account's installation
        else if (origin.hosting === "github") {
            const providerRepositoryId = await this.#requireGitHub().identify(scope, origin);

            return { ...BLANK_COLUMNS, ...origin, providerRepositoryId };
        }
        // keep a host by its subject's key
        else if (origin.hosting === "host") {
            return { ...BLANK_COLUMNS, hosting: "host", host: Subject.key(origin.host) };
        }
        // keep a Git remote as named
        else {
            return { ...BLANK_COLUMNS, ...origin };
        }
    }

    /** List an origin's references through storage or Git's advertisement. */
    async #list(target: Select<typeof repository.table>): Promise<GitListing> {
        // list a platform repository in the region's storage
        if (target.hosting === "platform") {
            return this.#options.storage.references(this.#storageId(target));
        }

        // read the advertisement of other remotes with read access
        return GitAdvertisement.read(await this.#lease(target, "read"), this.#fetch);
    }

    /** Lease an origin to read, or to write as well. */
    async #lease(target: Select<typeof repository.table>, mode: LeaseMode): Promise<Lease> {
        const { storage } = this.#options;

        // lease a platform repository through the region's storage
        if (target.hosting === "platform") {
            return storage.open(this.#storageId(target), mode);
        }
        // lease a GitHub repository through an installation token limited to it and the mode
        else if (target.hosting === "github") {
            return this.#requireGitHub().open(target, mode);
        }
        // leave host repositories to their host
        else if (target.hosting === "host") {
            throw new ServiceError("BAD_REQUEST", {
                message: "host repositories are refreshed and opened through their host",
            });
        }
        // pull an anonymous remote unchanged
        else if (target.authentication === "anonymous" && mode === "read") {
            if (target.remote === null) {
                throw new TypeError(`git repository ${target.id} has no remote`);
            }

            return { url: target.remote, mode, headers: {} };
        }
        // refuse pushing to an anonymous remote, for which the platform has no credential
        else if (target.authentication === "anonymous") {
            throw new ServiceError("BAD_REQUEST", {
                message: "anonymous remotes take no push credentials",
            });
        }
        // TODO #Incomplete: read secret credentials through the vault of their space once services read vaults
        else {
            throw new ServiceError("NOT_IMPLEMENTED", {
                message: "secret credentials resolve through the vault of their space",
            });
        }
    }

    /** Create a committed platform repository in storage, removing it when a purge came first. */
    async #createStorage(id: string, columns: OriginColumns): Promise<void> {
        // create nothing for origins the platform does not store
        if (columns.hosting !== "platform") {
            return;
        }

        // create the storage
        const storageId = this.#storageId(columns);
        await this.#options.storage.create(storageId);

        // remove the storage when a purge removed the repository first
        const table = repository.table;
        const [row] = await this.#options.database
            .select({ id: table.id })
            .from(table)
            .where(eq(table.id, schema.identifier("repository").parse(id)));
        if (row === undefined) {
            await this.#options.storage.delete(storageId);
        }
    }

    /** Delete a purged platform repository from storage. */
    async #deleteStorage(columns: OriginColumns): Promise<void> {
        if (columns.hosting === "platform") {
            await this.#options.storage.delete(this.#storageId(columns));
        }
    }

    /** Read a platform repository's identifier in this region's storage. */
    #storageId(columns: Pick<OriginColumns, "provider" | "providerRepositoryId">): string {
        // refuse repositories another storage keeps
        const { storage } = this.#options;
        if (columns.provider !== storage.provider) {
            throw new ServiceError("SERVICE_UNAVAILABLE", {
                message: `repository is stored by ${columns.provider}, this region stores ${storage.provider}`,
            });
        }
        // refuse a platform repository without its storage identifier
        else if (
            columns.providerRepositoryId === null ||
            columns.providerRepositoryId === undefined
        ) {
            throw new TypeError("platform repository has no storage identifier");
        }

        return columns.providerRepositoryId;
    }

    /** Read the repositories kept at GitHub, refusing them in a universe running no GitHub App. */
    #requireGitHub(): GitHubHosting {
        const github = this.#github;
        if (github === undefined) {
            throw new ServiceError("PRECONDITION_FAILED", {
                message: "this universe runs no github app for github repositories",
            });
        }

        return github;
    }
}

/** Run a read as the caller, reporting a refusal or a missing object as a missing one with a message. */
async function concealed(message: string, read: () => Promise<unknown>): Promise<void> {
    try {
        await read();
    } catch (error) {
        if (!isServiceError(error) || (error.code !== "NOT_FOUND" && error.code !== "FORBIDDEN")) {
            throw error;
        }
        throw conceal(error, message);
    }
}

/** Read the package a pushed manifest names, leaving the request's body to the store. */
async function pushedPackage(request: Request): Promise<PackageId> {
    // read a copy of the body within the limit
    const body: ReadableStream<Uint8Array<ArrayBuffer>> | null = request.clone().body;
    if (body === null) {
        throw new ServiceError("BAD_REQUEST", { message: "the pushed manifest is empty" });
    }
    const reader = body.getReader();
    const chunks: Uint8Array<ArrayBuffer>[] = [];
    let size = 0;
    for (let read = await reader.read(); !read.done; read = await reader.read()) {
        size += read.value.byteLength;
        if (size > MAX_MANIFEST_BYTES) {
            await reader.cancel();
            throw new ServiceError("PAYLOAD_TOO_LARGE", {
                message: "the pushed manifest exceeds 1 MiB",
            });
        }
        chunks.push(read.value);
    }

    // parse the manifest
    let value: unknown;
    try {
        value = JSON.parse(await new Blob(chunks).text());
    } catch (error) {
        if (!(error instanceof SyntaxError)) {
            throw error;
        }
        throw new ServiceError("BAD_REQUEST", { message: "the pushed manifest is not JSON" });
    }
    const parsed = PackageManifest.safeParse(value);
    if (!parsed.success) {
        throw new ServiceError("BAD_REQUEST", { message: "the pushed manifest is invalid" });
    }

    return parsed.data.package.id;
}
