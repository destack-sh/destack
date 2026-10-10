import { schema, Clock } from "@destack/schema";
import type { Fetch } from "@destack/service";
import type { Token } from "../credential/credential.ts";
import { ConnectionError } from "../error/error.ts";
import type { InstallationGrant, InstallationIssuer } from "../installation/installation.ts";
import type { Integration } from "../object/integration.ts";
import { Assertion } from "../oauth/assertion.ts";

/** The REST API version the requests send. */
const API_VERSION = "2022-11-28";

/** The user agent GitHub requires on every API request. */
const USER_AGENT = "Destack";

/** GitHub's answer to an installation access token request. */
const InstallationToken = schema.looseObject({
    token: schema.string().min(1),
    expires_at: schema.string().min(1),
});

/** GitHub Apps installed on accounts, issuing installation access tokens with the app's private key, and the REST API they call. */
export class GitHubIssuer implements InstallationIssuer {
    /** The GitHub instance's origin, such as https://github.com. */
    readonly issuer: string;
    /** The REST API's base URL, such as https://api.github.com. */
    readonly api: string;
    /** Send requests to GitHub. */
    readonly #fetch: Fetch;
    /** The clock the app's JWTs are issued by. */
    readonly #clock: Clock;

    /** Call github.com, or a GitHub Enterprise Server at its origin and API, on the system clock unless given another. */
    constructor(options: {
        readonly issuer?: string;
        readonly api?: string;
        readonly fetch: Fetch;
        readonly clock?: Clock;
    }) {
        // keep the instance, the fetch and the clock
        this.issuer = options.issuer ?? "https://github.com";
        this.api = options.api ?? "https://api.github.com";
        this.#fetch = options.fetch;
        this.#clock = options.clock ?? Clock.system;
    }

    /** Write the app's installation page, returning with the state. */
    installationUrl(integration: Pick<Integration, "clientId" | "endpoints">, state: string): URL {
        // require the installation page and return to it with the state
        const page = integration.endpoints.installation;
        if (page === undefined) {
            throw new ConnectionError(
                "INVALID_AUTHORIZATION",
                `integration ${integration.clientId} has no installation page`,
            );
        }
        const url = new URL(page);
        url.searchParams.set("state", state);

        return url;
    }

    /** Read the installation identifier of GitHub's setup redirect. */
    installation(parameters: Readonly<Record<string, string>>): string {
        const installation = parameters["installation_id"];
        if (installation === undefined || !/^\d+$/u.test(installation)) {
            throw new ConnectionError(
                "INVALID_AUTHORIZATION",
                "the installation redirect has no installation",
            );
        }

        return installation;
    }

    /** Issue an installation access token, limited to some repositories by identifier or name and some permissions when granted. */
    async token(
        clientId: string,
        installation: string,
        privateKey: string,
        grant: InstallationGrant = {},
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
            await appToken(clientId, privateKey, this.#clock.now()),
            body,
        );
        const answered = InstallationToken.parse(await response.json());

        return {
            accessToken: answered.token,
            tokenType: "token",
            expiresAt: Date.parse(answered.expires_at),
        };
    }

    /** Uninstall the app from the account. */
    async revoke(clientId: string, installation: string, privateKey: string): Promise<void> {
        const bearer = await appToken(clientId, privateKey, this.#clock.now());
        await this.request("DELETE", `/app/installations/${installation}`, bearer);
    }

    /** Send a REST API request with a bearer credential, the API version and the user agent, refusing an error answer. */
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
            throw new ConnectionError(
                "REFUSED",
                `GitHub answered ${response.status} to ${method} ${path}`,
            );
        }

        return response;
    }
}

/** Sign the app's JWT with its private key, issued now by its client identifier. */
function appToken(clientId: string, privateKey: string, now: number): Promise<string> {
    return Assertion.sign({ issuer: clientId, key: privateKey }, now);
}
