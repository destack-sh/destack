import type { Authorizer } from "@destack/access";
import type { DatabaseConnection } from "@destack/db";
import type { Environment } from "@destack/environment";
import { REQUEST_OBJECT_TYPE } from "../authentication/token.ts";
import { ConsentEndpoint } from "./consent.ts";
import { CALL_ERROR_STATUSES, Failure } from "@destack/error";
import type { JwtClaims, KeySet } from "@destack/identity";
import { Bookmark, BOOKMARK_HEADER } from "@destack/object/client";
import type { ObjectStore } from "@destack/object/server";
import { Scope } from "@destack/sync";
import type { AuthContext } from "better-auth";
import {
    AUTHORIZATION_PATH,
    AUTHORIZATION_REQUEST_PATH,
    CONSENT_PATH,
    DEVICE_AUTHORIZATION_PATH,
    DEVICE_REQUEST_PATH,
    INTROSPECTION_PATH,
    JWKS_PATH,
    METADATA_PATHS,
    REVOCATION_PATH,
    TOKEN_PATH,
    USER_INFO_PATH,
} from "../authentication/path.ts";
import { ConsentRequest, OAUTH_ERROR_CODES, OAuthError } from "../authentication/token.ts";
import { OAUTH_CLIENT_AUTHENTICATIONS } from "../object/application.ts";
import { Prompt } from "../object/authorization-request.ts";
import { AuthorizationEndpoint, AuthorizationRequestEndpoint, codeGrant } from "./authorization.ts";
import { assertionVerifier, bearerGrant, JWT_BEARER } from "./bearer.ts";
import { DeviceAuthorizationEndpoint, deviceGrant, DeviceRequestEndpoint } from "./device.ts";
import { type CodeCheck, emailGrant } from "./email.ts";
import { exchangeGrant, TOKEN_EXCHANGE } from "./exchange.ts";
import { DEVICE_CODE, EMAIL_CODE } from "./party.ts";
import { refreshGrant } from "./refresh.ts";
import type { TokenLifetime } from "./resource.ts";
import { IntrospectionEndpoint, RevocationEndpoint } from "./revocation.ts";
import { TokenEndpoint } from "./token.ts";
import { UserInfoEndpoint } from "./userinfo.ts";
import { OAUTH_SCOPES } from "../object/application.ts";

/** The claims the server issues in ID tokens and at the userinfo endpoint (OIDC Core 2, 5.1). */
const CLAIMS = [
    "iss",
    "sub",
    "aud",
    "exp",
    "iat",
    "auth_time",
    "nonce",
    "sid",
    "azp",
    "name",
    "picture",
    "email",
    "email_verified",
    "preferred_username",
    "locale",
    "zoneinfo",
];

/** What the authorization server signs, stores, reads and pages with. */
export interface AuthorizationServerOptions {
    /** The issuer's URL, below which every endpoint answers. */
    readonly issuer: string;
    /** The account database. */
    readonly database: DatabaseConnection;
    /** The account objects, which store the server's records and register devices' clients. */
    readonly objects: Pick<ObjectStore, "mutate" | "as" | "database" | "environment" | "watermark">;
    /** The clock, randomness, WebCrypto and network the server runs on. */
    readonly environment: Environment;
    /** The policies expanding memberships, which token exchanges check. */
    readonly authorizer: Authorizer;
    /** How long the packages' tokens live. */
    readonly lifetime: TokenLifetime;
    /** Sign claims with the universe's active key. */
    readonly sign: (claims: JwtClaims, header: { readonly typ: string }) => Promise<string>;
    /** Read the universe's current key set. */
    readonly keys: () => Promise<KeySet>;
    /** Read the session a browser's request signs in, none for a signed-out browser. */
    readonly session: (headers: Headers) => Promise<string | undefined>;
    /** The check of mailed sign-in codes, spending each. */
    readonly codes: CodeCheck;
    /** The Better Auth context, which creates the people and sessions mailed codes sign in. */
    readonly context: Promise<
        Pick<AuthContext, "internalAdapter" | "sessionConfig" | "isTrustedOrigin">
    >;
    /** The pages a person signs in, consents and approves devices on. */
    readonly pages: { readonly signIn: URL; readonly consent: URL; readonly verification: URL };
}

/** An endpoint of the server. */
interface Endpoint {
    /** Answer a request. */
    readonly fetch: (request: Request) => Promise<Response>;
}

/** The universe's OAuth 2.1 and OpenID Connect authorization server, below the issuer's URL. */
export class AuthorizationServer {
    /** The authorization endpoint. */
    readonly authorization: AuthorizationEndpoint;
    /** The token endpoint. */
    readonly token: TokenEndpoint;
    /** The device authorization endpoint. */
    readonly device: DeviceAuthorizationEndpoint;
    /** The issuer's path, below which every endpoint answers. */
    readonly #base: string;
    /** The endpoints, by their path below the issuer. */
    readonly #endpoints: ReadonlyMap<string, Endpoint>;
    /** The account objects, whose universe watermark each answer reports. */
    readonly #objects: Pick<ObjectStore, "watermark">;

