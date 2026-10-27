import { type Select, Snapshot } from "@destack/db";
import { BATCH_LIMIT, type MethodContextOf, ObjectWatch } from "@destack/object";
import { connection } from "@destack/connection";
import { present } from "@destack/schema";
import type { Runner } from "@destack/connection/server";
import type { Lease, LeaseMode } from "@destack/resource";
import type { Fetch } from "@destack/object";
import { ForgeError } from "../error/index.ts";

import { SecretReading } from "@destack/vault";
import { secret, vault } from "@destack/vault/object";
import { type Call, ObjectReference } from "@destack/sync";
import type { GitHubHosting } from "../github/index.ts";
import {
    type GitRemoteCredential,
    ORIGIN_FIELDS,
    OriginColumns,
    type OriginMove,
    reference,
    repository,
    RepositoryOrigin,
} from "../object/index.ts";
import {
    GitAdvertisement,
    GitCredential,
    type GitListing,
    type GitReference,
    type GitStorage,
} from "../storage/index.ts";

/** A repository's row. */
type Row = Select<typeof repository.table>;

/** A call recording a listing of a repository's references. */
type RecordCall =
    | MethodContextOf<typeof repository, "refresh">
    | MethodContextOf<typeof repository, "report">;

/** Where repositories live: the platform's storage, GitHub and Git remotes. */
export interface RepositoryOptions {
    /** The storage of platform repositories. */
    readonly storage: GitStorage;
    /** The repositories hosted at GitHub, absent in a universe running no GitHub App. */
    readonly github: GitHubHosting | undefined;
    /** The fetch calling Git remotes. */
    readonly fetch: Fetch;
    /** The forge's own vault, which stores the credentials of Git remotes. */
    readonly vault: ObjectReference;
}

/** Implement repositories. */
export function implementRepository(options: RepositoryOptions): typeof repository {
    const hosting = new RepositoryHosting(options);
    const { github } = options;

    return repository.implement({
        methods: {
            create: {
                prepare: async (context) => {
                    // require a credential exactly for a remote authenticating by credential
                    const origin = RepositoryOrigin.read(context.input);
                    RepositoryHosting.requireCredential(origin, context.input.credential);

                    return hosting.create(context.id, origin);
                },
                handle: async (context, next) => {
                    // record the repository, then store its credential in the forge's vault
                    const created = await next(
                        context.with({ input: { ...context.input, ...context.prepared } }),
                    );
                    const { credential } = context.input;
                    if (credential !== undefined) {
                        await context.mutate(context.self, [hosting.store(context.id, credential)]);
                    }

                    return created;
                },
                settle: async (context, settled) => {
                    // delete the storage a creation made before its record rolled back
                    if (settled.status === "rolledBack") {
                        await hosting.discard(context.requireId(), settled.prepared);
                    }
                },
            },
            open: {
                authorize: async (context) => {
                    // require push for push access, and for any lease of a stored credential, whatever the mode
                    const isCredential =
                        context.target.hosting === "git" &&
                        context.target.authentication === "credential";
                    if (context.input.mode === "write" || isCredential) {
                        await context.require(repository.permission("push"), context.reference());
                    }
                },
                prepare: (context) => hosting.lease(context.target, context.input.mode, context),
                handle: async (context) => context.prepared,
            },
            refresh: {
                prepare: (context) => hosting.list(context.target, context),
                handle: (context) => RepositoryHosting.record(context, context.prepared),
            },
            identify: {
                prepare: (context) => hosting.requireGitHub().identify(context.target, context),
                handle: (context) => context.update({ providerRepositoryId: context.prepared }),
            },
            report: (call) => RepositoryHosting.record(call, call.input),
            update: {
                prepare: async (context) => hosting.move(context),
                handle: async (context, next) => {
                    // require the revision a moved origin was prepared at
                    const move = context.prepared;
                    if (move !== null && context.target.revision !== move.revision) {
                        throw ForgeError.of("REVISION_STALE", "repository revision has changed");
                    }

                    // change the repository, and its origin when it moves
                    const updated = await next(
                        move === null
                            ? context
                            : context.with({ input: { ...context.input, ...move.columns } }),
                    );

                    // store a new credential, or remove the credential of an origin moving off it
                    const id = context.target.id;
                    const authentication =
                        move?.columns.authentication ?? context.target.authentication;
                    const { credential } = context.input;
                    if (credential !== undefined) {
                        await context.mutate(context.self, [hosting.store(id, credential)]);
                    } else if (
                        context.target.authentication === "credential" &&
                        authentication !== "credential"
                    ) {
                        await context.mutate(context.self, [hosting.remove(id)]);
                    }

                    return updated;
                },
            },
            purge: {
                prepare: async (context) => OriginColumns.of(context.target),
                handle: async (context, next) => {
                    // remove the stored credential with the repository
                    if (context.target.authentication === "credential") {
                        await context.mutate(context.self, [hosting.remove(context.target.id)]);
                    }

                    return next();
                },
                settle: async (_call, settled) => {
                    // delete the storage once the purge committed
                    if (settled.status === "committed") {
                        await hosting.delete(settled.prepared);
                    }
                },
            },
        },
        // identify each live GitHub repository at GitHub once the forge has its connection's replica
        ...(github === undefined
            ? {}
            : {
                  controller: {
                      pending: {
                          hosting: "github",
                          providerRepositoryId: { isNull: true },
                          deletedAt: { isNull: true },
                      },
                      // identify an account's waiting repositories again as their connection's replica changes
                      watches: [
                          ObjectWatch.of(connection.table, async (row, database) => {
                              const rows = await Snapshot.live(database)
                                  .read(repository.table)
                                  .findMany({
                                      where: { connectionId: row.id, hosting: "github" },
                                  });

                              return rows.map((each) => ({ id: each.id }));
                          }),
                      ],
                      reconcile: async (reconciliation): Promise<undefined> => {
                          for (const row of reconciliation.rows) {
                              // identify the repository once its connection's replica names an installation
                              const reader = Snapshot.live(reconciliation.database);
                              const connectionId = present(
                                  row.connectionId,
                                  `the connection of GitHub repository ${row.id}`,
                              );
                              const found = await github.installation(
                                  reader,
                                  row.accountId,
                                  connectionId,
                              );
                              if (found !== undefined) {
                                  await reconciliation.mutate([
                                      repository.call("identify", repository.target(row)),
                                  ]);
                              }
                          }

                          return undefined;
                      },
                  },
              }),
    });
}

