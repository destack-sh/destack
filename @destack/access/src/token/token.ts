import { Bearer } from "./bearer.ts";
import { PackageId } from "@destack/package";
import {
    createLocalJWKSet,
    createRemoteJWKSet,
    customFetch,
    jwtVerify,
    type JSONWebKeySet,
    type JWTVerifyGetKey,
} from "jose";
import { type Identifier, schema } from "@destack/schema";
import { Scope, Subject } from "@destack/sync";
import {
    type Authentication,
    Caller,
    CLOCK_TOLERANCE_MILLISECONDS,
    Credential,
    GrantClaim,
} from "../context/index.ts";
import { isPrincipal, principal } from "../declare/index.ts";
import { AccessError } from "../error/index.ts";

/** The longest interval between public key refreshes, in milliseconds. */
export const KEY_CACHE_MILLISECONDS = 60000;
/** The longest key discovery wait and the pause after a failed one, in milliseconds. */
export const KEY_TIMEOUT_MILLISECONDS = 5000;
/** The longest lifetime of a token and of the verified identity and membership records in it. */
export const TOKEN_LIFETIME_MILLISECONDS = 60000;
/** The credential kind of a machine's key. */
export const MACHINE_KEY = "machine-key";
/** The credential kind of an installation's key. */
export const INSTALLATION_KEY = "installation-key";
/** The credential kind of a token that a space's key signs. */
export const SPACE_KEY = "space-key";

/** The schema of the verified identity an access token signs. */
const tokenIdentity = Caller.schema.pick({ subject: true, groups: true, contacts: true }).extend({
    /** The credential the caller presented to the token's issuer. */
    credential: Credential.pick({
        kind: true,
        id: true,
        assurance: true,
        deployments: true,
        attributes: true,
        watermarks: true,
    }),
});

/** The verified identity an access token signs: subject, its groups, and what it proved. */
export const TokenIdentity = Object.assign(tokenIdentity, {
    /** Write an authentication's Destack claims: identity, actors, grants and space. */
    claims(authentication: Authentication) {
        const { caller, credential } = authentication;
        const identity = tokenIdentity.parse(
            schema.defined({
                subject: caller.subject,
                groups: caller.groups,
                contacts: caller.contacts,
                credential: schema.defined({
                    kind: credential.kind,
                    id: credential.id,
                    assurance: credential.assurance,
                    deployments: credential.deployments,
                    attributes: credential.attributes,
                    watermarks: credential.watermarks,
                }),
            }),
        );

        return {
            caller: identity,
            ...(caller.actors.length === 0 ? {} : { act: [...caller.actors] }),
            ...(caller.grants.length === 0 ? {} : { grants: [...caller.grants] }),
            ...(credential.scope === undefined
                ? {}
                : { space: schema.identifier("space").parse(credential.scope) }),
        };
    },
});
/** The verified identity an access token signs. */
export type TokenIdentity = schema.Infer<typeof tokenIdentity>;

/** The JWT type of an access token (RFC 9068 2.1). */
export const ACCESS_TOKEN_TYPE = "at+jwt";

/** An access token as RFC 9068 shapes it, with RFC 8693's `act` and Destack's claims. */
export const AccessToken = schema
    .object({
        /** The issuer: the token endpoint's issuer URL, or a space signing for itself. */
        iss: schema.string(),
        /** The represented subject's identifier. */
        sub: schema.string(),
        /** The receiving packages, each as its resource URI. */
        aud: schema.union([schema.string(), schema.array(schema.string()).min(1)]),
        /** The key holder the token was issued to: the sending principal, or a user's client. */
        client_id: schema.union([
            schema.identifier("client"),
            schema.identifier("machine"),
            schema.identifier("installation"),
            schema.identifier("space"),
            schema.identifier("token"),
            schema.literal(Scope.universe.id),
        ]),
        /** The granted scopes, space separated. */
        scope: schema.string().exactOptional(),
        /** The token's unique identifier. */
        jti: schema.string().min(1),
        /** The issue time, in UTC epoch seconds. */
        iat: schema.number().int(),
        /** The expiry time, in UTC epoch seconds. */
        exp: schema.number().int(),
        /** The principals acting for the subject in order, the last sending the call. */
        act: schema.array(Subject).min(1).exactOptional(),
        /** The grants each actor acts under, from the one before it. */
        grants: schema.array(GrantClaim).min(1).exactOptional(),
        /** The space the token is bound to, absent for a call to a universe package. */
        space: schema.identifier("space").exactOptional(),
        /** The verified identity. */
        caller: TokenIdentity,
    })
    .loose();
