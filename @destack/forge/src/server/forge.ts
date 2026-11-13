import { Caller, principal } from "@destack/access";
import { connectionShape, Runner } from "@destack/connection/server";
import { BuildStore } from "@destack/build/store";
import type { BuildLayer } from "@destack/package/manifest";
import type { Environment } from "@destack/environment";
import { type BlobStore, inArray, Snapshot, TABLE, Transaction } from "@destack/db";
import { Directory } from "@destack/account/address";
import { Account, account, claimShape, spaceRecordShape } from "@destack/account/object";
import {
    Claim,
    claimTable,
    type MethodContext,
    type ObjectType,
    type Fetch,
    PackageMount,
    CallError,
} from "@destack/object";
import type { ObjectStore, ImplementationInput, RequestContext } from "@destack/object/server";
import { PackageName } from "@destack/package";
import { type Digest, type Identifier, present, schema } from "@destack/schema";
import { RequestId, type ObjectReference, Scope, type Subject } from "@destack/sync";
import type { ServerContext, PackageContext } from "@destack/object/workload";
import type { WebhookDelivery } from "@destack/workflow";
import { GITHUB_API, GitHubApp, GitHubHosting } from "../github/index.ts";
import { packageObject, reference, release, repository } from "@destack/registry/object";
import forgePackage from "../package.ts";
import { github } from "../stack/setting.ts";
import {
    credentialVault,
    githubPrivateKey,
    githubWebhook,
    repositoryStorage,
} from "../stack/index.ts";
import type { GitStorage } from "@destack/git";
import { ForgeError, type ForgeReason } from "../error/index.ts";
import { implementPackageObject } from "./package.ts";
import { implementRelease, ReleaseMethod } from "./release.ts";
import { implementRepository } from "./repository.ts";
import { implementTag } from "./tag.ts";
import { implementIssuer } from "./issuer.ts";

/** How long the forge follows a read name: an hour, at most 2 renewals and 300 B per name. */
const CLAIM_TTL = { hours: 1 } as const;

/** How long the forge leases an unreleased pushed build: two weeks, as Git's gc.pruneExpire. */
export const UNRELEASED_MILLISECONDS = 14 * 24 * 60 * 60 * 1000;

/** The home, stores, URL and GitHub App the forge serves with. */
export interface ForgeOptions {
    /** The residency home the forge serves, which stores every package of its residency. */
    readonly home: Identifier<"space">;
    /** The home's blobs, with the store's tables beside the forge's objects. */
    readonly blobs: BlobStore;
    /** The builds of the host's distribution, which the forge has without anyone pushing them. */
    readonly distribution: BuildLayer;
    /** The environment the store of builds runs in. */
    readonly environment: Environment;
    /** The URL clients call the forge at, below which they push and pull builds. */
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
    /** The installation's resources, which bind the GitHub App's webhook secret. */
    readonly server: Pick<ServerContext, "resources">;
}

/** How long the forge follows a calling space's row: an hour, 2 renewals and 1 KB per space. */
const STANDING_TTL = { hours: 1 } as const;

/** Follow the row of each space calling the forge, which places it inside its account. */
export const standingFollow = spaceRecordShape.onDemand(STANDING_TTL);

