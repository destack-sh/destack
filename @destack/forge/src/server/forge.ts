import { Caller, principal } from "@destack/access";
import { Directory } from "@destack/account/address";
import { account, claimShape } from "@destack/account/object";
import { connectionShape, Runner } from "@destack/connection/server";
import { PackageStore } from "@destack/build/store";
import { eq, inArray, sql, type DatabaseConnection, Snapshot, Transaction } from "@destack/db";
import { Claim, ClaimLookup, claimTable, Follow, type ObjectType } from "@destack/object";
import type { ObjectStore, ServerDefinition } from "@destack/object/server";
import { PackageId, PackageName } from "@destack/package";
import { PackageManifest } from "@destack/package/manifest";
import { type Identifier, schema } from "@destack/schema";
import { type Fetch, PackageMount } from "@destack/runtime";
import { conceal, isCallError, CallError } from "@destack/runtime/error";
import { RequestBody, RequestId } from "@destack/runtime/request";
import type { RequestContext } from "@destack/runtime/server";
import type { ServerContext } from "@destack/runtime/workload";
import { type ObjectReference, Scope, type Subject } from "@destack/sync";
import { GITHUB_API, GitHubApp, GitHubHosting } from "../github/index.ts";
import { packageObject, reference, release, repository } from "../object/index.ts";
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
import { GitHubController } from "./github-controller.ts";
import { NpmServer } from "./npm-server.ts";
import { implementPackage } from "./package.ts";
import { implementRelease } from "./release.ts";
import { implementRepository } from "./repository.ts";
import { SWEEP_INTERVAL_MILLISECONDS, SweepController } from "./sweep-controller.ts";
import { implementTag } from "./tag.ts";
import { implementIssuer } from "./issuer.ts";
import { Clock } from "@destack/environment";

/** How long an unreleased pushed build survives sweeps: two weeks, Git's gc.pruneExpire. */
export const UNRELEASED_MILLISECONDS = 14 * 24 * 60 * 60 * 1000;

/** How long the forge follows a read name: an hour, at most 2 renewals and 300 B per name. */
const CLAIM_TTL = { hours: 1 } as const;

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
    readonly fetch?: Fetch;
    /** The forge's own vault, which stores the credentials of Git remotes. */
    readonly vault: ObjectReference;
    /** The forge installation, the one consumer reading those credentials. */
    readonly consumer: Subject;
}

/** Follow the connection of each live repository from its account's home. */
const connectionFollow = Follow.of(connectionShape, {
    from: repository.query({
        where: { connectionId: { isNotNull: true }, deletedAt: { isNull: true } },
    }),
    key: async (row) => ({
        key: {
            scope: schema.identifier("account").parse(row["accountId"]),
            id: schema.identifier("connection").parse(row["connectionId"]),
        },
    }),
});

