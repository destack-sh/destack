import type { PackageId } from "@destack/package";
import {
    createLocalJWKSet,
    createRemoteJWKSet,
    customFetch,
    errors,
    jwtVerify,
    type JSONWebKeySet,
} from "jose";
import { schema, identifier } from "@destack/schema";
import {
    Caller,
    CallerAuthentication,
    CALLER_LIFETIME_MILLISECONDS,
    CALLER_CLOCK_TOLERANCE_MILLISECONDS,
} from "./caller.ts";
import { ServiceError } from "../error/index.ts";

/** The longest interval between public key refreshes, in milliseconds. */
const KEY_CACHE_MILLISECONDS = 60000;
/** The longest key discovery wait and the pause after a failed one, in milliseconds. */
const KEY_TIMEOUT_MILLISECONDS = 5000;

/** The signed identity and restrictions of one space: a caller's authentication without its receiver and times. */
export const TokenAuthentication = CallerAuthentication.omit({
    scope: true,
    audience: true,
    verifiedAt: true,
    expiresAt: true,
}).extend({
    /** The space. */
    spaceId: identifier("space"),
});

/** Verify space-scoped access tokens. */
export class TokenVerifier {
    /** The verifier configuration. */
    readonly options: TokenVerifierOptions;
    /** The cached public keys. */
    readonly keys: ReturnType<typeof createLocalJWKSet> | ReturnType<typeof createRemoteJWKSet>;

    /** Create the verifier. */
    constructor(options: TokenVerifierOptions) {
        this.options = options;

        // allow plain HTTP only for loopback
        for (const url of [
            new URL(options.issuer),
            ...(options.keys instanceof URL ? [options.keys] : []),
        ]) {
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
                      [customFetch]: options.fetch,
                  })
                : createLocalJWKSet(options.keys);
    }

    /** Verify a bearer token, optionally for one space. */
    async authenticate(
        request: Request,
        spaceId?: string,
        now = Date.now(),
    ): Promise<Caller<schema.Infer<typeof TokenAuthentication>["credential"]>> {
        // require one bearer token
        const authorization = request.headers.get("authorization");
        if (
            !authorization ||
            !/^Bearer \S+$/.test(authorization) ||
            request.headers.has("cookie")
        ) {
            throw new ServiceError("UNAUTHORIZED", { message: "invalid bearer credential" });
        }

        // verify the signature and registered claims
        let payload;
        try {
            ({ payload } = await jwtVerify(authorization.slice("Bearer ".length), this.keys, {
                issuer: this.options.issuer,
                audience: this.options.audience,
                algorithms: ["ES256"],
                requiredClaims: ["iss", "aud", "sub", "iat", "exp", "jti"],
                maxTokenAge: CALLER_LIFETIME_MILLISECONDS / 1000,
                clockTolerance: CALLER_CLOCK_TOLERANCE_MILLISECONDS / 1000,
                currentDate: new Date(now),
            }));
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
            throw new ServiceError("UNAVAILABLE", {
                message: "authentication keys are unavailable",
                cause: error,
            });
        }

        // parse the caller claims and check the space
        const parsed = TokenAuthentication.safeParse(payload.caller);
        if (
            !parsed.success ||
            payload.token_use !== "access" ||
            (spaceId !== undefined && parsed.data.spaceId !== spaceId) ||
            payload.sub !== parsed.data.subject.id ||
            payload.aud !== this.options.audience ||
            typeof payload.jti !== "string" ||
            payload.jti.length === 0 ||
            !Number.isInteger(payload.iat) ||
            !Number.isInteger(payload.exp) ||
            payload.exp! <= payload.iat! ||
            (payload.exp! - payload.iat!) * 1000 > CALLER_LIFETIME_MILLISECONDS
        ) {
            throw new ServiceError("UNAUTHORIZED", { message: "invalid access token claims" });
        }

        // build the caller and check it
        const caller = new Caller({
            ...parsed.data,
            scope: parsed.data.spaceId,
            audience: this.options.audience,
            verifiedAt: payload.iat! * 1000,
            expiresAt: payload.exp! * 1000,
        });
        caller.requireCurrent(this.options.audience, now, parsed.data.spaceId);
        caller.requireAuthority(this.options.authority);

        return caller;
    }
}

/** The configuration of a token verifier. */
export interface TokenVerifierOptions {
    /** The identity authority of the trusted keys. */
    readonly authority: TokenIssuerAuthority;
    /** The issuer URL. */
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
          /** The global account authority. */
          readonly kind: "global";
      }
    | {
          /** An authority over workload identities of one space. */
          readonly kind: "space";
          /** The space. */
          readonly spaceId: string;
      };
