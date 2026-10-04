import { Bearer } from "./bearer.ts";
import type { PackageId } from "@destack/package";
import {
    createLocalJWKSet,
    createRemoteJWKSet,
    customFetch,
    errors,
    jwtVerify,
    type JSONWebKeySet,
} from "jose";
import { schema } from "@destack/schema";
import {
    Authentication,
    AuthenticationClaims,
    AUTHENTICATION_LIFETIME_MILLISECONDS,
    AUTHENTICATION_CLOCK_TOLERANCE_MILLISECONDS,
} from "./authentication.ts";
import { ServiceError } from "../error/index.ts";

/** The longest interval between public key refreshes, in milliseconds. */
const KEY_CACHE_MILLISECONDS = 60000;
/** The longest key discovery wait and the pause after a failed one, in milliseconds. */
const KEY_TIMEOUT_MILLISECONDS = 5000;

/** The signed identity and restrictions of a caller in one space or the universe: its authentication without its receiver and times. */
export const TokenAuthentication = AuthenticationClaims.omit({
    scope: true,
    audience: true,
    verifiedAt: true,
    expiresAt: true,
}).extend({
    /** The space, absent for a call to a universe service. */
    spaceId: schema.identifier("space").exactOptional(),
});

/** The claims of a verified access token, in their RFC 7519 names. */
const AccessTokenClaims = schema
    .object({
        /** The signed caller. */
        caller: TokenAuthentication,
        /** The token's use, always an access token. */
        token_use: schema.literal("access"),
        /** The caller's subject. */
        sub: schema.string(),
        /** The receiving package. */
        aud: schema.string(),
        /** The token's unique identifier. */
        jti: schema.string().min(1),
        /** The issue time, in UTC epoch seconds. */
        iat: schema.number().int(),
        /** The expiry time, in UTC epoch seconds. */
        exp: schema.number().int(),
    })
    .loose();

/** Verify space-scoped access tokens. */
export class TokenVerifier {
    /** The verifier configuration. */
    readonly options: TokenVerifierOptions;
    /** The cached public keys. */
    readonly keys: ReturnType<typeof createLocalJWKSet> | ReturnType<typeof createRemoteJWKSet>;

    /** Create the verifier. */
    constructor(options: TokenVerifierOptions) {
        this.options = options;

        // fetch keys over plain HTTP only from loopback
        for (const url of options.keys instanceof URL ? [options.keys] : []) {
            if (
                url.protocol !== "https:" &&
                !(
                    url.protocol === "http:" &&
                    ["localhost", "127.0.0.1", "[::1]"].includes(url.hostname)
                )
            ) {
                throw new ServiceError("PRECONDITION_FAILED", {
                    message: "authentication requires HTTPS outside loopback",
                });
            }
        }

        // cache keys through JOSE
        this.keys =
            options.keys instanceof URL
                ? createRemoteJWKSet(options.keys, {
                      cacheMaxAge: KEY_CACHE_MILLISECONDS,
                      cooldownDuration: KEY_TIMEOUT_MILLISECONDS,
                      timeoutDuration: KEY_TIMEOUT_MILLISECONDS,
                      ...(options.fetch === undefined ? {} : { [customFetch]: options.fetch }),
                  })
                : createLocalJWKSet(options.keys);
    }

    /** Verify a request's bearer token, optionally for one space. */
    async authenticate(
        request: Request,
        spaceId?: string,
        now = Date.now(),
    ): Promise<Authentication<schema.Infer<typeof TokenAuthentication>["credential"]>> {
        return this.verify(Bearer.require(request.headers), spaceId, now);
    }

    /** Verify an access token, optionally for one space. */
    async verify(
        token: string,
        spaceId?: string,
        now = Date.now(),
    ): Promise<Authentication<schema.Infer<typeof TokenAuthentication>["credential"]>> {
        // verify the signature and registered claims
        const payload = await this.#verifySignature(token, now);

        // parse the caller claims and check the space
        const result = AccessTokenClaims.safeParse(payload);
        if (
            !result.success ||
            (spaceId !== undefined && result.data.caller.spaceId !== spaceId) ||
            result.data.sub !== result.data.caller.subject.id ||
            result.data.aud !== this.options.audience ||
            result.data.exp <= result.data.iat ||
            (result.data.exp - result.data.iat) * 1000 > AUTHENTICATION_LIFETIME_MILLISECONDS
        ) {
            throw new ServiceError("UNAUTHORIZED", { message: "invalid access token claims" });
        }

        // build the caller in the space it calls
        const claims = result.data;
        const { spaceId: scope, ...identity } = claims.caller;
        const caller = new Authentication({
            ...identity,
            ...(scope === undefined ? {} : { scope }),
            audience: this.options.audience,
            verifiedAt: claims.iat * 1000,
            expiresAt: claims.exp * 1000,
        });

        // require a current caller within the issuer's authority
        caller.requireCurrent(this.options.audience, now, scope);
        caller.requireAuthority(this.options.authority);

        return caller;
    }

    /** Verify a token's signature and registered claims, returning its payload. */
    async #verifySignature(token: string, now: number): Promise<unknown> {
        try {
            const { payload } = await jwtVerify(token, this.keys, {
                issuer: this.options.issuer,
                audience: this.options.audience,
                algorithms: ["ES256"],
                requiredClaims: ["iss", "aud", "sub", "iat", "exp", "jti"],
                maxTokenAge: AUTHENTICATION_LIFETIME_MILLISECONDS / 1000,
                clockTolerance: AUTHENTICATION_CLOCK_TOLERANCE_MILLISECONDS / 1000,
                currentDate: new Date(now),
            });

            return payload;
        } catch (error) {
            // report a failed verification as unauthorized
            if (
                error instanceof errors.JWTClaimValidationFailed ||
                error instanceof errors.JWTExpired ||
                error instanceof errors.JWTInvalid ||
                error instanceof errors.JWSInvalid ||
                error instanceof errors.JWSSignatureVerificationFailed ||
                error instanceof errors.JOSEAlgNotAllowed ||
                error instanceof errors.JOSENotSupported ||
                error instanceof errors.JWKSNoMatchingKey
            ) {
                throw new ServiceError("UNAUTHORIZED", {
                    message: "invalid access token",
                    cause: error,
                });
            }

            // report other failures as unavailable
            throw new ServiceError("SERVICE_UNAVAILABLE", {
                message: "authentication keys are unavailable",
                cause: error,
            });
        }
    }
}

/** The configuration of a token verifier. */
export interface TokenVerifierOptions {
    /** The identity authority of the trusted keys. */
    readonly authority: TokenIssuerAuthority;
    /** The issuer, a URL for a remote key set or a host identifier for its host keys. */
    readonly issuer: string;
    /** The receiving package. */
    readonly audience: PackageId;
    /** The public keys, or an HTTPS JWKS endpoint. */
    readonly keys: URL | JSONWebKeySet;
    /** The fetch of key discovery. */
    readonly fetch?: (...arguments_: Parameters<typeof globalThis.fetch>) => Promise<Response>;
}

/** The identities an issuer's keys may assert. */
export type TokenIssuerAuthority =
    | {
          /** The universe's account authority. */
          readonly kind: "universe";
      }
    | {
          /** An authority over workload identities of one space. */
          readonly kind: "space";
          /** The space. */
          readonly spaceId: string;
      };