/** Where the forge's repositories store their Git data: its storage, GitHub or a Git remote. */
class RepositoryHosting {
    /** The repositories hosted at GitHub, absent in a universe running no GitHub App. */
    readonly github: GitHubHosting | undefined;
    /** The storage of platform repositories. */
    readonly #storage: GitStorage;
    /** The fetch calling Git remotes. */
    readonly #fetch: Fetch;
    /** The forge's own vault, which stores the credentials of Git remotes. */
    readonly #vault: ObjectReference;

    /** Host repositories in a storage, at GitHub and at Git remotes. */
    constructor(options: RepositoryOptions) {
        // take the storage, GitHub, the fetch and the vault
        this.github = options.github;
        this.#storage = options.storage;
        this.#fetch = options.fetch;
        this.#vault = options.vault;
    }

    /** Require a credential exactly for a Git remote authenticating by credential. */
    static requireCredential(origin: RepositoryOrigin, credential: unknown): void {
        const isCredential = origin.hosting === "git" && origin.authentication === "credential";
        if (isCredential !== (credential !== undefined)) {
            throw ForgeError.of(
                "CREDENTIAL_MISMATCH",
                isCredential
                    ? "a git remote authenticating by credential needs its credential"
                    : "only a git remote authenticating by credential takes a credential",
            );
        }
    }

