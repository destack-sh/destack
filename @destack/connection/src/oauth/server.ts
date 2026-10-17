import { nth, Duration, schema } from "@destack/schema";
import type { Fetch } from "@destack/runtime";
import type { Token } from "../credential/credential.ts";
import { ConnectionError } from "../error/error.ts";
import type { AuthorizationEndpoints, ClientAuthentication } from "../object/integration.ts";
import { Assertion } from "./assertion.ts";
import { Clock } from "@destack/environment";

/** The grant type of a JWT assertion presented for a token (RFC 7523 2.1). */
export const JWT_BEARER = "urn:ietf:params:oauth:grant-type:jwt-bearer";

/** The grant type of a device code presented for a token (RFC 8628 3.4). */
export const DEVICE_CODE = "urn:ietf:params:oauth:grant-type:device_code";

/** The grant type of a token exchange (RFC 8693 2.1). */
export const TOKEN_EXCHANGE = "urn:ietf:params:oauth:grant-type:token-exchange";

/** The assertion type of a client authenticating with a signed JWT (RFC 7523 2.2). */
const CLIENT_ASSERTION_TYPE = "urn:ietf:params:oauth:client-assertion-type:jwt-bearer";

/** The seconds between device polls for a server that gives none (RFC 8628 3.2). */
const DEVICE_INTERVAL = 5;

/** A token response (RFC 6749 5.1). */
const TokenResponse = schema.looseObject({
    access_token: schema.string().min(1),
    token_type: schema.string().min(1),
    expires_in: schema.number().exactOptional(),
    refresh_token: schema.string().min(1).exactOptional(),
    scope: schema.string().exactOptional(),
});

/** An error response (RFC 6749 5.2). */
const ErrorResponse = schema.looseObject({
    error: schema.string().min(1),
    error_description: schema.string().exactOptional(),
});

/** A device authorization response (RFC 8628 3.2). */
const DeviceResponse = schema.looseObject({
    device_code: schema.string().min(1),
    user_code: schema.string().min(1),
    verification_uri: schema.httpUrl(),
    verification_uri_complete: schema.httpUrl().exactOptional(),
    interval: schema.number().int().positive().exactOptional(),
});

/** Protected resource metadata (RFC 9728 2). */
const ResourceMetadata = schema.looseObject({
    resource: schema.string().min(1),
    authorization_servers: schema.array(schema.string().min(1)).min(1),
});

/** Authorization server metadata (RFC 8414 2). */
const ServerMetadata = schema.looseObject({
    issuer: schema.string().min(1),
    authorization_endpoint: schema.url({ protocol: /^https$/u }).exactOptional(),
    token_endpoint: schema.url({ protocol: /^https$/u }).exactOptional(),
    revocation_endpoint: schema.url({ protocol: /^https$/u }).exactOptional(),
    device_authorization_endpoint: schema.url({ protocol: /^https$/u }).exactOptional(),
    registration_endpoint: schema.url({ protocol: /^https$/u }).exactOptional(),
    authorization_response_iss_parameter_supported: schema.boolean().default(false),
});

/** A client registration response (RFC 7591 3.2.1). */
const RegistrationResponse = schema.looseObject({
    client_id: schema.string().min(1),
    client_secret: schema.string().min(1).exactOptional(),
    token_endpoint_auth_method: schema
        .enum(["none", "client_secret_basic", "client_secret_post", "private_key_jwt"])
        .exactOptional(),
});

/** A client of an authorization server: its identifier, authentication, and secret or key. */
export interface Client {
    /** The client identifier. */
    readonly clientId: string;
    /** How the client authenticates at the token endpoint. */
    readonly authentication: ClientAuthentication;
    /** The client secret or private key as JWK or PEM text, absent for a public client. */
    readonly secret?: string;
}

/** A started device authorization (RFC 8628 3.2). */
export const DeviceAuthorization = schema.object({
    /** The device code the client polls with. */
    deviceCode: schema.string().min(1),
    /** The code the person enters. */
    userCode: schema.string().min(1),
    /** The page the person enters the code on, with the code when the server includes it. */
    url: schema.httpUrl(),
    /** The seconds between polls. */
    interval: schema.number().int().positive(),
});
/** A started device authorization. */
export type DeviceAuthorization = schema.Infer<typeof DeviceAuthorization>;

