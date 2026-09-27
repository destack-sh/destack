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
    Attribute,
    AuthenticationAssurance,
    VerifiedIdentifier,
    PermissionReference,
    Delegate,
    Subject,
} from "@destack/access";
import {
    Caller,
    CallerDeployment,
    CALLER_LIFETIME_MS,
    CALLER_CLOCK_TOLERANCE_MS,
} from "./caller.ts";
import { ServiceError } from "../error/index.ts";

/** Maximum interval between public-key refreshes in milliseconds. */
const KEY_CACHE_MS = 60000;
/** The longest key discovery wait, and the pause after a failed one, in milliseconds. */
const KEY_TIMEOUT_MS = 5000;

/** Exact permission selection retained by credentials and delegation steps. */
const permission = PermissionReference.extend({
    /** Exact authority scope. */
    scope: schema.string().min(1),
    /** Optional object restriction. */
    objectId: schema.string().optional(),
});

/** Signed identity and restrictions for one space. */
export const TokenAuthentication = schema.object({
    /** Exact space selected during credential exchange. */
    spaceId: identifier("space"),
    /** Persistent credential reference, excluding its bearer value. */
    credential: schema.object({
        /** Issuing authority's credential category. */
        kind: schema.string().min(1),
        /** Stable credential identifier. */
        id: schema.string().min(1),
    }),
    /** Represented identity. */
    subject: Subject,
    /** How strongly and how recently the represented subject authenticated. */
    assurance: AuthenticationAssurance.optional(),
    /** Identifiers, such as email addresses, the represented subject proved control of. */
    identifiers: schema.array(VerifiedIdentifier).optional(),
    /** The principals acting in order, each for the one before and the first for the subject; the last sends the request. */
    delegates: schema.array(Delegate).optional(),
    /** Deployment identities authenticated by the issuing host. */
    deployments: schema.array(CallerDeployment).optional(),
    /** The verified principals and the subject sets the caller belongs to. */
    subjects: schema.array(Subject),
    /** Credential restrictions intersected with current local grants. */
    permissions: schema.array(permission).optional(),
    /** Trusted attributes asserted by the issuer. */
    attributes: schema.record(schema.string(), Attribute).optional(),
});

/** Verify space-scoped access tokens using the deployment's trusted issuer and public keys. */
export class TokenVerifier {
    /** Exact issuer and receiving package configured by the host. */
    readonly options: TokenVerifierOptions;
    /** Cached public-key resolver; token contents never select a discovery URL. */
    readonly keys: ReturnType<typeof createLocalJWKSet> | ReturnType<typeof createRemoteJWKSet>;

    /** Retain public keys across requests and constrain remote discovery to trusted URLs. */
    constructor(options: TokenVerifierOptions) {
        this.options = options;

        // permit plain HTTP only for loopback development authorities
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

        // reuse JOSE's key cache and concurrent discovery coordination
        this.keys =
            options.keys instanceof URL
                ? createRemoteJWKSet(options.keys, {
                      cacheMaxAge: KEY_CACHE_MS,
                      cooldownDuration: KEY_TIMEOUT_MS,
                      timeoutDuration: KEY_TIMEOUT_MS,
                      [customFetch]: options.fetch,
                  })
                : createLocalJWKSet(options.keys);
    }

    /** Verify a bearer token and optionally require one fixed receiving space. */
    async authenticate(
        request: Request,
        spaceId?: string,
        now = Date.now(),
    ): Promise<Caller<schema.Infer<typeof TokenAuthentication>["credential"]>> {
        // require one bearer token without cookies
        const authorization = request.headers.get("authorization");
        if (
            !authorization ||
            !/^Bearer \S+$/.test(authorization) ||
            request.headers.has("cookie")
        ) {
            throw new ServiceError("UNAUTHORIZED");
        }

        // verify signatures and registered claims before interpreting application claims
        let payload;
        try {
            ({ payload } = await jwtVerify(authorization.slice("Bearer ".length), this.keys, {
                issuer: this.options.issuer,
                audience: this.options.audience,
                algorithms: ["ES256"],
                requiredClaims: ["iss", "aud", "sub", "iat", "exp", "jti"],
                maxTokenAge: CALLER_LIFETIME_MS / 1000,
                clockTolerance: CALLER_CLOCK_TOLERANCE_MS / 1000,
                currentDate: new Date(now),
            }));
        } catch (error) {
            // report a token that fails verification as unauthorized
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

            // report other failures as unavailable keys
            throw new ServiceError("UNAVAILABLE", {
                message: "authentication keys are unavailable",
                cause: error,
            });
        }

        // require the access-token purpose, bounded lifetime and any configured fixed space
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
            (payload.exp! - payload.iat!) * 1000 > CALLER_LIFETIME_MS
        ) {
            throw new ServiceError("UNAUTHORIZED", { message: "invalid access token claims" });
        }

        // build the caller and check its freshness and issuer authority
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

/** Trusted deployment configuration for signed access tokens. */
export interface TokenVerifierOptions {
    /** Explicit identity authority assigned to these trusted signing keys. */
    readonly authority: TokenIssuerAuthority;
    /** Exact authentication issuer URL. */
    readonly issuer: string;
    /** Exact receiving service package identifier. */
    readonly audience: PackageId;
    /** Trusted public keys, or a fixed HTTPS JWKS endpoint. */
    readonly keys: URL | JSONWebKeySet;
    /** Host transport for public-key discovery. */
    readonly fetch?: (...arguments_: Parameters<typeof globalThis.fetch>) => Promise<Response>;
}

/** Identity assertions permitted for a trusted issuer's signing keys. */
export type TokenIssuerAuthority =
    | {
          /** The global account authority. */
          readonly kind: "global";
      }
    | {
          /** An authority restricted to workload identities in one space. */
          readonly kind: "space";
          /** Exact administered space. */
          readonly spaceId: string;
      };