/** An access token as RFC 9068 shapes it. */
export type AccessToken = schema.Infer<typeof AccessToken>;

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
                throw AccessError.of(
                    "INSECURE_KEYS",
                    "authentication requires HTTPS outside loopback",
                );
            }
        }

        // cache keys through JOSE, or take a key set a directory stores
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
    async authenticate(request: Request, now: number, spaceId?: string): Promise<Authentication> {
        return this.verify(Bearer.require(request.headers), now, spaceId);
    }

    /** Verify an access token at a time, optionally for one space. */
    async verify(token: string, now: number, spaceId?: string): Promise<Authentication> {
        // verify the signature and registered claims
        const payload = await this.#verifySignature(token, now);

        // parse the claims and check the space and the client
        const result = AccessToken.safeParse(payload);
        const client = result.success ? clientOf(result.data) : undefined;
        if (
            !result.success ||
            client === undefined ||
            (spaceId !== undefined && result.data.space !== spaceId) ||
            result.data.sub !== result.data.caller.subject.id ||
            result.data.exp <= result.data.iat
        ) {
            throw AccessError.of("INVALID_TOKEN", "invalid access token claims");
        }

        // refuse a token outliving its audience's lifetime
        const claims = result.data;
        const { credential: presented, ...identity } = claims.caller;
        const verifiedAt = claims.iat * 1000;
        const lifetime = this.options.lifetime ?? TOKEN_LIFETIME_MILLISECONDS;
        if ((claims.exp - claims.iat) * 1000 > lifetime) {
            throw AccessError.of("INVALID_TOKEN", "invalid access token claims");
        }

        // verify it for this audience in the space it calls
        const caller: Caller = {
            ...identity,
            actors: claims.act ?? [],
            grants: claims.grants ?? [],
        };
        const credential: Credential = {
            ...presented,
            ...client,
            audience: this.options.audience,
            ...(claims.space === undefined ? {} : { scope: claims.space }),
            verifiedAt,
            expiresAt: Math.min(claims.exp * 1000, verifiedAt + lifetime),
            tokenId: claims.jti,
        };

        // require it current and within its issuer's authority
        Credential.requireCurrent(credential, this.options.audience, now, claims.space);
        TokenIssuerAuthority.require(this.options.authority, { caller, credential });

        return { caller, credential };
    }

    /** Verify a token's signature and registered claims, returning its payload. */
    async #verifySignature(token: string, now: number): Promise<unknown> {
        try {
            const { payload } = await jwtVerify(token, this.keys, {
                typ: ACCESS_TOKEN_TYPE,
                issuer: this.options.issuer,
                audience: PackageId.resource(this.options.audience),
                algorithms: ["ES256"],
                requiredClaims: ["iss", "aud", "sub", "client_id", "iat", "exp", "jti"],
                maxTokenAge: (this.options.lifetime ?? TOKEN_LIFETIME_MILLISECONDS) / 1000,
                clockTolerance: CLOCK_TOLERANCE_MILLISECONDS / 1000,
                currentDate: new Date(now),
            });

            return payload;
        } catch (error) {
            throw Bearer.refusal(error, "INVALID_TOKEN", "invalid access token");
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
    /** The public keys, an HTTPS JWKS endpoint, or a key set a directory stores. */
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
    /** Require an authentication's claims within an issuer's authority. */
    require(authority: TokenIssuerAuthority, authentication: Authentication): void {
        // limit a space's key to the space and its installations
        const { caller, credential } = authentication;
        const subjects = [caller.subject, ...caller.groups];
        if (authority.kind === "space") {
            requireSpaceSubjects(authority.space, caller, subjects);
        }

        // bind each asserted installation to one deployment
        requireDeployments(caller, credential, subjects);

        // require an acyclic chain of single principals, a delegate without a grant impersonating
        requireDelegation(caller);
    },
};