/** The requests a client sends its authorization server, authenticated where needed. */
export interface AuthorizationClient {
    /** The issuer identifier. */
    readonly issuer: string;
    /** The endpoints. */
    readonly endpoints: AuthorizationEndpoints;
    /** Write the authorization page a person approves a code grant on. */
    readonly authorizationUrl: (request: AuthorizationPage) => URL;
    /** Request a token with a grant's parameters at the token endpoint. */
    readonly token: (grant: Readonly<Record<string, string>>) => Promise<Token>;
    /** Start a device authorization. */
    readonly authorizeDevice: (
        scopes: readonly string[],
        resource: string | undefined,
    ) => Promise<DeviceAuthorization>;
    /** Revoke a token at the revocation endpoint. */
    readonly revoke: (token: string, hint: "refresh_token" | "access_token") => Promise<void>;
}

/** The parameters of an authorization page (RFC 6749 4.1.1, RFC 7636 4.3, RFC 8707 2). */
export interface AuthorizationPage {
    /** The state the server returns. */
    readonly state: string;
    /** The PKCE code challenge. */
    readonly challenge: string;
    /** The URI the server redirects the person back to. */
    readonly redirectUri: string;
    /** The scopes asked for. */
    readonly scopes: readonly string[];
    /** The resource the token will be for. */
    readonly resource?: string;
}

/** An external OAuth 2.0 authorization server as one client calls it. */
export class AuthorizationServer implements AuthorizationClient {
    /** The issuer identifier. */
    readonly issuer: string;
    /** The endpoints. */
    readonly endpoints: AuthorizationEndpoints;
    /** The client. */
    readonly client: Client;
    /** Send requests to the server. */
    readonly #fetch: Fetch;
    /** The clock assertions are issued and tokens expire by. */
    readonly #clock: Clock;

    /** Call a server as a client, on the system clock unless given another. */
    constructor(options: {
        readonly issuer: string;
        readonly endpoints: AuthorizationEndpoints;
        readonly client: Client;
        readonly fetch: Fetch;
        readonly clock: Clock;
    }) {
        // take the server, the client, the fetch and the clock
        this.issuer = options.issuer;
        this.endpoints = options.endpoints;
        this.client = options.client;
        this.#fetch = options.fetch;
        this.#clock = options.clock;
    }

    /** Find a resource's authorization server (RFC 9728 3) and read its metadata (RFC 8414 3). */
    static async discover(
        resource: string,
        fetch: Fetch,
    ): Promise<{ readonly issuer: string; readonly endpoints: AuthorizationEndpoints }> {
        // read the resource's metadata listing its authorization servers
        const resourceUrl = new URL(resource);
        const metadataUrl = new URL(
            `/.well-known/oauth-protected-resource${trimmed(resourceUrl.pathname)}`,
            resourceUrl,
        );
        const described = ResourceMetadata.parse(
            await readJson(fetch, new Request(metadataUrl.href)),
        );
        if (described.resource !== resource) {
            throw new ConnectionError(
                "INVALID_RESPONSE",
                `${metadataUrl.origin} describes resource ${described.resource} for ${resource}`,
            );
        }

        // read the first authorization server's metadata and require it as the issuer
        const issuer = nth(described.authorization_servers, 0);
        const issuerUrl = new URL(issuer);
        const serverUrl = new URL(
            `/.well-known/oauth-authorization-server${trimmed(issuerUrl.pathname)}`,
            issuerUrl,
        );
        const server = ServerMetadata.parse(await readJson(fetch, new Request(serverUrl.href)));
        if (server.issuer !== issuer) {
            throw new ConnectionError(
                "INVALID_RESPONSE",
                `${serverUrl.origin} gives issuer ${server.issuer} instead of ${issuer}`,
            );
        }

        return {
            issuer,
            endpoints: {
                ...(server.authorization_endpoint === undefined
                    ? {}
                    : { authorization: server.authorization_endpoint }),
                ...(server.token_endpoint === undefined ? {} : { token: server.token_endpoint }),
                ...(server.revocation_endpoint === undefined
                    ? {}
                    : { revocation: server.revocation_endpoint }),
                ...(server.device_authorization_endpoint === undefined
                    ? {}
                    : { deviceAuthorization: server.device_authorization_endpoint }),
                ...(server.registration_endpoint === undefined
                    ? {}
                    : { registration: server.registration_endpoint }),
                isIssuerReturned: server.authorization_response_iss_parameter_supported,
            },
        };
    }

