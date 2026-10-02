import { LeaseMode, type Lease } from "@destack/resource";
import type { CallKey } from "@destack/service/request";
import type { Directory } from "@destack/directory";
import { Subject } from "@destack/sync";
import { Connections } from "@destack/account/server";
import { and, eq, isNull, type DatabaseConnection, type Insert } from "@destack/db";
import type { Call } from "@destack/object";
import { ObjectServer, SystemCall } from "@destack/object/server";
import { identifier } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import type { ServiceImplementation } from "@destack/service/server";
import { WEBHOOK_SIGNATURES } from "@destack/service/trigger";
import { GitHubApp, GitHubEvent } from "../github/index.ts";
import {
    ORIGIN_COLUMNS,
    ORIGIN_FIELDS,
    reference,
    repository,
    RepositoryOrigin,
    type Repository,
} from "../object/index.ts";
import { repositoryService } from "../service/index.ts";
import { GitAdvertisement, GitListing, type Fetch, type GitStorage } from "../storage/index.ts";

/** The hostings an update may move an origin between. */
const MOVABLE_HOSTINGS: ReadonlySet<Repository["hosting"]> = new Set(["github", "git"]);

/** One call of a repository method. */
type RepositoryCall = Call<typeof repository.table>;

/** The columns naming a repository's origin and the storage or GitHub identity it has there. */
type OriginColumns = Pick<Insert<typeof repository.table>, (typeof ORIGIN_COLUMNS)[number]>;

/** The origin an update moves to, prepared at the revision the transaction requires. */
interface Move {
    /** The revision the move was prepared at. */
    readonly revision: number;
    /** The origin columns the update records. */
    readonly columns: OriginColumns;
}

/** What a region serving repositories keeps and reaches. */
export interface RepositoryServerOptions {
    /** The regional database with repositories and their references. */
    readonly database: DatabaseConnection;
    /** The key sensitive call inputs are fingerprinted under in the journal. */
    readonly callKey: CallKey;
    /** The global database with account connections. */
    readonly global: DatabaseConnection;
    /** The directory whose key index keeps repository names. */
    readonly directory: Directory;
    /** The storage of platform repositories. */
    readonly storage: GitStorage;
    /** The GitHub App reaching repositories connected through its installations. */
    readonly github: GitHubApp;
    /** The fetch reaching Git remotes. */
    readonly fetch?: Fetch;
    /** Report committed external work that fails to settle. */
    readonly report?: (error: unknown) => void;
}

/** Serves repositories and their references as objects, with the GitHub webhook. */
export class RepositoryServer {
    /** The object server executing repository and reference methods. */
    readonly objects: ObjectServer<{
        readonly repository: typeof repository;
        readonly reference: typeof reference;
    }>;
    /** The repository object with this server's handlers. */
    readonly #repository: typeof repository;
    /** What the region keeps and reaches. */
    readonly #options: RepositoryServerOptions;
    /** The fetch reaching Git remotes. */
    readonly #fetch: Fetch;