/** Read a token's client: none for the sender's key, else a user's client. */
function clientOf(token: AccessToken): { readonly client?: Identifier<"client"> } | undefined {
    // take the sender's key, the last delegate or else the subject, else require a client
    const sender = token.act?.at(-1)?.id ?? token.sub;
    const client = schema.identifier("client").safeParse(token.client_id);
    if (token.client_id === sender) {
        return {};
    } else if (client.success) {
        return { client: client.data };
    }

    return undefined;
}

/** Require a space key's claims to assert only the space or its installations. */
function requireSpaceSubjects(space: string, caller: Caller, subjects: readonly Subject[]): void {
    const isSpaceSubject = (subject: Subject) =>
        (principal.space.is(subject) && subject.id === space) ||
        (principal.installation.is(subject) && subject.scope === space);
    if (caller.contacts.length > 0 || caller.actors.length > 0 || !subjects.every(isSpaceSubject)) {
        throw AccessError.of("TOKEN_OUTSIDE_AUTHORITY", "token exceeds issuer authority");
    }
}

/** Require one deployment per asserted installation, at most one per actor, and no other. */
function requireDeployments(
    caller: Caller,
    credential: Credential,
    subjects: readonly Subject[],
): void {
    // list the asserted installations a deployment binds, and the actors one may bind
    const { kind } = credential;
    const isKeyed = kind === INSTALLATION_KEY || kind === SPACE_KEY;
    const workloads = subjects.filter(
        (subject) =>
            principal.installation.is(subject) &&
            !(isKeyed && Subject.same(subject, caller.subject)),
    );
    const actors = caller.actors.filter((subject) => principal.installation.is(subject));

    // bind each workload to one deployment, and each actor to at most one
    const deployments = credential.deployments ?? [];
    const matching = (subject: Subject) =>
        deployments.filter((deployment) => Subject.same(deployment.subject, subject)).length;
    if (
        workloads.some((subject) => matching(subject) !== 1) ||
        actors.some((subject) => matching(subject) > 1)
    ) {
        throw AccessError.of("INVALID_WORKLOAD_IDENTITY", "invalid workload token identity");
    }

    // reject deployments of no asserted workload or actor
    const bound = [...workloads, ...actors];
    if (
        deployments.some(
            (deployment) => !bound.some((subject) => Subject.same(subject, deployment.subject)),
        )
    ) {
        throw AccessError.of("INVALID_WORKLOAD_IDENTITY", "unexpected workload token identity");
    }
}

/** Require an acyclic chain of principals, an actor without a grant impersonating a person. */
function requireDelegation(caller: Caller): void {
    // list the chain from the subject to the sender
    const { actors } = caller;
    const chain = [caller.subject, ...actors];
    const isImpersonation = caller.grants.length === 0 && actors.length > 0;

    // refuse other subjects than principals, a repeated one, and an impersonation by other than one person
    if (
        actors.some((actor) => !isPrincipal(actor)) ||
        (isImpersonation &&
            (actors.length > 1 || !actors.every((actor) => principal.user.is(actor)))) ||
        chain.some((entry, position) =>
            chain.slice(position + 1).some((other) => Subject.same(entry, other)),
        )
    ) {
        throw AccessError.of("INVALID_DELEGATION", "invalid token delegation");
    }
}
