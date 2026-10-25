import { type Authentication, Caller, principal } from "@destack/access";
import { ACCESS_TOKEN_TYPE, TokenIdentity, TokenIssuerAuthority } from "@destack/access/token";
import type { DatabaseConnection } from "@destack/db";
import type { Environment } from "@destack/environment";
import { Jwks, Jwt, type JwtClaims, type KeySet } from "@destack/identity";
import { CallError } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import { Uuid } from "@destack/schema";
import { USER_INFO_PATH } from "../authentication/path.ts";
import { OAuthError, SECOND_MILLISECONDS } from "../authentication/token.ts";
import { AccountError } from "../error/error.ts";
import type { OAuthConsent } from "../object/oauth-consent.ts";
import { User } from "../object/user.ts";
import { RefreshToken } from "../object/refresh-token.ts";
import { PackageResource, type TokenLifetime } from "./resource.ts";

/** How long an access token to a person's claims alone lives: ten minutes, as RFC 9068 practice. */
export const PERSON_TOKEN_MILLISECONDS = 10 * 60_000;

/** The JWT type of an ID token (RFC 7519 5.1). */
const ID_TOKEN_TYPE = "JWT";

/** The scope granting an ID token (OIDC Core 3.1.2.1). */
export const OPENID = "openid";

/** The scope granting a refresh token (OIDC Core 11). */
export const OFFLINE_ACCESS = "offline_access";

/** The media type of a posted form (RFC 6749 3.2). */
const FORM_TYPE = "application/x-www-form-urlencoded";

/** A token request: its posted form and its headers, which may authenticate the client. */
export interface TokenRequest {
    /** The posted form's fields. */
    readonly body: Readonly<Record<string, string>>;
    /** The request's headers. */
    readonly headers: Headers;
}

/** A grant the token endpoint issues tokens by, deciding what to issue for a request. */
export type TokenGrant = (request: TokenRequest) => Promise<Issue>;

/** What a grant issues: an access token, and a consenting person's ID and refresh tokens. */
export interface Issue {
    /** The caller the access token describes. */
    readonly authentication: Authentication;
    /** The client receiving the tokens. */
    readonly clientId: string;
    /** The granted OAuth scopes. */
    readonly scopes: readonly string[];
    /** The package the access token is for, absent for a person's claims alone. */
    readonly resource?: string;
    /** The consent of the person signing the client in, with the family its refresh tokens join. */
    readonly consent?: {
        /** The consent. */
        readonly row: OAuthConsent;
        /** The family the refresh token joins. */
        readonly family: string;
        /** The session the person signed in with. */
        readonly sessionId?: string;
        /** The client's value the ID token repeats. */
        readonly nonce?: string;
        /** The refresh token the grant spends, rotated in the same mutation as its successor. */
        readonly rotating?: RefreshToken;
    };
}

/** What the token endpoint signs, stores and times tokens with. */
export interface TokenEndpointOptions {
    /** The issuer's URL, which every token includes. */
    readonly issuer: string;
    /** The account database with the people. */
    readonly database: DatabaseConnection;
    /** The account objects, which store the refresh tokens. */
    readonly objects: Pick<ObjectStore, "mutate" | "as">;
    /** The clock tokens are issued at, the randomness of their identifiers and the WebCrypto. */
    readonly environment: Pick<Environment, "clock" | "crypto" | "random">;
    /** How long the packages' tokens live. */
    readonly lifetime: TokenLifetime;
    /** Sign claims with the universe's active key. */
    readonly sign: (claims: JwtClaims, header: { readonly typ: string }) => Promise<string>;
}

/** The token endpoint: issue tokens to clients by their grants (RFC 6749 3.2). */
export class TokenEndpoint {
    /** The grants, by grant type. */
    readonly grants: ReadonlyMap<string, TokenGrant>;
    /** What the endpoint signs, stores and times tokens with. */
    readonly #options: TokenEndpointOptions;

    /** Issue tokens by grants. */
    constructor(options: TokenEndpointOptions, grants: ReadonlyMap<string, TokenGrant>) {
        this.#options = options;
        this.grants = grants;
    }

    /** Read a request's posted form, refusing another body (RFC 6749 3.2). */
    static async form(request: Request): Promise<Readonly<Record<string, string>>> {
        // refuse another body than a posted form
        const isForm = request.headers.get("content-type")?.startsWith(FORM_TYPE) === true;
        if (request.method !== "POST" || !isForm) {
            throw OAuthError.of("INVALID_REQUEST", "the endpoint takes a posted form");
        }

        // refuse a parameter given twice (RFC 6749 3.2)
        const form = new URLSearchParams(await request.text());
        const names = [...form.keys()];
        if (new Set(names).size !== names.length) {
            throw OAuthError.of("INVALID_REQUEST", "a parameter is given more than once");
        }

        return Object.fromEntries(form);
    }

