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
    CallerGrant,
    CLOCK_TOLERANCE_MILLISECONDS,
    isPrincipal,
    principal,
} from "@destack/access";
import { CallError } from "../error/index.ts";

/** The longest interval between public key refreshes, in milliseconds. */
export const KEY_CACHE_MILLISECONDS = 60000;
/** The longest key discovery wait and the pause after a failed one, in milliseconds. */
export const KEY_TIMEOUT_MILLISECONDS = 5000;
/** The longest lifetime of a token and the verified identity and membership records it carries. */
export const TOKEN_LIFETIME_MILLISECONDS = 60000;
/** The credential kind of a machine's key. */
export const MACHINE_KEY = "machine-key";
/** The credential kind of an installation's key. */
export const INSTALLATION_KEY = "installation-key";
/** The credential kind of a token that a space's own key signs. */
export const SPACE_KEY = "space-key";

/** The verified identity an access token signs: subject, subject sets, and what it proved. */
export const TokenIdentity = Caller.schema
    .pick({ subjects: true, assurance: true, contacts: true, deployments: true, attributes: true })
    .extend({
        /** The represented subject. */
        subject: Subject,
        /** The credential the caller presented to the token's issuer. */
        credential: CallerCredential.pick({ kind: true, id: true }),
    });
/** The verified identity an access token signs. */
export type TokenIdentity = schema.Infer<typeof TokenIdentity>;

/** The claims of an access token, in their RFC 7519, 7800, 8693 and 9396 names. */
export const AccessTokenClaims = schema
    .object({
        /** The issuer: the universe's URL, or a space signing for itself. */
        iss: schema.string(),
        /** The represented subject's identifier. */
        sub: schema.string(),
        /** The receiving package. */
        aud: schema.string(),
        /** The token's unique identifier. */
        jti: schema.string().min(1),
        /** The issue time, in UTC epoch seconds. */
        iat: schema.number().int(),
        /** The expiry time, in UTC epoch seconds. */
        exp: schema.number().int(),
        /** The token's use, always an access token. */
        token_use: schema.literal("access"),
        /** The principals acting for the subject in order, the last sending the call. */
        act: schema.array(Subject).min(1).exactOptional(),
        /** The grant limiting the call to what the subject gave. */
        grant: CallerGrant.exactOptional(),
        /** The client whose key the token is bound to. */
        cnf: schema.object({ client: schema.identifier("client") }).exactOptional(),
        /** The space the token is bound to, absent for a call to a universe package. */
        space: schema.identifier("space").exactOptional(),
        /** The verified identity. */
        caller: TokenIdentity,
    })
    .loose();
/** The claims of an access token. */
export type AccessTokenClaims = schema.Infer<typeof AccessTokenClaims>;

/** A caller a token verified: its subject and its credential, both present. */
type VerifiedCaller = Caller & Required<Pick<Caller, "subject" | "credential">>;

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
                throw new CallError("PRECONDITION_FAILED", {
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

        // parse the claims and check the space
        const result = AccessTokenClaims.safeParse(payload);
        if (
            !result.success ||
            (spaceId !== undefined && result.data.space !== spaceId) ||
            result.data.sub !== result.data.caller.subject.id ||
            result.data.aud !== this.options.audience ||
            result.data.exp <= result.data.iat
        ) {
            throw new CallError("UNAUTHORIZED", { message: "invalid access token claims" });
        }

        // refuse a token outliving its audience's lifetime
        const claims = result.data;
        const { credential, ...identity } = claims.caller;
        const verifiedAt = claims.iat * 1000;
        const lifetime = this.options.lifetime ?? TOKEN_LIFETIME_MILLISECONDS;
        if ((claims.exp - claims.iat) * 1000 > lifetime) {
            throw new CallError("UNAUTHORIZED", { message: "invalid access token claims" });
        }

        // verify it for this audience in the space it calls
        const caller: VerifiedCaller = {
            ...identity,
            ...(claims.act === undefined ? {} : { delegates: claims.act }),
            ...(claims.grant === undefined ? {} : { grant: claims.grant }),
            credential: {
                ...credential,
                ...(claims.cnf === undefined ? {} : { client: claims.cnf.client }),
                audience: this.options.audience,
                ...(claims.space === undefined ? {} : { scope: claims.space }),
                verifiedAt,
                expiresAt: Math.min(claims.exp * 1000, verifiedAt + lifetime),
                token: claims.jti,
            },
        };

        // require it current and within its issuer's authority
        Caller.requireCurrent(caller, this.options.audience, now, claims.space);
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
                maxTokenAge: (this.options.lifetime ?? TOKEN_LIFETIME_MILLISECONDS) / 1000,
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
        // limit a space's key to the space and its installations
        const subjects = [
            ...(caller.subject === undefined ? [] : [caller.subject]),
            ...caller.subjects,
        ];
        if (authority.kind === "space") {
            requireSpaceSubjects(authority.space, caller, subjects);
        }

        // bind each asserted installation to one deployment
        requireDeployments(caller, subjects);

        // require an acyclic chain of single principals, a delegate without a grant impersonating
        requireDelegation(caller);
    },
};

