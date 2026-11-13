import { Bearer } from "./bearer.ts";
import { PackageId } from "@destack/package";
import type { Environment } from "@destack/environment";
import { IdentityError, Jwk, Jwks, type JwkSet, Jwt, type KeyLookup } from "@destack/identity";
import { schema } from "@destack/schema";
import { Scope, Subject } from "@destack/sync";
import {
    type Authentication,
    Caller,
    ClientId,
    CLOCK_TOLERANCE_MILLISECONDS,
    Credential,
    GrantClaim,
} from "../context/index.ts";
import { isPrincipal, principal } from "../declare/index.ts";
import { AccessError } from "../error/index.ts";
import { TelemetryScope } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";

/** The longest lifetime of a token and of the verified identity and membership records in it. */
export const TOKEN_LIFETIME_MILLISECONDS = 60000;
/** The credential kind of a machine's key. */
export const MACHINE_KEY = "machine-key";
/** The credential kind of an installation's key. */
export const INSTALLATION_KEY = "installation-key";
/** The credential kind of a space's key. */
export const SPACE_KEY = "space-key";
/** The credential kind of a token an OIDC issuer signs for a workload an account trusts. */
export const FEDERATED_TOKEN = "federated-token";
/** A partition a token is bound to: a space, an account, a machine or a device. */
export const PartitionId = schema.union([
    schema.identifier("space"),
    schema.identifier("account"),
    schema.identifier("machine"),
    schema.identifier("device"),
]);
/** A partition a token is bound to. */
export type PartitionId = schema.Infer<typeof PartitionId>;

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
    /** Write an authentication's Destack claims: identity, actors, grants and partition. */
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
                : { partition: PartitionId.parse(credential.scope) }),
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
        /** The client the token was issued to: the sender, a user's client, or a client's URL. */
        client_id: schema.union([
            ClientId,
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
        /** The partition the token is bound to, absent for a call to a universe package. */
        partition: PartitionId.exactOptional(),
        /** The space claim of older tokens, refused so none reads as unbound. */
        space: schema.never().exactOptional(),
        /** The verified identity. */
        caller: TokenIdentity,
    })
    .loose();
/** An access token as RFC 9068 shapes it. */
export type AccessToken = schema.Infer<typeof AccessToken>;

/** Verify partition-bound access tokens from the issuers it trusts, for any audience. */
export class TokenVerifier {
    /** The verifier configuration. */
    readonly options: TokenVerifierOptions;
    /** The verifier's spans and measures. */
    readonly #scope: TelemetryScope;
    /** Each issuer's last key set and its lookup, importing each key once per set. */
    readonly #lookups = new Map<
        string,
        { readonly set: string; readonly lookup: Promise<KeyLookup> }
    >();

    /** Create the verifier. */
    constructor(options: TokenVerifierOptions) {
        this.options = options;
        this.#scope = TelemetryScope.of(import.meta.destack.package, options.environment.telemetry);
    }

    /** Verify a request's bearer token. */
    async authenticate(request: Request, verification: TokenVerification): Promise<Authentication> {
        return this.verify(Bearer.require(request.headers), verification);
    }

