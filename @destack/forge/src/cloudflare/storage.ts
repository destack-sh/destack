import type { Connector, Lease, LeaseMode } from "@destack/resource";
import { schema } from "@destack/schema";
import type { Fetch } from "@destack/object";
import { ForgeError } from "../error/index.ts";
import { GitAdvertisement } from "../storage/advertisement.ts";
import type { GitStorageDeclaration } from "../storage/kind.ts";
import type { GitListing, GitStorage } from "../storage/storage.ts";

/** The provider code of Git storages in Cloudflare Artifacts. */
export const ARTIFACTS_PROVIDER = "cloudflare-artifacts";

/** How long a Git token lives, in seconds: an hour, as GitHub installation tokens do. */
const TOKEN_TTL_SECONDS = 3600;

/** The separator between a storage's prefix and a repository's identifier in an Artifacts name. */
const NAME_SEPARATOR = ".";

/** The most repositories one list page answers, the binding's maximum. */
const LIST_LIMIT = 200;

/** A repository as Artifacts describes it: its name and its credential-free Git remote. */
const ArtifactsRepository = schema.looseObject({
    name: schema.string().min(1),
    remote: schema.url(),
});

/** A Git token as Artifacts issues it: the token and its ISO 8601 expiry. */
const ArtifactsToken = schema.looseObject({
    plaintext: schema.string().min(1),
    expiresAt: schema.string().min(1),
});

// TODO #Incomplete: verify on the live service once access exists: handle disposal, name length limits, get right after create, Bearer leases for push and fetch
/** An Artifacts namespace binding, with the methods of the Workers binding the storage calls. */
export interface ArtifactsNamespace {
    /** Create a repository, refusing a name in use with ALREADY_EXISTS. */
    readonly create: (name: string) => Promise<{ readonly name: string; readonly remote: string }>;
    /** Open a repository's handle, refusing a missing one with NOT_FOUND. */
    readonly get: (name: string) => Promise<ArtifactsRepositoryHandle>;
    /** List a page of repositories, with the cursor of the next page while one follows. */
    readonly list: (options: { readonly limit: number; readonly cursor?: string }) => Promise<{
        readonly repos: readonly { readonly name: string }[];
        readonly cursor?: string;
    }>;
    /** Delete a repository and its tokens, answering false for a missing one. */
    readonly delete: (name: string) => Promise<boolean>;
}

/** A handle of one Artifacts repository. */
export interface ArtifactsRepositoryHandle extends Disposable {
    /** The credential-free Git remote. */
    readonly remote: string;
    /** Issue a Git token with a scope that lives for some seconds. */
    readonly createToken: (
        scope: "read" | "write",
        ttl: number,
    ) => Promise<{ readonly plaintext: string; readonly expiresAt: string }>;
}

/** Repositories stored in an Artifacts namespace binding, named below a prefix. */
export class ArtifactsStorage implements GitStorage {
    /** The provider name repositories stored here record. */
    readonly provider = ARTIFACTS_PROVIDER;
    /** The namespace binding. */
    readonly #namespace: ArtifactsNamespace;
    /** The prefix of the storage's repository names, its resource's reference. */
    readonly #prefix: string;
    /** The fetch reading Git's advertisement. */
    readonly #fetch: Fetch;

    /** Store repositories in a namespace below a name prefix. */
    constructor(namespace: ArtifactsNamespace, prefix: string, fetch: Fetch) {
        this.#namespace = namespace;
        this.#prefix = prefix;
        this.#fetch = fetch;
    }

    /** Build the connector opening the storages a host binds to a namespace. */
    static connector(
        namespace: ArtifactsNamespace,
        fetch: Fetch,
    ): Connector<GitStorage, GitStorageDeclaration> {
        return {
            code: ARTIFACTS_PROVIDER,
            connect: async (binding) =>
                Object.assign(new ArtifactsStorage(namespace, binding.reference, fetch), {
                    [Symbol.asyncDispose]: async () => {},
                }),
        };
    }

    /** Build the Artifacts name of a repository below a prefix. */
    static name(prefix: string, id: string): string {
        return `${prefix}${NAME_SEPARATOR}${id}`;
    }

    /** Create an empty repository and answer the remote Artifacts gives it. */
    async create(id: string): Promise<string> {
        // create the repository under its name
        const name = ArtifactsStorage.name(this.#prefix, id);
        const created = ArtifactsRepository.parse(await this.#namespace.create(name));

        // refuse a repository created under another name
        if (created.name !== name) {
            throw ForgeError.of(
                "ARTIFACT_NAME_MISMATCH",
                `expected cloudflare artifacts to create ${name}, got ${created.name}`,
            );
        }

        return created.remote;
    }

    /** Delete a repository and its tokens, accepting one already gone. */
    async delete(id: string): Promise<void> {
        await this.#namespace.delete(ArtifactsStorage.name(this.#prefix, id));
    }

    /** Delete every repository below the prefix, listing them all before deleting any. */
    async deleteAll(): Promise<void> {
        // list the names below the prefix across every page, as the binding lists by no prefix
        const prefix = ArtifactsStorage.name(this.#prefix, "");
        const names: string[] = [];
        let cursor: string | undefined;
        do {
            const page = await this.#namespace.list({
                limit: LIST_LIMIT,
                ...(cursor === undefined ? {} : { cursor }),
            });
            names.push(
                ...page.repos.map((repo) => repo.name).filter((name) => name.startsWith(prefix)),
            );
            cursor = page.cursor;
        } while (cursor !== undefined);

        // delete them in parallel once listed, so no deletion moves a page
        await Promise.all(names.map((name) => this.#namespace.delete(name)));
    }

    /** List the branches and tags of Git's advertisement. */
    async references(id: string): Promise<GitListing> {
        return GitAdvertisement.read(await this.open(id, "read"), this.#fetch);
    }

    /** Lease a Git client a Bearer token scoped to the repository and mode for an hour. */
    async open(id: string, mode: LeaseMode): Promise<Lease> {
        // issue a token for the repository and mode at its remote
        using repository = await this.#namespace.get(ArtifactsStorage.name(this.#prefix, id));
        const remote = schema.url().parse(repository.remote);
        const issued = ArtifactsToken.parse(await repository.createToken(mode, TOKEN_TTL_SECONDS));

        // refuse an unreadable expiry
        const expiresAt = Date.parse(issued.expiresAt);
        if (Number.isNaN(expiresAt)) {
            throw ForgeError.of(
                "TOKEN_EXPIRY_INVALID",
                `cloudflare artifacts issued a token with an invalid expiry: ${issued.expiresAt}`,
            );
        }

        return {
            url: remote,
            mode,
            headers: { authorization: `Bearer ${issued.plaintext}` },
            expiresAt,
        };
    }
}