/** Require a space key's claims to assert the space or its installations alone. */
function requireSpaceSubjects(space: string, caller: Caller, subjects: readonly Subject[]): void {
    const isOwn = (subject: Subject) =>
        (principal.space.is(subject) && subject.id === space) ||
        (principal.installation.is(subject) && subject.scope === space);
    if (
        (caller.contacts ?? []).length > 0 ||
        (caller.delegates ?? []).length > 0 ||
        !subjects.every(isOwn)
    ) {
        throw new CallError("UNAUTHORIZED", { message: "token exceeds issuer authority" });
    }
}

/** Require one deployment per asserted installation, at most one per delegate, and no other. */
function requireDeployments(caller: Caller, subjects: readonly Subject[]): void {
    // list the asserted installations a deployment binds, and the delegates one may bind
    const kind = caller.credential?.kind;
    const isKeyed = kind === INSTALLATION_KEY || kind === SPACE_KEY;
    const workloads = subjects.filter(
        (subject) =>
            principal.installation.is(subject) &&
            !(isKeyed && caller.subject !== undefined && Subject.same(subject, caller.subject)),
    );
    const delegates = (caller.delegates ?? []).filter((subject) =>
        principal.installation.is(subject),
    );

    // bind each workload to one deployment, and each delegate to at most one
    const deployments = caller.deployments ?? [];
    const matching = (subject: Subject) =>
        deployments.filter((deployment) => Subject.same(deployment.subject, subject)).length;
    if (
        workloads.some((subject) => matching(subject) !== 1) ||
        delegates.some((subject) => matching(subject) > 1)
    ) {
        throw new CallError("UNAUTHORIZED", { message: "invalid workload token identity" });
    }

    // reject deployments of no asserted workload or delegate
    const bound = [...workloads, ...delegates];
    if (
        deployments.some(
            (deployment) => !bound.some((subject) => Subject.same(subject, deployment.subject)),
        )
    ) {
        throw new CallError("UNAUTHORIZED", { message: "unexpected workload token identity" });
    }
}

/** Require an acyclic chain of principals, a delegate without a grant impersonating a person. */
function requireDelegation(caller: Caller): void {
    // list the chain from the subject to the sender
    const delegates = caller.delegates ?? [];
    const chain = [...(caller.subject === undefined ? [] : [caller.subject]), ...delegates];
    const isImpersonation = caller.grant === undefined && delegates.length > 0;

    // refuse other subjects than principals, a repeated one, and an impersonation by other than one person
    if (
        delegates.some((delegate) => !isPrincipal(delegate)) ||
        (isImpersonation &&
            (delegates.length > 1 ||
                !delegates.every((delegate) => principal.user.is(delegate)))) ||
        chain.some((entry, position) =>
            chain.slice(position + 1).some((other) => Subject.same(entry, other)),
        )
    ) {
        throw new CallError("UNAUTHORIZED", { message: "invalid token delegation" });
    }
}
