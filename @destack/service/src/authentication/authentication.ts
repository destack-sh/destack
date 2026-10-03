import { PackageId } from "@destack/package";
import { Subject } from "@destack/sync";
import { type AccessContext, Attribute, Caller, isPrincipal, principal } from "@destack/access";
import { type Identifier, schema } from "@destack/schema";
import { ServiceError } from "../error/index.ts";
import type { TokenIssuerAuthority } from "./token.ts";

/** The maximum age of verified identity and membership records. */
export const AUTHENTICATION_LIFETIME_MILLISECONDS = 60000;
/** The maximum clock difference between the authority and the receiving service. */
export const AUTHENTICATION_CLOCK_TOLERANCE_MILLISECONDS = 5000;

/** A workload identity authenticated within one deployment. */
export const DeploymentClaim = schema.object({
    /** The workload identity. */
    subject: Subject,
    /** The deployment. */
    id: schema.identifier("deployment"),
});
/** A workload identity authenticated within one deployment. */
export type DeploymentClaim = schema.Infer<typeof DeploymentClaim>;

/** A verified caller in transit between hosts, runners and services: the caller, its credential and its lifetime. */
export const AuthenticationClaims = Caller.schema.extend({
    /** The represented identity. */
    subject: Subject,
    /** The authority scope of a scoped credential. */
    scope: schema.string().min(1).exactOptional(),
    /** The credential reference. */
    credential: schema.object({
        /** The credential kind. */
        kind: schema.string().min(1),
        /** The credential identifier. */
        id: schema.string().min(1),
    }),
    /** The receiving package. */
    audience: PackageId,
    /** The verification time, in Unix milliseconds. */
    verifiedAt: schema.number(),
    /** The exclusive expiry, in Unix milliseconds. */
    expiresAt: schema.number(),
    /** The verified deployments of workload identities. */
    deployments: schema.array(DeploymentClaim).exactOptional(),
    /** The trusted attributes access policies read. */
    attributes: schema.record(schema.string(), Attribute).exactOptional(),
    /** A lending of the caller's authority to the installation it called, which its holder signed, for the calls that installation sends. */
    delegation: schema.string().min(1).exactOptional(),
});

/** The header with the authentication a host forwards to a runner. */
export const AUTHENTICATION_HEADER = "x-destack-authentication";

/** A host-verified authentication of a caller, with its credential and lifetime. */
export class Authentication<Credential extends CredentialReference = CredentialReference> {
    /** The verified claims. */
    readonly claims: AuthenticationClaims<Credential>;

    /** Create the authentication from its verified claims. */
    constructor(claims: AuthenticationClaims<Credential>) {
        this.claims = structuredClone(claims);
    }

    /** Read the caller a host forwarded with a request, or null for a request without one. */
    static forwarded(request: Request): Authentication | null {
        const forwarded = request.headers.get(AUTHENTICATION_HEADER);

        return forwarded === null
            ? null
            : new Authentication(AuthenticationClaims.parse(JSON.parse(forwarded)));
    }

    /** Forward the caller to a runner in a request's headers. */
    forward(headers: Headers): void {
        headers.set(AUTHENTICATION_HEADER, JSON.stringify(this.claims));
    }

    /** The credential reference. */
    get credential(): Credential {
        return this.claims.credential;
    }

    /** The time the caller's authentication lapses, in Unix milliseconds. */
    get lapsesAt(): number {
        const { expiresAt, verifiedAt } = this.claims;

        return Math.min(expiresAt, verifiedAt + AUTHENTICATION_LIFETIME_MILLISECONDS);
    }

    /** Clamp a time to the caller's lifetime: the time itself, or just before the lapse once passed. */
    within(now: number): number {
        return Math.min(now, this.lapsesAt - 1);
    }

    /** Read the verified deployment a workload identity among the caller's runs in, absent for none. */
    deployment(subject: Subject): Identifier<"deployment"> | undefined {
        const deployments = this.claims.deployments ?? [];

        return deployments.find((entry) => Subject.same(entry.subject, subject))?.id;
    }

    /** The principal sending the request: the last delegate, or else the subject. */
    get sender(): Subject {
        const { delegates, subject } = this.claims;

        return delegates?.at(-1)?.subject ?? subject;
    }

    /** The retry identity of the subject and the sending principal. */
    get id(): string {
        return JSON.stringify([Subject.key(this.claims.subject), Subject.key(this.sender)]);
    }

