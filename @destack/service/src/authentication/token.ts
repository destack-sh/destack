import { Bearer } from "./bearer.ts";
import type { PackageId } from "@destack/package";
import {
    createLocalJWKSet,
    createRemoteJWKSet,
    customFetch,
    jwtVerify,
    type JSONWebKeySet,
    type JWTVerifyGetKey,
} from "jose";
import { schema } from "@destack/schema";
import { Subject } from "@destack/sync";
import {
    Caller,
    CallerCredential,
    CLOCK_TOLERANCE_MILLISECONDS,
    isPrincipal,
    principal,
} from "@destack/access";
import { ServiceError } from "../error/index.ts";

/** The longest interval between public key refreshes, in milliseconds. */
export const KEY_CACHE_MILLISECONDS = 60000;
/** The longest key discovery wait and the pause after a failed one, in milliseconds. */
export const KEY_TIMEOUT_MILLISECONDS = 5000;
/** The longest lifetime of a token and the verified identity and membership records it carries. */
export const TOKEN_LIFETIME_MILLISECONDS = 60000;
/** The longest lifetime of a token a space exchanged for a caller, lending its authority. */
export const EXCHANGE_LIFETIME_MILLISECONDS = 60 * 60_000;
/** The credential kind of an installation's key. */
export const INSTALLATION_KEY = "installation-key";
/** The credential kind of a token that a space's own key signs. */
export const SPACE_KEY = "space-key";

/** The caller an access token signs. */
export const TokenCaller = Caller.schema.extend({
    /** The represented identity. */
    subject: Subject,
    /** The credential the caller presented to the token's issuer. */
    credential: CallerCredential.pick({ kind: true, id: true, client: true }),
    /** The space, absent for a call to a universe service. */
    spaceId: schema.identifier("space").exactOptional(),
});
/** The caller an access token signs. */
export type TokenCaller = schema.Infer<typeof TokenCaller>;

/** A caller a token verified: its subject and its credential, both present. */
type VerifiedCaller = Caller & Required<Pick<Caller, "subject" | "credential">>;

/** The claims of a verified access token, in their RFC 7519 names. */
const AccessTokenClaims = schema
    .object({
        /** The signed caller. */
        caller: TokenCaller,
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
    readonly keys: JWTVerifyGetKey;

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

        // cache keys through JOSE, or take a key set a directory keeps
        this.keys =
            typeof options.keys === "function"
                ? options.keys
                : options.keys instanceof URL
                  ? createRemoteJWKSet(options.keys, {
                        cacheMaxAge: KEY_CACHE_MILLISECONDS,
                        cooldownDuration: KEY_TIMEOUT_MILLISECONDS,
                        timeoutDuration: KEY_TIMEOUT_MILLISECONDS,
                        ...(options.fetch === undefined ? {} : { [customFetch]: options.fetch }),
                    })
                  : createLocalJWKSet(options.keys);
    }

    /** Verify a request's bearer token at a time, optionally for one space. */
    async authenticate(request: Request, now: number, spaceId?: string): Promise<VerifiedCaller> {
        return this.verify(Bearer.require(request.headers), now, spaceId);
    }

    /** Verify an access token at a time, optionally for one space. */
    async verify(token: string, now: number, spaceId?: string): Promise<VerifiedCaller> {
        // verify the signature and registered claims
        const payload = await this.#verifySignature(token, now);

        // parse the caller claims and check the space
        const result = AccessTokenClaims.safeParse(payload);
        if (
            !result.success ||
            (spaceId !== undefined && result.data.caller.spaceId !== spaceId) ||
            result.data.sub !== result.data.caller.subject.id ||
            result.data.aud !== this.options.audience ||
            result.data.exp <= result.data.iat
        ) {
            throw new ServiceError("UNAUTHORIZED", { message: "invalid access token claims" });
        }

        // refuse a token outliving its audience's lifetime
        const claims = result.data;
        const { spaceId: scope, credential, ...identity } = claims.caller;
        const verifiedAt = claims.iat * 1000;
        const lifetime = Caller.isExchanged(identity)
            ? EXCHANGE_LIFETIME_MILLISECONDS
            : (this.options.lifetime ?? TOKEN_LIFETIME_MILLISECONDS);
        if ((claims.exp - claims.iat) * 1000 > lifetime) {
            throw new ServiceError("UNAUTHORIZED", { message: "invalid access token claims" });
        }

        // verify it for this audience in the space it calls
        const caller: VerifiedCaller = {
            ...identity,
            credential: {
                ...credential,
                audience: this.options.audience,
                ...(scope === undefined ? {} : { scope }),
                verifiedAt,
                expiresAt: Math.min(claims.exp * 1000, verifiedAt + lifetime),
                token: claims.jti,
            },
        };

        // require it current and within its issuer's authority
        Caller.requireCurrent(caller, this.options.audience, now, scope);
        TokenIssuerAuthority.require(this.options.authority, caller);

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
                maxTokenAge: EXCHANGE_LIFETIME_MILLISECONDS / 1000,
                clockTolerance: CLOCK_TOLERANCE_MILLISECONDS / 1000,
                currentDate: new Date(now),
            });

            return payload;
        } catch (error) {
            throw Bearer.refusal(error, "invalid access token");
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
    /** The public keys, an HTTPS JWKS endpoint, or a key set a directory keeps. */
    readonly keys: URL | JSONWebKeySet | JWTVerifyGetKey;
    /** The fetch of key discovery. */
    readonly fetch?: (...arguments_: Parameters<typeof globalThis.fetch>) => Promise<Response>;
    /** The longest lifetime of the tokens it accepts, a minute by default. */
    readonly lifetime?: number;
}