/** Follow the universe's claims of account handles and package names as the forge reads them. */
export const claimFollow = Follow.of(claimShape, { onDemand: { ttl: CLAIM_TTL } });

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
                fetch: options.fetch ?? globalThis.fetch,
                vault: options.vault,
                consumer: options.consumer,
            }),
            reference,
            implementPackage(options.home),
            implementRelease({ store: options.store, home: options.home }),
            implementTag(),
            ...(options.github === undefined ? [] : [implementIssuer(options.github)]),
        ];
    }

    /** Run a read, reporting a refusal or a missing object as missing with a message. */
    static async conceal<Result>(message: string, read: () => Promise<Result>): Promise<Result> {
        try {
            return await read();
        } catch (error) {
            // rethrow other failures
            if (!isCallError(error) || (error.code !== "NOT_FOUND" && error.code !== "FORBIDDEN")) {
                throw error;
            }

            throw conceal(error, message);
        }
    }

    /** Serve the forge as its installation: its bound storages, its GitHub App, and its URL. */
    static of(context: ServerContext): Forge {
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
                      clock: Clock.system,
                  });

        // serve npm and builds below the forge's package on the installation's URL
        return new Forge({
            home: schema.identifier("space").parse(context.space),
            store: new PackageStore(context.resources.get(packageBucket)),
            endpoint: new URL(`${context.url}${PackageMount.path(forgePackage.id)}`),
            storage: context.resources.get(repositoryStorage),
            ...(hosting === undefined ? {} : { github: hosting }),
            fetch,
            vault: context.resources.get(credentialVault).reference,
            consumer: principal.installation.reference(context.space, context.installation),
        });
    }

    /** Define the forge's server: its objects, follows, controllers and adapters. */
    definition(): ServerDefinition<typeof forgePackage> {
        const hosting = this.#github;

        return {
            objects: this.handled,
            follows: [connectionFollow, claimFollow],
            // sweep the store, and identify GitHub repositories where the universe runs a GitHub App
            controllers: [
                (store) => new SweepController(this, store),
                ...(hosting === undefined
                    ? []
                    : [(store: ObjectStore) => new GitHubController(store, hosting)]),
            ],
            // answer npm reads, builds and the GitHub App's deliveries
            adapters: {
                npm: (request, _caller, context, store) =>
                    new NpmServer(this.#options.endpoint, this, store).answer(request, context),
                builds: (request, _caller, context, store) => this.#builds(request, context, store),
                github: (request, _caller, _context, store) => this.#delivered(request, store),
            },
        };
    }

    /** Delete the pushed builds and archives no release in a database lists. */
    async sweep(now: number, database: DatabaseConnection): Promise<void> {
        // read the manifests and archives releases reference, by the digests the store lists
        const table = release.table;
        const archive = sql<string>`${table.distribution} ->> 'digest'`;
        const manifests = async (digests: readonly string[]) => {
            const rows = await database
                .select({ digest: table.manifest })
                .from(table)
                .where(inArray(table.manifest, digests));

            return new Set(rows.map((row) => row.digest));
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

    /** Find the package of a global name, reporting one the caller may not read as missing. */
    async find(
        name: string,
        context: RequestContext,
        objects: ObjectStore,
    ): Promise<ObjectReference> {
        // read the package as the caller, concealing a refusal
        const found = await Forge.named(
            name,
            this.#options.home,
            (claim) =>
                objects.acquire(
                    claimFollow,
                    { key: { scope: Scope.universe.id, ...claim } },
                    context.signal,
                ),
            Snapshot.live(objects.database),
        );
        await Forge.conceal("package not found", () =>
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
            throw new CallError("NOT_FOUND", { message: "the forge runs no GitHub App" });
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
        const now = objects.clock.now();
        const changed = await hosting.receive(request, Runner.of(objects));
        if (changed.length === 0) {
            return;
        }

        // refresh them, observing references as they are now
        request.signal.throwIfAborted();
        await objects.mutate(
            objects.serving(repository),
            {
                id: RequestId.create(now),
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

                return {};
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
            throw new CallError("UNAUTHORIZED", { message: "missing caller credential" });
        }
        const packageId = PackageId.safeParse(scope);
        if (!packageId.success) {
            throw new CallError("NOT_FOUND", { message: "upload below a package" });
        }

        // require publish on the package, as a release's creation does
        if (!(await this.#isPublisher(packageId.data, context, objects))) {
            throw new CallError("NOT_FOUND", { message: "package not found" });
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

    /** Require a release of a build whose package the caller reads, yanked ones included. */
    async #readable(digest: string, context: RequestContext, objects: ObjectStore): Promise<void> {
        // find a release of the build
        const table = release.table;
        const [published] = await objects.database
            .select({ scope: table.scope, id: table.id })
            .from(table)
            .where(eq(table.manifest, digest))
            .limit(1);
        if (published === undefined) {
            throw new CallError("NOT_FOUND", { message: "build not found" });
        }

        // read the release as the caller, concealing a refusal
        await Forge.conceal("build not found", () =>
            objects.query(
                context.access(published.scope),
                release.call("get", { spaceId: published.scope, id: published.id }),
            ),
        );
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
            throw new CallError("BAD_REQUEST", { message: "the pushed manifest is invalid" });
        }
        // refuse a manifest of another package
        else if (parsed.data.package.id !== packageId) {
            throw new CallError("BAD_REQUEST", {
                message: `the pushed manifest belongs to ${parsed.data.package.id}, not ${packageId}`,
            });
        }
    }

    /** Find the package with a global name `@<account handle>/<name>` through its claims. */
    static async named(
        name: string,
        home: Identifier<"space">,
        acquire: (claim: Pick<Claim, "index" | "key">) => Promise<void>,
        view: Pick<Snapshot, "read">,
    ): Promise<ObjectReference> {
        // resolve the account handle, refusing an unclaimed one
        const { handle, local } = PackageName.split(name);
        await acquire(Claim.key(account, "handle", [handle]));
        const accountId = await new Directory(view).account(handle);
        if (accountId === undefined) {
            throw new CallError("NOT_FOUND", { message: "package not found" });
        }

        // find the package claiming the name within the account
        const keyed = Claim.key(packageObject, "name", [accountId, local]);
        await acquire(keyed);
        const found = await ClaimLookup.of(view.read(claimTable)).owner(keyed.index, keyed.key);
        if (found === undefined) {
            throw new CallError("NOT_FOUND", { message: "package not found" });
        }

        // refer to the package in the forge's space, which stores every package of its residency
        return { ...packageObject.reference(home, found.objectId), id: found.objectId };
    }
}
