import { schema } from "@destack/schema";
import type { Fetch } from "@destack/object";
import type { Environment } from "@destack/environment";
import { ConnectionError, type InstallationGrant, type Token } from "@destack/connection";
import { Assertion } from "@destack/connection/oauth";

/** The REST API version the requests send. */
const API_VERSION = "2022-11-28";

/** The user agent GitHub requires on every API request. */
const USER_AGENT = "Destack";

/** GitHub's answer to an installation access token request. */
const InstallationToken = schema.looseObject({
    token: schema.string().min(1),
    expires_at: schema.string().min(1),
});

/** GitHub Apps installed on accounts, and the REST API they call. */
export class GitHubIssuer {
    /** The REST API's base URL, such as https://api.github.com. */
    readonly api: string;
    /** Send requests to GitHub. */
    readonly #fetch: Fetch;
    /** The WebCrypto and randomness signing the app's tokens. */
    readonly #environment: Pick<Environment, "crypto" | "random">;

    /** Call github.com or a GitHub Enterprise Server. */
    constructor(options: {
        readonly api?: string;
        readonly fetch: Fetch;
        readonly environment: Pick<Environment, "crypto" | "random">;
    }) {
        // take the API, the fetch and the environment signing the app's tokens
        this.api = options.api ?? "https://api.github.com";
        this.#fetch = options.fetch;
        this.#environment = options.environment;
    }

    /** Read the installation identifier of GitHub's setup redirect. */
    installation(parameters: Readonly<Record<string, string>>): string {
        const installation = parameters["installation_id"];
        if (installation === undefined || !/^\d+$/u.test(installation)) {
            throw ConnectionError.of(
                "AUTHORIZATION_INVALID",
                "the installation redirect has no installation",
            );
        }

        return installation;
    }

    /** Issue an installation access token now, limited to a grant's repositories and rights. */
    async token(
        clientId: string,
        installation: string,
        privateKey: string,
        grant: InstallationGrant,
        now: number,
    ): Promise<Token> {
        // limit the token to the granted repositories and permissions
        const { resources, permissions } = grant;
        const ids =
            resources !== undefined && "ids" in resources
                ? { repository_ids: resources.ids.map(Number) }
                : {};
        const names =
            resources !== undefined && "names" in resources
                ? { repositories: [...resources.names] }
                : {};
        const body = { ...ids, ...names, ...(permissions === undefined ? {} : { permissions }) };

        // request the token as the app
        const path = `/app/installations/${installation}/access_tokens`;
        const response = await this.request(
            "POST",
            path,
            await this.#appToken(clientId, privateKey, now),
            body,
        );
        const answered = InstallationToken.parse(await response.json());

        return {
            accessToken: answered.token,
            tokenType: "token",
            expiresAt: Date.parse(answered.expires_at),
        };
    }

    /** Uninstall the app from the account now. */
    async revoke(
        clientId: string,
        installation: string,
        privateKey: string,
        now: number,
    ): Promise<void> {
        const bearer = await this.#appToken(clientId, privateKey, now);
        await this.request("DELETE", `/app/installations/${installation}`, bearer);
    }

    /** Sign the app's JWT with its private key, issued now by its client identifier. */
    #appToken(clientId: string, privateKey: string, now: number): Promise<string> {
        return Assertion.sign({ issuer: clientId, key: privateKey }, now, this.#environment);
    }

    /** Send a REST API request with a bearer, API version and user agent, refusing errors. */
    async request(method: string, path: string, bearer: string, body?: unknown): Promise<Response> {
        // send the API version, the user agent and the credential
        const response = await this.#fetch(
            new Request(`${this.api}${path}`, {
                method,
                headers: {
                    accept: "application/vnd.github+json",
                    "user-agent": USER_AGENT,
                    authorization: `Bearer ${bearer}`,
                    "x-github-api-version": API_VERSION,
                    ...(body === undefined ? {} : { "content-type": "application/json" }),
                },
                ...(body === undefined ? {} : { body: JSON.stringify(body) }),
            }),
        );

        // refuse an error answer
        if (!response.ok) {
            throw ConnectionError.of(
                "SERVER_FAILED",
                `GitHub responded with ${response.status} to ${method} ${path}`,
            );
        }

        return response;
    }
}