    /** Require current authentication for the audience and scope. */
    requireCurrent(audience: PackageId, now: number, scope?: string): void {
        // reject another audience or scope and stale authentication
        const claims = this.claims;
        if (
            claims.audience !== audience ||
            (claims.scope !== undefined && claims.scope !== scope) ||
            !Number.isFinite(claims.verifiedAt) ||
            !Number.isFinite(claims.expiresAt) ||
            claims.verifiedAt > now + AUTHENTICATION_CLOCK_TOLERANCE_MILLISECONDS ||
            now >= this.lapsesAt
        ) {
            throw new ServiceError("UNAUTHORIZED", {
                message: "caller authentication is expired or has a different audience or scope",
            });
        }

        // require the subject among the verified identities
        if (!claims.subjects.some((subject) => Subject.same(subject, claims.subject))) {
            throw new ServiceError("UNAUTHORIZED", {
                message: "caller subject is missing from verified identities",
            });
        }
    }

    /** Build an access context for the audience after checking freshness. */
    context(audience: PackageId, now = Date.now(), scope?: string): AccessContext {
        this.requireCurrent(audience, now, scope);
        const claims = this.claims;

        return {
            subject: claims.subject,
            subjects: claims.subjects,
            ...(claims.assurance === undefined ? {} : { assurance: claims.assurance }),
            ...(claims.contacts === undefined ? {} : { contacts: claims.contacts }),
            session: SessionKey.of(claims.credential),
            ...(claims.delegates === undefined ? {} : { delegates: claims.delegates }),
            attributes: claims.attributes ?? {},
            ...(claims.permissions === undefined ? {} : { permissions: claims.permissions }),
            now,
        };
    }

    /** Require identity claims within an issuer's authority. */
    requireAuthority(authority: TokenIssuerAuthority): void {
        // collect every asserted identity
        const claims = this.claims;
        const subjects = [claims.subject, ...claims.subjects];

        // allow a space key to assert only installations of its space, calling any space
        if (
            authority.kind === "space" &&
            ((claims.contacts ?? []).length > 0 ||
                (claims.delegates ?? []).length > 0 ||
                subjects.some(
                    (subject) =>
                        !principal.installation.is(subject) || subject.scope !== authority.spaceId,
                ))
        ) {
            throw new ServiceError("UNAUTHORIZED", { message: "token exceeds issuer authority" });
        }

        // bind each asserted installation to one deployment
        const workloads = [
            ...subjects,
            ...(claims.delegates ?? []).map((delegate) => delegate.subject),
        ].filter((subject) => principal.installation.is(subject));
        const deployments = claims.deployments ?? [];
        for (const subject of workloads) {
            const matching = deployments.filter((deployment) =>
                Subject.same(deployment.subject, subject),
            ).length;
            if (matching !== 1) {
                throw new ServiceError("UNAUTHORIZED", {
                    message: "invalid workload token identity",
                });
            }
        }

        // reject deployments of no asserted workload
        if (
            deployments.some(
                (deployment) =>
                    !workloads.some((subject) => Subject.same(subject, deployment.subject)),
            )
        ) {
            throw new ServiceError("UNAUTHORIZED", {
                message: "unexpected workload token identity",
            });
        }

        // require an acyclic chain of single principals
        const delegates = claims.delegates ?? [];
        const chain = [claims.subject, ...delegates.map((delegate) => delegate.subject)];
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
}

/** A verified caller in transit with a credential reference from its verifier. */
export type AuthenticationClaims<Credential extends CredentialReference = CredentialReference> =
    Omit<schema.Infer<typeof AuthenticationClaims>, "credential"> & {
        /** The credential reference. */
        readonly credential: Credential;
    };

/** A credential a caller presented, by kind and identifier. */
export interface CredentialReference {
    /** The credential kind, such as session or host-key. */
    readonly kind: string;
    /** The credential identifier. */
    readonly id: string;
}

/** A caller's session key in access decisions: its credential's kind and identifier. */
export const SessionKey = {
    /** Build a credential's session key. */
    of(credential: CredentialReference): string {
        return `${credential.kind}:${credential.id}`;
    },

    /** Read the credential identifier of a session key of one kind, absent for another kind. */
    id(key: string | undefined, kind: string): string | undefined {
        const prefix = `${kind}:`;

        return key?.startsWith(prefix) === true ? key.slice(prefix.length) : undefined;
    },
};