    /** Verify an access token for an audience at a time, optionally for one partition. */
    verify(token: string, verification: TokenVerification): Promise<Authentication> {
        return this.#scope.span(
            "destack.access.token.verify",
            {},
            () => this.#verify(token, verification),
            { duration: { attributes: {} } },
        );
    }

    /** Verify a token's signature, claims, partition and client. */
    async #verify(token: string, verification: TokenVerification): Promise<Authentication> {
        // verify the signature and registered claims by the token's trusted issuer
        const { audience, now, partition } = verification;
        const { payload, issuer } = await this.#verifySignature(token, verification);

        // parse the claims and check the partition and the client
        const result = AccessToken.safeParse(payload);
        const client = result.success ? clientOf(result.data) : undefined;
        if (
            !result.success ||
            client === undefined ||
            (partition !== undefined && result.data.partition !== partition) ||
            result.data.sub !== result.data.caller.subject.id ||
            result.data.exp <= result.data.iat
        ) {
            throw AccessError.of(
                "TOKEN_UNAUTHENTICATED",
                "authentication required: access token claims do not verify",
            );
        }

        // refuse a token outliving its audience's lifetime
        const claims = result.data;
        const { credential: presented, ...identity } = claims.caller;
        const verifiedAt = claims.iat * 1000;
        const lifetime = this.options.lifetime ?? TOKEN_LIFETIME_MILLISECONDS;
        if ((claims.exp - claims.iat) * 1000 > lifetime) {
            throw AccessError.of(
                "TOKEN_UNAUTHENTICATED",
                "authentication required: access token claims do not verify",
            );
        }

        // verify it for this audience in the partition it calls
        const caller: Caller = {
            ...identity,
            actors: claims.act ?? [],
            grants: claims.grants ?? [],
        };
        const credential: Credential = {
            ...presented,
            ...client,
            audience,
            ...(claims.partition === undefined ? {} : { scope: claims.partition }),
            verifiedAt,
            expiresAt: Math.min(claims.exp * 1000, verifiedAt + lifetime),
            tokenId: claims.jti,
        };

        // require it current and within its issuer's authority
        Credential.requireCurrent(credential, audience, now, claims.partition);
        TokenIssuerAuthority.require(issuer.authority, { caller, credential });

        return { caller, credential };
    }

    /** Find an issuer's lookup of a key set, importing a changed set's keys once. */
    #lookup(issuer: string, keys: JwkSet): Promise<KeyLookup> {
        // reuse the lookup of an unchanged set
        const set = JSON.stringify(keys);
        const known = this.#lookups.get(issuer);
        if (known?.set === set) {
            return known.lookup;
        }

        // identify each key by its thumbprint unless it has an identifier, then look up the set
        const { crypto } = this.options.environment;
        const identified = keys.keys.map(async (jwk) =>
            jwk.kid === undefined ? { ...jwk, kid: await Jwk.thumbprint(jwk, crypto) } : jwk,
        );
        const lookup = Promise.all(identified).then((each) => Jwks.local({ keys: each }, crypto));
        this.#lookups.set(issuer, { set, lookup });

        return lookup;
    }

    /** Verify a token's signature and registered claims by its trusted issuer, returning both. */
    async #verifySignature(
        token: string,
        { audience, now }: TokenVerification,
    ): Promise<{ readonly payload: unknown; readonly issuer: TokenTrust }> {
        try {
            // resolve the token's issuer, refusing one the verifier does not trust
            const name = Bearer.issuer(token);
            const issuer = name === undefined ? undefined : await this.options.issuers(name);
            if (name === undefined || issuer === undefined) {
                throw IdentityError.of(
                    "ISSUER_UNAUTHENTICATED",
                    "authentication required: the token's issuer is not trusted",
                );
            }

            // verify the signature and registered claims by its keys
            const { claims } = await Jwt.verify(
                token,
                async (header) => (await this.#lookup(name, issuer.keys))(header),
                {
                    typ: ACCESS_TOKEN_TYPE,
                    issuer: name,
                    audience: PackageId.resource(audience),
                    algorithms: ["ES256"],
                    requiredClaims: ["iss", "aud", "sub", "client_id", "iat", "exp", "jti"],
                    maxTokenAge: (this.options.lifetime ?? TOKEN_LIFETIME_MILLISECONDS) / 1000,
                    clockTolerance: CLOCK_TOLERANCE_MILLISECONDS / 1000,
                    now,
                },
                this.options.environment.crypto,
            );

            return { payload: claims, issuer };
        } catch (error) {
            throw Bearer.refusal(error, "TOKEN_UNAUTHENTICATED", "invalid access token");
        }
    }
}

/** The configuration of a token verifier. */
export interface TokenVerifierOptions {
    /** Resolve an issuer to the verifier's trust in it, absent for an untrusted issuer. */
    readonly issuers: (issuer: string) => Promise<TokenTrust | undefined>;
    /** The WebCrypto verifying, and the telemetry tracing each verify. */
    readonly environment: Pick<Environment, "crypto" | "telemetry">;
    /** The longest lifetime of the tokens it accepts, a minute by default. */
    readonly lifetime?: number;
}

/** The trust in an issuer: the identities it may assert and its current keys. */
export interface TokenTrust {
    /** The identities the issuer's keys may assert. */
    readonly authority: TokenIssuerAuthority;
    /** The issuer's current keys, identified by thumbprint when they have no identifier. */
    readonly keys: JwkSet;
}

/** What one verify checks a token for. */
export interface TokenVerification {
    /** The receiving package. */
    readonly audience: PackageId;
    /** The time of the verify. */
    readonly now: number;
    /** The partition the token must be bound to, any when absent. */
    readonly partition?: PartitionId;
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
function clientOf(token: AccessToken): { readonly client?: ClientId } | undefined {
    // take the sender's key, the last delegate or else the subject, else require a client
    const sender = token.act?.at(-1)?.id ?? token.sub;
    const client = ClientId.safeParse(token.client_id);
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
        throw AccessError.of(
            "TOKEN_OUTSIDE_AUTHORITY_UNAUTHENTICATED",
            "authentication required: token exceeds issuer authority",
        );
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
        throw AccessError.of(
            "WORKLOAD_IDENTITY_UNAUTHENTICATED",
            "authentication required: workload token identity must bind each workload to one deployment",
        );
    }

    // reject deployments of no asserted workload or actor
    const bound = [...workloads, ...actors];
    if (
        deployments.some(
            (deployment) => !bound.some((subject) => Subject.same(subject, deployment.subject)),
        )
    ) {
        throw AccessError.of(
            "WORKLOAD_IDENTITY_UNAUTHENTICATED",
            "authentication required: workload token has a deployment of no asserted subject",
        );
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
        throw AccessError.of(
            "DELEGATION_UNAUTHENTICATED",
            "authentication required: token delegation does not verify",
        );
    }
}