    /** Answer a posted token request with the tokens its grant issues. */
    async answer(request: Request): Promise<Response> {
        // decide by the request's grant, refusing a grant check as an invalid grant
        const body = await TokenEndpoint.form(request);
        const grant = this.grants.get(body["grant_type"] ?? "");
        if (grant === undefined) {
            throw OAuthError.of("UNSUPPORTED_GRANT_TYPE", "unsupported grant_type");
        }
        const issue = await grant({ body, headers: request.headers }).catch((error: unknown) => {
            throw TokenEndpoint.refusal(error);
        });

        return Response.json(await this.issue(issue));
    }

    /** Issue a grant's tokens, ending the access token no later than its session or grant. */
    async issue(issue: Issue) {
        // time the access token by its package, else as a person's claims alone, refusing a lapsed session or grant
        const { issuer, environment, lifetime } = this.#options;
        const now = environment.clock.now();
        const audience =
            issue.resource === undefined ? undefined : PackageResource.package(issue.resource);
        const lasts = audience === undefined ? PERSON_TOKEN_MILLISECONDS : lifetime.of(audience);
        const lapse = issue.authentication.credential.expiresAt;
        if (lapse <= now) {
            throw OAuthError.of("INVALID_GRANT", "the session or grant lapsed");
        }
        const ends = Math.min(now + lasts, lapse);

        // refuse a consenting person the universe suspended
        const { consent, scopes } = issue;
        if (
            consent !== undefined &&
            !(await User.maySignIn(this.#options.database, consent.row.scope))
        ) {
            throw OAuthError.of("INVALID_GRANT", "the user may no longer sign in");
        }

        // address the package, and the person's claims with the openid scope
        const isOpenId = consent !== undefined && scopes.includes(OPENID);
        const audiences = [
            ...(issue.resource === undefined ? [] : [issue.resource]),
            ...(isOpenId ? [`${issuer}${USER_INFO_PATH}`] : []),
        ];
        if (audiences.length === 0) {
            throw OAuthError.of("INVALID_TARGET", "the request names no resource");
        }

        // sign the access token, and the person's ID and refresh tokens as the scopes grant
        const issuedAt = Math.floor(now / SECOND_MILLISECONDS);
        const expiresAt = Math.floor(ends / SECOND_MILLISECONDS);
        const accessToken = await AccessToken.sign(
            { ...issue, audiences, issuedAt, expiresAt },
            this.#options,
        );
        const idToken = isOpenId
            ? await IdToken.sign({ ...issue, consent, issuedAt, expiresAt }, this.#options)
            : undefined;
        const refreshToken = await this.#refresh(issue);

        return {
            access_token: accessToken,
            token_type: "Bearer",
            expires_in: expiresAt - issuedAt,
            scope: scopes.join(" "),
            refresh_token: refreshToken,
            id_token: idToken,
        };
    }

    /** Issue the refresh token the scopes grant, spending the one the grant rotates. */
    async #refresh(issue: Issue): Promise<string | undefined> {
        // issue a successor with the offline scope, and spend the rotated token alone without it
        const { objects, database, environment } = this.#options;
        const { consent, scopes } = issue;
        if (consent === undefined) {
            return undefined;
        }
        const isOffline = scopes.includes(OFFLINE_ACCESS);
        const { assurance } = issue.authentication.credential;
        const terms = { ...consent, scopes, resource: issue.resource, assurance };
        try {
            return isOffline
                ? await RefreshToken.issue(objects, terms, environment)
                : await RefreshToken.spend(objects, consent.rotating, environment);
        } catch (error) {
            // revoke the family of a token spent before, as its replay reveals a theft (RFC 9700 4.14.2)
            if (!AccountError.is(error, "TOKEN_REUSED") || consent.rotating === undefined) {
                throw error;
            }
            await RefreshToken.revoke(objects, database, consent.rotating, environment);
            throw OAuthError.of("INVALID_GRANT", "the refresh token was used before");
        }
    }

    /** Read a grant check's refusal as an invalid grant, passing on other failures. */
    static refusal(error: unknown): unknown {
        // pass on an OAuth error, and any failure that is no refusal
        const refused = CallError.refusal(error);
        if (OAuthError.is(error) || refused === undefined) {
            return error;
        }

        return OAuthError.of("INVALID_GRANT", refused.message, { cause: error });
    }
}

/** The access tokens the token endpoint signs (RFC 9068), with the Destack claims. */
export const AccessToken = {
    /** Sign an access token for a caller toward its audiences. */
    sign(
        terms: Pick<Issue, "authentication" | "clientId" | "scopes" | "consent"> & {
            /** The receivers: the package's resource and the userinfo endpoint. */
            readonly audiences: readonly string[];
            /** The issue time, in UTC epoch seconds. */
            readonly issuedAt: number;
            /** The expiry, in UTC epoch seconds. */
            readonly expiresAt: number;
        },
        options: Pick<TokenEndpointOptions, "issuer" | "environment" | "sign">,
    ): Promise<string> {
        // write the Destack claims within the universe's authority
        const { authentication } = terms;
        TokenIssuerAuthority.require({ kind: "universe" }, authentication);
        const [single] = terms.audiences;

        return options.sign(
            {
                ...TokenIdentity.claims(authentication),
                iss: options.issuer,
                sub: authentication.caller.subject.id,
                aud:
                    terms.audiences.length === 1 && single !== undefined
                        ? single
                        : [...terms.audiences],
                client_id: terms.clientId,
                azp: terms.clientId,
                sid: terms.consent?.sessionId,
                scope: terms.scopes.join(" "),
                jti: Uuid.v4(options.environment.random).replaceAll("-", ""),
                iat: terms.issuedAt,
                exp: terms.expiresAt,
            },
            { typ: ACCESS_TOKEN_TYPE },
        );
    },

    /** Verify an access token the issuer signed, for an audience when given. */
    async verify(
        token: string,
        audience: string | undefined,
        options: AccessTokenOptions,
    ): Promise<JwtClaims> {
        // verify the token by the universe's current keys, refusing any failure as an invalid token
        const { crypto } = options.environment;
        const keys = Jwks.local(await options.keys(), crypto);
        const verified = await Jwt.verify(
            token,
            keys,
            {
                typ: ACCESS_TOKEN_TYPE,
                issuer: options.issuer,
                ...(audience === undefined ? {} : { audience }),
                algorithms: ["ES256"],
                now: options.environment.clock.now(),
            },
            crypto,
        ).catch((error: unknown) => {
            throw OAuthError.of("INVALID_TOKEN", "invalid access token", { cause: error });
        });

        return verified.claims;
    },

    /** Read the client a caller's token goes to: its client, else its sending key holder. */
    client(authentication: Authentication): string {
        // refuse a person's token without a client, as a person holds no keys
        const { caller, credential } = authentication;
        const sender = Caller.sender(caller);
        if (credential.client === undefined && principal.user.is(sender)) {
            throw OAuthError.of("INVALID_GRANT", "a user's token needs a client");
        }

        return credential.client ?? sender.id;
    },
};

/** What access tokens verify with: the issuer and its keys at the clock's time. */
export interface AccessTokenOptions {
    /** The issuer's URL, which every token includes. */
    readonly issuer: string;
    /** Read the universe's current key set. */
    readonly keys: () => Promise<KeySet>;
    /** The clock tokens are verified at, and the WebCrypto verifying them. */
    readonly environment: Pick<Environment, "clock" | "crypto">;
}

/** The ID tokens the token endpoint signs for a signed-in person (OIDC Core 2). */
export const IdToken = {
    /** Sign an ID token of a consenting person for the client. */
    sign(
        terms: Pick<Issue, "authentication" | "clientId"> & {
            /** The person's consent, the session they signed in with and the client's nonce. */
            readonly consent: NonNullable<Issue["consent"]>;
            /** The issue time, in UTC epoch seconds. */
            readonly issuedAt: number;
            /** The expiry, in UTC epoch seconds. */
            readonly expiresAt: number;
        },
        options: Pick<TokenEndpointOptions, "issuer" | "sign">,
    ): Promise<string> {
        // state when the person authenticated, as their session's assurance says
        const { consent } = terms;
        const authenticatedAt = terms.authentication.credential.assurance?.authenticatedAt;

        return options.sign(
            {
                iss: options.issuer,
                sub: consent.row.scope,
                aud: terms.clientId,
                azp: terms.clientId,
                iat: terms.issuedAt,
                exp: terms.expiresAt,
                auth_time:
                    authenticatedAt === undefined
                        ? undefined
                        : Math.floor(authenticatedAt / SECOND_MILLISECONDS),
                sid: consent.sessionId,
                nonce: consent.nonce,
            },
            { typ: ID_TOKEN_TYPE },
        );
    },
};

/** Split a space-separated parameter into its values, such as scopes (RFC 6749 3.3). */
export function spaceSeparated(value: string | undefined): string[] {
    return (value ?? "").split(" ").filter((word) => word !== "");
}
