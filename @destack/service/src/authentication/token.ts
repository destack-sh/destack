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
import { Subject, PermissionReference, Attribute, sameSubject } from "@destack/access";
import {
    Caller,
    type CallerAuthentication,
    CALLER_LIFETIME_MS,
    CALLER_CLOCK_TOLERANCE_MS,
} from "./caller.ts";
import { ServiceError } from "../error/index.ts";

/** Maximum interval between public-key refreshes in milliseconds. */
const KEY_CACHE_MS = 60000;
/** Bound key discovery latency in milliseconds. */
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
    /** Acting software identity, when acting on behalf of another subject. */
    actor: Subject.optional(),
    /** Complete delegation chain, intersected with current object permissions. */
    delegations: schema
        .array(
            schema.object({
                /** Stable delegation identifier. */
                id: schema.string().min(1),
                /** Identity granting this step. */
                subject: Subject,
                /** Software identity receiving this step. */
                actor: Subject,
                /** Exact restrictions applied by this step. */
                permissions: schema.array(permission),
                /** Creation time in Unix milliseconds. */
                createdAt: schema.number().int(),
                /** Exclusive expiry in Unix milliseconds. */
                expiresAt: schema.number().int(),
                /** Revocation time, or null while active. */
                revokedAt: schema.number().int().nullable(),
            }),
        )
        .optional(),
    /** Deployment identities authenticated by the issuing host. */
    deployments: schema
        .array(
            schema.object({
                /** Represented or acting workload identity. */
                subject: Subject,
                /** Exact authenticated deployment. */
                id: identifier("deployment"),
            }),
        )
        .optional(),
    /** Verified direct and group identities. */
    subjects: schema.array(Subject),
    /** Account memberships verified at issuance. */
    memberships: schema.array(
        schema.object({
            /** Member identity. */
            subject: Subject,
            /** Administering account. */
            accountId: identifier("account"),
            /** Current membership record. */
            id: identifier("account-membership"),
        }),
    ),
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
            ({ payload } = await jwtVerify(authorization.slice(7), this.keys, {
                issuer: this.options.issuer,
                audience: this.options.audience,
                algorithms: ["ES256"],
                requiredClaims: ["iss", "aud", "sub", "iat", "exp", "jti"],
                maxTokenAge: CALLER_LIFETIME_MS / 1000,
                clockTolerance: CALLER_CLOCK_TOLERANCE_MS / 1000,
                currentDate: new Date(now),
            }));
        } catch (error) {
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

        const caller = new Caller({
            ...parsed.data,
            scope: parsed.data.spaceId,
            audience: this.options.audience,
            verifiedAt: payload.iat! * 1000,
            expiresAt: payload.exp! * 1000,
        });
        caller.context(this.options.audience, now, parsed.data.spaceId);
        verifyTokenAuthentication(caller.authentication, this.options.authority, now);

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
    readonly fetch?: typeof globalThis.fetch;
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

/** Verify identity claims against issuer authority, deployment identities and delegation chains. */
export function verifyTokenAuthentication(
    authentication: CallerAuthentication,
    authority: TokenIssuerAuthority,
    now: number,
): void {
    const subjects = [authentication.subject, ...authentication.subjects];

    // a space signing key cannot assert global users, memberships or another space's identities
    if (
        authority.kind === "space" &&
        (authentication.scope !== authority.spaceId ||
            authentication.memberships?.length ||
            authentication.actor ||
            authentication.delegations?.length ||
            subjects.some(
                (subject) =>
                    subject.kind !== "service-account" || subject.authority !== authority.spaceId,
            ))
    ) {
        throw new ServiceError("UNAUTHORIZED", { message: "token exceeds issuer authority" });
    }

    // bind regional software identities to an authenticated deployment
    const workloads = [
        ...subjects,
        ...(authentication.delegations ?? []).map((delegation) => delegation.actor),
    ].filter(
        (subject) =>
            subject.kind === "service-account" && subject.authority === authentication.scope,
    );
    const deployments = authentication.deployments ?? [];
    for (const subject of workloads) {
        if (
            deployments.filter((deployment) => sameSubject(deployment.subject, subject)).length !==
            1
        ) {
            throw new ServiceError("UNAUTHORIZED", { message: "invalid workload token identity" });
        }
    }
    if (
        deployments.some(
            (deployment) => !workloads.some((subject) => sameSubject(subject, deployment.subject)),
        )
    ) {
        throw new ServiceError("UNAUTHORIZED", { message: "unexpected workload token identity" });
    }

    // reject incomplete, revoked, expired and cyclic delegation before invoking application code
    const chain = authentication.delegations ?? [];
    if (!authentication.actor) {
        if (chain.length) {
            throw new ServiceError("UNAUTHORIZED", { message: "delegation requires an actor" });
        }
        return;
    }
    let previous = authentication.subject;
    const seen = new Set<string>([
        JSON.stringify([previous.authority, previous.kind, previous.id]),
    ]);
    const ids = new Set<string>();
    for (const delegation of chain) {
        const identity = JSON.stringify([
            delegation.actor.authority,
            delegation.actor.kind,
            delegation.actor.id,
        ]);
        if (
            !sameSubject(previous, delegation.subject) ||
            delegation.actor.kind !== "service-account" ||
            ids.has(delegation.id) ||
            seen.has(identity) ||
            delegation.revokedAt !== null ||
            delegation.createdAt > now ||
            delegation.expiresAt < authentication.expiresAt ||
            delegation.permissions.some((permission) => permission.scope !== authentication.scope)
        ) {
            throw new ServiceError("UNAUTHORIZED", { message: "invalid token delegation" });
        }
        ids.add(delegation.id);
        seen.add(identity);
        previous = delegation.actor;
    }
    if (!chain.length || !sameSubject(previous, authentication.actor)) {
        throw new ServiceError("UNAUTHORIZED", { message: "invalid token delegation actor" });
    }
}