    /** Store a repository's credential in the forge's vault, readable by the forge alone. */
    store(id: string, credential: GitRemoteCredential): Call {
        return vault.call("put", {
            ...vault.target(this.#vault),
            secret: RepositoryHosting.#secret(id),
            name: id,
            value: { encoding: "text", value: `${credential.user}:${credential.password}` },
        });
    }

    /** Remove a repository's credential from the forge's vault. */
    remove(id: string): Call {
        return vault.call("remove", {
            ...vault.target(this.#vault),
            secret: RepositoryHosting.#secret(id),
        });
    }

    /** Derive the vault secret of a repository's credential from its identifier. */
    static #secret(id: string): ReturnType<typeof secret.identifier> {
        return secret.identifier(`secret-${id.slice(id.indexOf("-") + 1)}`);
    }

    /** Read the columns of a new origin, creating a platform repository in storage. */
    async create(id: string, origin: RepositoryOrigin): Promise<OriginColumns> {
        // create a platform repository in storage before its record commits, refusing while its storage fails
        if (origin.hosting === "platform") {
            const remote = await this.#storage.create(id).catch((error: unknown) => {
                throw ForgeError.of("STORAGE_UNAVAILABLE", "repository storage is unavailable", {
                    cause: error,
                });
            });

            return {
                ...OriginColumns.blank,
                hosting: "platform",
                provider: this.#storage.provider,
                providerRepositoryId: id,
                remote,
            };
        }
        // record a machine by its identifier
        else if (origin.hosting === "machine") {
            return { ...OriginColumns.blank, hosting: "machine", machineId: origin.machineId };
        }
        // take a GitHub or Git remote as given, a GitHub repository for its controller to identify
        else {
            return { ...OriginColumns.blank, ...origin };
        }
    }

    /** Prepare the origin an update moves to, none when it changes none of its fields. */
    move(context: MethodContextOf<typeof repository, "update", unknown>): OriginMove {
        // leave the origin when the update changes none of its fields
        const target = context.target;
        const isMoved = ORIGIN_FIELDS.some((name) => context.input[name] !== undefined);
        if (!isMoved) {
            return null;
        }

        // move GitHub and Git repositories only, between GitHub and Git origins
        if (target.hosting !== "github" && target.hosting !== "git") {
            throw ForgeError.of(
                "IMMOVABLE_REPOSITORY",
                `${target.hosting} repositories cannot move`,
            );
        }
        const merged = Object.fromEntries(
            ORIGIN_FIELDS.map((name) => [
                name,
                context.input[name] === undefined ? target[name] : context.input[name],
            ]),
        );
        const moved = RepositoryOrigin.read(merged);
        if (moved.hosting !== "github" && moved.hosting !== "git") {
            throw ForgeError.of(
                "IMMOVABLE_REPOSITORY",
                `${target.hosting} repositories cannot move to ${moved.hosting} origins`,
            );
        }

        return { revision: target.revision, columns: { ...OriginColumns.blank, ...moved } };
    }

    /** List an origin's references through storage or Git's advertisement. */
    async list(
        target: Row,
        runner: Pick<Runner, "query" | "self" | "read" | "now">,
    ): Promise<GitListing> {
        // list a platform repository in the storage
        if (target.hosting === "platform") {
            return this.#storage.references(this.#storageId(target));
        }
        // read the advertisement of other remotes with read access
        else {
            return GitAdvertisement.read(await this.lease(target, "read", runner), this.#fetch);
        }
    }

    /** Lease an origin to read, or to write as well. */
    async lease(
        target: Row,
        mode: LeaseMode,
        runner: Pick<Runner, "query" | "self" | "read" | "now">,
    ): Promise<Lease> {
        // lease a platform repository through the storage
        if (target.hosting === "platform") {
            return this.#storage.open(this.#storageId(target), mode);
        }
        // lease a GitHub repository through an installation token limited to it and the mode
        else if (target.hosting === "github") {
            return this.requireGitHub().open(target, mode, runner);
        }
        // leave machine repositories to their machine
        else if (target.hosting === "machine") {
            throw ForgeError.of(
                "MACHINE_REPOSITORY",
                "machine repositories are refreshed and opened through their machine",
            );
        }
        // refuse pushing to an anonymous remote, for which the platform has no credential
        else if (target.authentication === "anonymous" && mode === "write") {
            throw ForgeError.of("ANONYMOUS_PUSH", "anonymous remotes do not take push credentials");
        }
        // pull an anonymous remote unchanged
        else if (target.authentication === "anonymous") {
            return { url: OriginColumns.requireRemote(target), mode, headers: {} };
        }
        // lease a remote with the credential the forge reads from its vault, which its open requires push for
        else {
            return {
                url: OriginColumns.requireRemote(target),
                mode,
                headers: await this.#basic(target, runner),
            };
        }
    }

    /** Delete the storage of a rolled back creation, its columns unread when unprepared. */
    async discard(id: string, columns: OriginColumns | undefined): Promise<void> {
        if (columns === undefined || columns.hosting === "platform") {
            await this.#storage.delete(id);
        }
    }

    /** Delete a purged platform repository from storage. */
    async delete(columns: OriginColumns): Promise<void> {
        if (columns.hosting === "platform") {
            await this.#storage.delete(this.#storageId(columns));
        }
    }

    /** Read the repositories hosted at GitHub, refused in a universe running no GitHub App. */
    requireGitHub(): GitHubHosting {
        if (this.github === undefined) {
            throw ForgeError.of(
                "GITHUB_APP_MISSING",
                "this universe has no github app for github repositories",
            );
        }

        return this.github;
    }

    /** Read a remote's stored `user:password` credential as Basic authentication headers. */
    async #basic(
        target: Row,
        runner: Pick<Runner, "query" | "self" | "read" | "now">,
    ): Promise<Readonly<Record<string, string>>> {
        // read the credential's current version through the forge's vault, as its consumer
        const stored = { scope: this.#vault.scope, id: RepositoryHosting.#secret(target.id) };
        const read = secret.call("read", secret.target(stored));
        const { value } = SecretReading.parse(await runner.query(runner.self, read));
        if (value.encoding !== "text") {
            throw ForgeError.of(
                "SECRET_TEXT_MISSING",
                `the secret of repository ${target.id} has no text value`,
            );
        }
        const credential = value.value;

        // require a user and a password, as Basic authentication sends them
        const separator = credential.indexOf(":");
        if (separator < 1) {
            throw ForgeError.of(
                "CREDENTIAL_INVALID",
                `the secret of repository ${target.id} is not a user:password credential`,
            );
        }

        return GitCredential.basic(credential.slice(0, separator), credential.slice(separator + 1));
    }

    /** Read a platform repository's identifier in the storage. */
    #storageId(columns: Pick<OriginColumns, "provider" | "providerRepositoryId">): string {
        // refuse repositories of another storage
        if (columns.provider !== this.#storage.provider) {
            throw ForgeError.of(
                "FOREIGN_STORAGE",
                `repository is stored by ${columns.provider}, but this forge uses ${this.#storage.provider}`,
            );
        }
        // refuse a platform repository without its storage identifier
        else if (columns.providerRepositoryId === null) {
            throw new TypeError("platform repository has no storage identifier");
        }

        return columns.providerRepositoryId;
    }

    /** Record a listing's references and default branch. */
    static async record(call: RecordCall, listing: GitListing): Promise<Row> {
        // read the recorded references by name
        const { target } = call;
        const rows = await call.read(reference).findMany({ where: { parentId: target.id } });
        const recorded = new Set(rows.map((row) => row.name));
        const listed = new Map(listing.references.map((entry) => [entry.name, entry]));

        // create the new references a batch at a time, and update the references that changed
        const fresh = listing.references.filter((entry) => !recorded.has(entry.name));
        const created = Array.from({ length: Math.ceil(fresh.length / BATCH_LIMIT) }, (_, batch) =>
            RepositoryHosting.#created(
                call,
                fresh.slice(batch * BATCH_LIMIT, (batch + 1) * BATCH_LIMIT),
            ),
        );
        const changed = rows.flatMap((row) =>
            RepositoryHosting.#changed(call, row, listed.get(row.name)),
        );
        const updated = Array.from(
            { length: Math.ceil(changed.length / BATCH_LIMIT) },
            (_, batch) =>
                reference.call("updateMany", {
                    items: changed.slice(batch * BATCH_LIMIT, (batch + 1) * BATCH_LIMIT),
                }),
        );
        await call.mutate(call.caller, [...created, ...updated]);

        // move the default branch when it changed
        return target.defaultReference === listing.defaultReference
            ? target
            : call.update({ defaultReference: listing.defaultReference });
    }

    /** Build the call creating references a listing has for the first time. */
    static #created(call: RecordCall, entries: readonly GitReference[]): Call {
        return reference.call("createMany", {
            items: entries.map((entry) => ({
                parentId: call.target.id,
                name: entry.name,
                object: entry.object,
                commit: entry.commit,
                observedAt: call.now,
            })),
        });
    }

    /** Build the update item of a recorded reference the listing moved, returned or dropped. */
    static #changed(
        call: RecordCall,
        row: Select<typeof reference.table>,
        entry: GitReference | undefined,
    ): {
        readonly id: string;
        readonly revision: number;
        readonly object: string;
        readonly commit: string | null;
        readonly observedAt: number;
        readonly deletedAt: number | null;
    }[] {
        // compare the listing with the recorded reference
        const isGone = entry === undefined && row.deletedAt === null;
        const isChanged =
            entry !== undefined &&
            (row.object !== entry.object || row.commit !== entry.commit || row.deletedAt !== null);

        // mark a vanished reference deleted, at its read revision
        if (isGone) {
            return [
                {
                    id: row.id,
                    revision: row.revision,
                    object: row.object,
                    commit: row.commit,
                    observedAt: row.observedAt,
                    deletedAt: call.now,
                },
            ];
        }
        // move or return a changed reference, at its read revision
        else if (isChanged) {
            return [
                {
                    id: row.id,
                    revision: row.revision,
                    object: entry.object,
                    commit: entry.commit,
                    observedAt: call.now,
                    deletedAt: null,
                },
            ];
        }
        // leave an unchanged reference
        else {
            return [];
        }
    }
}