    /** Serve repositories with the region's storage and GitHub App. */
    constructor(options: RepositoryServerOptions) {
        // reach origins and storage outside each transaction, and record their results inside it
        this.#options = options;
        this.#fetch = options.fetch ?? globalThis.fetch;
        this.#repository = repository.handle({
            create: {
                prepare: (call) =>
                    this.#provide(call.id!, call.scope, RepositoryServer.#origin(call.input)),
                effect: (call, next) =>
                    next(
                        call.with({
                            input: { ...call.input, ...(call.prepared as OriginColumns) },
                        }),
                    ),
                settle: async (call, prepared, isCommitted) => {
                    if (isCommitted) {
                        await this.#store(call.id!, prepared as OriginColumns);
                    }
                },
            },
            update: {
                prepare: (call) => this.#move(call),
                effect: (call, next) => RepositoryServer.#update(call, next),
            },
            refresh: {
                prepare: (call) => this.#list(call.target!),
                effect: (call) => RepositoryServer.#record(call, call.prepared as GitListing),
            },
            report: (call) => RepositoryServer.#record(call, GitListing.parse(call.input)),
            open: {
                authorize: (call) => RepositoryServer.#authorizeWrite(call),
                prepare: (call) => this.#reach(call.target!, LeaseMode.parse(call.input.mode)),
                effect: async (call) => call.prepared,
            },
            purge: {
                prepare: async (call) => RepositoryServer.#columns(call.target!),
                settle: async (_call, prepared, isCommitted) => {
                    if (isCommitted) {
                        await this.#erase(prepared as OriginColumns);
                    }
                },
            },
        });

        // decide every method through the roles of the account each repository lives in, claiming names in the directory
        this.objects = new ObjectServer({
            objects: { repository: this.#repository, reference },
            database: options.database,
            directory: options.directory,
            callKey: options.callKey,
            origin: { package: repositoryService.package, service: repositoryService.name },
            ...(options.report === undefined ? {} : { report: options.report }),
        });
    }

    /** Implement the repository service. */
    service(): ServiceImplementation {
        return this.objects.implement(repositoryService);
    }

    /** Receive a delivery of the repository GitHub App signed with its webhook secret, refreshing as the system the repositories it changes. */
    async receive(request: Request, secret: string): Promise<void> {
        // verify the delivery and read the changed repository, ignoring the ping
        const delivery = await WEBHOOK_SIGNATURES.github.verify(request, secret, {}, Date.now());
        const signal = request.signal;
        const event = GitHubEvent.read(delivery);
        if (event === null) {
            return;
        }

        // find the live repositories of the GitHub repository and the connections to the delivering installation
        const table = repository.table;
        const [candidates, connections] = await Promise.all([
            this.#options.database
                .select()
                .from(table)
                .where(
                    and(
                        eq(table.hosting, "github"),
                        eq(table.providerRepositoryId, event.repositoryId),
                        isNull(table.deletionRequestedAt),
                    ),
                ),
            this.#installations({ installationId: event.installationId }),
        ]);

        // refresh the repositories connected through that installation only, observing references as they are now
        const installations = new Set(connections.map((row) => `${row.scope} ${row.id}`));
        const repositories = candidates.filter((candidate) =>
            installations.has(`${candidate.scope} ${candidate.connectedAccountId}`),
        );
        signal.throwIfAborted();
        if (repositories.length > 0) {
            await this.objects.executeAsSystem(
                this.#repository,
                "refresh",
                repositories.map((row) => SystemCall.of(row)),
                delivery.receivedAt,
            );
        }
    }

    /** Prepare the origin an update names, none when it names none of its fields. */
    async #move(call: RepositoryCall): Promise<Move | null> {
        // keep the origin when the update names none of its fields
        const target = call.target!;
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
        const origin = RepositoryServer.#origin(merged);
        if (!MOVABLE_HOSTINGS.has(origin.hosting)) {
            throw new ServiceError("BAD_REQUEST", {
                message: `${target.hosting} repositories do not move to ${origin.hosting} origins`,
            });
        }

        return {
            revision: target.revision,
            columns: await this.#provide(target.id, call.scope, origin),
        };
    }

    /** Read the columns of an origin and its identity in storage or at GitHub. */
    async #provide(id: string, scope: string, origin: RepositoryOrigin): Promise<OriginColumns> {
        const { storage, github } = this.#options;
        const empty = Object.fromEntries(ORIGIN_COLUMNS.map((name) => [name, null]));

        // derive the platform repository the region's storage creates once the repository commits
        if (origin.hosting === "platform") {
            return {
                ...empty,
                hosting: "platform",
                provider: storage.provider,
                providerRepositoryId: id,
                remote: storage.remote(id),
            };
        }
        // identify a GitHub repository through the account's installation
        else if (origin.hosting === "github") {
            const installation = await this.#installation(scope, origin.connectedAccountId);
            const found = await github.repository(installation, GitHubApp.fullName(origin.remote));

            return { ...empty, ...origin, providerRepositoryId: found.id };
        }
        // keep a host by its subject's key
        else if (origin.hosting === "host") {
            return { ...empty, hosting: "host", host: Subject.key(origin.host) };
        }
        // keep a Git remote as named
        else {
            return { ...empty, ...origin };
        }
    }

    /** List an origin's references through storage or Git's advertisement. */
    async #list(target: Repository): Promise<GitListing> {
        // list a platform repository in the region's storage
        if (target.hosting === "platform") {
            return this.#options.storage.references(this.#storageId(target));
        }

        // read the advertisement of other remotes with read access
        return GitAdvertisement.read(await this.#reach(target, "read"), this.#fetch);
    }

    /** Lease an origin to read, or to write as well. */
    async #reach(target: Repository, mode: LeaseMode): Promise<Lease> {
        const { storage, github } = this.#options;

        // reach a platform repository through the region's storage
        if (target.hosting === "platform") {
            return storage.open(this.#storageId(target), mode);
        }
        // reach a GitHub repository through an installation token limited to it and the mode
        else if (target.hosting === "github") {
            const installation = await this.#installation(target.scope, target.connectedAccountId!);

            return github.open(installation, target.providerRepositoryId!, target.remote!, mode);
        }
        // leave host repositories to their host
        else if (target.hosting === "host") {
            throw new ServiceError("BAD_REQUEST", {
                message: "host repositories are refreshed and reached through their host",
            });
        }
        // pull an anonymous remote unchanged
        else if (target.authentication === "anonymous" && mode === "read") {
            return { url: target.remote!, mode, headers: {} };
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
    async #store(id: string, columns: OriginColumns): Promise<void> {
        // create nothing for origins the platform does not store
        if (columns.hosting !== "platform") {
            return;
        }

        // create the storage, then require the repository, since a purge settling first leaves nothing to remove it later
        const storageId = this.#storageId(columns);
        await this.#options.storage.create(storageId);
        const table = repository.table;
        const [row] = await this.#options.database
            .select({ id: table.id })
            .from(table)
            .where(eq(table.id, identifier("repository").parse(id)));
        if (row === undefined) {
            await this.#options.storage.delete(storageId);
        }
    }

    /** Delete a purged platform repository from storage. */
    async #erase(columns: OriginColumns): Promise<void> {
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

        return columns.providerRepositoryId!;
    }

    /** Read the installation of this GitHub App an account's connection names. */
    async #installation(scope: string, connectedAccountId: string): Promise<string> {
        const [row] = await this.#installations({
            scope: identifier("account").parse(scope),
            id: identifier("connected-account").parse(connectedAccountId),
        });
        if (row === undefined) {
            throw new ServiceError("BAD_REQUEST", {
                message: `connection ${connectedAccountId} is no installation of the github app`,
            });
        }

        return row.installationId;
    }

    /** Read the live installations of this GitHub App a selection names. */
    #installations(
        selection: Omit<
            Parameters<typeof Connections.installations>[1],
            "provider" | "applicationId"
        >,
    ) {
        return Connections.installations(this.#options.global, {
            ...selection,
            provider: "github",
            applicationId: this.#options.github.id,
        });
    }

    /** Require the push permission for push access. */
    static async #authorizeWrite(call: RepositoryCall): Promise<void> {
        if (LeaseMode.parse(call.input.mode) === "write") {
            await call.authorization!.require(repository.permission("push"), call.reference());
        }
    }

    /** Update the repository with its prepared origin, at the prepared revision. */
    static async #update(
        call: RepositoryCall,
        next: (call?: RepositoryCall) => Promise<unknown>,
    ): Promise<unknown> {
        // change the repository alone when its origin stays
        const move = call.prepared as Move | null;
        if (move === null) {
            return next();
        }

        // require the revision the origin was prepared at
        if (call.target!.revision !== move.revision) {
            throw new ServiceError("CONFLICT", { message: "repository revision has changed" });
        }

        return next(call.with({ input: { ...call.input, ...move.columns } }));
    }

    /** Record a listing's references and default branch. */
    static async #record(call: RepositoryCall, listing: GitListing): Promise<unknown> {
        // read the recorded references by name
        const { database, now } = call;
        const target = call.target!;
        const rows = await database
            .select()
            .from(reference.table)
            .where(eq(reference.table.parentId, target.id));
        const recorded = new Map(rows.map((row) => [row.name, row]));
        const listed = new Map(listing.references.map((entry) => [entry.name, entry]));

        // create new references
        // NOTE #Performance: one invoked call per reference; a first refresh of 10k tags runs 10k calls
        for (const entry of listing.references) {
            if (!recorded.has(entry.name)) {
                await call.invoke(reference, "create", {
                    parentId: target.id,
                    name: entry.name,
                    object: entry.object,
                    commit: entry.commit,
                    observedAt: now,
                });
            }
        }

        // update the references that changed, at their read revisions
        for (const row of rows) {
            const entry = listed.get(row.name);
            const isGone = entry === undefined && row.deletedAt === null;
            const isChanged =
                entry !== undefined &&
                (row.object !== entry.object ||
                    row.commit !== entry.commit ||
                    row.deletedAt !== null);

            // mark a vanished reference deleted
            if (isGone) {
                await call.invoke(reference, "update", {
                    id: row.id,
                    revision: row.revision,
                    deletedAt: now,
                });
            }
            // move or return a changed reference
            else if (isChanged) {
                await call.invoke(reference, "update", {
                    id: row.id,
                    revision: row.revision,
                    object: entry.object,
                    commit: entry.commit,
                    observedAt: now,
                    deletedAt: null,
                });
            }
        }

        // move the default branch when it changed
        return target.defaultReference === listing.defaultReference
            ? target
            : call.update({ defaultReference: listing.defaultReference });
    }

    /** Read a repository's origin columns. */
    static #columns(target: Repository): OriginColumns {
        return Object.fromEntries(
            ORIGIN_COLUMNS.map((name) => [name, target[name]]),
        ) as OriginColumns;
    }

    /** Read the origin from a call's fields and refuse invalid combinations. */
    static #origin(fields: Readonly<Record<string, unknown>>): RepositoryOrigin {
        // parse the call's origin fields and read a host's subject from its key
        const named = Object.fromEntries(
            ORIGIN_FIELDS.filter((name) => fields[name] !== undefined && fields[name] !== null).map(
                (name) => [
                    name,
                    name === "host" ? Subject.read(String(fields[name])) : fields[name],
                ],
            ),
        );
        const origin = RepositoryOrigin.safeParse(named);
        if (!origin.success) {
            throw new ServiceError("BAD_REQUEST", {
                message: "repository origin names no platform, github, git or host origin",
                data: { issues: origin.error.issues },
            });
        }

        return origin.data;
    }
}