/** Follow the universe's claims of account handles and package names as the forge reads them. */
export const claimFollow = claimShape.onDemand(CLAIM_TTL);

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
    readonly store: BuildStore;
    /** The forge's options: its home, storage and endpoint. */
    readonly #options: ForgeOptions;
    /** The repositories hosted at GitHub, absent in a universe running no GitHub App. */
    readonly #github: GitHubHosting | undefined;

    /** Serve the forge over its home's storage, stores and GitHub App. */
    constructor(options: ForgeOptions) {
        // take the options and the server of published builds
        this.#options = options;
        this.store = Forge.openStore(options);

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
            implementPackageObject({ store: this.store }),
            implementRelease(),
            implementTag(),
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
    static of(context: PackageContext): Forge {
        // open the GitHub App the installation's setting selects
        const fetch = (request: Request) => context.fetch(request);
        const app = github.read(context.settings);
        const hosting =
            app.kind === "none"
                ? undefined
                : new GitHubApp({
                      id: app.appId,
                      key: context.resources.get(githubPrivateKey),
                      api: GITHUB_API,
                      fetch,
                      environment: context.environment,
                  });

        // serve npm and builds below the forge's package on the installation's URL
        return new Forge({
            home: schema.identifier("space").parse(context.scope),
            blobs: context.blobs,
            distribution: context.distribution,
            environment: context.environment,
            endpoint: new URL(`${context.url}${PackageMount.path(forgePackage.id)}`),
            storage: context.resources.get(repositoryStorage),
            ...(hosting === undefined ? {} : { github: hosting }),
            fetch,
            vault: context.resources.get(credentialVault).reference,
            installation: principal.installation.reference(context.scope, context.installation),
            server: context,
        });
    }

    /** Define the forge's server: its objects, follows, mounts and webhooks. */
    definition(): ImplementationInput<typeof forgePackage> {
        // take the GitHub App's deliveries, refusing them where the universe runs no GitHub App
        const hosting = this.#github;
        const webhook =
            hosting === undefined
                ? undefined
                : githubWebhook.serve(
                      (delivery, store) => this.#receive(delivery, store, hosting),
                      this.#options.server,
                  );

        return {
            objects: this.handled,
            // delete the prereleases past the space's retention
            cursors: [(store) => ReleaseMethod.retention(store)],
            follows: [this.#connectionFollow(), claimFollow],
            standing: [standingFollow],
            // answer builds, and the GitHub App's deliveries
            mounts: {
                builds: (request, _caller, context, store) => this.#builds(request, context, store),
                github:
                    webhook ??
                    (async () => {
                        throw ForgeError.of(
                            "GITHUB_WEBHOOK_NOT_FOUND",
                            "forge GitHub App not found",
                        );
                    }),
            },
        };
    }

    /** Follow the connection of each live repository from its account's home, as its consumer. */
    #connectionFollow() {
        const { installation } = this.#options;

        return connectionShape.per(repository, {
            where: { connectionId: { isNotNull: true }, deletedAt: { isNull: true } },
            key: async (row) => ({
                key: { scope: row.accountId, id: present(row.connectionId, "a connection") },
                subject: installation,
            }),
        });
    }

    /** Open the store of pushed builds in the home's blobs over the host's distribution. */
    static openStore(
        options: Pick<ForgeOptions, "blobs" | "environment" | "distribution">,
    ): BuildStore {
        return new BuildStore({
            blobs: options.blobs,
            environment: options.environment,
            layer: options.distribution,
            leaseMilliseconds: UNRELEASED_MILLISECONDS,
        });
    }

    /** Find the package claiming a global name in a forge's home, following the claims it needs. */
    static async package(
        name: string,
        reader: ClaimReader,
        home: Identifier<"space">,
    ): Promise<ObjectReference> {
        const found = (await Forge.find(name, reader, home)).package;
        if (found === undefined) {
            throw ForgeError.of("PACKAGE_NOT_FOUND", "package not found");
        }

        return found;
    }

    /** Find the account of a name's handle, and the package claiming the name, absent for none. */
    static async find(
        name: string,
        reader: ClaimReader,
        home: Identifier<"space">,
    ): Promise<{
        readonly accountId: Identifier<"account">;
        readonly package: ObjectReference | undefined;
    }> {
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
        const claimed =
            found === undefined
                ? undefined
                : { ...packageObject.reference(home, found.objectId), id: found.objectId };

        return { accountId, package: claimed };
    }

    /** Refuse a builtin outside the platform's organization, the one owning the forge's home. */
    static async requireBuiltin(
        name: string,
        reader: ClaimReader,
        home: Identifier<"space">,
    ): Promise<void> {
        // read the organization of the forge's home account and of the package's account
        const homeAccount = await reader
            .read(account.table)
            .findFirst({ where: { id: Account.ofHome(home) } });
        const platform = present(homeAccount, `the account of the forge's home ${home}`).scope;
        const { handle } = PackageName.split(name);
        await Forge.#acquire(reader, Claim.key(account, "handle", [handle]));
        const holder = await Claim.owner(
            reader.read(claimTable),
            Claim.key(account, "handle", [handle]),
        );

        // refuse another organization's or a person's package
        if (holder?.owner !== platform) {
            throw ForgeError.of(
                "BUILTIN_DENIED",
                `builtin '${name}' is forbidden: only the platform's organization publishes builtins`,
            );
        }
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

    /** Refresh the repositories a verified delivery of the forge's GitHub App changes. */
    async #receive(
        delivery: WebhookDelivery,
        objects: ObjectStore,
        hosting: GitHubHosting,
    ): Promise<void> {
        // read the repositories the delivery changes, none for a ping
        const changed = await hosting.receive(delivery, Runner.of(objects));
        if (changed.length === 0) {
            return;
        }

        // refresh them, observing references as they are now
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
            pull: async (_request, digest) => {
                await this.#readable(digest, context, objects);
            },
            push: async (upload, scope) => this.#authorizeUpload(upload, scope, context, objects),
        });
    }

    /** Authorize an upload below an account's handle: an enrolled machine's or a publisher's. */
    async #authorizeUpload(
        request: Request,
        scope: string | undefined,
        context: RequestContext,
        objects: ObjectStore,
    ): Promise<void> {
        // require an identified caller uploading below an account
        if (Caller.isAnonymous(context.access())) {
            throw ForgeError.of(
                "CREDENTIAL_UNAUTHENTICATED",
                "authentication required: caller credential is missing",
            );
        }
        if (scope === undefined) {
            throw ForgeError.of(
                "UPLOAD_OUTSIDE_ACCOUNT_NOT_FOUND",
                "upload target account not found",
            );
        }

        // stage an enrolled machine's upload
        const isMachine = principal.machine.is(context.access().subject);
        if (!isMachine && !(await this.#isPublisher(scope, context, objects))) {
            throw ForgeError.of(
                "UPLOAD_OUTSIDE_ACCOUNT_NOT_FOUND",
                "upload target account not found",
            );
        }
    }

    /** Decide whether the caller may publish packages of a handle's account. */
    async #isPublisher(
        handle: string,
        context: RequestContext,
        objects: ObjectStore,
    ): Promise<boolean> {
        // find the account claiming the handle
        const keyed = Claim.key(account, "handle", [handle]);
        const snapshot = Snapshot.live(objects.database);
        const reader: ClaimReader = {
            acquire: (follow, at) => objects.acquire(follow, at),
            read: (table) => snapshot.read(table),
        };
        await Forge.#acquire(reader, keyed);
        const found = await Claim.owner(reader.read(claimTable), keyed);
        if (found === undefined) {
            return false;
        }

        // check publish on a new package of the account
        const scope = present(context.scope, "the upload's space");
        const authorization = await objects.authorize(
            Transaction.reading(objects.database),
            scope,
            context.access(scope),
        );
        const id = packageObject.generateId(objects.environment);
        const row = packageObject.table[TABLE].defaults({ id, scope, accountId: found.objectId });
        const decision = await authorization.checkRows(packageObject.permission("publish"), scope, [
            row,
        ]);

        return decision.permitted.has(0);
    }

    /** Require the caller to read a package with a release whose lock is the digest or lists it. */
    async #readable(digest: Digest, context: RequestContext, objects: ObjectStore): Promise<void> {
        // find the packages of the releases of the lock
        const table = release.table;
        const locks = [digest, ...(await this.store.referencing(digest))];
        const rows = await objects.database
            .selectDistinct({ scope: table.scope, id: table.parentId })
            .from(table)
            .where(inArray(table.lock, locks));

        // pass at the first package the caller reads, concealing every refusal
        for (const owner of rows) {
            const isReadable = await Forge.conceal("BUILD_NOT_FOUND", "build not found", () =>
                objects.query(
                    context.access(owner.scope),
                    packageObject.call("get", { spaceId: owner.scope, id: owner.id }),
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
}