    /** Register a public authorization code client with redirect URIs (RFC 7591 3.1). */
    static async register(
        endpoint: string,
        redirectUris: readonly string[],
        fetch: Fetch,
    ): Promise<Client> {
        // ask for a public client of the code and refresh grants
        const request = new Request(endpoint, {
            method: "POST",
            headers: { "content-type": "application/json", accept: "application/json" },
            body: JSON.stringify({
                redirect_uris: redirectUris,
                grant_types: ["authorization_code", "refresh_token"],
                response_types: ["code"],
                token_endpoint_auth_method: "none",
            }),
        });
        const registered = RegistrationResponse.parse(await readJson(fetch, request));

        // record the registered client as confidential where the server issued a secret
        const authentication =
            registered.token_endpoint_auth_method ??
            (registered.client_secret === undefined ? "none" : "client_secret_basic");

        return {
            clientId: registered.client_id,
            authentication,
            ...(registered.client_secret === undefined ? {} : { secret: registered.client_secret }),
        };
    }

    /** Write the authorization page of a code grant (RFC 6749 4.1.1, RFC 7636 4.3, RFC 8707 2). */
    authorizationUrl(request: AuthorizationPage): URL {
        // write the code grant's parameters, PKCE and the resource indicator into the page's query
        const url = new URL(this.#require("authorization"));
        const parameters = {
            response_type: "code",
            client_id: this.client.clientId,
            redirect_uri: request.redirectUri,
            state: request.state,
            code_challenge: request.challenge,
            code_challenge_method: "S256",
            ...(request.scopes.length === 0 ? {} : { scope: request.scopes.join(" ") }),
            ...(request.resource === undefined ? {} : { resource: request.resource }),
        };
        for (const [name, value] of Object.entries(parameters)) {
            url.searchParams.set(name, value);
        }

        return url;
    }

    /** Request a token with a grant's parameters as the client (RFC 6749 3.2). */
    async token(grant: Readonly<Record<string, string>>): Promise<Token> {
        // authenticate the client and post the grant
        const endpoint = this.#require("token");
        const request = await this.#authenticated(endpoint, grant);
        const answered = TokenResponse.parse(await readJson(this.#fetch, request));

        // take the token with its expiry
        const expiresIn = answered.expires_in;

        return {
            accessToken: answered.access_token,
            tokenType: answered.token_type,
            ...(answered.refresh_token === undefined
                ? {}
                : { refreshToken: answered.refresh_token }),
            ...(expiresIn === undefined
                ? {}
                : {
                      expiresAt: this.#clock.now() + Duration.milliseconds({ seconds: expiresIn }),
                  }),
            ...(answered.scope === undefined ? {} : { scopes: answered.scope.split(" ") }),
        };
    }

    /** Start a device authorization (RFC 8628 3.1). */
    async authorizeDevice(
        scopes: readonly string[],
        resource: string | undefined,
    ): Promise<DeviceAuthorization> {
        // ask for a device and user code as the client
        const endpoint = this.#require("deviceAuthorization");
        const request = await this.#authenticated(endpoint, {
            ...(scopes.length === 0 ? {} : { scope: scopes.join(" ") }),
            ...(resource === undefined ? {} : { resource: resource }),
        });
        const answered = DeviceResponse.parse(await readJson(this.#fetch, request));

        return {
            deviceCode: answered.device_code,
            userCode: answered.user_code,
            url: answered.verification_uri_complete ?? answered.verification_uri,
            interval: answered.interval ?? DEVICE_INTERVAL,
        };
    }

    /** Revoke a token (RFC 7009 2.1), doing nothing at a server without a revocation endpoint. */
    async revoke(token: string, hint: "refresh_token" | "access_token"): Promise<void> {
        // revoke nothing at a server without a revocation endpoint
        const endpoint = this.endpoints.revocation;
        if (endpoint === undefined) {
            return;
        }

        // post the token as the client and refuse an error answer
        const request = await this.#authenticated(endpoint, { token, token_type_hint: hint });
        const response = await send(this.#fetch, request);
        if (!response.ok) {
            throw await refusal(request, response);
        }
    }

    /** Build a form post authenticating the client as registered (RFC 6749 2.3, RFC 7523 2.2). */
    async #authenticated(
        endpoint: string,
        parameters: Readonly<Record<string, string>>,
    ): Promise<Request> {
        // authenticate in the form or in the Authorization header
        const { clientId, authentication } = this.client;
        const headers = new Headers({
            "content-type": "application/x-www-form-urlencoded",
            accept: "application/json",
        });
        const form = new URLSearchParams(parameters);
        if (authentication === "client_secret_basic") {
            const user = encodeURIComponent(clientId);
            const password = encodeURIComponent(this.#secret());
            headers.set("authorization", `Basic ${btoa(`${user}:${password}`)}`);
        } else if (authentication === "client_secret_post") {
            form.set("client_id", clientId);
            form.set("client_secret", this.#secret());
        } else if (authentication === "private_key_jwt") {
            form.set("client_id", clientId);
            form.set("client_assertion_type", CLIENT_ASSERTION_TYPE);
            form.set(
                "client_assertion",
                await Assertion.sign(
                    {
                        issuer: clientId,
                        subject: clientId,
                        audience: endpoint,
                        key: this.#secret(),
                    },
                    this.#clock.now(),
                ),
            );
        } else {
            form.set("client_id", clientId);
        }

        return new Request(endpoint, { method: "POST", headers, body: form });
    }

    /** Read the client's secret, refusing a client without one. */
    #secret(): string {
        const secret = this.client.secret;
        if (secret === undefined) {
            throw new ConnectionError(
                "INVALID_AUTHORIZATION",
                `client ${this.client.clientId} authenticates with a secret it does not hold`,
            );
        }

        return secret;
    }

    /** Read an endpoint the server must have, refusing a server without it. */
    #require(name: "authorization" | "token" | "deviceAuthorization"): string {
        const endpoint = this.endpoints[name];
        if (endpoint === undefined) {
            throw new ConnectionError(
                "INVALID_AUTHORIZATION",
                `authorization server ${this.issuer} has no ${name} endpoint`,
            );
        }

        return endpoint;
    }
}

/** Send a request, reading a network failure as an unreachable server. */
async function send(fetch: Fetch, request: Request): Promise<Response> {
    try {
        return await fetch(request);
    } catch (cause) {
        throw new ConnectionError("UNREACHABLE", `${new URL(request.url).origin} did not answer`, {
            cause,
        });
    }
}

/** Send a request and read its JSON answer, refusing an error answer. */
async function readJson(fetch: Fetch, request: Request): Promise<unknown> {
    // refuse an error answer
    const response = await send(fetch, request);
    if (!response.ok) {
        throw await refusal(request, response);
    }

    // read the JSON body
    const origin = new URL(request.url).origin;

    return response.json().catch(() => {
        throw new ConnectionError("INVALID_RESPONSE", `${origin} answered no JSON`);
    });
}

/** Read a request's error answer as the authorization server's OAuth error, or its status. */
async function refusal(request: Request, response: Response): Promise<ConnectionError> {
    // read the OAuth error of the body or else the status
    const body: unknown = await response.json().catch(() => undefined);
    const parsed = ErrorResponse.safeParse(body);
    const origin = new URL(request.url).origin;

    return parsed.success
        ? new ConnectionError("REFUSED", parsed.data.error_description ?? parsed.data.error, {
              error: parsed.data.error,
          })
        : new ConnectionError("REFUSED", `${origin} answered ${response.status}`);
}

/** Write a well-known suffix's path: the given path without a trailing slash, empty at the root. */
function trimmed(pathname: string): string {
    return pathname === "/" ? "" : pathname.replace(/\/$/u, "");
}
