import { JsonWebToken } from "../token/token.ts";
import { aligned, schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import type { Lease, LeaseMode } from "@destack/resource";
import { GitCredential, type Fetch } from "../storage/index.ts";

/** How far back an app JWT dates its issue in seconds, for clock drift as GitHub advises. */
const DRIFT_SECONDS = 60;

/** How long an app JWT lives, in seconds, under the ten minutes GitHub accepts. */
const APP_TOKEN_TTL_SECONDS = 9 * 60;

/** The REST API version the requests name. */
const API_VERSION = "2022-11-28";

/** The host of github.com remotes. */
const GITHUB_HOST = "github.com";

/** The user name GitHub expects beside an installation token in Git basic authentication. */
const TOKEN_USER = "x-access-token";

/** The contents permission an installation token needs for each Git mode. */
const MODE_CONTENTS: Readonly<Record<LeaseMode, "read" | "write">> = {
    read: "read",
    write: "write",
};

/** The installation access token GitHub creates. */
const Token = schema.looseObject({ token: schema.string().min(1), expires_at: schema.string() });

/** The repository fields read. */
const Repository = schema.looseObject({ id: schema.number().int(), full_name: schema.string() });

/** An installation access token and its expiry. */
export interface GitHubToken {
    /** The token, sent as a bearer credential or as Git's basic password. */
    readonly token: string;
    /** The expiry in UTC epoch milliseconds. */
    readonly expiresAt: number;
}

/** A GitHub repository as the platform records it. */
export interface GitHubRepository {
    /** GitHub's stable numeric identifier, as text. */
    readonly id: string;
    /** The owner and name, such as octocat/hello-world. */
    readonly fullName: string;
}

/** The repositories an installation token opens, by identifier or by name. */
export type GitHubTokenScope =
    | { readonly ids: readonly string[] }
    | { readonly names: readonly string[] };

/** The repository permissions an installation token has. */
export interface GitHubPermissions {
    /** Reading or writing code for Git. */
    readonly contents?: "read" | "write";
    /** Reading the repository's metadata to identify it. */
    readonly metadata?: "read";
}

/** The GitHub App the platform opens repositories through, and the REST API it calls. */
export interface GitHubAppOptions {
    /** The app's client ID or app ID, the JWT issuer and the application connected accounts name. */
    readonly id: string;
    /** The app's RS256 private key. */
    readonly key: CryptoKey;
    /** The REST API base, https://api.github.com for github.com. */
    readonly api: URL;
    /** The fetch calling the API. */
    readonly fetch?: Fetch;
}

/** A GitHub App: app JWTs, installation access tokens, and the repositories installations open. */
export class GitHubApp {
    /** The app's client ID or app ID. */
    readonly id: string;
    /** The app's private key. */
    readonly #key: CryptoKey;
    /** The REST API base. */
    readonly #api: string;
    /** The fetch calling the API. */
    readonly #fetch: Fetch;

    /** Use an app's identity and key. */
    constructor(options: GitHubAppOptions) {
        // retain the identity, key and API
        this.id = options.id;
        this.#key = options.key;
        this.#api = options.api.href.replace(/\/$/u, "");
        this.#fetch = options.fetch ?? globalThis.fetch;
    }

    /** Read a github.com remote's owner and name. */
    static fullName(remote: string): string {
        // require a github.com path of an owner and a name
        const url = new URL(remote);
        const match = /^\/([^/]+)\/([^/]+?)(?:\.git)?\/?$/u.exec(url.pathname);
        if (url.host !== GITHUB_HOST || match === null) {
            throw new ServiceError("BAD_REQUEST", {
                message: `not a github.com repository: ${remote}`,
            });
        }

        return `${match[1]}/${match[2]}`;
    }

    /** Create an installation access token for some repositories and permissions: POST /app/installations/{id}/access_tokens. */
    async token(
        installationId: string,
        scope: GitHubTokenScope,
        permissions: GitHubPermissions,
    ): Promise<GitHubToken> {
        // sign the app JWT GitHub requires for app endpoints
        const now = Math.floor(Date.now() / 1000);
        const jwt = await JsonWebToken.sign(
            { iat: now - DRIFT_SECONDS, exp: now + APP_TOKEN_TTL_SECONDS, iss: this.id },
            this.#key,
        );

        // restrict the token to the repositories and permissions named
        const repositories =
            "ids" in scope
                ? { repository_ids: scope.ids.map(Number) }
                : { repositories: [...scope.names] };
        const created = Token.parse(
            await this.#json(`/app/installations/${installationId}/access_tokens`, jwt, {
                method: "POST",
                body: { ...repositories, permissions },
            }),
        );

        return { token: created.token, expiresAt: Date.parse(created.expires_at) };
    }

    /** Grant a Git client access to one repository through an installation token limited to the mode. */
    async open(
        installationId: string,
        repositoryId: string,
        remote: string,
        mode: LeaseMode,
    ): Promise<Lease> {
        const token = await this.token(
            installationId,
            { ids: [repositoryId] },
            { contents: MODE_CONTENTS[mode] },
        );

        return GitCredential.lease(
            { username: TOKEN_USER, password: token.token, expiresAt: token.expiresAt },
            remote,
            mode,
        );
    }

    /** Identify a repository an installation opens by its owner and name: GET /repos/{owner}/{repo}. */
    async repository(installationId: string, fullName: string): Promise<GitHubRepository> {
        // issue a metadata token for the repository by name
        const token = await this.token(
            installationId,
            { names: [aligned(fullName.split("/"), 1)] },
            { metadata: "read" },
        );

        // read its identifier
        const repository = Repository.parse(await this.#json(`/repos/${fullName}`, token.token));

        return { id: String(repository.id), fullName: repository.full_name };
    }

    /** Send an API request as GitHub expects it and read its JSON body, failing loudly on a refusal. */
    async #json(
        path: string,
        bearer: string,
        init: { readonly method?: "POST"; readonly body?: unknown } = {},
    ): Promise<unknown> {
        // set the API version, a user agent and the credential
        const headers = new Headers({
            Accept: "application/vnd.github+json",
            Authorization: `Bearer ${bearer}`,
            "User-Agent": "destack",
            "X-GitHub-Api-Version": API_VERSION,
        });
        if (init.body !== undefined) {
            headers.set("Content-Type", "application/json");
        }
        const response = await this.#fetch(`${this.#api}${path}`, {
            method: init.method ?? "GET",
            headers,
            ...(init.body === undefined ? {} : { body: JSON.stringify(init.body) }),
        });

        // report GitHub's refusal
        if (!response.ok) {
            const text = await response.text();
            throw new ServiceError("BAD_GATEWAY", {
                message: `github refused ${path}: ${response.status} ${text}`,
            });
        }

        return response.json();
    }
}
