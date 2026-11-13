import type { Environment } from "@destack/environment";
import { type Secret, SecretReading } from "@destack/resource";
import { GitHubIssuer } from "./issuer.ts";
import type { InstallationGrant, Token } from "@destack/connection";
import { nth, present, schema } from "@destack/schema";
import type { Fetch } from "@destack/object";
import { ForgeError } from "../error/index.ts";
import type { Lease, LeaseMode } from "@destack/resource";
import { GitCredential } from "@destack/git";
import { TelemetryScope } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";

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
    /** The REST API base, https://api.github.com for github.com. */
    readonly api: URL;
    /** The fetch calling the API. */
    readonly fetch: Fetch;
    /** The WebCrypto and randomness signing the app's tokens, and the telemetry of its calls. */
    readonly environment: Pick<Environment, "crypto" | "random" | "telemetry">;
}

/** A GitHub App: the repositories its installations open. */
export class GitHubApp {
    /** The app's client ID or app ID. */
    readonly id: string;
    /** The WebCrypto and randomness signing the app's tokens, and the telemetry of its calls. */
    readonly environment: Pick<Environment, "crypto" | "random" | "telemetry">;
    /** The app's private key. */
    readonly #key: Secret;
    /** The issuer of the installations' tokens, which also sends the REST API's requests. */
    readonly #issuer: GitHubIssuer;
    /** The forge's spans and durations. */
    readonly #scope: TelemetryScope;

    /** Use an app's identity and key. */
    constructor(options: GitHubAppOptions) {
        // take the identity and key, and call the API through the issuer
        this.id = options.id;
        this.#key = options.key;
        this.environment = options.environment;
        this.#issuer = new GitHubIssuer({
            api: options.api.href.replace(/\/$/u, ""),
            fetch: options.fetch,
            environment: options.environment,
        });
        this.#scope = TelemetryScope.of(import.meta.destack.package, options.environment.telemetry);
    }

    /** Read a github.com remote's owner and name. */
    static fullName(remote: string): string {
        // require a github.com path of an owner and a name
        const url = new URL(remote);
        const match = /^\/([^/]+)\/([^/]+?)(?:\.git)?\/?$/u.exec(url.pathname);
        if (url.host !== GITHUB_HOST || match === null) {
            throw ForgeError.of(
                "GITHUB_REPOSITORY_REQUIRED",
                `github.com repository is required: remote '${remote}' is not one`,
            );
        }

        return `${match[1]}/${match[2]}`;
    }

    /** Grant a Git client access to one repository through an installation token issued now. */
    async open(
        installationId: string,
        repositoryId: string,
        remote: string,
        mode: LeaseMode,
        now: number,
    ): Promise<Lease> {
        const token = await this.issue(
            installationId,
            { resources: { ids: [repositoryId] }, permissions: { contents: mode } },
            now,
        );

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

    /** Identify a repository an installation opens by its owner and name, now. */
    async repository(
        installationId: string,
        fullName: string,
        now: number,
    ): Promise<GitHubRepository> {
        return this.#scope.span(
            "destack.github_app.identify",
            { "destack.github.installation": installationId },
            async () => {
                // issue a metadata token for the repository by name
                const token = await this.issue(
                    installationId,
                    {
                        resources: { names: [nth(fullName.split("/"), 1)] },
                        permissions: { metadata: "read" },
                    },
                    now,
                );

                // read its identifier
                const path = `/repos/${fullName}`;
                const response = await this.#issuer.request("GET", path, token.accessToken);
                const repository = Repository.parse(await response.json());

                return { id: String(repository.id), fullName: repository.full_name };
            },
        );
    }

    /** Issue an installation's token now, narrowed to a grant. */
    issue(installationId: string, grant: InstallationGrant, now: number): Promise<Token> {
        return this.#scope.span(
            "destack.github_app.issue",
            { "destack.github.installation": installationId },
            async () => {
                const key = SecretReading.text(await this.#key.read());

                return this.#issuer.token(this.id, installationId, key, grant, now);
            },
        );
    }

    /** Remove the app from an installation's account now. */
    uninstall(installationId: string, now: number): Promise<void> {
        return this.#scope.span(
            "destack.github_app.uninstall",
            { "destack.github.installation": installationId },
            async () => {
                const key = SecretReading.text(await this.#key.read());
                await this.#issuer.revoke(this.id, installationId, key, now);
            },
        );
    }

    /** Read the installation GitHub's setup redirect returns with. */
    installation(parameters: Readonly<Record<string, string>>): string {
        return this.#issuer.installation(parameters);
    }
}
