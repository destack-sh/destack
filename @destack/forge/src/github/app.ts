import { Secret } from "@destack/vault";
import { GitHubIssuer } from "./issuer.ts";
import type { InstallationGrant, Token } from "@destack/connection";
import { nth, present, schema } from "@destack/schema";
import type { Fetch } from "@destack/runtime";
import { CallError } from "@destack/runtime/error";
import type { Lease, LeaseMode } from "@destack/resource";
import { GitCredential } from "../storage/index.ts";
import type { Clock } from "@destack/environment";

/** The host of github.com remotes. */
const GITHUB_HOST = "github.com";

/** The REST API base of github.com, which GitHub Apps call. */
export const GITHUB_API = new URL("https://api.github.com");

/** The user name GitHub expects beside an installation token in Git basic authentication. */
const TOKEN_USER = "x-access-token";

/** The fields of a GitHub repository the app reads. */
const Repository = schema.looseObject({ id: schema.number().int(), full_name: schema.string() });

/** A GitHub repository as the platform records it. */
export interface GitHubRepository {
    /** GitHub's stable numeric identifier, as text. */
    readonly id: string;
    /** The owner and name, such as octocat/hello-world. */
    readonly fullName: string;
}

/** The GitHub App the platform opens repositories through, and the REST API it calls. */
export interface GitHubAppOptions {
    /** The app's client ID or app ID, the JWT issuer. */
    readonly id: string;
    /** The app's private key, a PKCS #1 or PKCS #8 PEM. */
    readonly key: Secret;
    /** The secret GitHub signs the app's webhook deliveries with. */
    readonly secret: Secret;
    /** The REST API base, https://api.github.com for github.com. */
    readonly api: URL;
    /** The fetch calling the API. */
    readonly fetch?: Fetch;
    /** The clock the app's tokens expire by. */
    readonly clock: Clock;
}

/** A GitHub App: the repositories its installations open. */
export class GitHubApp {
    /** The app's client ID or app ID. */
    readonly id: string;
    /** The secret GitHub signs the app's webhook deliveries with. */
    readonly secret: Secret;
    /** The app's private key. */
    readonly #key: Secret;
    /** The issuer of the installations' tokens, which also sends the REST API's requests. */
    readonly #issuer: GitHubIssuer;

    /** Use an app's identity and key. */
    constructor(options: GitHubAppOptions) {
        // take the identity, secret and key, and call the API through the issuer
        this.id = options.id;
        this.secret = options.secret;
        this.#key = options.key;
        this.#issuer = new GitHubIssuer({
            api: options.api.href.replace(/\/$/u, ""),
            fetch: options.fetch ?? globalThis.fetch,
            clock: options.clock,
        });
    }

    /** Read a github.com remote's owner and name. */
    static fullName(remote: string): string {
        // require a github.com path of an owner and a name
        const url = new URL(remote);
        const match = /^\/([^/]+)\/([^/]+?)(?:\.git)?\/?$/u.exec(url.pathname);
        if (url.host !== GITHUB_HOST || match === null) {
            throw new CallError("BAD_REQUEST", {
                message: `not a github.com repository: ${remote}`,
            });
        }

        return `${match[1]}/${match[2]}`;
    }

    /** Grant a Git client access to one repository through an installation token. */
    async open(
        installationId: string,
        repositoryId: string,
        remote: string,
        mode: LeaseMode,
    ): Promise<Lease> {
        const token = await this.issue(installationId, {
            resources: { ids: [repositoryId] },
            permissions: { contents: mode },
        });

        return GitCredential.lease(
            {
                username: TOKEN_USER,
                password: token.accessToken,
                expiresAt: present(token.expiresAt, "an installation token's expiry"),
            },
            remote,
            mode,
        );
    }

    /** Identify a repository an installation opens by its owner and name. */
    async repository(installationId: string, fullName: string): Promise<GitHubRepository> {
        // issue a metadata token for the repository by name
        const token = await this.issue(installationId, {
            resources: { names: [nth(fullName.split("/"), 1)] },
            permissions: { metadata: "read" },
        });

        // read its identifier
        const path = `/repos/${fullName}`;
        const response = await this.#issuer.request("GET", path, token.accessToken);
        const repository = Repository.parse(await response.json());

        return { id: String(repository.id), fullName: repository.full_name };
    }

    /** Issue an installation's token, narrowed to a grant. */
    async issue(installationId: string, grant: InstallationGrant = {}): Promise<Token> {
        return this.#issuer.token(this.id, installationId, await Secret.text(this.#key), grant);
    }

    /** Remove the app from an installation's account. */
    async uninstall(installationId: string): Promise<void> {
        await this.#issuer.revoke(this.id, installationId, await Secret.text(this.#key));
    }

    /** Read the installation GitHub's setup redirect returns with. */
    installation(parameters: Readonly<Record<string, string>>): string {
        return this.#issuer.installation(parameters);
    }
}