    /** Compose the server's endpoints. */
    constructor(options: AuthorizationServerOptions) {
        // compose the endpoints that decide and issue
        const { issuer, pages } = options;
        this.#base = new URL(issuer).pathname;
        this.#objects = options.objects;
        const endpoint = `${issuer}${TOKEN_PATH}`;
        this.authorization = new AuthorizationEndpoint({
            ...options,
            ...pages,
            requests: assertionVerifier({ ...options, endpoint: issuer }, REQUEST_OBJECT_TYPE),
        });
        this.device = new DeviceAuthorizationEndpoint({ ...options, ...pages });
        this.token = new TokenEndpoint(
            options,
            new Map([
                ["authorization_code", codeGrant(options)],
                ["refresh_token", refreshGrant(options)],
                [JWT_BEARER, bearerGrant({ ...options, endpoint })],
                [TOKEN_EXCHANGE, exchangeGrant(options)],
                [EMAIL_CODE, emailGrant(options)],
                [DEVICE_CODE, deviceGrant(options)],
            ]),
        );

        // describe the server in one document at both metadata paths
        const metadata = this.#metadata(issuer);
        const document: Endpoint = { fetch: () => Promise.resolve(Response.json(metadata)) };
        this.#endpoints = new Map<string, Endpoint>([
            ...METADATA_PATHS.map((path) => [path, document] as const),
            [AUTHORIZATION_PATH, this.authorization],
            [TOKEN_PATH, this.token],
            [DEVICE_AUTHORIZATION_PATH, this.device],
            [
                AUTHORIZATION_REQUEST_PATH,
                new AuthorizationRequestEndpoint(this.authorization, options.context),
            ],
            [
                DEVICE_REQUEST_PATH,
                new DeviceRequestEndpoint(this.device, options.session, options.context),
            ],
            [CONSENT_PATH, new ConsentEndpoint(options)],
            [REVOCATION_PATH, new RevocationEndpoint(options)],
            [INTROSPECTION_PATH, new IntrospectionEndpoint(options, this.token)],
            [USER_INFO_PATH, new UserInfoEndpoint(options, this.token)],
        ]);
    }

    /** The endpoints' paths below the issuer. */
    get paths(): readonly string[] {
        return [...this.#endpoints.keys()];
    }

    /** Report whether a path below the issuer is one of the server's endpoints. */
    handles(path: string): boolean {
        return this.#endpoints.has(path);
    }

    /** Take a request at an endpoint below the issuer, refusing as RFC 6749 5.2 does. */
    async fetch(request: Request): Promise<Response> {
        // find the endpoint of the request's path below the issuer
        const path = new URL(request.url).pathname.slice(this.#base.length);
        const endpoint = this.#endpoints.get(path);
        if (endpoint === undefined) {
            throw new TypeError(`no endpoint takes '${path}'`);
        }

        // take the request at the endpoint, refusing as RFC 6749 5.2 does
        const response = await endpoint
            .fetch(request)
            .catch((error: unknown) => AuthorizationServer.refusal(error));

        // mark every response but the discovery document uncacheable (RFC 6749 5.1)
        if (!METADATA_PATHS.some((each) => each === path)) {
            response.headers.set("cache-control", "no-store");
        }

        // add the universe's watermark, so the caller's later calls read the sign-in's writes
        const bookmark = new Bookmark();
        bookmark.observe(await this.#objects.watermark(Scope.universe.id));
        response.headers.set(BOOKMARK_HEADER, bookmark.format());

        return response;
    }

    /** Answer an OAuth refusal as its JSON error, passing on any other failure. */
    static refusal(error: unknown): Response {
        // pass on a failure that is no OAuth refusal
        if (!OAuthError.is(error)) {
            throw error;
        }

        // answer the error, with the challenge of a refused token or client (RFC 6750 3, RFC 6749 5.2)
        const { reason, code, message, data } = Failure.of(error);
        const wire = OAUTH_ERROR_CODES[error.reason];
        const challenge =
            reason === "TOKEN_UNAUTHENTICATED"
                ? { "www-authenticate": `Bearer error="${wire}"` }
                : reason === "CLIENT_UNAUTHENTICATED"
                  ? { "www-authenticate": `Basic error="${wire}"` }
                  : undefined;

        // answer a consent with the permissions it needs (RFC 9396)
        const details = reason === "CONSENT_REQUIRED" ? ConsentRequest.parse(data) : {};

        return Response.json(
            { error: wire, error_description: message, ...details },
            {
                status: CALL_ERROR_STATUSES[code],
                ...(challenge === undefined ? {} : { headers: challenge }),
            },
        );
    }

    /** Describe the server's endpoints and what they support (OIDC Discovery 3, RFC 8414 2). */
    #metadata(issuer: string) {
        return {
            issuer,
            authorization_endpoint: `${issuer}${AUTHORIZATION_PATH}`,
            token_endpoint: `${issuer}${TOKEN_PATH}`,
            device_authorization_endpoint: `${issuer}${DEVICE_AUTHORIZATION_PATH}`,
            revocation_endpoint: `${issuer}${REVOCATION_PATH}`,
            introspection_endpoint: `${issuer}${INTROSPECTION_PATH}`,
            userinfo_endpoint: `${issuer}${USER_INFO_PATH}`,
            jwks_uri: `${issuer}${JWKS_PATH}`,
            response_types_supported: ["code"],
            response_modes_supported: ["query"],
            grant_types_supported: [...this.token.grants.keys()],
            code_challenge_methods_supported: ["S256"],
            token_endpoint_auth_methods_supported: [...OAUTH_CLIENT_AUTHENTICATIONS],
            scopes_supported: OAUTH_SCOPES,
            subject_types_supported: ["public"],
            id_token_signing_alg_values_supported: ["ES256"],
            prompt_values_supported: Prompt.options,
            claims_supported: CLAIMS,
            authorization_response_iss_parameter_supported: true,
        };
    }
}