/** The identities an issuer's keys may assert. */
export type TokenIssuerAuthority =
    | {
          /** The universe's account authority. */
          readonly kind: "universe";
      }
    | {
          /** The authority of one space over itself and its installations. */
          readonly kind: "space";
          /** The space. */
          readonly space: string;
      };

/** The identities an issuer's keys may assert, and the check of a caller against them. */
export const TokenIssuerAuthority = {
    /** Require a caller's claims within an issuer's authority. */
    require(authority: TokenIssuerAuthority, caller: Caller): void {
        // limit a space's key to the space, its installations and exchanges in it
        const subjects = [
            ...(caller.subject === undefined ? [] : [caller.subject]),
            ...caller.subjects,
        ];
        if (authority.kind === "space") {
            requireSpaceSubjects(authority.space, caller, subjects);
        }

        // bind each asserted installation to one deployment
        requireDeployments(caller, subjects);

        // require an acyclic chain of single principals
        requireDelegation(caller);
    },
};

/** Require a space key's claims to assert the space or its installations. */
function requireSpaceSubjects(space: string, caller: Caller, subjects: readonly Subject[]): void {
    // accept a person lent to one of the space's installations within the space
    const lent = caller.delegates?.[0]?.subject;
    if (
        Caller.isExchanged(caller) &&
        lent !== undefined &&
        "scope" in lent &&
        lent.scope === space
    ) {
        if (caller.credential?.scope !== space) {
            throw new ServiceError("UNAUTHORIZED", {
                message: "an exchanged token lends authority in its issuer's space alone",
            });
        }

        return;
    }

    // accept only the space and its installations otherwise
    const isOwn = (subject: Subject) =>
        (principal.space.is(subject) && subject.id === space) ||
        (principal.installation.is(subject) && subject.scope === space);
    if (
        (caller.contacts ?? []).length > 0 ||
        (caller.delegates ?? []).length > 0 ||
        !subjects.every(isOwn)
    ) {
        throw new ServiceError("UNAUTHORIZED", { message: "token exceeds issuer authority" });
    }
}

/** Require one deployment per asserted installation, and no other. */
function requireDeployments(caller: Caller, subjects: readonly Subject[]): void {
    // list the asserted installations a deployment binds
    const kind = caller.credential?.kind;
    const isKeyed = kind === INSTALLATION_KEY || kind === SPACE_KEY;
    const lent = Caller.isExchanged(caller) ? caller.delegates?.[0]?.subject : undefined;
    const workloads = [
        ...subjects,
        ...(caller.delegates ?? []).map((delegate) => delegate.subject),
    ].filter(
        (subject) =>
            principal.installation.is(subject) &&
            !(isKeyed && caller.subject !== undefined && Subject.same(subject, caller.subject)),
    );

    // bind each one to one deployment
    const deployments = caller.deployments ?? [];
    for (const subject of workloads) {
        const matching = deployments.filter((deployment) =>
            Subject.same(deployment.subject, subject),
        ).length;
        const isLent = lent !== undefined && Subject.same(lent, subject);
        if (matching > 1 || (matching === 0 && !isLent)) {
            throw new ServiceError("UNAUTHORIZED", { message: "invalid workload token identity" });
        }
    }

    // reject deployments of no asserted workload
    if (
        deployments.some(
            (deployment) => !workloads.some((subject) => Subject.same(subject, deployment.subject)),
        )
    ) {
        throw new ServiceError("UNAUTHORIZED", { message: "unexpected workload token identity" });
    }
}

/** Require an acyclic delegation chain of single principals. */
function requireDelegation(caller: Caller): void {
    const delegates = caller.delegates ?? [];
    const chain = [
        ...(caller.subject === undefined ? [] : [caller.subject]),
        ...delegates.map((delegate) => delegate.subject),
    ];
    if (
        delegates.some(
            (delegate, position) =>
                !isPrincipal(delegate.subject) ||
                (delegate.authority === "full" &&
                    (position !== 0 || !principal.user.is(delegate.subject))),
        ) ||
        chain.some((entry, position) =>
            chain.slice(position + 1).some((other) => Subject.same(entry, other)),
        )
    ) {
        throw new ServiceError("UNAUTHORIZED", { message: "invalid token delegation" });
    }
}
